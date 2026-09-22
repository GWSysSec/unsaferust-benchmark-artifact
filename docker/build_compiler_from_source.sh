#!/usr/bin/env bash
# Build the instrumented stage-1 rustc inside the release container.
# No prebuilt compiler files or network access enter this build directory.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCE_ARCHIVE="$HERE/compiler/compiler-src.tar.zst"
BOOTSTRAP_ARCHIVE="$HERE/compiler/offline-bootstrap.tar.zst"
ROOT="${COMPILER_REBUILD_ROOT:-/workspace/compiler-rebuild}"
SOURCE_DIR="$ROOT/source"
BUILD_DIR="$SOURCE_DIR/build"

[ "$(uname -m)" = x86_64 ] || { echo "x86_64 is required" >&2; exit 1; }
(
  cd "$HERE/compiler"
  sha256sum --check --status offline-bootstrap-source.sha256
  sha256sum --check --status offline-bootstrap.sha256
) || { echo "compiler archive checksum failed" >&2; exit 1; }

mem_kb="$(awk '/MemAvailable/ {print $2}' /proc/meminfo)"
mem_gb=$((mem_kb / 1024 / 1024))
cpus="$(nproc)"
jobs="${COMPILE_JOBS:-$((mem_gb / 2))}"
link_jobs="${LINK_JOBS:-$((mem_gb / 8))}"
[[ "$jobs" =~ ^[1-9][0-9]*$ && "$link_jobs" =~ ^[1-9][0-9]*$ ]] || {
  echo "COMPILE_JOBS and LINK_JOBS must be positive integers" >&2; exit 2;
}
[ "$jobs" -le "$cpus" ] || jobs="$cpus"

mkdir -p "$BUILD_DIR"
source_hash="$(sha256sum "$SOURCE_ARCHIVE" | cut -d' ' -f1)"
bootstrap_hash="$(sha256sum "$BOOTSTRAP_ARCHIVE" | cut -d' ' -f1)"
build_id="$source_hash $bootstrap_hash"
if [ -f "$BUILD_DIR/.artifact-build-id" ]; then
  [ "$(cat "$BUILD_DIR/.artifact-build-id")" = "$build_id" ] || {
    echo "this build directory belongs to different compiler archives" >&2; exit 1;
  }
else
  [ ! -e "$BUILD_DIR/x86_64-unknown-linux-gnu/stage1/bin/rustc" ] || {
    echo "build directory contains a compiler from another build" >&2; exit 1;
  }
  printf '%s\n' "$build_id" > "$BUILD_DIR/.artifact-build-id"
fi

zstd -dc "$SOURCE_ARCHIVE" | tar x -C "$SOURCE_DIR"
zstd -dc "$BOOTSTRAP_ARCHIVE" | tar x -C "$SOURCE_DIR"
test -f "$SOURCE_DIR/.cargo/config.toml"
test -f "$SOURCE_DIR/src/llvm-project/llvm/lib/Transforms/InstMarker/InstMarker.cpp"
test -f "$SOURCE_DIR/src/llvm-project/llvm/lib/Transforms/DynamicAnalysis/UnsafeInstCounter.cpp"
test ! -e "$SOURCE_DIR/src/llvm-project/llvm/lib/SVF"
test ! -e "$SOURCE_DIR/src/llvm-project/llvm/lib/Transforms/SVFAnalysis"

cat > "$SOURCE_DIR/config.toml" <<CONFIG
change-id = 125535

[llvm]
build-config = { "LLVM_ENABLE_RTTI" = "ON", "LLVM_PARALLEL_COMPILE_JOBS" = "$jobs", "LLVM_PARALLEL_LINK_JOBS" = "$link_jobs" }
download-ci-llvm = false
optimize = true
assertions = true
ninja = true
targets = "X86"
link-shared = false

[build]
submodules = false
vendor = true
extended = false
profiler = true
CONFIG

mkdir -p "$BUILD_DIR/cache"
cp -a "$SOURCE_DIR/offline-stage0-cache/." "$BUILD_DIR/cache/"
cd "$SOURCE_DIR"
unset RUSTUP_TOOLCHAIN RUSTC CARGO
export CARGO_NET_OFFLINE=true
export PYTHONDONTWRITEBYTECODE=1
python3 x.py build library --stage 1 -j "$jobs"

stage1="$BUILD_DIR/x86_64-unknown-linux-gnu/stage1"
test -x "$stage1/bin/rustc"
test -d "$stage1/lib/rustlib/x86_64-unknown-linux-gnu/lib"
cp "$(rustup which --toolchain "$ARTIFACT_CARGO_TOOLCHAIN" cargo)" "$stage1/bin/cargo"

sample_dir="$(mktemp -d)"
trap 'rm -rf "$sample_dir"' EXIT
printf '%s\n' 'fn main() { let a = [1u8, 2]; unsafe { assert_eq!(*a.as_ptr().add(1), 2); } }' > "$sample_dir/sample.rs"
(
  cd "$sample_dir"
  RUSTC_BOOTSTRAP=1 "$stage1/bin/rustc" -O \
    -C unsafe_include_native_lib=false \
    -C llvm-args=--enable-unsafe-inst-counter \
    -C llvm-args=--enable-unsafe-function-tracker sample.rs -o sample
  ./sample
)
"$stage1/bin/rustc" --version --verbose
"$stage1/bin/cargo" --version
printf 'stage1: %s\n' "$stage1"
