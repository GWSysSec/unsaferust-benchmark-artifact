"""
runtime_bench.py — build a crate's test binaries with the instrumentation and
collect the stat files they write.

The LLVM passes insert calls into the unsafe-perf runtime, so every rustc
invocation cargo makes gets a prebuilt unsafe-perf rlib through RUSTFLAGS,
without editing the crate under test:
  --extern force:unsafe_perf=<rlib>   force-link the prebuilt rlib
  -L <deps_dir>                       resolve the rlib's dependencies
  -Z unstable-options                 unlock the unstable codegen flag below
  -C unsafe_include_native_lib=bool   whether unsafe code in core/std/alloc counts
  -C llvm-args=<feature_passes>       per-feature LLVM pass selection
  -C debuginfo=2                      symbolicated stack frames for trackers

The rlib must be built by the same stage1 rustc with the matching cargo
feature, so ensure_unsafe_perf_built() builds one per feature into
<unsafe_perf_path>/target-<feature>/release/.
"""

import json
import logging
import os
import shutil
import subprocess
from dataclasses import dataclass, field
from pathlib import Path

log = logging.getLogger(__name__)


# every feature needs the instmarker pass
BASE_LLVM_FLAGS = ["--enable-instmarker"]

# per feature: the LLVM passes to add and the environment for build and run
FEATURE_MATRIX: dict[str, dict] = {
    "heap_tracker": {
        "llvm_flags": ["--enable-heap-tracker"],
        "env": {},
    },
    "cpu_cycle_counter": {
        "llvm_flags": [
            "--enable-cpu-cycle-count",
            "--enable-external-call-tracker",
        ],
        "env": {},
    },
    "unsafe_counter": {
        "llvm_flags": [
            "--enable-unsafe-inst-counter",
            "--enable-unsafe-function-tracker",
            "--enable-stdlib-api-tracker",
        ],
        # enables the MIR pass that marks calls into the standard library
        "env": {"UNSAFE_ENABLE_STDLIB_TRACKER": "1"},
    },
}


# When set, the full output of any failed cargo build is written here, one file
# per (crate, feature, native variant).
_FAILURE_LOG_DIR: Path | None = None


def set_failure_log_dir(path) -> None:
    global _FAILURE_LOG_DIR
    _FAILURE_LOG_DIR = Path(path) if path else None


# -C unsafe_include_native_lib: whether unsafe code in core/std/alloc counts
NATIVE_LIB_VARIANTS: list[tuple[str, bool]] = [
    ("without_native", False),
    ("with_native", True),
]


# ---------------------------------------------------------------------------
# result types
# ---------------------------------------------------------------------------

@dataclass
class BinResult:
    name: str
    exit_code: int = 0
    stat_files: list[str] = field(default_factory=list)
    stderr_tail: str = ""


@dataclass
class VariantResult:
    """one (feature, unsafe_include_native_lib) combo."""
    native_lib: bool
    ok: bool = False
    error: str = ""
    bins: list[BinResult] = field(default_factory=list)


@dataclass
class FeatureResult:
    feature: str
    error: str = ""  # setup error raised before any variant ran
    variants: dict = field(default_factory=dict)  # variant name -> VariantResult


# ---------------------------------------------------------------------------
# unsafe-perf rlib build (idempotent)
# ---------------------------------------------------------------------------

