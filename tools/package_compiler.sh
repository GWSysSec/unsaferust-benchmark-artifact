#!/usr/bin/env bash
# Package the instrumented compiler source into compiler/compiler-src.tar.zst.
#
# Exports the committed state, not the working tree, so what ships is exactly
# the commits recorded in compiler/COMMIT. The rustc tree has twelve git
# submodules; `git archive` covers only the superproject, and leaving the rest
# out breaks the build immediately, because the workspace manifest names
# members that live inside src/doc/book. So every submodule is exported too.
#
# The rustc test suite is dropped: 157 MB that no measurement needs.
set -euo pipefail

HERE="$(cd "$(dirname "$0")/.." && pwd)"
BENCH="${BENCH:-$HOME/Projects/unsafebench/unsafe-rust-benchmark}"
RUNTIME="${RUNTIME:-$HOME/Projects/unsafebench/unsafe-dyn-rust-expr}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "exporting the committed compiler tree from $BENCH"
mkdir -p "$WORK/src"
git -C "$BENCH" archive HEAD | tar x -C "$WORK/src"

echo "exporting its submodules"
git -C "$BENCH" submodule --quiet foreach --recursive \
  'mkdir -p "'"$WORK"'/src/$sm_path" && git archive HEAD | tar x -C "'"$WORK"'/src/$sm_path"'

rm -rf "$WORK/src/tests"

echo "compressing"
tar cf - -C "$WORK/src" . | zstd -12 -T0 -q -o "$HERE/compiler/compiler-src.tar.zst" --force

{
  echo "rustc   $(git -C "$BENCH" rev-parse HEAD)  branch $(git -C "$BENCH" rev-parse --abbrev-ref HEAD)"
  echo "llvm    $(git -C "$BENCH/src/llvm-project" rev-parse HEAD)  branch $(git -C "$BENCH/src/llvm-project" rev-parse --abbrev-ref HEAD)"
  echo "runtime $(git -C "$RUNTIME" rev-parse HEAD)"
  echo
  echo "submodules:"
  git -C "$BENCH" submodule status --recursive | sed 's/^/  /'
} > "$HERE/compiler/COMMIT"

echo
echo "wrote $(du -h "$HERE/compiler/compiler-src.tar.zst" | cut -f1)"
echo "files: $(zstd -dc "$HERE/compiler/compiler-src.tar.zst" | tar t | wc -l)"
