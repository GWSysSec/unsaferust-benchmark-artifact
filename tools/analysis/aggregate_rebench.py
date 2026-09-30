"""Aggregate per-binary stat files into per-crate totals.

A crate directory holds <feature>/<variant>/*.json, one stat file per test
binary, and rebench_summary.json, which records each binary's exit code. CPU
cycles are read from cpu_cycle_counter/avg.json, the mean over repeated runs.
"""

from __future__ import annotations

import json
import os
import re
from collections import defaultdict
from pathlib import Path

# Cargo appends a 16-hex hash that differs between builds with different
# RUSTFLAGS, so binaries are matched across variants by the name without it.
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

# Summed only when a stat file carries them, so older runs stay unchanged.
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


def _is_balanced_workload(bin_name: str) -> bool:
    name = logical_bin_name(bin_name)
    return name.startswith("gen_") and name.endswith("_balanced")


def _failed_bin_names(summary: dict, feature: str, variant: str) -> set[str]:
    """Binaries of (feature, variant) whose measurement is discarded.

    A binary that exited nonzero is discarded, except the gen_*_balanced
    workloads, which the published data keeps whatever their exit code.
    """
    f = summary.get("features", {}).get(feature)
    if f is None:
        return set()
    runs = f if isinstance(f, list) else [f]
    failed: set[str] = set()
    for run in runs:
        v = run.get("variants", {}).get(variant, {})
        for b in v.get("bins", []):
            name = b.get("name", "")
            if b.get("exit_code", 0) != 0 and not _is_balanced_workload(name):
                failed.add(name)
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
    for k in (optional_fields or []):
        if k not in seen_optional:
            out.pop(k, None)
    out["_bin_count"] = bin_count
    return out


def _aggregate_cpu_cycle(crate_dir: Path) -> dict:
    """Sum the per-binary means in avg.json, whatever each binary's exit code."""
    avg_path = crate_dir / "cpu_cycle_counter" / "avg.json"
    if not avg_path.is_file():
        return {}
    avg = json.loads(avg_path.read_text())
    out: dict = {}
    for variant, bins in avg.get("variants", {}).items():
        agg: dict[str, float] = defaultdict(float)
        bin_count = 0
        for stats in bins.values():
            for k in CPU_CYCLE_RAW_FIELDS:
                v = stats.get(k)
                if isinstance(v, dict) and "mean" in v:
                    agg[k] += float(v["mean"])
            bin_count += 1
        if bin_count == 0:
            continue
        unsafe_total = agg.get("unsafe_cycles_total", 0.0)
        unsafe_external = min(agg.get("unsafe_cycles_external", 0.0), unsafe_total)
        out[variant] = {
            **agg,
            "internal_total_cycles": agg.get("total_cycles", 0.0) - agg.get("external_cycles", 0.0),
            "internal_unsafe_cycles": unsafe_total - unsafe_external,
            "_bin_count": bin_count,
        }
    return out


def _heap_common_logical_bins(crate_dir: Path) -> set[str] | None:
    """Logical binaries present in both heap_tracker variants of the crate."""
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
    """logical binary -> unsafe loads + stores, filtered as in _aggregate_per_bin_dir."""
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


def aggregate_crate(crate_dir: Path, heap_common_bins_only: bool = False) -> dict | None:
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
        failed_uc = _failed_bin_names(summary, "unsafe_counter", variant)
        uc = _aggregate_per_bin_dir(
            crate_dir / "unsafe_counter" / variant,
            UNSAFE_COUNTER_FIELDS,
            [],
            failed_uc,
            optional_fields=OPTIONAL_COUNTER_FIELDS,
        )
        if uc:
            crate["unsafe_counter"][variant] = uc

        failed_h = _failed_bin_names(summary, "heap_tracker", variant)
        ht = _aggregate_per_bin_dir(
            crate_dir / "heap_tracker" / variant,
            HEAP_SCALAR_FIELDS,
            HEAP_VECTOR_FIELDS,
            failed_h,
            allowed_logical_bins=heap_allow,
        )
        if ht:
            crate["heap_tracker"][variant] = ht

        # Mem Inst: heap-touching unsafe loads and stores over all unsafe loads
        # and stores, both summed over the binaries present in both features.
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

    crate["cpu_cycle_counter"] = _aggregate_cpu_cycle(crate_dir)
    return crate


def aggregate_all(root: Path = REBENCH_ROOT, heap_common_bins_only: bool = False) -> dict:
    out: dict = {}
    for crate_dir in sorted(root.iterdir()):
        if not crate_dir.is_dir() or crate_dir.name.startswith("_"):
            continue
        agg = aggregate_crate(crate_dir, heap_common_bins_only=heap_common_bins_only)
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
