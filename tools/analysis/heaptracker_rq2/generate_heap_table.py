#!/usr/bin/env python3
"""RQ2: heap memory accessed by unsafe code, by allocation size.

For each crate per variant:
  - Aggregate size_histogram and unsafe_size_histogram across bins.
  - Clamp per-bucket unsafe = min(unsafe, total) to handle instrumentation
    artifacts in a small number of buckets across 7 crates.
  - Compute Small/Medium/Large object-count and memory-size percentages,
    plus the Total per-crate percentage.
  - Mem Inst column = (heap.unsafe_load + heap.unsafe_store) divided by the
    cross-feature unsafe-mem-inst total from unsafe_counter
    (unsafe_loads + unsafe_stores).
Summarize per-crate values with min/geomean/median/max for both variants.
"""

from __future__ import annotations

import argparse
import json
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
import datasets  # noqa: E402


def load_variant(ds: datasets.Dataset, variant: str) -> dict:
    """Per-crate heap records for one native-library setting.

    Reads the committed JSON rather than re-aggregating the raw per-binary
    dumps, so this table can be rebuilt from the data the artifact ships, the
    way every other table already could. Refresh with ../export_dataset.py.
    """
    path = HERE / f"{ds.stem('heap_' + variant)}.json"
    out = {}
    for row in json.loads(path.read_text())["crates"]:
        rec = dict(row["aggregated_stats"])
        if "mem_inst_common" in row:
            rec["_mem_inst_common"] = row["mem_inst_common"]
        out[row["crate_name"]] = rec
    return out


# Allocation-size bin midpoints (bytes). Bins 8+ double from 192KB.
BIN_MIDPOINTS = [512, 1536, 3072, 6144, 12288, 24576, 49152, 98304]


def geomean(values):
    nz = [v for v in values if v > 0]
    if not nz:
        return 0.0
    return statistics.geometric_mean(nz)


def mmgm_max(values):
    if not values:
        return (0.0, 0.0, 0.0, 0.0)
    return (min(values), geomean(values), statistics.median(values), max(values))


def clamp_unsafe(total: list[int], unsafe: list[int]) -> list[int]:
    return [min(u, t) for u, t in zip(unsafe, total)]


def categorize_objects(hist: list[int]) -> tuple[int, int, int]:
    if not hist:
        return (0, 0, 0)
    small = hist[0] if len(hist) > 0 else 0
    medium = sum(hist[1:8])
    large = sum(hist[8:])
    return (small, medium, large)


def categorize_memory(hist: list[int]) -> tuple[int, int, int]:
    if not hist:
        return (0, 0, 0)
    small = hist[0] * BIN_MIDPOINTS[0] if len(hist) > 0 else 0
    medium = sum(hist[i] * BIN_MIDPOINTS[i] for i in range(1, min(8, len(hist))))
    large = 0
    for i in range(8, len(hist)):
        midpoint = 196608 * (2 ** (i - 8))
        large += hist[i] * midpoint
    return (small, medium, large)


def fmt(v, is_pct=True):
    if is_pct:
        return f"{v:.1f}\\%" if v > 0 else "0\\%"
    return f"{v:.1f}"


