# Dynamic Analysis of Unsafe Rust

## Get Started

We provide our complete artifact as a docker image. The requirements to run our artifact include having an **x86** machine, with some distro of **Linux** with **Docker** installed.

```bash
#Load the tarball image into docker
docker load -i unsaferust-artifact-v2.tar

# Run an interactive session (terminal) inside the docker image
docker run -it unsaferust-artifact:v2
```

You should now be inside the our docker image and can continue as below.

## Quick Start

The provided Docker image starts in `/workspace/artifact`. To run every
experiment and generate every table and figure, run:

```bash
# 100 crate full run
bash run/reproduce.sh --tier full
```

This is our complete test reproduction script. It extracts the
100 crates if needed, measures all 100 crates with the supplied
instrumented compiler, and generates every table and figure. Results are saved
under `results/<timestamp>/`.

To run smaller subsets instead:

```bash
# 12 crate fastest run
bash run/reproduce.sh

# 58 crate manageable run
bash run/reproduce.sh --tier fast

# 1 crate run
bash run/reproduce.sh --crate bytes
```

## Image Structure

The image contains both our pre-built compiler and source code for our customized Rust compiler with our passes. You start inside the **workspace/artifact** path.

Below is our structure for the main artifact directory

```text
/workspace/artifact/
├── compiler/              # Compiler source archive + prebuilt toolchain
├── corpus/                # 100 crate archive + generated test workloads
├── data/                  # Published data + submitted tables and figures
├── docker/                # Docker scripts
├── run/                   # Scripts to reproduce experiments
├── tables/                # Scripts that generate table + figures
├── tools/                 # Scripts that compare results
├── unsafe_perf_source/    # Rust runtime library for instrumentation
├── benchmark_suite/       # 18 crate unsafe benchmark suite
├── docs/                  # Test workload descriptions + analyzed data
└── results/               # Output directory for your experiments
```

## Useful Commands

Additional commands that can be run inside the docker image.

| Command                                   | What it does                                                                                          |
| ----------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| `bash docker/run.sh`                      | Opens a shell with the compiler volume and persistent results mounts.                                 |
| `bash run/reproduce.sh`                   | Extracts the crates if needed, measures them, and produces every table and figure.                    |
| `bash run/reproduce.sh --tier fast`       | Runs the 58-crate fast tier, then generates every table and figure.                                   |
| `bash run/reproduce.sh --tier full`       | Runs all 100 corpus crates, then generates every table and figure.                                    |
| `bash run/fetch_corpus.sh`                | Extracts the shipped corpus source trees without network access.                                      |
| `bash run/measure.sh --tier smoke`        | Measures 12 representative crates without generating tables.                                          |
| `bash run/measure.sh --tier fast`         | Measures 58 faster crates without generating tables.                                                  |
| `bash run/measure.sh --tier full`         | Measures all 100 corpus crates without generating tables.                                             |
| `bash run/measure.sh --crate tokio,bytes` | Measures only the named crates.                                                                       |
| `bash run/check_shipped_data.sh`          | Rebuilds the five paper tables from shipped data and checks exact equality with the submitted tables. |

After a measurement, generate an individual output with one of:

```bash
bash tables/rq1_cpu_cycles.sh
bash tables/rq2_heap.sh
bash tables/rq3_unsafe_inst_frequency.sh
bash tables/rq4_inst_types.sh
bash tables/rq5_unsafe_functions.sh
bash tables/figure_cycles_cdf.sh
bash tables/figure_heap_cdf.sh
```

## Unsafe Rust Benchmarks

The separate 18 crate benchmark suite lives under `benchmark_suite/` and is not
part of the test crate reproduction flows above. See
`benchmark_suite/README.md` for the per-crate configurations and commands if
you want to inspect or run the individual repositories directly.

To run the benchmark suite without instrumentation, use its standalone entry
point from `/workspace/artifact`:

```bash
python3 benchmark_suite/run_pipeline.py
```

To run only one benchmark crate, pass its name e.g:

```bash
python3 benchmark_suite/run_pipeline.py --crate matrixmultiply
```

## Full Test Suite Runtime

The full crate list contains 100 crates. On our machine with 16 cores with 30 GB of memory, `bash run/reproduce.sh --tier full` takes about 2-3 days
of measurement time. The supported smaller runs are:

| Tier    | Crates Ran | Approximate runtime |
| ------- | ---------: | ------------------: |
| `smoke` |         12 |    about 25 minutes |
| `fast`  |         58 |   about 1.5-2 hours |
| `full`  |        100 |      about 2-3 days |

## Questions?

### Where are the compiler source and compiled toolchain?

The raw source archive is `compiler/compiler-src.tar.zst`. Docker unpacks it
into `/workspace/compiler-src` in the image. The built compiler is stored in
the `unsaferust-compiler` Docker volume, mounted at
`/workspace/compiler-src/build` when the container starts.

`bash docker/build.sh` uses the prebuilt toolchain by default. To build the
compiler from the shipped raw source instead, use:

```bash
bash docker/build.sh --from-source
```

### Cargo and Crate questions?

The corpus archive contains the exact 100 source trees and extracts without
network access. Cargo still downloads their pinned dependencies from crates.io
during measurement.
