#!/usr/bin/env bash
# Export the built compiler from the Docker volume into
# compiler/stage1-toolchain.tar.zst, so an evaluator can use it without
# building it.
#
# The tarball holds the stage-1 toolchain only -- rustc, the shared libraries
# it links, and the standard library compiled against it -- not the 7 GB of
# LLVM object files the build needed to produce them. It unpacks to the same
# path inside the volume, which is what docker/build.sh does by default.
#
# The compiler is tested before it is packaged: it must report the right
# version, compile a program that uses unsafe code, and accept the
# instrumentation flags the measurements use.
set -euo pipefail

HERE="$(cd "$(dirname "$0")/.." && pwd)"
IMAGE="${IMAGE:-unsaferust-artifact:local}"
VOLUME="${VOLUME:-unsaferust-compiler}"
OUT="$HERE/compiler/stage1-toolchain.tar.zst"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "testing the compiler in $VOLUME"
docker run --rm -v "$VOLUME:/workspace/compiler-src/build" "$IMAGE" bash -c '
set -e
S="$ARTIFACT_STAGE1"
cd /tmp
# Not piped into head: rustc panics on the broken pipe and writes an ICE
# report, which looks alarming and is purely an artefact of closing the pipe.
v="$("$S/bin/rustc" --version)"
echo "$v"
case "$v" in *1.80.0-dev*) ;; *) echo "unexpected compiler: $v" >&2; exit 1;; esac
printf "fn main(){ let v=vec![1u8,2,3]; unsafe { assert_eq!(*v.as_ptr().add(1), 2); } }\n" > t.rs
"$S/bin/rustc" -O t.rs -o t && ./t
RUSTC_BOOTSTRAP=1 "$S/bin/rustc" -O \
  -C unsafe_include_native_lib=false \
  -C llvm-args=--enable-unsafe-inst-counter \
  -C llvm-args=--enable-unsafe-function-tracker \
  t.rs -o t2
echo "the compiler works and accepts the instrumentation flags"'

echo "exporting"
docker run --rm -v "$VOLUME:/b" -v "$WORK:/out" "$IMAGE" \
  bash -c 'cd /b && tar -cf /out/stage1.tar x86_64-unknown-linux-gnu/stage1 && chmod 644 /out/stage1.tar'

echo "compressing"
zstd -19 -T0 -q -f "$WORK/stage1.tar" -o "$OUT.new"
mv "$OUT.new" "$OUT"
ls -lh "$OUT"