def collect_variant(crates: dict, variant: str) -> dict:
    """Return per-metric lists across crates for one variant."""
    out = {
        "obj_small": [], "obj_medium": [], "obj_large": [], "obj_total": [],
        "mem_small": [], "mem_medium": [], "mem_large": [], "mem_total": [],
        "mem_inst": [],
    }
    for ht in crates.values():
        if not ht:
            continue
        sh = list(ht.get("size_histogram", []))
        ush_raw = list(ht.get("unsafe_size_histogram", []))
        # Pad to equal length, then clamp.
        n = max(len(sh), len(ush_raw))
        sh += [0] * (n - len(sh))
        ush_raw += [0] * (n - len(ush_raw))
        ush = clamp_unsafe(sh, ush_raw)

        ts, tm, tl = categorize_objects(sh)
        us, um, ul = categorize_objects(ush)
        if ts > 0:
            out["obj_small"].append(us / ts * 100.0)
        if tm > 0:
            out["obj_medium"].append(um / tm * 100.0)
        if tl > 0:
            out["obj_large"].append(ul / tl * 100.0)
        tot_obj = ts + tm + tl
        if tot_obj > 0:
            out["obj_total"].append((us + um + ul) / tot_obj * 100.0)

        tsm, tmm, tlm = categorize_memory(sh)
        usm, umm, ulm = categorize_memory(ush)
        if tsm > 0:
            out["mem_small"].append(usm / tsm * 100.0)
        if tmm > 0:
            out["mem_medium"].append(umm / tmm * 100.0)
        if tlm > 0:
            out["mem_large"].append(ulm / tlm * 100.0)
        tot_mem = tsm + tmm + tlm
        if tot_mem > 0:
            out["mem_total"].append((usm + umm + ulm) / tot_mem * 100.0)

        # Mem Inst column: heap(unsafe ld+st) / unsafe_counter(unsafe ld+st),
        # both summed over the bins present in BOTH features (mem_inst_common,
        # computed in aggregate_rebench). Aligning the bin sets restores the
        # subset bound (heap-touching unsafe mem ops <= all unsafe mem ops);
        # without it, tokio reads 1291% because rt_threaded is in the heap
        # numerator but absent from the unsafe_counter denominator (timed out).
        mic = ht.get("_mem_inst_common")
        if mic:
            numer = mic["heap_unsafe_ldst"]
            denom = mic["uc_unsafe_ldst"]
            if denom > 0:
                out["mem_inst"].append(numer / denom * 100.0)
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    datasets.add_argument(ap)
    ds = datasets.get(ap.parse_args().dataset)

    # The committed JSONs were written with the common-bin filter applied, so
    # bins present in one heap_tracker variant but not the other (e.g. tokio's
    # rt_threaded under with_native, which timed out at the 4h cap) are already
    # excluded and the two variants compare like-for-like.
    wo = collect_variant(load_variant(ds, "without_native"), "without_native")
    w = collect_variant(load_variant(ds, "with_native"), "with_native")

    keys = ["mem_small", "mem_medium", "mem_large", "mem_total", "mem_inst"]
    stats_wo = {k: mmgm_max(wo[k]) for k in keys}
    stats_w = {k: mmgm_max(w[k]) for k in keys}

    rows = [("Min", 0), ("Geomean", 1), ("Median", 2), ("Max", 3)]
    body = ""
    for name, idx in rows:
        cells = []
        for k in keys:
            cells.append(fmt(stats_wo[k][idx]))
        for k in keys:
            cells.append(fmt(stats_w[k][idx]))
        body += f"\\textbf{{{name}}} & " + " & ".join(cells) + " \\\\\n"

    latex = (
        "\\begin{table*}[tb]\n"
        "\\caption{Summary of heap memory (by size) accessed by unsafe code\n"
        "and the percentages of unsafe memory instructions ({\\tt load} and {\\tt store})\n"
        "targeting the heap.}\n"
        "\\label{table:heap}\n"
        "\\centering\n"
        "% \\resizebox{\\textwidth}{!}{%\n"
        "{\\sffamily\n"
        "\\begin{tabular}{@{}lrrrr|r||rrrr|r@{}}\n"
        "\\toprule\n"
        "& \\multicolumn{5}{c||}{\\textbf{Without standard libraries}}"
        " & \\multicolumn{5}{c}{\\textbf{With standard libraries}} \\\\\n"
        "\\midrule\n"
        "& \\multicolumn{4}{c|}{\\textbf{Memory by Size}} & \\textbf{Mem}"
        " & \\multicolumn{4}{c|}{\\textbf{Memory by Size}} & \\textbf{Mem} \\\\\n"
        "\\cmidrule(r){2-5} \\cmidrule(l){7-10}\n"
        "& \\textbf{Small} & \\textbf{Medium} & \\textbf{Large} & \\textbf{Total} & \\textbf{Inst}"
        " & \\textbf{Small} & \\textbf{Medium} & \\textbf{Large} & \\textbf{Total} & \\textbf{Inst} \\\\\n"
        "\\midrule\n"
        f"{body}"
        "\\bottomrule\n"
        "\\end{tabular}\n"
        "}\n"
        "% }%\n"
        "\\end{table*}\n"
    )

    out = HERE.parent / "Latex" / "tables" / f"{ds.stem('heap')}.tex"
    out.write_text(latex)
    print(f"Wrote {out}")
    print(f"dataset: {ds.name} ({ds.scope})")
    print(f"crates w/o={len(wo['mem_total'])} w/={len(w['mem_total'])}")
    for k in keys:
        print(f"  {k}: w/o {stats_wo[k]}  | w/ {stats_w[k]}")


if __name__ == "__main__":
    main()
