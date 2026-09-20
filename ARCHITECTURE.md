# How this artifact is put together, and why

An evaluator has hours, not weeks. The full measurement takes 121.7 hours of
wall time, so an artifact that only offers "run everything" is an artifact
nobody can check. This one is built so that every claim in the paper can be
checked in minutes, and the expensive re-measurement is an optional deepening
rather than the price of entry.

## The evaluator measures; our data is what they compare against

Each table in the paper has one script here, and by default that script builds
the table out of the evaluator's own measurement. It then puts three things next
to each other: the table from their numbers, the same table from the paper, and
a crate-by-crate comparison against ours. Our measurement data ships too, but it
is the reference, not the input.

The comparison grades rather than demanding equality, because the corpus is not
deterministic. Over 200 crate-and-variant pairs measured twice, 112 agreed to
better than one part in ten thousand, 59 more to within 1%, 22 to within 10%,
and 7 differed by more than 10%; those 7 belong to four crates whose tests drive
randomly generated input. A handful of crates outside 10% is the expected
outcome, not a failure.

There is a second, much weaker check: `run/check_shipped_data.sh` rebuilds every
table from the data we ship and compares it character for character with the
paper, in about twenty seconds. It answers a different question — whether the
data behind the paper was ever consistent with the paper — and it exists so that
when a fresh measurement and the paper disagree, the evaluator can tell which
side moved. It is not the artifact's main path.

One consequence worth stating plainly: a table summarises the crates that were
measured. A 12-crate run produces a 12-crate table, and the paper's is over 100.
Min, geomean, median and max over different populations are not the same
statistic, so the scripts say so rather than letting the two be compared
silently.

## Layout

    README.md              what to run, in order, with the time each step takes
    ARCHITECTURE.md        this file
    docker/                the image, and the script that builds the compiler
    compiler/              the compiler source (rustc 1.80 + LLVM 18 + passes)
    corpus/                how to obtain the 100 crates at the exact versions used
    benchmark_suite/       the 19-crate suite, which is a different thing (below)
    run/                   get the corpus, measure in three sizes, check our data
    tables/                one script per table and figure in the paper
    data/                  the measurement data the paper reports
    tools/                 aggregation and comparison code

## The corpus and the benchmark suite are different things

The corpus is the 100 crates the empirical study measures, through
`cargo test --tests`. The benchmark suite is 19 crates proposed for measuring
what unsafe-Rust defenses cost, through `cargo bench`. Sixteen crates are in
both; `rayon`, `rebar` and `simd-json` are only in the suite. They are kept in
separate directories because mixing them would make it look as though the suite
were a subset of the corpus, and because their per-crate run commands differ.

The suite is shipped as source rather than fetched from a lock: unlike the
corpus crates, those trees carry no repository or release marker, so there is
nothing to reconstruct them from.

## Two ways to get the corpus, answering two questions

`run/fetch_corpus.sh` unpacks `corpus/corpus-sources.tar.zst` — the 100 trees as
measured, 75 MB compressed and 419 MB on disk — in a couple of seconds and with
no network. That is the default because it is what the measurement steps need,
and because it still works on the day a repository disappears or is force-pushed.

`run/fetch_corpus.sh --from-upstream` answers the other question: can the corpus
be rebuilt from its published sources? It reads `corpus/corpus_lock.csv`, which
records for each crate the exact commit or released version it was measured on,
clones or downloads each one, and then applies `corpus/corpus_overlay/`, 2.3 MB
holding every difference between that upstream tree and the tree we measured.
With `--verify DIR` it compares the result against an unpacked copy, file by
file; against our own trees all 100 crates rebuild and verify.

The overlay carries 113 `Cargo.lock` files, and that is the part that matters.
Upstream commits a lockfile for only 14 of these crates; for 81 more the
lockfile was generated on our machine at measurement time from whatever
crates.io served that day. Without them a rebuild re-resolves every dependency
to whatever is newest today and measures a different program.

## The compiler is built into a volume, not into an image layer

`docker/build.sh` builds the image first, in a few minutes, and then runs
`docker/build_compiler.sh` inside it with a Docker volume mounted where the
build output goes. The compiler build is hours of work and about 40 GB, so
making it resumable matters: an interrupted build continues when the script is
run again, instead of starting over inside a layer that has to be rebuilt from
the beginning. It also keeps the image small enough to rebuild when a script
changes, and leaves the 40 GB somewhere the evaluator can reclaim with a single
`docker volume rm`.

One trap, recorded because it cost a day. Ubuntu 22.04's CMake is 3.22.1, and it
cannot read the dependency file LLVM 18's TableGen writes for `GenVT.inc`: that
file names an output and lists no dependencies, which 3.22 treats as a parse
error. The build stops with a bare `FAILED` line and nothing underneath it, at a
step number that moves between runs, which looks exactly like the kernel killing
a process. The image installs CMake 3.31.6 from PyPI instead.
