"""
recipes.py — per-crate recipe records and generated-test persistence.

a recipe captures everything downstream benchmarking needs to replay coverage
and rerun the generated test suite (cargo args, target dirs, package names,
api/loc% snapshots before and after generation).

generated tests that pass compile+run are written into
recipes/<crate>/tests/<test_name>.rs as a portable archive alongside the
recipe json. callers may also mirror them into bootcamp/<crate>/<rel>.
"""

import json
import logging
from dataclasses import dataclass, field, asdict
from pathlib import Path

log = logging.getLogger(__name__)


@dataclass
class Recipe:
    crate: str = ""
    primary_package_name: str = ""
    primary_lib_name: str = ""
    workspace_pathway: str = ""
    members_profiled: list = field(default_factory=list)
    feature_label: str = ""
    feature_args: list = field(default_factory=list)
    test_target_dir: str = ""
    cargo_test_cmd: str = ""

    # pre-generation (from CSV baseline or initial measure)
    api_total: int = 0
    api_covered: int = 0
    api_pct: float = 0.0
    line_cov_pct: float = 0.0

    # post-generation (after pipeline run)
    api_covered_post: int = 0
    api_pct_post: float = 0.0
    line_cov_pct_post: float = 0.0
    tests_generated: int = 0
    iterations_used: int = 0

    needs_test_generation: bool = False
    error: str = ""


def write_recipe(recipe: Recipe, recipes_dir: Path) -> Path:
    """emit recipes/<crate>.json."""
    recipes_dir = Path(recipes_dir)
    recipes_dir.mkdir(parents=True, exist_ok=True)
    out = recipes_dir / f"{recipe.crate}.json"
    out.write_text(json.dumps(asdict(recipe), indent=2))
    return out


def read_recipe(crate: str, recipes_dir: Path) -> Recipe | None:
    p = Path(recipes_dir) / f"{crate}.json"
    if not p.exists():
        return None
    try:
        data = json.loads(p.read_text())
        # tolerate older recipes lacking the new post-gen fields
        return Recipe(**{k: v for k, v in data.items() if k in Recipe.__dataclass_fields__})
    except Exception as e:
        log.warning(f"  failed to read recipe for {crate}: {e}")
        return None


def persist_test(
    test_file: Path,
    crate: str,
    recipes_dir: Path,
    bootcamp_dst: Path | None = None,
    rel_inside_crate: str = "tests",
) -> list[Path]:
    """copy a passing test into the recipe archive and (optionally) bootcamp.

    test_file: source path (in tmp working copy).
    rel_inside_crate: where the test should sit relative to the crate root
                     (e.g. "tests" for primary, "ouroboros/tests" for workspace).
    returns the list of paths written.
    """
    test_file = Path(test_file)
    if not test_file.exists():
        return []

    written: list[Path] = []
    fname = test_file.name

    # recipes archive: recipes/<crate>/tests/<fname>
    archive = Path(recipes_dir) / crate / "tests" / fname
    archive.parent.mkdir(parents=True, exist_ok=True)
    archive.write_bytes(test_file.read_bytes())
    written.append(archive)

    # bootcamp mirror: bootcamp/<crate>/<rel_inside_crate>/<fname>
    if bootcamp_dst is not None:
        bc = Path(bootcamp_dst) / crate / rel_inside_crate / fname
        bc.parent.mkdir(parents=True, exist_ok=True)
        bc.write_bytes(test_file.read_bytes())
        written.append(bc)

    return written