def ensure_unsafe_perf_built(
    unsafe_perf_path: Path, feature: str, rustc: str, cargo: str,
) -> tuple[Path, Path]:
    """build unsafe-perf with `--features <feature>` (per-feature target dir);
    return (rlib_path, deps_dir).

    rustc must be the one cargo will use to compile the crate-under-test,
    otherwise rustc rejects the rlib's metadata.
    """
    unsafe_perf_path = Path(unsafe_perf_path).resolve()
    if not unsafe_perf_path.exists():
        raise FileNotFoundError(f"unsafe-perf path does not exist: {unsafe_perf_path}")

    target_dir = unsafe_perf_path / f"target-{feature}"
    rlib = target_dir / "release" / "libunsafe_perf.rlib"
    deps = target_dir / "release" / "deps"
    if rlib.exists() and deps.exists():
        return rlib, deps

    # the unsafe_counter passes also call the stdlib_api_tracker runtime
    cargo_feats = feature
    if feature == "unsafe_counter":
        cargo_feats = "unsafe_counter,stdlib_api_tracker"

    log.info(f"  [bench] building unsafe-perf ({cargo_feats}) at {target_dir}")
    env = os.environ.copy()
    env["RUSTC"] = rustc
    env["CARGO_TARGET_DIR"] = str(target_dir)
    env.pop("RUSTFLAGS", None)
    proc = subprocess.run(
        [cargo, "build", "--release", "--features", cargo_feats],
        cwd=unsafe_perf_path, env=env,
        capture_output=True, text=True, timeout=8 * 3600,
    )
    if proc.returncode != 0:
        raise RuntimeError(
            f"unsafe-perf build ({feature}) failed: {(proc.stderr or '')[-1500:]}"
        )
    if not rlib.exists():
        raise RuntimeError(f"unsafe-perf build succeeded but no rlib at {rlib}")
    return rlib, deps


# ---------------------------------------------------------------------------
# rustflags assembly
# ---------------------------------------------------------------------------

def _build_rustflags(
    feature: str, include_native_lib: bool, rlib: Path, deps: Path,
) -> str:
    """assemble the RUSTFLAGS string for one (feature, native-variant) combo."""
    spec = FEATURE_MATRIX[feature]
    parts: list[str] = [
        f"--extern force:unsafe_perf={rlib}",
        f"-L {deps}",
        "-Z unstable-options",
        f"-C unsafe_include_native_lib={'true' if include_native_lib else 'false'}",
        "-C debuginfo=2",
    ]
    for flag in BASE_LLVM_FLAGS + spec["llvm_flags"]:
        parts.append(f"-C llvm-args={flag}")
    return " ".join(parts)


# ---------------------------------------------------------------------------
# build + run
# ---------------------------------------------------------------------------

_FALLBACK_RESOLVE_HINTS = (
    "edition 2024 is unstable",
    "use of unstable library feature 'noop_waker'",
    "use of unstable library feature 'strict_provenance'",
    "use of unstable feature: 'lint_reasons'",
    "requires Rust ",
)


def _run_cargo_test_no_run(
    crate_path: Path, env: dict, cmd: list[str], timeout: int | None,
) -> tuple[int, str, str]:
    """thin wrapper returning (returncode, stdout, stderr)."""
    try:
        proc = subprocess.run(
            cmd, cwd=crate_path, env=env,
            capture_output=True, text=True, timeout=timeout,
        )
    except subprocess.TimeoutExpired:
        return -1, "", f"timed out after {timeout}s"
    return proc.returncode, proc.stdout or "", proc.stderr or ""


