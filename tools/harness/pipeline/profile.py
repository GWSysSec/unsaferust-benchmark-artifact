"""
profile.py — batch driver for the agentic test-generation pipeline.

reads the seed CSV ("Final Crates - Sheet6_updated.csv") for the per-crate
baseline, filters to crates inside the target api% band, copies each crate
into a tmp working dir, drives run_pipeline (measure → generate → re-measure),
updates the CSV with post-gen api%/line%/tests-generated/iterations-used,
and emits a per-crate recipe under recipes/<crate>.json.

usage:
    python3 -m pipeline.profile                          # full run, in-band only
    python3 -m pipeline.profile --crate siphasher        # single crate (band ignored)
    python3 -m pipeline.profile --target-band 60 70      # custom band
    python3 -m pipeline.profile --measure-only           # skip generation, just baseline
    python3 -m pipeline.profile --with-description       # opt-in LLM description
    python3 -m pipeline.profile --resume                 # resume from checkpoint
"""

import argparse
import json
import logging
import re
import shutil
import sys
import threading
import time
from collections import deque
from dataclasses import asdict
from pathlib import Path

from pipeline.config import load_config, PipelineConfig
from pipeline.llm.client import LLMClient
from pipeline.runner import run_pipeline
from pipeline.tools.crate_prep import copy_crate
from pipeline.tools.csv_book import (
    load_csv, write_csv, update_row_postgen, update_row_baseline,
    baseline_in_band, baseline_values, POSTGEN_COLUMNS,
)
from pipeline.tools.recipes import Recipe, write_recipe
from pipeline.tools.workspace import discover_workspace, derive_workspace_pathway
from pipeline.tools.api_discovery import (
    ApiSurface, discover_workspace_member_apis, compute_api_coverage,
)
from pipeline.tools.coverage import run_full_coverage
from pipeline.stages import measure_coverage
from pipeline.tools.runtime_bench import run_all_features as bench_run_all_features

log = logging.getLogger(__name__)


PROJECT_ROOT = Path(__file__).resolve().parent.parent


DESCRIPTION_SYSTEM = (
    "you are a concise technical reviewer. given metadata about a rust crate's "
    "test suite, write a 2-3 sentence description focused on the test suite quality, "
    "not the crate itself. mention: what the tests cover, key strengths or gaps, "
    "and whether the suite is adequate for unsafe instrumentation benchmarking. "
    "be direct and factual. no markdown formatting. no bullet points."
)

DESCRIPTION_PROMPT = """\
Crate: {crate_name}
Type: {crate_type}
LOC: {loc}
Unsafe%: {unsafe_pct}
Test count: {test_count}
API surface: {api_total} public functions/methods
API coverage: {api_covered}/{api_total} ({api_pct:.1f}%)
Line coverage (llvm-cov): {line_cov:.1f}%
Workspace type: {ws_type}
Coverage error: {cov_error}

Write a 2-3 sentence description of this crate's test suite quality. Focus on:
- What the tests exercise and their adequacy
- Key gaps relevant to unsafe code instrumentation
- Whether the suite needs augmentation
"""


# ---------------------------------------------------------------------------
# baseline-only path: profile without running generation. mirrors the legacy
# profile_all.py behavior so we can refresh CSV pre-gen columns when needed.
# ---------------------------------------------------------------------------

def measure_baseline_only(cfg: PipelineConfig, crate_path: Path,
                          llm: LLMClient | None = None) -> dict:
    """workspace + member-aware api + llvm-cov, no generation. returns a
    dict with the same shape that the CSV update + recipe writers expect.

    delegates to `measure_coverage` so the agent-debug fallback applies here
    too — without it, `measure_baseline_only` and the iter-1 `measure_coverage`
    inside `run_pipeline` could disagree when [all-features] flakes (jni
    postmortem: baseline path read 119/192 while the agent-recovered inner
    path read 141/192 on the same crate). one measurement path = one truth.
    """
    if llm is None:
        llm = LLMClient(cfg)
    m = measure_coverage(cfg, llm, crate_path)
    ws = m.workspace

    rel_primary = (ws.primary_crate_path.relative_to(ws.path)
                   if ws.is_workspace else Path("."))
    rel_str = str(rel_primary / "tests")
    test_target_dir = rel_str if rel_str != "tests" else "tests"
    feature_args = list(m.feature_args)
    if ws.is_workspace:
        cmd_parts = ["cargo", "test", "--workspace"] + feature_args
    else:
        cmd_parts = ["cargo", "test", "-p", ws.primary_package_name] + feature_args

    cov_ok = bool(m.test_coverage and m.test_coverage.ok)
    cov_err = (m.test_coverage.error if (m.test_coverage and not cov_ok) else "") or ""

    return {
        "crate": crate_path.name,
        "primary_package_name": ws.primary_package_name,
        "primary_lib_name": ws.primary_lib_name,
        "workspace_pathway": m.workspace_pathway,
        "members_profiled": list(m.members_profiled),
        "feature_label": m.feature_label,
        "feature_args": feature_args,
        "test_target_dir": test_target_dir,
        "cargo_test_cmd": " ".join(cmd_parts),
        "api_total": m.api_total,
        "api_covered": m.api_covered,
        "api_coverage_pct": m.api_coverage_pct,
        "line_coverage_pct": m.best_coverage_pct,
        "error": "" if cov_ok else (cov_err or "coverage failed"),
    }


