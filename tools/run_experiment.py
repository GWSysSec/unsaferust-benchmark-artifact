#!/usr/bin/env python3
"""Run one full-corpus experiment and preserve its primary-only comparisons."""
from __future__ import annotations

import argparse
import csv
import datetime
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
ANALYSIS = ROOT / "tools" / "analysis"
sys.path.insert(0, str(ANALYSIS))
import datasets
from heaptracker_rq2.generate_heap_table import collect_variant
from unsafeinsttype_rq4.generate_unsafe_inst_table import ROW_DEFS

EXPERIMENTS = {
    "heap": ("unsafe_counter,heap_tracker", ["rq2_heap", "figure_heap_cdf"]),
    "cpu": ("cpu_cycle_counter", ["rq1_cpu_cycles", "figure_cycles_cdf"]),
    "rq345": ("unsafe_counter", ["rq3_unsafe_inst_frequency", "rq4_inst_types",
                                "rq5_unsafe_functions"]),
}
VARIANTS = ("without_native", "with_native")


def run_command(command: list[str], log: Path, env: dict) -> int:
    """Stream progress to both the terminal and a persistent log."""
    with log.open("a") as stream:
        stream.write("\n$ " + " ".join(command) + "\n")
        stream.flush()
        with subprocess.Popen(command, cwd=ROOT, env=env, stdout=subprocess.PIPE,
                              stderr=subprocess.STDOUT, text=True) as process:
            for line in process.stdout:
                print(line, end="", flush=True)
                stream.write(line)
                stream.flush()
            return process.wait()


def load_records(analysis: Path, group: str, variant: str, suffix: str) -> dict:
    if group == "cpu":
        folder = "cpucyclecount_rq1"
        base = "cpucycle_withnative" if variant == "with_native" else "cpucycle_withoutnative"
    elif group == "heap":
        folder, base = "heaptracker_rq2", f"heap_{variant}"
    else:
        folder = "unsafeinstfrequency_rq3"
        base = "unsafe_counter_nativetrue" if variant == "with_native" else "unsafe_counter_nativefalse"
    data = json.loads((analysis / folder / f"{base}{suffix}.json").read_text())
    meta = data.get("metadata", {})
    expected = "yourrun" if suffix else "published"
    if meta.get("dataset") != expected or meta.get("instrumentation_scope") != "primary package only":
        raise ValueError(f"Unexpected reference or measurement scope: {folder}/{base}{suffix}")
    if group == "heap":
        return {r["crate_name"]: {**r["aggregated_stats"],
                "_mem_inst_common": r.get("mem_inst_common")} for r in data["crates"]}
    return data["crates"] if group == "cpu" else data["per_crate_data"]


def ratio(record: dict, numerator: str, denominator: str) -> float | None:
    den = record.get(denominator, 0) or 0
    return 100.0 * (record.get(numerator, 0) or 0) / den if den > 0 else None


def metrics(group: str, record: dict, variant: str) -> dict:
    if group == "cpu":
        return {"RQ1 unsafe CPU cycles": record["unsafe_percentage"]}
    if group == "heap":
        values = collect_variant({"crate": record}, variant)
        result = {"RQ2 " + key: value[0] if value else None
                  for key, value in values.items()}
        result["RQ2 unsafe heap bytes (exact)"] = ratio(record, "unsafe_heap_memory", "total_heap_usage")
        return result
    result = {"RQ3 unsafe executed instructions": ratio(record, "unsafe_instructions", "total_instructions")}
    total = record.get("unsafe_instructions", 0) or 0
    for label, fields in ROW_DEFS:
        # Match the paper generator's zero convention for instruction types.
        result["RQ4 " + label] = 100.0 * sum(record.get(k, 0) or 0 for k in fields) / total if total else 0.0
    result["RQ5 distinctive"] = ratio(record, "unsafe_functions_executed", "total_functions_executed")
    result["RQ5 cumulative"] = ratio(record, "unsafe_function_calls", "total_function_calls")
    return result


