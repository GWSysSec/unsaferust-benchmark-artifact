"""The measurement runs the table and figure scripts can read.

`published` is the run the paper reports: CPU cycles come from `rebench_v3`,
everything else from `rebench_v2`. `yourrun` is a measurement of your own,
registered when ARTIFACT_RUN_DIR names its directory. Every root can be
overridden by an environment variable.
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
    scope: str = "primary package only"

    def stem(self, base: str) -> str:
        return f"{base}{self.suffix}"


PUBLISHED = Dataset(
    name="published",
    counter_root=_root("REBENCH_V2_ROOT", "rebench_v2"),
    heap_root=_root("REBENCH_V2_ROOT", "rebench_v2"),
    cpu_root=_root("REBENCH_V3_ROOT", "rebench_v3"),
    suffix="",
)

DATASETS = {PUBLISHED.name: PUBLISHED}

_RUN_DIR = os.environ.get("ARTIFACT_RUN_DIR")
if _RUN_DIR:
    DATASETS["yourrun"] = Dataset(
        name="yourrun",
        counter_root=Path(_RUN_DIR),
        heap_root=Path(_RUN_DIR),
        cpu_root=Path(_RUN_DIR),
        suffix="_yourrun",
    )


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
        help="which measurement run to read (default: published). "
             "'yourrun' appears when ARTIFACT_RUN_DIR names a measurement "
             "directory of your own.",
    )
