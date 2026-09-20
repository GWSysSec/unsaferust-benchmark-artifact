"""
prompts.py — system and user prompts for the pipeline

brain (opus): analyze test scenarios, generate integration tests
fixer (opus): fix compilation and runtime errors
"""

PLATFORM_CONSTRAINTS = """\

## Platform Constraints
- Target: Linux (Ubuntu 22.04), x86_64, 32GB RAM
- Do NOT spawn more than 4 threads in a single test function
- Do NOT allocate more than 512MB in a single test
- Do NOT use `std::thread::sleep` with durations > 100ms
- Always use `try_lock` or timed lock acquisitions — never bare `.lock().unwrap()` \
on a Mutex that another thread might hold
- No infinite loops without exit conditions
- No blocking network I/O without timeouts
- Tests MUST complete within 60 seconds
"""

SYSTEM_EVALUATE = """\
You are an expert compiler engineer and Rust static analyzer evaluating the \
quality of a crate's test suite for use as instrumentation benchmarks for \
unsafe Rust code analysis.

## Scoring Axes (rate each 0-100)
1. **comprehensiveness**: fraction of public API surface exercised by tests
2. **edge_coverage**: boundary conditions, empty inputs, error paths, capacity overflow
3. **realism**: realistic data patterns resembling production usage
4. **regression_sensitivity**: would the suite catch silent no-op regressions?
5. **unsafe_boundary_coverage**: tests exercising unsafe blocks, raw pointers, FFI, transmute

## Output Format
Reply ONLY with valid JSON:
{
  "score": <integer 0-100>,
  "axes": {
    "comprehensiveness": <int>,
    "edge_coverage": <int>,
    "realism": <int>,
    "regression_sensitivity": <int>,
    "unsafe_boundary_coverage": <int>
  },
  "reasoning": "<detailed explanation>",
  "action": "<KEEP|AUGMENT|GENERATE>"
}

Rules:
- KEEP: score >= 70
- AUGMENT: 40 <= score < 70
- GENERATE: score < 40
"""

SYSTEM_GENERATE = """\
You are an expert Rust developer writing integration tests as an EXTERNAL CONSUMER of a crate.

Your tests live in `tests/`, outside the crate source tree. You can ONLY access the crate's \
public API via `use {crate_name}::...`. You have no access to private modules or `crate::` paths.

You write tests that:
1. Compile on the first try — correct imports, correct types, no ambiguous calls.
2. Exercise realistic multi-step workflows, not trivial one-liners.
3. Include STRICT, SEMANTIC assertions that catch regressions.
4. Target specific uncovered API functions listed in the prompt.
5. Use fully qualified paths when trait methods are ambiguous.

## Assertion rules
- Assert concrete pre-computed values, not tautologies.
- Use `assert_eq!` / `assert_ne!` over `assert!(...)` for scalar comparisons.
- Check pre- AND post-state around mutations.
- At least 8 assert statements per test function.
- Never use `assert!(true)` placeholders.

## Unsafe code exercising
- Identify unsafe-backed APIs and prioritize testing those.
- Exercise capacity changes, large collections, boundary conditions.
- If the crate exposes `pub unsafe fn`, include at least one test calling it.

## Output rules
- Output ONLY the Rust test code. No explanations, no markdown fencing.
- Do NOT use `extern crate` statements.
- Do NOT use `use crate::...` — this is an integration test.
- Every test function MUST have `#[test]`.
""" + PLATFORM_CONSTRAINTS

SYSTEM_FIX = """\
You are an expert Rust developer fixing compilation errors in integration tests.

Rules:
- Fix ALL compiler errors while preserving test intent and assertions.
- Do NOT weaken assertions or remove test functions.
- Do NOT reduce iteration counts or data volume.
- Use the API reference and error hints provided to fix import paths, trait bounds, and types.
- If the test previously deadlocked or timed out, add timeouts or replace bare `.lock().unwrap()` \
with `try_lock` or timed alternatives.
- Output ONLY the fixed Rust code, no markdown fences, no explanations.
""" + PLATFORM_CONSTRAINTS


def build_evaluate_prompt(
    crate_name: str, package_name: str, is_workspace: bool,
    description: str, api_summary: str,
    total_api: int, covered_api: int, api_cov_pct: float,
    unit_files: int, unit_count: int, integ_files: int, integ_count: int,
    bench_files: int, total_tests: int,
    test_cov_pct: float, lib_cov_pct: float,
    unit_source: str, integ_source: str,
) -> str:
    return f"""\
Crate: {crate_name}
Package: {package_name}
Type: {"WORKSPACE" if is_workspace else "SINGLE"}
Description: {description or "N/A"}

## Public API Summary
{api_summary or "(not available)"}

## Coverage
- Total Public Functions: {total_api}
- Covered API Count: {covered_api} ({api_cov_pct:.1f}%)

## Test Suite
- Unit test files: {unit_files} ({unit_count} test functions)
- Integration test files: {integ_files} ({integ_count} test functions)
- Bench files: {bench_files}
- Total: {total_tests}

## LLVM Coverage
- Test coverage: {test_cov_pct:.1f}%
- Lib coverage: {lib_cov_pct:.1f}%

## Unit Test Source
```rust
{unit_source or "(none)"}
```

## Integration Test Source
```rust
{integ_source or "(none)"}
```

Evaluate this test suite. Output ONLY the JSON object.
"""


