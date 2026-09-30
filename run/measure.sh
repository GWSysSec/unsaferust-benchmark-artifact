#!/usr/bin/env bash
# Measure the corpus with the instrumented compiler.
#
#   smoke   12 crates
#   fast    58 crates
#   full    all 100 crates
#
# --crate names a comma-separated list instead of a tier. --features limits the
# run to some of unsafe_counter, heap_tracker and cpu_cycle_counter.
# --bin-timeout is the per-test-binary limit in seconds; a binary that reaches
# it is killed and records no measurement.
#
# Usage:  run/measure.sh --tier smoke [--out DIR]
#         run/measure.sh --crate tokio,bytes
#         run/measure.sh --crate memchr --features unsafe_counter,heap_tracker \
#                        --bin-timeout 43200
set -euo pipefail

HERE="$(cd "$(dirname "$0")/.." && pwd)"
TIER=smoke
CRATES=
FEATURES=
BIN_TIMEOUT=28800
OUT="$HERE/results/$(date +%Y%m%d_%H%M%S)"
CORPUS="${CORPUS_DIR:-$HERE/corpus/sources}"

while [ $# -gt 0 ]; do
  case "$1" in
    --tier)  TIER="$2"; shift 2;;
    --crate) CRATES="$2"; shift 2;;
    --out)   OUT="$2"; shift 2;;
    --features)    FEATURES="$2"; shift 2;;
    --bin-timeout)
      [ $# -ge 2 ] && [[ "$2" =~ ^[1-9][0-9]*$ ]] || {
        echo "--bin-timeout needs a positive integer (seconds)" >&2; exit 2;
      }
      BIN_TIMEOUT="$2"; shift 2;;
    *) echo "unknown option: $1" >&2; exit 2;;
  esac
done

[ -d "$CORPUS" ] || { echo "no corpus at $CORPUS; run run/fetch_corpus.sh first" >&2; exit 1; }
STAGE1="${ARTIFACT_STAGE1:-/workspace/compiler-src/build/x86_64-unknown-linux-gnu/stage1}"
[ -x "$STAGE1/bin/rustc" ] || { echo "no compiler at $STAGE1/bin/rustc" >&2; exit 1; }

# Twelve crates spanning the range of unsafe shares.
SMOKE=borsh-rs,ron,siphasher,libsecp256k1,nu-ansi-term,scroll,deranged,git2,cc-rs,slotmap,curl,dashmap

# The 58 crates that each finish in under five minutes.
FAST=arrayvec,async-lock,async-task,bimap,borsh-rs,bytemuck,byteorder,bytes,cc-rs,clru,colored,cssparser,curl,dashmap,deranged,dlv-list,ego-tree,filetime,fixedbitset,fragile,fs4,getrandom,git2,hashlink,headers,http-body,imgref,iri-string,lexical-core,log,memmap2,metrics,msgpack-rust,native-tls,nu-ansi-term,ordered-float,ordered-multimap,os_info,ouroboros,pin-project-lite,polling,postcard,rand,rgb,rmp,rust-openssl,rustc-demangle,scroll,semver,serde_yaml,simdutf8,siphasher,slotmap,smallvec,triomphe,unicode-normalization,uuid,value-bag

if [ -n "$CRATES" ]; then
  SEL=(--crate "$CRATES")
  TIER="crates: $CRATES"
else
  case "$TIER" in
    smoke) SEL=(--crate "$SMOKE");;
    fast)  SEL=(--crate "$FAST");;
    full)  SEL=(--from-corpus);;
    *) echo "unknown tier: $TIER (smoke|fast|full)" >&2; exit 2;;
  esac
fi
[ -n "$FEATURES" ] && SEL+=(--features "$FEATURES")

mkdir -p "$OUT"
CONF="$OUT/config.yaml"
cat > "$CONF" <<YAML
toolchain:
  rustc: $STAGE1/bin/rustc
  cargo: $(command -v cargo)

pipeline:
  bootcamp_dir: $CORPUS
  csv_path: "$HERE/corpus/crates.csv"
  recipes_dir: $HERE/corpus/generated_tests

bench_runtime:
  unsafe_perf_path: $HERE/unsafe_perf_source
YAML

echo "tier:    $TIER"
echo "corpus:  $CORPUS"
echo "output:  $OUT"
echo "limit:   ${BIN_TIMEOUT}s per test binary"
[ -n "$FEATURES" ] && echo "features: $FEATURES"
echo

cd "$OUT"
exec python3 "$HERE/tools/harness/scripts/rebench.py" \
     "${SEL[@]}" \
     --bin-timeout "$BIN_TIMEOUT" \
     --no-coverage --out-dir "$OUT" --tmp-rebench "$OUT/tmp"