# ---------------------------------------------------------------------------
# description generation (kept for parity with profile_all.py; opt-in)
# ---------------------------------------------------------------------------

def generate_description(llm: LLMClient, crate_summary: dict, csv_row: dict) -> str:
    prompt = DESCRIPTION_PROMPT.format(
        crate_name=crate_summary.get("crate", ""),
        crate_type=csv_row.get("Type", ""),
        loc=csv_row.get("loc", ""),
        unsafe_pct=csv_row.get("unsafe%", ""),
        test_count=crate_summary.get("tests_total", 0),
        api_total=crate_summary.get("api_total", 0),
        api_covered=crate_summary.get("api_covered", 0),
        api_pct=float(crate_summary.get("api_coverage_pct", 0.0)),
        line_cov=float(crate_summary.get("line_coverage_pct", 0.0)),
        ws_type="workspace" if crate_summary.get("workspace_pathway") else "single crate",
        cov_error=crate_summary.get("error", "none") or "none",
    )
    try:
        desc = llm.call("fixer", DESCRIPTION_SYSTEM, prompt)
        desc = " ".join(desc.strip().strip('"').strip().split())
        if len(desc) > 600:
            desc = desc[:597] + "..."
        return desc
    except Exception as e:
        log.warning(f"  llm description failed: {e}")
        return ""


# ---------------------------------------------------------------------------
# checkpointing
# ---------------------------------------------------------------------------

def _checkpoint_path(cfg: PipelineConfig) -> Path:
    p = Path(cfg.checkpoint_path)
    return p if p.is_absolute() else PROJECT_ROOT / p


def load_checkpoint(cfg: PipelineConfig) -> dict[str, dict]:
    p = _checkpoint_path(cfg)
    if not p.exists():
        return {}
    try:
        return json.loads(p.read_text())
    except Exception as e:
        log.warning(f"failed to load checkpoint: {e}")
        return {}


def save_checkpoint(cfg: PipelineConfig, state: dict[str, dict]):
    p = _checkpoint_path(cfg)
    p.write_text(json.dumps(state, indent=2))


# ---------------------------------------------------------------------------
# per-crate timeout
# ---------------------------------------------------------------------------

def _run_with_timeout(fn, timeout_s: int, *args, **kwargs):
    """run fn(*args) on a daemon thread with a wall-clock timeout. caveats:
    cargo subprocesses spawned inside fn are NOT killed when the timeout
    fires — we just stop waiting and move on, leaking the subtree.
    """
    box: dict = {}

    def runner():
        try:
            box["result"] = fn(*args, **kwargs)
        except Exception as e:
            box["error"] = e

    t = threading.Thread(target=runner, daemon=True)
    t.start()
    t.join(timeout=timeout_s)
    if t.is_alive():
        return None, f"timeout (>{timeout_s}s)"
    if "error" in box:
        return None, str(box["error"])
    # _process_crate already returns (summary, error_string); flatten so callers
    # can `summary, err = _run_with_timeout(...)` without re-nesting.
    res = box.get("result")
    if isinstance(res, tuple) and len(res) == 2:
        return res
    return res, ""


# ---------------------------------------------------------------------------
# per-crate driver
# ---------------------------------------------------------------------------

def _select_crates(args, rows: list[dict], cfg: PipelineConfig) -> list[dict]:
    if args.crate:
        for r in rows:
            if r.get("name", "").strip() == args.crate:
                return [r]
        log.error(f"crate {args.crate!r} not found in CSV")
        sys.exit(1)

    lo = args.target_band[0] if args.target_band else cfg.target_band_min
    hi = args.target_band[1] if args.target_band else cfg.target_band_max
    if args.no_band:
        return rows
    selected = [r for r in rows if baseline_in_band(r, lo, hi)]
    log.info(f"target band [{lo}, {hi}]%: {len(selected)} crates of {len(rows)} in band")
    return selected


def _build_recipe_from_summary(summary: dict, baseline: tuple[float, float] | None) -> Recipe:
    """build a Recipe given a run_pipeline summary + the CSV baseline (if any)."""
    pre_api, pre_loc = baseline if baseline else (summary["api_coverage_pct"],
                                                   summary["line_coverage_pct"])
    api_total = summary["api_total"]
    api_pct_post = summary["api_coverage_pct"]
    line_pct_post = summary["line_coverage_pct"]

    return Recipe(
        crate=summary["crate"],
        primary_package_name=summary.get("primary_package_name", ""),
        primary_lib_name=summary.get("primary_lib_name", ""),
        workspace_pathway=summary.get("workspace_pathway", ""),
        members_profiled=list(summary.get("members_profiled", [])),
        feature_label=summary.get("feature_label", ""),
        feature_args=list(summary.get("feature_args", [])),
        test_target_dir=summary.get("test_target_dir", ""),
        cargo_test_cmd=summary.get("cargo_test_cmd", ""),
        api_total=api_total,
        api_covered=int(round(pre_api / 100.0 * api_total)) if api_total else 0,
        api_pct=pre_api,
        line_cov_pct=pre_loc,
        api_covered_post=summary["api_covered"],
        api_pct_post=api_pct_post,
        line_cov_pct_post=line_pct_post,
        tests_generated=summary["tests_generated"],
        iterations_used=summary["iterations"],
        # macro-only crates have api_total=0 — don't flag them.
        needs_test_generation=(api_total > 0 and api_pct_post < 80.0),
    )


