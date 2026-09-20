#!/usr/bin/env bash
# Build the image, then build the compiler inside it.
#
# Two steps, because they behave differently. The image is apt packages, the
# compiler source and a current cargo: about six minutes, and it either works or
# fails at once. The compiler is LLVM 18 with assertions plus rustc 1.80, which
# took 876 seconds and produced 7.0 GB on a 32-core machine.
#
# The compiler build writes into a Docker volume rather than into an image
# layer. That makes it resumable — if it is interrupted, running this script
# again continues from where it stopped — and keeps the 40 GB out of the image.
#
# Run this script again at any time. The image rebuild is cached, and the
# compiler build is incremental.
set -euo pipefail

HERE="$(cd "$(dirname "$0")/.." && pwd)"
IMAGE="${IMAGE:-unsaferust-artifact:local}"
VOLUME="${VOLUME:-unsaferust-compiler}"

# The image decides the compiler build's parallelism from the memory it sees.
# Override either number from the environment, for example
#   COMPILE_JOBS=8 LINK_JOBS=2 docker/build.sh
BUILD_ARGS=()
[ -n "${COMPILE_JOBS:-}" ] && BUILD_ARGS+=(--build-arg "COMPILE_JOBS=$COMPILE_JOBS")
[ -n "${LINK_JOBS:-}" ]    && BUILD_ARGS+=(--build-arg "LINK_JOBS=$LINK_JOBS")

echo "step 1 of 2: image $IMAGE (a few minutes)"
docker build "${BUILD_ARGS[@]}" -f "$HERE/docker/Dockerfile" -t "$IMAGE" "$HERE"

echo
echo "step 2 of 2: compiler into volume $VOLUME (15 min on 32 cores, longer on fewer)"
docker volume create "$VOLUME" >/dev/null

TTY=()
[ -t 1 ] && TTY=(-t)
exec docker run --rm "${TTY[@]}" \
  -v "$VOLUME:/workspace/compiler-src/build" \
  "$IMAGE" bash /workspace/artifact/docker/build_compiler.sh
