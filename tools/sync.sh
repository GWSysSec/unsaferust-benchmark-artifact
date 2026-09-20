#!/usr/bin/env bash
# Refresh the artifact's copy of the analysis code and the measurement data.
#
# The analysis code and the data live in the two working repositories where the
# study is done. The artifact carries a copy so that it is self-contained, and
# this script is the only way that copy is made, so its provenance is never a
# guess. Run it before packaging, then commit whatever it changed.
#
#   PAPER  the paper repository (tables, aggregation, per-question data)
#   BENCH  the harness repository (corpus lock, overlay, measurement driver)
set -euo pipefail

HERE="$(cd "$(dirname "$0")/.." && pwd)"
PAPER="${PAPER:-$HOME/Projects/unsafebench/rust-dyn-bench-paper}"
BENCH="${BENCH:-$HOME/Projects/unsafebench/rusttest-gen}"

for d in "$PAPER" "$BENCH"; do
  [ -d "$d" ] || { echo "missing source repository: $d" >&2; exit 1; }
done

echo "analysis code and per-question data <- $PAPER"
mkdir -p "$HERE/tools/analysis"
for f in aggregate_rebench.py datasets.py export_dataset.py \
         verify_reproduction.py summarise_datasets.py audit_duplicate_stats.py; do
  cp "$PAPER/$f" "$HERE/tools/analysis/$f"
done
for d in cpucyclecount_rq1 heaptracker_rq2 unsafeinstfrequency_rq3 \
         unsafeinsttype_rq4 unsafefunction_rq5; do
  mkdir -p "$HERE/tools/analysis/$d"
  # Data and generators only. `*_yourrun.*` is skipped: those files are written
  # when someone builds a table from their own measurement, so they are output
  # of the artifact rather than part of it.
  find "$PAPER/$d" -maxdepth 1 -type f \( -name '*.py' -o -name '*.json' -o -name '*.csv' \) \
      -not -name '*_yourrun.*' \
      -exec cp {} "$HERE/tools/analysis/$d/" \;
done

echo "the tables as submitted <- $PAPER/Latex"
mkdir -p "$HERE/data/paper_tables"
for t in cpu_cycles heap unsafe_inst_frequency inst function scope_comparison; do
  [ -f "$PAPER/Latex/tables/$t.tex" ] && cp "$PAPER/Latex/tables/$t.tex" "$HERE/data/paper_tables/"
done
mkdir -p "$HERE/data/paper_figures"
for f in cumulative_frequency_unsafe_execution_rq1_rq6 heap_native_cdf; do
  [ -f "$PAPER/Latex/figures/$f.pdf" ] && cp "$PAPER/Latex/figures/$f."{pdf,png} "$HERE/data/paper_figures/" 2>/dev/null || true
done

echo "crate list and measurement harness <- $BENCH"
cp "$BENCH/rebench_100crates.csv" "$HERE/corpus/crates.csv"
# rebench.py resolves the corpus list relative to its own repository root, so it
# needs a copy under the harness directory too, under the name it expects.
mkdir -p "$HERE/tools/harness"
cp "$BENCH/rebench_100crates.csv" "$HERE/tools/harness/rebench_100crates.csv"

# Only what a measurement run imports. The test-generation half of that
# repository is not copied here.
mkdir -p "$HERE/tools/harness/scripts" "$HERE/tools/harness/pipeline/tools"
cp "$BENCH/scripts/rebench.py" "$HERE/tools/harness/scripts/"
cp "$BENCH/scripts/rustc-wrapper-alldeps.sh" "$HERE/tools/harness/scripts/"
cp "$BENCH/pipeline/__init__.py" "$BENCH/pipeline/config.py" "$HERE/tools/harness/pipeline/"
for m in __init__ api_discovery coverage crate_prep csv_book runtime_bench workspace; do
  cp "$BENCH/pipeline/tools/$m.py" "$HERE/tools/harness/pipeline/tools/"
done
find "$HERE/tools/harness" -name '__pycache__' -type d -exec rm -rf {} + 2>/dev/null || true

echo "generated workloads <- $BENCH/recipes"
rm -rf "$HERE/corpus/generated_tests"
mkdir -p "$HERE/corpus/generated_tests"
for d in "$BENCH"/recipes/*/tests; do
  [ -d "$d" ] || continue
  crate="$(basename "$(dirname "$d")")"
  mkdir -p "$HERE/corpus/generated_tests/$crate/tests"
  cp "$d"/*.rs "$HERE/corpus/generated_tests/$crate/tests/" 2>/dev/null || true
done

echo
echo "done. Counts:"
echo "  analysis data files : $(find "$HERE/tools/analysis" -name '*.json' | wc -l)"
echo "  paper tables        : $(ls "$HERE/data/paper_tables" 2>/dev/null | wc -l)"
echo "  generated workloads : $(find "$HERE/corpus/generated_tests" -name '*.rs' | wc -l)"
