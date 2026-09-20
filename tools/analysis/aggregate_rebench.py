"""Aggregate per-bin rebench JSONs into per-crate stats.

Sources data from rusttest-gen/rebench_v2/<crate>/<feature>/<variant>/*.json.
Per-bin JSONs follow the nested schema: top-level {schema, pid, binary,
metric, stats: {...counters...}}; cpu_cycle is read from the per-crate
avg.json rolled up across run1/run2/run3. Failed bins (exit_code != 0 in
rebench_summary.json) are excluded.

CPU cycle metrics are restricted to *internal* cycles, i.e.:
  internal_total_cycles   = total_cycles         - external_cycles
  internal_unsafe_cycles  = unsafe_cycles_total  - min(unsafe_cycles_external,
                                                       unsafe_cycles_total)
"""

from __future__ import annotations

import json
import os
import re
from collections import defaultdict
from pathlib import Path

# strip cargo's 16-hex compile-hash suffix to recover the logical bin name.
# bins compiled under different RUSTFLAGS (e.g. with_native vs without_native)
# get different hashes, so logical names are how we match across variants.
_HASH_SUFFIX_RE = re.compile(r"-[0-9a-f]{16}$")


def logical_bin_name(bin_name: str) -> str:
    """Strip cargo's hash suffix: 'rt_threaded-194917f9b31de8a6' -> 'rt_threaded'."""
    return _HASH_SUFFIX_RE.sub("", bin_name)


REBENCH_ROOT = Path(
    os.environ.get(
        "REBENCH_ROOT",
        "/home/oscar/Projects/unsafebench/rusttest-gen/rebench_v2",
    )
)

UNSAFE_COUNTER_FIELDS = [
    "total_instructions",
    "unsafe_instructions",
    "unsafe_loads",
    "unsafe_stores",
    "unsafe_calls_direct",
    "unsafe_calls_indirect",
    "unsafe_calls_intrinsic",
    "unsafe_casts",
    "unsafe_geps",
    "unsafe_atomics",
    "unsafe_others",
    "total_functions_defined",
    "unsafe_functions_defined",
    "total_functions_executed",
    "unsafe_functions_executed",
    "unsafe_functions_with_executed_insts",
    "total_function_calls",
    "unsafe_function_calls",
]

# Counters added after the 2026-09 runs were measured. They are kept out of
# UNSAFE_COUNTER_FIELDS on purpose: summing a key no stat file has would add a
# zero-valued field to every older dataset, which would make the committed
# per-question JSONs change without any new measurement behind it. These are
# summed only when at least one stat file in a directory actually carries them,
# so an older run stays exactly as it was and a newer run gains the field.
#
# total_calls counts every call the program itself makes, and
# total_calls_indirect the subset made through a function pointer. Both cover
# whole basic blocks rather than unsafe spans, unlike unsafe_calls_indirect,
# so total_calls_indirect / total_calls is the share of the program's calling
# that an analysis would have to resolve indirectly. Inline asm, LLVM
# intrinsics and the instrumentation's own hooks are excluded.
OPTIONAL_COUNTER_FIELDS = [
    "total_calls",
    "total_calls_indirect",
]

HEAP_SCALAR_FIELDS = [
    "total_heap_usage",
    "total_heap_alloc",
    "total_heap_realloc",
    "total_heap_dealloc",
    "unsafe_heap_memory",
    "unsafe_heap_objects",
    "total_memory_instructions",
    "unsafe_load",
    "unsafe_store",
]

HEAP_VECTOR_FIELDS = ["size_histogram", "unsafe_size_histogram"]

CPU_CYCLE_RAW_FIELDS = [
    "total_cycles",
    "unsafe_cycles_total",
    "unsafe_cycles_external",
    "external_cycles",
    "unsafe_blocks",
    "external_calls",
    "unsafe_external_calls",
]


