"""
runtime_bench.py — link the unsafe-perf benchmarking runtime into a crate's
cargo test bins and harvest per-feature stat files.

design
------
the LLVM passes (--enable-instmarker / --enable-heap-tracker / ...) inject
references to runtime symbols (dyn_mem_access, dyn_unsafe_mem_access,
record_program_start, ...) into every compiled object. those symbols live in
the unsafe-perf crate. for a bench to work, every test binary cargo produces
must end up with the unsafe-perf rlib in its link command — including binaries
cargo synthesizes for build scripts, which never name unsafe-perf as a dep.

we use the rustc `--extern force:` modifier (cf. cpu_cycle_count_pipeline.sh
prototype) to force-link a pre-built unsafe-perf rlib into every rustc
invocation cargo issues, regardless of source-level extern crate declarations.
this avoids touching the crate-under-test entirely:

  * no Cargo.toml mutation (which corrupts virtual workspace manifests)
  * no `extern crate unsafe_perf;` injection into tests/*.rs or src/lib.rs
  * no .cargo/config.toml backup-and-swap dance

all configuration goes through env RUSTFLAGS:
  --extern force:unsafe_perf=<rlib>   force-link the pre-built rlib
  -L <deps_dir>                       resolve the rlib's transitive deps
  -Z unstable-options                 unlock the unstable codegen flag below
  -C unsafe_include_native_lib=bool   (per-variant) whether ignore_fn() in
                                      rustc_middle/mir/unsafety.rs short-
                                      circuits unsafe instrumentation inside
                                      core/std/alloc/proc_macro/test/unwind
  -C llvm-args=<feature_passes>       per-feature LLVM pass selection
  -C debuginfo=2                      symbolicated stack frames for trackers

the unsafe-perf rlib must be pre-built with the same stage1 rustc cargo will
use AND with the matching cargo feature enabled (heap_tracker /
cpu_cycle_counter / unsafe_counter), because the per-feature tracker modules
in lib/perf/src/ are `#[cfg(feature)]`-gated. ensure_unsafe_perf_built()
builds one rlib per feature into a separate target dir
(`<unsafe_perf_path>/target-<feature>/release/`) and caches it idempotently.

note: cargo's lockfile may pin packages whose Cargo.toml requires a newer
rustc than stage1 1.80-dev (e.g. getrandom 0.4.2 wants 1.85). callers should
either pin those down (`cargo update -p X --precise <older>`) or set
CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=fallback on a fresh resolve. this
module sets the env var defensively but does not edit lockfiles.
"""

import json
import logging
import os
import shutil
import subprocess
from dataclasses import asdict, dataclass, field
from pathlib import Path

log = logging.getLogger(__name__)


# instmarker is the foundation pass — every feature needs it.
BASE_LLVM_FLAGS = ["--enable-instmarker"]

# mirror of verify-per-feature.sh FEATURE_FLAGS / env. each entry maps to:
#   - llvm_flags: list of -C llvm-args= switches (BASE_LLVM_FLAGS prepended)
#   - env: process env vars for both the cargo build and the bin run
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
        # UNSAFE_ENABLE_STDLIB_TRACKER=1 enables the rustc MIR pass that
        # inserts `# __unsafe_stdlib_call:<path>` inline-asm markers at
        # stdlib unsafe call sites; the LLVM --enable-stdlib-api-tracker
        # pass converts those markers into runtime calls.
        "env": {"UNSAFE_ENABLE_STDLIB_TRACKER": "1"},
    },
}

# ---------------------------------------------------------------------------
# whole-dependency-graph instrumentation (opt in)
# ---------------------------------------------------------------------------
# By default the LLVM analysis passes instrument only the crate cargo marked
# primary, because each pass guards on isPrimaryPackage(), which reads the
# CARGO_PRIMARY_PACKAGE environment variable. A dependency therefore executes
# unsafe code that no counter ever sees.
#
# set_instrument_all_packages(True) turns that off for the whole run. It sets
# UNSAFE_INSTRUMENT_ALL_PACKAGES=1, which the compiler reads as "instrument
# every crate", and points RUSTC_WRAPPER at rustc-wrapper-alldeps.sh, which
# keeps build scripts and proc-macro crates out of the instrumented set
# because their code runs during compilation rather than in the measured test
# binary. Both are compile-time settings, so only the cargo build environment
# needs them; the environment used to run a finished test binary does not.
_INSTRUMENT_ALL_PACKAGES = False

