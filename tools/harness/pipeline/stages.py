"""
stages.py — pipeline stages: measure, evaluate, generate

stage 1 (measure): discover workspace, api surface, test inventory, llvm-cov
stage 2 (evaluate): llm assessment of test quality
stage 3 (generate): targeted test generation for uncovered apis

supports chain-of-thought across iterations via IterationHistory,
blind generation for macro crates, and test execution in the repair loop.
"""

import json
import logging
import re
from dataclasses import dataclass, field, asdict
from pathlib import Path

from pipeline.config import PipelineConfig
from pipeline.llm.client import LLMClient, QuotaExhausted
from pipeline.llm.prompts import (
    SYSTEM_EVALUATE, SYSTEM_GENERATE, SYSTEM_FIX,
    build_evaluate_prompt, build_generation_prompt, build_fix_prompt,
    build_blind_generation_prompt,
    extract_rust_code,
)
from pipeline.tools.workspace import (
    WorkspaceInfo, discover_workspace, derive_workspace_pathway, build_member_lib_map,
)
from pipeline.tools.test_locator import TestInventory, locate_tests
from pipeline.tools.coverage import CoverageResult, run_full_coverage
from pipeline.tools.coverage_agent import debug_coverage_agentic
from pipeline.tools.api_discovery import (
    ApiSurface, FunctionRecord, discover_public_api,
    discover_workspace_member_apis, compute_api_coverage,
)
from pipeline.tools.context_builder import build_generation_context
from pipeline.tools.build_fixer import (
    compile_check, try_run_test, classify_errors, classify_runtime_errors,
    extract_missing_symbols, extract_relevant_errors,
    run_rust_analyzer_diagnostics,
    error_specific_hints, error_signature,
    quarantine_cross_compile_failures,
)
from pipeline.tools.recipes import persist_test as _persist_to_recipes

log = logging.getLogger(__name__)


def _persist_immediately(cfg: PipelineConfig, out_path: Path, crate_name: str) -> None:
    """archive a passing test to recipes/<crate>/tests/ as soon as it compiles+
    runs, so an outer per-crate timeout firing later in the iteration loop
    doesn't wipe completed work along with tmp_gen_test/. paired with
    _unpersist_quarantined for the cross-compile sweep that runs after the
    batch loop. idempotent: write_bytes overwrites if the file already exists.
    """
    try:
        _persist_to_recipes(out_path, crate_name, Path(cfg.recipes_dir))
    except Exception as e:
        log.warning(f"  failed to archive {out_path.name} to recipes: {e}")


def _unpersist_quarantined(cfg: PipelineConfig, crate_name: str, names: list[str]) -> None:
    """remove already-archived tests when the cross-compile sweep later
    quarantines them. matches by stem (the .rs filename without extension)."""
    base = Path(cfg.recipes_dir) / crate_name / "tests"
    if not base.exists():
        return
    for n in names:
        p = base / f"{n}.rs"
        try:
            if p.exists():
                p.unlink()
        except Exception as e:
            log.warning(f"  failed to remove quarantined archive {p}: {e}")


@dataclass
class CrateMeasurement:
    crate_name: str = ""
    workspace: WorkspaceInfo = field(default_factory=WorkspaceInfo)
    test_inventory: TestInventory = field(default_factory=TestInventory)
    api_surface: ApiSurface = field(default_factory=ApiSurface)
    test_coverage: CoverageResult = field(default_factory=CoverageResult)
    api_covered: int = 0
    api_total: int = 0
    api_coverage_pct: float = 0.0
    uncovered_fns: list = field(default_factory=list)
    best_coverage_pct: float = 0.0

    # recipe / bookkeeping fields populated alongside the measurement
    members_profiled: list = field(default_factory=list)
    workspace_pathway: str = ""
    feature_args: list = field(default_factory=list)
    feature_label: str = ""
    test_target_dir: str = ""
    cargo_test_cmd: str = ""


@dataclass
class TestEvaluation:
    score: int = 0
    axes: dict = field(default_factory=dict)
    reasoning: str = ""
    action: str = "GENERATE"


