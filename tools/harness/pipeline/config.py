"""
config.py — pipeline configuration loader

loads config.yaml, merges with env var overrides.
supports anthropic-native sdk with opus/sonnet model routing.
"""

import os
import csv
import yaml
from dataclasses import dataclass, field
from pathlib import Path


@dataclass
class ModelConfig:
    model: str = ""
    temperature: float = 0.1
    max_tokens: int = 4096


@dataclass
class PipelineConfig:
    # toolchain
    rustc: str = ""
    cargo: str = ""

    # api (anthropic-native)
    api_key: str = ""
    base_url: str = "https://api.anthropic.com"
    protocol: str = "anthropic"

    # model routing: brain (opus) for analysis/generation, fixer (sonnet) for repairs
    model_brain: ModelConfig = field(default_factory=lambda: ModelConfig(
        model="claude-opus-4-7", temperature=0.3, max_tokens=8192
    ))
    model_fixer: ModelConfig = field(default_factory=lambda: ModelConfig(
        model="claude-sonnet-4-6", temperature=0.1, max_tokens=4096
    ))

    # pipeline behavior
    max_repair_attempts: int = 3
    max_coverage_iterations: int = 5
    coverage_target: float = 80.0
    test_output_dir: str = "tests"
    tmp_dir: str = "tmp_gen_test"

    # focus controls (for crates with 100+ APIs)
    # cap how many module-batches we generate per iteration; sorted by uncovered
    # count descending so high-impact modules go first.
    max_batches_per_iteration: int = 5

    # generation gating: only run pipeline on crates whose pre-gen api% sits in
    # this band (defaults match current focus: pull 60-70% crates upward).
    target_band_min: float = 60.0
    target_band_max: float = 70.0

    # llm gating
    skip_evaluation: bool = True   # drop the brain-eval call by default
    skip_description: bool = True  # don't generate the test-suite description

    # platform constraints for test generation
    platform_os: str = "Ubuntu 22.04"
    platform_arch: str = "x86_64"
    platform_ram_gb: int = 32
    test_timeout: int = 60
    max_test_threads: int = 4

    # paths
    bootcamp_dir: str = "/home/oscar/lab/unsafe-dyn-rust-expr/static-analysis/bootcamp"
    crate_list_csv: str = ""
    csv_path: str = "Final Crates - Sheet6_updated.csv"
    recipes_dir: str = "recipes"
    # whether successfully generated tests get mirrored back into bootcamp_dir.
    # off by default — generated tests always land in recipes/<crate>/tests/;
    # opt in here to also commit them into the source tree.
    write_to_bootcamp: bool = True

    # batch driver
    per_crate_timeout_s: int = 3600  # 60 min hard cap per crate end-to-end
    checkpoint_path: str = ".pipeline_checkpoint.json"

    # ra analysis (kept for bug-fixing context)
    rusttest_gen_binary: str = ""

    # runtime benchmarking (verify-per-feature.sh analogue, applied to cargo test bins)
    # path to the unsafe-perf crate (provides ctor/global-allocator hooks + the
    # symbols emitted by the InstMarker / HeapTracker / etc. LLVM passes).
    unsafe_perf_path: str = ""
    # default off — enable via --bench-runtime in profile.py.
    bench_runtime: bool = False
    # absolute dir to dump per-feature stat-file snapshots into. when empty,
    # falls back to <recipes_dir>/<crate>/runtime/{before,after}/.
    bench_runtime_out_dir: str = ""
    # only run the after-bench when the post-gen api% reaches this threshold;
    # set to 0 to bench every crate regardless of generation outcome.
    bench_runtime_min_api_pct: float = 0.0


