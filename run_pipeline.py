#!/usr/bin/env python3
import os
import sys
import shutil
import subprocess
import argparse
import time
from pathlib import Path
from datetime import datetime

# Impor aggregator
try:
    # Legacy stats aggregator (retired in this artifact); kept import optional so
    # --showstats degrades gracefully and the per-binary .stat files remain the
    # source of truth.
    from aggregator import Aggregator
except ImportError:
    Aggregator = None

# Configuration
SCRIPT_DIR = Path(__file__).parent.absolute()
BENCHMARK_DIR = SCRIPT_DIR / "benchmarks"
PERF_DIR = SCRIPT_DIR / "unsafe_perf_source"
PERF_TARGET_DIR = PERF_DIR / "target" / "release"
PERF_RLIB = PERF_TARGET_DIR / "libunsafe_perf.rlib"
PERF_DEPS = PERF_TARGET_DIR / "deps"
# Prebuilt per-feature instrumentation libraries shipped with the artifact
# (unsafe_perf_prebuilt/<feature>/libunsafe_perf.rlib + deps/). Built once with
# the same stage1 compiler that ships under toolchain/, so they are
# link-compatible.
RUNTIME_DIR = SCRIPT_DIR / "unsafe_perf_prebuilt"

# Experiment Definitions
EXPERIMENTS = {
    "cpu_cycle": {
        "feature": "cpu_cycle_counter",
        "output_file": "cpu_cycle.stat",
        "flags": [
            "-C", "unsafe_include_native_lib=false",
            "-C", "llvm-args=-enable-instmarker",
            "-C", "llvm-args=-enable-cpu-cycle-count",
            "-C", "llvm-args=-enable-external-call-tracker",
        ]
    },
    "heap_tracker": {
        "feature": "heap_tracker",
        "output_file": "heap_stat.stat",
        "flags": [
            "-C", "unsafe_include_native_lib=false",
            "-C", "llvm-args=-enable-instmarker",
            "-C", "llvm-args=-enable-heap-tracker",
        ]
    },
    "unsafe_counter": {
        "feature": "unsafe_counter",
        "output_file": "unsafe_counter.stat",
        "flags": [
            "-C", "unsafe_include_native_lib=false",
            "-C", "llvm-args=-enable-instmarker",
            "-C", "llvm-args=-enable-unsafe-function-tracker",
            "-C", "llvm-args=-enable-unsafe-inst-counter",
        ]
    },
    "coverage": {
        "feature": "unsafe_coverage",
        "output_file": "unsafe_coverage.stat",
        "flags": [
            "-C", "unsafe_include_native_lib=false", # Note: false for coverage
            "-C", "llvm-args=-enable-instmarker",
            "-C", "llvm-args=-enable-dynamic-line-count",
        ]
    },
    "native": {
        "feature": "", 
        "output_file": "",
        "flags": []
    }
}

# Crate-Specific Configurations
CRATE_CONFIGS = {
    "rayon": {
        "cwd": "rayon-demo",
        "cmds": [
            "cargo clean",
            "cargo build --release",
            "../target/release/rayon-demo nbody bench --bodies 500"
        ]
    },
    "parking_lot": {
        "cwd": "benchmark", 
        "cmds": [
            "cargo clean",
            "cargo build --release",
            "./target/release/mutex 2 4 10 2 4",
            "./target/release/rwlock 4 4 4 10 2 4"
        ]
    },
    "memchr": {
        "cmds": [
            "cargo install --path ../rebar --force",
            f"{os.path.expanduser('~/.cargo/bin/rebar')} build -e 'rust/memchr/memmem/(oneshot)'",
            f"{os.path.expanduser('~/.cargo/bin/rebar')} measure --verify -e 'rust/memchr/memmem/(oneshot)'",
        ],
        "flags": ["-C", "target-feature=-sse2,-avx2"]
    },
    "simd-json": {
        "cmds": [
            "cargo update -p half --precise 2.3.1",
            "cargo update -p proptest --precise 1.4.0",
            "cargo clean",
            "cargo build --release",
            "cargo bench"
        ]
    },
    "jni": {
        "cmds": [
            "cargo clean",
            "cargo build --release --features invocation",
            "cargo bench --features invocation"
        ]
    },
    "ring": {
        "cwd": "bench",
        "cmds": [
            "cargo clean", 
            "cargo build --release",
            "cargo bench"
        ],
        "use_absolute_rustflags": True,
        "env": {"CC": "clang"}
    },
    "tokio": {
        "cwd": "benches",
        "cmds": [
            "cargo update -p half --precise 2.3.1",
            "cargo update -p proptest --precise 1.4.0",
            "cargo clean",
            "cargo build --release", 
            "cargo bench --locked"
        ],
        "timeout": 3600
    },
    "rayon-core": {
        "skip": True
    }
}

