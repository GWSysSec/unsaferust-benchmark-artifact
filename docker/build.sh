#!/usr/bin/env bash
# Populate a compiler volume from the released image, or rebuild from source.
# Both evaluator paths run with Docker networking disabled.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE="${IMAGE:-unsaferust-artifact:v3}"

if [ "${1:-}" = --from-source ]; then
  shift
  BASE_IMAGE="$IMAGE" VOLUME="${VOLUME:-unsaferust-compiler-from-source-build}" \
    bash "$HERE/docker/build_instrumented_compiler.sh" "$@"
  exit
fi
[ "$#" -eq 0 ] || { echo "usage: docker/build.sh [--from-source]" >&2; exit 2; }
VOLUME="${VOLUME:-unsaferust-compiler}"

docker image inspect "$IMAGE" >/dev/null || {
  echo "load the distributed $IMAGE image before running this script" >&2
  exit 1
}
docker volume create "$VOLUME" >/dev/null
docker run --rm --pull=never --network none --entrypoint /bin/bash \
  --mount "type=volume,source=$VOLUME,target=/workspace/compiler-src/build,volume-nocopy" \
  -v "$HERE/compiler/stage1-toolchain.tar.zst:/tmp/stage1-toolchain.tar.zst:ro" \
  "$IMAGE" -euo pipefail -c '
    if [ ! -x "$ARTIFACT_STAGE1/bin/rustc" ]; then
      zstd -dc /tmp/stage1-toolchain.tar.zst | tar x -C /workspace/compiler-src/build
      cp "$(rustup which --toolchain "$ARTIFACT_CARGO_TOOLCHAIN" cargo)" "$ARTIFACT_STAGE1/bin/cargo"
    fi
    test -x "$ARTIFACT_STAGE1/bin/rustc"
    "$ARTIFACT_STAGE1/bin/rustc" --version
  '
echo "compiler volume: $VOLUME"
