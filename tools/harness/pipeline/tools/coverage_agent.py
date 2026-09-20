"""
coverage_agent.py — agentic debugging loop for cargo llvm-cov

shares workspace discovery, api surface, and cargo tools with the llm agent
so it can reason about complex workspace structures.
"""

import json
import logging
import os
import subprocess
from dataclasses import asdict
from pathlib import Path

from pipeline.llm.client import LLMClient
from pipeline.tools.workspace import WorkspaceInfo, resolve_workspace_members
from pipeline.tools.coverage import CoverageResult, _extract_covered_functions, _extract_line_summary
from pipeline.tools.api_discovery import ApiSurface, compute_api_coverage

log = logging.getLogger(__name__)

AGENT_TOOLS = [
    {
        "type": "function",
        "function": {
            "name": "run_cargo_llvm_cov",
            "description": "Run cargo llvm-cov with specific arguments to get code coverage.",
            "parameters": {
                "type": "object",
                "properties": {
                    "args": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Arguments to pass to cargo llvm-cov. e.g. ['-p', 'ouroboros_examples', '--tests', '--all-features']"
                    }
                },
                "required": ["args"]
            }
        }
    },
    {
        "type": "function",
        "function": {
            "name": "list_directory",
            "description": "List the contents of a directory inside the crate workspace.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to list, relative to the crate root (e.g. '.', 'tests')"}
                }
            }
        }
    },
    {
        "type": "function",
        "function": {
            "name": "read_file",
            "description": "Read the contents of a file inside the crate workspace.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to the file to read (e.g. 'Cargo.toml')"}
                },
                "required": ["path"]
            }
        }
    },
    {
        "type": "function",
        "function": {
            "name": "get_workspace_info",
            "description": "Get the pre-discovered workspace structure including primary crate, workspace members, test subcrates, and extra coverage packages. Use this FIRST to understand the workspace layout.",
            "parameters": {
                "type": "object",
                "properties": {}
            }
        }
    },
    {
        "type": "function",
        "function": {
            "name": "get_api_surface",
            "description": "Get the discovered public API surface of the crate. Returns function names, signatures, and coverage status.",
            "parameters": {
                "type": "object",
                "properties": {}
            }
        }
    },
    {
        "type": "function",
        "function": {
            "name": "run_cargo_command",
            "description": "Run an arbitrary cargo subcommand (e.g. 'cargo metadata', 'cargo doc'). Useful for investigating build issues.",
            "parameters": {
                "type": "object",
                "properties": {
                    "args": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Arguments to pass to cargo. e.g. ['metadata', '--format-version=1'] or ['doc', '--no-deps']"
                    }
                },
                "required": ["args"]
            }
        }
    },
]


