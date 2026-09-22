#!/usr/bin/env bash
# Maintainer-only preparation: resolve dependencies while online, then ship
# their exact Cargo cache and updated corpus locks for offline evaluators.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE="${PREP_IMAGE:-unsaferust-artifact:v2}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
mkdir -p "$WORK/corpus" "$WORK/cargo-home"
cp -a "$HERE/benchmark_suite" "$WORK/benchmark_suite"
zstd -dc "$HERE/corpus/corpus-sources.tar.zst" | tar x -C "$WORK/corpus"

cat > "$WORK/fetch.sh" <<'INNER'
#!/usr/bin/env bash
set -euo pipefail
export CARGO_HOME=/prepare/cargo-home
export CARGO_NET_OFFLINE=false
export CARGO_NET_RETRY=2
export CARGO_HTTP_TIMEOUT=30

for manifest in /prepare/corpus/*/Cargo.toml; do
  echo "fetching corpus $(basename "$(dirname "$manifest")")"
  cargo fetch --target x86_64-unknown-linux-gnu --manifest-path "$manifest"
done
# The harness applies this pin before building miette.
cargo update --manifest-path /prepare/corpus/miette/Cargo.toml \
  -p backtrace --precise 0.3.73
cargo fetch --target x86_64-unknown-linux-gnu \
  --manifest-path /prepare/corpus/miette/Cargo.toml

for manifest in /prepare/benchmark_suite/*/Cargo.toml \
  /prepare/benchmark_suite/rayon/rayon-demo/Cargo.toml \
  /prepare/benchmark_suite/parking_lot/benchmark/Cargo.toml \
  /prepare/benchmark_suite/ring/bench/Cargo.toml \
  /prepare/benchmark_suite/tokio/benches/Cargo.toml \
  /prepare/benchmark_suite/rebar/engines/rust/memchr/Cargo.toml \
  /prepare/unsafe_perf_source/Cargo.toml; do
  [ -f "$manifest" ] || continue
  echo "fetching benchmark $(dirname "$manifest")"
  cargo fetch --target x86_64-unknown-linux-gnu --manifest-path "$manifest"
done
for directory in simd-json-0.14.3 tokio/benches; do
  cd "/prepare/benchmark_suite/$directory"
  cargo update -p half --precise 2.3.1
  cargo update -p proptest --precise 1.4.0
  cargo fetch --target x86_64-unknown-linux-gnu
done
INNER

docker image inspect "$IMAGE" >/dev/null
docker run --rm --pull=never --entrypoint /bin/bash \
  -v "$WORK:/prepare" \
  -v "$HERE/unsafe_perf_source:/prepare/unsafe_perf_source:ro" \
  "$IMAGE" /prepare/fetch.sh

# Cargo can unpack registry sources again from cache/*.crate. Omitting
# registry/src saves multiple gigabytes without changing offline behavior.
tar -C "$WORK/cargo-home" -cf - registry/cache registry/index git \
  | zstd -10 -T0 -q -f -o "$HERE/corpus/offline-cargo-cache.tar.zst"
tar -C "$WORK/corpus" -cf - . \
  | zstd -10 -T0 -q -f -o "$HERE/corpus/corpus-sources.tar.zst"
(
  cd "$HERE/corpus"
  sha256sum corpus-sources.tar.zst > corpus-sources.sha256
  sha256sum offline-cargo-cache.tar.zst > offline-cargo-cache.sha256
)
echo "Packaged the corpus and offline Cargo cache. Build a new release image."
