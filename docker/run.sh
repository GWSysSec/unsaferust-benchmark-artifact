#!/usr/bin/env bash
# Open a shell in the image, with the compiler volume and two host directories
# mounted, so that what you fetch and what you measure survive the container.
#
#   corpus/sources   the 100 crate source trees, about 1.7 GB once fetched
#   results/         every measurement run, which the tables/ scripts read
#
# Both are written by root inside the container, so they belong to root on the
# host afterwards.
set -euo pipefail

HERE="$(cd "$(dirname "$0")/.." && pwd)"
IMAGE="${IMAGE:-unsaferust-artifact:local}"
VOLUME="${VOLUME:-unsaferust-compiler}"

mkdir -p "$HERE/results" "$HERE/corpus/sources"

TTY=(-i)
[ -t 1 ] && TTY=(-it)
exec docker run --rm "${TTY[@]}" \
  -v "$VOLUME:/workspace/compiler-src/build" \
  -v "$HERE/results:/workspace/artifact/results" \
  -v "$HERE/corpus/sources:/workspace/artifact/corpus/sources" \
  "$IMAGE" "$@"
