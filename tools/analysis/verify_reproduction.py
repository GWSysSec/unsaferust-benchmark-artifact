#!/usr/bin/env python3
"""Compare a fresh measurement run with the numbers the paper reports.

    python3 verify_reproduction.py /path/to/rebench_output --dataset published

Every research question is compared as a share -- unsafe instructions out of
all executed instructions, unsafe heap bytes out of all heap bytes, and so on.
That is what the paper's tables report, and it is what carries across machines:
cycle counts and instruction counts depend on the hardware and on how much work
a test suite happened to do, while the ratio between them does not.

Rerunning does not give identical numbers and is not expected to. We measured
the spread by running the whole instruction counter twice over all 100 crates:
of the 200 crate-and-variant pairs, 112 agreed to better than one part in ten
thousand, 59 more to within 1%, 22 to within 10%, and 7 differed by more than
10%. The 7 belong to crates whose test suites generate their input, so each run
does a different amount of work. This tool therefore grades each crate against
a tolerance and reports the distribution. A difference below a tenth of a
percentage point counts as agreement, because a share near zero otherwise
produces enormous relative differences out of nothing.
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
# Crates whose tests generate their input, so each run does different work.
# slotmap was added after a fresh run put its main test binary at 3.41% unsafe
# against our 10.99%: its tests are driven by quickcheck.
KNOWN_VARIABLE = {"petgraph", "zopfli", "portable-atomic", "http", "slotmap"}


def grade(rel: float) -> str:
    for name, bound in GRADES:
        if rel < bound:
            return name
    return "outside"


def share(d: dict, numerator: str, denominator: str) -> float:
    """numerator/denominator out of one stat block, or 0 if the denominator is."""
    den = d.get(denominator, 0) or 0
    return (d.get(numerator, 0) or 0) / den if den else 0.0


def rel_diff(fresh: float, ours: float, floor: float = 0.0) -> float:
    """How far apart two shares are, relative to ours.

    `floor` is the size below which a difference is not worth reporting, in the
    same unit as the two values; every caller passes a tenth of a percentage
    point. Without it, two shares that are both near zero read as hundreds of
    percent apart.
    """
    if abs(fresh - ours) <= floor:
        return 0.0
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
            res.append((crate, variant,
                        rel_diff(pct, o["unsafe_percentage"], floor=0.1)))
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
                        rel_diff(share(d, "unsafe_heap_memory", "total_heap_usage"),
                                 share(o, "unsafe_heap_memory", "total_heap_usage"),
                                 floor=1e-3)))
    out += report("RQ2  share of heap bytes reached by unsafe code", res)

    # RQ3 to RQ5: the instruction and function counters.
    # RQ4 asks what fraction of the unsafe instructions are pointer arithmetic,
    # which is the question its table answers.
    for label, num, den in (
            ("RQ3  share of executed instructions that are unsafe",
             "unsafe_instructions", "total_instructions"),
            ("RQ4  share of unsafe instructions that are pointer arithmetic",
             "unsafe_geps", "unsafe_instructions"),
            ("RQ5  share of calls that enter a function holding unsafe code",
             "unsafe_function_calls", "total_function_calls")):
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
                            rel_diff(share(d, num, den), share(o, num, den),
                                     floor=1e-3)))
        out += report(label, res)

    print("\n".join(out))
    print("\nA handful of crates outside 10% is the expected outcome, not a")
    print("failure. slotmap, petgraph, zopfli, portable-atomic and http")
    print("generate their test input, so they do different work on every run,")
    print("and a crate's share is dominated by its largest test binary.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