def _bench_out_dir(cfg: PipelineConfig, recipes_dir: Path, crate_name: str, label: str) -> Path:
    if cfg.bench_runtime_out_dir:
        return Path(cfg.bench_runtime_out_dir) / crate_name / label
    return Path(recipes_dir) / crate_name / "runtime" / label


def _summarize_bench(bench) -> dict:
    """compact stats for the run summary / recipe payload. one entry per
    feature with a nested per-(native-lib)-variant block.
    """
    out = {"label": bench.label, "out_dir": bench.out_dir, "features": {}}
    for fname, fres in bench.features.items():
        feat: dict = {"error": fres.error, "variants": {}}
        for vname, vres in fres.variants.items():
            feat["variants"][vname] = {
                "native_lib": vres.native_lib,
                "ok": vres.ok,
                "error": vres.error,
                "bins": [
                    {"name": b.name, "exit": b.exit_code, "stat_files": b.stat_files}
                    for b in vres.bins
                ],
            }
        out["features"][fname] = feat
    return out


def _run_bench_phase(
    cfg: PipelineConfig, primary_crate_path: Path, crate_name: str,
    recipes_dir: Path, label: str,
) -> dict | None:
    """thin wrapper: pick out dir, drive the bench, return a compact summary.

    `primary_crate_path` MUST point at the dir whose Cargo.toml has [package]
    (the primary member in a workspace). Patching a virtual workspace
    manifest would break cargo on the next invocation.
    """
    if not cfg.bench_runtime:
        return None
    if not cfg.unsafe_perf_path:
        log.warning("  [bench] cfg.unsafe_perf_path is empty; skipping")
        return None
    out_dir = _bench_out_dir(cfg, recipes_dir, crate_name, label)
    log.info(f"  [bench] {label}: writing to {out_dir}")
    bench = bench_run_all_features(
        primary_crate_path, cfg, out_dir, label=label,
    )
    return _summarize_bench(bench)


def _process_crate(
    cfg: PipelineConfig, crate_name: str, csv_row: dict,
    bootcamp: Path, tmp_dir: Path, recipes_dir: Path,
    measure_only: bool, llm: LLMClient | None,
    skip_description: bool,
) -> tuple[dict, str]:
    """end-to-end for one crate. returns (summary_or_none, error_string)."""
    crate_path = copy_crate(bootcamp, crate_name, tmp_dir)

    # resolve the primary crate path once. workspaces with virtual manifests
    # need this — patching the workspace root's Cargo.toml is illegal (it has
    # no [package] section, so [dev-dependencies] won't parse).
    from pipeline.tools.workspace import discover_workspace
    ws_info = discover_workspace(crate_path) if cfg.bench_runtime else None
    primary_path = ws_info.primary_crate_path if ws_info else crate_path

    if measure_only:
        baseline = measure_baseline_only(cfg, crate_path)
        # adapt baseline keys into a run_pipeline-style summary
        summary = {
            "crate": crate_name,
            "api_coverage_pct": baseline["api_coverage_pct"],
            "api_covered": baseline["api_covered"],
            "api_total": baseline["api_total"],
            "line_coverage_pct": baseline["line_coverage_pct"],
            "tests_total": 0,
            "tests_generated": 0,
            "tests_failed": 0,
            "iterations": 0,
            "evaluation_action": "MEASURE_ONLY",
            "members_profiled": baseline["members_profiled"],
            "workspace_pathway": baseline["workspace_pathway"],
            "feature_label": baseline["feature_label"],
            "feature_args": baseline["feature_args"],
            "test_target_dir": baseline["test_target_dir"],
            "cargo_test_cmd": baseline["cargo_test_cmd"],
            "primary_package_name": baseline["primary_package_name"],
            "primary_lib_name": baseline["primary_lib_name"],
        }
        # measure-only: only the "before" snapshot is meaningful — no generation
        # happens, so there's no "after" to compare against.
        bench_before = _run_bench_phase(
            cfg, primary_path, crate_name, recipes_dir, "before",
        )
        if bench_before:
            summary["bench_before"] = bench_before
        return summary, baseline.get("error", "")

    summary = run_pipeline(
        cfg, crate_path, crate_name=crate_name,
        bootcamp_dir=bootcamp, recipes_dir=recipes_dir,
    )

    # bench AFTER: only when post-gen api% clears the gate (default 0 = always).
    if cfg.bench_runtime:
        api_pct = float(summary.get("api_coverage_pct", 0.0))
        if api_pct >= cfg.bench_runtime_min_api_pct:
            bench_after = _run_bench_phase(
                cfg, primary_path, crate_name, recipes_dir, "after",
            )
            if bench_after:
                summary["bench_after"] = bench_after
        else:
            log.info(f"  [bench] skipping after-bench: api_pct={api_pct:.1f}% "
                     f"< min={cfg.bench_runtime_min_api_pct}%")

    # optional description
    if llm is not None and not skip_description:
        log.info(f"  generating description...")
        desc = generate_description(llm, summary, csv_row)
        summary["description"] = desc

    return summary, ""