@dataclass
class GenerationResult:
    files_generated: list[str] = field(default_factory=list)
    files_failed: list[str] = field(default_factory=list)
    total_attempted: int = 0
    quota_exhausted: bool = False  # set when upstream returned 429 mid-batch


@dataclass
class IterationHistory:
    """accumulates chain-of-thought context across pipeline iterations."""
    previous_attempts: list[dict] = field(default_factory=list)
    failed_patterns: list[str] = field(default_factory=list)
    coverage_progression: list[float] = field(default_factory=list)
    generated_files: list[str] = field(default_factory=list)

    def record_attempt(self, test_name: str, compiled: bool, error_summary: str = ""):
        self.previous_attempts.append({
            "test_name": test_name,
            "compiled": compiled,
            "error_summary": error_summary[:200],
        })

    def record_coverage(self, api_pct: float):
        self.coverage_progression.append(api_pct)


# ---------------------------------------------------------------------------
# stage 1: measure
# ---------------------------------------------------------------------------

def measure_coverage(cfg: PipelineConfig, llm: LLMClient, crate_path: Path) -> CrateMeasurement:
    log.info(f"[stage 1] measuring coverage for {crate_path.name}...")
    m = CrateMeasurement(crate_name=crate_path.name)

    # workspace discovery
    m.workspace = discover_workspace(crate_path)
    m.workspace_pathway = derive_workspace_pathway(m.workspace, crate_path.name)

    # api surface: union of primary + extras enumerated in EXTRA_COVERAGE_PKGS.
    # using the member-aware path keeps the denominator aligned with the crates
    # we actually run llvm-cov against (msgpack-rust, serde_with, rust-cpp).
    member_records, members_profiled, pre_covered = discover_workspace_member_apis(
        m.workspace, rusttest_gen_binary=cfg.rusttest_gen_binary,
    )
    m.api_surface = ApiSurface(
        crate_name=m.workspace.primary_lib_name,
        functions=member_records,
        total=len(member_records),
        pre_covered=pre_covered,
    )
    m.members_profiled = members_profiled
    countable = m.api_surface.countable_total
    surface_msg = f"  api surface: {m.api_surface.total} public items"
    if countable != m.api_surface.total:
        surface_msg += f" ({countable} countable, traits excluded)"
    surface_msg += f" across {len(members_profiled)} member(s)"
    log.info(surface_msg)

    # test inventory
    m.test_inventory = locate_tests(m.workspace)
    log.info(f"  tests: {m.test_inventory.total_test_count} "
             f"(unit={m.test_inventory.unit_test_count}, integ={m.test_inventory.integ_test_count})")

    # llvm-cov coverage (--tests only; --lib was deprecated)
    m.test_coverage = run_full_coverage(m.workspace)
    m.best_coverage_pct = m.test_coverage.percent if m.test_coverage.ok else 0.0

    # initial api coverage: intersect llvm-cov covered functions with discovered api
    best_cov = m.test_coverage
    covered_fns = best_cov.covered_functions if best_cov.ok else set()

    m.api_covered, m.api_total, m.api_coverage_pct, uncovered_paths = \
        compute_api_coverage(m.api_surface, covered_fns)

    # fallback to agentic debugging ONLY when llvm-cov failed outright (zero line cov).
    # below-target api% is the entire reason the pipeline exists — it's not a signal
    # that coverage extraction is broken, so re-running the agent every iteration is
    # pure waste (~220s of opus each call, same answer back). workspace-layout
    # debugging is already deterministic via workspace.py + EXTRA_COVERAGE_PKGS.
    if m.best_coverage_pct == 0.0:
        error_context = (
            f"Initial run got {m.api_coverage_pct:.1f}% API coverage (target: {cfg.coverage_target}%).\n"
            f"Test coverage ok={m.test_coverage.ok}, percent={m.test_coverage.percent}%\n"
            f"Test coverage error: {m.test_coverage.error}"
        )
        agent_res = debug_coverage_agentic(llm, m.workspace, error_context, m.api_surface, cfg.coverage_target)
        if agent_res.ok:
            m.best_coverage_pct = agent_res.percent
            m.test_coverage = agent_res
            
            # recompute api coverage
            m.api_covered, m.api_total, m.api_coverage_pct, uncovered_paths = \
                compute_api_coverage(m.api_surface, agent_res.covered_functions)

    # map uncovered paths back to FunctionRecord objects
    uncovered_set = set(uncovered_paths)
    m.uncovered_fns = [fn for fn in m.api_surface.functions if fn.module_path in uncovered_set]

    log.info(f"  api coverage: {m.api_covered}/{m.api_total} ({m.api_coverage_pct:.1f}%)")
    log.info(f"  line coverage: {m.best_coverage_pct:.1f}%")

    # recipe-side fields: feature args + cargo replay command + test target dir
    cov = m.test_coverage
    m.feature_args = list(cov.feature_args) if cov.ok else []
    m.feature_label = cov.feature_label if cov.ok else ""
    ws = m.workspace
    rel_primary = (ws.primary_crate_path.relative_to(ws.path)
                   if ws.is_workspace else Path("."))
    rel_str = str(rel_primary / "tests")
    m.test_target_dir = rel_str if rel_str != "tests" else "tests"
    if ws.is_workspace:
        cmd_parts = ["cargo", "test", "--workspace"] + m.feature_args
    else:
        cmd_parts = ["cargo", "test", "-p", ws.primary_package_name] + m.feature_args
    m.cargo_test_cmd = " ".join(cmd_parts)

    return m


