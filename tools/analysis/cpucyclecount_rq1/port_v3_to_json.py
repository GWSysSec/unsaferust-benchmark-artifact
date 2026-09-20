#!/usr/bin/env python3
"""Port rebench_v3 cpu_cycle data into the two RQ1 JSON inputs.

Self-contained (does NOT import aggregate_rebench): reads each crate's
rebench_v3/<crate>/cpu_cycle_counter/avg.json, sums the per-bin means, and
computes the internal-cycle unsafe ratio. Writes:

    cpucycle_withoutnative.json   (variant without_native — "w/o std libs", RQ1)
    cpucycle_withnative.json      (variant with_native    — "w/ std libs", RQ6)

The plot/table generators read these JSONs directly, so this is the ONLY
step that touches rebench_v3. rebench_v3 is a cpu_cycle-only rerun (corrected
counter); heap_tracker / unsafe_counter RQs still source rebench_v2 via
aggregate_rebench, so REBENCH_ROOT is deliberately NOT repointed.

Metric (per crate, per variant):
    internal_total_cycles  = total_cycles        - external_cycles
    internal_unsafe_cycles = unsafe_cycles_total - min(unsafe_cycles_external,
                                                       unsafe_cycles_total)
    unsafe_percentage      = internal_unsafe / internal_total * 100

The min() is purely defensive (internal_unsafe can't go negative). Under the
corrected rebench_v3 counter NO crate over-counts external unsafe cycles
(unsafe_cycles_external <= unsafe_cycles_total for all 100 crates, both
variants) — so the v2-era loom-only "external := 0" guard is gone; it never
fires here and is dropped to keep the metric a plain subtraction.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REBENCH_V3 = Path(
    "/home/oscar/Projects/unsafebench/rusttest-gen/rebench_v3"
)

RAW_FIELDS = [
    "total_cycles",
    "unsafe_cycles_total",
    "unsafe_cycles_external",
    "external_cycles",
]


def crate_variant_stats(avg: dict, variant: str) -> dict | None:
    """Sum per-bin means for one variant; return derived internal-cycle stats."""
    bins = avg.get("variants", {}).get(variant)
    if not bins:
        return None
    agg = {k: 0.0 for k in RAW_FIELDS}
    n_bins = 0
    for _bname, metrics in bins.items():
        for k in RAW_FIELDS:
            v = metrics.get(k)
            if isinstance(v, dict) and "mean" in v:
                agg[k] += float(v["mean"])
        n_bins += 1
    if n_bins == 0:
        return None

    total = agg["total_cycles"]
    external = agg["external_cycles"]
    unsafe_total = agg["unsafe_cycles_total"]
    unsafe_external = agg["unsafe_cycles_external"]

    internal_total = total - external
    # defensive floor at 0; v3 has no over-count cases so this == plain subtract.
    internal_unsafe = unsafe_total - min(unsafe_external, unsafe_total)

    if internal_total <= 0:
        return None
    return {
        "total_cycles": total,
        "external_cycles": external,
        "internal_cycles": internal_total,
        "unsafe_cycles": internal_unsafe,
        "unsafe_percentage": internal_unsafe / internal_total * 100.0,
        "_overcounted": unsafe_external > unsafe_total,   # expected: always False
        "_bin_count": n_bins,
    }


def build(variant: str) -> dict[str, dict]:
    out: dict[str, dict] = {}
    for crate_dir in sorted(REBENCH_V3.iterdir()):
        if not crate_dir.is_dir() or crate_dir.name.startswith("_"):
            continue
        avg_path = crate_dir / "cpu_cycle_counter" / "avg.json"
        if not avg_path.is_file():
            continue
        avg = json.loads(avg_path.read_text())
        stats = crate_variant_stats(avg, variant)
        if stats is None:
            continue
        # drop the private helper keys from the on-disk artifact
        overcounted = stats.pop("_overcounted")
        stats.pop("_bin_count")
        out[crate_dir.name] = stats
        if overcounted:
            print(f"  [overcount guard] {crate_dir.name}/{variant}: "
                  f"external unsafe > total -> external:=0")
    return out


def main() -> int:
    meta = {
        "source": "rebench_v3 cpu_cycle avg.json (corrected counter, 3-run)",
        "metric": "internal_unsafe_cycles / internal_total_cycles",
    }
    for variant, fname in [("without_native", "cpucycle_withoutnative.json"),
                           ("with_native", "cpucycle_withnative.json")]:
        crates = build(variant)
        (HERE / fname).write_text(
            json.dumps({"metadata": meta, "crates": crates}, indent=2))
        pcts = [c["unsafe_percentage"] for c in crates.values()]
        print(f"{fname}: {len(crates)} crates; "
              f"min={min(pcts):.3f} max={max(pcts):.3f} "
              f"median={sorted(pcts)[len(pcts)//2]:.3f}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