def main():
    parser = argparse.ArgumentParser(description="batch-drive the test-gen pipeline")
    parser.add_argument("--crate", type=str, default=None,
                        help="run a single crate by name")
    parser.add_argument("--target-band", type=float, nargs=2, metavar=("MIN", "MAX"),
                        default=None,
                        help="api%% baseline band [MIN, MAX] for crate selection "
                             "(default from cfg.target_band_min/max)")
    parser.add_argument("--no-band", action="store_true",
                        help="ignore baseline band and run on every crate in CSV")
    parser.add_argument("--measure-only", action="store_true",
                        help="skip generation; refresh baseline columns only")
    parser.add_argument("--with-description", action="store_true",
                        help="enable LLM description generation (off by default)")
    parser.add_argument("--coverage-target", type=float, default=None,
                        help="override cfg.coverage_target")
    parser.add_argument("--max-iterations", type=int, default=None,
                        help="override cfg.max_coverage_iterations")
    parser.add_argument("--max-batches", type=int, default=None,
                        help="override cfg.max_batches_per_iteration")
    parser.add_argument("--per-crate-timeout", type=int, default=None,
                        help="override cfg.per_crate_timeout_s")
    parser.add_argument("--config", type=str, default=None)
    parser.add_argument("--bench-runtime", action="store_true",
                        help="run unsafe-perf runtime bench before+after gen "
                             "(after the test-gen phase 4) and harvest /tmp/*.json "
                             "stat files into recipes/<crate>/runtime/{before,after}/")
    parser.add_argument("--bench-runtime-min-pct", type=float, default=None,
                        help="only run the after-bench when post-gen api%% >= this "
                             "threshold (default 0; gate is per-crate, not batch-wide)")
    parser.add_argument("--bench-runtime-out", type=str, default=None,
                        help="absolute dir to dump bench output into (overrides "
                             "the default recipes/<crate>/runtime path)")
    parser.add_argument("--unsafe-perf-path", type=str, default=None,
                        help="override cfg.unsafe_perf_path")
    parser.add_argument("--resume", action="store_true",
                        help="resume from checkpoint, skipping completed crates")
    parser.add_argument("--dry-run", action="store_true",
                        help="run pipeline but don't write CSV/recipes")
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args()

    level = logging.DEBUG if args.verbose else logging.INFO
    logging.basicConfig(level=level, format="%(asctime)s %(levelname)-5s %(message)s",
                        datefmt="%H:%M:%S")

    cfg = load_config(args.config)

    # cli overrides
    if args.coverage_target is not None:
        cfg.coverage_target = args.coverage_target
    if args.max_iterations is not None:
        cfg.max_coverage_iterations = args.max_iterations
    if args.max_batches is not None:
        cfg.max_batches_per_iteration = args.max_batches
    if args.per_crate_timeout is not None:
        cfg.per_crate_timeout_s = args.per_crate_timeout
    if args.with_description:
        cfg.skip_description = False
    if args.bench_runtime:
        cfg.bench_runtime = True
    if args.bench_runtime_min_pct is not None:
        cfg.bench_runtime_min_api_pct = args.bench_runtime_min_pct
    if args.bench_runtime_out is not None:
        cfg.bench_runtime_out_dir = args.bench_runtime_out
    if args.unsafe_perf_path is not None:
        cfg.unsafe_perf_path = args.unsafe_perf_path

    bootcamp = Path(cfg.bootcamp_dir)
    if not bootcamp.exists():
        log.error(f"bootcamp directory not found: {bootcamp}")
        sys.exit(1)

    tmp_dir = Path(cfg.tmp_dir)
    if not tmp_dir.is_absolute():
        tmp_dir = PROJECT_ROOT / tmp_dir
    tmp_dir.mkdir(parents=True, exist_ok=True)

    csv_path = Path(cfg.csv_path)
    if not csv_path.is_absolute():
        csv_path = PROJECT_ROOT / csv_path

    recipes_dir = Path(cfg.recipes_dir)
    if not recipes_dir.is_absolute():
        recipes_dir = PROJECT_ROOT / recipes_dir
    recipes_dir.mkdir(parents=True, exist_ok=True)

    fieldnames, rows = load_csv(csv_path)
    csv_by_name = {r["name"].strip(): r for r in rows}
    log.info(f"loaded {len(rows)} crates from {csv_path.name}")

    selected = _select_crates(args, rows, cfg)
    if not selected:
        log.warning("no crates matched selection; nothing to do")
        return

    state = load_checkpoint(cfg) if args.resume else {}
    log.info(f"checkpoint has {len(state)} prior entries")

    llm: LLMClient | None = None
    if args.with_description:
        llm = LLMClient(cfg)

    total = len(selected)
    for idx, row in enumerate(selected):
        crate_name = row["name"].strip()
        if args.resume and crate_name in state and not state[crate_name].get("error"):
            log.info(f"[{idx+1}/{total}] skipping {crate_name} (checkpointed)")
            continue

        log.info(f"\n{'='*60}")
        log.info(f"[{idx+1}/{total}] processing: {crate_name}")
        log.info(f"{'='*60}")

        baseline = baseline_values(row)
        t0 = time.time()
        summary, err = _run_with_timeout(
            _process_crate,
            cfg.per_crate_timeout_s,
            cfg, crate_name, row,
            bootcamp, tmp_dir, recipes_dir,
            args.measure_only, llm, cfg.skip_description,
        )

        if summary is None:
            log.error(f"  FAILED: {err}")
            state[crate_name] = {"error": err}
            save_checkpoint(cfg, state)
            _cleanup_tmp(tmp_dir, crate_name)
            continue

        # at this point the per-crate work succeeded (or non-fatal cov error)
        elapsed = time.time() - t0
        log.info(f"  completed in {elapsed:.1f}s: api={summary['api_coverage_pct']:.1f}%, "
                 f"line={summary['line_coverage_pct']:.1f}%, "
                 f"tests_generated={summary['tests_generated']}, "
                 f"iters={summary['iterations']}")

        # update CSV row in-place
        if args.measure_only:
            update_row_baseline(
                row,
                api_pct=summary["api_coverage_pct"],
                line_cov_pct=summary["line_coverage_pct"],
                workspace_pathway=summary.get("workspace_pathway", ""),
                description=summary.get("description", ""),
            )
        else:
            update_row_postgen(
                row,
                api_pct_post=summary["api_coverage_pct"],
                line_cov_pct_post=summary["line_coverage_pct"],
                tests_generated=summary["tests_generated"],
                iterations_used=summary["iterations"],
            )
            if summary.get("description"):
                row["Description"] = summary["description"]

        # write recipe
        if not args.dry_run:
            recipe = _build_recipe_from_summary(summary, baseline)
            try:
                write_recipe(recipe, recipes_dir)
            except Exception as e:
                log.warning(f"  failed to write recipe: {e}")

        # write CSV after every crate so partial progress survives a crash
        if not args.dry_run:
            try:
                write_csv(csv_path, fieldnames, rows)
            except Exception as e:
                log.warning(f"  failed to write csv: {e}")

        state[crate_name] = {
            "api_pct_post": summary["api_coverage_pct"],
            "line_cov_pct_post": summary["line_coverage_pct"],
            "tests_generated": summary["tests_generated"],
            "iterations": summary["iterations"],
            "error": "",
        }
        save_checkpoint(cfg, state)

        _cleanup_tmp(tmp_dir, crate_name)

    # summary
    log.info(f"\n{'='*60}")
    log.info("BATCH SUMMARY")
    log.info(f"{'='*60}")
    succeeded = sum(1 for v in state.values() if not v.get("error"))
    failed = sum(1 for v in state.values() if v.get("error"))
    log.info(f"  succeeded: {succeeded}, failed: {failed}")


