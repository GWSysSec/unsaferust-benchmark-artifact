"""
runner.py — coverage loop orchestrator

iterates measure → (evaluate) → generate until coverage target is met
or max iterations reached or persistent errors are detected.
maintains chain-of-thought across iterations via IterationHistory.

eval stage is gated behind cfg.skip_evaluation (default true): we already
know which APIs are uncovered, so spending an opus call to score the suite
is wasted in the targeted-uncovered-API workflow.

after the loop, passing tests are persisted to recipes/<crate>/tests/ and
optionally mirrored into bootcamp/<crate>/<test_target_dir>/.
"""

import logging
import shutil
import time
from pathlib import Path

from pipeline.config import PipelineConfig
from pipeline.llm.client import LLMClient, QuotaExhausted
from pipeline.stages import (
    CrateMeasurement, TestEvaluation, GenerationResult, IterationHistory,
    measure_coverage, evaluate_test_strength, generate_tests,
)
from pipeline.tools.recipes import persist_test

log = logging.getLogger(__name__)


def run_pipeline(
    cfg: PipelineConfig,
    crate_path: Path,
    crate_name: str | None = None,
    bootcamp_dir: Path | None = None,
    recipes_dir: Path | None = None,
) -> dict:
    """run the full coverage-gated pipeline for a single crate.

    crate_path: the (likely tmp) working copy where cargo runs.
    crate_name: logical crate id (defaults to crate_path.name) — used as the
                bootcamp and recipes subdir name.
    bootcamp_dir: if set and cfg.write_to_bootcamp, mirror passing tests there.
    recipes_dir: passing tests are always archived under
                 <recipes_dir>/<crate_name>/tests/ regardless.
    """
    crate_path = Path(crate_path).resolve()
    crate_name = crate_name or crate_path.name
    log.info(f"{'='*60}")
    log.info(f"pipeline start: {crate_name}")
    log.info(f"{'='*60}")

    llm = LLMClient(cfg)

    measurement: CrateMeasurement | None = None
    evaluation: TestEvaluation | None = None
    generation_results: list[GenerationResult] = []
    consecutive_no_progress = 0
    history = IterationHistory()
    iterations_run = 0

    quota_exhausted = False
    for iteration in range(cfg.max_coverage_iterations):
        log.info(f"\n--- iteration {iteration + 1}/{cfg.max_coverage_iterations} ---")
        t0 = time.time()
        iterations_run = iteration + 1

        # stage 1: measure
        measurement = measure_coverage(cfg, llm, crate_path)
        history.record_coverage(measurement.api_coverage_pct)

        if measurement.api_coverage_pct >= cfg.coverage_target:
            log.info(f"coverage target reached: {measurement.api_coverage_pct:.1f}% "
                     f">= {cfg.coverage_target}%")
            break

        # stage 2: evaluate (gated — skip in targeted-uncovered workflow)
        if not cfg.skip_evaluation:
            try:
                evaluation = evaluate_test_strength(cfg, llm, measurement)
            except QuotaExhausted as e:
                log.warning(f"  api quota exhausted during evaluate stage ({e}) — aborting iterations")
                quota_exhausted = True
                break
            if evaluation.action == "KEEP" and \
                    measurement.api_coverage_pct >= cfg.coverage_target:
                log.info(f"evaluation says KEEP, coverage sufficient")
                break

        # stage 3: generate (with chain-of-thought history). generate_tests
        # catches QuotaExhausted internally and sets gen_result.quota_exhausted;
        # we propagate that flag up and stop iterating.
        gen_result = generate_tests(cfg, llm, crate_path, measurement, history=history)
        generation_results.append(gen_result)

        elapsed = time.time() - t0
        log.info(f"iteration {iteration + 1} completed in {elapsed:.1f}s")

        if gen_result.quota_exhausted:
            quota_exhausted = True
            log.warning("  generate_tests reported quota exhausted — stopping iterations")
            break

        # progress check: abort if no new tests generated in consecutive iterations
        if not gen_result.files_generated:
            consecutive_no_progress += 1
            log.warning(f"no new tests generated ({consecutive_no_progress} consecutive)")
            if consecutive_no_progress >= 2:
                log.error("persistent failure — no progress in 2 consecutive iterations, aborting")
                break
        else:
            consecutive_no_progress = 0

    # final measurement (post-generation snapshot)
    final = measure_coverage(cfg, llm, crate_path)

    # persist passing tests — recipes always, bootcamp if configured.
    persisted = _persist_passing_tests(
        cfg, crate_name, generation_results, crate_path, final,
        bootcamp_dir=bootcamp_dir,
        recipes_dir=recipes_dir,
    )

    summary = _build_summary(
        crate_name, final, evaluation, generation_results, history,
        iterations_run=iterations_run,
        persisted_tests=persisted,
    )
    summary["quota_exhausted"] = quota_exhausted
    log.info(f"\n{'='*60}")
    log.info(f"pipeline complete: {crate_name}")
    log.info(f"  api coverage: {final.api_coverage_pct:.1f}%")
    log.info(f"  line coverage: {final.best_coverage_pct:.1f}%")
    log.info(f"  tests generated: {sum(len(g.files_generated) for g in generation_results)}")
    log.info(f"  tests persisted: {len(persisted)}")
    log.info(f"  {llm.stats()}")
    log.info(f"{'='*60}")

    return summary


