# The image holds everything the measurements need except the compiler itself:
# the build dependencies, the compiler source, a current cargo, and the artifact
# scripts. It takes a few minutes to build.
#
# The compiler is built separately, by `docker/build.sh`, into a Docker volume.
# See the comment above the STAGE1 lines below for why it is not built here.

FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

# ninja builds LLVM; g++ is the bootstrap compiler the measurements used;
# default-jdk is needed by the jni crate and clang by ring; zstd unpacks the
# compiler source.
RUN apt-get update && apt-get install -y --no-install-recommends \
      build-essential ninja-build python3 python3-pip git curl \
      pkg-config libssl-dev zstd ca-certificates default-jdk clang file \
    && rm -rf /var/lib/apt/lists/*

# CMake comes from PyPI, not from apt, and the version matters. Ubuntu 22.04
# ships CMake 3.22.1, which cannot read the dependency file that LLVM 18's
# TableGen writes for GenVT.inc. That file names an output and then lists no
# dependencies at all, and 3.22's depfile reader treats that as a parse error.
# The build stops at "Building GenVT.inc..." with a bare FAILED line and no
# error message underneath it, which is easy to mistake for the kernel killing
# the process. CMake 3.31 reads the same file without complaint.
RUN pip3 install --no-cache-dir "cmake==3.31.6" \
 && cmake --version

WORKDIR /workspace

# --- the compiler ------------------------------------------------------------
COPY compiler/compiler-src.tar.zst /tmp/compiler-src.tar.zst
RUN mkdir -p /workspace/compiler-src \
 && zstd -dc /tmp/compiler-src.tar.zst | tar x -C /workspace/compiler-src \
 && rm /tmp/compiler-src.tar.zst

# The bootstrap configuration is not part of the source tree, so write the one
# the measurements used. Assertions stay on: they are slower, but turning them
# off would be a different compiler from the one that produced the data.
#
# Parallelism is capped by memory rather than by core count, as a precaution.
# Compiling LLVM and the vendored Z3 at one job per core needs more memory than
# a typical machine has, and an LLVM link job can need several gigabytes on its
# own. The image budgets roughly 2 GB per compile job and 8 GB per link job, so
# a 32-core machine with 30 GB of memory builds with 15 compile jobs and 3 link
# jobs. Override with --build-arg COMPILE_JOBS=n LINK_JOBS=m.
ARG COMPILE_JOBS=
ARG LINK_JOBS=
RUN set -eu; \
    mem_gb=$(awk '/MemTotal/ {printf "%d", $2/1024/1024}' /proc/meminfo); \
    cpus=$(nproc); \
    cj="${COMPILE_JOBS:-$(( mem_gb/2 < cpus ? mem_gb/2 : cpus ))}"; \
    lj="${LINK_JOBS:-$(( mem_gb/8 > 0 ? mem_gb/8 : 1 ))}"; \
    [ "$cj" -lt 1 ] && cj=1; [ "$lj" -lt 1 ] && lj=1; \
    echo "building with $cj compile jobs and $lj link jobs ($cpus cpus, ${mem_gb} GB)"; \
    printf '%s\n' \
      'change-id = 125535' \
      '' \
      '[llvm]' \
      "build-config = { \"LLVM_ENABLE_RTTI\" = \"ON\", \"LLVM_PARALLEL_COMPILE_JOBS\" = \"$cj\", \"LLVM_PARALLEL_LINK_JOBS\" = \"$lj\" }" \
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
      > /workspace/compiler-src/config.toml; \
    echo "$cj" > /workspace/jobs; \
    cat /workspace/compiler-src/config.toml

# --- cargo -------------------------------------------------------------------
# A current cargo drives the build. The stage-0 cargo that ships with a 1.80
# bootstrap cannot parse registry manifests that require edition 2024, which
# blocks dependency resolution for several corpus crates. cargo only drives the
# build, so pairing a current cargo with the stage-1 rustc is fine.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --default-toolchain stable --profile minimal
ENV PATH="/root/.cargo/bin:${PATH}"

# The compiler itself is not built here. `docker/build.sh` builds it in a
# container, into a Docker volume mounted at /workspace/compiler-src/build, for
# three reasons. The build takes hours, and a volume makes it resumable: if it
# is interrupted, running the step again continues from where it stopped rather
# than starting over. The 40 GB it needs stays out of the image, so the image
# remains small enough to rebuild in minutes when a script changes. And the
# progress of a `docker run` is easier to watch than the progress of a layer.
ENV STAGE1=/workspace/compiler-src/build/x86_64-unknown-linux-gnu/stage1
ENV ARTIFACT_STAGE1="$STAGE1"

# --- everything else ---------------------------------------------------------
COPY . /workspace/artifact
WORKDIR /workspace/artifact

RUN python3 -c "import sys; assert sys.version_info >= (3,10)" \
 && pip3 install --no-cache-dir matplotlib numpy pyyaml toml >/dev/null

ENV RUSTC_BOOTSTRAP=1

# The entrypoint links the stage-1 compiler as a rustup toolchain, but only
# once it exists. The link cannot be made here: rustup refuses to link a
# toolchain whose rustc is not already a file, and the compiler is built later,
# into a volume.
ENTRYPOINT ["/workspace/artifact/docker/entrypoint.sh"]
CMD ["/bin/bash"]