def _cleanup_tmp(tmp_dir: Path, crate_name: str):
    p = Path(tmp_dir) / crate_name
    if p.exists():
        try:
            shutil.rmtree(p)
        except Exception as e:
            log.warning(f"  cleanup failed: {e}")


# ---------------------------------------------------------------------------
# bootcamp-iteration mode: no CSV. walk bootcamp_dir, measure baseline, gate
# generation by a per-crate api% threshold (default 50%), bench AFTER on
# successfully generated crates. recipes are the only output artifact.
# ---------------------------------------------------------------------------

def _list_bootcamp_crates(bootcamp: Path) -> list[str]:
    """every immediate sub-dir of bootcamp that has a Cargo.toml at the root
    or one level down (workspace members live in <crate>/<member>/).

    skips backup/legacy variants (e.g. `fs4.v0.10.0_old`) — names ending in
    `_old`/`_legacy`/`_backup` or containing a `.vN.N.N` version segment.
    """
    out: list[str] = []
    legacy_re = re.compile(r"(_old$|_legacy$|_backup$|\.v\d+\.\d+\.\d+)")
    for entry in sorted(bootcamp.iterdir()):
        if not entry.is_dir():
            continue
        if entry.name.startswith("."):
            continue
        if legacy_re.search(entry.name):
            log.debug(f"  skipping legacy/backup dir: {entry.name}")
            continue
        # any Cargo.toml within 2 levels — covers single crates and workspaces
        has_cargo = (entry / "Cargo.toml").exists() or any(
            (sub / "Cargo.toml").exists() for sub in entry.iterdir() if sub.is_dir()
        )
        if has_cargo:
            out.append(entry.name)
    return out


def _build_recipe_from_baseline(baseline: dict) -> Recipe:
    """build a Recipe from a measure-only baseline (no generation happened —
    pre and post snapshots are identical)."""
    api_total = baseline.get("api_total", 0)
    api_covered = baseline.get("api_covered", 0)
    api_pct = baseline.get("api_coverage_pct", 0.0)
    line_pct = baseline.get("line_coverage_pct", 0.0)
    return Recipe(
        crate=baseline["crate"],
        primary_package_name=baseline.get("primary_package_name", ""),
        primary_lib_name=baseline.get("primary_lib_name", ""),
        workspace_pathway=baseline.get("workspace_pathway", ""),
        members_profiled=list(baseline.get("members_profiled", [])),
        feature_label=baseline.get("feature_label", ""),
        feature_args=list(baseline.get("feature_args", [])),
        test_target_dir=baseline.get("test_target_dir", ""),
        cargo_test_cmd=baseline.get("cargo_test_cmd", ""),
        api_total=api_total,
        api_covered=api_covered,
        api_pct=api_pct,
        line_cov_pct=line_pct,
        api_covered_post=api_covered,
        api_pct_post=api_pct,
        line_cov_pct_post=line_pct,
        tests_generated=0,
        iterations_used=0,
        needs_test_generation=(api_total > 0 and api_pct < 80.0),
        error=baseline.get("error", "") or "",
    )


