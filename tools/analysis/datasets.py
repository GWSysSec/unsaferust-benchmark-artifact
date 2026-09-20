"""The measurement runs the research-question scripts can read.

A *dataset* is one complete measurement run over the 100-crate corpus. Each
research question reads exactly one dataset at a time, and every table and
figure script in this repository takes `--dataset <name>` to choose which.

Two datasets exist.

`published` is the run the submitted paper reports. Only the crate cargo marks
as the primary package is instrumented, so a measurement counts the crate's own
code and not the code of its dependencies. It is split across two directories
for historical reasons: the CPU-cycle numbers come from `rebench_v3`, a
cycle-only rerun with a corrected counter, and everything else from
`rebench_v2`.

`alldeps` is the run finished on 2026-09-10 with the environment variable
`UNSAFE_INSTRUMENT_ALL_PACKAGES=1` set, which instruments every crate in the
dependency graph rather than only the primary one. All three measurement
features live in one directory, `rebench_alldeps_corpus`.

The CPU-cycle ratio is read differently in the two runs, and the difference is
not cosmetic. Under `published` the ratio divides *internal* cycles, that is
total cycles minus the cycles spent in code the run does not instrument. Under
`alldeps` that correction is wrong: the external-call tracker calls a callee
external whenever it has no body in the current module, which is true of every
cross-crate call whether or not the callee is instrumented, so the corrected
denominator would drop most dependency execution while the numerator keeps the
dependency unsafe time. `alldeps` therefore divides whole-program cycles, with
no correction at all. Each dataset carries its choice in `cycle_metric`.

The two runs also need different treatment of test binaries that exited 101,
which is how a Rust test binary reports a failed assertion. Such a binary ran
its whole workload and dumped complete counters; only its test verdict is
negative. The published run's summary files do not list the generated coverage
workload binaries at all, because a later top-up added them without rewriting
the summaries, so those binaries were never filtered out and their measurements
are inside the published numbers. The dependency-include run does list them,
with their real exit codes. Filtering on exit 101 in the new run but not the old
one removes nearly all unsafe execution from chrono, cpp_demangle, flate2 and
num-bigint -- for chrono, 198,863,189 of its 198,863,209 unsafe instructions --
and makes the two runs incomparable for those crates. So `alldeps` keeps
exit-101 binaries (`keep_test_failures=True`) and `published` does not, which is
what puts the same set of binaries in both. Corpus-wide the choice is small: the
share of executed instructions that are unsafe moves from 9.3552% to 9.3803%.
Binaries that timed out (124) or aborted (-6) are dropped under either setting;
they write no stat file at all.

Every root may be overridden by an environment variable so that an artifact
evaluator can place the measurement data anywhere.
"""

from __future__ import annotations

import os
from dataclasses import dataclass
from pathlib import Path

_DEFAULT_RUNS = Path("/home/oscar/Projects/unsafebench/rusttest-gen")


def _root(env_name: str, default_leaf: str) -> Path:
    return Path(os.environ.get(env_name, str(_DEFAULT_RUNS / default_leaf)))


@dataclass(frozen=True)
class Dataset:
    name: str
    counter_root: Path      # unsafe_counter: RQ3, RQ4, RQ5
    heap_root: Path         # heap_tracker: RQ2
    cpu_root: Path          # cpu_cycle_counter: RQ1
    suffix: str             # appended to output file stems ("" for published)
    cycle_metric: str       # "internal" or "whole_program"
    scope: str              # plain-English description of what was instrumented
    keep_test_failures: bool  # see the note below

    def stem(self, base: str) -> str:
        """'cpucycle_withnative' -> 'cpucycle_alldeps_withnative' for alldeps."""
        return f"{base}{self.suffix}"


PUBLISHED = Dataset(
    name="published",
    counter_root=_root("REBENCH_V2_ROOT", "rebench_v2"),
    heap_root=_root("REBENCH_V2_ROOT", "rebench_v2"),
    cpu_root=_root("REBENCH_V3_ROOT", "rebench_v3"),
    suffix="",
    cycle_metric="internal",
    scope="primary package only",
    keep_test_failures=False,
)

ALLDEPS = Dataset(
    name="alldeps",
    counter_root=_root("REBENCH_ALLDEPS_ROOT", "rebench_alldeps_corpus"),
    heap_root=_root("REBENCH_ALLDEPS_ROOT", "rebench_alldeps_corpus"),
    cpu_root=_root("REBENCH_ALLDEPS_ROOT", "rebench_alldeps_corpus"),
    suffix="_alldeps",
    cycle_metric="whole_program",
    scope="every crate in the dependency graph",
    keep_test_failures=True,
)

DATASETS = {d.name: d for d in (PUBLISHED, ALLDEPS)}


def get(name: str) -> Dataset:
    try:
        return DATASETS[name]
    except KeyError:
        raise SystemExit(
            f"unknown dataset {name!r}; choose one of {sorted(DATASETS)}"
        ) from None


def add_argument(parser) -> None:
    """Give an argparse parser the standard --dataset option."""
    parser.add_argument(
        "--dataset",
        default="published",
        choices=sorted(DATASETS),
        help="which measurement run to read (default: published)",
    )
