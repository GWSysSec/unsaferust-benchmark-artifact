# Dynamic Analysis of Unsafe Rust: Behavior and Benchmarks

This is the research artifact for our paper on the dynamic (runtime) behavior of
unsafe Rust. It bundles everything needed to reproduce our measurements:

- a **prebuilt instrumented Rust compiler** (`rustc 1.80.0-dev` with our LLVM
  instrumentation passes),
- the **instrumentation runtime library** (`unsafe_perf`), prebuilt per feature,
- the **benchmark suite** and the **enhanced coverage workloads** (LLM-generated
  integration tests) used to drive execution,
- the **dynamic measurement results** for all 100 analyzed crates, and
- the scripts to re-run any experiment.

The compiler and instrumentation libraries are shipped **prebuilt**, so there is
no multi-hour `rustc`/LLVM build. The Docker image builds in a few minutes.

## Important: clone with Git LFS

The prebuilt compiler and instrumentation libraries (~0.9 GB) are stored with
**Git LFS**. Install LFS *before* cloning, or the large files arrive as small
pointer text files:

```bash
git lfs install
git clone <repo-url>

# If you already cloned without LFS:
git lfs pull
```

Quick check that the real binaries are present (not pointers):

```bash
file toolchain/lib/librustc_driver-*.so   # should say "ELF ... shared object"
```

## Repository layout

| Path | Contents |
|------|----------|
| `toolchain/` | Prebuilt instrumented compiler (`rustc`/`rustdoc` + sysroot). Linked as the `stage1` rustup toolchain. |
| `unsafe_perf_source/` | Source of the `unsafe_perf` instrumentation library (`Makefile`, `src/`). |
| `unsafe_perf_prebuilt/` | Prebuilt `libunsafe_perf.rlib` (+ `deps/`) for each feature: `cpu_cycle_counter/`, `heap_tracker/`, `unsafe_counter/`. |
| `benchmarks/` | The benchmark suite (real crates run under `cargo bench`/`cargo test`). |
| `generated_tests/` | Enhanced coverage workloads: LLM-generated integration tests, per crate. |
| `dynamic_analysis_results/` | Per-crate dynamic measurements, grouped by research question (`rq1`–`rq5`) and variant. See its own `README.md`. |
| `analyzed_crates.csv` | The 100-crate dataset: version, LOC, static-unsafe %, and pre/post API coverage. |
| `workload_descriptions/` | One-line workload summary per crate (`workload_descriptions.csv`). |
| `experiment_env/` | Per-experiment environment presets for the manual flow (`env/*.sh`). |
| `benchmark_configs.md` | Per-benchmark commands, flags, and static characteristics. |
| `run_pipeline.py` | Automated experiment driver. |
| `Dockerfile`, `docker-compose.yml`, `docker-build.sh` | Container build/run. |

## Quick start (Docker, recommended)

```bash
git lfs pull                 # ensure prebuilt binaries are materialized
./docker-build.sh            # builds image unsaferust-bench:local (a few minutes)
docker run -it unsaferust-bench:local
```

Or with Compose:

```bash
docker compose build
docker compose run --rm unsaferust-bench
```

The container links the prebuilt compiler as the `stage1` toolchain and stages a
ready-to-use instrumentation library, so experiments run out of the box.

## Running experiments

Inside the container (working dir `/workspace`):

```bash
# Native baseline (compile + run, no instrumentation), all crates:
python3 run_pipeline.py

# A specific instrumentation experiment, all crates:
python3 run_pipeline.py --experiment unsafe_counter --showstats

# Everything:
python3 run_pipeline.py --all --showstats
```

Options:

- `--experiment <name>`: one of `native`, `coverage`, `cpu_cycle`,
  `heap_tracker`, `unsafe_counter`.
- `--crate <name>`: restrict to a single crate.
- `--showstats`: print an aggregated summary table.
- `--output <dir>`: results directory (default: `results/<timestamp>/`).

For each feature the driver stages the matching prebuilt library from
`unsafe_perf_prebuilt/` (no recompilation). `coverage` is the one feature that is
not prebuilt; it is compiled from `unsafe_perf_source/` on demand with the
prebuilt compiler.

## Manual flow (single crate, by hand)

```bash
# 1. Select an instrumentation by sourcing its preset (sets RUSTFLAGS + toolchain):
source experiment_env/env/cpu.sh        # or heap.sh / counter.sh / coverage.sh

# 2. Build + run a benchmark:
cd benchmarks/arrayvec-0.7.6
cargo bench

# 3. Inspect the per-binary stat dump:
ls -l /tmp/*.stat
```

The `cpu`, `heap`, and `counter` presets point directly at the prebuilt library,
so no build is needed. `coverage.sh` expects
`cd unsafe_perf_source && make coverage` first. Source only one preset per shell.

## Results and datasets

- **`dynamic_analysis_results/`** holds one JSON per crate under
  `rq{1..5}_<name>/{with_native,without_native}/<crate>.json`. `with_native`
  also counts code reached through `std`/`core`/`alloc`; `without_native` counts
  only the crate itself. See `dynamic_analysis_results/README.md` for the full
  field glossary.
- **`analyzed_crates.csv`** is the per-crate dataset; its `crate` column is the
  join key for the result files.
- **`generated_tests/`** are the enhanced coverage workloads that raise API
  coverage before measurement; **`workload_descriptions/`** summarizes each.

## Toolchain details

`toolchain/` is a prebuilt stage1 `rustc 1.80.0-dev`. It is self-contained
(relative rpath) and requires only glibc 2.34, so it runs as-is on the Ubuntu
22.04 base image. The Dockerfile registers it with:

```bash
rustup toolchain link stage1 /workspace/toolchain
```

Scripts select it via `RUSTUP_TOOLCHAIN=stage1` (with `RUSTC_BOOTSTRAP=1` for the
unstable instrumentation flags).

## Rebuilding the instrumentation library from source (optional)

```bash
cd unsafe_perf_source
make cpu          # or: heap | counter | coverage   (one feature at a time)
# produces target/release/libunsafe_perf.rlib (+ deps/)
```

## FAQ

- **Files look tiny / are text pointers.** Git LFS was not active at clone time.
  Run `git lfs install && git lfs pull`.
- **Memory.** Give Docker at least 8 GB RAM.
- **A crate shows no instrumentation data.** The crate's `Cargo.toml` must enable
  debug info so the passes can attribute instructions:

  ```toml
  [profile.release]
  debug = 2
  [profile.bench]
  debug = 2
  ```

  The bundled benchmark crates already have this set.
- **Output paths.** The automated pipeline writes to `results/<timestamp>/`;
  the manual flow writes `/tmp/*.stat` (controlled by `UNSAFE_BENCH_OUTPUT_DIR`).
