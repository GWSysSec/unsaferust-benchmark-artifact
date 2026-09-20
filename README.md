# Dynamic Analysis of Unsafe Rust: Behaviors and Benchmarks — artifact

Measure the 100-crate corpus and rebuild the paper's tables and figures from
what you measured. The artifact contains the instrumented compiler as source
and prebuilt, all 100 crate source trees, the 19-crate benchmark suite, and one
script per table and figure. No GitHub account, access token or other
credential is needed.

## Sizes

| | |
|---|---|
| artifact directory | 651 MB |
| Docker image | 4.88 GB |
| compiler volume, prebuilt | 343 MB |
| compiler volume, built from source | 7.0 GB |
| corpus, unpacked | 419 MB |

## Run it

```bash
docker/build.sh                 # 7 min   image, then the prebuilt compiler
docker/run.sh                   #         shell at /workspace/artifact
```

Inside that shell, one command does everything:

```bash
run/reproduce.sh                # 25 min  fetch, measure 12 crates, build every
                                #         table and figure from the result
```

Output is in `results/<timestamp>/`, and the tables and figures in
`results/<timestamp>/tables/`. `results/` and `corpus/sources/` are mounted from
the host and owned by your user account after the container exits.

Larger runs:

```bash
run/reproduce.sh --tier fast    # 58 crates, about 2 hours
run/reproduce.sh --tier full    # 100 crates, 121.7 hours — the paper's corpus
```

Times are from a 32-core machine with 30 GB of memory.

## For an automated reviewer

Run these and check the quoted strings. `--tier full` takes 121.7 hours; the
default tier is the one to use.

```bash
docker/build.sh                                     # "rustc 1.80.0-dev"
docker/run.sh bash -c 'run/check_shipped_data.sh'   # "The shipped data reproduces every table in the paper."
docker/run.sh bash -c 'run/reproduce.sh'            # "rebench complete: 12 crate(s)"
```

A non-zero exit is a failure. Each table script prints the table built from the
measurement, the same table as submitted, and a per-crate comparison. Read
**Expected variation** before reading the comparison.

## The compiler

```bash
docker/build.sh                 # unpack the prebuilt toolchain   1 min
docker/build.sh --from-source   # build it from source            836 s, 7.0 GB
```

`compiler/stage1-toolchain.tar.zst` (87 MB) is the stage-1 toolchain built from
`compiler/compiler-src.tar.zst` (213 MB): rustc, the libraries it links, and the
standard library compiled against it. `tools/package_toolchain.sh` produced it
and tested it first.

The source is rustc 1.80.0-dev with LLVM 18, the instrumentation passes, and the
rustc changes that carry unsafe metadata from HIR to LLVM IR. Assertions are on,
as in the measurements. `compiler/COMMIT` records the commit of the rustc tree
and of all twelve submodules.

The compiler lives in a Docker volume, not an image layer. An interrupted build
resumes when you run the script again. `docker volume rm unsaferust-compiler`
reclaims the space. Parallelism is capped by available memory; override it with
`COMPILE_JOBS=8 LINK_JOBS=2 docker/build.sh --from-source`.

Docker names: image `unsaferust-artifact:local`, volume `unsaferust-compiler`.

## The crate sources

```bash
run/fetch_corpus.sh
```

Unpacks all 100 crate source trees, exactly as measured, from
`corpus/corpus-sources.tar.zst` (75 MB compressed, 419 MB unpacked). No network.
`run/reproduce.sh` calls this for you if `corpus/sources/` is empty.

They are shipped rather than cloned because 97 of the 100 come from git
repositories, and cloning 97 repositories from GitHub in one sitting hits the
unauthenticated rate limit, which would force you to create and paste a personal
access token.

Measuring itself does need network: each crate's dependencies come from
crates.io at the versions its `Cargo.lock` pins.

## Measure on its own

```bash
run/measure.sh --tier smoke          # 12 crates, 20 min
run/measure.sh --tier fast           # 58 crates, 1.5 h
run/measure.sh --tier full           # 100 crates, 121.7 h
run/measure.sh --crate tokio,bytes   # a named list
```

