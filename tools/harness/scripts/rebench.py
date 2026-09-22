"""rebench.py — re-bench a crate against its merged tests.

Per crate, the flow is:
  1. copy bootcamp/<crate> -> tmp_rebench/<crate>
  2. discover workspace; pick primary
  3. plant tests from recipes/<crate>/tests/ -> primary/tests/
  4. measure api coverage scoped to primary only -> rebench/<crate>/coverage.json
  5. run features in order:
       a) unsafe_counter (1 run, with_native + without_native)
       b) heap_tracker (1 run)
       c) cpu_cycle_counter (3 runs into run1/run2/run3)
  6. aggregate cpu_cycle runs -> rebench/<crate>/cpu_cycle_counter/avg.json
  7. write rebench_summary.json
  8. delete tmp_rebench/<crate>  (unless --keep-tmp)

cargo test scope is restricted to `--tests` only — we don't want lib unit
tests, examples, or bins polluting the counters. Only the integ tests in
tests/*.rs (which include our generated + planted tests) participate.

Usage:
  /usr/bin/python3 scripts/rebench.py --crate hashbrown
  /usr/bin/python3 scripts/rebench.py --crate hashbrown,curl,rayon-core
  /usr/bin/python3 scripts/rebench.py --all-with-tests
  /usr/bin/python3 scripts/rebench.py --crate curl --cpu-runs 5 --keep-tmp
"""

from __future__ import annotations

import argparse
import json
import logging
import re
import shutil
import statistics
import subprocess
import sys
import time
from dataclasses import asdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from pipeline.config import load_config
from pipeline.tools.api_discovery import compute_api_coverage, discover_public_api
from pipeline.tools.coverage import run_full_coverage
from pipeline.tools.crate_prep import copy_crate
from pipeline.tools.csv_book import load_csv
from pipeline.tools.runtime_bench import (
    BenchResult,
    FeatureResult,
    ensure_unsafe_perf_built,
    run_feature,
    set_failure_log_dir,
)
from pipeline.tools.workspace import _extract_lib_name, discover_workspace

log = logging.getLogger("rebench")

# the rebench bench scope: only integration tests under tests/*.rs.
TEST_TARGETS = ["--tests"]

# Per-crate cargo features. Required for crates whose tests are cfg-gated
# behind non-default features (e.g. tokio's 150/151 test files are
# #![cfg(feature = "...")]-gated and its primary has default = []).
# This explicit map takes precedence over the feature_label decode below.
CRATE_FEATURES: dict[str, list[str]] = {
    "tokio": ["full"],
}

# Per-crate skip-list for the stdlib_api_tracker MIR pass. Stage1 1.80-dev rustc
# ICEs ("broken MIR ... violates unwind invariants") when the pass inserts an
# `asm!("# __unsafe_stdlib_call:...")` marker inside a cleanup block. Only
# `bytes` 1.0.1 trips this (shallow_clone_vec at src/bytes.rs:1053 has a
# stdlib Drop call in an unwind-edge basic block). Skipping the tracker drops
# only the .stdlib_api.json side dump; .unsafe_counter.json is still emitted
# in full, so RQ3/4/5 (which only use UNSAFE_COUNTER_FIELDS) are unaffected.
CRATE_NO_STDLIB_TRACKER: set[str] = {"bytes"}

# Per-crate primary-package override (workspace member, relative to the crate
# root). Use when discover_workspace picks the wrong member for a multi-package
# workspace. msgpack-rust: the corpus entry is `rmp-serde`, but discovery picks
# `rmp`; the planted gen_rmp_serde_* tests `use rmp_serde`, which only resolves
# inside the rmp-serde package (rmp does not depend on rmp_serde — it's the
# reverse). rmp-serde depends on rmp, so the rmp_* tests still compile there.
CRATE_PRIMARY: dict[str, str] = {
    "msgpack-rust": "rmp-serde",
}

