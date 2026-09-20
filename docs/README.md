# Reference material

These files describe the dataset and the workloads. Nothing here needs to be
run; the scripts in `run/` and `tables/` do not read them.

`analyzed_crates.csv` — the 100 crates, with version, lines of code, the static
percentage of unsafe code, public-API coverage before and after workload
enhancement, and download counts. This is the source of the paper's
analyzed-crates appendix. Join it with anything else here on the `crate` column.

`benchmark_configs.md` — per-crate build commands, cargo feature selection and
static characteristics.

`workloads_descriptions/` — one line per crate saying what its workload does.

The generated workloads themselves are not here. They are in
`corpus/generated_tests/`, because the measurement harness plants them into each
crate, so they belong with the corpus rather than with the documentation.
