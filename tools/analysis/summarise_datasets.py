#!/usr/bin/env python3
"""Put the two measurement runs side by side, one line per reported number.

Writes ALLDEPS_VS_PUBLISHED.md. Every figure here is recomputed from the
per-question JSON files that the tables themselves read, so the document cannot
drift from the tables. See datasets.py for what the two runs are.
"""

from __future__ import annotations

import json
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import datasets  # noqa: E402

PUB, ALL = datasets.PUBLISHED, datasets.ALLDEPS
BIN_MIDPOINTS = [512, 1536, 3072, 6144, 12288, 24576, 49152, 98304]


def geomean(vs):
    nz = [v for v in vs if v > 0]
    return statistics.geometric_mean(nz) if nz else 0.0


def spread(vs):
    if not vs:
        return (0.0, 0.0, 0.0, 0.0)
    return (min(vs), geomean(vs), statistics.median(vs), max(vs))


def load(ds, folder, base):
    return json.loads((HERE / folder / f"{ds.stem(base)}.json").read_text())


def rq1(ds, variant):
    base = "cpucycle_withnative" if variant == "with_native" else "cpucycle_withoutnative"
    crates = load(ds, "cpucyclecount_rq1", base)["crates"]
    pcts = [c["unsafe_percentage"] for c in crates.values()]
    num = sum(c["whole_program_unsafe_cycles"] if ds.cycle_metric == "whole_program"
              else c["unsafe_cycles"] for c in crates.values())
    den = sum(c["whole_program_total_cycles"] if ds.cycle_metric == "whole_program"
              else c["internal_cycles"] for c in crates.values())
    return pcts, num, den


def rq2(ds, variant):
    crates = load(ds, "heaptracker_rq2", f"heap_{variant}")["crates"]
    pcts, num, den = [], 0, 0
    for row in crates:
        st = row["aggregated_stats"]
        t, u = st["total_heap_usage"], st["unsafe_heap_memory"]
        num += u
        den += t
        if t > 0:
            pcts.append(u / t * 100.0)
    return pcts, num, den


def counters(ds, suffix):
    return load(ds, "unsafeinstfrequency_rq3", f"unsafe_counter_{suffix}")


def rq3(ds, suffix):
    d = counters(ds, suffix)
    per = d["per_crate_data"]
    pcts = [r["unsafe_inst_pct"] for r in per.values() if r["total_instructions"] > 0]
    s = d["summary"]
    return pcts, s["unsafe_instructions"], s["total_instructions"]


def rq5(ds, suffix, key):
    d = counters(ds, suffix)
    per = d["per_crate_data"]
    if key == "distinctive":
        n, t = "unsafe_functions_executed", "total_functions_executed"
    else:
        n, t = "unsafe_function_calls", "total_function_calls"
    pcts = [r[n] / r[t] * 100.0 for r in per.values() if r[t] > 0]
    s = d["summary"]
    return pcts, s[n], s[t]


def row(label, pub, alld, unit="%"):
    """One metric, both runs, as a markdown table row."""
    (p_pcts, p_num, p_den) = pub
    (a_pcts, a_num, a_den) = alld
    p = 100.0 * p_num / p_den if p_den else 0.0
    a = 100.0 * a_num / a_den if a_den else 0.0
    ps, as_ = spread(p_pcts), spread(a_pcts)
    return (f"| {label} | {p:.2f}{unit} | {a:.2f}{unit} | "
            f"{ps[1]:.2f} / {ps[2]:.2f} | {as_[1]:.2f} / {as_[2]:.2f} |")


def repaired_published() -> tuple[float, float]:
    """The published instruction totals with the duplicate builds averaged.

    Delegates to the audit tool so there is one implementation of the repair.
    """
    import audit_duplicate_stats as audit
    root = PUB.counter_root
    crates = sorted(p for p in root.iterdir()
                    if p.is_dir() and not p.name.startswith("_"))
    r = audit.audit_feature(crates, "unsafe_counter", "unsafe_counter", False)
    return r["kept"]["t"], r["kept"]["u"]