# Per-crate dependency pins applied before benching (host `cargo update
# --precise`). miette --all-features enables `fancy`, pulling backtrace ^0.3.69
# which resolves to 0.3.76 — its libunwind.rs uses the edition-2024
# `unsafe extern {}` form that the stage1 1.80-dev rustc cannot parse. Pin to
# the last backtrace without it. (version confirmed at runtime before use.)
CRATE_DEP_PINS: dict[str, list[tuple[str, str]]] = {
    "miette": [("backtrace", "0.3.73")],
}


def apply_dep_pins(crate_name: str, ws_root: Path, cfg) -> list[str]:
    """Run `cargo update -p <dep> --precise <ver>` for any configured pins.
    Returns the list of applied "dep@ver" strings (for the summary)."""
    pins = CRATE_DEP_PINS.get(crate_name)
    if not pins:
        return []
    applied = []
    for dep, ver in pins:
        cmd = [cfg.cargo, "update", "-p", dep, "--precise", ver]
        log.info(f"  [dep-pin] {' '.join(cmd)}  (cwd={ws_root})")
        r = subprocess.run(cmd, cwd=ws_root, capture_output=True, text=True)
        if r.returncode != 0:
            log.warning(f"  [dep-pin] FAILED {dep}@{ver}: "
                        f"{r.stderr.strip()[:300]}")
        else:
            applied.append(f"{dep}@{ver}")
    return applied

# fixed processing order requested in the task.
FEATURE_ORDER = ["unsafe_counter", "heap_tracker", "cpu_cycle_counter"]

REBENCH_ROOT = ROOT / "rebench"  # default; overridable via --out-dir
TMP_REBENCH = ROOT / "tmp_rebench"

# Canonical 100-crate corpus + the per-crate feature rung the profiling
# pipeline discovered (coverage.py's feature ladder). Driving the run from
# this file (rather than the Sheet6 CSV) decouples the crate list from stale
# bookkeeping and lets us honor each crate's feature_label — the snag in
# rebench_v2, where every crate but tokio was benched on DEFAULT features even
# though 32 of them need --all-features to compile the planted tests and reach
# their feature-gated unsafe paths.
CORPUS_CSV = ROOT / "rebench_100crates.csv"
_FEATURE_LABELS: dict[str, str] | None = None  # dir_name -> feature_label


def _load_feature_labels() -> dict[str, str]:
    """dir_name -> feature_label from the corpus CSV (cached)."""
    global _FEATURE_LABELS
    if _FEATURE_LABELS is None:
        import csv
        labels: dict[str, str] = {}
        if CORPUS_CSV.exists():
            for r in csv.DictReader(CORPUS_CSV.open()):
                d = (r.get("dir_name") or "").strip()
                if d:
                    labels[d] = (r.get("feature_label") or "").strip()
        _FEATURE_LABELS = labels
    return _FEATURE_LABELS


def crate_cargo_args(crate_name: str) -> list[str]:
    """resolve the cargo feature/scope flags for one crate.

    precedence: explicit CRATE_FEATURES override (-> --features X,Y) wins;
    otherwise decode the corpus feature_label. Only the feature RUNG matters
    for the bench (the workspace SCOPE is already handled by running cargo in
    the discovered primary-crate dir with --tests): any label containing
    'all-features' -> --all-features, everything else -> default (no flag).
    """
    if crate_name in CRATE_FEATURES:
        return ["--features", ",".join(CRATE_FEATURES[crate_name])]
    label = _load_feature_labels().get(crate_name, "")
    if "all-features" in label:
        return ["--all-features"]
    return []


def setup_logging() -> None:
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s %(levelname)s %(message)s",
        datefmt="%H:%M:%S",
    )


# ---------------------------------------------------------------------------
# steps
# ---------------------------------------------------------------------------

