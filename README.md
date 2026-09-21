# Dynamic Analysis of Unsafe Rust

This guide is the shortest path to getting started

## Quick Start

The provided Docker image starts in `/workspace/artifact`. To run every
experiment and generate every table and figure, run:

```bash
# 100 crate, full experimental run
bash run/reproduce.sh --tier full
```

This is the artifact's end-to-end reproduction entry point. It extracts the
100 crates if needed, measures all 100 crates with the supplied
instrumented compiler, and generates every table and figure. Results are saved
under `results/<timestamp>/`.

The 100-crate run takes about 121.7 hours on our 32 core machine.

To run smaller subsets instead:

```bash
# 12 crate smoke tier, about 25 minutes
bash run/reproduce.sh

# 58 crate manageable run
bash run/reproduce.sh --tier fast
```

## Image Structure

The image contains both our pre-built compiler and source code for our customized Rust compiler with our passes. You start inside the **workspace/artifact** path.

We list the main layout of the Docker image below

```text
Raw instrumented rustc 1.80.0-dev + LLVM 18 source
/workspace/compiler-src/

Built compiler, mounted from Docker volume
/workspace/compiler-src/build/

Repository files and artifact scripts
/workspace/artifact/
```

`bash docker/build.sh` uses the prebuilt toolchain by default. To build the
compiler from the shipped raw source instead, use:

```bash
bash docker/build.sh --from-source
```

Below is our structure for the main artifact directory

```text
Compiler source archive, prebuilt toolchain, source revision record
compiler/

100-crate crate archive, crate list, and generated test workloads
corpus/

Published measurement data plus submitted tables and figures
data/

Image definition and container setup scripts
docker/

Crate extraction, measurement, reproduction, and data-check scripts
run/

Wrappers that generate paper tables and figures
tables/

Measurement harness, aggregation, comparison, and analysis code
tools/

Rust runtime library used by the instrumentation
unsafe_perf_source/

19-crate benchmark suite
benchmark_suite/

Reference metadata and workload descriptions
docs/

Output directory created by measurements
results/
```

## Useful Commands

Run the commands inside
the container, for example with `bash docker/run.sh bash COMMAND`.

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

## Full Test Suite Runtime

The full crate list contains 100 crates. On our machine with 32 cores with 30 GB of memory, `bash run/reproduce.sh --tier full` takes about **121.7 hours**
of measurement time. The supported smaller runs are:

| Tier    | Crates Ran | Approximate runtime |
| ------- | ---------: | ------------------: |
| `smoke` |         12 |       25-32 minutes |
| `fast`  |         58 |   about 1.5-2 hours |
| `full`  |        100 |         121.7 hours |

## FAQ

### Where are the compiler source and compiled toolchain?

The raw source archive is `compiler/compiler-src.tar.zst`. Docker unpacks it
into `/workspace/compiler-src` in the image. The built compiler is stored in
the `unsaferust-compiler` Docker volume, mounted at
`/workspace/compiler-src/build` when the container starts.

### Why does the compiler not live in the image?

Keeping compiler output in a volume makes an interrupted source build resumable
and prevents its roughly 7 GB build output from becoming an image layer. Remove
it when no longer needed with:

```bash
docker volume rm unsaferust-compiler
```

### Why did a source compiler build fail around LLVM TableGen?

The artifact image installs CMake 3.31.6 because Ubuntu 22.04's packaged CMake
3.22.1 cannot parse an LLVM 18 TableGen dependency file.

### Cargo and Crate questions?

The corpus archive contains the exact 100 source trees and extracts without
network access. Cargo still downloads their pinned dependencies from crates.io
during measurement.

### Validating the measured data without running the whole artifact again?

Inside the container, run:

```bash
bash run/check_shipped_data.sh
```

It checks the paper tables and figures with your runs measurements.