def main() -> int:
    repaired_total, repaired_unsafe = repaired_published()
    out: list[str] = []
    w = out.append
    w("# The dependency-include run beside the published run")
    w("")
    w("Both runs measure the same 100 crates with the same three instrumentation")
    w("features. They differ in what gets instrumented. The **published** run")
    w("instruments only the crate cargo marks as the primary package, so a")
    w("measurement counts that crate's own code and not the code of its")
    w("dependencies. The **dependency-include** run, finished 2026-09-10, sets")
    w("`UNSAFE_INSTRUMENT_ALL_PACKAGES=1` and instruments every crate in the")
    w("dependency graph, excluding build scripts and procedural macros, which run")
    w("at compile time rather than in the measured program.")
    w("")
    w("Two columns per run. **Corpus** sums the numerator and the denominator over")
    w("all 100 crates and then divides, so a large crate counts for more than a")
    w("small one. **Geomean / median** summarise the 100 per-crate percentages,")
    w("where every crate counts the same; these are the numbers the paper's tables")
    w("report.")
    w("")
    w("Read the difference with care. The two runs used different compiler builds")
    w("and different harness revisions, and for every question but the first a")
    w("different workload, because the published run built nearly every crate on")
    w("default features while this corpus honours each crate's recorded feature")
    w("selection. The clean measurement of what instrumenting dependencies does is")
    w("the matched 10-crate pilot, not this table.")
    w("")
    w("One more caveat for the instruction and function questions: the published")
    w("numbers count many test binaries twice, which inflates its absolute")
    w("instruction total by about 1.6 times and moves its corpus unsafe share from")
    w("6.41% to about 8.20%. The corpus column below quotes the published data as")
    w("published. DATA_AUDIT_2026-09-01.md has the detail.")
    w("")

    for variant, suffix, title in (
            ("without_native", "nativefalse",
             "Without standard libraries (unsafe code inside core/std/alloc not counted)"),
            ("with_native", "nativetrue",
             "With standard libraries (unsafe code inside core/std/alloc counted)")):
        w(f"## {title}")
        w("")
        w("| Metric | Published, corpus | Dep-include, corpus | Published, geomean / median | Dep-include, geomean / median |")
        w("|---|---|---|---|---|")
        w(row("RQ1 CPU cycles in unsafe code", rq1(PUB, variant), rq1(ALL, variant)))
        w(row("RQ2 heap bytes reached by unsafe code", rq2(PUB, variant), rq2(ALL, variant)))
        w(row("RQ3 instructions executed that are unsafe", rq3(PUB, suffix), rq3(ALL, suffix)))
        w(row("RQ5 functions executed that hold unsafe code",
              rq5(PUB, suffix, "distinctive"), rq5(ALL, suffix, "distinctive")))
        w(row("RQ5 function calls into code holding unsafe",
              rq5(PUB, suffix, "cumulative"), rq5(ALL, suffix, "cumulative")))
        w("")
        w("RQ1 is not one metric across the two columns. The published run divides")
        w("internal cycles, meaning total cycles minus time in uninstrumented code;")
        w("the dependency-include run divides whole-program cycles with no")
        w("correction, because the internal/external split stops meaning what it did")
        w("once dependencies are instrumented. datasets.py explains why.")
        w("")

    w("## RQ4: what kind of instruction the unsafe ones are")
    w("")
    w("Each cell is that type's share of all unsafe instructions, summed over the")
    w("corpus, without standard libraries.")
    w("")
    w("| Instruction type | Published | Dep-include |")
    w("|---|---|---|")
    types = [("Load", ["unsafe_loads"]), ("Store", ["unsafe_stores"]),
             ("Pointer arithmetic", ["unsafe_geps"]), ("Cast", ["unsafe_casts"]),
             ("Call", ["unsafe_calls_direct", "unsafe_calls_indirect",
                       "unsafe_calls_intrinsic"]),
             ("Other", ["unsafe_others", "unsafe_atomics"])]
    sp = counters(PUB, "nativefalse")["summary"]
    sa = counters(ALL, "nativefalse")["summary"]
    for label, keys in types:
        p = 100.0 * sum(sp[k] for k in keys) / sp["unsafe_instructions"]
        a = 100.0 * sum(sa[k] for k in keys) / sa["unsafe_instructions"]
        w(f"| {label} | {p:.2f}% | {a:.2f}% |")
    w("")
    w("## How much bigger the measured program got")
    w("")
    w("Only two quantities can honestly be compared in absolute terms. The")
    w("published instruction counts are the ones the duplicate-counting defect")
    w("inflates, so the middle column repairs them: it groups the published stat")
    w("files by test binary and by build, averages the repeated builds instead of")
    w("summing them, and drops builds whose counters are not credible. That is the")
    w("same repair audit_duplicate_stats.py performs and reports.")
    w("")
    w("| Quantity, without standard libraries | Published, as published | Published, repaired | Dep-include | Dep-include over repaired |")
    w("|---|---|---|---|---|")
    w(f"| Instructions executed | {sp['total_instructions']:,} | "
      f"{repaired_total:,.0f} | {sa['total_instructions']:,} | "
      f"{sa['total_instructions']/repaired_total:.2f}x |")
    w(f"| Unsafe instructions executed | {sp['unsafe_instructions']:,} | "
      f"{repaired_unsafe:,.0f} | {sa['unsafe_instructions']:,} | "
      f"{sa['unsafe_instructions']/repaired_unsafe:.2f}x |")
    hp = sum(r["aggregated_stats"]["total_heap_usage"]
             for r in load(PUB, "heaptracker_rq2", "heap_without_native")["crates"])
    ha = sum(r["aggregated_stats"]["total_heap_usage"]
             for r in load(ALL, "heaptracker_rq2", "heap_without_native")["crates"])
    w(f"| Heap bytes allocated | {hp:,} | {hp:,} | {ha:,} | {ha/hp:.2f}x |")
    w("")
    w("Heap bytes need no repair: only three heap binaries were measured twice in")
    w("the published run, and repairing those three moves the corpus heap share by")
    w("more than the defect does, so the published heap sum is left alone.")
    w("")
    w("Do not read these ratios as the cost of instrumenting dependencies. The two")
    w("runs also differ in compiler build, harness revision and per-crate feature")
    w("selection. The matched 10-crate pilot, where only the instrumentation scope")
    w("changed, measured 1.21 times as many instructions executed.")
    w("")

    p = HERE / "ALLDEPS_VS_PUBLISHED.md"
    p.write_text("\n".join(out) + "\n")
    print(f"wrote {p.name} ({len(out)} lines)")

    write_latex_table()
    return 0


