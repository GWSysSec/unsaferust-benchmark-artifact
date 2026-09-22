#!/usr/bin/env bash
# Unpack the 100 crate source trees, exactly as measured, into corpus/sources.
# No network. About two seconds, 419 MB.
set -euo pipefail

HERE="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${CORPUS_DIR:-$HERE/corpus/sources}"
TARBALL="$HERE/corpus/corpus-sources.tar.zst"

[ -f "$TARBALL" ] || { echo "missing $TARBALL" >&2; exit 1; }
(
  cd "$HERE/corpus"
  sha256sum --check corpus-sources.sha256
)

mkdir -p "$OUT"
echo "unpacking $TARBALL"
zstd -dc "$TARBALL" | tar x -C "$OUT"
echo "crates: $(find "$OUT" -mindepth 1 -maxdepth 1 -type d | wc -l)"