`--scope alldeps` instruments every crate in the dependency graph instead of
only the crate under study.

## Script to table or figure

| Script | Paper | Content |
|---|---|---|
| `tables/rq1_cpu_cycles.sh` | Table `table:cpu_cycle` | CPU cycles spent in unsafe code |
| `tables/figure_cycles_cdf.sh` | Figure `fig:cpu_cycle` | distribution of that share |
| `tables/rq2_heap.sh` | Table `table:heap` | heap memory reached by unsafe code |
| `tables/figure_heap_cdf.sh` | Figure `fig:heap_standard_cdf` | distribution of the unsafe heap share |
| `tables/rq4_inst_types.sh` | Table `table:inst` | RQ3 and RQ4: how often executed instructions are unsafe, and of what type |
| `tables/rq5_unsafe_functions.sh` | Table `table:unsafe_function` | how often functions holding unsafe code run |
| `tables/rq3_unsafe_inst_frequency.sh` | — | RQ3 on its own; not a table in the submitted paper |

The paper's "on average, 1.8% of executed instructions are unsafe" is the
Geomean cell of the **Total** row under **Dynamic counts w/o std libs** in
`table:inst`, built by `tables/rq4_inst_types.sh`.

Each script builds the table from your newest run in `results/`, prints it
beside the submitted table, and compares crate by crate against our data.
`--run DIR` selects a run; `--our-data` uses ours as the input instead.

`bench_dyn`, `benchmarks` and `bench_comparisons` describe the benchmark suite
and were made by hand; there is no script for them.

## Expected variation

Comparisons are in shares — unsafe instructions out of all executed
instructions, unsafe heap bytes out of all heap bytes — because that is what the
paper's tables report and what carries across machines. Counts do not: they
depend on the hardware and on how much work a test suite happens to do.

Three things move the numbers between runs:

1. **Generated test input.** petgraph, zopfli, portable-atomic, http and slotmap
   generate their test input, so each run does different work. petgraph's unsafe
   instruction count moved 73% between two of our own runs.
2. **The machine.** CPU cycles depend on the processor and on load, so RQ1
   varies more than the instruction counters. Tests that time out or depend on
   thread scheduling move with load.
3. **Which binaries ran.** A crate's share is dominated by its largest test
   binary, so one binary doing different work moves the crate. deranged is the
   clearest case in RQ2.

Running `run/reproduce.sh` here and comparing against our data, the median
difference was 0.21% for RQ3, 0.00% for RQ4, 0.15% for RQ5, 8.25% for RQ1 and
8.52% for RQ2, over 24 crate-and-variant pairs. A few crates outside 10% is the
expected outcome.

A 12-crate table and the paper's 100-crate table are different statistics, and
each script says so. The per-crate comparison is unaffected: it compares your
crates against the same crates in our data.

## Check our data without measuring

```bash
run/check_shipped_data.sh
```

Rebuilds every table from the shipped data and compares it character for
character with the submitted table. Twenty seconds, no network. All five pass.

## The benchmark suite

`benchmark_suite/` is the paper's second contribution, not the study corpus: 19
crates for measuring what unsafe-Rust defenses cost, run through `cargo bench`.
Sixteen also appear in the corpus; `rayon`, `rebar` and `simd-json` do not.
Per-crate commands are in `docs/benchmark_configs.md`.

## Layout

    compiler/            compiler source (213 MB) and prebuilt toolchain (87 MB)
    corpus/              100 crate sources (75 MB), crate list, generated workloads
    benchmark_suite/     the 19-crate benchmark suite
    data/                our measurement data and the tables as submitted
    docker/              image and compiler-volume scripts
    run/                 reproduce, fetch, measure, check
    tables/              one script per table and figure
    tools/               aggregation, comparison, packaging, harness
    unsafe_perf_source/  instrumentation runtime library
    docs/                crate dataset and benchmark configurations

`ARCHITECTURE.md` explains why the artifact is built this way.

## Scope of the results

The corpus crates are libraries, not applications, and most sit low in a
dependency tree. The workloads are each crate's integration tests plus generated
drivers that raise public-API coverage from 76.4% to 90.4%.
