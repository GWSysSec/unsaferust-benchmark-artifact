#!/usr/bin/env bash
# Package the instrumented compiler source into compiler/compiler-src.tar.zst.
#
# Exports the committed state, not the working tree, before removing unused
# LLVM projects, tests, optional SVF integration and vendored Z3. The rustc
# tree has twelve git submodules; `git archive` covers only the superproject.
# The workspace manifest names members inside src/doc/book, so every submodule
# is exported before pruning.
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
python3 "$HERE/tools/prune_compiler_source.py" "$WORK/src"

echo "compressing"
tar cf - -C "$WORK/src" . | zstd -12 -T0 -q -o "$HERE/compiler/compiler-src.tar.zst" --force

{
  echo "rustc   $(git -C "$BENCH" rev-parse HEAD)  branch $(git -C "$BENCH" rev-parse --abbrev-ref HEAD)"
  echo "llvm    $(git -C "$BENCH/src/llvm-project" rev-parse HEAD)  branch $(git -C "$BENCH/src/llvm-project" rev-parse --abbrev-ref HEAD)"
  echo "runtime $(git -C "$RUNTIME" rev-parse HEAD)"
  echo
  echo "submodules:"
  git -C "$BENCH" submodule status --recursive | sed 's/^/  /'
  echo
  echo "packaging: unused LLVM projects, tests, SVF passes, and Z3 source removed"
} > "$HERE/compiler/COMMIT"

echo
echo "wrote $(du -h "$HERE/compiler/compiler-src.tar.zst" | cut -f1)"
echo "files: $(zstd -dc "$HERE/compiler/compiler-src.tar.zst" | tar t | wc -l)"
