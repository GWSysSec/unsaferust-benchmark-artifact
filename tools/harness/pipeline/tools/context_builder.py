"""
context_builder.py — generation context assembly

assembles crate metadata, api index, existing test patterns,
and target function details for llm prompt construction.
also supports RA-based analysis for detailed bug-fixing context.
"""

import html
import json
import re
import subprocess
import logging
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

from pipeline.tools.api_discovery import FunctionRecord

log = logging.getLogger(__name__)


@dataclass
class GenerationContext:
    crate_name: str = ""
    crate_version: str = ""
    crate_description: str = ""
    readme_summary: str = ""
    api_index: list[dict] = field(default_factory=list)
    existing_test_files: dict[str, str] = field(default_factory=dict)
    target_functions: list[FunctionRecord] = field(default_factory=list)
    ra_api_summary: str = ""


def build_generation_context(
    crate_path: Path,
    uncovered_fns: list[FunctionRecord],
    rusttest_gen_binary: str = "",
) -> GenerationContext:
    crate_path = Path(crate_path)
    ctx = GenerationContext()

    # cargo metadata
    cargo_meta = _parse_cargo_toml(crate_path)
    ctx.crate_name = cargo_meta["name"]
    ctx.crate_version = cargo_meta["version"]
    ctx.crate_description = cargo_meta.get("description", "")

    # readme
    readme = _read_readme(crate_path)
    ctx.readme_summary = readme[:600] if readme else ""

    # existing test files (for pattern matching)
    ctx.existing_test_files = _load_test_files(crate_path)

    # api index from cargo doc
    ctx.api_index = _build_api_index(crate_path, ctx.crate_name)

    # target functions
    ctx.target_functions = uncovered_fns

    # RA-based analysis (optional, for richer context during bug fixing)
    if rusttest_gen_binary:
        ctx.ra_api_summary = _run_ra_analysis(rusttest_gen_binary, crate_path)

    return ctx


def _parse_cargo_toml(crate_path: Path) -> dict:
    toml_path = crate_path / "Cargo.toml"
    if not toml_path.exists():
        return {"name": "unknown", "version": "0.0.0"}
    try:
        import tomllib
        with open(toml_path, "rb") as f:
            data = tomllib.load(f)
    except (ImportError, Exception):
        data = _parse_toml_naive(toml_path.read_text())
    pkg = data.get("package", {})
    return {
        "name": pkg.get("name", "unknown"),
        "version": pkg.get("version", "0.0.0"),
        "description": pkg.get("description", ""),
    }


def _parse_toml_naive(text: str) -> dict:
    result = {"package": {}}
    section = None
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("["):
            section = line.strip("[]").strip()
        elif "=" in line and section:
            key, _, val = line.partition("=")
            if section in result:
                result[section][key.strip()] = val.strip().strip('"')
    return result


def _read_readme(crate_path: Path) -> str:
    for name in ("README.md", "README.rst", "README.txt", "README"):
        p = crate_path / name
        if p.exists():
            return p.read_text(encoding="utf-8", errors="ignore")
    return ""


def _load_test_files(crate_path: Path) -> dict[str, str]:
    d = crate_path / "tests"
    if not d.exists():
        return {}
    result = {}
    for f in sorted(d.glob("*.rs")):
        content = f.read_text(encoding="utf-8", errors="ignore")
        if len(content) < 50000:
            result[f.name] = content
    return result


def _build_api_index(crate_path: Path, crate_name: str) -> list[dict]:
    doc_root = crate_path / "target" / "doc"
    if not doc_root.exists():
        return []
    crate_doc_dir = doc_root / crate_name.replace("-", "_")
    if not crate_doc_dir.exists():
        return []

    index = []
    seen = set()
    html_files = (
        sorted(crate_doc_dir.rglob("fn.*.html"))
        + sorted(crate_doc_dir.rglob("struct.*.html"))
        + sorted(crate_doc_dir.rglob("enum.*.html"))
    )
    for html_file in html_files:
        item_name = html_file.stem.split(".", 1)[1]
        rel = html_file.relative_to(crate_doc_dir)
        module_parts = list(rel.parts[:-1])
        module_path = "::".join([crate_name, *module_parts, item_name])
        if module_path in seen:
            continue
        seen.add(module_path)
        content = html_file.read_text(encoding="utf-8", errors="ignore")
        sig = _extract_sig(content)
        doc = _extract_doc(content)
        index.append({"name": module_path, "signature": sig, "doc": doc})
    return index


def _extract_sig(content: str) -> str:
    matches = re.findall(r"<code[^>]*>(.*?)</code>", content, re.DOTALL)
    matches += re.findall(r'<h4[^>]*class="code-header"[^>]*>(.*?)</h4>', content, re.DOTALL)
    for m in matches:
        text = re.sub(r"\s+", " ", html.unescape(re.sub(r"<[^>]+>", "", m)).strip())
        if re.search(r"(?:pub\s+)?(?:async\s+)?(?:unsafe\s+)?fn\s+", text):
            return text
    return ""


def _extract_doc(content: str) -> str:
    m = re.search(r'<div[^>]*class="docblock"[^>]*>(.*?)</div>', content, re.DOTALL)
    if not m:
        return ""
    return re.sub(r"\s+", " ", html.unescape(re.sub(r"<[^>]+>", "", m.group(1))).strip())[:300]


def _run_ra_analysis(binary: str, crate_path: Path) -> str:
    """run rusttest-gen analyze for RA-based api summary (used during bug fixing)."""
    try:
        result = subprocess.run(
            [binary, "analyze", str(crate_path), "--format", "json"],
            capture_output=True, text=True, timeout=120,
        )
        if result.returncode != 0:
            return ""
        data = json.loads(result.stdout)
        return data.get("public_api_summary", "")
    except Exception as e:
        log.warning(f"  ra analysis failed: {e}")
        return ""