def run_cmd(cmd, cwd=None, env=None, timeout=600):
    """Run a shell command with default 10min timeout."""
    print(f"Running: {cmd} (cwd={cwd})")
    try:
        subprocess.run(
            cmd, 
            cwd=cwd, 
            env=env, 
            check=True, 
            shell=True,
            timeout=timeout
        )
        return True
    except subprocess.CalledProcessError as e:
        print(f"Command failed with exit code {e.returncode}: {cmd}")
        return False
    except subprocess.TimeoutExpired:
        print(f"Command timed out: {cmd}")
        return False

def build_perf(feature):
    """Provision the instrumentation library for the given feature.

    Prefers the prebuilt per-feature library shipped under
    unsafe_perf_prebuilt/<feature>/ (rlib + deps) and stages it into
    unsafe_perf_source/target/release/, so no per-feature recompilation is
    needed. Falls back to compiling it from source only when no prebuilt library
    exists for the feature (e.g. unsafe_coverage)."""
    if not feature:  # native: no instrumentation library
        return

    prebuilt_rlib = RUNTIME_DIR / feature / "libunsafe_perf.rlib"
    prebuilt_deps = RUNTIME_DIR / feature / "deps"

    if prebuilt_rlib.exists():
        print(f"using prebuilt instrumentation library for feature: {feature}")
        # Replace unsafe_perf_source/target/release with this feature's prebuilt
        # rlib + deps so the PERF_RLIB / PERF_DEPS paths resolve to the right
        # instrumentation and features never mix.
        if PERF_TARGET_DIR.exists():
            shutil.rmtree(PERF_TARGET_DIR)
        PERF_TARGET_DIR.mkdir(parents=True, exist_ok=True)
        shutil.copy2(prebuilt_rlib, PERF_RLIB)
        if prebuilt_deps.is_dir():
            shutil.copytree(prebuilt_deps, PERF_DEPS)
        return

    # Fallback: compile the library from source with the prebuilt stage1 compiler.
    print(f"no prebuilt instrumentation library for '{feature}'; building from source...")
    env = os.environ.copy()
    env.pop("RUSTFLAGS", None)  # avoid injecting instrumentation flags into this build
    run_cmd("cargo clean", cwd=PERF_DIR, env=env)
    if not run_cmd(f"cargo build --release --features {feature}", cwd=PERF_DIR, env=env):
        print(f"Failed to build perf library for {feature}")
        sys.exit(1)

