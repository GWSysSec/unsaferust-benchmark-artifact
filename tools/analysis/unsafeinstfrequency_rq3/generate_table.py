#!/usr/bin/env python3
"""RQ3: unsafe instruction frequency = unsafe_instructions / total_instructions.

Per-crate ratios summarized with min/geomean/median/max for both variants.
Writes Latex/tables/unsafe_inst_frequency.tex.
"""

from __future__ import annotations

import argparse
import json
import sys
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
import datasets  # noqa: E402


def load_variant(ds: datasets.Dataset, suffix: str) -> dict:
    """Return per-crate flat unsafe_counter records for one native-library
    setting. `suffix` is 'nativefalse' or 'nativetrue'. Refresh the JSONs with
    ../export_dataset.py after a new measurement run."""
    p = HERE / f"{ds.stem(f'unsafe_counter_{suffix}')}.json"
    return json.loads(p.read_text())["per_crate_data"]


def geomean(values):
    nz = [v for v in values if v > 0]
    if not nz:
        return 0.0
    return statistics.geometric_mean(nz)


def collect(per_crate: dict) -> list[float]:
    out = []
    for rec in per_crate.values():
        total = rec.get("total_instructions", 0)
        unsafe = rec.get("unsafe_instructions", 0)
        if total > 0:
            out.append(unsafe / total * 100.0)
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    datasets.add_argument(ap)
    ds = datasets.get(ap.parse_args().dataset)

    wo = collect(load_variant(ds, "nativefalse"))
    w  = collect(load_variant(ds, "nativetrue"))

    mn_wo, gm_wo, md_wo, mx_wo = min(wo), geomean(wo), statistics.median(wo), max(wo)
    mn_w, gm_w, md_w, mx_w = min(w), geomean(w), statistics.median(w), max(w)
    multiplier = gm_w / gm_wo if gm_wo > 0 else 0.0

    latex = (
        "\\begin{table}[tb]\n"
        "\\caption{Unsafe Instruction Frequency Analysis for Common Crates.\n"
        "Dynamic runtime behavior comparison showing how native library attribution affects\n"
        "unsafe instruction execution frequency (RQ3: How frequently are unsafe instructions executed?)}\n"
        "\\label{table:unsafe_inst_frequency}\n"
        "\\centering\n"
        "\\begin{tabular}{@{}l|c|c@{}}\n"
        "\\toprule\n"
        "\\textbf{Metric} & \\textbf{w/o native lib} & \\textbf{w/ native lib} \\\\\n"
        "\\midrule\n"
        f"Unsafe \\% (Min) & {mn_wo:.2f}\\% & {mn_w:.2f}\\% \\\\\n"
        f"Unsafe \\% (Geo Mean) & {gm_wo:.2f}\\% & {gm_w:.2f}\\% \\\\\n"
        f"Unsafe \\% (Median) & {md_wo:.2f}\\% & {md_w:.2f}\\% \\\\\n"
        f"Unsafe \\% (Max) & {mx_wo:.2f}\\% & {mx_w:.2f}\\% \\\\\n"
        "\\midrule\n"
        f"\\multicolumn{{3}}{{c}}{{\\textbf{{Impact of native libraries: {multiplier:.1f}x}}}} \\\\\n"
        "\\bottomrule\n"
        "\\end{tabular}\n"
        "\\end{table}\n"
    )

    out = (HERE.parent / "Latex" / "tables"
           / f"{ds.stem('unsafe_inst_frequency')}.tex")
    out.write_text(latex)
    print(f"Wrote {out}")
    print(f"dataset: {ds.name} ({ds.scope})")
    print(f"crates: w/o={len(wo)} w/={len(w)}")
    print(f"w/o: min={mn_wo:.2f} geo={gm_wo:.2f} med={md_wo:.2f} max={mx_wo:.2f}")
    print(f"w/ : min={mn_w:.2f} geo={gm_w:.2f} med={md_w:.2f} max={mx_w:.2f}")
    print(f"native multiplier: {multiplier:.1f}x")


if __name__ == "__main__":
    main()