def plant_tests(recipes_tests_dir: Path, primary_path: Path) -> int:
    """copy every *.rs from recipes/<crate>/tests/ into primary/tests/.

    duplicates (same filename) are overwritten — there should not be any
    given our gen_* / <crate>_* prefix convention vs. upstream test names.
    """
    if not recipes_tests_dir.exists():
        log.info(f"  no tests in {recipes_tests_dir} — nothing to plant")
        return 0
    dst = primary_path / "tests"
    dst.mkdir(parents=True, exist_ok=True)
    planted = 0
    for rs in sorted(recipes_tests_dir.glob("*.rs")):
        shutil.copy2(rs, dst / rs.name)
        planted += 1
    log.info(f"  planted {planted} tests from {recipes_tests_dir} -> {dst}")
    return planted


def measure_primary_coverage(
    primary_path: Path, ws, cfg, out_path: Path,
) -> dict:
    """compute primary-crate-only api coverage. Writes coverage.json and
    returns the summary dict.
    """
    log.info(f"  [coverage] discovering primary api for {ws.primary_package_name}")
    api = discover_public_api(
        primary_path,
        rusttest_gen_binary=cfg.rusttest_gen_binary,
        lib_name=ws.primary_lib_name,
        workspace_root=ws.path if ws.is_workspace else None,
    )
    log.info(f"  [coverage] api surface: {api.total} items "
             f"({api.countable_total} countable)")

    log.info(f"  [coverage] running cargo llvm-cov --tests (primary scope)")
    cov = run_full_coverage(ws)
    if not cov.ok:
        log.warning(f"  [coverage] llvm-cov failed: {cov.error[:200]}")

    covered_fns = cov.covered_functions if cov.ok else set()
    api_covered, api_total, api_pct, uncovered_paths = compute_api_coverage(
        api, covered_fns,
    )
    log.info(f"  [coverage] api: {api_covered}/{api_total} ({api_pct:.1f}%)  "
             f"line: {cov.percent:.1f}%")

    summary = {
        "crate": primary_path.name,
        "primary_package": ws.primary_package_name,
        "primary_lib": ws.primary_lib_name,
        "is_workspace": ws.is_workspace,
        "api_surface_total": api.total,
        "api_countable_total": api.countable_total,
        "api_covered": api_covered,
        "api_pct": api_pct,
        "line_pct": cov.percent if cov.ok else 0.0,
        "coverage_ok": cov.ok,
        "coverage_error": cov.error if not cov.ok else "",
        "feature_label": cov.feature_label if cov.ok else "",
        "uncovered_api_paths": uncovered_paths,
    }
    out_path.write_text(json.dumps(summary, indent=2))
    return summary


def _feature_to_dict(fr: FeatureResult) -> dict:
    d = asdict(fr)
    # variants is dict[str, VariantResult] — asdict already handles dataclasses
    # nested in dicts. nothing extra needed.
    return d


def run_one_feature(
    crate_path: Path, cfg, feature: str, target_dir: Path, bin_timeout: int,
    cargo_features: list[str] | None = None,
    extra_cargo_args: list[str] | None = None,
) -> FeatureResult:
    """run one feature into a flat target_dir/<variant>/ layout."""
    log.info(f"[rebench] feature={feature} -> {target_dir}")
    target_dir.mkdir(parents=True, exist_ok=True)
    return run_feature(
        crate_path, cfg, feature,
        out_dir=target_dir.parent,        # unused when variants_root is set
        bin_timeout=bin_timeout,
        test_targets=TEST_TARGETS,
        variants_root=target_dir,
        cargo_features=cargo_features,
        extra_cargo_args=extra_cargo_args,
    )


