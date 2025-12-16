FROM ubuntu:22.04

# Avoid interactive prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive

# Install build dependencies
RUN apt-get update && apt-get install -y \
    git \
    curl \
    build-essential \
    cmake \
    ninja-build \
    python3 \
    python3-pip \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /workspace

# Copy the local repository contents
# Note: This includes all files from your local clone
COPY . /workspace/

# Build the custom Rust compiler
# User requested optimization: single command without separate install step
RUN cd unsafe-rust-benchmark && \
    python3 x.py

# Build the instrumentation library (coverage as default)
RUN cd perf && make coverage

# Set up environment variables for the instrumentation
ENV RUSTC_PATH=/workspace/unsafe-rust-benchmark/build/x86_64-unknown-linux-gnu/stage1/bin/rustc
ENV PATH="/workspace/unsafe-rust-benchmark/build/x86_64-unknown-linux-gnu/stage1/bin:${PATH}"

# Set the default working directory to perf
WORKDIR /workspace/perf

# Default command
CMD ["/bin/bash"]
