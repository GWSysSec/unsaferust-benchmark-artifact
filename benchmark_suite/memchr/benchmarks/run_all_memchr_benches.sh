#!/bin/bash
# Run all memchr benchmarks as defined in engines.toml
set -e

# List of engine directories and benchmark names (from engines.toml)
declare -A engines
engines["rust-memchr"]="memchr-oneshot memchr-prebuilt memchr-onlycount memchr-fallback memchr-naive memchr2 memchr2-fallback memchr2-naive memchr3 memchr3-fallback memchr3-naive memrchr memrchr2 memrchr3 memmem-prebuilt memmem-oneshot memmem-binary memmem-twoway memmem-rabinkarp memmem-shiftor"

for engine_dir in "${!engines[@]}"; do
    echo "Building $engine_dir..."
    pushd "./engines/$engine_dir" > /dev/null
    cargo build --release
    for bench_name in ${engines[$engine_dir]}; do
        echo "Running $engine_dir: $bench_name"
        ./target/release/main "$bench_name"
    done
    popd > /dev/null
    echo "Cleaning $engine_dir..."
    pushd "./engines/$engine_dir" > /dev/null
    cargo clean
    popd > /dev/null
    echo "---"
done
