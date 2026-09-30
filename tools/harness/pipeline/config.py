"""
config.py — pipeline configuration loader

loads config.yaml, merges with env var overrides.
"""

import os
import yaml
from dataclasses import dataclass
from pathlib import Path


@dataclass
class PipelineConfig:
    # toolchain
    rustc: str = ""
    cargo: str = ""

    # paths
    bootcamp_dir: str = ""
    csv_path: str = ""
    recipes_dir: str = "recipes"

    # rusttest-gen binary for rust-analyzer based API discovery
    rusttest_gen_binary: str = ""

    # the unsafe-perf runtime crate linked into every instrumented test binary
    unsafe_perf_path: str = ""


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

        pipe = raw.get("pipeline", {})
        cfg.bootcamp_dir = pipe.get("bootcamp_dir", cfg.bootcamp_dir)
        cfg.csv_path = pipe.get("csv_path", cfg.csv_path)
        cfg.recipes_dir = pipe.get("recipes_dir", cfg.recipes_dir)

        bench = raw.get("bench_runtime", {})
        cfg.unsafe_perf_path = bench.get("unsafe_perf_path", cfg.unsafe_perf_path)

    if os.environ.get("RUSTC"):
        cfg.rustc = os.environ["RUSTC"]
    if os.environ.get("CARGO"):
        cfg.cargo = os.environ["CARGO"]

    if not cfg.cargo:
        cfg.cargo = "cargo"
    if not cfg.rustc:
        cfg.rustc = "rustc"

    project_root = Path(__file__).parent.parent
    for profile in ("release", "debug"):
        candidate = project_root / "target" / profile / "rusttest-gen"
        if candidate.is_file() and os.access(candidate, os.X_OK):
            cfg.rusttest_gen_binary = str(candidate)
            break

    return cfg
