#!/bin/bash
# Environment setup for Heap Tracker

# Prebuilt instrumentation library (shipped under unsafe_perf_prebuilt/heap_tracker/; no build needed).
export PERF_LIB="/workspace/unsafe_perf_prebuilt/heap_tracker/libunsafe_perf.rlib"
export PERF_DEPS="/workspace/unsafe_perf_prebuilt/heap_tracker/deps"

export RUSTC_BOOTSTRAP=1
export RUSTUP_TOOLCHAIN=stage1

# Flags for Heap Tracker:
# - enable instmarker
# - enable heap-tracker
export RUSTFLAGS="--emit=llvm-ir,link -Z unstable-options --extern force:unsafe_perf=$PERF_LIB -L $PERF_DEPS -C unsafe_include_native_lib=false -C llvm-args=-enable-instmarker -C llvm-args=-enable-heap-tracker"

export UNSAFE_BENCH_OUTPUT_DIR="/tmp"

echo "Environment configured for Heap Tracking."
echo "Output will be written to: $UNSAFE_BENCH_OUTPUT_DIR"