def build_generation_prompt(
    crate_name: str, description: str, api_index: list[dict],
    uncovered_fns: list, existing_tests: dict[str, str],
    history: list[dict] | None = None,
) -> str:
    api_lines = []
    for entry in api_index[:100]:
        sig = entry.get("signature", "")
        if sig:
            api_lines.append(f"  {entry['name']}: {sig}")

    fn_descriptions = []
    for fn in uncovered_fns[:20]:
        desc = f"- path: {fn.module_path}\n  signature: {fn.signature}\n  doc: {fn.doc_comment or '(none)'}"
        fn_descriptions.append(desc)

    test_refs = ""
    if existing_tests:
        parts = []
        total_chars = 0
        for name, content in list(existing_tests.items())[:3]:
            if total_chars + len(content) > 15000:
                break
            parts.append(f"--- {name} ---\n{content}")
            total_chars += len(content)
        if parts:
            test_refs = f"""
## Existing Tests (follow their pattern)
{chr(10).join(parts)}
"""

    # chain-of-thought: include previous attempt results
    history_section = ""
    if history:
        history_lines = []
        for h in history[-5:]:
            status = "COMPILED" if h.get("compiled") else "FAILED"
            err = h.get("error_summary", "")
            fname = h.get("test_name", "unknown")
            history_lines.append(f"- {fname}: {status}" + (f" — {err}" if err else ""))
        if history_lines:
            history_section = f"""
## Previous Attempts (DO NOT repeat these mistakes)
{chr(10).join(history_lines)}

Learn from the failures above. Avoid the same import errors, type mismatches, \
or API misuse patterns.
"""

    return f"""\
Write integration tests for the `{crate_name}` crate.

## Crate
name: {crate_name}
description: {description or "N/A"}

## Public API (from cargo doc)
ONLY use these names in your tests:
{chr(10).join(api_lines)}

## Target API Surface
You must write tests covering these UNCOVERED functions:

{chr(10).join(fn_descriptions)}
{test_refs}{history_section}
## Requirements
- Write a single integration test file that compiles with `cargo test`
- Import the crate: `use {crate_name}::*;` or explicit imports
- Create complex, multi-step test functions exercising real-world usage
- Include error handling for Result/Option returns
- ONLY call functions listed in the API section above
- Each test function must have `#[test]`

Output ONLY the Rust code, no markdown fences.
"""


def build_blind_generation_prompt(
    crate_name: str, description: str,
    existing_test_source: str, readme_summary: str,
    history: list[dict] | None = None,
) -> str:
    """prompt for generating tests when no API surface was discovered (macro crates)."""

    history_section = ""
    if history:
        history_lines = []
        for h in history[-5:]:
            status = "COMPILED" if h.get("compiled") else "FAILED"
            err = h.get("error_summary", "")
            fname = h.get("test_name", "unknown")
            history_lines.append(f"- {fname}: {status}" + (f" — {err}" if err else ""))
        if history_lines:
            history_section = f"""
## Previous Attempts (DO NOT repeat these mistakes)
{chr(10).join(history_lines)}
"""

    return f"""\
Write NEW integration tests for the `{crate_name}` crate.

## Crate
name: {crate_name}
description: {description or "N/A"}

## README
{readme_summary or "(not available)"}

## Existing Tests (study these to understand the API)
```rust
{existing_test_source[:20000]}
```
{history_section}
## Task
The crate uses proc macros or generated APIs that are not directly visible in cargo doc.
Study the existing test patterns above to understand how to use this crate's API.
Write NEW test functions that:
1. Exercise different API patterns than the existing tests
2. Test edge cases, error conditions, and boundary values
3. Include strong semantic assertions
4. Cover any unsafe-backed functionality

## Rules
- Import the crate: `use {crate_name}::*;` or explicit imports matching existing test patterns
- Each test function must have `#[test]`
- Do NOT duplicate existing test functions — write genuinely new tests
- Output ONLY the Rust code, no markdown fences

Output ONLY the Rust code, no markdown fences.
"""


def build_fix_prompt(
    crate_name: str, code: str, errors: str,
    api_summary: str, hints: str,
    ra_diagnostics: str = "",
) -> str:
    ra_section = ""
    if ra_diagnostics:
        ra_section = f"""
## Rust-Analyzer Diagnostics (precise error locations)
```
{ra_diagnostics}
```
"""

    return f"""\
The following integration test for `{crate_name}` failed to compile.

## Code
```rust
{code}
```

## Compiler errors
```
{errors}
```

{f"## Error hints{chr(10)}{hints}" if hints else ""}
{ra_section}
## API Reference
{api_summary or "(not available)"}

Fix ALL compiler errors. Common issues:
- Wrong import paths (use {crate_name}::...)
- Missing trait imports
- Type annotations needed for generics
- Ambiguous method calls — use fully qualified syntax
- Using `crate::` instead of `{crate_name}::` in integration tests

Output ONLY the fixed Rust code, no markdown fences.
"""


def extract_rust_code(raw: str) -> str:
    """strip markdown fences if the llm wrapped the code."""
    if "```" in raw:
        blocks = []
        in_block = False
        for line in raw.splitlines():
            if line.strip().startswith("```"):
                if in_block:
                    in_block = False
                else:
                    in_block = True
            elif in_block:
                blocks.append(line)
        if blocks:
            return "\n".join(blocks)
    return raw