def debug_coverage_agentic(llm: LLMClient, ws: WorkspaceInfo, error_context: str, api_surface: ApiSurface, target_pct: float) -> CoverageResult:
    # serialize workspace info for the agent
    ws_info = {
        "name": ws.name,
        "path": str(ws.path),
        "is_workspace": ws.is_workspace,
        "primary_crate_path": str(ws.primary_crate_path),
        "primary_package_name": ws.primary_package_name,
        "workspace_members": ws.workspace_members,
        "test_subcrates": ws.test_subcrates,
        "extra_coverage_pkgs": ws.extra_coverage_pkgs,
    }

    # resolve actual member directories
    resolved_members = []
    if ws.is_workspace:
        for mp in resolve_workspace_members(ws):
            resolved_members.append(str(mp.relative_to(ws.path)))

    system_prompt = f"""You are an expert compiler engineer debugging cargo llvm-cov coverage extraction for a Rust crate.

## Workspace Structure (pre-discovered)
- Name: {ws.name}
- Type: {"WORKSPACE" if ws.is_workspace else "SINGLE CRATE"}
- Primary package: {ws.primary_package_name} at {ws.primary_crate_path}
- Workspace members: {ws.workspace_members}
- Resolved member dirs: {resolved_members}
- Test subcrates: {ws.test_subcrates}
- Extra coverage packages: {ws.extra_coverage_pkgs}

## API Surface
- Total discovered API functions: {api_surface.total}
- Coverage target: {target_pct}%

You have tools to:
1. get_workspace_info — get full workspace structure details
2. get_api_surface — get the API function list
3. run_cargo_llvm_cov — run coverage with custom arguments
4. run_cargo_command — run arbitrary cargo subcommands
5. list_directory / read_file — inspect workspace files

## Strategy
1. Call get_workspace_info first to understand the layout
2. Try different -p flags, --workspace, and --all-features combinations
3. For workspace crates, try covering individual subcrates
4. Return SUCCESS: when API coverage >= {target_pct}%, or FAILURE: if impossible."""

    user_prompt = f"""The initial coverage run failed or returned below {target_pct}% API coverage.

Context:
{error_context}

Workspace info: {json.dumps(ws_info, indent=2)}

Please debug and fix the coverage extraction."""

    best_res = CoverageResult(ok=False, mode="agentic", error=f"Agent could not find API coverage >= {target_pct}%")
    best_api_pct = 0.0

    def agent_run_cargo_llvm_cov(args: list) -> dict:
        nonlocal best_res, best_api_pct
        cmd = ["cargo", "llvm-cov", "--json", "--ignore-run-fail", "--no-cfg-coverage", "--no-cfg-coverage-nightly"] + args
        try:
            res = subprocess.run(cmd, cwd=str(ws.path), capture_output=True, text=True, timeout=300)
            if res.returncode != 0:
                return {"error": "cargo llvm-cov failed", "stdout": res.stdout[:1000], "stderr": res.stderr[:2000]}

            data = json.loads(res.stdout)
            lines_covered, lines_total, percent = _extract_line_summary(data)
            covered_fns = _extract_covered_functions(data, ws.primary_package_name)

            api_covered, api_total, api_pct, _ = compute_api_coverage(api_surface, covered_fns)

            if api_pct > best_api_pct:
                best_api_pct = api_pct
                best_res = CoverageResult(
                    ok=True, percent=percent, lines_covered=lines_covered,
                    lines_total=lines_total, covered_functions=covered_fns,
                    mode=f"agent_run ({' '.join(args)})"
                )

            return {
                "ok": True,
                "line_percent": percent,
                "api_percent": api_pct,
                "lines_covered": lines_covered,
                "lines_total": lines_total,
                "api_covered": api_covered,
                "api_total": api_total,
                "covered_functions_count": len(covered_fns)
            }
        except Exception as e:
            return {"error": str(e)}

    def agent_list_directory(path: str = ".") -> dict:
        target = os.path.join(str(ws.path), path)
        try:
            if not os.path.isdir(target):
                return {"error": "Not a directory"}
            return {"contents": os.listdir(target)}
        except Exception as e:
            return {"error": str(e)}

    def agent_read_file(path: str) -> dict:
        target = os.path.join(str(ws.path), path)
        try:
            with open(target, "r") as f:
                return {"content": f.read()[:5000]}
        except Exception as e:
            return {"error": str(e)}

    def agent_get_workspace_info() -> dict:
        return {
            "name": ws.name,
            "path": str(ws.path),
            "is_workspace": ws.is_workspace,
            "primary_crate_path": str(ws.primary_crate_path),
            "primary_package_name": ws.primary_package_name,
            "workspace_members": ws.workspace_members,
            "resolved_member_dirs": resolved_members,
            "test_subcrates": ws.test_subcrates,
            "extra_coverage_pkgs": ws.extra_coverage_pkgs,
        }

    def agent_get_api_surface() -> dict:
        fns = []
        for fn in api_surface.functions[:50]:
            fns.append({
                "module_path": fn.module_path,
                "fn_name": fn.fn_name,
                "signature": fn.signature,
            })
        return {
            "crate_name": api_surface.crate_name,
            "total_functions": api_surface.total,
            "functions": fns,
        }

    def agent_run_cargo_command(args: list) -> dict:
        cmd = ["cargo"] + args
        try:
            res = subprocess.run(cmd, cwd=str(ws.path), capture_output=True, text=True, timeout=120)
            return {
                "returncode": res.returncode,
                "stdout": res.stdout[:3000],
                "stderr": res.stderr[:2000],
            }
        except Exception as e:
            return {"error": str(e)}

    avail_funcs = {
        "run_cargo_llvm_cov": agent_run_cargo_llvm_cov,
        "list_directory": agent_list_directory,
        "read_file": agent_read_file,
        "get_workspace_info": agent_get_workspace_info,
        "get_api_surface": agent_get_api_surface,
        "run_cargo_command": agent_run_cargo_command,
    }

    log.info("  [agent] triggering agentic coverage debugging loop...")
    final_resp = llm.call_agentic(system_prompt, [{"role": "user", "content": user_prompt}], AGENT_TOOLS, avail_funcs, max_turns=10)

    if best_res.ok and best_api_pct > 0:
        log.info(f"  [agent] SUCCESS: found API coverage {best_api_pct:.2f}% using args: {best_res.mode}")
        return best_res
    else:
        log.warning(f"  [agent] FAILURE: agent could not find coverage. Final response: {final_resp[:200]}")
        return best_res
