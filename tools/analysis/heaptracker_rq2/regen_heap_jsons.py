#!/usr/bin/env python3
"""Regenerate the legacy heap_{with,without}_native.json and
unsafe_heap_percentage_*.json files from rebench_v2, applying the
common-bin filter (matches the same data feed used by generate_heap_table.py).

Output schema preserves the legacy field names so generate_heap_plot.py and
analyze_unsafe_percentages.py can consume the files unchanged:

  {
    "metadata": {...},
    "crates": [
      {"crate_name": ..., "aggregated_stats": {
         "total_heap_usage", "total_heap_allocations", ...
         "size_histogram", "unsafe_size_histogram"
      }},
      ...
    ],
    "summary": {...}
  }
"""

from __future__ import annotations

import json
import os
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
from aggregate_rebench import aggregate_all, REBENCH_ROOT  # noqa: E402


# rebench_v2 field name -> legacy field name expected by plot/analyzer
FIELD_RENAME = {
    "total_heap_alloc": "total_heap_allocations",
    "total_heap_realloc": "total_heap_reallocations",
    "total_heap_dealloc": "total_heap_deallocations",
    "total_memory_instructions": "unsafe_memory_instructions",
}

LEGACY_FIELDS_ORDER = [
    "total_heap_usage",
    "total_heap_allocations",
    "total_heap_reallocations",
    "total_heap_deallocations",
    "unsafe_heap_memory",
    "unsafe_heap_objects",
    "unsafe_memory_instructions",
    "unsafe_load",
    "unsafe_store",
    "size_histogram",
    "unsafe_size_histogram",
]


def to_legacy_stats(ht: dict) -> dict:
    out: dict = {}
    for k, v in ht.items():
        if k.startswith("_"):
            continue
        key = FIELD_RENAME.get(k, k)
        out[key] = v
    # ensure required fields are present, defaulting to 0/empty list
    for k in LEGACY_FIELDS_ORDER:
        out.setdefault(k, [] if k.endswith("_histogram") else 0)
    # stable key order
    return {k: out[k] for k in LEGACY_FIELDS_ORDER if k in out}


def write_variant(crates: dict, variant: str, out_path: Path) -> int:
    rows = []
    for crate_name in sorted(crates):
        ht = crates[crate_name].get("heap_tracker", {}).get(variant)
        if not ht:
            continue
        rows.append({
            "crate_name": crate_name,
            "aggregated_stats": to_legacy_stats(ht),
        })

    payload = {
        "metadata": {
            "processing_date": time.strftime("%Y-%m-%d_%H-%M-%S"),
            "total_crates": len(rows),
            "filter_applied": "common-logical-bins between heap_tracker "
                              "with_native and without_native variants",
            "data_source_directory": str(REBENCH_ROOT),
        },
        "crates": rows,
        "summary": {
            "variant": variant,
            "field_schema_version": 2,
            "notes": "legacy field names retained for backwards-compat with "
                     "generate_heap_plot.py and analyze_unsafe_percentages.py",
        },
    }
    out_path.write_text(json.dumps(payload, indent=2))
    return len(rows)


def write_percentages(heap_path: Path, pct_path: Path) -> int:
    d = json.loads(heap_path.read_text())
    rows = []
    for c in d["crates"]:
        s = c["aggregated_stats"]
        total = s.get("total_heap_usage", 0)
        unsafe = s.get("unsafe_heap_memory", 0)
        pct = round(100.0 * unsafe / total, 2) if total > 0 else 0.0
        rows.append({
            "crate_name": c["crate_name"],
            "total_heap_usage": total,
            "unsafe_heap_memory": unsafe,
            "unsafe_heap_percentage": pct,
        })
    pct_path.write_text(json.dumps({
        "metadata": {
            "description": "Percentage of absolute unsafe heap memory",
            "source_file": heap_path.name,
            "metric": "unsafe_heap_memory / total_heap_usage * 100",
        },
        "crates": rows,
    }, indent=2))
    return len(rows)


def main() -> int:
    crates = aggregate_all(heap_common_bins_only=True)

    n_w   = write_variant(crates, "with_native",    HERE / "heap_with_native.json")
    n_wo  = write_variant(crates, "without_native", HERE / "heap_without_native.json")
    print(f"wrote heap_with_native.json    ({n_w} crates)")
    print(f"wrote heap_without_native.json ({n_wo} crates)")

    np_w  = write_percentages(HERE / "heap_with_native.json",
                              HERE / "unsafe_heap_percentage_with_native.json")
    np_wo = write_percentages(HERE / "heap_without_native.json",
                              HERE / "unsafe_heap_percentage_without_native.json")
    print(f"wrote unsafe_heap_percentage_with_native.json    ({np_w} crates)")
    print(f"wrote unsafe_heap_percentage_without_native.json ({np_wo} crates)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
