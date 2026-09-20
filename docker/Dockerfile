# The image builds the instrumented compiler from source.
#
# That build is the long step: roughly two to four hours on a machine with a
# dozen cores, and about 40 GB of disk while it runs. It is a full LLVM 18 build
# with assertions enabled, which is what the paper's measurements were taken
# with, followed by stage-0 and stage-1 rustc. Nothing about it needs
# supervision; start it and come back.
#
# Everything after that step is fast, so the image is ordered to keep the
# compiler layer cached: change a script and only the last layers rebuild.

FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

# cmake and ninja build LLVM; g++ is the bootstrap compiler the measurements
# used; default-jdk is needed by the jni crate and clang by ring; zstd unpacks
# the compiler source.
RUN apt-get update && apt-get install -y --no-install-recommends \
      build-essential cmake ninja-build python3 python3-pip git curl \
      pkg-config libssl-dev zstd ca-certificates default-jdk clang file \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

# --- the compiler ------------------------------------------------------------
COPY compiler/compiler-src.tar.zst /tmp/compiler-src.tar.zst
RUN mkdir -p /workspace/compiler-src \
 && zstd -dc /tmp/compiler-src.tar.zst | tar x -C /workspace/compiler-src \
 && rm /tmp/compiler-src.tar.zst

# The bootstrap configuration is not part of the source tree, so write the one
# the measurements used. Assertions stay on: they are slower, but turning them
# off would be a different compiler from the one that produced the data.
RUN printf '%s\n' \
    'change-id = 125535' \
    '' \
    '[llvm]' \
    'build-config = { "LLVM_ENABLE_RTTI" = "ON" }' \
    'download-ci-llvm = false' \
    'optimize = true' \
    'assertions = true' \
    'ninja = true' \
    'targets = "X86"' \
    'link-shared = false' \
    '' \
    '[build]' \
    'submodules = false' \
    'extended = false' \
    'profiler = true' \
    > /workspace/compiler-src/config.toml

RUN cd /workspace/compiler-src && python3 x.py build library --stage 1

# --- cargo -------------------------------------------------------------------
# A current cargo drives the build. The stage-0 cargo that ships with a 1.80
# bootstrap cannot parse registry manifests that require edition 2024, which
# blocks dependency resolution for several corpus crates. cargo only drives the
# build, so pairing a current cargo with the stage-1 rustc is fine.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --default-toolchain stable --profile minimal
ENV PATH="/root/.cargo/bin:${PATH}"

ENV STAGE1=/workspace/compiler-src/build/x86_64-unknown-linux-gnu/stage1
RUN rustup toolchain link stage1 "$STAGE1" \
 && cp "$(rustup which --toolchain stable cargo)" "$STAGE1/bin/cargo"

# --- everything else ---------------------------------------------------------
COPY . /workspace/artifact
WORKDIR /workspace/artifact

RUN python3 -c "import sys; assert sys.version_info >= (3,10)" \
 && pip3 install --no-cache-dir matplotlib numpy pyyaml toml >/dev/null

ENV RUSTC_BOOTSTRAP=1
ENV ARTIFACT_STAGE1="$STAGE1"

CMD ["/bin/bash"]
