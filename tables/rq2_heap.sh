#!/usr/bin/env bash
# Build this table from YOUR measurement.
# Measure first with run/measure.sh. Options:
#   --run DIR      a measurement directory other than the newest in results/
#   --scope SCOPE  override the scope read from the run (primary|alldeps)
#   --our-data     build from the data this artifact ships instead of yours
exec python3 "$(dirname "$0")/../tools/make_table.py" rq2_heap "$@"
