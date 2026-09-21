## What is in it that a stock rustc 1.80 does not have

Under `src/llvm-project/llvm/lib/Transforms/`:

InstMarker/ brackets the unsafe span of each basic block with a pair
of side-effecting nop markers, so the passes below can
still find unsafe code after optimization has rewritten it
DynamicAnalysis/ the measurement passes: CPU cycles, heap tracking,
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