# ---------------------------------------------------------------------------
# stage 2: evaluate
# ---------------------------------------------------------------------------

def evaluate_test_strength(
    cfg: PipelineConfig, llm: LLMClient, measurement: CrateMeasurement,
) -> TestEvaluation:
    log.info(f"[stage 2] evaluating test strength for {measurement.crate_name}...")
    inv = measurement.test_inventory
    ws = measurement.workspace

    if not inv.has_any_tests():
        return TestEvaluation(score=0, reasoning="no tests found", action="GENERATE")

    # build api summary from api index
    api_lines = []
    for fn in measurement.api_surface.functions[:50]:
        api_lines.append(f"  {fn.module_path}: {fn.signature}")
    api_summary = "\n".join(api_lines) if api_lines else "(not available)"

    user_prompt = build_evaluate_prompt(
        crate_name=ws.name,
        package_name=ws.primary_package_name,
        is_workspace=ws.is_workspace,
        description="",
        api_summary=api_summary,
        total_api=measurement.api_total,
        covered_api=measurement.api_covered,
        api_cov_pct=measurement.api_coverage_pct,
        unit_files=inv.unit_test_files,
        unit_count=inv.unit_test_count,
        integ_files=inv.integ_test_files,
        integ_count=inv.integ_test_count,
        bench_files=inv.bench_files,
        total_tests=inv.total_test_count,
        test_cov_pct=measurement.test_coverage.percent if measurement.test_coverage.ok else 0.0,
        lib_cov_pct=0.0,  # --lib coverage path was deprecated
        unit_source=inv.unit_test_source[:8000],
        integ_source=inv.integ_test_source[:8000],
    )

    raw = llm.call("brain", SYSTEM_EVALUATE, user_prompt)

    try:
        cleaned = raw.replace("```json", "").replace("```", "").strip()
        data = json.loads(cleaned)
        score = int(data.get("score", 0))
        axes = data.get("axes", {})
        reasoning = data.get("reasoning", str(data))
        action = data.get("action", "GENERATE")

        if score >= 70:
            action = "KEEP"
        elif score >= 40:
            action = "AUGMENT"
        else:
            action = "GENERATE"

        log.info(f"  evaluation: score={score}, action={action}")
        return TestEvaluation(score=score, axes=axes, reasoning=reasoning, action=action)

    except (json.JSONDecodeError, Exception) as e:
        log.warning(f"  evaluation parse error: {e}")
        return TestEvaluation(score=0, reasoning=f"parse error: {e}", action="GENERATE")


# ---------------------------------------------------------------------------
# stage 3: generate
# ---------------------------------------------------------------------------

