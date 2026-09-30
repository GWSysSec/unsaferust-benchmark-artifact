#!/usr/bin/env python3
"""RQ1: the share of CPU cycles a program spends in unsafe code.

Reads each crate's `unsafe_percentage`, unsafe cycles over internal cycles
(total cycles minus cycles in external calls), from the exported JSON files.
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


def geomean(values):
    nz = [v for v in values if v > 0]
    if not nz:
        return 0.0
    return statistics.geometric_mean(nz)


def stats(values):
    if not values:
        return (0.0, 0.0, 0.0, 0.0)
    return (min(values), geomean(values), statistics.median(values), max(values))


def collect(ds: datasets.Dataset, base: str) -> list[float]:
    path = HERE / f"{ds.stem(base)}.json"
    crates = json.loads(path.read_text())["crates"]
    return [cc["unsafe_percentage"] for cc in crates.values()
            if cc.get("unsafe_percentage") is not None]


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    datasets.add_argument(ap)
    ds = datasets.get(ap.parse_args().dataset)

    ratios_wo = collect(ds, "cpucycle_withoutnative")
    ratios_w = collect(ds, "cpucycle_withnative")

    mn_wo, gm_wo, md_wo, mx_wo = stats(ratios_wo)
    mn_w, gm_w, md_w, mx_w = stats(ratios_w)
    # The paper shows minima below 0.01% as 0.01%.
    mn_wo = max(mn_wo, 0.01)
    mn_w = max(mn_w, 0.01)

    # Static unsafe SLOC share over the 100-crate corpus.
    loc_min, loc_geo, loc_med, loc_max = 0.01, 1.4652, 1.8888, 55.73

    latex = f"""% \\captionof{{table}}{{}}
\\begin{{table}}[tb]
\\caption{{Summary of unsafe source code and
its execution.
% without and with accounting for unsafe code introduced by standard libraries.
}}
\\label{{table:cpu_cycle}}
\\centering
% \\resizebox{{0.88\\columnwidth}}{{!}}{{%
{{\\sffamily
% \\setlength{{\\tabcolsep}}{{3pt}}
\\begin{{tabular}}{{@{{}}l@{{\\hspace{{0.5em}}}}r@{{\\hspace{{0.5em}}}}r@{{\\hspace{{0.5em}}}}r@{{\\hspace{{0.5em}}}}r@{{}}}}
\\toprule
\\textbf{{}} & \\textbf{{Min}} & \\textbf{{Geomean}} & \\textbf{{Median}} & \\textbf{{Max}} \\\\
\\midrule
Unsafe SLOC & {loc_min:.2f}\\% & {loc_geo:.1f}\\% & {loc_med:.1f}\\% & {loc_max:.1f}\\% \\\\
\\midrule
\\multicolumn{{5}}{{c}}{{\\textbf{{CPU Cycles in Unsafe Code}}}} \\\\
\\midrule
\\textbf{{Config}} & \\textbf{{Min}} & \\textbf{{Geomean}} & \\textbf{{Median}} & \\textbf{{Max}} \\\\
\\midrule
w/o std libs & {mn_wo:.2f}\\% & {gm_wo:.1f}\\% & {md_wo:.1f}\\% & {mx_wo:.1f}\\% \\\\
w/ std libs & {mn_w:.2f}\\% & {gm_w:.1f}\\% & {md_w:.1f}\\% & {mx_w:.1f}\\% \\\\
\\bottomrule
\\end{{tabular}}
}}
% }}%
\\end{{table}}
"""

    out = HERE.parent / "Latex" / "tables" / f"{ds.stem('cpu_cycles')}.tex"
    out.write_text(latex)
    print(f"Wrote {out}")
    print(f"dataset: {ds.name}")
    print(f"crates: w/o={len(ratios_wo)} w/={len(ratios_w)}")
    print(f"w/o std: min={mn_wo:.2f} geo={gm_wo:.2f} med={md_wo:.2f} max={mx_wo:.2f}")
    print(f"w/  std: min={mn_w:.2f} geo={gm_w:.2f} med={md_w:.2f} max={mx_w:.2f}")


if __name__ == "__main__":
    main()
