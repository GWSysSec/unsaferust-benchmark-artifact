#!/usr/bin/env bash
# Confirm that the data this artifact ships is what the paper reports.
#
# This is not the artifact's main path — that is to measure the corpus yourself
# and build the tables from your own numbers, which run/measure.sh and the
# scripts in tables/ do. This check exists so that when your measurement and the
# paper differ, you can tell whether the data we shipped was ever consistent
# with the paper in the first place.
#
# No measurement, no network, about twenty seconds.
set -uo pipefail
HERE="$(cd "$(dirname "$0")/.." && pwd)"
fail=0
for t in rq1_cpu_cycles rq2_heap rq3_unsafe_inst_frequency rq4_inst_types \
         rq5_unsafe_functions; do
  printf '%-30s ' "$t"
  if bash "$HERE/tables/$t.sh" --our-data >/dev/null 2>&1; then
    echo "reproduces the paper's table"
  else
    echo "DOES NOT REPRODUCE   (run tables/$t.sh --our-data to see why)"
    fail=1
  fi
done
echo
[ $fail -eq 0 ] && echo "The shipped data reproduces every table in the paper." \
                || echo "Something did not reproduce; see above."
exit $fail
