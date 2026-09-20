# Dynamic Analysis of Unsafe Rust: Behaviors and Benchmarks — artifact

This artifact lets you rerun the study and compare what you measure against
what the paper reports. It contains the source of the instrumented compiler,
the source of all 100 crates the study measures, the 19-crate benchmark suite
the paper proposes, and one script per table in the paper.

The intended path is that you measure the corpus yourself and build the tables
from your own numbers. Our measurement data is here too, but as the thing to
compare against, not as the input.

## In order

    docker/build.sh                  build the compiler from source
    docker/run.sh                    open a shell in the image
    run/fetch_corpus.sh              obtain the 100 crates at the exact versions
    run/measure.sh --tier smoke      measure
    tables/rq3_unsafe_inst_frequency.sh    build that table from what you measured

Each step is described below with the time it takes.

## 1. Build the compiler

```bash
docker/build.sh
```

Builds LLVM 18 with assertions and then rustc 1.80 from
`compiler/compiler-src.tar.zst`. Assertions are on because that is what the
measurements were taken with; turning them off would be a different compiler.

Budget about 40 GB of disk. The build caps its own parallelism by available
memory, roughly 2 GB per compile job, because an uncapped build on a machine
with less memory than cores gets its tablegen steps killed by the kernel and
reports only `FAILED` with nothing underneath. Override with
`--build-arg COMPILE_JOBS=n --build-arg LINK_JOBS=m`.

Then `docker/run.sh` gives you a shell at `/workspace/artifact` with the
compiler linked as the `stage1` toolchain.

## 2. Get the crate sources

```bash
run/fetch_corpus.sh
```

Clones or downloads each of the 100 crates at the exact commit or released
version in `corpus/corpus_lock.csv`, then applies `corpus/corpus_overlay/`,
which carries every difference between that upstream tree and the tree we
measured — including the 113 `Cargo.lock` files that pin the dependency
versions. Without those a rebuild resolves different dependencies and measures
a different program.

A few minutes and about 1.7 GB. `corpus/corpus-sources.tar.zst` is the same
thing packaged for evaluation without network access.

To confirm you got what we measured, pass `--verify-against` a reference copy.
Run against our own trees, all 100 crates rebuild and verify file by file.

## 3. Measure

```bash
run/measure.sh --tier smoke     # 12 crates, about 32 minutes
run/measure.sh --tier fast      # 58 crates, about 1.5 hours
run/measure.sh --tier full      # all 100 crates, 121.7 hours
```

Add `--scope alldeps` to instrument every crate in the dependency graph rather
than only the crate under study.

The tiers exist because the full corpus really does take that long. The median
crate finishes in 2.7 minutes, but ten take over four hours each and tokio alone
takes 16.3. The smoke tier spans unsafe shares from 0.03% to 10.15%, so it
exercises the range rather than a corner of it.

## 4. Build the tables from your measurement

```bash
tables/rq1_cpu_cycles.sh             # CPU cycles in unsafe code (RQ1, RQ6)
tables/rq2_heap.sh                   # heap memory reached by unsafe code (RQ2)
tables/rq3_unsafe_inst_frequency.sh  # how often executed instructions are unsafe (RQ3)
tables/rq4_inst_types.sh             # what kind of instruction the unsafe ones are (RQ4)
tables/rq5_unsafe_functions.sh       # how often functions holding unsafe code run (RQ5)
tables/figure_cycles_cdf.sh          # the RQ1/RQ6 distribution figure
tables/figure_heap_cdf.sh            # the RQ2 distribution figure
```

Each uses your most recent run under `results/`, builds the table from it,
prints it beside the same table in the paper, and then compares crate by crate
against our measurement. `--run DIR` picks a different run; `--scope alldeps`
says what the run instrumented.

Two things to expect.

**A partial run makes a partial table.** A table summarises the crates that were
measured, so the smoke tier gives you a 12-crate table and the paper's is over
100. Those are not the same statistic and the script says so. Only `--tier full`
covers the paper's population.

**Rerunning does not give identical numbers.** We measured the spread by running
the whole instruction counter twice over all 100 crates: of the 200
crate-and-variant pairs, 112 agreed to better than one part in ten thousand, 59
more to within 1%, 22 to within 10%, and 7 differed by more than 10%. Those 7
belong to four crates — petgraph, zopfli, portable-atomic and http — whose tests
drive randomly generated input; petgraph's unsafe instruction count moved 73%
between our own two runs. The comparison grades against that spread rather than
demanding equality, and a handful of crates outside 10% is the expected outcome.

## 5. If your numbers and the paper's disagree

```bash
run/check_shipped_data.sh
```

Rebuilds every table from the data we ship and checks it against the paper, in
about twenty seconds with no measurement and no network. This tells you whether
the data behind the paper was ever consistent with the paper, which is a
different question from whether your machine reproduces it.

## The benchmark suite

`benchmark_suite/` is the paper's second contribution and is not the study
corpus. It is 19 crates proposed for measuring the overhead that unsafe-Rust
defenses impose, run through `cargo bench` rather than `cargo test`. Sixteen
also appear in the corpus; `rayon`, `rebar` and `simd-json` do not. Several need
something other than a plain `cargo bench`, and `docs/benchmark_configs.md`
gives the command for each. See `benchmark_suite/README.md`.

## What is where

    compiler/            the instrumented compiler as source, with its commit hashes
    corpus/              the 100-crate lock, overlay, fetch script, generated workloads
    benchmark_suite/     the 19-crate benchmark suite, as source
    data/                our measurement data, and the tables as submitted
    docker/              the image definition
    run/                 fetch, measure, and the shipped-data check
    tables/              one script per table and figure
    tools/               aggregation, comparison, and the measurement harness
    unsafe_perf_source/  the instrumentation runtime library
    docs/                the crate dataset and the per-benchmark configurations

## Limits worth knowing before you start

Four crates drive random input and will not reproduce closely, as above.

The corpus crates are libraries, not applications, and most sit low in a
dependency tree. The paper says so, and it bounds what the results generalise to.

The workloads are each crate's own integration tests plus generated drivers that
raise public-API coverage from 76.4% to 90.4%. High API coverage is not the same
as representing how the API is used in production.

Three tables in the paper describe the benchmark suite rather than the corpus —
its dynamic characteristics, its composition, and its comparison against the
suites prior defenses use. Those were produced by hand rather than by a script,
so there is no `tables/` entry for them yet.
