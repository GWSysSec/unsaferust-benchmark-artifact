# The instrumented compiler

Two files, the same compiler.

`stage1-toolchain.tar.zst` is that compiler already built: rustc, the shared
libraries it links, and the standard library compiled against it, 87 MB
compressed and 343 MB unpacked. `docker/build.sh` unpacks it by default, so an
evaluator does not have to build anything. `tools/package_toolchain.sh` produced
it and tests the compiler before packaging: the right version, a program using
unsafe code, and the instrumentation flags the measurements use.

`compiler-src.tar.zst` is the full source it was built from: rustc 1.80.0-dev
with LLVM 18, the instrumentation passes, and the rustc-side changes that carry
unsafe metadata from HIR down to LLVM IR. It is shipped as a tarball rather than
as a git checkout so that building it does not depend on any repository still
being reachable. `docker/build.sh --from-source` builds it.

It excludes only `.git`, the rustc test suite, and previous build output. The
two commit hashes it was made from are in `COMMIT`: the rustc tree first, then
its LLVM submodule.

`docker/Dockerfile` unpacks the source into the image and
`docker/build_compiler.sh` builds it, both driven by `docker/build.sh
--from-source`. Nothing here needs to be run by hand.

## What is in it that a stock rustc 1.80 does not have

Under `src/llvm-project/llvm/lib/Transforms/`:

  InstMarker/         brackets the unsafe span of each basic block with a pair
                      of side-effecting nop markers, so the passes below can
                      still find unsafe code after optimization has rewritten it
  DynamicAnalysis/    the measurement passes: CPU cycles, heap tracking,
                      instruction and function counters, external-call and
                      standard-library-API tracking

Under `compiler/`: the rustc changes that attach `unsafe_inst` metadata during
lowering, the `-C unsafe_include_native_lib` flag that decides whether unsafe
code inside core, std and alloc counts, and the MIR pass that marks calls into
the standard library.

The environment variable `UNSAFE_INSTRUMENT_ALL_PACKAGES=1` makes the passes
instrument every crate in the dependency graph instead of only the crate under
study. It is off by default, so a build that does not set it behaves exactly as
the one the paper's main results come from.