def _cargo_test_no_run(
    crate_path: Path, cfg, feature: str, include_native_lib: bool,
    rlib: Path, deps: Path, timeout: int | None = None,
    test_targets: list[str] | None = None,
    cargo_features: list[str] | None = None,
    extra_cargo_args: list[str] | None = None,
) -> tuple[list[Path], str]:
    """build the test bins for one (feature, native-variant) combo; return
    their executable paths.

    `crate_path` MUST be the dir whose Cargo.toml carries [package].
    `test_targets` defaults to ["--lib", "--bins", "--tests", "--examples"].

    The first build passes --ignore-rust-version. If a dependency resolved to
    a version stage1 rustc cannot build, the lockfile is regenerated and the
    build retried without it, so the fallback resolver picks older versions.
    """
    spec = FEATURE_MATRIX[feature]
    env = os.environ.copy()
    env["RUSTC"] = cfg.rustc
    env["RUSTFLAGS"] = _build_rustflags(feature, include_native_lib, rlib, deps)
    env.setdefault("CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS", "fallback")
    env.update(spec.get("env", {}))

    targets = test_targets if test_targets is not None else [
        "--lib", "--bins", "--tests", "--examples",
    ]
    feature_args: list[str] = []
    if cargo_features:
        feature_args = ["--features", ",".join(cargo_features)]
    base_cmd = [
        cfg.cargo, "test", "--release", "--no-run", "--no-fail-fast",
        *targets, *feature_args, *(extra_cargo_args or []),
        "--message-format=json",
    ]
    cmd = base_cmd[:2] + ["--ignore-rust-version"] + base_cmd[2:]

    rc, stdout, stderr = _run_cargo_test_no_run(crate_path, env, cmd, timeout)
    if rc == -1:
        return [], f"cargo test --no-run {stderr}"

    if rc != 0 and any(h in (stdout + stderr) for h in _FALLBACK_RESOLVE_HINTS):
        lock = crate_path / "Cargo.lock"
        # workspace members may not have their own lockfile
        if not lock.exists():
            p = crate_path.parent
            while p != p.parent:
                if (p / "Cargo.lock").exists():
                    lock = p / "Cargo.lock"
                    break
                p = p.parent
        if lock.exists():
            try:
                lock.unlink()
                log.info(f"  [bench] retrying after lockfile regen ({lock.name} cleared)")
            except OSError:
                pass
        rc, stdout, stderr = _run_cargo_test_no_run(crate_path, env, base_cmd, timeout)

    if rc != 0:
        if _FAILURE_LOG_DIR is not None:
            try:
                _FAILURE_LOG_DIR.mkdir(parents=True, exist_ok=True)
                tag = f"{crate_path.name}.{feature}." \
                      f"{'with' if include_native_lib else 'without'}_native"
                (_FAILURE_LOG_DIR / f"{tag}.log").write_text(
                    f"$ {' '.join(cmd)}\n\n--- stdout ---\n{stdout}"
                    f"\n\n--- stderr ---\n{stderr}\n"
                )
            except OSError:
                pass
        msgs: list[str] = []
        for line in stdout.splitlines():
            if not line.startswith("{"):
                continue
            try:
                m = json.loads(line)
            except json.JSONDecodeError:
                continue
            if m.get("reason") == "compiler-message":
                lvl = m.get("message", {}).get("level", "")
                rendered = m.get("message", {}).get("rendered", "")
                if lvl == "error" and rendered:
                    msgs.append(rendered)
        diag = "\n".join(msgs)[-2000:] if msgs else stderr[-1500:]
        return [], f"cargo test --no-run exit {rc}: {diag}"

    bins: list[Path] = []
    for line in stdout.splitlines():
        if not line.startswith("{"):
            continue
        try:
            msg = json.loads(line)
        except json.JSONDecodeError:
            continue
        if msg.get("reason") != "compiler-artifact":
            continue
        if not msg.get("profile", {}).get("test"):
            continue
        exe = msg.get("executable")
        if exe:
            bins.append(Path(exe))
    return bins, ""


def _writable_tmp_jsons() -> list[Path]:
    """`$UNSAFE_STAT_DIR/*.json` (default /tmp) that this user may remove."""
    stat_dir = Path(os.environ.get("UNSAFE_STAT_DIR", "/tmp"))
    if not stat_dir.is_dir():
        return []
    out: list[Path] = []
    for p in stat_dir.glob("*.json"):
        if os.access(p, os.W_OK):
            out.append(p)
    return out


def _clear_tmp_stats() -> None:
    for p in _writable_tmp_jsons():
        try:
            p.unlink()
        except OSError:
            pass


