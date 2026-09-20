"""
__main__.py — cli entry point for the unified pipeline

usage:
    # batch (drives the full CSV-bookkept workflow)
    python -m pipeline batch [--target-band MIN MAX] [--crate NAME] [--measure-only] ...

    # single-crate dev mode (no CSV bookkeeping)
    python -m pipeline run --crate-path /path/to/crate
    python -m pipeline run --crate-name <name> --bootcamp <dir>

`batch` is the default if no subcommand is given.
"""

import argparse
import json
import logging
import sys
from pathlib import Path


def setup_logging(verbose: bool = False):
    level = logging.DEBUG if verbose else logging.INFO
    logging.basicConfig(level=level, format="%(asctime)s %(levelname)-5s %(message)s",
                        datefmt="%H:%M:%S")


def _run_single():
    """legacy single-crate mode. argparse layered to keep this self-contained."""
    from pipeline.config import load_config
    from pipeline.runner import run_pipeline, copy_crate_to_tmp

    parser = argparse.ArgumentParser(prog="python -m pipeline run",
                                     description="single-crate dev entry point")
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--crate-path", type=str,
                      help="path to a crate (runs in-place — best for tmp copies)")
    mode.add_argument("--crate-name", type=str,
                      help="crate name from bootcamp directory")
    parser.add_argument("--bootcamp", type=str, default=None)
    parser.add_argument("--tmp", type=str, default=None)
    parser.add_argument("--config", type=str, default=None)
    parser.add_argument("--coverage-target", type=float, default=None)
    parser.add_argument("--max-iterations", type=int, default=None)
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args(sys.argv[2:])
    setup_logging(args.verbose)
    log = logging.getLogger(__name__)

    cfg = load_config(args.config)
    if args.coverage_target is not None:
        cfg.coverage_target = args.coverage_target
    if args.max_iterations is not None:
        cfg.max_coverage_iterations = args.max_iterations
    if args.bootcamp:
        cfg.bootcamp_dir = args.bootcamp
    if args.tmp:
        cfg.tmp_dir = args.tmp

    project_root = Path(__file__).parent.parent
    tmp_dir = project_root / cfg.tmp_dir
    tmp_dir.mkdir(parents=True, exist_ok=True)

    if args.crate_path:
        crate_path = Path(args.crate_path).resolve()
        if not crate_path.exists():
            log.error(f"crate path not found: {crate_path}")
            sys.exit(1)
        summary = run_pipeline(cfg, crate_path)
    else:
        bootcamp = Path(cfg.bootcamp_dir)
        if not bootcamp.exists():
            log.error(f"bootcamp directory not found: {bootcamp}")
            sys.exit(1)
        crate_path = copy_crate_to_tmp(bootcamp, args.crate_name, tmp_dir)
        summary = run_pipeline(cfg, crate_path, crate_name=args.crate_name,
                               bootcamp_dir=bootcamp,
                               recipes_dir=project_root / cfg.recipes_dir)

    print(f"\n{'='*60}\nPIPELINE RESULT\n{'='*60}")
    print(f"  crate: {summary['crate']}")
    print(f"  api%: {summary['api_coverage_pct']:.1f}  "
          f"line%: {summary['line_coverage_pct']:.1f}  "
          f"tests gen: {summary['tests_generated']}  "
          f"iters: {summary['iterations']}")

    results_path = project_root / "pipeline_results.json"
    with open(results_path, "w") as f:
        json.dump(summary, f, indent=2)
    log.info(f"results saved to {results_path}")


def main():
    # parse just the subcommand without consuming the inner args
    sub = sys.argv[1] if len(sys.argv) > 1 else "batch"

    if sub in ("-h", "--help"):
        print(__doc__)
        sys.exit(0)

    if sub == "run":
        _run_single()
    elif sub == "bootcamp":
        # bootcamp-iteration mode: walk bootcamp_dir, no CSV
        from pipeline.profile import bootcamp_main
        bootcamp_main()
    else:
        # default: batch driver. forward any args (drop the explicit "batch" if present)
        if sub == "batch":
            sys.argv.pop(1)
        from pipeline.profile import main as batch_main
        batch_main()


if __name__ == "__main__":
    main()
