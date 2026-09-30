#!/usr/bin/env python3
import os
import sys
import subprocess
import argparse
from pathlib import Path
from datetime import datetime

SCRIPT_DIR = Path(__file__).parent.absolute()
ROOT = SCRIPT_DIR.parent
BENCHMARK_DIR = SCRIPT_DIR
PERF_DIR = ROOT / "unsafe_perf_source"
PERF_TARGET_DIR = PERF_DIR / "target" / "release"
PERF_RLIB = PERF_TARGET_DIR / "libunsafe_perf.rlib"
PERF_DEPS = PERF_TARGET_DIR / "deps"

TIMEOUT = 8 * 3600

BENCHMARK_CRATES = [
    "matrixmultiply", "arrayvec-0.7.6", "ndarray-0.16.1",
    "hashbrown-0.15.3", "async-task-4.7.1", "getrandom-0.3.2",
    "httparse-1.10.1", "smallvec-2.0.0-alpha.11", "memchr",
    "jpeg-decoder-master", "semver-1.0.26", "rayon", "jni-0.21.1",
    "parking_lot", "simd-json-0.14.3", "ring", "tokio",
    "petgraph-0.8.1",
]

EXPERIMENTS = {
    "cpu_cycle": {
        "feature": "cpu_cycle_counter",
        "metric": "cpu_cycle",
        "flags": [
            "-C", "unsafe_include_native_lib=false",
            "-C", "llvm-args=-enable-instmarker",
            "-C", "llvm-args=-enable-cpu-cycle-count",
            "-C", "llvm-args=-enable-external-call-tracker",
        ]
    },
    "heap_tracker": {
        "feature": "heap_tracker",
        "metric": "heap",
        "flags": [
            "-C", "unsafe_include_native_lib=false",
            "-C", "llvm-args=-enable-instmarker",
            "-C", "llvm-args=-enable-heap-tracker",
        ]
    },
    "unsafe_counter": {
        "feature": "unsafe_counter",
        "metric": "unsafe_counter",
        "flags": [
            "-C", "unsafe_include_native_lib=false",
            "-C", "llvm-args=-enable-instmarker",
            "-C", "llvm-args=-enable-unsafe-function-tracker",
            "-C", "llvm-args=-enable-unsafe-inst-counter",
        ]
    },
    "coverage": {
        "feature": "unsafe_coverage",
        "metric": "coverage",
        "flags": [
            "-C", "unsafe_include_native_lib=false",
            "-C", "llvm-args=-enable-instmarker",
            "-C", "llvm-args=-enable-dynamic-line-count",
        ]
    },
    "native": {
        "feature": "",
        "metric": None,
        "flags": []
    }
}

# Per-crate working directory, commands and extra environment.
CRATE_CONFIGS = {
    "rayon": {
        "cwd": "rayon-demo",
        "cmds": [
            "cargo clean",
            "cargo build --release --locked",
            "../target/release/rayon-demo nbody bench --bodies 500"
        ]
    },
    "parking_lot": {
        # not a member of parking_lot's workspace; has its own Cargo.lock
        "cwd": "benchmark",
        "cmds": [
            "cargo clean",
            "cargo build --release --locked",
            "./target/release/mutex 2 4 10 2 4",
            "./target/release/rwlock 4 4 4 10 2 4"
        ]
    },
    "memchr": {
        "cmds": [
            "cargo install --path ../rebar --locked --force",
            f"{os.path.expanduser('~/.cargo/bin/rebar')} build -e 'rust/memchr/memmem/(oneshot)'",
            f"{os.path.expanduser('~/.cargo/bin/rebar')} measure --verify -e 'rust/memchr/memmem/(oneshot)'",
        ]
    },
    "simd-json": {
        "cmds": [
            "cargo clean",
            "cargo build --release --locked",
            "cargo bench --locked"
        ]
    },
    "jni": {
        "cmds": [
            "cargo clean",
            "cargo build --release --locked --features invocation",
            "cargo bench --locked --features invocation"
        ]
    },
    "ring": {
        "cwd": "bench",
        "cmds": [
            "cargo clean",
            "cargo build --release --locked",
            "cargo bench --locked"
        ],
        "env": {"CC": "clang"}
    },
    "tokio": {
        "cwd": "benches",
        "cmds": [
            "cargo clean",
            "cargo build --release --locked",
            "cargo bench --locked"
        ]
    },
}


def run_cmd(cmd, cwd=None, env=None):
    print(f"Running: {cmd} (cwd={cwd})")
    try:
        subprocess.run(cmd, cwd=cwd, env=env, check=True, shell=True, timeout=TIMEOUT)
        return True
    except subprocess.CalledProcessError as e:
        print(f"Command failed with exit code {e.returncode}: {cmd}")
        return False
    except subprocess.TimeoutExpired:
        print(f"Command timed out: {cmd}")
        return False


