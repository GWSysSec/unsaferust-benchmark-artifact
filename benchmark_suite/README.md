# The benchmark suite

This is the paper's second contribution, and it is not the study corpus. The
corpus in `../corpus/` is the 100 crates the empirical study measures. This is
the 19-crate suite proposed for measuring the performance and memory overhead
that unsafe-Rust defenses impose, and the point of it is that the benchmarks
defenses currently use contain little or no unsafe code.

Sixteen of these crates also appear in the corpus. Three do not: `rayon`,
`rebar` and `simd-json`. They are run differently as well. The corpus is
measured through `cargo test --tests`, which drives each crate through its
public API the way a downstream user would. The suite is run through
`cargo bench`, because overhead is what it measures.

## Per-crate configuration

`CONFIGS.md` gives the command for each crate. Most are a plain `cargo bench`,
but several are not, and running those with the default command measures
something else:

    rayon         build rayon-demo with cargo build --release, then run only
                  `nbody bench --bodies 500`
    jni           cargo bench --features invocation
    parking_lot   build the benchmarks folder with cargo build --release, then
                  ./mutex 2 4 10 2 4 and ./rwlock 4 4 4 10 2 4
    ring          cargo bench --benches
    memchr        driven through rebar, which calls cargo underneath

## Running it

`env_presets/env/` holds one preset per instrumentation feature: `cpu.sh`,
`heap.sh`, `counter.sh` and `coverage.sh`. Source one, then run the crate's
command from `CONFIGS.md`:

```bash
source env_presets/env/cpu.sh
cd matrixmultiply && cargo bench
```

`run_pipeline.py` is the driver that automates this across the suite.

## Why these are shipped as source

Unlike the corpus crates, these trees carry no repository or release marker, so
there is nothing to reconstruct them from. The source here is the record. That
is also why the suite is not covered by `../corpus/corpus_lock.csv`.

## What the paper reports about the suite

Three tables: the dynamic characteristics of each benchmark, the composition of
the suite, and its comparison against the benchmarks four prior defenses use.
Those three were produced by hand rather than by a script, so unlike the corpus
tables there is no entry for them under `../tables/`.