def copy_crate_to_tmp(bootcamp_dir: Path, crate_name: str, tmp_dir: Path) -> Path:
    """thin wrapper over crate_prep.copy_crate; kept for backward import compatibility."""
    from pipeline.tools.crate_prep import copy_crate
    return copy_crate(bootcamp_dir, crate_name, tmp_dir)


def _persist_passing_tests(
    cfg: PipelineConfig, crate_name: str,
    gen_results: list[GenerationResult],
    crate_path: Path, final: CrateMeasurement,
    bootcamp_dir: Path | None,
    recipes_dir: Path | None,
) -> list[str]:
    """copy each passing test file into recipes/<crate>/tests/ and (optionally)
    bootcamp/<crate>/<test_target_dir>/. dedupes across iterations by filename.
    """
    if recipes_dir is None:
        recipes_dir = Path(cfg.recipes_dir)
    recipes_dir = Path(recipes_dir)

    bootcamp_dst = None
    if cfg.write_to_bootcamp and bootcamp_dir is not None:
        bootcamp_dst = Path(bootcamp_dir)

    rel_inside = final.test_target_dir or "tests"

    seen: set[str] = set()
    persisted: list[str] = []
    for gen in gen_results:
        for path_str in gen.files_generated:
            p = Path(path_str)
            if p.name in seen:
                continue
            seen.add(p.name)
            try:
                written = persist_test(
                    p, crate_name, recipes_dir,
                    bootcamp_dst=bootcamp_dst,
                    rel_inside_crate=rel_inside,
                )
                persisted.extend(str(w) for w in written)
            except Exception as e:
                log.warning(f"  failed to persist {p.name}: {e}")
    return persisted


def _build_summary(
    crate_name: str, measurement: CrateMeasurement,
    evaluation: TestEvaluation | None, gen_results: list[GenerationResult],
    history: IterationHistory,
    iterations_run: int,
    persisted_tests: list[str],
) -> dict:
    total_generated = sum(len(g.files_generated) for g in gen_results)
    total_failed = sum(len(g.files_failed) for g in gen_results)

    return {
        "crate": crate_name,
        "api_coverage_pct": measurement.api_coverage_pct,
        "api_covered": measurement.api_covered,
        "api_total": measurement.api_total,
        "line_coverage_pct": measurement.best_coverage_pct,
        "tests_total": measurement.test_inventory.total_test_count,
        "tests_generated": total_generated,
        "tests_failed": total_failed,
        "evaluation_score": evaluation.score if evaluation else 0,
        "evaluation_action": evaluation.action if evaluation else "SKIPPED",
        "iterations": iterations_run,
        "coverage_progression": history.coverage_progression,
        "persisted_tests": persisted_tests,
        "members_profiled": list(measurement.members_profiled),
        "workspace_pathway": measurement.workspace_pathway,
        "feature_label": measurement.feature_label,
        "feature_args": list(measurement.feature_args),
        "test_target_dir": measurement.test_target_dir,
        "cargo_test_cmd": measurement.cargo_test_cmd,
        "primary_package_name": measurement.workspace.primary_package_name,
        "primary_lib_name": measurement.workspace.primary_lib_name,
    }
