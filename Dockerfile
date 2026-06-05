FROM ubuntu:22.04

# Avoid interactive prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive

# Runtime/build dependencies for the benchmark suite.
# NOTE: the instrumented Rust compiler is shipped PREBUILT under toolchain/, so
# none of the heavy rustc/LLVM build tooling (cmake, ninja, ~8.7 GB, hours of
# build time) is needed anymore. default-jdk is for the JNI benchmark, clang for
# ring.
RUN apt-get update && apt-get install -y \
    git \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    python3 \
    python3-pip \
    default-jdk \
    clang \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

# --- Prebuilt instrumented compiler (stage1, rustc 1.80.0-dev) ---------------
# Shipped via Git LFS under toolchain/ (rustc + sysroot, ~674 MB). Requires only
# glibc 2.34, so it runs as-is on this base image. Copied first so it stays in
# the build cache across script changes.
COPY toolchain /workspace/toolchain

# --- Prebuilt per-feature instrumentation libraries -------------------------
# libunsafe_perf.rlib is built once per instrumentation feature; the three
# feature builds live under unsafe_perf_prebuilt/<feature>/ (rlib + deps).
COPY unsafe_perf_prebuilt /workspace/unsafe_perf_prebuilt

# Stock cargo (build driver). The instrumented compiler is selected via the
# rustup toolchain link below; run_pipeline.py sets RUSTUP_TOOLCHAIN=stage1 so
# every `cargo` invocation compiles with the prebuilt rustc.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --default-toolchain stable --profile minimal
ENV PATH="/root/.cargo/bin:${PATH}"

# Register the prebuilt compiler as the `stage1` toolchain, and complete it with
# a cargo binary (the prebuilt sysroot ships rustc/rustdoc only) so that
# `cargo +stage1` / RUSTUP_TOOLCHAIN=stage1 resolves both cargo and rustc.
RUN rustup toolchain link stage1 /workspace/toolchain \
 && cp "$(rustup which --toolchain stable cargo)" /workspace/toolchain/bin/cargo

# Copy the rest of the artifact (benchmarks, unsafe_perf_source, experiment_env,
# scripts, generated_tests, dynamic_analysis_results, analyzed_crates.csv).
# unsafe_perf_source/target is excluded by .dockerignore.
COPY . /workspace/

# Stage a ready-to-use instrumentation library into the path run_pipeline.py
# expects (unsafe_perf_source/target/release). Switch instrumentation by copying
# a different unsafe_perf_prebuilt/<feature>/ into place, or by `make <feature>`
# in unsafe_perf_source/ (which rebuilds with the prebuilt stage1 compiler).
RUN mkdir -p /workspace/unsafe_perf_source/target/release \
 && cp -a /workspace/unsafe_perf_prebuilt/unsafe_counter/libunsafe_perf.rlib /workspace/unsafe_perf_source/target/release/ \
 && cp -a /workspace/unsafe_perf_prebuilt/unsafe_counter/deps /workspace/unsafe_perf_source/target/release/deps

CMD ["/bin/bash"]
