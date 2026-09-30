#!/usr/bin/env bash
# Link the stage-1 compiler as the rustup toolchain `stage1`, which may come
# from a mounted volume, then run the requested command.
set -e

if [ -n "${ARTIFACT_STAGE1:-}" ] && [ -x "$ARTIFACT_STAGE1/bin/rustc" ]; then
  rustup toolchain link stage1 "$ARTIFACT_STAGE1" >/dev/null 2>&1 || true
fi

exec "$@"