def generate_tests(
    cfg: PipelineConfig, llm: LLMClient,
    crate_path: Path, measurement: CrateMeasurement,
    history: IterationHistory | None = None,
) -> GenerationResult:
    log.info(f"[stage 3] generating tests for {measurement.crate_name} "
             f"({len(measurement.uncovered_fns)} uncovered functions)...")

    result = GenerationResult()
    ws = measurement.workspace

    # workspace-aware paths: when the primary crate is a subdirectory of the
    # workspace root (e.g., ouroboros/ouroboros), tests must go in the subcrate
    # and cargo needs -p. when root has [package] and primary == root (e.g., h2),
    # tests at root work fine without -p.
    needs_subcrate = (ws.is_workspace
                      and ws.primary_crate_path.resolve() != ws.path.resolve())
    test_crate_path = str(ws.primary_crate_path) if needs_subcrate else None
    package_name = ws.primary_package_name if needs_subcrate else None

    # workspace-member routing: tests targeting non-primary members (e.g.
    # msgpack-rust's `rmpv::*`) must land in that member's `tests/` dir with
    # `-p <member>`, not the primary's. without this they fail per-iteration
    # with E0432 `unresolved import` because the primary member doesn't depend
    # on the sibling crate. empty for single-crate setups.
    member_lib_map = build_member_lib_map(ws) if ws.is_workspace else {}

    # blind generation path: when API discovery found nothing but the crate
    # clearly has an API (tests exist or evaluation says GENERATE)
    if not measurement.uncovered_fns and measurement.api_surface.total == 0:
        return _blind_generate(cfg, llm, crate_path, measurement, history, result,
                               test_crate_path=test_crate_path, package_name=package_name)

    if not measurement.uncovered_fns:
        log.info("  no uncovered functions to target")
        return result

    # build generation context — use primary crate path for context
    ctx = build_generation_context(
        ws.primary_crate_path, measurement.uncovered_fns,
        rusttest_gen_binary=cfg.rusttest_gen_binary,
    )

    # group uncovered functions into batches, capped at max_batches_per_iteration
    # for crates with 100+ APIs. unfocused iteration would burn opus calls on
    # low-leverage tail modules; the cap forces us to land big-impact modules
    # first and let the next iteration tackle the rest.
    cap = getattr(cfg, "max_batches_per_iteration", 5) or None
    batches = _group_by_module(measurement.uncovered_fns, max_batches=cap)

    # within a single iteration, dedupe APIs we just generated tests for in
    # earlier batches — saves the LLM from re-targeting fns it already covered
    # in this same iteration. cheap proxy for re-running cargo llvm-cov.
    tentatively_covered: set[str] = set()

    for batch_name, fns in batches.items():
        # filter out fns covered by an earlier batch in THIS iteration; if the
        # whole batch is already covered, skip without paying the LLM call.
        fns = [fn for fn in fns if fn.module_path not in tentatively_covered]
        if not fns:
            log.info(f"  skipping batch {batch_name} — all targets covered earlier this iteration")
            continue

        result.total_attempted += 1
        slug = batch_name.replace("::", "_")
        test_name = f"gen_{slug}"

        # route this batch to the workspace member that owns its target lib.
        # batch_name's first segment is the lib_name; if it matches a member
        # other than the primary, override the write root and `-p` package.
        # default to the primary-crate paths otherwise.
        batch_lib = batch_name.split("::", 1)[0]
        batch_test_crate_path = test_crate_path
        batch_package_name = package_name
        if member_lib_map and batch_lib in member_lib_map:
            mp, mpkg = member_lib_map[batch_lib]
            if mp.resolve() != ws.primary_crate_path.resolve():
                batch_test_crate_path = str(mp)
                batch_package_name = mpkg
                log.info(f"  routing {batch_name} to member {mpkg} at {mp}")

        # a prior iteration already wrote a test file under this slug, but the
        # llvm-cov measure that produced this iteration's `uncovered_fns` says
        # there's still work in this module — generate a continuation
        # `_part<N>.rs` instead of skipping. without this, rayon-core-style
        # crates stall: all top-level type-batches are written in iter1, then
        # iter3+ skip everything despite real uncovered methods remaining.
        if history and test_name in history.generated_files:
            base = test_name
            m = re.match(r'^(.*)_part(\d+)$', base)
            start_n = int(m.group(2)) + 1 if m else 2
            if m:
                base = m.group(1)
            n = start_n
            while f"{base}_part{n}" in history.generated_files:
                n += 1
            new_name = f"{base}_part{n}"
            log.info(f"  {test_name} already generated — continuing as {new_name}")
            test_name = new_name

        log.info(f"  generating batch: {batch_name} ({len(fns)} functions)")

        # generate test code (brain)
        system = SYSTEM_GENERATE.replace("{crate_name}", ctx.crate_name)
        user_prompt = build_generation_prompt(
            crate_name=ctx.crate_name,
            description=ctx.crate_description,
            api_index=ctx.api_index,
            uncovered_fns=fns,
            existing_tests=ctx.existing_test_files,
            history=history.previous_attempts if history else None,
        )

        try:
            raw = llm.call("brain", system, user_prompt)
        except QuotaExhausted as e:
            log.warning(f"  [stage 3] api quota exhausted before batch {batch_name} — "
                        f"aborting generation for this crate ({e})")
            result.quota_exhausted = True
            break
        code = extract_rust_code(raw)

        if not code.strip():
            log.warning(f"  llm returned empty code for {batch_name}")
            result.files_failed.append(test_name)
            if history:
                history.record_attempt(test_name, False, "llm returned empty code")
            continue

        # compile check + test execution + repair loop (fixer)
        try:
            code, ok = _repair_loop(cfg, llm, crate_path, code, test_name, ctx, fns,
                                    test_crate_path=batch_test_crate_path,
                                    package_name=batch_package_name,
                                    feature_args=measurement.feature_args)
        except QuotaExhausted as e:
            log.warning(f"  [stage 3] api quota exhausted during repair of {test_name} — "
                        f"aborting generation for this crate ({e})")
            result.quota_exhausted = True
            break

        if ok:
            # write the passing test file to the per-batch member's tests/
            # (falls back to primary / single-crate root when no routing fired).
            write_root = Path(batch_test_crate_path) if batch_test_crate_path else crate_path
            out_path = write_root / "tests" / f"{test_name}.rs"
            out_path.parent.mkdir(parents=True, exist_ok=True)
            out_path.write_text(code)
            result.files_generated.append(str(out_path))
            _persist_immediately(cfg, out_path, crate_path.name)
            # mark this batch's fns covered for the rest of THIS iteration only.
            # the next iteration will re-measure for ground truth.
            for fn in fns:
                tentatively_covered.add(fn.module_path)
            if history:
                history.generated_files.append(test_name)
                history.record_attempt(test_name, True)
            log.info(f"  -> wrote {out_path.name}")
        else:
            result.files_failed.append(test_name)
            if history:
                history.record_attempt(test_name, False, "repair loop exhausted")
            log.warning(f"  -> failed to compile {test_name}")

    # cross-test quarantine sweep — even with feature-aware repair, two
    # generated tests can still conflict at the workspace build level (mod
    # name collisions, feature-gated symbol resolution that differs when other
    # bins exist, etc). drop the offenders so the next coverage measure sees
    # a clean compile. when batches are routed across workspace members, run
    # one sweep per (member tests/ dir, package_name) group so the sweep's
    # `-p` and tests_dir match the files it's responsible for.
    if result.files_generated and measurement.feature_args is not None:
        # group generated files by their tests/ parent — that's the member root
        groups: dict[tuple[str, str | None], set[str]] = {}
        for path_str in result.files_generated:
            p = Path(path_str)
            member_root = str(p.parent.parent)
            # infer package name: primary's package if this is the primary
            # tests/ dir, else look up from member_lib_map by member path.
            pkg = package_name
            if member_lib_map:
                for lib, (mp, mpkg) in member_lib_map.items():
                    if str(mp.resolve()) == str(Path(member_root).resolve()):
                        pkg = mpkg
                        break
            groups.setdefault((member_root, pkg), set()).add(p.stem)

        all_qed: list[str] = []
        for (member_root, pkg), eligible in groups.items():
            try:
                qed, qlog = quarantine_cross_compile_failures(
                    cfg, str(crate_path),
                    test_crate_path=member_root if member_root != str(crate_path) else None,
                    package_name=pkg,
                    feature_args=measurement.feature_args,
                    eligible_test_names=eligible,
                )
            except Exception as e:
                log.warning(f"  cross-compile sweep raised: {e}")
                qed, qlog = [], []
            for line in qlog:
                log.warning(f"  {line}")
            all_qed.extend(qed)
        qed = all_qed
        if qed:
            qed_set = set(qed)
            kept = [p for p in result.files_generated if Path(p).stem not in qed_set]
            result.files_generated = kept
            result.files_failed.extend(qed)
            _unpersist_quarantined(cfg, crate_path.name, qed)
            if history:
                for name in qed:
                    history.record_attempt(name, False, "quarantined: cross-compile failure")
                    if name in history.generated_files:
                        history.generated_files.remove(name)

    log.info(f"  generation complete: {len(result.files_generated)} ok, "
             f"{len(result.files_failed)} failed"
             + (" (QUOTA_EXHAUSTED)" if result.quota_exhausted else ""))
    return result


