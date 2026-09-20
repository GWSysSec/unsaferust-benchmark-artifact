#!/usr/bin/env bash
# Build this table from YOUR measurement and compare it with the paper.
# Measure first with run/measure.sh. Options:
#   --run DIR      a measurement directory other than the newest in results/
#   --scope alldeps  the run instrumented every crate in the dependency graph
#   --our-data     build from the data this artifact ships instead of yours
exec python3 "$(dirname "$0")/../tools/make_table.py" rq2_heap "$@"
