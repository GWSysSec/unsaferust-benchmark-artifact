# Dynamic Analysis of Unsafe Rust: Behaviors and Benchmarks — artifact

This artifact reruns the paper's measurements and compares what you measure
against what the paper reports. It contains the instrumented compiler as both
source and a prebuilt toolchain, the source of all 100 crates the study
measures, the 19-crate benchmark suite the paper proposes, and one script per
table and figure in the paper.

The intended path is that you measure the corpus yourself and build the tables
from your own numbers. Our measurement data is here too, but as the thing to
compare against, not as the input. Nothing needs a GitHub account, an access
token, or any credential.

**Sizes.** The artifact directory is 651 MB. Building the Docker image produces
a 4.88 GB image. The compiler lands in a Docker volume: 343 MB if you use the
prebuilt toolchain, 7.0 GB if you build it from source. Unpacking the corpus
adds 419 MB, and measuring writes into `results/`, which grows with the number
of crates you measure.

---

## Quick start

Five commands. The times are what they took on a 32-core machine with 30 GB of
memory; a smaller machine is proportionally slower.

```bash
docker/build.sh                  # ~7 min: image, then our prebuilt compiler
docker/run.sh                    # opens a shell at /workspace/artifact
# the remaining commands run inside that shell:
run/fetch_corpus.sh              # ~2 s: unpack the 100 crate source trees
run/measure.sh --tier smoke      # ~20 min: measure 12 crates
tables/rq4_inst_types.sh         # ~1 min: build Table 5 from what you measured
```

Everything after `docker/run.sh` runs inside the container. `results/` and
`corpus/sources/` are mounted from the host, so what you fetch and what you
measure survive after the shell exits, and they are handed back to your user
account rather than left owned by root.

### If you are an automated agent reviewing this artifact

Run exactly this, from the artifact directory, and check the quoted strings.
Do not run `--tier full`: it takes 121.7 hours.

```bash
docker/build.sh                                     # expect: "rustc 1.80.0-dev"
docker/run.sh bash -c 'run/fetch_corpus.sh'         # expect: "crates: 100"
docker/run.sh bash -c 'run/check_shipped_data.sh'   # expect: "The shipped data reproduces every table in the paper."
docker/run.sh bash -c 'run/measure.sh --tier smoke' # expect: "rebench complete: 12 crate(s)"
docker/run.sh bash -c 'for s in tables/*.sh; do bash "$s"; done'
```

Each `tables/*.sh` prints three things in order: the table built from the
measurement, the same table as submitted, and a per-crate comparison against our
data. A non-zero exit means the script failed; a difference in the comparison
does not. Read **What varies between runs** below before judging any difference,
and note that a 12-crate table and the paper's 100-crate table are different
statistics, which the scripts say each time.

Two directories hold the evidence afterwards, both on the host:
`results/<timestamp>/` is the raw measurement, and `results/<timestamp>/tables/`
holds every table and figure that was built from it.

---

## 1. The compiler: prebuilt by default, from source on request

```bash
docker/build.sh                 # unpack the toolchain we built (about 1 minute)
docker/build.sh --from-source   # build the same thing from source
```

Both start by building the Docker image, which takes about six minutes: apt
packages, a CMake from PyPI, the compiler source, a current cargo, and the
artifact scripts.

The default then unpacks `compiler/stage1-toolchain.tar.zst`, which is the
stage-1 toolchain we built from the source in this artifact — rustc, the
libraries it links, and the standard library compiled against it. It is 87 MB
compressed and 343 MB unpacked. `tools/package_toolchain.sh` is what produced
it, and it tests the compiler before packaging: the right version, a program
using unsafe code, and the instrumentation flags the measurements use.

`--from-source` builds LLVM 18 with assertions and then rustc 1.80 from
`compiler/compiler-src.tar.zst` instead. Ours took **876 seconds and 7.0 GB on
32 cores**. Assertions are on because that is what the measurements were taken
with; turning them off would be a different compiler. The build goes into a
Docker volume rather than an image layer, so an interrupted build continues from
where it stopped when you run the script again, and
`docker volume rm unsaferust-compiler` reclaims the space when you are done.
Parallelism is capped by the memory the container sees, roughly 2 GB per compile
job and 8 GB per link job; override it with
`COMPILE_JOBS=8 LINK_JOBS=2 docker/build.sh --from-source`.

`compiler/COMMIT` records the commit the source was taken from, for the rustc
tree, its LLVM submodule and all ten other submodules.

## 2. The crate sources are packaged, so you need no GitHub credentials

```bash
run/fetch_corpus.sh
```

This unpacks `corpus/corpus-sources.tar.zst` into `corpus/sources/`: the 100
trees exactly as measured, 75 MB compressed and 419 MB unpacked, in about two
seconds with no network access at all.

