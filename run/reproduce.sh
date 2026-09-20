#!/usr/bin/env bash
# One command: get the crate sources, measure, and build every table and figure
# from what was measured.
#
#   run/reproduce.sh                 12 crates, about 25 minutes
#   run/reproduce.sh --tier fast     58 crates, about 2 hours
#   run/reproduce.sh --tier full    100 crates, 121.7 hours
#
# Everything it produces lands in results/<timestamp>/, including
# results/<timestamp>/tables/ with one file per table and figure.
set -euo pipefail

HERE="$(cd "$(dirname "$0")/.." && pwd)"
cd "$HERE"

TIER=smoke
[ "${1:-}" = "--tier" ] && TIER="$2"

if [ ! -d corpus/sources ] || [ -z "$(ls -A corpus/sources 2>/dev/null)" ]; then
  run/fetch_corpus.sh
fi

RUN="$HERE/results/$(date +%Y%m%d_%H%M%S)"
run/measure.sh --tier "$TIER" --out "$RUN"

echo
echo "========================================================================"
echo "building every table and figure from $RUN"
echo "========================================================================"
for s in tables/*.sh; do
  echo
  echo "### $s"
  bash "$s" --run "$RUN"
done

echo
echo "done. Tables and figures: $RUN/tables/"
