# Runtime statistics

Per-crate dynamic-analysis measurements for all 100 analyzed crates, collected
by running each crate's integration tests (upstream tests + the LLM-enhanced
coverage workloads in `../generated_tests/`) under the instrumented toolchain.

One small JSON file per crate, grouped by research question and by measurement
variant:

```
runtime_stats/
  rq1_cpu_cycles/                     CPU cycles spent in unsafe vs. total code
  rq2_heap_memory/                    heap allocation behaviour + unsafe heap share
  rq3_unsafe_instruction_frequency/   how much of executed work is unsafe
  rq4_unsafe_instruction_types/       breakdown of unsafe instructions by kind
  rq5_unsafe_functions/               unsafe function definition/execution counts
      <rq>/with_native/<crate>.json
      <rq>/without_native/<crate>.json
```

Each leaf file is named by the published crate name and is self-describing:
it carries `crate`, `variant`, `research_question`, and the metric fields.
Crate names join 1:1 with the `crate` column of `../analyzed_crates.csv`
(static metadata, LOC, static-unsafe %, and pre/post API coverage).

## Variants: `with_native` vs `without_native`

- **`without_native`** counts unsafe code only inside the crate under study.
- **`with_native`** additionally counts unsafe code reached through the
  standard library (`core`/`std`/`alloc`) and other native dependencies.

Comparing the two isolates the crate's own unsafe footprint from the unsafe it
inherits by calling into the platform.

## Field glossary

**rq1_cpu_cycles**
- `total_cycles`, `external_cycles`, `internal_cycles`, `unsafe_cycles` — rdtsc
  cycle counts; `unsafe_percentage` — unsafe share of executed cycles.
  (Cycle *totals* carry instrumentation overhead and are not meant for
  cross-variant subtraction; `unsafe_percentage` is the comparable figure.)

**rq2_heap_memory**
- `total_heap_usage` / `unsafe_heap_memory` (bytes), `total_heap_allocations` /
  `unsafe_heap_objects` (counts), `total_heap_reallocations`,
  `total_heap_deallocations`, `unsafe_memory_instructions`, `unsafe_load`,
  `unsafe_store`, `size_histogram` / `unsafe_size_histogram` (power-of-two
  allocation-size buckets), `unsafe_heap_percentage`.

**rq3_unsafe_instruction_frequency**
- `total_instructions`, `unsafe_instructions`, `unsafe_inst_pct`,
  `distinctive_pct`, `exec_w_unsafe_pct`, `accumulated_pct` — alternative
  denominators for "how unsafe is the executed work."

**rq4_unsafe_instruction_types**
- `unsafe_loads`, `unsafe_stores`, `unsafe_calls_{direct,indirect,intrinsic}`,
  `unsafe_casts`, `unsafe_geps`, `unsafe_atomics`, `unsafe_others`
  (+ `total_instructions`, `unsafe_instructions` for context).

**rq5_unsafe_functions**
- runtime: `total_functions_defined`, `unsafe_functions_defined`,
  `total_functions_executed`, `unsafe_functions_executed`,
  `unsafe_functions_with_executed_insts`, `total_function_calls`,
  `unsafe_function_calls`, `unsafe_calls_total`.
- static (variant-invariant): `static_rs_files`, `static_total_fns`,
  `static_unsafe_fns`, `static_unsafe_fn_pct`.
