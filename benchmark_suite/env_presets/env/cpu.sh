#!/bin/bash
# Environment setup for CPU Cycle Counting
# Build the library first: cd /workspace/artifact/unsafe_perf_source && make cpu

export PERF_LIB="/workspace/artifact/unsafe_perf_source/target/release/libunsafe_perf.rlib"
export PERF_DEPS="/workspace/artifact/unsafe_perf_source/target/release/deps"

export RUSTC_BOOTSTRAP=1
export RUSTUP_TOOLCHAIN=stage1

export RUSTFLAGS="--emit=llvm-ir,link -Z unstable-options --extern force:unsafe_perf=$PERF_LIB -L $PERF_DEPS -C unsafe_include_native_lib=false -C llvm-args=-enable-instmarker -C llvm-args=-enable-cpu-cycle-count -C llvm-args=-enable-external-call-tracker"

export UNSAFE_STAT_DIR="/tmp"

echo "Environment configured for CPU Cycle Counting."
echo "Output will be written to: $UNSAFE_STAT_DIR"
