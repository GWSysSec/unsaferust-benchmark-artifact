#!/usr/bin/env bash
# Open a shell in the image, with the compiler volume and two host directories
# mounted, so that what you fetch and what you measure survive the container.
#
#   corpus/sources   the 100 crate source trees, 419 MB once unpacked
#   results/         every measurement run, which the tables/ scripts read
#
# The container runs as root, because the toolchain it uses lives in /root, so
# everything it writes into those two directories is owned by root. This script
# hands them back to you when the container exits, which is what lets you delete
# a run or read it with your own tools afterwards.
set -euo pipefail

HERE="$(cd "$(dirname "$0")/.." && pwd)"
IMAGE="${IMAGE:-unsaferust-artifact:v2}"
VOLUME="${VOLUME:-unsaferust-compiler}"

mkdir -p "$HERE/results" "$HERE/corpus/sources"

give_back() {
  docker run --rm \
    --pull=never --network none \
    -v "$HERE/results:/results" \
    -v "$HERE/corpus/sources:/sources" \
    "$IMAGE" chown -R "$(id -u):$(id -g)" /results /sources >/dev/null 2>&1 || true
}
trap give_back EXIT

TTY=(-i)
[ -t 1 ] && TTY=(-it)
docker run --rm "${TTY[@]}" \
  --pull=never --network none -e CARGO_NET_OFFLINE=true \
  -v "$VOLUME:/workspace/compiler-src/build" \
  -v "$HERE/results:/workspace/artifact/results" \
  -v "$HERE/corpus/sources:/workspace/artifact/corpus/sources" \
  "$IMAGE" "$@"
