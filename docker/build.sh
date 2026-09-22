#!/usr/bin/env bash
# Build the image when needed, then populate a compiler volume from the
# prebuilt toolchain or rebuild the compiler from source.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE="${IMAGE:-unsaferust-artifact:v2}"
MODE=prebuilt
SOURCE_ARGS=()

usage() {
  cat <<'USAGE'
Usage: docker/build.sh [--from-source [OPTIONS]]

Build the artifact image, then prepare its compiler.

  --from-source    rebuild rustc from the shipped source instead of using the
                   prebuilt toolchain; accepts --output FILE and --volume NAME
USAGE
}

while (($#)); do
  case "$1" in
    --from-source) MODE=source; shift ;;
    --output|--volume)
      [ "$#" -ge 2 ] || { echo "$1 needs a value" >&2; exit 2; }
      SOURCE_ARGS+=("$1" "$2")
      shift 2
      ;;
    --help|-h) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

if [ "$MODE" = prebuilt ] && [ "${#SOURCE_ARGS[@]}" -gt 0 ]; then
  echo "--output and --volume require --from-source" >&2
  exit 2
fi

echo "building image: $IMAGE"
DOCKER_BUILDKIT="${DOCKER_BUILDKIT:-1}" docker build \
  -f "$HERE/docker/Dockerfile" \
  -t "$IMAGE" \
  "$HERE"

if [ "$MODE" = source ]; then
  BASE_IMAGE="$IMAGE" VOLUME="${VOLUME:-unsaferust-compiler-from-source-build}" \
    bash "$HERE/docker/build_instrumented_compiler.sh" "${SOURCE_ARGS[@]}"
  exit
fi
VOLUME="${VOLUME:-unsaferust-compiler}"

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