def _harvest_stat_files(out_dir: Path, prefix: str) -> list[str]:
    """move the stat files into out_dir, prefixed with the bin name."""
    out_dir.mkdir(parents=True, exist_ok=True)
    moved: list[str] = []
    for p in sorted(_writable_tmp_jsons()):
        target = out_dir / f"{prefix}__{p.name}"
        try:
            shutil.move(str(p), target)
            moved.append(target.name)
        except OSError as e:
            log.warning(f"  [bench] failed to harvest {p}: {e}")
    return moved


def _run_variant(
    crate_path: Path, cfg, feature: str, include_native_lib: bool,
    rlib: Path, deps: Path, variant_dir: Path, bin_timeout: int | None,
    test_targets: list[str] | None = None,
    cargo_features: list[str] | None = None,
    extra_cargo_args: list[str] | None = None,
) -> VariantResult:
    res = VariantResult(native_lib=include_native_lib)
    native_tag = "with_native" if include_native_lib else "without_native"

    bins, build_err = _cargo_test_no_run(
        crate_path, cfg, feature, include_native_lib, rlib, deps,
        test_targets=test_targets,
        cargo_features=cargo_features,
        extra_cargo_args=extra_cargo_args,
    )
    if build_err:
        res.error = build_err
        log.warning(
            f"    [bench] {feature}/{native_tag} build failed: {build_err[:300]}"
        )
        return res

    if not bins:
        res.error = "no test bins emitted by cargo"
        log.warning(f"    [bench] {feature}/{native_tag}: no test bins")
        return res

    variant_dir.mkdir(parents=True, exist_ok=True)
    bin_env = os.environ.copy()
    bin_env.update(FEATURE_MATRIX[feature].get("env", {}))

    for exe in bins:
        _clear_tmp_stats()
        try:
            proc = subprocess.run(
                [str(exe)], cwd=crate_path, env=bin_env,
                capture_output=True, text=True, timeout=bin_timeout,
            )
            exit_code = proc.returncode
            stderr_tail = (proc.stderr or "")[-400:]
        except subprocess.TimeoutExpired:
            exit_code = 124
            stderr_tail = "(timeout)"
        except OSError as e:
            exit_code = -1
            stderr_tail = f"exec failed: {e}"

        moved = _harvest_stat_files(variant_dir, prefix=exe.name)
        res.bins.append(BinResult(
            name=exe.name, exit_code=exit_code,
            stat_files=moved, stderr_tail=stderr_tail,
        ))
        log.info(f"      [bench] {exe.name} exit={exit_code} stats={len(moved)}")

    res.ok = any(b.stat_files for b in res.bins)
    return res


def run_feature(
    crate_path: Path, cfg, feature: str, out_dir: Path,
    bin_timeout: int | None = 8 * 3600,
    test_targets: list[str] | None = None,
    variants_root: Path | None = None,
    cargo_features: list[str] | None = None,
    extra_cargo_args: list[str] | None = None,
) -> FeatureResult:
    """build the matching unsafe-perf rlib if needed, then run both native-lib
    variants for this feature into `variants_root` (default out_dir/<feature>/).

    `crate_path` is the primary crate dir (the one carrying [package]).
    """
    res = FeatureResult(feature=feature)
    log.info(f"  [bench] feature={feature}")
    try:
        rlib, deps = ensure_unsafe_perf_built(
            Path(cfg.unsafe_perf_path), feature, cfg.rustc, cfg.cargo,
        )
    except Exception as e:
        res.error = f"unsafe-perf build: {e}"
        log.warning(f"  [bench] {feature}: {res.error}")
        return res

    feat_dir = variants_root if variants_root is not None else (Path(out_dir) / feature)
    for variant_name, include_native in NATIVE_LIB_VARIANTS:
        log.info(f"  [bench]   variant={variant_name}")
        res.variants[variant_name] = _run_variant(
            crate_path, cfg, feature, include_native, rlib, deps,
            feat_dir / variant_name, bin_timeout,
            test_targets=test_targets,
            cargo_features=cargo_features,
            extra_cargo_args=extra_cargo_args,
        )
    return res