# Exit code 101 is how a Rust test binary reports that a test assertion failed.
# The binary still ran its whole workload and still dumped its counters at exit,
# so its measurement is complete; only the test verdict is negative. Every other
# nonzero exit in this corpus means the work was cut short: 124 is the harness
# timeout and -6 is an abort, and neither writes a stat file at all.
TEST_ASSERTION_EXIT = 101


def _failed_bin_names(summary: dict, feature: str, variant: str,
                      keep_test_failures: bool = False) -> set[str]:
    """Bin names whose measurement should be discarded for (feature, variant).

    With keep_test_failures=True, a bin that exited 101 is kept. That matters
    for cross-run comparisons. rebench_v2's summaries do not list the generated
    coverage workload binaries at all -- they were added by a later top-up that
    did not rewrite the summaries -- so those bins were never filtered there and
    their measurements are in the published numbers. The 2026-09 dependency-
    include run does list them with their real exit codes, so filtering on that
    run alone removes nearly all unsafe execution from chrono, cpp_demangle,
    flate2 and num-bigint and makes the two runs incomparable for those crates.
    """
    f = summary.get("features", {}).get(feature)
    if f is None:
        return set()
    runs = f if isinstance(f, list) else [f]
    failed: set[str] = set()
    for run in runs:
        v = run.get("variants", {}).get(variant, {})
        for b in v.get("bins", []):
            code = b.get("exit_code", 0)
            if code == 0:
                continue
            if keep_test_failures and code == TEST_ASSERTION_EXIT:
                continue
            failed.add(b.get("name", ""))
    failed.discard("")
    return failed


def _bin_name_from_stat_filename(name: str) -> str:
    """Stat filenames: '<bin>__<pid>.<bin>.<metric>.json' -> '<bin>'."""
    return name.split("__", 1)[0]


def _aggregate_per_bin_dir(
    directory: Path,
    scalar_fields: list[str],
    vector_fields: list[str],
    failed_bins: set[str],
    allowed_logical_bins: set[str] | None = None,
    optional_fields: list[str] | None = None,
) -> dict | None:
    if not directory.is_dir():
        return None
    scalar: dict[str, int] = defaultdict(int)
    vector: dict[str, list[int]] = {k: [] for k in vector_fields}
    seen_optional: set[str] = set()
    bin_count = 0
    for jf in sorted(directory.glob("*.json")):
        bin_name = _bin_name_from_stat_filename(jf.name)
        if bin_name in failed_bins:
            continue
        if allowed_logical_bins is not None \
           and logical_bin_name(bin_name) not in allowed_logical_bins:
            continue
        try:
            data = json.loads(jf.read_text())
        except json.JSONDecodeError:
            continue
        stats = data.get("stats", {})
        for k in scalar_fields:
            scalar[k] += int(stats.get(k, 0) or 0)
        for k in (optional_fields or []):
            if k in stats:
                seen_optional.add(k)
                scalar[k] += int(stats.get(k, 0) or 0)
        for k in vector_fields:
            v = stats.get(k, []) or []
            if len(v) > len(vector[k]):
                vector[k].extend([0] * (len(v) - len(vector[k])))
            for i, x in enumerate(v):
                vector[k][i] += int(x)
        bin_count += 1
    if bin_count == 0:
        return None
    out: dict = dict(scalar)
    out.update(vector)
    # Drop optional keys no stat file carried, so an older run is unchanged.
    for k in (optional_fields or []):
        if k not in seen_optional:
            out.pop(k, None)
    out["_bin_count"] = bin_count
    return out


