#!/usr/bin/env bash
# One command: get the crate sources, measure, and build every table and figure
# from what was measured.
#
#   run/reproduce.sh                 12 crates, about 25 minutes
#   run/reproduce.sh --tier fast     58 crates, about 2 hours
#   run/reproduce.sh --tier full    100 crates, 121.7 hours
#   run/reproduce.sh --crate bytes   one crate
#
# Everything it produces lands in results/<timestamp>/, including
# results/<timestamp>/tables/ with one file per table and figure.
set -euo pipefail

HERE="$(cd "$(dirname "$0")/.." && pwd)"
cd "$HERE"

TIER=smoke
TIER_SET=0
CRATES=

while [ $# -gt 0 ]; do
  case "$1" in
    --tier)
      [ $# -ge 2 ] || { echo "--tier needs a value" >&2; exit 2; }
      [ "$TIER_SET" -eq 0 ] && [ -z "$CRATES" ] || {
        echo "use either --tier or --crate, not both" >&2
        exit 2
      }
      TIER="$2"
      TIER_SET=1
      shift 2
      ;;
    --crate)
      [ $# -ge 2 ] && [ -n "$2" ] || { echo "--crate needs a value" >&2; exit 2; }
      [ "$TIER_SET" -eq 0 ] && [ -z "$CRATES" ] || {
        echo "use either --tier or --crate, not both" >&2
        exit 2
      }
      CRATES="$2"
      shift 2
      ;;
    *)
      echo "unknown option: $1" >&2
      exit 2
      ;;
  esac
done

if [ -n "$CRATES" ]; then
  CRATE_COUNT_HINT=0
  IFS=',' read -ra REQUESTED_CRATES <<< "$CRATES"
  for crate in "${REQUESTED_CRATES[@]}"; do
    [ -n "$crate" ] && CRATE_COUNT_HINT=$((CRATE_COUNT_HINT + 1))
  done
else
  case "$TIER" in
    smoke) CRATE_COUNT_HINT=12;;
    fast)  CRATE_COUNT_HINT=58;;
    full)  CRATE_COUNT_HINT=100;;
    *)     CRATE_COUNT_HINT="unknown";;
  esac
fi

echo "Crates: $CRATE_COUNT_HINT"
echo

if [ ! -d corpus/sources ] || [ -z "$(ls -A corpus/sources 2>/dev/null)" ]; then
  run/fetch_corpus.sh
fi

RUN="$HERE/results/$(date +%Y%m%d_%H%M%S)"
if [ -n "$CRATES" ]; then
  run/measure.sh --crate "$CRATES" --out "$RUN"
else
  run/measure.sh --tier "$TIER" --out "$RUN"
fi

REPORT_LOG="$RUN/table-generation.log"
: > "$REPORT_LOG"
for s in tables/*.sh; do
  if ! bash "$s" --run "$RUN" >> "$REPORT_LOG" 2>&1; then
    echo "failed to generate results with $s; full output:" >&2
    cat "$REPORT_LOG" >&2
    exit 1
  fi
done

CRATE_COUNT=0
for summary in "$RUN"/*/rebench_summary.json; do
  [ -f "$summary" ] || continue
  CRATE_COUNT=$((CRATE_COUNT + 1))
done

echo
echo "Crates measured: $CRATE_COUNT"
echo
echo "RQ results:"
echo "  RQ1 CPU cycles:              $RUN/tables/cpu_cycles_yourrun.tex"
echo "  RQ2 heap usage:              $RUN/tables/heap_yourrun.tex"
echo "  RQ3 unsafe instruction rate: $RUN/tables/unsafe_inst_frequency_yourrun.tex"
echo "  RQ4 unsafe instruction types: $RUN/tables/inst_yourrun.tex"
echo "  RQ5 unsafe functions:        $RUN/tables/function_yourrun.tex"
echo
echo "Plots and figures:"
echo "  CPU-cycle CDF (PNG): $RUN/tables/cumulative_frequency_unsafe_execution_rq1_rq6_yourrun.png"
echo "  CPU-cycle CDF (PDF): $RUN/tables/cumulative_frequency_unsafe_execution_rq1_rq6_yourrun.pdf"
echo "  Heap CDF (PNG):      $RUN/tables/heap_native_cdf_yourrun.png"
echo "  Heap CDF (PDF):      $RUN/tables/heap_native_cdf_yourrun.pdf"
echo
echo "Measurement data: $RUN"
echo "Generation log:   $REPORT_LOG"
