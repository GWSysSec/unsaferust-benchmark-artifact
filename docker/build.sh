#!/usr/bin/env bash
# Get a working compiler into the Docker volume the measurements read from.
#
#   docker/build.sh                 use the compiler we built (default)
#   docker/build.sh --from-source   build it from compiler/compiler-src.tar.zst
#
# Both paths start by building the image, which takes about six minutes: apt
# packages, the compiler source, a current cargo and the artifact scripts.
#
# The default then unpacks compiler/stage1-toolchain.tar.zst into the volume,
# which takes under a minute. That tarball is the stage-1 toolchain we built
# from the source in this artifact: rustc, the libraries it links, and the
# standard library compiled against it, 87 MB compressed and 343 MB unpacked.
#
# --from-source builds the same thing instead, from the source tarball. It took
# 876 seconds and 7.0 GB on a 32-core machine, and a machine with fewer cores
# takes roughly proportionally longer. The build goes into a Docker volume
# rather than an image layer, so an interrupted build continues where it
# stopped when this script is run again.
#
# Either way the result is in the volume named below, and
# `docker volume rm unsaferust-compiler` reclaims the space afterwards.
set -euo pipefail

HERE="$(cd "$(dirname "$0")/.." && pwd)"
IMAGE="${IMAGE:-unsaferust-artifact:local}"
VOLUME="${VOLUME:-unsaferust-compiler}"
TOOLCHAIN="$HERE/compiler/stage1-toolchain.tar.zst"

MODE=prebuilt
[ "${1:-}" = "--from-source" ] && MODE=source
[ -f "$TOOLCHAIN" ] || MODE=source

# The image decides the compiler build's parallelism from the memory it sees.
# Override either number from the environment, for example
#   COMPILE_JOBS=8 LINK_JOBS=2 docker/build.sh --from-source
BUILD_ARGS=()
[ -n "${COMPILE_JOBS:-}" ] && BUILD_ARGS+=(--build-arg "COMPILE_JOBS=$COMPILE_JOBS")
[ -n "${LINK_JOBS:-}" ]    && BUILD_ARGS+=(--build-arg "LINK_JOBS=$LINK_JOBS")

echo "step 1 of 2: image $IMAGE (about six minutes)"
docker build "${BUILD_ARGS[@]}" -f "$HERE/docker/Dockerfile" -t "$IMAGE" "$HERE"

docker volume create "$VOLUME" >/dev/null
TTY=()
[ -t 1 ] && TTY=(-t)

if [ "$MODE" = prebuilt ]; then
  echo
  echo "step 2 of 2: unpacking our compiler into volume $VOLUME (under a minute)"
  docker run --rm "${TTY[@]}" \
    -v "$VOLUME:/workspace/compiler-src/build" \
    -v "$TOOLCHAIN:/tmp/stage1.tar.zst:ro" \
    "$IMAGE" bash -c '
      set -eu
      zstd -dc /tmp/stage1.tar.zst | tar x -C /workspace/compiler-src/build
      cp "$(rustup which --toolchain stable cargo)" "$ARTIFACT_STAGE1/bin/cargo"
      cd /tmp && "$ARTIFACT_STAGE1/bin/rustc" --version'
  echo
  echo "done. To build the same compiler from source instead:"
  echo "    docker/build.sh --from-source"
else
  echo
  echo "step 2 of 2: building the compiler into volume $VOLUME"
  echo "(876 seconds on 32 cores; longer on fewer. Progress is printed.)"
  docker run --rm "${TTY[@]}" \
    -v "$VOLUME:/workspace/compiler-src/build" \
    "$IMAGE" bash /workspace/artifact/docker/build_compiler.sh
fi

echo
echo "next: docker/run.sh opens a shell with this compiler mounted"