def aggregate_cpu_cycle(cpu_dir: Path, run_results: list[FeatureResult]) -> Path:
    """walk per-run JSON stat files and emit avg.json with mean+stddev per
    (variant, bin, metric).
    """
    log.info(f"  [aggregate] cpu_cycle avg from {len(run_results)} run(s)")
    # bin_name is keyed without the pid suffix; stat filenames look like
    # `<bin>__<pid>.<bin>.cpu_cycle.json` so we parse the bin substring out.
    agg: dict[str, dict[str, dict[str, list[float]]]] = {}
    # agg[variant][bin][metric] -> [values from each run]

    hash_suffix = re.compile(r"-[0-9a-f]{16}$")

    # pre-scan for logical-name collisions: cargo names both a lib unittest and
    # a same-named integration test `<crate>`, so stripping the 16-hex hash
    # would merge two distinct bins under one key and interleave their per-run
    # samples (e.g. brotli/pulldown-cmark/zopfli). Detect (variant, logical)
    # pairs backed by >1 distinct hashed bin and keep the hash for those only;
    # the hash is stable across run1/2/3 (same RUSTFLAGS), so per-bin sample
    # arrays stay correctly aligned. Clean crates keep hash-free keys.
    raw_by_logical: dict[tuple[str, str], set[str]] = {}
    for run_idx, _ in enumerate(run_results, start=1):
        run_dir = cpu_dir / f"run{run_idx}"
        if not run_dir.exists():
            continue
        for variant_dir in run_dir.iterdir():
            if not variant_dir.is_dir():
                continue
            for jf in variant_dir.glob("*.cpu_cycle.json"):
                raw_bin = jf.name.split("__", 1)[0]
                logical = hash_suffix.sub("", raw_bin)
                raw_by_logical.setdefault((variant_dir.name, logical),
                                          set()).add(raw_bin)
    collided = {key for key, raws in raw_by_logical.items() if len(raws) > 1}

    for run_idx, fr in enumerate(run_results, start=1):
        run_dir = cpu_dir / f"run{run_idx}"
        if not run_dir.exists():
            log.warning(f"    run{run_idx} dir missing — skip")
            continue
        for variant_dir in run_dir.iterdir():
            if not variant_dir.is_dir():
                continue
            variant = variant_dir.name
            for jf in variant_dir.glob("*.cpu_cycle.json"):
                try:
                    data = json.loads(jf.read_text())
                except Exception as e:
                    log.warning(f"    skip unparseable {jf}: {e}")
                    continue
                # canonical bin name: prefer the JSON's "binary" field, then
                # strip cargo's `-<16hex>` hash suffix so the avg.json keys
                # don't change just because the hash bumped.
                raw_bin = data.get("binary") or jf.name.split("__", 1)[0]
                logical = hash_suffix.sub("", raw_bin)
                # keep the hash only when this logical name is ambiguous
                bin_name = raw_bin if (variant, logical) in collided else logical
                # only flatten the `stats` block — the schema/pid/binary/metric
                # fields aren't measurements.
                stats = data.get("stats", {})
                flat = _flatten_metrics(stats)
                bucket = agg.setdefault(variant, {}).setdefault(bin_name, {})
                for k, v in flat.items():
                    bucket.setdefault(k, []).append(v)

    out: dict = {"runs": len(run_results), "variants": {}}
    for variant, bins in agg.items():
        out["variants"][variant] = {}
        for bin_name, metrics in bins.items():
            entry: dict = {"runs_observed": 0}
            for k, values in metrics.items():
                entry["runs_observed"] = max(entry["runs_observed"], len(values))
                entry[k] = {
                    "mean": statistics.fmean(values) if values else 0.0,
                    "stddev": statistics.pstdev(values) if len(values) > 1 else 0.0,
                    "samples": values,
                }
            out["variants"][variant][bin_name] = entry

    avg_path = cpu_dir / "avg.json"
    avg_path.write_text(json.dumps(out, indent=2))
    log.info(f"  [aggregate] wrote {avg_path}")
    return avg_path


def _flatten_metrics(data: dict, prefix: str = "") -> dict[str, float]:
    """flatten one level of nested dict into 'key.subkey' = value, keeping
    only numeric leaves."""
    out: dict[str, float] = {}
    for k, v in data.items():
        full = f"{prefix}{k}" if not prefix else f"{prefix}.{k}"
        if isinstance(v, dict):
            out.update(_flatten_metrics(v, full))
        elif isinstance(v, (int, float)):
            out[full] = float(v)
        # ignore lists/strings/None
    return out