def _aggregate_cpu_cycle(crate_dir: Path, summary: dict) -> dict:
    """Aggregate cpu_cycle from avg.json; compute internal totals.

    NB: avg.json is built by scripts/rebench.aggregate_cpu_cycle, which walks
    every per-run *.cpu_cycle.json file that exists on disk regardless of
    exit_code. Bins that exited 101 (e.g. insta snapshot tests on first run,
    `colored-*` / `flate2-*`) still dump atexit stats and contribute samples.
    Stem-level filtering here would drop the whole stem because cpu_cycle is
    keyed by hash-stripped stem; that diverges from heap_tracker/unsafe_counter
    (per-hash filter) and silently zeroes out crates whose only failures are
    benign snapshot-write exits. So: trust avg.json as-is.

    When external unsafe cycles exceed the measured unsafe total (the
    `overcounted` case — only loom in this corpus), the external classification
    is corrupted, so we attribute all measured unsafe cycles to internal
    (external := 0) rather than the old min()-cap that zeroed internal. This
    replaces the prior guard, which merely prevented a negative internal_unsafe
    by forcing it to 0; that zeroing is what masked loom's real unsafe.
    """
    avg_path = crate_dir / "cpu_cycle_counter" / "avg.json"
    if not avg_path.is_file():
        return {}
    avg = json.loads(avg_path.read_text())
    out: dict = {}
    for variant, bins in avg.get("variants", {}).items():
        agg: dict[str, float] = defaultdict(float)
        bin_count = 0
        for bname, stats in bins.items():
            for k in CPU_CYCLE_RAW_FIELDS:
                v = stats.get(k)
                if isinstance(v, dict) and "mean" in v:
                    agg[k] += float(v["mean"])
            bin_count += 1
        if bin_count == 0:
            continue
        unsafe_total = agg.get("unsafe_cycles_total", 0.0)
        unsafe_external_raw = agg.get("unsafe_cycles_external", 0.0)
        internal_total = agg.get("total_cycles", 0.0) - agg.get("external_cycles", 0.0)
        overcounted = unsafe_external_raw > unsafe_total
        if overcounted:
            # External split is corrupted: external unsafe cycles exceed the
            # measured unsafe total, which is impossible if external ⊆ total.
            # Cause: a long-lived worker thread leaks the IN_UNSAFE depth counter
            # (loom's model checker reuses threads across thousands of
            # catch_unwind iterations; the install_panic_hook reset only fires on
            # the panicking thread, so the leak persists on the reused workers).
            # The outer unsafe rdtsc bracket then gets skipped while the inner
            # external-call tracker keeps firing. The classification is
            # untrustworthy, so attribute ALL measured unsafe cycles to internal
            # (external := 0) rather than the old min()-cap that forced internal
            # to 0. loom is the ONLY corpus crate that trips this (both variants).
            unsafe_external_capped = 0.0
            internal_unsafe = unsafe_total
        else:
            unsafe_external_capped = min(unsafe_external_raw, unsafe_total)
            internal_unsafe = unsafe_total - unsafe_external_capped
        # Whole-program view: no internal/external correction at all.
        #
        # The correction exists to remove time spent in code the run does not
        # instrument. That reasoning holds when only the primary package is
        # instrumented. It breaks once dependencies are instrumented too,
        # because ExternalCallTracker calls a callee external whenever it has
        # no body in the current module, which is true of every cross-crate
        # call regardless of whether the callee is instrumented. The corrected
        # denominator then excludes most dependency execution while the
        # numerator includes dependency unsafe time, which inflates the ratio.
        #
        # So a whole-dependency-graph run should be read through these two
        # fields, and a primary-package-only run through the internal_ pair.
        out[variant] = {
            **agg,
            "internal_total_cycles": internal_total,
            "internal_unsafe_cycles": internal_unsafe,
            "whole_program_total_cycles": agg.get("total_cycles", 0.0),
            "whole_program_unsafe_cycles": unsafe_total,
            "unsafe_external_capped": unsafe_external_capped,
            "unsafe_external_overcounted": overcounted,
            "_bin_count": bin_count,
        }
    return out


def _heap_common_logical_bins(crate_dir: Path) -> set[str] | None:
    """Return the set of logical bin names present in BOTH heap_tracker
    variants for `crate_dir`. Used to keep variant comparisons apples-to-apples
    when one variant lost a bin to a timeout (e.g. tokio rt_threaded)."""
    seen: dict[str, set[str]] = {}
    for variant in ("with_native", "without_native"):
        d = crate_dir / "heap_tracker" / variant
        if not d.is_dir():
            return None
        names = {logical_bin_name(_bin_name_from_stat_filename(p.name))
                 for p in d.glob("*.json")}
        seen[variant] = names
    return seen["with_native"] & seen["without_native"]


