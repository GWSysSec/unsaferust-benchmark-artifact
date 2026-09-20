#!/usr/bin/env python3
"""RQ5: unsafe function execution metrics.

Three metric groups:
  Static       = unsafe_fn declarations / total fn declarations in lib src/
                 (loaded from static_fn_counts.json; refresh via
                  rusttest-gen/scripts/compute_static_fn_counts.py)
  Distinctive  = unsafe_functions_executed / total_functions_executed
  Cumulative  = unsafe_function_calls    / total_function_calls
Distinctive/Cumulative come in w/o-std and w/-std variants; Static is a
single column (stdlib is never part of the user crate's source).
Summarized with min/geomean/median/max across crates.
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


def mmgm_max(values):
    if not values:
        return (0.0, 0.0, 0.0, 0.0)
    return (min(values), geomean(values), statistics.median(values), max(values))


def collect(per_crate: dict) -> tuple[list[float], list[float]]:
    distinctive, accumulated = [], []
    for rec in per_crate.values():
        funcs_exec = rec.get("total_functions_executed", 0)
        unsafe_funcs_exec = rec.get("unsafe_functions_executed", 0)
        total_calls = rec.get("total_function_calls", 0)
        unsafe_calls = rec.get("unsafe_function_calls", 0)

        if funcs_exec > 0:
            distinctive.append(unsafe_funcs_exec / funcs_exec * 100.0)
        if total_calls > 0:
            accumulated.append(unsafe_calls / total_calls * 100.0)
    return distinctive, accumulated


def load_static() -> list[float]:
    p = HERE / "static_fn_counts.json"
    data = json.loads(p.read_text())["per_crate_data"]
    return [rec["unsafe_fn_pct"] for rec in data.values()]


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    datasets.add_argument(ap)
    ds = datasets.get(ap.parse_args().dataset)

    s_static = mmgm_max(load_static())
    d_wo, a_wo = collect(load_variant(ds, "nativefalse"))
    d_w,  a_w  = collect(load_variant(ds, "nativetrue"))

    s_d_wo, s_a_wo = mmgm_max(d_wo), mmgm_max(a_wo)
    s_d_w,  s_a_w  = mmgm_max(d_w),  mmgm_max(a_w)

    labels = ("Min", "Geomean", "Median", "Max")
    body = ""
    for i, label in enumerate(labels):
        body += (
            f"    \\textbf{{{label}}}"
            f" & {s_static[i]:.1f}\\%"
            f" & {s_d_wo[i]:.1f}\\% & {s_d_w[i]:.1f}\\%"
            f" & {s_a_wo[i]:.1f}\\% & {s_a_w[i]:.1f}\\% \\\\\n"
        )

    latex = (
        "\\begin{table}[tb]\n"
        "    \\centering\n"
        "    \\caption{Execution frequency of functions with unsafe code.\n"
        "    {\\bf Distinctive} counts each function only once;\n"
        "    {\\bf Cumulative} counts function invocations cumulatively.}\n"
        "    \\label{table:unsafe_function}\n"
        "    \\begin{tabular}{@{}l|r|rr|rr@{}}\n"
        "    \\toprule\n"
        "    & \\textbf{Static}"
        " & \\multicolumn{2}{c|}{\\textbf{Distinctive}}"
        " & \\multicolumn{2}{c}{\\textbf{Cumulative}} \\\\\n"
        "    \\cmidrule(lr){2-2} \\cmidrule(lr){3-4} \\cmidrule(lr){5-6}\n"
        "    \\textbf{ } & \\textbf{w/o std}"
        " & \\textbf{w/o std} & \\textbf{w/ std}"
        " & \\textbf{w/o std} & \\textbf{w/ std} \\\\\n"
        "    \\midrule\n"
        f"{body}"
        "    \\bottomrule\n"
        "    \\end{tabular}\n"
        "\\end{table}\n"
    )

    out = HERE.parent / "Latex" / "tables" / f"{ds.stem('function')}.tex"
    out.write_text(latex)
    print(f"Wrote {out}")
    print(f"dataset: {ds.name} ({ds.scope})")
    print(f"crates: distinct w/o={len(d_wo)} w/={len(d_w)} | "
          f"accum w/o={len(a_wo)} w/={len(a_w)}")
    print(f"Static       w/o: {s_static}")
    print(f"Distinctive  w/o: {s_d_wo}  | w/: {s_d_w}")
    print(f"Cumulative  w/o: {s_a_wo}  | w/: {s_a_w}")


if __name__ == "__main__":
    main()
