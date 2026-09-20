"""
build_fixer.py — compile check, error classification, symbol resolution,
                  and precise error extraction for the fixer llm.
"""

import re
import subprocess
import logging
from pathlib import Path

from pipeline.config import PipelineConfig, cargo_env

log = logging.getLogger(__name__)

SYSTEMIC_PATTERNS = [
    re.compile(r"error\[E0635\]:\s*unknown feature"),
    re.compile(r"error\[E0463\]:\s*can't find crate"),
    re.compile(r"error: internal compiler error"),
    re.compile(r"error: linker .* not found"),
    re.compile(r"LLVM ERROR:"),
    re.compile(r"error: failed to download"),
    re.compile(r"error: failed to get"),
    re.compile(r"error: failed to parse manifest"),
]

REGISTRY_PATH_RE = re.compile(r"-->\s*(/home/[^/]+/\.cargo/registry/|/[^:]+/registry/src/)")

ERROR_HINTS = {
    "E0277": "trait bound not satisfied — check trait imports",
    "E0308": "mismatched types — check reference depth: &&T vs &T vs T",
    "E0502": "cannot borrow as mutable — save immutable read first",
    "E0599": "method not found — the method may be on a trait, add import",
    "E0433": "failed to resolve — check full module path",
    "E0596": "cannot borrow as mutable — add `mut`",
    "E0061": "wrong number of arguments — check signature",
}

DEADLOCK_PATTERNS = [
    re.compile(r"deadlock", re.IGNORECASE),
    re.compile(r"timed?\s*out", re.IGNORECASE),
    re.compile(r"test .+ has been running for over \d+ seconds"),
    re.compile(r"cannot acquire lock"),
    re.compile(r"lock poisoned"),
]


def compile_check(cfg: PipelineConfig, crate_path: str, test_code: str,
                  test_name: str = "pipeline_gen_test",
                  test_crate_path: str = None,
                  package_name: str = None,
                  feature_args: list[str] | None = None) -> tuple[bool, str]:
    """compile-check a generated test file.

    crate_path: workspace root (used as cwd for cargo)
    test_crate_path: where to place the test file (defaults to crate_path).
                     for workspace crates, this is the primary subcrate.
    package_name: if set, adds -p <name> to cargo (needed for workspace crates).
    feature_args: cargo feature flags (e.g. ["--all-features"]) that match the
                  coverage step's invocation — without this, a test can pass
                  per-file repair under default features but fail during
                  coverage measurement when extra cfg-gated code paths light up.
    """
    crate_path = Path(crate_path)
    file_root = Path(test_crate_path) if test_crate_path else crate_path
    test_file = file_root / "tests" / f"{test_name}.rs"
    test_file.parent.mkdir(parents=True, exist_ok=True)
    try:
        test_file.write_text(test_code)
        cmd = [cfg.cargo, "test"]
        if package_name:
            cmd.extend(["-p", package_name])
        cmd.extend(["--test", test_name, "--no-run", "--quiet", "--color=never"])
        if feature_args:
            cmd.extend(feature_args)
        result = subprocess.run(cmd, cwd=crate_path, capture_output=True, text=True,
                                timeout=120, env=cargo_env(cfg))
        if result.returncode == 0:
            return True, ""
        return False, result.stderr[:32000]
    except subprocess.TimeoutExpired:
        return False, "compilation timed out (120s)"
    finally:
        if test_file.exists():
            test_file.unlink()


def try_run_test(cfg: PipelineConfig, crate_path: str, test_code: str,
                 test_name: str,
                 test_crate_path: str = None,
                 package_name: str = None,
                 feature_args: list[str] | None = None) -> tuple[bool, str]:
    """compile and run a test with timeout to catch deadlocks and panics.

    crate_path: workspace root (used as cwd for cargo)
    test_crate_path: where to place the test file (defaults to crate_path).
    package_name: if set, adds -p <name> to cargo.
    feature_args: feature flags matching the coverage step (see compile_check).
    """
    crate_path_p = Path(crate_path)
    file_root = Path(test_crate_path) if test_crate_path else crate_path_p
    test_file = file_root / "tests" / f"{test_name}.rs"
    test_file.parent.mkdir(parents=True, exist_ok=True)
    test_file.write_text(test_code)
    try:
        timeout = getattr(cfg, "test_timeout", 60)
        max_threads = getattr(cfg, "max_test_threads", 4)
        cmd = [cfg.cargo, "test"]
        if package_name:
            cmd.extend(["-p", package_name])
        cmd.extend(["--test", test_name, "--quiet", "--color=never"])
        if feature_args:
            cmd.extend(feature_args)
        cmd.extend(["--", f"--test-threads={max_threads}"])
        result = subprocess.run(cmd, cwd=crate_path, capture_output=True, text=True,
                                timeout=timeout, env=cargo_env(cfg))
        output = result.stdout + "\n" + result.stderr
        return (True, "") if result.returncode == 0 else (False, output[-32000:])
    except subprocess.TimeoutExpired:
        return False, f"test execution timed out ({cfg.test_timeout}s) — likely deadlock or hang"
    finally:
        if test_file.exists():
            test_file.unlink()


