#!/usr/bin/env bash
# Build the image. Expect two to four hours, almost all of it the compiler.
set -euo pipefail
HERE="$(cd "$(dirname "$0")/.." && pwd)"
IMAGE="${IMAGE:-unsaferust-artifact:local}"
echo "building $IMAGE from $HERE"
echo "the compiler build inside takes hours; progress is printed as it goes."
exec docker build -f "$HERE/docker/Dockerfile" -t "$IMAGE" "$HERE"
