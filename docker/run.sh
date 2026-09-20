#!/usr/bin/env bash
# Open a shell in the image, with the artifact at /workspace/artifact.
set -euo pipefail
IMAGE="${IMAGE:-unsaferust-artifact:local}"
exec docker run -it --rm "$IMAGE" "$@"
