#!/usr/bin/env bash
# Rebuild this table from the data this artifact ships and check it against the
# submitted paper. Add --dataset alldeps for the dependency-include numbers,
# or --from-run <dir> to also check a measurement run of your own.
exec python3 "$(dirname "$0")/../tools/check_table.py" rq5_unsafe_functions "$@"
