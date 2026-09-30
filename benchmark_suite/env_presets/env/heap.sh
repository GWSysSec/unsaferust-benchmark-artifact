#!/bin/bash
# Environment setup for Heap Tracker
# Build the library first: cd /workspace/artifact/unsafe_perf_source && make heap

export PERF_LIB="/workspace/artifact/unsafe_perf_source/target/release/libunsafe_perf.rlib"
export PERF_DEPS="/workspace/artifact/unsafe_perf_source/target/release/deps"

export RUSTC_BOOTSTRAP=1
export RUSTUP_TOOLCHAIN=stage1

export RUSTFLAGS="--emit=llvm-ir,link -Z unstable-options --extern force:unsafe_perf=$PERF_LIB -L $PERF_DEPS -C unsafe_include_native_lib=false -C llvm-args=-enable-instmarker -C llvm-args=-enable-heap-tracker"

export UNSAFE_STAT_DIR="/tmp"

echo "Environment configured for Heap Tracking."
echo "Output will be written to: $UNSAFE_STAT_DIR"
