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

if [ ! -d corpus/sources ] || [ -z "$(ls -A corpus/sources 2>/dev/null)" ]; then
  run/fetch_corpus.sh
fi

RUN="$HERE/results/$(date +%Y%m%d_%H%M%S)"
if [ -n "$CRATES" ]; then
  run/measure.sh --crate "$CRATES" --out "$RUN"
else
  run/measure.sh --tier "$TIER" --out "$RUN"
fi

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