They are shipped rather than fetched on purpose. Ninety-seven of the 100 crates
came from git repositories, and cloning a hundred repositories from GitHub in
one sitting runs into unauthenticated rate limits, which means an evaluator
would have to create and paste a personal access token to get the inputs. The
tarball removes that step. It also removes the day a repository is renamed,
deleted or force-pushed from the list of things that can break this artifact.

The sources stay read-only during measurement — each crate is copied into
`results/<run>/` and built there — so budget disk for `results/`, not for
`corpus/sources/`.

Rebuilding the corpus from its original sources is still available, and answers
a different question: can the corpus be reconstructed from what is published?

```bash
run/fetch_corpus.sh --from-upstream                       # needs network
run/fetch_corpus.sh --from-upstream --verify corpus/sources  # and compare
```

That reads `corpus/corpus_lock.csv`, which records for each crate the exact
commit or released version it was measured on, clones or downloads each one, and
applies `corpus/corpus_overlay/` — every difference between the upstream tree
and the tree we measured, including the 113 `Cargo.lock` files that pin the
dependency versions. Without those a rebuild re-resolves every dependency to
whatever is newest today and measures a different program. Run against our own
trees, all 100 crates rebuild and verify file by file.

**Measuring does need network access**, even though getting the sources does
not: each crate's own dependencies are downloaded from crates.io at the versions
its `Cargo.lock` pins, and the instrumentation runtime in `unsafe_perf_source/`
is built the same way.

The lockfiles are respected but not enforced. If a build fails because a
transitive dependency needs a newer edition than this 1.80 toolchain
understands, the harness deletes that crate's `Cargo.lock`, lets cargo
re-resolve to older compatible versions, and retries. It prints `retrying after
lockfile regen` when it does. In our smoke run this happened to one crate,
borsh-rs, whose unsafe share came out identical to ours anyway.

## 3. Measure

```bash
run/measure.sh --tier smoke     # 12 crates, about 20 minutes
run/measure.sh --tier fast      # 58 crates, about 1.5 hours
run/measure.sh --tier full      # all 100 crates, 121.7 hours
run/measure.sh --crate tokio,bytes   # a named list instead of a tier
```

The tiers exist because the full corpus really does take that long. The median
crate finishes in 2.7 minutes, but ten take over four hours each and tokio alone
takes 16.3. The smoke tier spans unsafe shares from 0.03% to 10.15%, so it
exercises the range rather than a corner of it.

`--scope alldeps` instruments every crate in the dependency graph instead of
only the crate under study. The two scopes are not comparable, so the scope
travels with the measurement and the table scripts read it back out.

## 4. Which script produces which table or figure

Each script builds its table from your most recent run under `results/`, prints
it beside the same table as submitted, and then compares crate by crate against
our measurement. `--run DIR` picks a different run. `--our-data` builds from the
data we ship instead of yours.

| Run this | Paper | What it answers |
|---|---|---|
| `tables/rq1_cpu_cycles.sh` | Table `table:cpu_cycle` (§RQ1) | share of CPU cycles spent in unsafe code |
| `tables/figure_cycles_cdf.sh` | Figure `fig:cpu_cycle` (§RQ1) | distribution of that share across crates |
| `tables/rq2_heap.sh` | Table `table:heap` (§RQ2) | heap memory reached by unsafe code, by allocation size |
| `tables/figure_heap_cdf.sh` | Figure `fig:heap_standard_cdf` (§RQ2) | distribution of the unsafe heap share |
| `tables/rq4_inst_types.sh` | Table `table:inst` (§RQ3, §RQ4) | how often executed instructions are unsafe, and of what type |
| `tables/rq5_unsafe_functions.sh` | Table `table:unsafe_function` (§RQ5) | how often functions holding unsafe code run |
| `tables/rq3_unsafe_inst_frequency.sh` | not in the submitted paper | the RQ3 summary on its own; the submitted paper folds RQ3 into `table:inst` |

The paper's headline RQ3 sentence, "on average, 1.8% of executed instructions
are unsafe", is the Geomean cell of the **Total** row under **Dynamic counts w/o
std libs** in `table:inst`. `tables/rq4_inst_types.sh` is the script that builds
that table.

Three further tables — `bench_dyn`, `benchmarks` and `bench_comparisons` —
describe the 19-crate benchmark suite rather than the corpus. They were produced
by hand, so there is no script for them.

Everything a script builds is copied into `results/<run>/tables/`, which is on
the host, so it is still there after the container exits.

## 5. What varies between runs, and why

Rerunning this corpus does not give identical numbers, and it is not supposed
to. Three separate causes, in the order they matter.

