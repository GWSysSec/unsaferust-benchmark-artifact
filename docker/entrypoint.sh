#!/usr/bin/env bash
# Runs on every container start, before whatever command was asked for.
#
# Its only job is to link the stage-1 compiler as a rustup toolchain named
# `stage1`, so that `cargo +stage1 build` works in a shell. The link is made
# here rather than in the image because rustup refuses to link a toolchain
# whose rustc does not exist yet, and the compiler is built after the image,
# into a Docker volume.
set -e

if [ -n "${ARTIFACT_STAGE1:-}" ] && [ -x "$ARTIFACT_STAGE1/bin/rustc" ]; then
  rustup toolchain link stage1 "$ARTIFACT_STAGE1" >/dev/null 2>&1 || true
fi

exec "$@"
