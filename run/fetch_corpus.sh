#!/usr/bin/env bash
# Obtain the 100 crate source trees at the exact versions that were measured.
#
# Clones or downloads each crate at the commit or released version recorded in
# corpus/corpus_lock.csv, then applies corpus/corpus_overlay/, which carries
# every difference between that upstream tree and the tree we measured,
# including the 113 Cargo.lock files that pin the dependency versions.
#
# Needs network access. About 1.7 GB on disk and a few minutes.
set -euo pipefail
HERE="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${CORPUS_DIR:-$HERE/corpus/sources}"
mkdir -p "$OUT"
cd "$HERE/corpus"
exec python3 fetch_corpus.py --out "$OUT" "$@"