ALLDEPS_WRAPPER = (
    Path(__file__).resolve().parents[2] / "scripts" / "rustc-wrapper-alldeps.sh"
)


# When set, the complete stdout and stderr of any failed cargo build is
# written here, one file per (crate, feature, native variant).
_FAILURE_LOG_DIR: Path | None = None


def set_failure_log_dir(path) -> None:
    global _FAILURE_LOG_DIR
    _FAILURE_LOG_DIR = Path(path) if path else None


def set_instrument_all_packages(enabled: bool) -> None:
    """Instrument every crate in the dependency graph, not just the primary."""
    global _INSTRUMENT_ALL_PACKAGES
    _INSTRUMENT_ALL_PACKAGES = bool(enabled)
    if enabled and not ALLDEPS_WRAPPER.exists():
        raise FileNotFoundError(
            f"instrument-all-deps mode needs the wrapper at {ALLDEPS_WRAPPER}"
        )


def instrument_all_packages() -> bool:
    return _INSTRUMENT_ALL_PACKAGES


def _apply_all_packages_env(env: dict) -> None:
    """Add the whole-graph instrumentation settings to a cargo build env.

    TRAP: neither setting enters cargo's fingerprint. UNSAFE_INSTRUMENT_ALL_
    PACKAGES is read by the LLVM pass at compile time, not passed in RUSTFLAGS,
    and RUSTC_WRAPPER changes only which units carry the pass flags. So the two
    instrumentation scopes produce binaries with the SAME cargo hash: the
    aho-corasick binary is 90baa2b297641e14 in both scopes and in the published
    rebench_v2 data.

    A target directory shared between the two scopes would therefore be reused
    rather than rebuilt, and the second scope would silently measure the first
    scope's binary. Every caller must give each scope its own target directory.
    rebench.py does this by copying the crate under a per-arm --tmp-rebench
    root; measure_heap_runtime.py sets a distinct CARGO_TARGET_DIR per scope.
    Repeat runs within one scope reuse the build deliberately, which is what a
    repeat measurement wants: same binary, executed again.
    """
    if not _INSTRUMENT_ALL_PACKAGES:
        return
    env["UNSAFE_INSTRUMENT_ALL_PACKAGES"] = "1"
    env["RUSTC_WRAPPER"] = str(ALLDEPS_WRAPPER)


# both variants of the -C unsafe_include_native_lib codegen flag are run for
# every feature. when false (rustc default), ignore_fn() short-circuits unsafe
# instrumentation inside core/std/alloc/proc_macro/test/unwind, so native-lib
# unsafe ops don't show up in any of the per-feature stats. when true, those
# functions get instrumented like user code and totals typically rise. it's a
# measurement decision, not a correctness one — we record both. the flag is
# unstable, so each variant also passes -Z unstable-options.
NATIVE_LIB_VARIANTS: list[tuple[str, bool]] = [
    ("without_native", False),
    ("with_native", True),
]


# ---------------------------------------------------------------------------
# result types (consumed by profile._summarize_bench)
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


@dataclass
class BenchResult:
    label: str           # "before" | "after"
    crate: str
    out_dir: str
    features: dict = field(default_factory=dict)  # feature -> FeatureResult


# ---------------------------------------------------------------------------
# unsafe-perf rlib build (idempotent)
# ---------------------------------------------------------------------------

