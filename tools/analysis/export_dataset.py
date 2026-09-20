#!/usr/bin/env python3
"""Write one measurement run into the per-research-question data files.

For the dataset named on the command line this writes, into each research
question's folder, two things:

  * a JSON file in exactly the schema that question's table and figure scripts
    already read, so those scripts need no new parsing code, and
  * a CSV file holding the same numbers one row per crate per variant, so the
    data can be read without running anything.

The published dataset's files are already committed and are what the submitted
paper reports, so writing them again needs --force. The default dataset to
export is therefore `alldeps`, the run that instruments every crate in the
dependency graph.

A *variant* is one of the two native-library settings the corpus is measured
under. `with_native` counts unsafe code inside the standard library
(core/std/alloc) towards the unsafe totals; `without_native` does not. The
research-question JSONs spell these two `nativetrue`/`nativefalse` for the
instruction counters and `with_native`/`without_native` everywhere else; this
script writes whichever spelling each folder already uses.

Usage:
    python3 export_dataset.py                 # alldeps -> *_alldeps.{json,csv}
    python3 export_dataset.py --dataset published --force

This script was checked against the committed published files before it was
used on any new run: regenerating the published dataset reproduces all twelve
of them value for value, including the loom crate, whose CPU-cycle record is
the one the two earlier porters treated differently. The only differences are
additive: each RQ1 crate record gains the two whole_program_* fields, which the
published files predate.
"""

from __future__ import annotations

import argparse
import csv
import json
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import datasets  # noqa: E402
from aggregate_rebench import (  # noqa: E402
    UNSAFE_COUNTER_FIELDS,
    aggregate_all,
    logical_bin_name,
)

RQ1 = HERE / "cpucyclecount_rq1"
RQ2 = HERE / "heaptracker_rq2"
RQ3 = HERE / "unsafeinstfrequency_rq3"
RQ4 = HERE / "unsafeinsttype_rq4"
RQ5 = HERE / "unsafefunction_rq5"

# The two native-library settings, with the spelling each folder uses.
VARIANTS = [("without_native", "nativefalse"), ("with_native", "nativetrue")]


def _pct(num: float, den: float) -> float:
    return (num / den * 100.0) if den > 0 else 0.0