# matches the cargo error line that pinpoints which integ test bin failed to compile.
# example: error: could not compile `jni` (test "gen_jni_JNIEnv_part1") due to 15 previous errors
_FAILED_TEST_RE = re.compile(r'could not compile\s+`[^`]+`\s+\(test\s+"([^"]+)"\)')


def quarantine_cross_compile_failures(
    cfg: PipelineConfig,
    crate_path: str,
    test_crate_path: str | None,
    package_name: str | None,
    feature_args: list[str] | None,
    eligible_test_names: set[str],
) -> tuple[list[str], list[str]]:
    """sweep generated integ tests together under the coverage feature set;
    iteratively quarantine (delete) any `gen_*.rs` that causes a cross-compile
    failure until the build succeeds or no further pin-pointable culprit can
    be found.

    a test can pass per-file repair under default features but fail under
    `--all-features` due to feature-gated code paths (jni postmortem). it can
    also pass alone but conflict with another generated test (duplicate `mod`
    names, etc). this sweep is the safety net for both cases.

    only files in `eligible_test_names` (without `.rs` suffix) are ever
    deleted; existing user tests are never touched. paths are resolved against
    `test_crate_path` if given, else `crate_path`.

    returns (quarantined_test_names, sweep_log_lines) — caller decides how to
    surface them in the generation result.
    """
    crate_path_p = Path(crate_path)
    file_root = Path(test_crate_path) if test_crate_path else crate_path_p
    tests_dir = file_root / "tests"

    quarantined: list[str] = []
    log_lines: list[str] = []

    # cap iterations at the number of eligible files + 1 to guarantee termination.
    max_iters = len(eligible_test_names) + 1
    for _ in range(max_iters):
        cmd = [cfg.cargo, "test", "--no-run", "--quiet", "--color=never"]
        if package_name:
            cmd.extend(["-p", package_name])
        if feature_args:
            cmd.extend(feature_args)
        try:
            result = subprocess.run(cmd, cwd=crate_path_p, capture_output=True,
                                    text=True, timeout=300, env=cargo_env(cfg))
        except subprocess.TimeoutExpired:
            log_lines.append("cross-compile sweep timed out (300s) — leaving files in place")
            return quarantined, log_lines

        if result.returncode == 0:
            return quarantined, log_lines

        # find the first generated test the build blames; cargo emits one
        # "could not compile ... (test \"NAME\")" line per failing bin.
        culprit = None
        for m in _FAILED_TEST_RE.finditer(result.stderr):
            name = m.group(1)
            if name in eligible_test_names:
                culprit = name
                break

        if not culprit:
            # build is failing but not on a file we own (e.g. an existing
            # user test or the lib itself). don't touch anything.
            log_lines.append(
                "cross-compile sweep: build failing but no generated test pinpointed — stopping"
            )
            return quarantined, log_lines

        bad_path = tests_dir / f"{culprit}.rs"
        if not bad_path.exists():
            # already gone — likely renamed/moved; defensively stop.
            log_lines.append(f"cross-compile sweep: culprit {culprit} not on disk — stopping")
            return quarantined, log_lines
        bad_path.unlink()
        quarantined.append(culprit)
        eligible_test_names = eligible_test_names - {culprit}
        log_lines.append(f"quarantined {culprit}: causes cross-compile failure under {feature_args or 'default features'}")

    log_lines.append(f"cross-compile sweep hit iteration cap ({max_iters})")
    return quarantined, log_lines


def classify_errors(errors: str) -> tuple[bool, str]:
    lines = errors.split("\n")
    for pattern in SYSTEMIC_PATTERNS:
        match = pattern.search(errors)
        if match:
            match_line_idx = errors[:match.start()].count("\n")
            context = "\n".join(lines[max(0, match_line_idx-2):min(len(lines), match_line_idx+5)])
            if REGISTRY_PATH_RE.search(context):
                return True, f"dependency error: {match.group(0)[:100]}"
            if "internal compiler error" in match.group(0):
                return True, "internal compiler error (ICE)"
            if "LLVM ERROR" in match.group(0):
                return True, "LLVM crash"
            if any(k in match.group(0) for k in ["failed to download", "failed to get", "failed to parse manifest"]):
                return True, f"cargo fetch error: {match.group(0)[:100]}"

    error_locations = re.findall(r"-->\s*([^\s:]+):\d+:\d+", errors)
    if error_locations:
        non_registry = [loc for loc in error_locations if ".cargo/registry" not in loc]
        if not non_registry:
            return True, "all errors originate from dependencies"
    return False, ""


