#!/usr/bin/env python3
"""Build one of the paper's tables from YOUR measurement, and compare.

This is the artifact's main path. You run the harness, and this builds the
table out of what you measured, then puts it beside the table in the paper so
you can see whether they say the same thing. Our own measurement data is in the
artifact too, but only as a third column to compare against, never as the input.

    run/measure.sh --tier smoke                  # measure
    tables/rq3_unsafe_inst_frequency.sh          # build the table from it

By default it uses the most recent directory under results/. Point it somewhere
else with --run.

One thing to watch. A table summarises the crates that were measured, so a run
over a subset produces a subset table, and the paper's is over 100. Those two
are not the same statistic and the script says so rather than letting the
numbers be compared silently. Only `--tier full` produces a table over the same
population as the paper's.

To check our shipped data instead of your own measurement -- useful to confirm
the data in this artifact is what the paper reports -- pass --our-data.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ANALYSIS = ROOT / "tools" / "analysis"
PAPER_TABLES = ROOT / "data" / "paper_tables"
BUILT = ANALYSIS / "Latex" / "tables"
BUILT_FIG = ANALYSIS / "Latex" / "figures"
RESULTS = ROOT / "results"

TARGETS = {
    "rq1_cpu_cycles": ("RQ1 and RQ6: the share of CPU cycles spent in unsafe code",
                       "cpucyclecount_rq1/generate_cpu_cycles_table.py",
                       "cpu_cycles", "table"),
    "rq2_heap": ("RQ2: heap memory reached by unsafe code, by allocation size",
                 "heaptracker_rq2/generate_heap_table.py", "heap", "table"),
    "rq3_unsafe_inst_frequency": ("RQ3: how often executed instructions are unsafe",
                                  "unsafeinstfrequency_rq3/generate_table.py",
                                  "unsafe_inst_frequency", "table"),
    "rq4_inst_types": ("RQ4: what kind of instruction the unsafe ones are",
                       "unsafeinsttype_rq4/generate_unsafe_inst_table.py",
                       "inst", "table"),
    "rq5_unsafe_functions": ("RQ5: how often functions holding unsafe code run",
                             "unsafefunction_rq5/generate_unsafe_function_table.py",
                             "function", "table"),
    "figure_cycles_cdf": ("RQ1 and RQ6 figure: distribution of the unsafe cycle share",
                          "cpucyclecount_rq1/comparative_plots.py",
                          "cumulative_frequency_unsafe_execution_rq1_rq6", "figure"),
    "figure_heap_cdf": ("RQ2 figure: distribution of the unsafe heap share",
                        "heaptracker_rq2/generate_heap_plot.py",
                        "heap_native_cdf", "figure"),
}


def newest_run() -> Path | None:
    if not RESULTS.is_dir():
        return None
    runs = [p for p in RESULTS.iterdir()
            if p.is_dir() and any(p.glob("*/rebench_summary.json"))]
    return max(runs, key=lambda p: p.stat().st_mtime) if runs else None


SCOPE_WORDS = {
    "primary": "only the crate under study",
    "alldeps": "every crate in the dependency graph",
}


def scope_of(run: Path) -> str:
    """Read what the run instrumented out of the run itself.

    The harness writes `instrument_all_deps` into every crate's
    rebench_summary.json, so the measurement says which of the two scopes it
    used and the evaluator does not have to remember. A run whose crates
    disagree is not a single measurement, so say so rather than pick one.
    """
    seen = set()
    for s in run.glob("*/rebench_summary.json"):
        try:
            seen.add(bool(json.loads(s.read_text()).get("instrument_all_deps")))
        except (OSError, ValueError):
            continue
    if len(seen) > 1:
        print("warning: this run mixes both scopes; treating it as primary-only")
        return "primary"
    return "alldeps" if seen == {True} else "primary"


def keep(built: Path, run_dir: Path | None) -> None:
    """Copy a generated table or figure next to the run it was built from.

    Everything the generators write lands under tools/analysis/Latex/, which is
    inside the image when this runs in Docker and is therefore gone when the
    container exits. The run directory is on the host, so a copy there is what
    the evaluator still has afterwards.
    """
    if run_dir is None or not built.is_file():
        return
    out = run_dir / "tables"
    out.mkdir(parents=True, exist_ok=True)
    shutil.copy2(built, out / built.name)
    print(f"kept a copy at {out / built.name}")


def crates_in(run: Path) -> list[str]:
    return sorted(p.name for p in run.iterdir()
                  if p.is_dir() and (p / "rebench_summary.json").is_file())


def sh(cmd: list[str], env: dict | None = None) -> tuple[int, str]:
    e = dict(os.environ)
    if env:
        e.update(env)
    r = subprocess.run(cmd, capture_output=True, text=True, cwd=str(ANALYSIS), env=e)
    return r.returncode, r.stdout + r.stderr


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("target", choices=sorted(TARGETS))
    ap.add_argument("--run", type=Path, help="a measurement directory of your own")
    ap.add_argument("--scope", default=None, choices=["primary", "alldeps"],
                    help="what the run instrumented; read from the run itself "
                         "unless you say otherwise")
    ap.add_argument("--our-data", action="store_true",
                    help="build from the data this artifact ships instead")
    args = ap.parse_args()

    what, rel, stem, kind = TARGETS[args.target]
    BUILT.mkdir(parents=True, exist_ok=True)
    BUILT_FIG.mkdir(parents=True, exist_ok=True)
    print(f"{args.target}: {what}\n")

    if args.our_data:
        dataset, env, suffix, run_dir = "published", {}, "", None
        print("source: the measurement data this artifact ships")
        print("        (use this to confirm our data is what the paper reports;")
        print("         the artifact's main path is to measure it yourself)\n")
        n_crates = 100
    else:
        run = args.run or newest_run()
        if run is None:
            print("No measurement of your own was found under results/.\n")
            print("Measure first, for example:")
            print("    run/measure.sh --tier smoke      12 crates, about 32 minutes")
            print("    run/measure.sh --tier fast       58 crates, about 1.5 hours")
            print("    run/measure.sh --tier full      100 crates, 121.7 hours\n")
            print("Or pass --our-data to build the table from the data we ship.")
            return 2
        crates = crates_in(run)
        n_crates = len(crates)
        print(f"source: your measurement at {run}")
        print(f"        {n_crates} crates: {', '.join(crates[:8])}"
              f"{' ...' if n_crates > 8 else ''}\n")
        scope = args.scope or scope_of(run)
        print(f"        scope: {SCOPE_WORDS[scope]}\n")
        dataset, suffix = "yourrun", "_yourrun"
        run_dir = run.resolve()
        env = {"ARTIFACT_RUN_DIR": str(run_dir),
               "ARTIFACT_RUN_SCOPE": scope}
        code, out = sh([sys.executable, str(ANALYSIS / "export_dataset.py"),
                        "--dataset", "yourrun"], env)
        if code != 0:
            print("could not aggregate your run:")
            print(out)
            return 1

    code, out = sh([sys.executable, str(ANALYSIS / rel), "--dataset", dataset], env)
    if code != 0:
        print("the table generator failed:")
        print(out)
        return 1
    for line in out.splitlines():
        if line.strip():
            print(f"  | {line}")

    if kind == "figure":
        built = BUILT_FIG / f"{stem}{suffix}.png"
        print(f"\nfigure written to {built}")
        for ext in (".png", ".pdf"):
            keep(BUILT_FIG / f"{stem}{suffix}{ext}", run_dir)
        return 0

    built = BUILT / f"{stem}{suffix}.tex"
    keep(built, run_dir)
    paper = PAPER_TABLES / f"{stem}.tex"
    print(f"\n{'-'*70}\nyour table\n{'-'*70}")
    print(built.read_text())
    print(f"{'-'*70}\nthe same table in the paper\n{'-'*70}")
    print(paper.read_text() if paper.is_file() else "(not in the submitted paper)")

    if not args.our_data:
        if n_crates < 100:
            print("=" * 70)
            print(f"This table summarises {n_crates} crates. The paper's summarises 100.")
            print("Min, geomean, median and max over different populations are not")
            print("the same statistic, so read them as a sanity check, not as a")
            print("reproduction. Only --tier full covers the paper's population.")
            print("=" * 70)
        print("\nper-crate comparison against our measurement:")
        sys.stdout.flush()   # the child writes straight to the terminal
        subprocess.run([sys.executable, str(ANALYSIS / "verify_reproduction.py"),
                        str(run_dir),
                        "--dataset", "published" if scope == "primary" else "alldeps"],
                       cwd=str(ANALYSIS))
    elif paper.is_file():
        same = built.read_text() == paper.read_text()
        print("=" * 70)
        print("our data reproduces the paper's table exactly" if same
              else "our data does NOT reproduce the paper's table")
        print("=" * 70)
        return 0 if same else 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