def rebench_crate(
    crate_name: str, cfg, cpu_runs: int, bin_timeout: int, keep_tmp: bool,
    selected_features: set[str] | None = None,
    skip_coverage: bool = False,
    counter_runs: int = 1,
    heap_runs: int = 1,
) -> dict:
    """end-to-end rebench for one crate. returns the top-level summary dict."""
    if selected_features is None:
        selected_features = set(FEATURE_ORDER)
    started = time.time()
    log.info(f"\n{'='*60}\nrebench: {crate_name} "
             f"features={sorted(selected_features)}\n{'='*60}")
    bootcamp = Path(cfg.bootcamp_dir)
    if not (bootcamp / crate_name).exists():
        return {"crate": crate_name, "error": f"bootcamp/{crate_name} not found"}

    recipes_tests = ROOT / cfg.recipes_dir / crate_name / "tests"
    rebench_out = REBENCH_ROOT / crate_name
    rebench_out.mkdir(parents=True, exist_ok=True)

    # step 1: copy
    TMP_REBENCH.mkdir(parents=True, exist_ok=True)
    crate_path = copy_crate(bootcamp, crate_name, TMP_REBENCH)

    summary: dict = {
        "crate": crate_name,
        "tmp_path": str(crate_path),
        "out_dir": str(rebench_out),
        "instrument_all_deps": False,
    }
    try:
        # step 2: workspace discovery
        ws = discover_workspace(crate_path)
        override = CRATE_PRIMARY.get(crate_name)
        if override:
            member_path = crate_path / override
            mtoml = member_path / "Cargo.toml"
            if mtoml.exists():
                import toml
                pdata = toml.loads(mtoml.read_text())
                ws.is_workspace = True
                ws.primary_crate_path = member_path
                ws.primary_package_name = pdata.get("package", {}).get("name", override)
                ws.primary_lib_name = _extract_lib_name(pdata, ws.primary_package_name)
                log.info(f"  primary OVERRIDE -> {ws.primary_package_name} "
                         f"(lib={ws.primary_lib_name}) at {member_path}")
            else:
                log.warning(f"  CRATE_PRIMARY override {override!r} not found at "
                            f"{member_path} — using discovered primary")
        primary_path = ws.primary_crate_path if ws.is_workspace else crate_path
        summary["primary_package"] = ws.primary_package_name
        summary["primary_lib"] = ws.primary_lib_name
        summary["is_workspace"] = ws.is_workspace

        # step 3: plant tests
        planted = plant_tests(recipes_tests, primary_path)
        summary["tests_planted"] = planted

        # step 3b: apply any per-crate dependency pins (cargo update --precise)
        # at the workspace root, before the bench builds.
        pinned = apply_dep_pins(crate_name, crate_path, cfg)
        if pinned:
            summary["dep_pins"] = pinned
        if planted == 0:
            log.warning(f"  no recipes/{crate_name}/tests/ entries — bench will "
                        f"only exercise upstream tests under --tests scope")

        # step 4: coverage (optional — skipped for bench-only runs)
        if skip_coverage:
            log.info("  [coverage] skipped (--no-coverage)")
        else:
            try:
                cov_summary = measure_primary_coverage(
                    primary_path, ws, cfg, rebench_out / "coverage.json",
                )
                summary["coverage"] = cov_summary
            except Exception as e:
                log.warning(f"  coverage failed: {e}")
                summary["coverage"] = {"error": str(e)}

        # step 5: pre-build the unsafe-perf rlibs ahead of time so the first
        # run of each feature doesn't carry the build cost. ensure_unsafe_perf
        # is idempotent.
        for feature in FEATURE_ORDER:
            if feature not in selected_features:
                continue
            try:
                ensure_unsafe_perf_built(
                    Path(cfg.unsafe_perf_path), feature, cfg.rustc, cfg.cargo,
                )
            except Exception as e:
                log.warning(f"  pre-build {feature} rlib: {e}")

        # step 6: run features
        features_out: dict = {}
        # per-crate feature rung (decoded from the corpus feature_label, with
        # CRATE_FEATURES overriding). passed verbatim as cargo args.
        extra = crate_cargo_args(crate_name)
        label = _load_feature_labels().get(crate_name, "")
        summary["feature_label"] = label
        summary["cargo_feature_args"] = extra
        log.info(f"  feature_label={label!r} -> cargo args {extra or '(default)'}")

        # unsafe_counter (1 run)
        if "unsafe_counter" in selected_features:
            target = rebench_out / "unsafe_counter"
            # See CRATE_NO_STDLIB_TRACKER docstring: for the listed crates,
            # remove the LLVM stdlib-api-tracker llvm-arg + the matching MIR-
            # pass env var for the duration of this feature run, then restore.
            from pipeline.tools.runtime_bench import FEATURE_MATRIX as _FM
            _uc_spec = _FM["unsafe_counter"]
            _saved = (list(_uc_spec["llvm_flags"]), dict(_uc_spec["env"]))
            if crate_name in CRATE_NO_STDLIB_TRACKER:
                _uc_spec["llvm_flags"] = [
                    f for f in _uc_spec["llvm_flags"]
                    if f != "--enable-stdlib-api-tracker"
                ]
                _uc_spec["env"] = {
                    k: v for k, v in _uc_spec["env"].items()
                    if k != "UNSAFE_ENABLE_STDLIB_TRACKER"
                }
                log.info(f"  [unsafe_counter] {crate_name}: skipping stdlib-api-"
                         f"tracker (rustc MIR pass ICE workaround)")
            try:
                fr = run_one_feature(primary_path, cfg, "unsafe_counter", target, bin_timeout,
                                     extra_cargo_args=extra)
                # Repetitions land under <feature>/rep<N>/ so the first run
                # keeps the exact layout the aggregator and every published
                # analysis already expect. Only the spread is computed from
                # the extra runs.
                for i in range(2, counter_runs + 1):
                    run_one_feature(primary_path, cfg, "unsafe_counter",
                                    target / f"rep{i}", bin_timeout,
                                    extra_cargo_args=extra)
            finally:
                _uc_spec["llvm_flags"], _uc_spec["env"] = _saved
            features_out["unsafe_counter"] = _feature_to_dict(fr)

        # heap_tracker (1 run)
        if "heap_tracker" in selected_features:
            target = rebench_out / "heap_tracker"
            fr = run_one_feature(primary_path, cfg, "heap_tracker", target, bin_timeout,
                                 extra_cargo_args=extra)
            for i in range(2, heap_runs + 1):
                run_one_feature(primary_path, cfg, "heap_tracker",
                                target / f"rep{i}", bin_timeout,
                                extra_cargo_args=extra)
            features_out["heap_tracker"] = _feature_to_dict(fr)

        # cpu_cycle_counter (N runs)
        if "cpu_cycle_counter" in selected_features:
            cpu_dir = rebench_out / "cpu_cycle_counter"
            cpu_dir.mkdir(parents=True, exist_ok=True)
            cpu_runs_results: list[FeatureResult] = []
            for i in range(1, cpu_runs + 1):
                run_target = cpu_dir / f"run{i}"
                fr = run_one_feature(
                    primary_path, cfg, "cpu_cycle_counter", run_target, bin_timeout,
                    extra_cargo_args=extra,
                )
                cpu_runs_results.append(fr)
            features_out["cpu_cycle_counter"] = [_feature_to_dict(fr) for fr in cpu_runs_results]

            # aggregate cpu_cycle
            try:
                aggregate_cpu_cycle(cpu_dir, cpu_runs_results)
            except Exception as e:
                log.warning(f"  cpu_cycle aggregation failed: {e}")

        # if running a partial feature set, merge with any existing summary
        # on disk so we don't blow away features that aren't being re-bench'd.
        if selected_features != set(FEATURE_ORDER):
            existing_path = rebench_out / "rebench_summary.json"
            if existing_path.exists():
                try:
                    existing = json.loads(existing_path.read_text())
                    prior_feats = existing.get("features", {}) or {}
                    for fname, fdata in prior_feats.items():
                        if fname not in selected_features and fname not in features_out:
                            features_out[fname] = fdata
                except Exception as e:
                    log.warning(f"  could not merge existing summary: {e}")

        summary["features"] = features_out
        summary["duration_s"] = time.time() - started

    finally:
        # step 7: cleanup
        if keep_tmp:
            log.info(f"  --keep-tmp: leaving {crate_path}")
        else:
            try:
                shutil.rmtree(crate_path, ignore_errors=True)
                log.info(f"  cleaned up {crate_path}")
            except Exception as e:
                log.warning(f"  tmp cleanup failed: {e}")

    (rebench_out / "rebench_summary.json").write_text(json.dumps(summary, indent=2))
    log.info(f"rebench {crate_name} done in {summary.get('duration_s', 0):.1f}s -> {rebench_out}")
    return summary


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def discover_crates_with_tests(recipes_dir: Path) -> list[str]:
    """all crate dirs under recipes/ that have non-empty tests/*.rs."""
    out: list[str] = []
    for child in sorted(recipes_dir.iterdir()):
        if not child.is_dir():
            continue
        tests = child / "tests"
        if tests.exists() and any(tests.glob("*.rs")):
            out.append(child.name)
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--crate", help="comma-separated crate names")
    ap.add_argument("--all-with-tests", action="store_true",
                    help="run on every recipes/<crate>/tests/ with at least one .rs")
    ap.add_argument("--all-csv", action="store_true",
                    help="run on every crate in cfg.csv_path (the Final Crates CSV)")
    ap.add_argument("--from-corpus", action="store_true",
                    help="run on the canonical 100-crate corpus (dir_name column "
                         f"of {CORPUS_CSV.name}); ground-truth crate list keyed to "
                         "the bootcamp dirs, with per-crate feature_label honored")
    ap.add_argument("--no-coverage", action="store_true",
                    help="skip the per-crate coverage measurement step (bench only)")
    ap.add_argument("--cpu-runs", type=int, default=3,
                    help="number of cpu_cycle_counter runs to average over (default 3)")
    ap.add_argument("--counter-runs", type=int, default=1,
                    help="how many times to run unsafe_counter, so RQ3 "
                         "through RQ5 get a spread rather than a single "
                         "reading (default 1). Cheap: about twelve minutes on "
                         "aho-corasick for both variants and both repeats. "
                         "The first run keeps the usual layout; repeats go to "
                         "<feature>/rep<N>/ and are read only by "
                         "summarise_repeats.py.")
    ap.add_argument("--heap-runs", type=int, default=1,
                    help="how many times to run heap_tracker (default 1). "
                         "Kept separate from --counter-runs because heap "
                         "tracking is far more expensive on heavy crates: it "
                         "measures 2.2 hours per run on aho-corasick, against "
                         "minutes for the counter, and that cost is the same "
                         "in both instrumentation scopes (1.04x), so it is a "
                         "property of the feature rather than of dependency "
                         "instrumentation.")
    ap.add_argument("--bin-timeout", type=int, default=900,
                    help="per-bin run timeout in seconds (default 900)")
    ap.add_argument("--keep-tmp", action="store_true",
                    help="don't delete tmp_rebench/<crate> after each crate")
    ap.add_argument("--force", action="store_true",
                    help="re-run even if rebench/<crate>/rebench_summary.json exists")
    ap.add_argument("--out-dir", default=None,
                    help="output dir (default: ROOT/rebench). Use to keep prior runs intact.")
    ap.add_argument("--features", default=None,
                    help="comma-separated subset of {unsafe_counter,heap_tracker,"
                         "cpu_cycle_counter} to run (default: all three)")
    ap.add_argument("--tmp-rebench", default=None,
                    help="working dir for crate copies (default: ROOT/tmp_rebench). "
                         "Use to isolate parallel rebench.py invocations.")
    args = ap.parse_args()

    global TMP_REBENCH
    if args.tmp_rebench:
        tp = Path(args.tmp_rebench)
        TMP_REBENCH = tp if tp.is_absolute() else (ROOT / tp)
        log.info(f"using tmp dir: {TMP_REBENCH}")

    selected_features: set[str] = set(FEATURE_ORDER)
    if args.features:
        selected_features = {f.strip() for f in args.features.split(",") if f.strip()}
        unknown = selected_features - set(FEATURE_ORDER)
        if unknown:
            ap.error(f"unknown features: {sorted(unknown)} "
                     f"(valid: {FEATURE_ORDER})")

    setup_logging()

    global REBENCH_ROOT
    if args.out_dir:
        p = Path(args.out_dir)
        REBENCH_ROOT = p if p.is_absolute() else (ROOT / p)
        log.info(f"using output dir: {REBENCH_ROOT}")

    # Must follow the out-dir handling above, so the logs land beside the run
    # they belong to rather than in the default directory.
    set_failure_log_dir(REBENCH_ROOT / "_build_failures")

    if not args.crate and not args.all_with_tests and not args.all_csv \
            and not args.from_corpus:
        ap.error("specify --crate <name[,name...]> or --all-with-tests "
                 "or --all-csv or --from-corpus")

    cfg = load_config()
    if not cfg.unsafe_perf_path:
        print("ERR: bench_runtime.unsafe_perf_path is empty in config.yaml", file=sys.stderr)
        return 1
    cfg.bench_runtime = True

    recipes_dir = ROOT / cfg.recipes_dir
    if args.from_corpus:
        import csv as _csv
        crates = [r["dir_name"].strip() for r in _csv.DictReader(CORPUS_CSV.open())
                  if r.get("dir_name", "").strip()]
        log.info(f"loaded {len(crates)} crates from corpus {CORPUS_CSV.name} "
                 f"(bootcamp dir_name ground truth)")
    elif args.all_csv:
        _, rows = load_csv(Path(cfg.csv_path) if Path(cfg.csv_path).is_absolute()
                           else ROOT / cfg.csv_path)
        crates = [r["name"].strip() for r in rows if r.get("name", "").strip()]
        log.info(f"loaded {len(crates)} crates from CSV")
    elif args.all_with_tests:
        crates = discover_crates_with_tests(recipes_dir)
    else:
        crates = [c.strip() for c in args.crate.split(",") if c.strip()]

    REBENCH_ROOT.mkdir(parents=True, exist_ok=True)

    overall: list[dict] = []
    for crate in crates:
        marker = REBENCH_ROOT / crate / "rebench_summary.json"
        if marker.exists() and not args.force:
            log.info(f"skip {crate}: {marker} exists (use --force to override)")
            continue
        try:
            res = rebench_crate(
                crate, cfg, args.cpu_runs, args.bin_timeout, args.keep_tmp,
                selected_features=selected_features,
                skip_coverage=args.no_coverage,
                counter_runs=args.counter_runs,
                heap_runs=args.heap_runs,
            )
        except KeyboardInterrupt:
            log.warning("interrupted")
            return 130
        except Exception as e:
            log.exception(f"rebench {crate} crashed: {e}")
            res = {"crate": crate, "error": f"crashed: {e}"}
        overall.append(res)

    # write a top-level index across all crates rebenched this invocation
    index_path = REBENCH_ROOT / "_runs" / f"index_{int(time.time())}.json"
    index_path.parent.mkdir(parents=True, exist_ok=True)
    index_path.write_text(json.dumps(overall, indent=2))
    log.info(f"\n{'='*60}\nrebench complete: {len(overall)} crate(s)\n"
             f"index: {index_path}\n{'='*60}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
