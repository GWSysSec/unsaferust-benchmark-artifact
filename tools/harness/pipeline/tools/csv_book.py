"""
csv_book.py — bookkeeper for "Final Crates - Sheet6_updated.csv".

reads the per-crate baseline (api%, loc cov%, workspace pathway), and
appends post-generation columns after the pipeline runs.

new columns appended (idempotent — added only if missing):
- TEST API% (post-gen)
- TEST LOC COV% (post-gen)
- Tests Generated
- Iterations Used
"""

import csv
import logging
from pathlib import Path

log = logging.getLogger(__name__)

POSTGEN_COLUMNS = [
    "TEST API% (post-gen)",
    "TEST LOC COV% (post-gen)",
    "Tests Generated",
    "Iterations Used",
]


def load_csv(csv_path: Path) -> tuple[list[str], list[dict]]:
    """load csv, return (fieldnames, rows). filters out blank-name rows."""
    csv_path = Path(csv_path)
    with open(csv_path, newline="", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        fieldnames = list(reader.fieldnames or [])
        rows: list[dict] = []
        for row in reader:
            name = (row.get("name") or "").strip()
            if name:
                rows.append(row)

    # ensure post-gen columns exist
    for col in POSTGEN_COLUMNS:
        if col not in fieldnames:
            fieldnames.append(col)
            for row in rows:
                row.setdefault(col, "")

    return fieldnames, rows


def update_row_postgen(
    row: dict,
    api_pct_post: float,
    line_cov_pct_post: float,
    tests_generated: int,
    iterations_used: int,
):
    """write post-gen fields onto a single CSV row in-place."""
    row["TEST API% (post-gen)"] = f"{api_pct_post:.4f}"
    row["TEST LOC COV% (post-gen)"] = f"{line_cov_pct_post:.4f}"
    row["Tests Generated"] = str(tests_generated)
    row["Iterations Used"] = str(iterations_used)


def update_row_baseline(
    row: dict,
    api_pct: float,
    line_cov_pct: float,
    workspace_pathway: str = "",
    description: str = "",
):
    """write pre-gen fields. used by the legacy profile-only path."""
    row["TEST API%"] = f"{api_pct:.4f}"
    row["TEST LOC COV%"] = f"{line_cov_pct:.4f}"
    if workspace_pathway:
        row["Workspace pathway"] = workspace_pathway
    if description:
        row["Description"] = description


def write_csv(csv_path: Path, fieldnames: list[str], rows: list[dict]):
    """atomically rewrite the CSV with current rows."""
    csv_path = Path(csv_path)
    tmp_path = csv_path.with_suffix(csv_path.suffix + ".tmp")
    with open(tmp_path, "w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames, extrasaction="ignore")
        writer.writeheader()
        for row in rows:
            writer.writerow(row)
    tmp_path.replace(csv_path)


def baseline_in_band(row: dict, lo: float, hi: float) -> bool:
    """check if existing TEST API% (pre-gen baseline from CSV) falls within [lo, hi]."""
    raw = (row.get("TEST API%") or "").strip()
    if not raw:
        return False
    try:
        v = float(raw)
    except ValueError:
        return False
    return lo <= v <= hi


def baseline_values(row: dict) -> tuple[float, float] | None:
    """parse (api_pct, line_cov_pct) from CSV baseline columns. None if missing."""
    api_raw = (row.get("TEST API%") or "").strip()
    loc_raw = (row.get("TEST LOC COV%") or "").strip()
    if not api_raw and not loc_raw:
        return None
    try:
        api = float(api_raw) if api_raw else 0.0
        loc = float(loc_raw) if loc_raw else 0.0
    except ValueError:
        return None
    return api, loc