def _blind_generate(
    cfg: PipelineConfig, llm: LLMClient,
    crate_path: Path, measurement: CrateMeasurement,
    history: IterationHistory | None,
    result: GenerationResult,
    test_crate_path: str = None,
    package_name: str = None,
) -> GenerationResult:
    """generate tests for crates where API discovery found nothing (macro crates).

    uses existing test source and README as the generation seed.
    """
    log.info("  blind generation mode: api surface is empty, using existing tests as seed")
    inv = measurement.test_inventory
    ws = measurement.workspace

    # combine available test source
    test_source = ""
    if inv.integ_test_source:
        test_source = inv.integ_test_source
    elif inv.unit_test_source:
        test_source = inv.unit_test_source

    if not test_source:
        log.warning("  no existing test source to seed blind generation")
        return result

    # read README for context
    readme = ""
    for name in ("README.md", "README.rst", "README.txt", "README"):
        p = ws.primary_crate_path / name
        if not p.exists():
            p = ws.path / name
        if p.exists():
            readme = p.read_text(encoding="utf-8", errors="ignore")[:2000]
            break

    result.total_attempted += 1
    test_name = f"gen_{ws.primary_package_name.replace('-', '_')}_blind"

    if history and test_name in history.generated_files:
        log.info(f"  skipping {test_name} (already generated)")
        return result

    system = SYSTEM_GENERATE.replace("{crate_name}", ws.primary_package_name)
    user_prompt = build_blind_generation_prompt(
        crate_name=ws.primary_package_name,
        description="",
        existing_test_source=test_source,
        readme_summary=readme,
        history=history.previous_attempts if history else None,
    )

    try:
        raw = llm.call("brain", system, user_prompt)
    except QuotaExhausted as e:
        log.warning(f"  [stage 3] api quota exhausted during blind generation ({e})")
        result.quota_exhausted = True
        return result
    code = extract_rust_code(raw)

    if not code.strip():
        log.warning("  llm returned empty code for blind generation")
        result.files_failed.append(test_name)
        if history:
            history.record_attempt(test_name, False, "llm returned empty code")
        return result

    # build minimal context for repair loop
    ctx = build_generation_context(
        ws.primary_crate_path, [],
        rusttest_gen_binary=cfg.rusttest_gen_binary,
    )

    try:
        code, ok = _repair_loop(cfg, llm, crate_path, code, test_name, ctx, [],
                                test_crate_path=test_crate_path, package_name=package_name,
                                feature_args=measurement.feature_args)
    except QuotaExhausted as e:
        log.warning(f"  [stage 3] api quota exhausted during blind repair ({e})")
        result.quota_exhausted = True
        return result

    if ok:
        write_root = Path(test_crate_path) if test_crate_path else crate_path
        out_path = write_root / "tests" / f"{test_name}.rs"
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(code)
        result.files_generated.append(str(out_path))
        _persist_immediately(cfg, out_path, crate_path.name)
        if history:
            history.generated_files.append(test_name)
            history.record_attempt(test_name, True)
        log.info(f"  -> wrote {out_path.name}")

        # cross-test quarantine sweep — see generate_tests for rationale.
        if measurement.feature_args is not None:
            try:
                qed, qlog = quarantine_cross_compile_failures(
                    cfg, str(crate_path),
                    test_crate_path=test_crate_path,
                    package_name=package_name,
                    feature_args=measurement.feature_args,
                    eligible_test_names={test_name},
                )
            except Exception as e:
                log.warning(f"  cross-compile sweep raised: {e}")
                qed, qlog = [], []
            for line in qlog:
                log.warning(f"  {line}")
            if qed:
                qed_set = set(qed)
                result.files_generated = [p for p in result.files_generated
                                          if Path(p).stem not in qed_set]
                result.files_failed.extend(qed)
                _unpersist_quarantined(cfg, crate_path.name, qed)
                if history:
                    for name in qed:
                        history.record_attempt(name, False, "quarantined: cross-compile failure")
                        if name in history.generated_files:
                            history.generated_files.remove(name)
    else:
        result.files_failed.append(test_name)
        if history:
            history.record_attempt(test_name, False, "repair loop exhausted")
        log.warning(f"  -> failed to compile {test_name}")

    return result


