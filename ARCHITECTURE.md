# How this artifact is put together, and why

An evaluator has hours, not weeks. The full measurement takes 121.7 hours of
wall time, so an artifact that only offers "run everything" is an artifact
nobody can check. This one is built so that every claim in the paper can be
checked in minutes, and the expensive re-measurement is an optional deepening
rather than the price of entry.

## The two things an evaluator can check, and the difference between them

**Does the shipped data produce the paper's numbers?** Instant, exact, and the
check most worth running. Each table in the paper has one script here. The
script rebuilds that table from the measurement data we ship and compares it,
character for character, against the table in the submitted paper. Nothing is
measured, nothing is random: it either matches or it does not.

**Does a fresh measurement reproduce the shipped data?** Slow, and approximate
by nature. The same scripts take `--from-run`, pointing at a measurement the
evaluator made themselves, and report how far each crate lands from ours. This
is approximate because the corpus is not deterministic: over 200
crate-and-variant pairs measured twice, 112 agreed to better than one part in
ten thousand, 59 more to within 1%, 22 to within 10%, and 7 differed by more
than 10%. Those 7 belong to four crates whose tests drive randomly generated
input. The scripts grade against that measured spread instead of demanding
equality.

Keeping these apart matters. The first says our data and our paper agree. The
second says our data and the world agree. A reader who conflates them will read
ordinary workload variance as a failure to reproduce.

## Layout

    README.md              what to run, in order, with the time each step takes
    ARCHITECTURE.md        this file
    docker/                the image: builds the compiler from source
    compiler/              the compiler source (rustc 1.80 + LLVM 18 + passes)
    corpus/                how to obtain the 100 crates at the exact versions used
    run/                   scripts that measure, in three sizes
    tables/                one script per table and figure in the paper
    data/                  the measurement data the paper reports
    tools/                 aggregation and comparison code

## Why the corpus is fetched rather than shipped whole

The 100 crate source trees are 436 MB, or 76 MB compressed. What is shipped
instead is `corpus/corpus_lock.csv`, which records for each crate the exact
commit or released version it was measured on, and `corpus/corpus_overlay/`,
2.3 MB holding every difference between that upstream tree and the tree we
measured. `corpus/fetch_corpus.py` rebuilds all 100 trees from those two, and
verifies file by file against a reference copy when given one.

The overlay carries 113 `Cargo.lock` files, and that is the part that matters.
Upstream commits a lockfile for only 14 of these crates; for 81 more the
lockfile was generated on our machine at measurement time from whatever
crates.io served that day. Without them a rebuild re-resolves every dependency
to whatever is newest today and measures a different program.

A source tarball is included as well, for evaluation without network access and
against the day a repository disappears or is force-pushed.
