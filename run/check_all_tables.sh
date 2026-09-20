#!/usr/bin/env bash
# Rebuild every table and figure in the paper from the data this artifact ships
# and check each against the submitted paper. No measurement, no network, about
# twenty seconds.
set -uo pipefail
HERE="$(cd "$(dirname "$0")/.." && pwd)"
DATASET="${1:-published}"
fail=0
for t in rq1_cpu_cycles rq2_heap rq3_unsafe_inst_frequency rq4_inst_types \
         rq5_unsafe_functions figure_cycles_cdf figure_heap_cdf; do
  printf '%-30s ' "$t"
  if bash "$HERE/tables/$t.sh" --dataset "$DATASET" >/dev/null 2>&1; then
    echo "match"
  else
    echo "MISMATCH   (run tables/$t.sh to see the difference)"
    fail=1
  fi
done
echo
[ $fail -eq 0 ] && echo "All tables and figures reproduce from the shipped data." \
                || echo "Something did not reproduce; see above."
exit $fail