def classify_runtime_errors(output: str) -> tuple[bool, str]:
    """detect deadlock/hang patterns in test execution output."""
    for pattern in DEADLOCK_PATTERNS:
        if pattern.search(output):
            return True, "deadlock_or_hang"
    if "SIGSEGV" in output or "SIGABRT" in output:
        return True, "crash"
    return False, ""


def extract_relevant_errors(errors: str, test_file_name: str) -> str:
    """filter compiler output to only errors from the generated test file.

    strips dependency warnings, registry-path errors, and unrelated noise
    to give the fixer llm a focused error set.
    """
    relevant = []
    lines = errors.split("\n")
    i = 0
    current_block = []
    in_relevant_block = False
    max_errors = 5
    error_count = 0

    while i < len(lines):
        line = lines[i]

        # start of a new error/warning block
        if line.startswith("error") or line.startswith("warning"):
            # flush previous block if relevant
            if in_relevant_block and current_block:
                relevant.extend(current_block)
                relevant.append("")
                error_count += 1
            current_block = [line]

            # skip warnings entirely
            if line.startswith("warning"):
                in_relevant_block = False
            else:
                in_relevant_block = True
            i += 1
            continue

        # location line: check if it points to our test file
        if line.strip().startswith("-->"):
            current_block.append(line)
            if ".cargo/registry" in line:
                in_relevant_block = False
            elif test_file_name in line:
                in_relevant_block = True
            i += 1
            continue

        current_block.append(line)
        i += 1

        if error_count >= max_errors:
            break

    # flush last block
    if in_relevant_block and current_block and error_count < max_errors:
        relevant.extend(current_block)

    # also grab the summary line
    for line in lines[-5:]:
        if re.match(r"error(\[E\d+\])?:", line) or "aborting due to" in line:
            relevant.append(line)

    return "\n".join(relevant) if relevant else errors[:4000]


def run_rust_analyzer_diagnostics(crate_path: str, test_file: str) -> str:
    """run rust-analyzer diagnostics on a specific file for precise error info."""
    import shutil
    ra = shutil.which("rust-analyzer")
    if not ra:
        return ""

    try:
        test_path = Path(crate_path) / "tests" / test_file
        if not test_path.exists():
            return ""

        cmd = [ra, "diagnostics", str(test_path)]
        result = subprocess.run(
            cmd, cwd=crate_path, capture_output=True, text=True, timeout=30,
        )
        if result.returncode != 0:
            return ""

        output = result.stdout.strip()
        if not output:
            return ""

        # limit output size
        return output[:3000]
    except (subprocess.TimeoutExpired, FileNotFoundError, OSError) as e:
        log.debug(f"  ra diagnostics failed: {e}")
        return ""


def extract_missing_symbols(errors: str) -> list[str]:
    symbols = set()
    for match in re.finditer(r"no method named `([^`]+)` found", errors):
        symbols.add(match.group(1))
    for match in re.finditer(r"cannot find type `([^`]+)` in", errors):
        symbols.add(match.group(1))
    for match in re.finditer(r"unresolved import `([^`]+)`", errors):
        parts = match.group(1).split("::")
        if parts:
            symbols.add(parts[-1])
    for match in re.finditer(r"could not find `([^`]+)` in", errors):
        symbols.add(match.group(1))
    return list(symbols)


def error_specific_hints(errors: str) -> str:
    codes = set(re.findall(r"E(\d{4})", errors))
    hints = [f"- E{c}: {ERROR_HINTS[f'E{c}']}" for c in sorted(codes) if f"E{c}" in ERROR_HINTS]
    return "\n".join(hints) if hints else ""


def error_signature(errors: str) -> str:
    codes = sorted(set(re.findall(r"error\[E\d+\]", errors)))
    first_msg = ""
    for line in errors.split("\n"):
        line = line.strip()
        if line.startswith("error["):
            first_msg = re.sub(r"-->\s*[^\s]+:\d+:\d+", "--> FILE:LINE", line)[:120]
            break
    if codes:
        return f"{','.join(codes)}|{first_msg}"
    for line in errors.split("\n"):
        if "panicked at" in line:
            return line.strip()[:120]
    for line in errors.split("\n"):
        line = line.strip()
        if line.startswith("error"):
            return re.sub(r"-->\s*[^\s]+:\d+:\d+", "--> FILE:LINE", line)[:120]
    return ""