def _logical_ldst_map(
    directory: Path,
    load_key: str,
    store_key: str,
    failed_bins: set[str],
    allowed_logical_bins: set[str] | None = None,
) -> dict[str, int]:
    """Map logical_bin_name -> summed (load + store) over `directory`, applying
    the same failed-bin and optional logical-allow filters as
    _aggregate_per_bin_dir. Used to align the Mem-Inst numerator (heap_tracker)
    and denominator (unsafe_counter) over a shared bin set."""
    out: dict[str, int] = defaultdict(int)
    if not directory.is_dir():
        return out
    for jf in sorted(directory.glob("*.json")):
        bin_name = _bin_name_from_stat_filename(jf.name)
        if bin_name in failed_bins:
            continue
        lb = logical_bin_name(bin_name)
        if allowed_logical_bins is not None and lb not in allowed_logical_bins:
            continue
        try:
            stats = json.loads(jf.read_text()).get("stats", {})
        except json.JSONDecodeError:
            continue
        out[lb] += int(stats.get(load_key, 0) or 0) + int(stats.get(store_key, 0) or 0)
    return out


def aggregate_crate(crate_dir: Path, heap_common_bins_only: bool = False,
                    keep_test_failures: bool = False) -> dict | None:
    summary_path = crate_dir / "rebench_summary.json"
    if not summary_path.is_file():
        return None
    summary = json.loads(summary_path.read_text())
    crate: dict = {
        "crate": crate_dir.name,
        "unsafe_counter": {},
        "heap_tracker": {},
        "cpu_cycle_counter": {},
    }
    heap_allow: set[str] | None = None
    if heap_common_bins_only:
        heap_allow = _heap_common_logical_bins(crate_dir)
        if heap_allow is not None:
            crate["heap_tracker_common_bins"] = sorted(heap_allow)
    for variant in ("with_native", "without_native"):
        failed_uc = _failed_bin_names(summary, "unsafe_counter", variant,
                                      keep_test_failures)
        uc = _aggregate_per_bin_dir(
            crate_dir / "unsafe_counter" / variant,
            UNSAFE_COUNTER_FIELDS,
            [],
            failed_uc,
            optional_fields=OPTIONAL_COUNTER_FIELDS,
        )
        if uc:
            crate["unsafe_counter"][variant] = uc

        failed_h = _failed_bin_names(summary, "heap_tracker", variant,
                                     keep_test_failures)
        ht = _aggregate_per_bin_dir(
            crate_dir / "heap_tracker" / variant,
            HEAP_SCALAR_FIELDS,
            HEAP_VECTOR_FIELDS,
            failed_h,
            allowed_logical_bins=heap_allow,
        )
        if ht:
            crate["heap_tracker"][variant] = ht

        # Mem-Inst column = heap_tracker(unsafe ld+st)  /  unsafe_counter(unsafe
        # ld+st). The numerator (heap-touching unsafe mem ops) is a SUBSET of
        # the denominator (all unsafe mem ops), so the ratio is bounded by 100%
        # ONLY when both sums run over the same logical bins. They don't by
        # default: tokio's rt_threaded is present under heap_tracker (47B unsafe
        # ld+st) but timed out under unsafe_counter, which pushed the ratio to
        # 1291%. Restrict both to the bins present in BOTH features (same
        # failed-bin + heap_allow filters), so the subset bound holds.
        heap_ldst = _logical_ldst_map(
            crate_dir / "heap_tracker" / variant,
            "unsafe_load", "unsafe_store", failed_h, heap_allow)
        uc_ldst = _logical_ldst_map(
            crate_dir / "unsafe_counter" / variant,
            "unsafe_loads", "unsafe_stores", failed_uc)
        common = set(heap_ldst) & set(uc_ldst)
        if common:
            crate.setdefault("mem_inst_common", {})[variant] = {
                "heap_unsafe_ldst": sum(heap_ldst[b] for b in common),
                "uc_unsafe_ldst": sum(uc_ldst[b] for b in common),
                "n_common_bins": len(common),
                "n_heap_bins": len(heap_ldst),
                "n_uc_bins": len(uc_ldst),
            }

    crate["cpu_cycle_counter"] = _aggregate_cpu_cycle(crate_dir, summary)
    return crate


