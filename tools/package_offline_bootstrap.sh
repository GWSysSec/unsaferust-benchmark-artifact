#!/usr/bin/env bash
# Maintainer-only: prepare the offline inputs for rebuilding the shipped rustc.
# Evaluators use the resulting archive and do not run this script.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BENCH="${BENCH:-$HOME/Projects/unsafebench/unsafe-rust-benchmark}"
SOURCE="$HERE/compiler/compiler-src.tar.zst"
OUT="$HERE/compiler/offline-bootstrap.tar.zst"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

[ -f "$SOURCE" ] || { echo "missing compiler source archive" >&2; exit 1; }
[ -d "$BENCH/build/cache" ] || { echo "missing compiler stage-0 cache: $BENCH" >&2; exit 1; }
expected_rustc="$(awk '$1 == "rustc" {print $2}' "$HERE/compiler/COMMIT")"
expected_llvm="$(awk '$1 == "llvm" {print $2}' "$HERE/compiler/COMMIT")"
[ "$(git -C "$BENCH" rev-parse HEAD)" = "$expected_rustc" ] || {
  echo "the compiler checkout does not match compiler/COMMIT" >&2; exit 1;
}
[ "$(git -C "$BENCH/src/llvm-project" rev-parse HEAD)" = "$expected_llvm" ] || {
  echo "the LLVM submodule does not match compiler/COMMIT" >&2; exit 1;
}

mkdir -p "$WORK/src/.cargo"
zstd -dc "$SOURCE" | tar -x -C "$WORK/src"
test ! -e "$WORK/src/config.toml"

# Cargo's vendor command can download missing locked dependencies here, at
# artifact preparation time. The evaluator build uses the output offline.
(
  cd "$WORK/src"
  RUSTC_BOOTSTRAP=1 cargo vendor --locked --versioned-dirs \
    --sync src/bootstrap/Cargo.toml vendor \
    > .cargo/config.toml
)

date="$(sed -n 's/^compiler_date=//p' "$WORK/src/src/stage0" | head -1)"
version="$(sed -n 's/^compiler_version=//p' "$WORK/src/src/stage0" | head -1)"
triple=x86_64-unknown-linux-gnu
[ -n "$date" ] && [ -n "$version" ]
mkdir -p "$WORK/src/offline-stage0-cache/$date"
for component in cargo rustc rust-std; do
  name="$component-$version-$triple.tar.xz"
  from="$BENCH/build/cache/$date/$name"
  expected="$(awk -F= -v key="dist/$date/$name" '$1 == key {print $2}' "$WORK/src/src/stage0")"
  [ -n "$expected" ] && [ -f "$from" ] || { echo "missing stage-0 component: $name" >&2; exit 1; }
  printf '%s  %s\n' "$expected" "$from" | sha256sum --check --status
  cp "$from" "$WORK/src/offline-stage0-cache/$date/"
done

tar -cf - -C "$WORK/src" .cargo vendor offline-stage0-cache \
  | zstd -12 -T0 -q -o "$OUT.new"
mv "$OUT.new" "$OUT"
(
  cd "$HERE/compiler"
  sha256sum "$(basename "$OUT")" > offline-bootstrap.sha256
  sha256sum "$(basename "$SOURCE")" > offline-bootstrap-source.sha256
)
ls -lh "$OUT"