def compare(group: str, analysis: Path, output: Path, names: dict) -> bool:
    rows = []
    missing = []
    for variant in VARIANTS:
        published = load_records(analysis, group, variant, "")
        measured = load_records(analysis, group, variant, "_yourrun")
        for crate, display in names.items():
            reference = metrics(group, published[crate], variant)
            fresh = metrics(group, measured[crate], variant) if crate in measured else {}
            if crate not in measured:
                missing.append(f"{crate}/{variant}")
            for metric, before in reference.items():
                after = fresh.get(metric)
                status = ("missing_measurement" if crate not in measured else
                          "undefined_denominator" if before is None or after is None else "compared")
                delta = after - before if status == "compared" else None
                relative = (100.0 * abs(delta) / abs(before) if before else
                            0.0 if delta == 0 else None) if delta is not None else None
                rows.append({"crate": display, "directory": crate, "variant": variant,
                             "metric": metric, "measured_percent": after,
                             "published_percent": before, "delta_percentage_points": delta,
                             "relative_difference_percent": relative, "status": status})
    with (output / "comparison.csv").open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)
    lines = [f"Experiment: {group}; reference: published; scope: primary package only",
             f"Expected: {len(names)} crates x 2 standard-library variants.",
             f"Missing crate/variant measurements: {len(missing)}",
             "Percentages are compared using signed percentage-point deltas.",
             "Relative differences are blank when the reference is zero and the measurement is nonzero.",
             "Undefined denominators are blank, not missing measurements or zero shares."]
    for metric in dict.fromkeys(r["metric"] for r in rows):
        pairs = [r for r in rows if r["metric"] == metric and r["status"] == "compared"]
        lines.append(f"\n{metric}: {len(pairs)} comparable crate/variant pairs")
        for row in sorted(pairs, key=lambda r: abs(r["delta_percentage_points"]), reverse=True)[:5]:
            lines.append(f"  {row['crate']} / {row['variant']}: "
                         f"measured={row['measured_percent']:.6f}%, "
                         f"published={row['published_percent']:.6f}%, "
                         f"delta={row['delta_percentage_points']:+.6f} pp")
    if missing:
        lines.append("\nMissing: " + ", ".join(missing))
    lines.append("\nNumerical differences are reported without treating workload/hardware variation as failure.")
    text = "\n".join(lines) + "\n"
    (output / "comparison.txt").write_text(text)
    print(text)
    return not missing


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("experiment", choices=EXPERIMENTS)
    parser.add_argument("--out", type=Path, help="new output directory for this experiment")
    parser.add_argument("--compare-only", type=Path, help="compare an existing run without measuring")
    args = parser.parse_args()
    if args.out and args.compare_only:
        parser.error("--out and --compare-only are mutually exclusive")
    features, tables = EXPERIMENTS[args.experiment]
    with (ROOT / "corpus/crates.csv").open() as stream:
        names = {row["dir_name"]: row["crate"] for row in csv.DictReader(stream)}
    if len(names) != 100:
        raise ValueError(f"Expected exactly 100 corpus crates, found {len(names)}")
    env = dict(os.environ)
    for key in ("UNSAFE_INSTRUMENT_ALL_PACKAGES", "CARGO_PRIMARY_PACKAGE",
                "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "ARTIFACT_RUN_DIR",
                "ARTIFACT_RUN_SCOPE", "ARTIFACT_ANALYSIS_DIR"):
        env.pop(key, None)
    stamp = datetime.datetime.now().strftime("%Y%m%d_%H%M%S_%f")
    output = (args.compare_only or args.out or ROOT / "results" / f"{args.experiment}_{stamp}").resolve()
    if args.compare_only:
        if not output.is_dir():
            parser.error(f"run directory does not exist: {output}")
    else:
        output.mkdir(parents=True, exist_ok=False)
        (output / "experiment.json").write_text(json.dumps({
            "experiment": args.experiment, "features": features.split(","),
            "instrumentation_scope": "primary package only", "reference": "published",
            "crates": list(names), "variants": VARIANTS,
        }, indent=2) + "\n")
        corpus = Path(env.get("CORPUS_DIR", ROOT / "corpus/sources"))
        if not all((corpus / crate).is_dir() for crate in names):
            if run_command(["bash", str(ROOT / "run/fetch_corpus.sh")], output / "measure.log", env):
                return 1
        code = run_command(["bash", str(ROOT / "run/measure.sh"), "--tier", "full",
                            "--features", features, "--out", str(output)], output / "measure.log", env)
        if code:
            return code
    # Scope must be known before making any numerical comparisons.
    datasets.require_primary(output)
    found = {path.parent.name for path in output.glob("*/rebench_summary.json")}
    unexpected = sorted(found - names.keys())
    if unexpected:
        raise ValueError(f"Run includes crates outside the 100-crate corpus: {unexpected}")
    missing = [crate for crate in names if not (output / crate / "rebench_summary.json").is_file()]
    (output / "coverage.json").write_text(json.dumps({
        "expected_crates": 100, "missing_summaries": missing,
        "instrumentation_scope": "primary package only",
    }, indent=2) + "\n")
    # Isolate generated JSON and tables, so separate experiments can run together.
    analysis = output / "analysis"
    if not analysis.exists():
        shutil.copytree(ANALYSIS, analysis, ignore=shutil.ignore_patterns("Latex", "__pycache__", "*_yourrun.*"))
    env["ARTIFACT_ANALYSIS_DIR"] = str(analysis)
    failures = bool(missing)
    for table in tables:
        code = run_command([sys.executable, str(ROOT / "tools/make_table.py"), table,
                            "--run", str(output), "--no-comparison"], output / "tables.log", env)
        failures |= code != 0
    failures |= not compare(args.experiment, analysis, output, names)
    print(f"Results: {output}")
    if failures:
        print("Incomplete measurement or report generation; inspect coverage.json and the logs.", file=sys.stderr)
    return 1 if failures else 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, KeyError) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