def aggregate_all(root: Path = REBENCH_ROOT,
                  heap_common_bins_only: bool = False,
                  keep_test_failures: bool = False) -> dict:
    out: dict = {}
    for crate_dir in sorted(root.iterdir()):
        if not crate_dir.is_dir() or crate_dir.name.startswith("_"):
            continue
        agg = aggregate_crate(crate_dir,
                              heap_common_bins_only=heap_common_bins_only,
                              keep_test_failures=keep_test_failures)
        if agg:
            out[crate_dir.name] = agg
    return out


# ---------- sanity check ----------

def _check_pair(
    issues: list[str], crate: str, variant: str, label: str, unsafe: float, total: float
) -> None:
    if total <= 0 and unsafe > 0:
        issues.append(f"{crate}/{variant}/{label}: unsafe={unsafe:.3g} but total=0")
    elif unsafe > total:
        issues.append(
            f"{crate}/{variant}/{label}: unsafe ({unsafe:.3g}) > total ({total:.3g})"
        )


def sanity_check(crates: dict) -> list[str]:
    issues: list[str] = []
    for crate, data in crates.items():
        # unsafe_counter
        for variant, uc in data.get("unsafe_counter", {}).items():
            _check_pair(
                issues, crate, variant, "unsafe_inst/total_inst",
                uc.get("unsafe_instructions", 0), uc.get("total_instructions", 0),
            )
            _check_pair(
                issues, crate, variant, "unsafe_calls/total_calls",
                uc.get("unsafe_function_calls", 0), uc.get("total_function_calls", 0),
            )
            _check_pair(
                issues, crate, variant, "unsafe_funcs_exec/total_funcs_exec",
                uc.get("unsafe_functions_executed", 0),
                uc.get("total_functions_executed", 0),
            )
            _check_pair(
                issues, crate, variant, "funcs_with_unsafe_insts/funcs_exec",
                uc.get("unsafe_functions_with_executed_insts", 0),
                uc.get("total_functions_executed", 0),
            )
        # heap_tracker
        for variant, ht in data.get("heap_tracker", {}).items():
            _check_pair(
                issues, crate, variant, "unsafe_heap_mem/total_heap_usage",
                ht.get("unsafe_heap_memory", 0), ht.get("total_heap_usage", 0),
            )
            _check_pair(
                issues, crate, variant, "unsafe_heap_objs/total_heap_alloc",
                ht.get("unsafe_heap_objects", 0), ht.get("total_heap_alloc", 0),
            )
            sh = ht.get("size_histogram", [])
            ush = ht.get("unsafe_size_histogram", [])
            for i, (u, t) in enumerate(zip(ush, sh)):
                if u > t:
                    issues.append(
                        f"{crate}/{variant}/size_histogram[{i}]: unsafe={u} > total={t}"
                    )
        # cpu_cycle: only internal
        for variant, cc in data.get("cpu_cycle_counter", {}).items():
            _check_pair(
                issues, crate, variant, "internal_unsafe/internal_total_cycles",
                cc.get("internal_unsafe_cycles", 0),
                cc.get("internal_total_cycles", 0),
            )
    return issues


if __name__ == "__main__":
    import sys

    data = aggregate_all()
    out_path = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("aggregated_rebench.json")
    out_path.write_text(json.dumps(data, indent=2))
    print(f"Aggregated {len(data)} crates -> {out_path}")

    issues = sanity_check(data)
    if issues:
        print(f"\nSanity check: {len(issues)} anomaly(ies)")
        for line in issues:
            print(f"  {line}")
    else:
        print("\nSanity check: clean (no unsafe>total cases)")
