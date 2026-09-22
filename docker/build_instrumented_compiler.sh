#!/usr/bin/env bash
# Run the independent in-container rustc rebuild in a persistent Docker volume.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE="${BASE_IMAGE:-unsaferust-artifact:v3}"
VOLUME="${VOLUME:-unsaferust-compiler-from-source-build}"
OUTPUT="${OUTPUT:-$HERE/compiler/stage1-from-source.tar.zst}"

usage() {
  cat <<'USAGE'
Usage: docker/build_instrumented_compiler.sh [--output FILE] [--volume NAME]

Rebuild rustc from the shipped source in a network-disabled container.
The build volume can resume an interrupted build. The result is exported
as a stage-1 archive. Optional environment: BASE_IMAGE, COMPILE_JOBS,
LINK_JOBS.
USAGE
}

while (($#)); do
  case "$1" in
    --output) OUTPUT="${2:?--output needs a path}"; shift 2 ;;
    --volume) VOLUME="${2:?--volume needs a name}"; shift 2 ;;
    --help|-h) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

[ "$(uname -m)" = x86_64 ] || { echo "x86_64 is required" >&2; exit 1; }
(
  cd "$HERE/compiler"
  sha256sum --check --status offline-bootstrap-source.sha256
  sha256sum --check --status offline-bootstrap.sha256
) || { echo "compiler archive checksum failed" >&2; exit 1; }
docker image inspect "$IMAGE" >/dev/null || {
  echo "load the distributed $IMAGE image before building" >&2; exit 1;
}

docker volume create "$VOLUME" >/dev/null
# The volume is mounted at the empty rebuild tree, never over the prebuilt
# compiler in /workspace/compiler-src/build. volume-nocopy blocks copy-up.
docker run --rm --pull=never --network none --entrypoint /bin/bash \
  --mount "type=volume,source=$VOLUME,target=/workspace/compiler-rebuild/source/build,volume-nocopy" \
  -v "$HERE/compiler:/workspace/artifact/compiler:ro" \
  -e "COMPILE_JOBS=${COMPILE_JOBS:-}" -e "LINK_JOBS=${LINK_JOBS:-}" \
  "$IMAGE" /workspace/artifact/docker/build_compiler_from_source.sh

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
docker run --rm --pull=never --network none --entrypoint /bin/bash \
  --mount "type=volume,source=$VOLUME,target=/build,readonly,volume-nocopy" \
  -v "$WORK:/out" "$IMAGE" -euo pipefail -c '
    cd /build
    test -x x86_64-unknown-linux-gnu/stage1/bin/rustc
    tar -cf - x86_64-unknown-linux-gnu/stage1 \
      | zstd -19 -T0 -q -o /out/stage1.tar.zst
    chmod 644 /out/stage1.tar.zst
  '
mkdir -p "$(dirname "$OUTPUT")"
mv "$WORK/stage1.tar.zst" "$OUTPUT"
echo "toolchain: $OUTPUT"
echo "build volume: $VOLUME"