def _group_by_module(
    fns: list[FunctionRecord],
    max_per_batch: int = 15,
    max_batches: int | None = None,
) -> dict[str, list]:
    """group functions by parent module/struct for batched generation.

    sorts groups by uncovered count desc so high-impact modules go first.
    if max_batches is set, truncates after that many groups (the rest carry
    over to subsequent iterations).
    """
    modules: dict[str, list] = {}
    for fn in fns:
        parts = fn.module_path.split("::")
        parent = "::".join(parts[:-1]) if len(parts) > 1 else parts[0]
        modules.setdefault(parent, []).append(fn)

    # split large groups, preserving the parent ordering for stable sort.
    expanded: list[tuple[str, list]] = []
    for mod_name, mod_fns in modules.items():
        if len(mod_fns) <= max_per_batch:
            expanded.append((mod_name, mod_fns))
        else:
            for i in range(0, len(mod_fns), max_per_batch):
                chunk = mod_fns[i:i + max_per_batch]
                suffix = f"_part{i // max_per_batch + 1}"
                expanded.append((f"{mod_name}{suffix}", chunk))

    # prioritize big modules — bigger uncovered batches are higher leverage
    # for crates with 100+ APIs where we can't afford to hit every group.
    expanded.sort(key=lambda kv: -len(kv[1]))

    if max_batches is not None and max_batches > 0:
        expanded = expanded[:max_batches]

    return dict(expanded)