def _baseline_to_summary(crate_name: str, baseline: dict, action: str) -> dict:
    """adapt a measure_baseline_only() result to a run_pipeline-style summary,
    so the same downstream code can pretty-print + checkpoint + write recipes.
    """
    return {
        "crate": crate_name,
        "api_coverage_pct": baseline.get("api_coverage_pct", 0.0),
        "api_covered": baseline.get("api_covered", 0),
        "api_total": baseline.get("api_total", 0),
        "line_coverage_pct": baseline.get("line_coverage_pct", 0.0),
        "tests_total": 0,
        "tests_generated": 0,
        "tests_failed": 0,
        "iterations": 0,
        "evaluation_action": action,
        "members_profiled": baseline.get("members_profiled", []),
        "workspace_pathway": baseline.get("workspace_pathway", ""),
        "feature_label": baseline.get("feature_label", ""),
        "feature_args": baseline.get("feature_args", []),
        "test_target_dir": baseline.get("test_target_dir", ""),
        "cargo_test_cmd": baseline.get("cargo_test_cmd", ""),
        "primary_package_name": baseline.get("primary_package_name", ""),
        "primary_lib_name": baseline.get("primary_lib_name", ""),
    }


def _process_crate_bootcamp(
    cfg: PipelineConfig, crate_name: str,
    bootcamp: Path, tmp_dir: Path, recipes_dir: Path,
    api_min: float, gen_target: float = 80.0,
) -> tuple[dict, dict | None, str]:
    """end-to-end for one bootcamp crate. returns (summary, baseline_dict, err).

    three-band flow keyed on baseline api%:
      < api_min        : measure-only; no gen, no bench (recipe captures cov)
      [api_min, target): gen + bench (full pipeline)
      >= target        : skip gen, bench only (recipe captures cov + runtime)
    """
    crate_path = copy_crate(bootcamp, crate_name, tmp_dir)

    # workspace info needed to point bench at the primary [package] dir
    from pipeline.tools.workspace import discover_workspace
    ws_info = discover_workspace(crate_path) if cfg.bench_runtime else None
    primary_path = ws_info.primary_crate_path if ws_info else crate_path

    log.info(f"  measuring baseline...")
    baseline = measure_baseline_only(cfg, crate_path)
    pre_api = baseline.get("api_coverage_pct", 0.0)
    pre_line = baseline.get("line_coverage_pct", 0.0)
    api_total = baseline.get("api_total", 0)
    log.info(
        f"  baseline: api={pre_api:.1f}% ({baseline.get('api_covered',0)}/{api_total}), "
        f"line={pre_line:.1f}%"
    )

    # band 1: below api_min — record baseline only, no bench.
    if api_total == 0 or pre_api < api_min:
        reason = "api_total=0" if api_total == 0 else f"pre_api {pre_api:.1f}% < min {api_min:.1f}%"
        log.info(f"  band=BELOW_MIN ({reason}); cov-only recipe, no bench")
        action = "BELOW_MIN" if api_total else "NO_API"
        return _baseline_to_summary(crate_name, baseline, action), baseline, baseline.get("error", "") or ""

    # band 3: at-or-above gen target — skip gen, bench only.
    if pre_api >= gen_target:
        log.info(f"  band=AT_TARGET (pre_api {pre_api:.1f}% >= {gen_target:.1f}%); "
                 f"skipping gen, running bench only")
        summary = _baseline_to_summary(crate_name, baseline, "AT_TARGET")
        if cfg.bench_runtime:
            bench_after = _run_bench_phase(
                cfg, primary_path, crate_name, recipes_dir, "after",
            )
            if bench_after:
                summary["bench_after"] = bench_after
        return summary, baseline, ""

    # band 2: in-band — full pipeline (gen + bench). run_pipeline does its own
    # measure→generate→measure; the first measure inside is redundant with our
    # baseline but cheap.
    log.info(f"  band=IN_BAND (api_min {api_min:.1f}% <= {pre_api:.1f}% < {gen_target:.1f}%); "
             f"gen + bench")
    summary = run_pipeline(
        cfg, crate_path, crate_name=crate_name,
        bootcamp_dir=bootcamp, recipes_dir=recipes_dir,
    )

    # daily-quota guard: if 429 fired before any test got generated, treat as
    # deferred — caller (bootcamp_main) requeues without checkpointing so we
    # come back to this crate after the quota refills.
    if summary.get("quota_exhausted") and summary.get("tests_generated", 0) == 0:
        log.warning(f"  quota exhausted before any test generated — deferring {crate_name}")
        return (
            _baseline_to_summary(crate_name, baseline, "QUOTA_DEFERRED"),
            baseline,
            "quota_exhausted",
        )

    # regression guard: a broken generated test can shadow the rest of the test
    # suite during the final measure pass (jni's gen_jni_JNIEnv_part1 was the
    # canonical example — passed isolated compile, then triggered "27 previous
    # errors" under `cargo llvm-cov --tests` and dragged post-cov to 14.6% from
    # a 73.4% baseline). when post collapses to <50% of pre, the generated
    # tests are not net-positive; wipe them and roll the recipe back to the
    # baseline so we don't ship a regression.
    api_pct_post = float(summary.get("api_coverage_pct", 0.0))
    if api_pct_post < pre_api * 0.5:
        log.warning(
            f"  post-cov {api_pct_post:.1f}% < 50% of pre-cov {pre_api:.1f}% — "
            f"generated tests are net-negative; discarding tests and using baseline"
        )
        tests_dir = Path(recipes_dir) / crate_name / "tests"
        if tests_dir.exists():
            shutil.rmtree(tests_dir, ignore_errors=True)
        return _baseline_to_summary(crate_name, baseline, "REGRESSED"), baseline, ""

    if summary.get("quota_exhausted"):
        log.warning(
            f"  quota exhausted mid-gen; persisting partial "
            f"({summary.get('tests_generated', 0)} tests) and continuing"
        )

    if cfg.bench_runtime:
        api_pct = float(summary.get("api_coverage_pct", 0.0))
        if api_pct >= cfg.bench_runtime_min_api_pct:
            bench_after = _run_bench_phase(
                cfg, primary_path, crate_name, recipes_dir, "after",
            )
            if bench_after:
                summary["bench_after"] = bench_after
        else:
            log.info(f"  [bench] skipping after-bench: api_pct={api_pct:.1f}% "
                     f"< min={cfg.bench_runtime_min_api_pct}%")

    return summary, baseline, ""


