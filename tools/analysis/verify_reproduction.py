#!/usr/bin/env python3
"""Compare a fresh measurement run against the numbers this repository ships.

This is the check an artifact evaluator runs. Point it at the output directory
the measurement harness wrote and say which of our datasets it should reproduce;
it aggregates the fresh run with the same code that produced our numbers and
reports, per research question, how many crates agree.

    python3 verify_reproduction.py /path/to/rebench_output --dataset alldeps

What "agree" means, and why it is not "identical".

Rerunning this corpus does not give identical numbers. We measured that: the
dependency-include run repeated its whole instruction-counter pass over all 100
crates, and comparing the two repetitions, 112 of the 200 crate-and-variant
pairs agreed to better than one part in ten thousand, 59 more agreed to within
1%, 22 to within 10%, and 7 differed by more than 10%. The 7 belong to four
crates -- petgraph, zopfli, portable-atomic and http -- whose test suites drive
randomly generated input, so each run does a different amount of work.
petgraph's unsafe instruction count moved by 73% between our own two runs.

So this tool grades each crate against a tolerance and reports the distribution,
rather than demanding equality. A crate outside tolerance is worth looking at;
a handful of them, especially the four named above, is the expected outcome.

CPU cycles are judged differently again. Cycle counts depend on the machine, so
comparing them against ours means nothing. What should carry over is the ratio:
the share of cycles spent in unsafe code. That is what this tool compares.
"""

from __future__ import annotations

import argparse
import json
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import datasets  # noqa: E402
from aggregate_rebench import aggregate_all  # noqa: E402

VARIANTS = [("without_native", "nativefalse"), ("with_native", "nativetrue")]

# Grades, from the repeated-run measurement described above.
GRADES = [("exact", 1e-4), ("tight", 1e-2), ("loose", 1e-1)]
KNOWN_VARIABLE = {"petgraph", "zopfli", "portable-atomic", "http"}


def grade(rel: float) -> str:
    for name, bound in GRADES:
        if rel < bound:
            return name
    return "outside"


def rel_diff(fresh: float, ours: float) -> float:
    if ours == 0:
        return 0.0 if fresh == 0 else float("inf")
    return abs(fresh - ours) / abs(ours)


def load_ours(ds: datasets.Dataset, folder: str, base: str) -> dict:
    return json.loads((HERE / folder / f"{ds.stem(base)}.json").read_text())


def report(title: str, results: list[tuple[str, str, float]]) -> list[str]:
    """One research question's verdict. results = (crate, variant, rel diff)."""
    lines = [f"\n--- {title} ---"]
    if not results:
        lines.append("  no crate in the fresh run could be compared")
        return lines
    counts = {"exact": 0, "tight": 0, "loose": 0, "outside": 0}
    for _c, _v, rel in results:
        counts[grade(rel)] += 1
    n = len(results)
    lines.append(f"  compared {n} crate-and-variant pairs")
    for k in ("exact", "tight", "loose", "outside"):
        label = {"exact": "agree to 1 part in 10,000",
                 "tight": "agree to within 1%",
                 "loose": "agree to within 10%",
                 "outside": "differ by more than 10%"}[k]
        lines.append(f"    {counts[k]:4d}  {label}")
    worst = sorted(results, key=lambda r: -r[2])[:8]
    if worst and worst[0][2] >= 1e-2:
        lines.append("  largest differences:")
        for c, v, rel in worst:
            if rel < 1e-2:
                break
            note = "  (randomised workload; expected)" if c in KNOWN_VARIABLE else ""
            lines.append(f"    {c:22s} {v:15s} {100*rel:8.2f}%{note}")
    finite = [r for _c, _v, r in results if r != float("inf")]
    if finite:
        lines.append(f"  median difference: {100*statistics.median(finite):.4f}%")
    return lines


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("run_dir", type=Path,
                    help="the measurement harness output directory to check")
    datasets.add_argument(ap)
    args = ap.parse_args()

    ds = datasets.get(args.dataset)
    if not args.run_dir.is_dir():
        raise SystemExit(f"no such run directory: {args.run_dir}")

    print(f"fresh run : {args.run_dir}")
    print(f"comparing against dataset '{ds.name}' ({ds.scope})")
    fresh = aggregate_all(root=args.run_dir, heap_common_bins_only=True,
                          keep_test_failures=ds.keep_test_failures)
    print(f"crates in the fresh run: {len(fresh)}")

    out: list[str] = []

    # RQ1: the share of cycles in unsafe code, not the cycle counts themselves.
    res = []
    for variant, _s in VARIANTS:
        base = ("cpucycle_withnative" if variant == "with_native"
                else "cpucycle_withoutnative")
        ours = load_ours(ds, "cpucyclecount_rq1", base)["crates"]
        num_f = ("whole_program_unsafe_cycles" if ds.cycle_metric == "whole_program"
                 else "internal_unsafe_cycles")
        den_f = ("whole_program_total_cycles" if ds.cycle_metric == "whole_program"
                 else "internal_total_cycles")
        for crate, cc in fresh.items():
            o = ours.get(crate)
            d = cc.get("cpu_cycle_counter", {}).get(variant)
            if not o or not d or d.get(den_f, 0) <= 0:
                continue
            pct = d[num_f] / d[den_f] * 100.0
            res.append((crate, variant, rel_diff(pct, o["unsafe_percentage"])))
    out += report("RQ1  share of CPU cycles spent in unsafe code", res)

    # RQ2: heap bytes reached by unsafe code.
    res = []
    for variant, _s in VARIANTS:
        ours = {r["crate_name"]: r["aggregated_stats"]
                for r in load_ours(ds, "heaptracker_rq2",
                                   f"heap_{variant}")["crates"]}
        for crate, cc in fresh.items():
            o = ours.get(crate)
            d = cc.get("heap_tracker", {}).get(variant)
            if not o or not d:
                continue
            res.append((crate, variant,
                        rel_diff(d.get("unsafe_heap_memory", 0),
                                 o.get("unsafe_heap_memory", 0))))
    out += report("RQ2  heap bytes reached by unsafe code", res)

    # RQ3 to RQ5: the instruction and function counters.
    for label, field in (("RQ3  unsafe instructions executed", "unsafe_instructions"),
                         ("RQ4  unsafe pointer-arithmetic instructions", "unsafe_geps"),
                         ("RQ5  calls into functions holding unsafe code",
                          "unsafe_function_calls")):
        res = []
        for variant, suffix in VARIANTS:
            ours = load_ours(ds, "unsafeinstfrequency_rq3",
                             f"unsafe_counter_{suffix}")["per_crate_data"]
            for crate, cc in fresh.items():
                o = ours.get(crate)
                d = cc.get("unsafe_counter", {}).get(variant)
                if not o or not d:
                    continue
                res.append((crate, variant,
                            rel_diff(d.get(field, 0), o.get(field, 0))))
        out += report(label, res)

    print("\n".join(out))
    print("\nA handful of crates outside 10% is the expected outcome, not a")
    print("failure. Compare the named crates against the list of randomised")
    print("workloads in this script's header before concluding anything.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
