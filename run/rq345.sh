#!/usr/bin/env bash
# Measure all 100 crates once with the counters shared by RQ3, RQ4, and RQ5.
set -euo pipefail
HERE="$(cd "$(dirname "$0")/.." && pwd)"
exec python3 "$HERE/tools/run_experiment.py" rq345 "$@"
