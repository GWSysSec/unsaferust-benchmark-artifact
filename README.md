# Dynamic Analysis of Unsafe Rust: Behaviors and Benchmarks — artifact

This artifact contains everything needed to check the paper's numbers: the
source of the instrumented compiler, the exact source of all 100 crates that
were measured, the measurement data the paper reports, and one script per table
and figure in the paper.

There are two different things you can check, and they take very different
amounts of time.

**Does the shipped data produce the paper's tables?** About twenty seconds, no
Docker and no network. This is exact: each table is rebuilt from the data and
compared character for character against the table in the paper.

**Does a fresh measurement reproduce the shipped data?** Hours to days,
depending on how much of the corpus you run. This one is approximate, because
the corpus is not deterministic. Numbers are below.

---

## 1. Check the tables (20 seconds)

Needs only Python 3.10+ with `matplotlib` and `numpy`.

```bash
run/check_all_tables.sh
```

Expected output: seven lines reading `match`, then
`All tables and figures reproduce from the shipped data.`

To see any single table, with its numbers and the comparison:

```bash
tables/rq1_cpu_cycles.sh            # CPU cycles spent in unsafe code (RQ1, RQ6)
tables/rq2_heap.sh                  # heap memory reached by unsafe code (RQ2)
tables/rq3_unsafe_inst_frequency.sh # how often executed instructions are unsafe (RQ3)
tables/rq4_inst_types.sh            # what kind of instruction the unsafe ones are (RQ4)
tables/rq5_unsafe_functions.sh      # how often functions holding unsafe code run (RQ5)
tables/figure_cycles_cdf.sh         # the RQ1/RQ6 distribution figure
tables/figure_heap_cdf.sh           # the RQ2 distribution figure
```

Add `--dataset alldeps` to any of them for the same measurement taken with every
crate in the dependency graph instrumented, rather than only the crate under
study. Those tables are new in the camera-ready, so there is nothing in the
submitted paper to compare them against; the script prints them instead.

## 2. Build the compiler (2 to 4 hours)

```bash
docker/build.sh
```

This builds LLVM 18 with assertions and then rustc 1.80, from the source in
`compiler/compiler-src.tar.zst`. Assertions are on because that is what the
measurements were taken with; turning them off would be a different compiler.
Budget about 40 GB of disk while it runs. Then:

```bash
docker/run.sh
```

which drops you in a shell at `/workspace/artifact` with the compiler linked as
the `stage1` toolchain.

## 3. Fetch the crate sources (a few minutes, needs network)

```bash
run/fetch_corpus.sh
```

Clones or downloads each of the 100 crates at the exact commit or released
version recorded in `corpus/corpus_lock.csv`, then applies
`corpus/corpus_overlay/`. About 1.7 GB.

To confirm the result is what was measured, point it at a reference copy:

```bash
run/fetch_corpus.sh --verify-against /path/to/reference
```

We ran that against our own trees: all 100 crates rebuild and verify file by
file.

## 4. Measure (32 minutes to 121 hours)

```bash
run/measure.sh --tier smoke     # 12 crates, about 32 minutes
run/measure.sh --tier fast      # 58 crates, about 1.5 hours
run/measure.sh --tier full      # all 100 crates, 121.7 hours
```

Add `--scope alldeps` to instrument every crate in the dependency graph instead
of only the crate under study.

The tiers exist because the full corpus really does take that long: the median
crate finishes in 2.7 minutes, but ten crates take more than four hours each and
tokio alone takes 16.3. The smoke tier spans unsafe shares from 0.03% to 10.15%,
so it exercises the range rather than a corner of it.

## 5. Compare your measurement against ours

```bash
tables/rq3_unsafe_inst_frequency.sh --from-run results/<timestamp>
```

This grades each crate rather than demanding equality, and it should: rerunning
this corpus does not give identical numbers. We measured the spread by running
the whole instruction counter twice over all 100 crates. Of the 200
crate-and-variant pairs, 112 agreed to better than one part in ten thousand, 59
more to within 1%, 22 to within 10%, and 7 differed by more than 10%. Those 7
belong to four crates — petgraph, zopfli, portable-atomic and http — whose tests
drive randomly generated input; petgraph's unsafe instruction count moved by 73%
between our own two runs.

A handful of crates outside 10% is the expected outcome, not a failure.

## What is where

    README.md              this file
    ARCHITECTURE.md        why the artifact is shaped this way
    compiler/              the instrumented compiler, as source, with its commit hashes
    corpus/                the crate lock, the overlay, the fetch script, the generated workloads
    data/                  the measurement data the paper reports, and the tables as submitted
    docker/                the image definition
    run/                   check the tables, fetch the corpus, measure
    tables/                one script per table and figure
    tools/                 aggregation, comparison, and the measurement harness
    unsafe_perf_source/    the instrumentation runtime library

## Limits worth knowing before you start

The corpus is not deterministic, as described above. Four crates drive random
input and will not reproduce closely.

The full measurement takes 121.7 hours sequentially. Nothing in this artifact
runs it for you in less; the tiers are the honest way to spend less time.

The crates are libraries, not applications, and most sit low in a dependency
tree. The paper says so, and it bounds what the results generalise to.

The workloads are each crate's own integration tests plus generated drivers that
raise public-API coverage from 76.4% to 90.4%. High API coverage is not the same
as representing how the API gets used in production.