def run_crate(crate_name, exp_name, config, output_dir):
    """Run experiment for a single crate."""
    print(f"Processing crate: {crate_name} [{exp_name}]")
    
    crate_dir = BENCHMARK_DIR / crate_name
    if not crate_dir.exists():
        # Try finding fuzzy match
        matches = [d for d in BENCHMARK_DIR.iterdir() if d.is_dir() and d.name.startswith(crate_name)]
        if matches:
             crate_dir = matches[0]
             print(f"Found crate directory: {crate_dir.name}")
        else:
             print(f"Crate directory not found: {crate_name}")
             return

    # Check for custom config
    # Matches 'rayon' or 'rayon-1.5.0' -> check if key is in name?
    # Better: check if crate_name (dirname) starts with key
    custom_config = None
    for k, v in CRATE_CONFIGS.items():
        if crate_name == k or crate_name.startswith(k + "-"):
            custom_config = v
            break
            
    if custom_config and custom_config.get("skip"):
        print(f"Skipping {crate_name} as per config.")
        return

    # Prepare environment
    env = os.environ.copy()
    env["RUSTC_BOOTSTRAP"] = "1" # Force nightly features for unstable flags
    env["RUSTUP_TOOLCHAIN"] = "stage1" # Force unified toolchain (1.80.0-dev) that supports unsafe info
    if "RUSTC" in env:
        del env["RUSTC"] # Ensure we use RUSTUP_TOOLCHAIN selection, not shell override
    
    # Calculate relative paths for flags when running inside crate_dir
    
    # Calculate relative paths for flags when running inside crate_dir
    # RUSTFLAGS paths must be relative to the CWD of the cargo process (crate_dir)
    try:
        # PERF_RLIB is typically "perf/target/release/libunsafe_perf.rlib" relative to root
        # crate_dir is "benchmarks/foo" relative to root
        # We need path from benchmarks/foo -> perf/target...
        # Using os.path.relpath(target, start)
        
        # Resolve to absolute first to be safe for calculation, then convert to relative
        abs_crate_dir = crate_dir.resolve()
        abs_perf_rlib = PERF_RLIB.resolve()
        abs_perf_deps = PERF_DEPS.resolve()
        abs_output_dir = output_dir.resolve()
        
        rel_perf_rlib = os.path.relpath(abs_perf_rlib, abs_crate_dir)
        rel_perf_deps = os.path.relpath(abs_perf_deps, abs_crate_dir)
        rel_output_dir = os.path.relpath(abs_output_dir, abs_crate_dir)
        
    except Exception as e:
        print(f"Error calculating relative paths: {e}")
        return

    # Construct RUSTFLAGS
    # Only inject unsafe_perf if NOT native experiment
    if exp_name != "native":
        use_absolute = custom_config.get("use_absolute_rustflags", False) if custom_config else False
        
        if use_absolute or (custom_config and "cwd" in custom_config):
            # Use absolute paths for workspace members
            rustflags = [
                "--emit=llvm-ir,link",
                "-Z", "unstable-options",
                f"--extern", f"force:unsafe_perf={PERF_RLIB.resolve()}",
                "-L", f"{PERF_DEPS.resolve()}"
            ]
        else:
            rustflags = [
                "--emit=llvm-ir,link",
                "-Z", "unstable-options",
                f"--extern", f"force:unsafe_perf={PERF_RLIB}",
                "-L", f"{PERF_DEPS}"
            ]
        rustflags.extend(config["flags"])
        
        env["RUSTFLAGS"] = " ".join(rustflags)
    
    env["UNSAFE_BENCH_OUTPUT_DIR"] = str(abs_output_dir)
    env["CARGO_PRIMARY_PACKAGE"] = "1"
    print(f"DEBUG: UNSAFE_BENCH_OUTPUT_DIR={abs_output_dir} (cwd={crate_dir})")
    
    # Determine execution strategy
    exec_cwd = crate_dir
    # Determine execution strategy
    exec_cwd = crate_dir
    cmds = ["cargo clean", "cargo build --release", "cargo bench"] # Default sequence
    
    if custom_config:
        if "cwd" in custom_config:
            exec_cwd = crate_dir / custom_config["cwd"]
            # Re-calculate relative output dir for the NEW cwd
            try:
                abs_exec_cwd = exec_cwd.resolve()
                rel_output_dir_custom = os.path.relpath(abs_output_dir, abs_exec_cwd)
                env["UNSAFE_BENCH_OUTPUT_DIR"] = str(rel_output_dir_custom)
                print(f"DEBUG: Custom CWD UNSAFE_BENCH_OUTPUT_DIR={rel_output_dir_custom}")
            except Exception as e:
                print(f"Error calculating relative path for custom cwd: {e}")
        
        if "cmds" in custom_config:
            cmds = custom_config["cmds"]

        if "env" in custom_config:
            env.update(custom_config["env"])

    # Execute Commands
    success = True
    cmd_timeout = custom_config.get("timeout", 600) if custom_config else 600
    for cmd in cmds:
        if not run_cmd(cmd, cwd=exec_cwd, env=env, timeout=cmd_timeout):
            print(f"Command failed: {cmd}")
            success = False
            break # Stop executing subsequent commands (e.g. run after build) if previous failed
            
    if success:
        print(f"Success: {crate_name}")
        
        # Rename output file
        # The runtime writes to output_dir / output_file
        expected_file = output_dir / config["output_file"]
        if config["output_file"] and expected_file.exists():
            new_name = output_dir / f"{crate_name}_{config['output_file']}"
            shutil.move(expected_file, new_name)
            print(f"Saved results to: {new_name.name}")
        else:
             fallback_file = Path("/tmp") / config["output_file"]
             if fallback_file.exists():
                 print(f"Found results in fallback location: {fallback_file}")
                 new_name = output_dir / f"{crate_name}_{config['output_file']}"
                 shutil.move(fallback_file, new_name)
                 print(f"Saved results to: {new_name.name}")
             else:
                 # It implies no coverage/stats were written.
                 # For some crates (like parking_lot), running the binary works.
                 # If file not found, maybe it wasn't named as expected?
                 # But the runtime always writes to {UNSAFE_BENCH_OUTPUT_DIR}/unsafe_coverage.stat etc.
                 print(f"Warning: Expected output file not found: {expected_file} or {fallback_file}")

