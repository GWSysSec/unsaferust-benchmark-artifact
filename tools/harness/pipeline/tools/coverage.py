"""
coverage.py — llvm-cov coverage analysis with rustfilt demangling

runs cargo llvm-cov, extracts line and function coverage,
uses rustfilt for correct rust symbol demangling (not c++filt).
"""

import json
import os
import re
import shutil
import subprocess
import logging
from dataclasses import dataclass, field
from pathlib import Path

from pipeline.tools.workspace import WorkspaceInfo


def _coverage_env() -> dict:
    """env for cargo llvm-cov: cap lints so older crates compile under newer rustc.

    many bootcamp crates have `#![deny(warnings)]` and were written against an
    older toolchain. newer rustc adds lints (e.g., mismatched_lifetime_syntaxes)
    that turn into hard compile errors. RUSTFLAGS=--cap-lints=allow neutralizes
    them without modifying source.
    """
    env = os.environ.copy()
    extra = "--cap-lints=allow"
    if "RUSTFLAGS" in env:
        if extra not in env["RUSTFLAGS"]:
            env["RUSTFLAGS"] = env["RUSTFLAGS"] + " " + extra
    else:
        env["RUSTFLAGS"] = extra
    return env

log = logging.getLogger(__name__)

PLATFORM_HINT_SEGMENTS = {
    "backends", "linux", "android", "apple", "darwin", "ios", "macos",
    "windows", "unix", "wasi", "wasm", "emscripten", "solaris",
    "freebsd", "netbsd", "openbsd", "dragonfly",
}


@dataclass
class CoverageResult:
    ok: bool = False
    mode: str = ""
    percent: float = 0.0
    lines_covered: int = 0
    lines_total: int = 0
    covered_functions: set[str] = field(default_factory=set)
    error: str = ""
    feature_args: list = field(default_factory=list)  # cargo args used (e.g. ["--all-features"])
    feature_label: str = ""  # human label for the winning rung


# features whose names suggest mutually-exclusive selectors (backends, runtimes, archs)
# enabling more than one usually breaks compilation
_CONFLICT_PREFIX_RE = re.compile(
    r"^(?:runtime|backend|tls|http\d?|tokio|async-std|smol|crypto|sha\d?|aes|"
    r"unstable|nightly|wasm|wasi|stdweb|js|web)[-_]",
    re.IGNORECASE,
)
# features that are almost always safe to enable (don't conflict, expand API)
_SAFE_FEATURE_NAMES = frozenset({
    "std", "alloc", "default", "serde", "serde-derive", "serde_derive",
    "derive", "use_std", "use-alloc", "full", "all",
})


def _read_features(cargo_toml: Path) -> dict:
    """parse [features] from a Cargo.toml. returns {feature_name: [deps]}."""
    try:
        import toml
        data = toml.loads(cargo_toml.read_text())
        feats = data.get("features", {})
        return {k: v for k, v in feats.items() if isinstance(v, list)}
    except Exception:
        return {}


def _select_safe_features(features: dict) -> list[str]:
    """from the [features] table, pick names likely safe to combine."""
    safe = []
    for name in features:
        low = name.lower()
        if low in _SAFE_FEATURE_NAMES:
            safe.append(name)
            continue
        if _CONFLICT_PREFIX_RE.match(low):
            continue
        # heuristic: anything starting with `with-` or `_unstable` skip
        if low.startswith(("_", "with-")) or low.endswith(("-backend", "_backend")):
            continue
        safe.append(name)
    return safe


def _feature_ladder(crate_path: Path) -> list[tuple[str, list[str]]]:
    """yield (label, cargo_args) tuples in the order to attempt them.

    rationale:
      1. default — what `cargo test` does; cheapest, most common success
      2. --all-features — best coverage when no conflicts exist
      3. --no-default-features --features <safe-set> — for crates whose
         default conflicts with --all-features (e.g. mutually exclusive backends)
      4. --no-default-features — last resort minimal compile
    """
    rungs: list[tuple[str, list[str]]] = [
        ("default", []),
        ("all-features", ["--all-features"]),
    ]
    feats = _read_features(crate_path / "Cargo.toml")
    safe = _select_safe_features(feats)
    if safe and len(safe) < len(feats):
        rungs.append((
            f"safe[{','.join(safe[:6])}{'+' if len(safe) > 6 else ''}]",
            ["--no-default-features", "--features", ",".join(safe)],
        ))
    if feats:
        rungs.append(("no-default", ["--no-default-features"]))
    return rungs


def _llvm_cov_available(crate_path: Path) -> bool:
    try:
        res = subprocess.run(
            ["cargo", "llvm-cov", "--help"],
            cwd=crate_path, capture_output=True, text=True, timeout=10,
        )
        return res.returncode == 0
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return False


