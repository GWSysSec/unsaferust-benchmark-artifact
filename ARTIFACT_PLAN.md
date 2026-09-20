# What an evaluator can do with this artifact, and what still has to be added

Written 2026-09-19.

An evaluator should be able to take this artifact, obtain the compiler and the
exact crate sources we measured, run the measurements, and end up with numbers
they can put beside ours. Today they cannot, for one reason: the artifact holds
19 crate source trees and the paper reports 100. Everything else needed is
either here already or was built this week and is ready to move in. This
document says what is here, what is missing, what the new pieces do, and the
four decisions that are yours to make.

The data itself checks out. Every one of the 100 crates in
`dynamic_analysis_results/` matches the paper's committed data exactly, for
total and unsafe instruction counts in both variants. Five crates appear under
their crates.io package name here and under their repository name in the paper
(`borsh`/`borsh-rs`, `cc`/`cc-rs`, `rmp-serde`/`msgpack-rust`,
`openssl`/`rust-openssl`, `utf-8`/`utf8parse`); the numbers are identical. The
562 generated coverage workloads in `generated_tests/` are also equivalent to
the ones the measurements ran: 204 of them differ textually, and every
difference is a stripped comment.

## What is here

| Piece | State |
|---|---|
| `toolchain/` | Prebuilt `rustc 1.80.0-dev` with the instrumentation passes, 674 MB, in Git LFS. Self-contained, needs only glibc 2.34. |
| `unsafe_perf_prebuilt/` | The runtime library, built per measurement feature, 246 MB. |
| `unsafe_perf_source/` | Its source, so it can be rebuilt. |
| `generated_tests/` | The 562 generated coverage workloads, all 100 crates' worth. |
| `dynamic_analysis_results/` | Our per-crate results for all 100 crates, all five research questions, both variants. |
| `analyzed_crates.csv` | The per-crate dataset: version, lines of code, static unsafe share, API coverage. |
| `benchmarks/` | 19 crate source trees. This is the gap. |
| `run_pipeline.py`, Docker | A driver, but not the harness that produced the published data. |

## What is missing

**Eighty-one of the hundred crate source trees.** Without them an evaluator can
measure 19 crates and compare 19 rows.

**The harness that produced the data.** `run_pipeline.py` here is a different
driver from `rusttest-gen/scripts/rebench.py`, which is what actually ran the
published measurements: it copies each crate, plants the generated workloads,
builds each measurement feature under both native-library settings, runs every
test binary, and writes the per-binary counter dumps the aggregation reads.

**A statement of how long this takes.** Measured on the 2026-09 run: 121.7
hours of sequential wall time for the whole corpus. The median crate takes 2.7
minutes and ten crates take more than four hours each, tokio alone 16.3. An
evaluator told to "run the corpus" without that warning will conclude the
artifact is broken.

## What was built this week and is ready to move in

All three live in the two working repositories and can be copied here.

**`corpus_lock.csv` and `corpus_overlay/`** (in `rusttest-gen`). The lock records,
for each of the 100 crates, where its source came from and the exact commit or
released version: 97 git clones and 3 crates.io releases. The overlay, 2.3 MB in
total, records every difference between that upstream tree and the tree we
measured, including the 113 `Cargo.lock` files that pin the dependency
versions.

**`scripts/fetch_corpus.py`** (in `rusttest-gen`) rebuilds the corpus from the
lock. Run with `--verify-against` a reference copy, all 100 crates rebuild and
verify file by file against the trees the measurements ran on. So 935 KB plus a
network fetch reconstructs what would otherwise be 986 MB of shipped source.

**`export_dataset.py` and `verify_reproduction.py`** (in the paper repository).
The first turns a measurement run into the per-question data files and CSVs. The
second compares a fresh run against ours and grades each crate.

That grading matters, because rerunning this corpus does not give identical
numbers, and we measured by how much. Over the 200 crate-and-variant pairs of a
repeated instruction-counter pass, 112 agreed to better than one part in ten
thousand, 59 more to within 1%, 22 to within 10%, and 7 differed by more than
10%. Those 7 belong to four crates whose tests drive randomly generated input:
petgraph, zopfli, portable-atomic and http. petgraph's unsafe instruction count
moved by 73% between our own two runs. An evaluator must be told this, or a
correct reproduction will look like a failed one.

## The workflow to offer

1. **Get the compiler.** Use `toolchain/`, the prebuilt one already here.
   Building it from source is a rustc plus LLVM 18 build: hours of compute and
   tens of gigabytes. Offer it as a documented alternative, not the default path.

2. **Get the crates.** `python3 fetch_corpus.py --out corpus` clones or
   downloads each of the 100 crates at its recorded commit or version and
   applies the overlay.