def bootcamp_main():
    """walk bootcamp_dir; for each crate run baseline → (gen if api>=min) →
    bench. no CSV reads/writes — recipes/<crate>.json is the sole output.
    """
    parser = argparse.ArgumentParser(
        prog="python -m pipeline bootcamp",
        description="iterate every crate in bootcamp_dir, no CSV bookkeeping"
    )
    parser.add_argument("--crate", type=str, default=None,
                        help="run a single crate by name (skips others)")
    parser.add_argument("--api-pct-min", type=float, default=50.0,
                        help="generate tests only when baseline api%% >= this "
                             "(default 50; below this the recipe records the "
                             "baseline and moves on)")
    parser.add_argument("--coverage-target", type=float, default=None)
    parser.add_argument("--max-iterations", type=int, default=None)
    parser.add_argument("--max-batches", type=int, default=None)
    parser.add_argument("--per-crate-timeout", type=int, default=None,
                        help="hard wall-clock cap per crate, end-to-end "
                             "(default cfg.per_crate_timeout_s)")
    parser.add_argument("--config", type=str, default=None)
    parser.add_argument("--bench-runtime", action="store_true", default=True,
                        help="run unsafe-perf runtime bench AFTER (default ON "
                             "in bootcamp mode; set --no-bench-runtime to skip)")
    parser.add_argument("--no-bench-runtime", action="store_false",
                        dest="bench_runtime",
                        help="disable post-gen bench (overrides default)")
    parser.add_argument("--bench-runtime-min-pct", type=float, default=None)
    parser.add_argument("--bench-runtime-out", type=str, default=None)
    parser.add_argument("--unsafe-perf-path", type=str, default=None)
    parser.add_argument("--resume", action="store_true",
                        help="resume from checkpoint, skipping completed crates")
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args(sys.argv[2:] if sys.argv[1:2] == ["bootcamp"] else None)

    level = logging.DEBUG if args.verbose else logging.INFO
    logging.basicConfig(level=level, format="%(asctime)s %(levelname)-5s %(message)s",
                        datefmt="%H:%M:%S")

    cfg = load_config(args.config)
    if args.coverage_target is not None:
        cfg.coverage_target = args.coverage_target
    if args.max_iterations is not None:
        cfg.max_coverage_iterations = args.max_iterations
    if args.max_batches is not None:
        cfg.max_batches_per_iteration = args.max_batches
    if args.per_crate_timeout is not None:
        cfg.per_crate_timeout_s = args.per_crate_timeout
    if args.bench_runtime:
        cfg.bench_runtime = True
    else:
        cfg.bench_runtime = False
    if args.bench_runtime_min_pct is not None:
        cfg.bench_runtime_min_api_pct = args.bench_runtime_min_pct
    if args.bench_runtime_out is not None:
        cfg.bench_runtime_out_dir = args.bench_runtime_out
    if args.unsafe_perf_path is not None:
        cfg.unsafe_perf_path = args.unsafe_perf_path
    # bootcamp mode does not generate descriptions
    cfg.skip_description = True

    bootcamp = Path(cfg.bootcamp_dir)
    if not bootcamp.exists():
        log.error(f"bootcamp directory not found: {bootcamp}")
        sys.exit(1)

    tmp_dir = Path(cfg.tmp_dir)
    if not tmp_dir.is_absolute():
        tmp_dir = PROJECT_ROOT / tmp_dir
    tmp_dir.mkdir(parents=True, exist_ok=True)

    recipes_dir = Path(cfg.recipes_dir)
    if not recipes_dir.is_absolute():
        recipes_dir = PROJECT_ROOT / recipes_dir
    recipes_dir.mkdir(parents=True, exist_ok=True)

    if args.crate:
        crates = [args.crate]
    else:
        crates = _list_bootcamp_crates(bootcamp)
    log.info(f"bootcamp at {bootcamp}: {len(crates)} crate(s) to process")
    log.info(f"api_pct_min={args.api_pct_min}, coverage_target={cfg.coverage_target}, "
             f"bench_runtime={cfg.bench_runtime}")

    state = load_checkpoint(cfg) if args.resume else {}
    log.info(f"checkpoint has {len(state)} prior entries")

    # timed-out crates get requeued at the end and retried up to
    # _MAX_TIMEOUT_RETRIES times before being recorded as a permanent failure.
    # this gives transient issues (gateway storms, contention) a second chance
    # without burning the per-crate timeout twice in a row.
    _MAX_TIMEOUT_RETRIES = 1
    timeout_retries: dict[str, int] = {}

    # quota-deferred crates (429 daily-limit hit before any test got generated)
    # are requeued without persisting to the checkpoint, so we revisit them
    # after the queue drains (and ideally after the quota window resets). a
    # generous cap so the loop doesn't degenerate if the quota stays exhausted.
    _MAX_QUOTA_RETRIES = 20
    quota_retries: dict[str, int] = {}

    total = len(crates)
    queue: deque[str] = deque(crates)
    processed = 0
    while queue:
        crate_name = queue.popleft()
        processed += 1
        if args.resume and crate_name in state and not state[crate_name].get("error"):
            log.info(f"[{processed}/{total}+{len(queue)}] skipping {crate_name} (checkpointed)")
            continue

        attempt_n = timeout_retries.get(crate_name, 0)
        q_attempt = quota_retries.get(crate_name, 0)
        tags = []
        if attempt_n:
            tags.append(f"timeout-retry {attempt_n}")
        if q_attempt:
            tags.append(f"quota-retry {q_attempt}")
        attempt_tag = f" ({', '.join(tags)})" if tags else ""
        log.info(f"\n{'='*60}")
        log.info(f"[{processed}/{total}+{len(queue)}] processing: {crate_name}{attempt_tag}")
        log.info(f"{'='*60}")

        t0 = time.time()
        result, err = _run_with_timeout(
            _process_crate_bootcamp,
            cfg.per_crate_timeout_s,
            cfg, crate_name, bootcamp, tmp_dir, recipes_dir, args.api_pct_min,
        )

        if result is None:
            log.error(f"  FAILED: {err}")
            is_timeout = isinstance(err, str) and err.startswith("timeout")
            if is_timeout and attempt_n < _MAX_TIMEOUT_RETRIES:
                timeout_retries[crate_name] = attempt_n + 1
                queue.append(crate_name)
                log.warning(f"  requeued (timeout retry {attempt_n + 1}/{_MAX_TIMEOUT_RETRIES})")
            else:
                state[crate_name] = {"error": err}
                save_checkpoint(cfg, state)
            _cleanup_tmp(tmp_dir, crate_name)
            continue

        # _process_crate_bootcamp returns (summary, baseline, err); _run_with_timeout
        # auto-flattens 2-tuples but our payload is a 3-tuple, so it lands as-is.
        summary, baseline, inner_err = result

        # quota-deferred: skip persistence, requeue at end of stack. on cap,
        # write a sticky error so we don't loop forever.
        if inner_err == "quota_exhausted":
            if q_attempt < _MAX_QUOTA_RETRIES:
                quota_retries[crate_name] = q_attempt + 1
                queue.append(crate_name)
                log.warning(
                    f"  quota deferred — requeued at end of stack "
                    f"({q_attempt + 1}/{_MAX_QUOTA_RETRIES})"
                )
            else:
                state[crate_name] = {"error": f"quota exhausted, {_MAX_QUOTA_RETRIES} requeues hit"}
                save_checkpoint(cfg, state)
                log.error(f"  quota retry cap hit — recorded as permanent failure")
            _cleanup_tmp(tmp_dir, crate_name)
            continue

        elapsed = time.time() - t0
        log.info(
            f"  completed in {elapsed:.1f}s: api={summary['api_coverage_pct']:.1f}%, "
            f"line={summary['line_coverage_pct']:.1f}%, "
            f"tests_generated={summary['tests_generated']}, "
            f"iters={summary['iterations']}"
        )

        # build + write recipe with explicit (pre, post) snapshots
        if baseline is not None and summary.get("tests_generated", 0) > 0:
            pre_api = baseline.get("api_coverage_pct", 0.0)
            pre_line = baseline.get("line_coverage_pct", 0.0)
            recipe = _build_recipe_from_summary(summary, (pre_api, pre_line))
        elif baseline is not None:
            # generation skipped or yielded no tests — pre == post
            recipe = _build_recipe_from_baseline(baseline)
        else:
            recipe = _build_recipe_from_summary(summary, None)

        try:
            write_recipe(recipe, recipes_dir)
        except Exception as e:
            log.warning(f"  failed to write recipe: {e}")

        state[crate_name] = {
            "api_pct_pre": baseline.get("api_coverage_pct", 0.0) if baseline else 0.0,
            "api_pct_post": summary["api_coverage_pct"],
            "line_cov_pct_post": summary["line_coverage_pct"],
            "tests_generated": summary["tests_generated"],
            "iterations": summary["iterations"],
            "error": inner_err or "",
        }
        save_checkpoint(cfg, state)
        _cleanup_tmp(tmp_dir, crate_name)

    log.info(f"\n{'='*60}")
    log.info("BOOTCAMP RUN SUMMARY")
    log.info(f"{'='*60}")
    succeeded = sum(1 for v in state.values() if not v.get("error"))
    failed = sum(1 for v in state.values() if v.get("error"))
    log.info(f"  succeeded: {succeeded}, failed: {failed}")


if __name__ == "__main__":
    main()
