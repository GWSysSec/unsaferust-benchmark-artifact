#!/usr/bin/env bash
# Build corpus/corpus-sources.tar.zst from a verified rebuild of the corpus.
#
# The tarball is what run/fetch_corpus.sh unpacks by default, so it has to be
# exactly the trees that were measured. This script does not take anyone's word
# for that: it rebuilds all 100 crates from corpus_lock.csv and the overlay,
# verifies the result file by file against the directory the measurements were
# taken from, and only then packages it. A single mismatch stops the script.
#
#   tools/package_corpus.sh [BOOTCAMP_DIR]
set -euo pipefail

HERE="$(cd "$(dirname "$0")/.." && pwd)"
REF="${1:-/home/oscar/lab/unsafe-dyn-rust-expr/static-analysis/bootcamp}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

[ -d "$REF" ] || { echo "no reference corpus at $REF" >&2; exit 1; }

echo "rebuilding the corpus from the lock, into $WORK"
python3 "$HERE/corpus/fetch_corpus.py" --out "$WORK/sources" \
        --verify-against "$REF" | tee "$WORK/verify.log"

grep -q "^100 crates rebuilt, 0 failed" "$WORK/verify.log" \
  || { echo "rebuild did not finish cleanly; not packaging" >&2; exit 1; }
if grep -q "^problems:" "$WORK/verify.log"; then
  echo "verification reported differences; not packaging" >&2
  sed -n '/^problems:/,$p' "$WORK/verify.log" >&2
  exit 1
fi

# The rebuild leaves a git repository inside every crate it cloned, and a cache
# of downloaded release tarballs. Neither is source: keeping them turned a
# 419 MB corpus into a 939 MB archive.
echo "packaging"
tar -C "$WORK/sources" --exclude=.crate_cache --exclude=.git -cf - . \
  | zstd -19 -T0 -q -o "$HERE/corpus/corpus-sources.tar.zst.new"
mv "$HERE/corpus/corpus-sources.tar.zst.new" "$HERE/corpus/corpus-sources.tar.zst"
ls -lh "$HERE/corpus/corpus-sources.tar.zst"
