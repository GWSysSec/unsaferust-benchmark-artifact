#!/bin/bash

# Script to build the unsafe Rust benchmark Docker image

set -e

echo "Building unsafe Rust benchmark Docker image..."
echo "The instrumented compiler and instrumentation libraries are shipped"
echo "PREBUILT (via Git LFS), so this build does NOT compile rustc/LLVM."
echo "It only:"
echo "  1. Installs system dependencies"
echo "  2. Installs a stock cargo and links the prebuilt 'stage1' toolchain"
echo "  3. Copies the artifact into the image"
echo "Typical build time is a few minutes (ensure 'git lfs pull' ran first)."
echo ""
# Non-interactive mode
echo "Start building..."




echo ""
echo "Step 1-4: Building Docker image..."
# Build the Docker image
docker build -t unsaferust-bench:local -f Dockerfile . 2>&1 | tee build.log

echo ""
echo "Build complete! To run the container:"
echo "  docker run -it unsaferust-bench:local"
echo ""
echo "To run a specific benchmark:"
echo "  docker run -it unsaferust-bench:local bash -c 'cd /workspace/benchmarks/arrayvec-0.7.6 && cargo bench'"
echo ""
echo "To run ALL experiments automatically:"
echo "  docker run --rm -v \$(pwd)/results:/workspace/results unsaferust-bench:local python3 run_pipeline.py --all --showstats"