def build_perf(feature):
    """Build the instrumentation library for the given feature."""
    if not feature:
        return
    print(f"building the instrumentation library for '{feature}'...")
    env = os.environ.copy()
    env.pop("RUSTFLAGS", None)
    run_cmd("cargo clean", cwd=PERF_DIR, env=env)
    if not run_cmd(f"cargo build --release --features {feature}", cwd=PERF_DIR, env=env):
        print(f"Failed to build perf library for {feature}")
        sys.exit(1)


def run_crate(crate_name, exp_name, config, output_dir):
    """Run experiment for a single crate."""
    print(f"Processing crate: {crate_name} [{exp_name}]")

    crate_dir = BENCHMARK_DIR / crate_name
    if not crate_dir.exists():
        matches = [d for d in BENCHMARK_DIR.iterdir() if d.is_dir() and d.name.startswith(crate_name)]
        if not matches:
            print(f"Crate directory not found: {crate_name}")
            return False
        crate_dir = matches[0]
        print(f"Found crate directory: {crate_dir.name}")

    custom_config = {}
    for k, v in CRATE_CONFIGS.items():
        if crate_name == k or crate_name.startswith(k + "-"):
            custom_config = v
            break

    # The runtime JSON names a binary and PID, but not its benchmark crate.
    output_dir = output_dir / crate_name
    output_dir.mkdir(parents=True, exist_ok=True)

    env = os.environ.copy()
    env["RUSTC_BOOTSTRAP"] = "1"
    env["RUSTUP_TOOLCHAIN"] = "stage1"
    env.pop("RUSTC", None)
    env.pop("RUSTFLAGS", None)

    if exp_name != "native":
        rustflags = [
            "--emit=llvm-ir,link",
            "-Z", "unstable-options",
            "--extern", f"force:unsafe_perf={PERF_RLIB}",
            "-L", f"{PERF_DEPS}",
        ]
        rustflags.extend(config["flags"])
        env["RUSTFLAGS"] = " ".join(rustflags)

    env["UNSAFE_STAT_DIR"] = str(output_dir.resolve())
    env["CARGO_PRIMARY_PACKAGE"] = "1"
    env.update(custom_config.get("env", {}))

    exec_cwd = crate_dir / custom_config.get("cwd", "")
    cmds = custom_config.get("cmds", ["cargo clean", "cargo build --release --locked", "cargo bench --locked"])

    metric = config["metric"]
    existing_stats = set(output_dir.glob(f"*.{metric}.json")) if metric else set()
    for cmd in cmds:
        if not run_cmd(cmd, cwd=exec_cwd, env=env):
            print(f"Command failed: {cmd}")
            return False

    print(f"Success: {crate_name}")
    if metric:
        written_stats = set(output_dir.glob(f"*.{metric}.json")) - existing_stats
        if written_stats:
            print(f"Saved {len(written_stats)} {metric} result(s) to: {output_dir}")
        else:
            print(f"Warning: No {metric} JSON results were written to: {output_dir}")
    return True


def main():
    parser = argparse.ArgumentParser(description="Unsafe Rust Benchmark Pipeline")
    parser.add_argument("--crate", help="Run for specific crate")
    parser.add_argument("--experiment", choices=EXPERIMENTS.keys(), help="Run specific experiment (default: native)")
    parser.add_argument("--all", action="store_true", help="Run all experiments")
    parser.add_argument("--output", help="Custom output directory")

    args = parser.parse_args()

    if not args.all and not args.experiment:
        print("No experiment specified, running in 'native' mode (compile/bench only).")
        print("For coverage, use: python3 run_pipeline.py --experiment coverage")
        args.experiment = "native"

    timestamp = datetime.now().strftime("%Y-%m-%d_%H-%M-%S")
    if args.output:
        base_output_dir = Path(args.output)
    else:
        base_output_dir = SCRIPT_DIR / "results" / timestamp

    base_output_dir.mkdir(parents=True, exist_ok=True)
    print(f"Results will be stored in: {base_output_dir}")

    if args.all:
        experiments_to_run = list(EXPERIMENTS.keys())
    else:
        experiments_to_run = [args.experiment]

    if args.crate:
        crates_to_run = [args.crate]
    else:
        crates_to_run = BENCHMARK_CRATES

    print(f"Experiments: {experiments_to_run}")
    print(f"Crates: {len(crates_to_run)}")

    failures = []
    for exp in experiments_to_run:
        print(f"\n=== Starting Experiment: {exp} ===")
        config = EXPERIMENTS[exp]
        build_perf(config["feature"])
        for crate in crates_to_run:
            if not run_crate(crate, exp, config, base_output_dir):
                failures.append((crate, exp))

    print(f"\nFull results in: {base_output_dir}")
    if failures:
        print(f"Failed crate/experiment pairs ({len(failures)}): "
              + ", ".join(f"{crate}/{exp}" for crate, exp in failures), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
