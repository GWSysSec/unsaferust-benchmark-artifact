#!/usr/bin/env bash
# Measure the corpus with the instrumented compiler.
#
# Three sizes, because the whole corpus takes 121.7 hours of wall time and an
# evaluator does not have that. Each tier runs the same harness on a different
# number of crates:
#
#   smoke   12 crates, about 32 minutes. Spans unsafe shares from 0.03% to
#           10.15%, so it exercises the range rather than a corner of it.
#   fast    the 58 crates that each finish in under five minutes, about 1.5
#           hours together.
#   full    all 100 crates, 121.7 hours. Ten crates take over four hours each
#           and tokio alone takes 16.3.
#
# --crate names a comma-separated list instead of a tier, for re-measuring one
# crate without redoing a whole tier.
#
# Usage:  run/measure.sh --tier smoke [--out DIR]
#         run/measure.sh --crate tokio,bytes
set -euo pipefail

HERE="$(cd "$(dirname "$0")/.." && pwd)"
TIER=smoke
CRATES=
OUT="$HERE/results/$(date +%Y%m%d_%H%M%S)"
CORPUS="${CORPUS_DIR:-$HERE/corpus/sources}"

while [ $# -gt 0 ]; do
  case "$1" in
    --tier)  TIER="$2"; shift 2;;
    --crate) CRATES="$2"; shift 2;;
    --out)   OUT="$2"; shift 2;;
    *) echo "unknown option: $1" >&2; exit 2;;
  esac
done

[ -d "$CORPUS" ] || { echo "no corpus at $CORPUS; run run/fetch_corpus.sh first" >&2; exit 1; }
STAGE1="${ARTIFACT_STAGE1:-/workspace/compiler-src/build/x86_64-unknown-linux-gnu/stage1}"
[ -x "$STAGE1/bin/rustc" ] || { echo "no compiler at $STAGE1/bin/rustc" >&2; exit 1; }

# Twelve crates spanning the range of unsafe shares, each finishing quickly.
SMOKE=borsh-rs,ron,siphasher,libsecp256k1,nu-ansi-term,scroll,deranged,git2,cc-rs,slotmap,curl,dashmap

# The 58 crates that each took under five minutes in our own run, 1.49 hours
# together. Listed rather than filtered at run time so the tier is the same set
# on every machine.
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

mkdir -p "$OUT"
CONF="$OUT/config.yaml"
cat > "$CONF" <<YAML
# Written by run/measure.sh. The harness reads config.yaml from the working
# directory, so this file is what selects the compiler and the corpus.
toolchain:
  rustc: $STAGE1/bin/rustc
  cargo: $(command -v cargo)

pipeline:
  bootcamp_dir: $CORPUS
  crate_list_csv: "$HERE/corpus/crates.csv"
  csv_path: "$HERE/corpus/crates.csv"
  recipes_dir: $HERE/corpus/generated_tests
  tmp_dir: $OUT/tmp_gen

bench_runtime:
  enabled: true
  unsafe_perf_path: $HERE/unsafe_perf_source
  out_dir: ""
  min_api_pct: 0.0
YAML

echo "tier:    $TIER"
echo "corpus:  $CORPUS"
echo "output:  $OUT"
echo "scope:   only the crate under study"
echo

cd "$OUT"
exec python3 "$HERE/tools/harness/scripts/rebench.py" \
     "${SEL[@]}" \
     --no-coverage --out-dir "$OUT" --tmp-rebench "$OUT/tmp"
