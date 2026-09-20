#!/usr/bin/env python3
"""RQ4: distribution of unsafe instruction types.

For each crate per variant, compute per-type % = unsafe_<type> / unsafe_instructions.
Rows: Load, Store, Ptr arith, Cast, Call (direct), Call (indirect),
      Call (intrinsic), Atomic, Others, plus Total (= unsafe_inst / total_inst).
Dynamic columns are sourced from rebench. The Static column group is preserved
from the prior hand-edited table (rebench has no static-count instrumentation).
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


# (label, list of unsafe_counter field names to sum for the numerator)
# Atomic counts are bundled into Others to match the committed table layout
# (Load/Store/Ptr arith/Cast/Call/Others — no separate Atomic row).
ROW_DEFS = [
    ("Load",      ["unsafe_loads"]),
    ("Store",     ["unsafe_stores"]),
    ("Ptr arith", ["unsafe_geps"]),
    ("Cast",      ["unsafe_casts"]),
    ("Call",      ["unsafe_calls_direct", "unsafe_calls_indirect", "unsafe_calls_intrinsic"]),
    ("Others",    ["unsafe_others", "unsafe_atomics"]),
]

# Hand-curated static-count percentages (from prior analysis).
# Call static row restored from the pre-fine-graining table (commit 703955b)
# since we now merge direct/indirect/intrinsic back into one Call row.
STATIC_ROWS = {
    "Load":      ("0.0\\%", "9.7\\%",  "9.3\\%",  "75.0\\%"),
    "Store":     ("0.0\\%", "6.0\\%",  "5.2\\%",  "33.7\\%"),
    "Ptr arith": ("0.0\\%", "12.0\\%", "12.0\\%", "34.6\\%"),
    "Cast":      ("0.0\\%", "1.4\\%",  "1.1\\%",  "16.6\\%"),
    "Call":      ("0.0\\%", "15.8\\%", "13.9\\%", "100.0\\%"),
    "Others":    ("0.0\\%", "41.3\\%", "48.3\\%", "100.0\\%"),
}
STATIC_TOTAL = ("0.0\\%", "3.4\\%", "4.9\\%", "61.5\\%")


def geomean(values):
    nz = [v for v in values if v > 0]
    if not nz:
        return 0.0
    return statistics.geometric_mean(nz)


def mmgm_max(values):
    if not values:
        return (0.0, 0.0, 0.0, 0.0)
    return (min(values), geomean(values), statistics.median(values), max(values))


def fmt(v):
    return f"{v:.1f}\\%" if v > 0 else "0.0\\%"


def collect(per_crate: dict) -> tuple[dict, list[float]]:
    """Return per-row percentage lists (across crates) and total-row percentage list."""
    rows: dict[str, list[float]] = {label: [] for label, _ in ROW_DEFS}
    totals: list[float] = []
    for rec in per_crate.values():
        total_inst = rec.get("total_instructions", 0)
        unsafe_inst = rec.get("unsafe_instructions", 0)
        if total_inst <= 0:
            continue
        totals.append(unsafe_inst / total_inst * 100.0)
        for label, fields in ROW_DEFS:
            num = sum(rec.get(f, 0) or 0 for f in fields)
            ratio = (num / unsafe_inst * 100.0) if unsafe_inst > 0 else 0.0
            rows[label].append(ratio)
    return rows, totals


def render_row(label: str, static_cells: tuple, dyn_wo: tuple, dyn_w: tuple, bold=False) -> str:
    lab = f"\\textbf{{{label}}}" if bold else label
    cells = list(static_cells) + [fmt(v) for v in dyn_wo] + [fmt(v) for v in dyn_w]
    return f"{lab} & " + " & ".join(cells) + " \\\\\n"


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    datasets.add_argument(ap)
    ds = datasets.get(ap.parse_args().dataset)

    rows_wo, totals_wo = collect(load_variant(ds, "nativefalse"))
    rows_w, totals_w   = collect(load_variant(ds, "nativetrue"))

    body = ""
    for label, _ in ROW_DEFS:
        body += render_row(
            label,
            STATIC_ROWS[label],
            mmgm_max(rows_wo[label]),
            mmgm_max(rows_w[label]),
        )

    body += "\\midrule\n"
    body += render_row(
        "Total",
        STATIC_TOTAL,
        mmgm_max(totals_wo),
        mmgm_max(totals_w),
        bold=True,
    )

    latex = (
        "\\begin{table*}[tb]\n"
        "\\caption{Frequencies of unsafe instructions.\n"
        "Percentages for instruction types are relative to all unsafe instructions.\n"
        "The last row shows the percentages of unsafe instructions relative to all\n"
        "instructions.}\n"
        "\\label{table:inst}\n"
        "\\centering\n"
        "\\resizebox{\\textwidth}{!}{%\n"
        "\\begin{tabular}{@{}l|rrrr|rrrr|rrrr@{}}\n"
        "\\toprule\n"
        "\\textbf{Inst} & \\multicolumn{4}{c|}{\\textbf{Static counts w/o std libs}} &\n"
        "\\multicolumn{4}{c|}{\\textbf{Dynamic counts w/o std libs}} &\n"
        "\\multicolumn{4}{c}{\\textbf{Dynamic counts with std libs}} \\\\\n"
        "\\cmidrule(lr){2-5} \\cmidrule(lr){6-9} \\cmidrule(lr){10-13}\n"
        "\\textbf{Type} & \\textbf{Min} & \\textbf{Geomean} & \\textbf{Median} & \\textbf{Max}"
        " & \\textbf{Min} & \\textbf{Geomean} & \\textbf{Median} & \\textbf{Max}"
        " & \\textbf{Min} & \\textbf{Geomean} & \\textbf{Median} & \\textbf{Max} \\\\\n"
        "\\midrule\n"
        f"{body}"
        "\\bottomrule\n"
        "\\end{tabular}\n"
        "}%\n"
        "\\end{table*}\n"
    )

    out = HERE.parent / "Latex" / "tables" / f"{ds.stem('inst')}.tex"
    out.write_text(latex)
    print(f"Wrote {out}")
    print(f"dataset: {ds.name} ({ds.scope})")
    print(f"crates: w/o={len(totals_wo)} w/={len(totals_w)}")
    for label, _ in ROW_DEFS:
        wo = mmgm_max(rows_wo[label])
        wn = mmgm_max(rows_w[label])
        print(f"  {label}: w/o geo={wo[1]:.1f} med={wo[2]:.1f} | w/ geo={wn[1]:.1f} med={wn[2]:.1f}")
    tw = mmgm_max(totals_wo)
    tn = mmgm_max(totals_w)
    print(f"  Total: w/o geo={tw[1]:.1f} med={tw[2]:.1f} | w/ geo={tn[1]:.1f} med={tn[2]:.1f}")


if __name__ == "__main__":
    main()
