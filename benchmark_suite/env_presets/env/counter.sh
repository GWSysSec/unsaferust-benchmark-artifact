#!/bin/bash
# Environment setup for Unsafe Counter

# Prebuilt instrumentation library (shipped under unsafe_perf_prebuilt/unsafe_counter/; no build needed).
export PERF_LIB="/workspace/unsafe_perf_prebuilt/unsafe_counter/libunsafe_perf.rlib"
export PERF_DEPS="/workspace/unsafe_perf_prebuilt/unsafe_counter/deps"

export RUSTC_BOOTSTRAP=1
export RUSTUP_TOOLCHAIN=stage1

# Flags for Unsafe Counter:
# - enable instmarker
# - enable unsafe-function-tracker
# - enable unsafe-inst-counter
export RUSTFLAGS="--emit=llvm-ir,link -Z unstable-options --extern force:unsafe_perf=$PERF_LIB -L $PERF_DEPS -C unsafe_include_native_lib=false -C llvm-args=-enable-instmarker -C llvm-args=-enable-unsafe-function-tracker -C llvm-args=-enable-unsafe-inst-counter"

export UNSAFE_BENCH_OUTPUT_DIR="/tmp"

echo "Environment configured for Unsafe Counter."
echo "Output will be written to: $UNSAFE_BENCH_OUTPUT_DIR"
