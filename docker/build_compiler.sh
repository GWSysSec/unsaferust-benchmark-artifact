#!/usr/bin/env bash
# Build the instrumented compiler inside the image: LLVM 18 with assertions,
# then rustc 1.80, from the source in /workspace/compiler-src.
set -euo pipefail

SRC=/workspace/compiler-src
BUILD="$SRC/build"
JOBS="$(cat /workspace/jobs)"
: "${ARTIFACT_STAGE1:?the image must set ARTIFACT_STAGE1}"

mountpoint -q "$BUILD" \
  || echo "warning: $BUILD is not a mounted volume, so this build will be lost" >&2

cd "$SRC"
started=$(date +%s)
python3 x.py build library --stage 1 -j "$JOBS"
elapsed=$(( $(date +%s) - started ))

# the 1.80 bootstrap cargo cannot parse edition-2024 manifests
cp "$(rustup which --toolchain "$ARTIFACT_CARGO_TOOLCHAIN" cargo)" "$ARTIFACT_STAGE1/bin/cargo"

printf '%s  %s seconds (%s hours) on %s cores\n' \
  "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  "$elapsed" "$(awk -v s="$elapsed" 'BEGIN{printf "%.1f", s/3600}')" "$(nproc)" \
  >> "$BUILD/BUILD_TIME"

echo
echo "compiler built in $elapsed seconds; every run of this step is in $BUILD/BUILD_TIME"
"$ARTIFACT_STAGE1/bin/rustc" --version --verbose
echo
echo "next: docker/run.sh opens a shell with this compiler mounted"
