#!/usr/bin/env bash
# Build the instrumented compiler. This runs inside the image; start it with
# docker/build.sh rather than by hand.
#
# It builds LLVM 18 with assertions and then rustc 1.80 from the source in
# /workspace/compiler-src. Assertions are on because that is what the paper's
# measurements were taken with; turning them off would be a different compiler.
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

# cargo only drives the build, so the stage-1 rustc is paired with the current
# cargo from the image: the 1.80 bootstrap cargo cannot parse registry manifests
# that require edition 2024, which blocks dependency resolution for several
# corpus crates.
cp "$(rustup which --toolchain stable cargo)" "$ARTIFACT_STAGE1/bin/cargo"

# Appended, not overwritten. Running this script again on a volume that already
# holds a finished build takes a few seconds, and that number would otherwise
# replace the one that says how long the build really took.
printf '%s  %s seconds (%s hours) on %s cores\n' \
  "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  "$elapsed" "$(awk -v s="$elapsed" 'BEGIN{printf "%.1f", s/3600}')" "$(nproc)" \
  >> "$BUILD/BUILD_TIME"

echo
echo "compiler built in $elapsed seconds; every run of this step is in $BUILD/BUILD_TIME"
"$ARTIFACT_STAGE1/bin/rustc" --version --verbose
echo
echo "next: docker/run.sh opens a shell with this compiler mounted"
