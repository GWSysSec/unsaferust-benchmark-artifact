#!/usr/bin/env python3
"""Rebuild one table or figure from the shipped data and check it against the paper.

Every table in the paper is produced by a script from a data file. This runs
that script against the data this artifact ships and compares the result with
the table as it appears in the submitted paper. Nothing is measured and nothing
is random, so the answer is exact: the table either matches or it does not.

With --from-run it does the second, weaker check as well: aggregate a
measurement the evaluator made themselves and report how far each crate lands
from ours. That one is approximate by nature, because the corpus is not
deterministic; see ARCHITECTURE.md.

Usually invoked through the wrappers in tables/, not directly.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ANALYSIS = ROOT / "tools" / "analysis"
PAPER_TABLES = ROOT / "data" / "paper_tables"
PAPER_FIGURES = ROOT / "data" / "paper_figures"
BUILT_TABLES = ANALYSIS / "Latex" / "tables"
BUILT_FIGURES = ANALYSIS / "Latex" / "figures"

# key -> (what it is, generator relative to tools/analysis, output stem, kind)
TARGETS = {
    "rq1_cpu_cycles": (
        "RQ1 and RQ6: the share of CPU cycles spent in unsafe code",
        "cpucyclecount_rq1/generate_cpu_cycles_table.py", "cpu_cycles", "table"),
    "rq2_heap": (
        "RQ2: heap memory reached by unsafe code, by allocation size",
        "heaptracker_rq2/generate_heap_table.py", "heap", "table"),
    "rq3_unsafe_inst_frequency": (
        "RQ3: how often the instructions a program executes are unsafe",
        "unsafeinstfrequency_rq3/generate_table.py", "unsafe_inst_frequency", "table"),
    "rq4_inst_types": (
        "RQ4: what kind of instruction the unsafe ones are",
        "unsafeinsttype_rq4/generate_unsafe_inst_table.py", "inst", "table"),
    "rq5_unsafe_functions": (
        "RQ5: how often functions holding unsafe code run",
        "unsafefunction_rq5/generate_unsafe_function_table.py", "function", "table"),
    "figure_cycles_cdf": (
        "RQ1 and RQ6 figure: distribution of the unsafe cycle share",
        "cpucyclecount_rq1/comparative_plots.py",
        "cumulative_frequency_unsafe_execution_rq1_rq6", "figure"),
    "figure_heap_cdf": (
        "RQ2 figure: distribution of the unsafe heap share",
        "heaptracker_rq2/generate_heap_plot.py", "heap_native_cdf", "figure"),
}


def run_generator(rel: str, dataset: str) -> tuple[int, str]:
    BUILT_TABLES.mkdir(parents=True, exist_ok=True)
    BUILT_FIGURES.mkdir(parents=True, exist_ok=True)
    r = subprocess.run([sys.executable, str(ANALYSIS / rel), "--dataset", dataset],
                       capture_output=True, text=True, cwd=str(ANALYSIS))
    return r.returncode, r.stdout + r.stderr


def compare_table(stem: str, dataset: str) -> bool:
    suffix = "" if dataset == "published" else "_alldeps"
    built = BUILT_TABLES / f"{stem}{suffix}.tex"
    if not built.is_file():
        print(f"  the generator wrote no table at {built.name}")
        return False
    if dataset != "published":
        print(f"  built {built.name}")
        print("  no counterpart in the submitted paper to compare against:")
        print("  the dependency-include tables are new in the camera-ready.")
        print(built.read_text())
        return True
    paper = PAPER_TABLES / f"{stem}.tex"
    if not paper.is_file():
        print(f"  no submitted table named {paper.name} to compare against")
        return False
    if built.read_text() == paper.read_text():
        print(f"  {built.name} matches the submitted paper exactly")
        print(built.read_text())
        return True
    print(f"  {built.name} DIFFERS from the submitted paper:")
    subprocess.run(["diff", "-u", str(paper), str(built)])
    return False


def compare_figure(stem: str, dataset: str) -> bool:
    suffix = "" if dataset == "published" else "_alldeps"
    built = BUILT_FIGURES / f"{stem}{suffix}.png"
    if not built.is_file():
        print(f"  the generator wrote no figure at {built.name}")
        return False
    if dataset != "published":
        print(f"  built {built.name} (new in the camera-ready, nothing to compare)")
        return True
    paper = PAPER_FIGURES / f"{stem}.png"
    if not paper.is_file():
        print(f"  built {built.name}; no submitted figure to compare against")
        return True
    # PNG is deterministic here; PDF embeds a creation timestamp, so compare PNG.
    same = built.read_bytes() == paper.read_bytes()
    print(f"  {built.name} {'matches' if same else 'DIFFERS from'} "
          f"the submitted figure")
    print(f"  built: {built}")
    return same


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("target", choices=sorted(TARGETS))
    ap.add_argument("--dataset", default="published",
                    choices=["published", "alldeps"],
                    help="published: only the crate under study is instrumented, "
                         "which is what the paper reports. alldeps: every crate "
                         "in the dependency graph is instrumented.")
    ap.add_argument("--from-run", type=Path, metavar="DIR",
                    help="also check a measurement run of your own against ours")
    args = ap.parse_args()

    what, rel, stem, kind = TARGETS[args.target]
    print(f"{args.target}: {what}")
    print(f"dataset: {args.dataset}\n")

    code, out = run_generator(rel, args.dataset)
    if code != 0:
        print("the generator failed:")
        print(out)
        return 1
    for line in out.splitlines():
        if line.strip():
            print(f"  | {line}")
    print()

    ok = (compare_table(stem, args.dataset) if kind == "table"
          else compare_figure(stem, args.dataset))

    if args.from_run:
        print("\n" + "=" * 70)
        print("checking your own measurement run against our data")
        print("=" * 70)
        r = subprocess.run([sys.executable, str(ANALYSIS / "verify_reproduction.py"),
                            str(args.from_run), "--dataset", args.dataset],
                           cwd=str(ANALYSIS))
        if r.returncode != 0:
            ok = False

    print("\n" + ("RESULT: match" if ok else "RESULT: MISMATCH — see the diff above"))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
