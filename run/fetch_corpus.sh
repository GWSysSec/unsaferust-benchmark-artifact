#!/usr/bin/env bash
# Put the 100 crate source trees, at the exact versions that were measured,
# into corpus/sources.
#
# Two ways to get them, and they answer different questions.
#
#   (default)        unpack corpus/corpus-sources.tar.zst, which is the trees
#                    we measured, packaged. No network, a couple of seconds,
#                    419 MB on disk. This is what the measurement steps need.
#
#   --from-upstream  clone or download each crate from its original home at the
#                    commit or released version in corpus/corpus_lock.csv, then
#                    apply corpus/corpus_overlay/, which carries every
#                    difference between that upstream tree and the tree we
#                    measured — including the 113 Cargo.lock files that pin the
#                    dependency versions. Needs network, a few minutes. This
#                    answers whether the corpus can be rebuilt from its
#                    published sources; add --verify DIR to compare the result
#                    against an unpacked copy, file by file.
set -euo pipefail

HERE="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${CORPUS_DIR:-$HERE/corpus/sources}"
TARBALL="$HERE/corpus/corpus-sources.tar.zst"

MODE=offline
ARGS=()
while [ $# -gt 0 ]; do
  case "$1" in
    --from-upstream) MODE=upstream; shift;;
    --verify)        ARGS+=(--verify-against "$2"); shift 2;;
    *)               ARGS+=("$1"); shift;;
  esac
done

mkdir -p "$OUT"

if [ "$MODE" = offline ]; then
  [ -f "$TARBALL" ] || {
    echo "no $TARBALL; use --from-upstream to fetch from the network" >&2
    exit 1
  }
  echo "unpacking $TARBALL into $OUT"
  zstd -dc "$TARBALL" | tar x -C "$OUT"
  echo "crates: $(find "$OUT" -mindepth 1 -maxdepth 1 -type d | wc -l)"
  exit 0
fi

cd "$HERE/corpus"
exec python3 fetch_corpus.py --out "$OUT" "${ARGS[@]}"