def load_config(config_path: str = None) -> PipelineConfig:
    cfg = PipelineConfig()

    if config_path is None:
        candidates = [
            Path.cwd() / "config.yaml",
            Path(__file__).parent.parent / "config.yaml",
        ]
        for c in candidates:
            if c.exists():
                config_path = str(c)
                break

    if config_path and Path(config_path).exists():
        with open(config_path) as f:
            raw = yaml.safe_load(f)

        tc = raw.get("toolchain", {})
        cfg.rustc = tc.get("rustc", "")
        cfg.cargo = tc.get("cargo", "")

        api = raw.get("api", {})
        cfg.api_key = api.get("api_key", cfg.api_key)
        cfg.base_url = api.get("base_url", cfg.base_url)
        cfg.protocol = api.get("protocol", cfg.protocol)

        models = raw.get("models", {})
        if "brain" in models:
            m = models["brain"]
            cfg.model_brain = ModelConfig(
                model=m.get("model", "claude-opus-4-7"),
                temperature=m.get("temperature", 0.3),
                max_tokens=m.get("max_tokens", 8192),
            )
        if "fixer" in models:
            m = models["fixer"]
            cfg.model_fixer = ModelConfig(
                model=m.get("model", "claude-sonnet-4-6"),
                temperature=m.get("temperature", 0.1),
                max_tokens=m.get("max_tokens", 4096),
            )

        pipe = raw.get("pipeline", {})
        cfg.max_repair_attempts = pipe.get("max_repair_attempts", cfg.max_repair_attempts)
        cfg.max_coverage_iterations = pipe.get("max_coverage_iterations", cfg.max_coverage_iterations)
        cfg.coverage_target = pipe.get("coverage_target", cfg.coverage_target)
        cfg.test_output_dir = pipe.get("test_output_dir", cfg.test_output_dir)
        cfg.tmp_dir = pipe.get("tmp_dir", cfg.tmp_dir)
        cfg.bootcamp_dir = pipe.get("bootcamp_dir", cfg.bootcamp_dir)
        cfg.crate_list_csv = pipe.get("crate_list_csv", cfg.crate_list_csv)
        cfg.csv_path = pipe.get("csv_path", cfg.csv_path)
        cfg.recipes_dir = pipe.get("recipes_dir", cfg.recipes_dir)
        cfg.write_to_bootcamp = pipe.get("write_to_bootcamp", cfg.write_to_bootcamp)
        cfg.skip_evaluation = pipe.get("skip_evaluation", cfg.skip_evaluation)
        cfg.skip_description = pipe.get("skip_description", cfg.skip_description)
        cfg.max_batches_per_iteration = pipe.get(
            "max_batches_per_iteration", cfg.max_batches_per_iteration)
        cfg.target_band_min = pipe.get("target_band_min", cfg.target_band_min)
        cfg.target_band_max = pipe.get("target_band_max", cfg.target_band_max)
        cfg.per_crate_timeout_s = pipe.get("per_crate_timeout_s", cfg.per_crate_timeout_s)
        cfg.checkpoint_path = pipe.get("checkpoint_path", cfg.checkpoint_path)

        bench = raw.get("bench_runtime", {})
        cfg.unsafe_perf_path = bench.get("unsafe_perf_path", cfg.unsafe_perf_path)
        cfg.bench_runtime = bench.get("enabled", cfg.bench_runtime)
        cfg.bench_runtime_out_dir = bench.get("out_dir", cfg.bench_runtime_out_dir)
        cfg.bench_runtime_min_api_pct = bench.get(
            "min_api_pct", cfg.bench_runtime_min_api_pct)

    # env var overrides
    cfg.api_key = os.environ.get("ANTHROPIC_API_KEY", cfg.api_key)
    cfg.base_url = os.environ.get("ANTHROPIC_BASE_URL", cfg.base_url)
    if os.environ.get("RUSTC"):
        cfg.rustc = os.environ["RUSTC"]
    if os.environ.get("CARGO"):
        cfg.cargo = os.environ["CARGO"]

    if not cfg.cargo:
        cfg.cargo = "cargo"
    if not cfg.rustc:
        cfg.rustc = "rustc"

    # find rusttest-gen binary for RA-based analysis
    project_root = Path(__file__).parent.parent
    for profile in ("release", "debug"):
        candidate = project_root / "target" / profile / "rusttest-gen"
        if candidate.is_file() and os.access(candidate, os.X_OK):
            cfg.rusttest_gen_binary = str(candidate)
            break

    return cfg


def cargo_env(cfg: PipelineConfig) -> dict:
    """build environment dict for subprocess calls to cargo."""
    env = os.environ.copy()
    env["RUSTC"] = cfg.rustc

    rustc_path = Path(cfg.rustc)
    if rustc_path.is_absolute():
        lib_dir = rustc_path.parent.parent / "lib" / "rustlib" / "x86_64-unknown-linux-gnu" / "bin"
        cov = lib_dir / "llvm-cov"
        prof = lib_dir / "llvm-profdata"
        if cov.exists():
            env["LLVM_COV"] = str(cov)
        if prof.exists():
            env["LLVM_PROFDATA"] = str(prof)

    return env


def load_crate_list(csv_path: str) -> list[dict]:
    """load the 95-crate seed dataset from CSV."""
    crates = []
    with open(csv_path, newline="", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        for row in reader:
            name = row.get("name", "").strip()
            if not name:
                continue
            crates.append({
                "name": name,
                "repository": row.get("repository", "").strip(),
                "loc": row.get("loc", "").strip(),
                "unsafe_pct": row.get("unsafe%", "").strip(),
                "type": row.get("Type", "").strip(),
                "workspace_pathway": row.get("Workspace pathway", "").strip(),
                "lib_or_app": row.get("lib_or_app", "").strip(),
            })
    return crates