def main():
    parser = argparse.ArgumentParser(description="Unsafe Rust Benchmark Pipeline")
    parser.add_argument("--crate", help="Run for specific crate")
    parser.add_argument("--experiment", choices=EXPERIMENTS.keys(), help="Run specific experiment (default: native)")
    parser.add_argument("--all", action="store_true", help="Run all experiments")
    parser.add_argument("--output", help="Custom output directory")
    parser.add_argument("--showstats", action="store_true", help="Display aggregated stats table")
    
    args = parser.parse_args()
    
    if not args.all and not args.experiment:
        print("No experiment specified, running in 'native' mode (compile/bench only).")
        print("For coverage, use: python3 run_pipeline.py --experiment coverage")
        args.experiment = "native"

    # Setup Output Directory
    timestamp = datetime.now().strftime("%Y-%m-%d_%H-%M-%S")
    if args.output:
        base_output_dir = Path(args.output)
    else:
        base_output_dir = SCRIPT_DIR / "results" / timestamp
    
    base_output_dir.mkdir(parents=True, exist_ok=True)
    print(f"Results will be stored in: {base_output_dir}")

    # Determine what to run
    experiments_to_run = []
    if args.all:
        experiments_to_run = list(EXPERIMENTS.keys())
    else:
        experiments_to_run = [args.experiment]

    crates_to_run = []
    if args.crate:
        crates_to_run = [args.crate]
    else:
        # Auto-discover crates
        crates_to_run = [d.name for d in BENCHMARK_DIR.iterdir() if d.is_dir()]

    print(f"Experiments: {experiments_to_run}")
    print(f"Crates: {len(crates_to_run)}")

    # Execution Loop
    for exp in experiments_to_run:
        print(f"\n=== Starting Experiment: {exp} ===")
        config = EXPERIMENTS[exp]
        
        # 1. Build Perf Lib
        build_perf(config["feature"])
        
        # 2. Run Crates
        for crate in crates_to_run:
            # We use the same output dir for all experiments, 
            # the file suffices (e.g. _cpu_cycle.stat) differentiate them.
            # But coverage tracks cumulative runs.
            # Each execute appends.
            
            run_crate(crate, exp, config, base_output_dir)

    # Aggregation
    # Aggregation
    if args.showstats:
        print("\n=== Aggregating Results ===")
        if Aggregator is None:
            print("Stats aggregator is not bundled in this artifact; "
                  "the per-binary .stat files in the results directory are the "
                  "source of truth.")
        else:
            agg = Aggregator(base_output_dir)
            agg.collect_all()
            agg.print_table()
    
    print(f"\nFull results in: {base_output_dir}")

if __name__ == "__main__":
    main()