3. **Measure.** `rebench.py` over the corpus. Three tiers, with the time each
   takes stated up front:
   - a twelve-crate smoke test, about 32 minutes, spanning unsafe shares from
     0.03% to 10.15%: borsh-rs, ron, siphasher, libsecp256k1, nu-ansi-term,
     scroll, deranged, git2, cc-rs, slotmap, curl, dashmap;
   - the 58 crates that each finish in under five minutes, about 1.5 hours
     together;
   - the full corpus, 121.7 hours, for anyone who wants it.

4. **Aggregate.** `export_dataset.py` writes the per-question data files from
   the fresh run, in the same schema as the ones we ship.

5. **Compare.** `verify_reproduction.py <run-dir>` grades the fresh run against
   ours, question by question.

## Four decisions

**1. Ship the crate sources, or fetch them?** Measured both ways. Shipping is
436 MB of source, 76 MB compressed with `zstd -19`, on top of the 674 MB
toolchain and 246 MB prebuilt runtime already here. Fetching is a 2.3 MB
overlay plus the clones and downloads, and rebuilds all 100 trees file for
file.

Fetching pins what gets compiled, not just what gets checked out. The overlay
carries 113 `Cargo.lock` files, which fix the version of every dependency that
was compiled and measured. That matters more than it sounds: upstream commits a
lockfile for only 14 of the corpus crates, so for 81 more the lockfile was
generated here at measurement time from whatever crates.io served then. Without
those files a rebuild re-resolves every dependency to whatever is newest on the
day and measures a different program.

One caveat on the lockfiles. The harness does not pass `cargo --locked`, so
cargo may update a lockfile rather than fail, and on a resolution failure the
harness deletes the lockfile and retries with an older-versions fallback, which
a few crates genuinely need. Shipping the lockfiles therefore makes a rebuild
start from the same input the measured run started from, which is the right
guarantee, but it is not an enforced one. Adding `--locked` to the artifact's
documented flow would turn silent drift into a visible error.

What fetching cannot pin is availability. The 97 git crates depend on their
repositories still existing and still holding the recorded commit; a deleted
repository or a force-pushed history breaks the rebuild years from now. The 3
crates.io releases are immutable and survive even a yank. So the safe answer is
both: make fetching the documented path, and include the 76 MB source tarball
as the offline fallback.

**2. Which dataset does the artifact reproduce?** The published run, which
instruments only the primary package, or also the dependency-include run. This
one has a consequence: the dependency-include run needs three compiler commits
that are not pushed anywhere (see below), so choosing it means publishing them.

A separate question turned out to have a clear answer: the artifact cannot keep
shipping the compiler binary it has. Measured across eight crates, all three
features and both variants, the four commits between that binary and the current
compiler change instruction counts by a median of 0.03% and heap totals by a
median of 0.56%, all inside the corpus's own run-to-run variation. But the
shipped binary cannot build `colored` under the heap tracker at all: it fails to
link with the undefined-symbol error that commit `6277bbba7148` repairs, with
only the primary package instrumented. An evaluator using it gets heap data for
99 of the 100 crates. `rust-dyn-bench-paper/COMPILER_AB.md` has the numbers.

**3. Keep `benchmarks/`?** Its 19 trees are a subset of the 100 the fetch script
rebuilds, and the directory also carries 7.1 GB of untracked build output on
this machine. Replacing it with the fetch flow removes a second, inconsistent
source of crate sources.

**4. Which driver?** `run_pipeline.py` here, or `rebench.py`, which is what
produced the data. Shipping both invites an evaluator to run the one that
cannot reproduce the numbers.

## The unpushed compiler changes

Three commits in `unsafe-rust-benchmark` on branch `feat/instrument-all-deps`,
and two in its LLVM submodule on the branch of the same name. None is pushed.

- `e9f8ec45b819` (LLVM) adds `UNSAFE_INSTRUMENT_ALL_PACKAGES`, which lifts the
  primary-package-only restriction. Default off, so an unflagged build behaves
  exactly as before.
- `07b4ab94a0d1` honours the same variable in the rustc-side stdlib API tracker.
- `d06361cdbf4b` fixes a compiler crash. The MIR pass hardcoded a basic block as
  non-cleanup, producing invalid MIR whenever the standard-library call it marked
  sat on an unwind path. Any crate whose dependency graph contains `bytes` or
  `crossbeam-epoch` can hit it.
- `6277bbba7148` (LLVM) fixes a link failure in the heap tracker, which created a
  reference to a static whose definition had already been dropped. It affected
  `colored` and `tokio`.

The last two are bug fixes that stand on their own merit, independent of whether
the dependency-include data is published.

## One thing to fix before packaging

`rusttest-gen/config.yaml` carries a live API key. It is not tracked by git, and
it must not be copied into the artifact. Check for it explicitly when packaging.
