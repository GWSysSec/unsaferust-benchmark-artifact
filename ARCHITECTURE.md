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

The comparison is in shares, and it grades rather than demanding equality,
because the corpus is not deterministic. Over 200 crate-and-variant pairs
measured twice, 112 agreed to better than one part in ten thousand, 59 more to
within 1%, 22 to within 10%, and 7 differed by more than 10%; those 7 belong to
crates whose tests generate their input. A handful of crates outside 10% is the
expected outcome, not a failure.

`run/check_shipped_data.sh` rebuilds every table from the data we ship and
compares it character for character with the paper, in about twenty seconds and
with no measurement. It is there so that a disagreement can be located: it shows
whether the data behind the paper matches the paper, independently of what a
fresh run on a different machine produces.

One consequence worth stating plainly: a table summarises the crates that were
measured. A 12-crate run produces a 12-crate table, and the paper's is over 100.
Min, geomean, median and max over different populations are not the same
statistic, so the scripts say so rather than letting the two be compared
silently.

## Layout

    README.md              what to run, in order, with the time each step takes
    ARCHITECTURE.md        this file
    docker/                the image, and the scripts that fill the compiler volume
    compiler/              the compiler source (rustc 1.80 + LLVM 18 + passes)
    corpus/                the 100 crate sources, the crate list, the workloads
    benchmark_suite/       the 19-crate suite, which is a different thing (below)
    run/                   reproduce in one command, or fetch and measure apart
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

## The corpus is shipped, not fetched

`run/fetch_corpus.sh` unpacks `corpus/corpus-sources.tar.zst` — the 100 trees as
measured, 75 MB compressed and 419 MB on disk — in a couple of seconds and with
no network. Ninety-seven of the crates came from git repositories, and cloning
that many from GitHub in one sitting hits the unauthenticated rate limit, so
fetching them would make an evaluator create and paste an access token to get
the inputs. Shipping them also means a renamed, deleted or force-pushed
repository cannot break the artifact.

## The compiler is built into a volume, not into an image layer

`docker/build.sh` builds the image first, in about six minutes, and then fills
a Docker volume with the compiler: by default by unpacking the toolchain we
built, which takes about a minute, and with `--from-source` by running
`docker/build_compiler.sh` inside the image with that volume mounted where the
build output goes. On our 32-core machine the compiler step took 876 seconds and
produced 7.0 GB. Making it resumable still matters on slower machines: an
interrupted build continues when the script is run again, instead of starting
over inside a layer that has to be rebuilt from the beginning. Keeping it out of
the image also lets the image be rebuilt in seconds when a script changes, and
leaves the build output somewhere the evaluator can reclaim with a single
`docker volume rm`.

One trap, recorded because it cost a day. Ubuntu 22.04's CMake is 3.22.1, and it
cannot read the dependency file LLVM 18's TableGen writes for `GenVT.inc`: that
file names an output and lists no dependencies, which 3.22 treats as a parse
error. The build stops with a bare `FAILED` line and nothing underneath it, at a
step number that moves between runs, which looks exactly like the kernel killing
a process. The image installs CMake 3.31.6 from PyPI instead.