def _demangle_rust_symbols(symbols: list[str]) -> list[str]:
    """demangle using rustfilt (correct for rust symbols) with c++filt fallback."""
    if not symbols:
        return []

    rustfilt = shutil.which("rustfilt")
    if rustfilt:
        try:
            proc = subprocess.run(
                [rustfilt],
                input="\n".join(symbols) + "\n",
                capture_output=True, text=True, timeout=20,
            )
            if proc.returncode == 0:
                out = proc.stdout.splitlines()
                if len(out) == len(symbols):
                    return out
                return out if out else symbols
        except (subprocess.TimeoutExpired, OSError):
            pass

    # fallback to c++filt
    try:
        proc = subprocess.Popen(
            ["c++filt"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            universal_newlines=True,
        )
        out, _ = proc.communicate("\n".join(symbols))
        demangled = out.strip().split("\n")
        if len(demangled) == len(symbols):
            return demangled
    except Exception:
        pass

    return symbols


def _strip_generics(name: str) -> str:
    prev = None
    cur = name
    while prev != cur:
        prev = cur
        cur = re.sub(r"<[^<>]*>", "", cur)
    return cur


def _normalize_function_name(name: str, lib_name: str) -> str | None:
    """normalize a demangled symbol to a canonical API path for coverage matching."""
    raw = name.strip()
    if not raw:
        return None

    # collapse trait-impl forms first, before generics get stripped:
    #   <T as crate::Mod::Trait>::method  ->  crate::Mod::Trait::method
    # we used to drop these entirely, but they're the ONLY symbols generated
    # for trait methods, so trait-heavy crates (borsh) scored near 0% without
    # this. non-greedy `[^<>]*?` keeps the rewrite safe under nested `<<...>>`
    # by only matching innermost `<...>` segments; outer wrappers fall through
    # to strip_generics below.
    raw = re.sub(r"<[^<>]*?\bas\s+([\w:]+)\s*>::(\w+)", r"\1::\2", raw)

    # inherent: <crate::Type>::method -> crate::Type::method
    if raw.startswith("<") and ">::" in raw:
        inside, method = raw[1:].split(">::", 1)
        raw = f"{inside}::{method}"

    # strip generics first so that closures appearing only as type
    # parameters (e.g. `join::<test::case::{closure#0}, ...>`) don't
    # cause us to drop the enclosing public function.
    raw = _strip_generics(raw)
    if "{closure" in raw or "::{impl" in raw:
        return None
    raw = raw.replace("::<_>", "").replace("::<>", "")
    raw = raw.split("$")[0].strip()
    raw = re.sub(r"\s+", "", raw)
    raw = raw.removesuffix("::")

    # normalize rustdoc disambiguation suffixes (e.g. `len-1`)
    parts = [seg for seg in raw.split("::") if seg]
    if not parts:
        return None
    parts[-1] = re.sub(r"-\d+$", "", parts[-1])
    raw = "::".join(parts)

    # must belong to this crate (filter by symbol prefix = lib name)
    lib_normalized = (lib_name or "").replace("-", "_")
    if not lib_normalized or not raw.startswith(f"{lib_normalized}::"):
        return None

    if _looks_platform_specific(raw):
        return None

    return raw


def _looks_platform_specific(path: str) -> bool:
    segments = [seg.lower() for seg in path.split("::") if seg]
    return any(seg in PLATFORM_HINT_SEGMENTS for seg in segments)


def _extract_covered_functions(data: dict, lib_name: str) -> set[str]:
    covered_raw = []
    for item in data.get("data", []):
        for fn in item.get("functions", []):
            count = fn.get("count", 0)
            name = fn.get("name", "")
            if not name or count <= 0:
                continue
            covered_raw.append(str(name))

    demangled = _demangle_rust_symbols(covered_raw)
    covered = set()
    for name in demangled:
        normalized = _normalize_function_name(name, lib_name)
        if normalized:
            covered.add(normalized)
    return covered


def _extract_line_summary(data: dict) -> tuple[int, int, float]:
    lines_covered = 0
    lines_total = 0
    for item in data.get("data", []):
        totals = item.get("totals", {})
        lines = totals.get("lines", {})
        lines_covered += int(lines.get("covered", 0) or 0)
        lines_total += int(lines.get("count", 0) or 0)
    percent = (lines_covered / lines_total * 100.0) if lines_total > 0 else 0.0
    return lines_covered, lines_total, percent


def _run_llvm_cov_cmd(
    crate_path: Path, cmd: list[str], mode: str,
    lib_name: str, timeout: int = 600,
) -> CoverageResult:
    try:
        res = subprocess.run(
            cmd, cwd=crate_path, capture_output=True, text=True, timeout=timeout,
            env=_coverage_env(),
        )
    except subprocess.TimeoutExpired:
        return CoverageResult(ok=False, mode=mode, error=f"timed out (>{timeout}s)")
    except FileNotFoundError:
        return CoverageResult(ok=False, mode=mode, error="cargo not found")

    if res.returncode != 0:
        # capture more of the tail end of stderr — the real rustc/cc1 error
        # usually appears near the bottom after a wall of "Compiling ..." lines.
        # 500 chars wasn't enough to see what failed (e.g. ndarray/ring).
        err_full = res.stderr or res.stdout or "cargo llvm-cov failed"
        err = err_full[-2000:] if len(err_full) > 2000 else err_full
        return CoverageResult(ok=False, mode=mode, error=err)

    try:
        data = json.loads(res.stdout)
    except json.JSONDecodeError:
        return CoverageResult(ok=False, mode=mode, error="could not parse llvm-cov JSON")

    lines_covered, lines_total, percent = _extract_line_summary(data)
    covered_fns = _extract_covered_functions(data, lib_name)

    return CoverageResult(
        ok=True, mode=mode, percent=percent,
        lines_covered=lines_covered, lines_total=lines_total,
        covered_functions=covered_fns,
    )


def _scope_candidates(ws: WorkspaceInfo) -> list[tuple[str, list[str]]]:
    """yield (scope_label, pkg_args) candidates in priority order.

    rationale: workspaces sometimes contain a sibling that won't compile in
    isolation (ndarray's blas-tests requires picking a BLAS backend feature
    that ndarray itself doesn't expose). --workspace tries to compile every
    member and dies on the broken sibling. -p primary skips the sibling but
    also misses tests in other members that exercise the primary's API
    (ouroboros's `examples` member is the actual test driver). so try the
    broad scope first, fall back to primary-only if everything failed.
    """
    if ws.extra_coverage_pkgs:
        # extras explicitly enumerated; cargo errors if --workspace is also set.
        pkg_args = ["-p", ws.primary_package_name]
        for extra in ws.extra_coverage_pkgs:
            pkg_args.extend(["-p", extra])
        return [("ws+extras", pkg_args)]
    if ws.is_workspace:
        return [("workspace", ["--workspace"]),
                ("primary-only", ["-p", ws.primary_package_name])]
    return [("single", [])]


def run_llvm_cov(ws: WorkspaceInfo, mode: str = "--tests") -> CoverageResult:
    """run cargo llvm-cov stepping through (scope, feature-flag) rungs.

    tries each rung in priority order; keeps the result with the most covered
    functions. early-exits if --all-features clearly maxes out.
    """
    if not _llvm_cov_available(ws.path):
        return CoverageResult(ok=False, error="cargo-llvm-cov unavailable")

    # --ignore-run-fail: keep coverage data even when individual tests fail
    # (otherwise crates like ron — 1 failing test out of 109 — get scored as 0%).
    common = ["--json", "--no-cfg-coverage", "--ignore-run-fail"]
    lib_name = ws.primary_lib_name or ws.primary_package_name
    ladder = _feature_ladder(ws.primary_crate_path)

    best: CoverageResult | None = None
    last_error: str = ""

    for scope_label, pkg_args in _scope_candidates(ws):
        # if a previous scope already produced any covered fns, don't burn time
        # on the fallback scope — it's strictly less informative.
        if best is not None and best.covered_functions:
            break
        for label, feat_args in ladder:
            cmd = ["cargo", "llvm-cov"] + pkg_args + [mode] + feat_args + common
            full_label = f"{scope_label}/{label}" if scope_label != "single" else label
            log.info(f"  coverage {mode} [{full_label}]...")
            res = _run_llvm_cov_cmd(ws.path, cmd, f"{mode} ({full_label})", lib_name)
            if not res.ok and "timed out" in res.error:
                log.warning(f"  coverage timed out, retrying with --test-threads=1...")
                res = _run_llvm_cov_cmd(
                    ws.path, cmd + ["--", "--test-threads=1"],
                    f"{mode} ({full_label}, threads=1)", lib_name,
                )
            if res.ok:
                res.feature_label = full_label
                res.feature_args = feat_args
                log.info(f"    [{full_label}] ok: {len(res.covered_functions)} fns covered, {res.percent:.1f}% lines")
                if best is None or len(res.covered_functions) > len(best.covered_functions):
                    best = res
                # early-exit: if this rung covered something AND --all-features was the rung, accept it
                if label == "all-features" and res.covered_functions:
                    return best
            else:
                last_error = res.error
                # show more of the tail so the actual rustc/cc1 error is visible.
                tail = res.error[-500:] if len(res.error) > 500 else res.error
                log.warning(f"    [{full_label}] failed: ...{tail}")

    if best is not None:
        return best
    return CoverageResult(ok=False, mode=mode, error=last_error or "all rungs failed")


def run_full_coverage(ws: WorkspaceInfo) -> CoverageResult:
    """run --tests coverage. --lib was deprecated: it duplicates --tests work
    on every crate we measure, doubling wall-clock per crate for no signal gain."""
    test_cov = run_llvm_cov(ws, "--tests")
    if test_cov.ok:
        log.info(f"  test coverage: {test_cov.percent:.2f}%")
    return test_cov