def write_latex_table() -> None:
    """One table putting the two instrumentation scopes beside each other.

    Each cell is the geomean and the median of the 100 per-crate percentages,
    which is how the paper's own tables summarise a research question. The
    caption states outright that the two columns are not a controlled pair, so
    the table cannot be read as the cost of instrumenting dependencies on its
    own.
    """
    rows = []
    for label, pub, alld in (
            ("CPU cycles in unsafe code",
             rq1(PUB, "without_native"), rq1(ALL, "without_native")),
            ("Heap bytes reached by unsafe code",
             rq2(PUB, "without_native"), rq2(ALL, "without_native")),
            ("Instructions executed that are unsafe",
             rq3(PUB, "nativefalse"), rq3(ALL, "nativefalse")),
            ("Functions executed holding unsafe code",
             rq5(PUB, "nativefalse", "distinctive"),
             rq5(ALL, "nativefalse", "distinctive")),
            ("Calls into code holding unsafe",
             rq5(PUB, "nativefalse", "cumulative"),
             rq5(ALL, "nativefalse", "cumulative"))):
        ps, as_ = spread(pub[0]), spread(alld[0])
        rows.append(f"{label} & {ps[1]:.1f}\\% & {ps[2]:.1f}\\% "
                    f"& {as_[1]:.1f}\\% & {as_[2]:.1f}\\% \\\\\n")

    latex = (
        "\\begin{table}[tb]\n"
        "\\caption{The same measurements under two instrumentation scopes,\n"
        "without standard libraries. \\textbf{Primary only} instruments the\n"
        "crate cargo marks as the primary package; \\textbf{All dependencies}\n"
        "instruments every crate in the dependency graph. Each cell summarises\n"
        "the 100 per-crate percentages. The two runs also differ in compiler\n"
        "build, harness revision and per-crate feature selection, so the gap\n"
        "between the columns is not the effect of instrumenting dependencies\n"
        "alone.}\n"
        "\\label{table:scope_comparison}\n"
        "\\centering\n"
        "{\\sffamily\n"
        "\\begin{tabular}{@{}l|rr|rr@{}}\n"
        "\\toprule\n"
        "& \\multicolumn{2}{c|}{\\textbf{Primary only}}"
        " & \\multicolumn{2}{c}{\\textbf{All dependencies}} \\\\\n"
        "\\cmidrule(lr){2-3} \\cmidrule(lr){4-5}\n"
        "\\textbf{Measurement} & \\textbf{Geomean} & \\textbf{Median}"
        " & \\textbf{Geomean} & \\textbf{Median} \\\\\n"
        "\\midrule\n"
        + "".join(rows) +
        "\\bottomrule\n"
        "\\end{tabular}\n"
        "}\n"
        "\\end{table}\n"
    )
    out = HERE / "Latex" / "tables" / "scope_comparison.tex"
    out.write_text(latex)
    print(f"wrote {out.relative_to(HERE)}")


if __name__ == "__main__":
    sys.exit(main())