def _repair_loop(
    cfg: PipelineConfig, llm: LLMClient,
    crate_path: Path, code: str, test_name: str,
    ctx, fns: list,
    test_crate_path: str = None,
    package_name: str = None,
    feature_args: list[str] | None = None,
) -> tuple[str, bool]:
    """compile check → test execution → fix loop.

    test_crate_path: where to place the test file (primary subcrate for workspaces).
    package_name: -p flag for workspace crates.
    feature_args: cargo feature flags matching the coverage step — required so
                  the repair loop sees the same compile context as `cargo
                  llvm-cov --tests` and catches feature-gated lifetime/borrow
                  failures at gen time (see jni postmortem).
    """
    error_history = []
    test_file_name = f"{test_name}.rs"

    for attempt in range(cfg.max_repair_attempts + 1):
        # step 1: compile check
        ok, errors = compile_check(cfg, str(crate_path), code, test_name,
                                   test_crate_path=test_crate_path,
                                   package_name=package_name,
                                   feature_args=feature_args)
        if not ok:
            # filter errors to only relevant ones from our test file
            filtered_errors = extract_relevant_errors(errors, test_file_name)
            err_lines = [l for l in filtered_errors.split("\n") if "error" in l.lower()][:3]
            short = "; ".join(err_lines)[:120]
            log.warning(f"    compile failed (attempt {attempt + 1}): {short}")

            # systemic error check
            is_systemic, reason = classify_errors(errors)
            if is_systemic:
                if "cargo fetch error" in reason:
                    log.warning("    systemic cargo error detected, attempting automatic cargo update...")
                    import subprocess
                    subprocess.run([cfg.cargo, "update"], cwd=crate_path, capture_output=True, timeout=120)
                    ok, retry_errors = compile_check(cfg, str(crate_path), code, test_name,
                                                     test_crate_path=test_crate_path,
                                                     package_name=package_name,
                                                     feature_args=feature_args)
                    if ok:
                        log.info(f"    cargo update fixed the error on attempt {attempt + 1}")
                        # fall through to test execution below
                    else:
                        log.warning("    cargo update did not fix the error — skipping")
                        return code, False
                else:
                    log.warning(f"    systemic error — skipping: {reason}")
                    return code, False

            if not ok:
                # loop detection
                sig = error_signature(errors)
                if sig and sig in error_history:
                    log.warning(f"    repeated error detected — aborting repair")
                    return code, False
                error_history.append(sig)

                if attempt < cfg.max_repair_attempts:
                    # build api summary for fix context
                    api_lines = [f"  {e['name']}: {e['signature']}" for e in ctx.api_index if e.get("signature")]
                    api_summary = "\n".join(api_lines)

                    hints = error_specific_hints(errors)

                    # optionally get RA diagnostics for precise error info
                    ra_diag = ""
                    try:
                        ra_diag = run_rust_analyzer_diagnostics(str(crate_path), test_file_name)
                    except Exception:
                        pass

                    fix_prompt = build_fix_prompt(
                        crate_name=ctx.crate_name,
                        code=code, errors=filtered_errors,
                        api_summary=api_summary or ctx.ra_api_summary,
                        hints=hints,
                        ra_diagnostics=ra_diag,
                    )

                    raw = llm.call("fixer", SYSTEM_FIX, fix_prompt)
                    new_code = extract_rust_code(raw)

                    if not new_code.strip():
                        log.warning(f"    fixer returned empty — aborting")
                        return code, False
                    code = new_code
                continue

        # step 2: test execution (compile succeeded, now run the test)
        log.info(f"    compiled on attempt {attempt + 1}, running test...")
        run_ok, run_output = try_run_test(cfg, str(crate_path), code, test_name,
                                          test_crate_path=test_crate_path,
                                          package_name=package_name,
                                          feature_args=feature_args)

        if run_ok:
            log.info(f"    test passed on attempt {attempt + 1}")
            return code, True

        # test failed at runtime
        is_deadlock, failure_type = classify_runtime_errors(run_output)
        if is_deadlock and failure_type == "deadlock_or_hang":
            log.warning(f"    test deadlocked/hung (attempt {attempt + 1})")
            err_context = "TEST DEADLOCK/HANG: The test timed out, likely due to a deadlock or infinite loop."
        elif is_deadlock and failure_type == "crash":
            log.warning(f"    test crashed (attempt {attempt + 1})")
            err_context = f"TEST CRASH: {run_output[-500:]}"
        else:
            log.warning(f"    test failed at runtime (attempt {attempt + 1})")
            err_context = f"RUNTIME ERROR:\n{run_output[-2000:]}"

        # loop detection for runtime errors
        sig = error_signature(run_output)
        if sig and sig in error_history:
            log.warning(f"    repeated runtime error — aborting repair")
            return code, False
        error_history.append(sig)

        if attempt < cfg.max_repair_attempts:
            api_lines = [f"  {e['name']}: {e['signature']}" for e in ctx.api_index if e.get("signature")]
            api_summary = "\n".join(api_lines)

            fix_prompt = build_fix_prompt(
                crate_name=ctx.crate_name,
                code=code, errors=err_context,
                api_summary=api_summary or ctx.ra_api_summary,
                hints="The test compiled but failed at runtime. Fix the logic, not the imports.",
            )

            raw = llm.call("fixer", SYSTEM_FIX, fix_prompt)
            new_code = extract_rust_code(raw)

            if not new_code.strip():
                log.warning(f"    fixer returned empty — aborting")
                return code, False
            code = new_code

    return code, False
