#!/usr/bin/env bash
# Measure all 100 crates and compare RQ1 CPU-cycle shares with the paper.
set -euo pipefail
HERE="$(cd "$(dirname "$0")/.." && pwd)"
exec python3 "$HERE/tools/run_experiment.py" cpu "$@"