**Randomised workloads.** Five crates generate their test input: petgraph,
zopfli, portable-atomic, http and slotmap, the last through `quickcheck`. Each
run executes a different amount of work. Running our own instruction counter
twice over all 100 crates, 112 of the 200 crate-and-variant pairs agreed to
better than one part in ten thousand, 59 more to within 1%, 22 to within 10%,
and 7 differed by more than 10%; those 7 belong to four of those crates.
petgraph's unsafe instruction count moved 73% between our own two runs.

**The machine.** CPU cycle counts depend on the processor, the memory system and
what else is running, so the RQ1 comparison uses the share of cycles rather than
the counts, and still expects more spread than the instruction counters. Tests
that time out, and tests whose work depends on timing or on thread scheduling,
also move with load. Measuring on a busy machine widens all of this. Nothing
here needs `sudo` or performance counters — the instrumentation counts inside
the program — but a quiet machine still gives tighter numbers.

**Which binaries ran.** A crate's share is dominated by its largest test binary,
so one binary that did a different amount of work moves the whole crate even
when every other binary agrees. In ron and deranged most test binaries agree to
three decimal places while a few do not; ron's `129_indexmap` binary did
essentially nothing in our run and real work in a fresh one.

Running the smoke tier here, against our published data, the median difference
was 0.21% for RQ3 and 0.00% for RQ4 and RQ5, with 12, 16 and 14 of the 24
crate-and-variant pairs agreeing to one part in ten thousand. RQ1 was 6.83% and
RQ2 8.14%.

## 6. Two things to know before quoting a number

**Everything is compared as a share, never as a raw count.** Our published
instruction-counter run is not one measurement pass: 1,082 of its 1,271 test
binaries were measured twice and three were measured four times, and the
aggregator sums every stat file it finds, so our absolute totals are 1.596 times
what one pass executes. Your run measures each binary once. Raw counts would put
libsecp256k1 a factor of two away from us while every share agreed to three
decimal places. `tools/analysis/audit_duplicate_stats.py` measures this, and
gates itself by first reproducing the published aggregation exactly. Shares are
also what the paper reports, so that is what the comparison uses. A difference
below a tenth of a percentage point counts as agreement, because a share near
zero otherwise produces enormous relative differences out of nothing.

**One per-crate heap figure does not hold up.** deranged's published heap share
is 0.0258% for both `with_native` and `without_native` — the same number — while
a fresh run reads 22.3% and 0.028%. Eight of the 100 crates report the same
unsafe heap bytes in both variants and three of those eight are nonzero. Check
those before quoting a per-crate heap figure.

## 7. Checking our data against the paper, without measuring

```bash
run/check_shipped_data.sh
```

Rebuilds every table from the data we ship and compares it character for
character with the table as submitted, in about twenty seconds with no
measurement and no network. It answers whether the data behind the paper is
consistent with the paper, which is a different question from whether your
machine reproduces it. All five tables pass.

## The benchmark suite

`benchmark_suite/` is the paper's second contribution and is not the study
corpus. It is 19 crates proposed for measuring what unsafe-Rust defenses cost,
run through `cargo bench` rather than `cargo test`. Sixteen also appear in the
corpus; `rayon`, `rebar` and `simd-json` do not. Several need something other
than a plain `cargo bench`, and `docs/benchmark_configs.md` gives the command
for each. See `benchmark_suite/README.md`.

## What is where

    compiler/            the compiler as source (213 MB) and prebuilt (87 MB),
                         with the commit hashes it was built from
    corpus/              the 100 crate sources (75 MB), the lock and overlay
                         that rebuild them from upstream, and the generated
                         workloads
    benchmark_suite/     the 19-crate benchmark suite, as source
    data/                our measurement data, and the tables as submitted
    docker/              the image, and the scripts that fill the compiler volume
    run/                 get the corpus, measure in three sizes, check our data
    tables/              one script per table and figure in the paper
    tools/               aggregation, comparison, packaging, and the harness
    unsafe_perf_source/  the instrumentation runtime library
    docs/                the crate dataset and the per-benchmark configurations

`ARCHITECTURE.md` explains why the artifact is put together this way.

## Limits worth knowing before you start

The corpus crates are libraries, not applications, and most sit low in a
dependency tree. The paper says so, and it bounds what the results generalise
to.

The workloads are each crate's own integration tests plus generated drivers that
raise public-API coverage from 76.4% to 90.4%. High API coverage is not the same
as representing how the API is used in production.

A table summarises the crates that were measured. A 12-crate run produces a
12-crate table and the paper's is over 100, so their minimum, geometric mean,
median and maximum are not the same statistic. Only `--tier full` covers the
paper's population, and it takes 121.7 hours. The per-crate comparison each
script prints does not have this problem: it compares your crates against the
same crates in our data.
