#!/usr/bin/env bash
# ============================================================================
# rustc-wrapper-alldeps.sh — keep instrumentation out of compile-time code
#
# Used together with UNSAFE_INSTRUMENT_ALL_PACKAGES=1, which lifts the
# primary-package restriction so every crate in the dependency graph is
# instrumented. Two kinds of crate must stay out of that set, because their
# code runs during the build rather than inside the measured test binary:
#
#   * build scripts  — cargo compiles build.rs into a host binary named
#     build_script_build and then RUNS it. Instrumenting it would emit stat
#     files at build time and count unsafe operations that the benchmark
#     never executes.
#   * proc-macro crates — compiled to a dynamic library that rustc itself
#     loads and calls. Instrumenting it runs the measurement hooks inside the
#     compiler, which pollutes the stats and risks crashing rustc.
#
# For those two cases this wrapper removes every `-C llvm-args=...` flag, so
# no analysis pass is enabled, and clears the environment variables that turn
# instrumentation on. Every other compilation is passed through untouched.
#
# The forced link of the measurement runtime (`--extern force:unsafe_perf=...`)
# is deliberately LEFT IN PLACE even here. A build script links ordinary
# library crates as build-dependencies -- autocfg, for instance -- and those
# libraries are instrumented like any other dependency, so they carry
# undefined references to the runtime hooks. Removing the runtime from the
# build script's link line would leave those references unresolved and the
# build would fail. The runtime is harmless in a build script: with no pass
# enabled nothing calls its hooks, and the stat file its exit handler writes is
# discarded, because the harness clears the stat directory before running each
# measured binary.
#
# Usage (as cargo's RUSTC_WRAPPER):
#   export RUSTC_WRAPPER=/path/to/rustc-wrapper-alldeps.sh
#   export UNSAFE_INSTRUMENT_ALL_PACKAGES=1
# ============================================================================
set -euo pipefail

RUSTC="$1"; shift

is_compile_time_unit=0
prev=""
for arg in "$@"; do
    case "$arg" in
        # `--crate-name build_script_build` arrives as two separate argv
        # entries, so match on the value while remembering the flag before it.
        # The name is build_script_<stem of the script file>, so a crate whose
        # build script is build/main.rs -- openssl-sys, for one -- compiles as
        # build_script_main rather than build_script_build. Match the family.
        build_script_*)
            [[ "$prev" == "--crate-name" ]] && is_compile_time_unit=1
            ;;
        --crate-name=build_script_*)
            is_compile_time_unit=1
            ;;
        proc-macro)
            [[ "$prev" == "--crate-type" ]] && is_compile_time_unit=1
            ;;
        --crate-type=proc-macro)
            is_compile_time_unit=1
            ;;
    esac
    prev="$arg"
done

if [[ "$is_compile_time_unit" -eq 0 ]]; then
    exec "$RUSTC" "$@"
fi

# Rebuild the argument list without the LLVM pass selection. RUSTFLAGS is
# split on whitespace by cargo, so `-C llvm-args=--enable-heap-tracker`
# reaches us as the two entries `-C` and `llvm-args=--enable-heap-tracker`;
# the joined `-Cllvm-args=...` form is handled too, defensively.
filtered=()
drop_next=0
for arg in "$@"; do
    if [[ "$drop_next" -eq 1 ]]; then
        drop_next=0
        continue
    fi
    case "$arg" in
        -C)
            drop_next=0
            filtered+=("$arg")
            ;;
        llvm-args=*)
            # Belongs to the `-C` we just appended: remove that one as well.
            if [[ "${#filtered[@]}" -gt 0 && "${filtered[-1]}" == "-C" ]]; then
                unset 'filtered[-1]'
            fi
            ;;
        -Cllvm-args=*)
            ;;
        *)
            filtered+=("$arg")
            ;;
    esac
done

# Optional audit trail: set UNSAFE_WRAPPER_LOG to a file path and every unit
# this wrapper neutralises is appended to it, one per line. Used to prove the
# exclusion fired, since a build script that merely links an instrumented
# build-dependency still names the runtime hooks in its symbol table.
if [[ -n "${UNSAFE_WRAPPER_LOG:-}" ]]; then
    crate_name="?"
    prev=""
    for arg in "$@"; do
        [[ "$prev" == "--crate-name" ]] && crate_name="$arg"
        prev="$arg"
    done
    echo "excluded crate_name=$crate_name" >> "$UNSAFE_WRAPPER_LOG"
fi

exec env -u UNSAFE_INSTRUMENT_ALL_PACKAGES \
         -u UNSAFE_ENABLE_STDLIB_TRACKER \
         -u CARGO_PRIMARY_PACKAGE \
         "$RUSTC" "${filtered[@]}"
