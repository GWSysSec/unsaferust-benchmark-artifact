# Dynamic Analysis of Unsafe Rust: Behaviors and Benchmarks — artifact

Measure the 100-crate corpus yourself and compare the result with the paper.
The artifact contains the instrumented compiler as source and prebuilt, all 100
crate source trees, the 19-crate benchmark suite, and one script per table and
figure. No GitHub account, access token or other credential is needed.

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
docker/build.sh                  # 7 min   image, then the prebuilt compiler
docker/run.sh                    #         shell at /workspace/artifact
```

Inside that shell:

```bash
run/fetch_corpus.sh              # 2 s     unpack the 100 crate sources
run/measure.sh --tier smoke      # 20 min  measure 12 crates
tables/rq4_inst_types.sh         # 1 min   build Table `table:inst` from it
```

Times are from a 32-core machine with 30 GB of memory.

`results/` and `corpus/sources/` are mounted from the host and owned by your
user account after the container exits.

## For an automated reviewer

Run these and check the quoted strings. Do not run `--tier full`; it takes
121.7 hours.

```bash
docker/build.sh                                     # "rustc 1.80.0-dev"
docker/run.sh bash -c 'run/fetch_corpus.sh'         # "crates: 100"
docker/run.sh bash -c 'run/check_shipped_data.sh'   # "The shipped data reproduces every table in the paper."
docker/run.sh bash -c 'run/measure.sh --tier smoke' # "rebench complete: 12 crate(s)"
docker/run.sh bash -c 'for s in tables/*.sh; do bash "$s"; done'
```

A non-zero exit is a failure. A difference in the per-crate comparison is not;
read **Why numbers vary** first.

Evidence afterwards, on the host: `results/<timestamp>/` is the measurement,
`results/<timestamp>/tables/` holds every table and figure built from it.

## The compiler: prebuilt and source

```bash
docker/build.sh                  # unpack the prebuilt toolchain   1 min
docker/build.sh --from-source    # build it from source            836 s, 7.0 GB
```

`compiler/stage1-toolchain.tar.zst` (87 MB) is the stage-1 toolchain we built
from `compiler/compiler-src.tar.zst` (213 MB): rustc, the libraries it links,
and the standard library compiled against it. `tools/package_toolchain.sh`
produced it and tested it first.

`compiler-src.tar.zst` is rustc 1.80.0-dev with LLVM 18, the instrumentation
passes, and the rustc changes that carry unsafe metadata from HIR to LLVM IR.
Assertions are on, as in the measurements. `compiler/COMMIT` records the commit
of the rustc tree and of all twelve submodules.

The compiler lives in a Docker volume, not an image layer. An interrupted build
resumes when you run the script again. `docker volume rm unsaferust-compiler`
reclaims the space. Parallelism is capped by available memory; override it with
`COMPILE_JOBS=8 LINK_JOBS=2 docker/build.sh --from-source`.

Docker names: image `unsaferust-artifact:local`, volume `unsaferust-compiler`.

## The crate sources are packaged

```bash
run/fetch_corpus.sh
```

Unpacks all 100 crate source trees, exactly as measured, from
`corpus/corpus-sources.tar.zst` (75 MB compressed, 419 MB unpacked). No network.

All 100 are in the tarball. They are shipped rather than cloned because 97 of
them come from git repositories and 3 from crates.io releases, and cloning 97
repositories from GitHub in one sitting hits the unauthenticated rate limit,
which would force you to create and paste a personal access token.

Rebuilding them from their original sources is still available:

```bash
run/fetch_corpus.sh --from-upstream                          # needs network
run/fetch_corpus.sh --from-upstream --verify corpus/sources  # and compare
```

This reads `corpus/corpus_lock.csv` (the exact commit or release per crate) and
applies `corpus/corpus_overlay/`, which holds every difference from upstream,
including the 113 `Cargo.lock` files that pin dependency versions. All 100
crates rebuild and verify file by file against our trees.

Measuring needs network: each crate's dependencies come from crates.io at the
versions its `Cargo.lock` pins.

If a build fails because a dependency needs a newer edition than rustc 1.80
understands, the harness deletes that crate's `Cargo.lock`, re-resolves to older
versions and retries, printing `retrying after lockfile regen`. This happened to
one crate, borsh-rs.

## Measure

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

## Why numbers vary

1. **Randomised workloads.** petgraph, zopfli, portable-atomic, http and
   slotmap generate their test input, so each run does different work.
   petgraph's unsafe instruction count moved 73% between two of our own runs.
2. **The machine.** CPU cycles depend on the processor and on load, so RQ1
   compares the share of cycles and still varies more than the instruction
   counters. Tests that time out or depend on thread scheduling move with load.
3. **Which binaries ran.** A crate's share is dominated by its largest test
   binary, so one binary doing different work moves the crate.

Smoke tier against our data: median difference 0.21% for RQ3, 0.00% for RQ4 and
RQ5, 6.83% for RQ1, 8.14% for RQ2. Of 24 crate-and-variant pairs, 12, 16 and 14
agreed to one part in ten thousand for RQ3, RQ4 and RQ5.

## Before quoting a number

**Comparisons use shares, not counts.** In our published instruction-counter
run, 1,082 of 1,271 test binaries were measured twice and three four times, and
the aggregator sums every stat file, so our absolute totals are 1.596 times one
pass. `tools/analysis/audit_duplicate_stats.py` measures this and gates itself
by reproducing the published aggregation first. A difference below a tenth of a
percentage point counts as agreement.

**One heap figure does not hold up.** deranged's published heap share is 0.0258%
for both `with_native` and `without_native`; a fresh run reads 22.3% and 0.028%.
Eight of 100 crates report the same unsafe heap bytes in both variants, three of
them nonzero.

**A partial run gives a partial table.** A 12-crate table and the paper's
100-crate table are different statistics. The per-crate comparison each script
prints is unaffected: it compares your crates against the same crates in our
data.

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
    corpus/              100 crate sources (75 MB), lock, overlay, workloads
    benchmark_suite/     the 19-crate benchmark suite
    data/                our measurement data and the tables as submitted
    docker/              image and compiler-volume scripts
    run/                 fetch, measure, check
    tables/              one script per table and figure
    tools/               aggregation, comparison, packaging, harness
    unsafe_perf_source/  instrumentation runtime library
    docs/                crate dataset and benchmark configurations

`ARCHITECTURE.md` explains why the artifact is built this way.

## Scope of the results

The corpus crates are libraries, not applications, and most sit low in a
dependency tree. The workloads are each crate's integration tests plus generated
drivers that raise public-API coverage from 76.4% to 90.4%.