def ensure_unsafe_perf_built(
    unsafe_perf_path: Path, feature: str, rustc: str, cargo: str,
) -> tuple[Path, Path]:
    """build unsafe-perf with `--features <feature>` (per-feature target dir);
    return (rlib_path, deps_dir).

    rustc must be the one cargo will use to compile the crate-under-test —
    otherwise the rlib's metadata hash mismatches and rustc rejects the
    --extern force resolution. we set RUSTC=<rustc> to enforce that.
    we don't share `target/` across features because each feature gates a
    different set of symbols and a single rlib can't satisfy all three.
    """
    unsafe_perf_path = Path(unsafe_perf_path).resolve()
    if not unsafe_perf_path.exists():
        raise FileNotFoundError(f"unsafe-perf path does not exist: {unsafe_perf_path}")

    target_dir = unsafe_perf_path / f"target-{feature}"
    rlib = target_dir / "release" / "libunsafe_perf.rlib"
    deps = target_dir / "release" / "deps"
    if rlib.exists() and deps.exists():
        return rlib, deps

    # unsafe_counter LLVM pass also inserts calls into stdlib_api_tracker
    # runtime symbols, so the rlib must include that module too.
    cargo_feats = feature
    if feature == "unsafe_counter":
        cargo_feats = "unsafe_counter,stdlib_api_tracker"

    log.info(f"  [bench] building unsafe-perf ({cargo_feats}) at {target_dir}")
    env = os.environ.copy()
    env["RUSTC"] = rustc
    env["CARGO_TARGET_DIR"] = str(target_dir)
    # don't carry caller RUSTFLAGS into this build — the rlib must be vanilla.
    env.pop("RUSTFLAGS", None)
    # the runtime library must never instrument itself, so drop the
    # whole-graph settings even when the surrounding run has them on.
    env.pop("UNSAFE_INSTRUMENT_ALL_PACKAGES", None)
    env.pop("RUSTC_WRAPPER", None)
    proc = subprocess.run(
        [cargo, "build", "--release", "--features", cargo_feats],
        cwd=unsafe_perf_path, env=env,
        capture_output=True, text=True, timeout=600,
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
    "requires Rust ",  # dep-version mismatch from the resolver
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
    """build all test bins (lib unit-test + tests/* + bins + examples) for one
    (feature, native-variant) combo; return their executable paths.

    `crate_path` MUST be the dir whose Cargo.toml carries [package].
    cargo emits one JSON line per artifact under --message-format=json; we
    keep every "compiler-artifact" with profile.test == true. on cargo
    failure we surface "compiler-message" diagnostics from stdout (cargo with
    --message-format=json routes most error rendering through stdout).

    `test_targets` overrides the cargo target selection flags. Default
    ["--lib", "--bins", "--tests", "--examples"] keeps the broad scope used
    by the original after-bench. Pass ["--tests"] to bench only the
    integration tests under tests/*.rs (the rebench flow uses this so the
    counters reflect just our generated tests).

    primary→transitive rust-version handling:
      - first attempt uses `--ignore-rust-version` so crates whose own
        manifest pins rust-version > 1.80 (bootcamp's getrandom@0.4.2, etc.)
        proceed past cargo's gate.
      - if that fails because a TRANSITIVE dep was resolved to a too-new
        version (borsh-rs/fs4 pull in getrandom 0.4.2 / async-lock 3.4.2 /
        wasmparser 0.244 through their dev-deps), regenerate the lockfile and
        retry WITHOUT `--ignore-rust-version`. CARGO_RESOLVER_INCOMPATIBLE_
        RUST_VERSIONS=fallback then picks older dep versions compatible with
        the toolchain rust-version (1.80).
    """
    spec = FEATURE_MATRIX[feature]
    env = os.environ.copy()
    env["RUSTC"] = cfg.rustc
    env["RUSTFLAGS"] = _build_rustflags(feature, include_native_lib, rlib, deps)
    # cargo 1.84+ resolver knob: prefer dep versions compatible with the
    # active rustc when the lockfile permits; harmless on older cargo.
    env.setdefault("CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS", "fallback")
    env.update(spec.get("env", {}))
    _apply_all_packages_env(env)

    targets = test_targets if test_targets is not None else [
        "--lib", "--bins", "--tests", "--examples",
    ]
    feature_args: list[str] = []
    if cargo_features:
        feature_args = ["--features", ",".join(cargo_features)]
    # extra_cargo_args carries scope/feature flags the simple --features list
    # can't express (e.g. --all-features, -p <pkg>); the rebench corpus driver
    # passes the per-crate feature rung decoded from rebench_100crates.csv.
    base_cmd = [
        cfg.cargo, "test", "--release", "--no-run", "--no-fail-fast",
        *targets, *feature_args, *(extra_cargo_args or []),
        "--message-format=json",
    ]
    cmd = base_cmd[:1] + base_cmd[1:2] + ["--ignore-rust-version"] + base_cmd[2:]

    rc, stdout, stderr = _run_cargo_test_no_run(crate_path, env, cmd, timeout)
    if rc == -1:
        return [], f"cargo test --no-run {stderr}"

    if rc != 0 and any(h in (stdout + stderr) for h in _FALLBACK_RESOLVE_HINTS):
        # transitive dep blew up on a feature/edition our toolchain can't
        # parse. wipe Cargo.lock so the resolver re-runs with fallback
        # picking older compat versions, and retry WITHOUT --ignore-rust-version
        # (the flag silences the fallback resolver's signal).
        lock = crate_path / "Cargo.lock"
        # workspace member crates may not have their own lockfile — search up.
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
        # Keep the whole transcript. The short message returned below is
        # truncated, and a compiler crash prints its report over many lines, so
        # the useful part is exactly what truncation removes.
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
        proc_stdout = stdout
        msgs: list[str] = []
        for line in proc_stdout.splitlines():
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
    """`$UNSAFE_STAT_DIR/*.json` (default /tmp) we have permission to touch.
    Ignores files owned by other users (runners on shared boxes leave
    covsnap/getrandom snapshots around).

    Respecting UNSAFE_STAT_DIR lets parallel rebench.py invocations isolate
    their bin-emitted stat files from each other; otherwise both harvesters
    would race over the same /tmp directory and cross-attribute stats.
    """
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
    """move /tmp/*.json into out_dir prefixed with the bin name."""
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
    bin_timeout: int | None = 900,
    test_targets: list[str] | None = None,
    variants_root: Path | None = None,
    cargo_features: list[str] | None = None,
    extra_cargo_args: list[str] | None = None,
) -> FeatureResult:
    """build the matching unsafe-perf rlib if needed, then run both native-lib
    variants for this feature.

    `crate_path` is the primary crate dir (the one carrying [package]).
    `variants_root` overrides the per-variant output parent. Default is
    `out_dir/<feature>/`; rebench passes a custom root so cpu_cycle's
    per-run dirs can be placed under `out_dir/cpu_cycle_counter/runN/`
    instead of the default nested layout.
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


def run_all_features(
    crate_path: Path, cfg, out_dir: Path, label: str,
) -> BenchResult:
    """build the unsafe-perf rlib (idempotent) → run every feature → write
    summary json. no crate-under-test mutation, so no cleanup needed.

    `crate_path` MUST be the primary crate's directory (where [package]
    lives), not a workspace root — `cargo test --lib --bins ...` from a
    virtual workspace root would build all members, which is rarely what we
    want for a bench.
    """
    crate_path = Path(crate_path)
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    bench = BenchResult(label=label, crate=crate_path.name, out_dir=str(out_dir))

    if not cfg.unsafe_perf_path:
        bench.features["_error"] = FeatureResult(
            feature="_error", error="cfg.unsafe_perf_path is empty",
        )
        return bench

    for feature in FEATURE_MATRIX:
        try:
            bench.features[feature] = run_feature(
                crate_path, cfg, feature, out_dir,
            )
        except Exception as e:
            bench.features[feature] = FeatureResult(
                feature=feature, error=f"unhandled: {e}",
            )

    summary = asdict(bench)
    summary["features"] = {k: asdict(v) for k, v in bench.features.items()}
    (out_dir / "bench_summary.json").write_text(json.dumps(summary, indent=2))
    return bench