def write_csv(path: Path, fieldnames: list[str], rows: list[dict]) -> None:
    with path.open("w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=fieldnames)
        w.writeheader()
        for r in rows:
            w.writerow(r)
    print(f"wrote {path.relative_to(HERE)} ({len(rows)} rows)")


def write_json(path: Path, payload: dict) -> None:
    path.write_text(json.dumps(payload, indent=2))
    print(f"wrote {path.relative_to(HERE)}")


def base_metadata(ds: datasets.Dataset, extra: dict | None = None) -> dict:
    meta = {
        "dataset": ds.name,
        "instrumentation_scope": ds.scope,
        "generated": time.strftime("%Y-%m-%dT%H:%M:%S"),
        "generator": "export_dataset.py",
    }
    meta.update(extra or {})
    return meta


# --------------------------------------------------------------- RQ1: cycles

def export_rq1(ds: datasets.Dataset, crates: dict) -> None:
    """CPU cycles spent in unsafe code.

    Both cycle pairs are written for every crate, and the metadata names which
    pair `unsafe_percentage` was computed from, so a reader never has to guess
    which correction is in force. See datasets.py for why the two runs differ.
    """
    num_field = ("internal_unsafe_cycles" if ds.cycle_metric == "internal"
                 else "whole_program_unsafe_cycles")
    den_field = ("internal_total_cycles" if ds.cycle_metric == "internal"
                 else "whole_program_total_cycles")

    csv_rows: list[dict] = []
    for variant, _suffix in VARIANTS:
        out: dict[str, dict] = {}
        for crate_name in sorted(crates):
            cc = crates[crate_name].get("cpu_cycle_counter", {}).get(variant)
            if not cc:
                continue
            total = cc.get(den_field, 0.0)
            unsafe = cc.get(num_field, 0.0)
            if total <= 0:
                continue
            rec = {
                # the field names the existing RQ1 scripts read
                "total_cycles": cc.get("total_cycles", 0.0),
                "external_cycles": cc.get("external_cycles", 0.0),
                "internal_cycles": cc.get("internal_total_cycles", 0.0),
                "unsafe_cycles": cc.get("internal_unsafe_cycles", 0.0),
                # the whole-program pair, named for what it is
                "whole_program_total_cycles": cc.get("whole_program_total_cycles", 0.0),
                "whole_program_unsafe_cycles": cc.get("whole_program_unsafe_cycles", 0.0),
                "unsafe_percentage": _pct(unsafe, total),
            }
            out[crate_name] = rec
            csv_rows.append({
                "crate": crate_name,
                "variant": variant,
                "total_cycles": f"{rec['total_cycles']:.0f}",
                "external_cycles": f"{rec['external_cycles']:.0f}",
                "internal_cycles": f"{rec['internal_cycles']:.0f}",
                "internal_unsafe_cycles": f"{rec['unsafe_cycles']:.0f}",
                "whole_program_unsafe_cycles":
                    f"{rec['whole_program_unsafe_cycles']:.0f}",
                "internal_unsafe_pct":
                    f"{_pct(rec['unsafe_cycles'], rec['internal_cycles']):.4f}",
                "whole_program_unsafe_pct":
                    f"{_pct(rec['whole_program_unsafe_cycles'], rec['total_cycles']):.4f}",
                "reported_unsafe_pct": f"{rec['unsafe_percentage']:.4f}",
            })

        name = ds.stem("cpucycle_withnative" if variant == "with_native"
                       else "cpucycle_withoutnative")
        write_json(RQ1 / f"{name}.json", {
            "metadata": base_metadata(ds, {
                "source": str(ds.cpu_root),
                "variant": variant,
                "cycle_metric": ds.cycle_metric,
                "metric": f"{num_field} / {den_field}",
                "note": "unsafe_percentage uses the pair named in `metric`; "
                        "both pairs are present in every record.",
            }),
            "crates": out,
        })

    write_csv(RQ1 / f"{ds.stem('cpu_cycles')}.csv", [
        "crate", "variant", "total_cycles", "external_cycles", "internal_cycles",
        "internal_unsafe_cycles", "whole_program_unsafe_cycles",
        "internal_unsafe_pct", "whole_program_unsafe_pct", "reported_unsafe_pct",
    ], csv_rows)


# ------------------------------------------------------------------ RQ2: heap

HEAP_RENAME = {
    "total_heap_alloc": "total_heap_allocations",
    "total_heap_realloc": "total_heap_reallocations",
    "total_heap_dealloc": "total_heap_deallocations",
    "total_memory_instructions": "unsafe_memory_instructions",
}

HEAP_LEGACY_ORDER = [
    "total_heap_usage", "total_heap_allocations", "total_heap_reallocations",
    "total_heap_deallocations", "unsafe_heap_memory", "unsafe_heap_objects",
    "unsafe_memory_instructions", "unsafe_load", "unsafe_store",
    "size_histogram", "unsafe_size_histogram",
]

# Allocation-size bin midpoints in bytes; bins 8 and up double from 192 KB.
BIN_MIDPOINTS = [512, 1536, 3072, 6144, 12288, 24576, 49152, 98304]


def to_legacy_heap(ht: dict) -> dict:
    out: dict = {}
    for k, v in ht.items():
        if k.startswith("_"):
            continue
        out[HEAP_RENAME.get(k, k)] = v
    for k in HEAP_LEGACY_ORDER:
        out.setdefault(k, [] if k.endswith("_histogram") else 0)
    return {k: out[k] for k in HEAP_LEGACY_ORDER}


def size_class_totals(hist: list[int]) -> tuple[int, int, int, int, int, int]:
    """Return (objects small, medium, large, bytes small, medium, large)."""
    if not hist:
        return (0, 0, 0, 0, 0, 0)
    o_small = hist[0]
    o_med = sum(hist[1:8])
    o_large = sum(hist[8:])
    b_small = hist[0] * BIN_MIDPOINTS[0]
    b_med = sum(hist[i] * BIN_MIDPOINTS[i] for i in range(1, min(8, len(hist))))
    b_large = sum(hist[i] * (196608 * 2 ** (i - 8)) for i in range(8, len(hist)))
    return (o_small, o_med, o_large, b_small, b_med, b_large)


def export_rq2(ds: datasets.Dataset, crates: dict) -> None:
    """Heap memory touched by unsafe code.

    The JSONs keep the legacy field names so generate_heap_plot.py and
    analyze_unsafe_percentages.py read them unchanged. The CSV adds the
    size-class percentages the RQ2 table reports, so the table can be checked
    without re-running the aggregation.
    """
    csv_rows: list[dict] = []
    for variant, _suffix in VARIANTS:
        rows = []
        pct_rows = []
        for crate_name in sorted(crates):
            ht = crates[crate_name].get("heap_tracker", {}).get(variant)
            if not ht:
                continue
            stats = to_legacy_heap(ht)
            row = {"crate_name": crate_name, "aggregated_stats": stats}
            # The Mem-Inst column divides heap-touching unsafe loads and stores
            # by all unsafe loads and stores, over the bins present in BOTH
            # features. That pairing is computed during aggregation, so carry it
            # in the JSON; otherwise the RQ2 table cannot be rebuilt without the
            # raw per-binary dumps, and every other table can.
            mic = crates[crate_name].get("mem_inst_common", {}).get(variant)
            if mic:
                row["mem_inst_common"] = mic
            rows.append(row)
            total = stats["total_heap_usage"]
            unsafe = stats["unsafe_heap_memory"]
            pct_rows.append({
                "crate_name": crate_name,
                "total_heap_usage": total,
                "unsafe_heap_memory": unsafe,
                "unsafe_heap_percentage": round(_pct(unsafe, total), 2),
            })

            sh = list(stats["size_histogram"])
            ush = list(stats["unsafe_size_histogram"])
            n = max(len(sh), len(ush))
            sh += [0] * (n - len(sh))
            ush += [0] * (n - len(ush))
            ush = [min(u, t) for u, t in zip(ush, sh)]
            ts, tm, tl, bs, bm, bl = size_class_totals(sh)
            us, um, ul, ubs, ubm, ubl = size_class_totals(ush)
            mic = crates[crate_name].get("mem_inst_common", {}).get(variant) or {}
            csv_rows.append({
                "crate": crate_name,
                "variant": variant,
                "total_heap_usage": total,
                "total_heap_allocations": stats["total_heap_allocations"],
                "total_heap_reallocations": stats["total_heap_reallocations"],
                "total_heap_deallocations": stats["total_heap_deallocations"],
                "unsafe_heap_memory": unsafe,
                "unsafe_heap_objects": stats["unsafe_heap_objects"],
                "unsafe_load": stats["unsafe_load"],
                "unsafe_store": stats["unsafe_store"],
                "unsafe_heap_memory_pct": f"{_pct(unsafe, total):.4f}",
                "obj_small_pct": f"{_pct(us, ts):.4f}",
                "obj_medium_pct": f"{_pct(um, tm):.4f}",
                "obj_large_pct": f"{_pct(ul, tl):.4f}",
                "obj_total_pct": f"{_pct(us + um + ul, ts + tm + tl):.4f}",
                "mem_small_pct": f"{_pct(ubs, bs):.4f}",
                "mem_medium_pct": f"{_pct(ubm, bm):.4f}",
                "mem_large_pct": f"{_pct(ubl, bl):.4f}",
                "mem_total_pct": f"{_pct(ubs + ubm + ubl, bs + bm + bl):.4f}",
                "mem_inst_pct": (
                    f"{_pct(mic['heap_unsafe_ldst'], mic['uc_unsafe_ldst']):.4f}"
                    if mic.get("uc_unsafe_ldst") else ""),
            })

        heap_name = ds.stem(f"heap_{variant}")
        write_json(RQ2 / f"{heap_name}.json", {
            "metadata": base_metadata(ds, {
                "total_crates": len(rows),
                "filter_applied": "common-logical-bins between heap_tracker "
                                  "with_native and without_native variants",
                "data_source_directory": str(ds.heap_root),
            }),
            "crates": rows,
            "summary": {
                "variant": variant,
                "field_schema_version": 2,
                "notes": "legacy field names retained for backwards-compat "
                         "with generate_heap_plot.py and "
                         "analyze_unsafe_percentages.py",
            },
        })
        pct_name = ds.stem(f"unsafe_heap_percentage_{variant}")
        write_json(RQ2 / f"{pct_name}.json", {
            "metadata": base_metadata(ds, {
                "description": "Percentage of absolute unsafe heap memory",
                "source_file": f"{heap_name}.json",
                "metric": "unsafe_heap_memory / total_heap_usage * 100",
            }),
            "crates": pct_rows,
        })

    write_csv(RQ2 / f"{ds.stem('heap')}.csv", [
        "crate", "variant", "total_heap_usage", "total_heap_allocations",
        "total_heap_reallocations", "total_heap_deallocations",
        "unsafe_heap_memory", "unsafe_heap_objects", "unsafe_load",
        "unsafe_store", "unsafe_heap_memory_pct",
        "obj_small_pct", "obj_medium_pct", "obj_large_pct", "obj_total_pct",
        "mem_small_pct", "mem_medium_pct", "mem_large_pct", "mem_total_pct",
        "mem_inst_pct",
    ], csv_rows)


# ------------------------------------------------- RQ3, RQ4, RQ5: instruction
#                                                    and function counters

def counter_record(uc: dict) -> dict:
    out: dict = {k: int(uc.get(k, 0) or 0) for k in UNSAFE_COUNTER_FIELDS}
    out["unsafe_calls_total"] = (out["unsafe_calls_direct"]
                                 + out["unsafe_calls_indirect"]
                                 + out["unsafe_calls_intrinsic"])
    out["unsafe_inst_pct"] = _pct(out["unsafe_instructions"],
                                  out["total_instructions"])
    out["distinctive_pct"] = _pct(out["unsafe_functions_executed"],
                                  out["total_functions_executed"])
    out["exec_w_unsafe_pct"] = _pct(out["unsafe_functions_with_executed_insts"],
                                    out["total_functions_executed"])
    out["accumulated_pct"] = _pct(out["unsafe_function_calls"],
                                  out["total_function_calls"])
    return out


RQ4_TYPES = [
    ("load", ["unsafe_loads"]),
    ("store", ["unsafe_stores"]),
    ("ptr_arith", ["unsafe_geps"]),
    ("cast", ["unsafe_casts"]),
    ("call", ["unsafe_calls_direct", "unsafe_calls_indirect",
              "unsafe_calls_intrinsic"]),
    ("others", ["unsafe_others", "unsafe_atomics"]),
]


def export_counters(ds: datasets.Dataset, crates: dict) -> None:
    per_variant: dict[str, dict] = {}
    for variant, suffix in VARIANTS:
        per_crate: dict[str, dict] = {}
        for crate_name in sorted(crates):
            uc = crates[crate_name].get("unsafe_counter", {}).get(variant)
            if not uc:
                continue
            per_crate[crate_name] = counter_record(uc)
        per_variant[variant] = per_crate

        summary = {k: 0 for k in UNSAFE_COUNTER_FIELDS}
        summary["unsafe_calls_total"] = 0
        for rec in per_crate.values():
            for k in summary:
                summary[k] += int(rec.get(k, 0) or 0)

        payload = {
            "metadata": base_metadata(ds, {
                "total_valid_crates": len(per_crate),
                "variant": suffix,
                "aggregator_variant_key": variant,
                "schema_version": 3,
                "data_source_directory": str(ds.counter_root),
            }),
            "summary": summary,
            "per_crate_data": per_crate,
        }
        for rq_dir in (RQ3, RQ4, RQ5):
            write_json(rq_dir / f"{ds.stem(f'unsafe_counter_{suffix}')}.json",
                       payload)

    # RQ3: how often unsafe instructions execute.
    rows = [{
        "crate": c, "variant": v,
        "total_instructions": r["total_instructions"],
        "unsafe_instructions": r["unsafe_instructions"],
        "unsafe_instruction_pct": f"{r['unsafe_inst_pct']:.4f}",
    } for v, _s in VARIANTS for c, r in sorted(per_variant[v].items())]
    write_csv(RQ3 / f"{ds.stem('unsafe_inst_frequency')}.csv",
              ["crate", "variant", "total_instructions", "unsafe_instructions",
               "unsafe_instruction_pct"], rows)

    # RQ4: what kind of instruction the unsafe ones are. Percentages are of
    # unsafe instructions, not of all instructions, matching the RQ4 table.
    fields = ["crate", "variant", "unsafe_instructions"]
    for label, _ in RQ4_TYPES:
        fields += [f"unsafe_{label}", f"unsafe_{label}_pct"]
    rows = []
    for v, _s in VARIANTS:
        for c, r in sorted(per_variant[v].items()):
            row = {"crate": c, "variant": v,
                   "unsafe_instructions": r["unsafe_instructions"]}
            for label, keys in RQ4_TYPES:
                n = sum(r.get(k, 0) for k in keys)
                row[f"unsafe_{label}"] = n
                row[f"unsafe_{label}_pct"] = \
                    f"{_pct(n, r['unsafe_instructions']):.4f}"
            rows.append(row)
    write_csv(RQ4 / f"{ds.stem('unsafe_inst_type')}.csv", fields, rows)

    # RQ5: how often functions holding unsafe code run. "Distinctive" counts a
    # function once however often it runs; "cumulative" counts every call.
    rows = [{
        "crate": c, "variant": v,
        "total_functions_defined": r["total_functions_defined"],
        "unsafe_functions_defined": r["unsafe_functions_defined"],
        "total_functions_executed": r["total_functions_executed"],
        "unsafe_functions_executed": r["unsafe_functions_executed"],
        "distinctive_pct": f"{r['distinctive_pct']:.4f}",
        "total_function_calls": r["total_function_calls"],
        "unsafe_function_calls": r["unsafe_function_calls"],
        "cumulative_pct": f"{r['accumulated_pct']:.4f}",
    } for v, _s in VARIANTS for c, r in sorted(per_variant[v].items())]
    write_csv(RQ5 / f"{ds.stem('unsafe_function')}.csv",
              ["crate", "variant", "total_functions_defined",
               "unsafe_functions_defined", "total_functions_executed",
               "unsafe_functions_executed", "distinctive_pct",
               "total_function_calls", "unsafe_function_calls",
               "cumulative_pct"], rows)


# --------------------------------------------------------------- repeatability

def export_repeats(ds: datasets.Dataset) -> None:
    """One row per crate per variant per repetition of the unsafe-counter run.

    The corpus driver writes repetition 1 into <feature>/<variant>/ and any
    further repetition into <feature>/rep<N>/<variant>/. Repetition 1 is what
    every table reports; the extra repetitions exist so the spread between
    repeated measurements of the same workload can be quoted. A crate with only
    one repetition simply contributes one row.
    """
    root = ds.counter_root
    rows: list[dict] = []
    for crate_dir in sorted(p for p in root.iterdir()
                            if p.is_dir() and not p.name.startswith("_")):
        feat = crate_dir / "unsafe_counter"
        if not feat.is_dir():
            continue
        reps = [("rep1", feat)] + [
            (p.name, p) for p in sorted(feat.iterdir())
            if p.is_dir() and p.name.startswith("rep")]
        for rep_name, rep_dir in reps:
            for variant, _suffix in VARIANTS:
                d = rep_dir / variant
                if not d.is_dir():
                    continue
                total = unsafe = 0
                bins: set[str] = set()
                for jf in sorted(d.glob("*.json")):
                    try:
                        stats = json.loads(jf.read_text()).get("stats", {})
                    except json.JSONDecodeError:
                        continue
                    total += int(stats.get("total_instructions", 0) or 0)
                    unsafe += int(stats.get("unsafe_instructions", 0) or 0)
                    bins.add(logical_bin_name(jf.name.split("__", 1)[0]))
                if total <= 0:
                    continue
                rows.append({
                    "crate": crate_dir.name,
                    "variant": variant,
                    "repetition": rep_name,
                    "logical_bins": len(bins),
                    "total_instructions": total,
                    "unsafe_instructions": unsafe,
                    "unsafe_instruction_pct": f"{_pct(unsafe, total):.4f}",
                })
    write_csv(RQ3 / f"{ds.stem('unsafe_counter_repeatability')}.csv",
              ["crate", "variant", "repetition", "logical_bins",
               "total_instructions", "unsafe_instructions",
               "unsafe_instruction_pct"], rows)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--dataset", default="alldeps", choices=sorted(datasets.DATASETS))
    ap.add_argument("--force", action="store_true",
                    help="allow overwriting the committed published data files")
    args = ap.parse_args()

    ds = datasets.get(args.dataset)
    if ds.suffix == "" and not args.force:
        raise SystemExit(
            f"dataset {ds.name!r} writes the committed paper data files "
            f"(no filename suffix); pass --force to overwrite them")

    for label, root in (("counter", ds.counter_root), ("heap", ds.heap_root),
                        ("cpu", ds.cpu_root)):
        if not root.is_dir():
            raise SystemExit(f"{label} root does not exist: {root}")

    print(f"dataset {ds.name}: {ds.scope}")
    crates = aggregate_all(root=ds.heap_root, heap_common_bins_only=True,
                           keep_test_failures=ds.keep_test_failures)
    if ds.cpu_root != ds.heap_root:
        cpu_crates = aggregate_all(root=ds.cpu_root,
                                   keep_test_failures=ds.keep_test_failures)
        for name, data in cpu_crates.items():
            crates.setdefault(name, {})["cpu_cycle_counter"] = \
                data.get("cpu_cycle_counter", {})
    print(f"crates aggregated: {len(crates)}\n")

    export_rq1(ds, crates)
    export_rq2(ds, crates)
    export_counters(ds, crates)
    export_repeats(ds)
    return 0


if __name__ == "__main__":
    sys.exit(main())
