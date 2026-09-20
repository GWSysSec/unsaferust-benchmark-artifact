"""
test_locator.py — test inventory collection

finds unit tests in src/, integration tests in tests/,
benchmarks in benches/, and tests in workspace subcrates.
"""

import re
import logging
from dataclasses import dataclass
from pathlib import Path

from pipeline.tools.workspace import WorkspaceInfo, resolve_workspace_members

log = logging.getLogger(__name__)


@dataclass
class TestInventory:
    unit_test_files: int = 0
    unit_test_count: int = 0
    unit_test_source: str = ""
    integ_test_files: int = 0
    integ_test_count: int = 0
    integ_test_source: str = ""
    bench_files: int = 0
    total_test_count: int = 0

    def has_any_tests(self) -> bool:
        return self.total_test_count > 0


def _extract_cfg_test_blocks(text: str) -> str:
    blocks = []
    lines = text.split("\n")
    i = 0
    while i < len(lines):
        line = lines[i]
        if "#[cfg(test)]" in line:
            block_lines = [line]
            i += 1
            brace_depth = 0
            found_open = False
            while i < len(lines):
                block_lines.append(lines[i])
                brace_depth += lines[i].count("{") - lines[i].count("}")
                if not found_open and "{" in lines[i]:
                    found_open = True
                if found_open and brace_depth <= 0:
                    break
                i += 1
            blocks.append("\n".join(block_lines))
        i += 1
    return "\n\n".join(blocks)


def _collect_unit_tests(src_dir: Path, max_chars: int = 15000) -> tuple[int, int, str]:
    if not src_dir.exists():
        return 0, 0, ""

    files_with_tests = 0
    total_tests = 0
    source_parts = []
    total_chars = 0

    for rs_file in sorted(src_dir.rglob("*.rs")):
        try:
            text = rs_file.read_text(encoding="utf-8", errors="replace")
        except Exception:
            continue

        if "#[cfg(test)]" not in text and "#[test]" not in text:
            continue

        test_count = len(re.findall(r'#\[(tokio::)?test', text))
        if test_count == 0:
            continue

        files_with_tests += 1
        total_tests += test_count

        cfg_blocks = _extract_cfg_test_blocks(text)
        if cfg_blocks:
            rel = rs_file.relative_to(src_dir)
            snippet = f"// unit tests from: src/{rel}\n{cfg_blocks}"
            if total_chars + len(snippet) > max_chars:
                remaining = max_chars - total_chars
                if remaining > 200:
                    source_parts.append(snippet[:remaining] + "\n... (truncated)")
                break
            source_parts.append(snippet)
            total_chars += len(snippet)

    return files_with_tests, total_tests, "\n\n".join(source_parts)


def _collect_integ_tests(tests_dir: Path, max_chars: int = 25000) -> tuple[int, int, str]:
    if not tests_dir.exists() or not tests_dir.is_dir():
        return 0, 0, ""

    files = 0
    total_tests = 0
    source_parts = []
    total_chars = 0

    for rs_file in sorted(tests_dir.rglob("*.rs")):
        try:
            text = rs_file.read_text(encoding="utf-8", errors="replace")
        except Exception:
            continue

        test_count = len(re.findall(r'#\[(tokio::)?test', text))
        files += 1
        total_tests += test_count

        rel = rs_file.relative_to(tests_dir)
        snippet = f"// integration test: tests/{rel}\n{text}"

        if total_chars + len(snippet) > max_chars:
            remaining = max_chars - total_chars
            if remaining > 200:
                source_parts.append(snippet[:remaining] + "\n... (truncated)")
            break
        source_parts.append(snippet)
        total_chars += len(snippet)

    return files, total_tests, "\n\n".join(source_parts)


def locate_tests(ws: WorkspaceInfo) -> TestInventory:
    inv = TestInventory()
    primary = ws.primary_crate_path

    inv.unit_test_files, inv.unit_test_count, inv.unit_test_source = \
        _collect_unit_tests(primary / "src")

    inv.integ_test_files, inv.integ_test_count, inv.integ_test_source = \
        _collect_integ_tests(primary / "tests")

    for test_subcrate_rel in ws.test_subcrates:
        test_sub_path = ws.path / test_subcrate_rel
        for candidate in [test_sub_path / "tests", test_sub_path / "src"]:
            if candidate.exists():
                f, c, src = _collect_integ_tests(candidate, max_chars=10000)
                inv.integ_test_files += f
                inv.integ_test_count += c
                if src:
                    inv.integ_test_source += f"\n\n// from test subcrate: {test_subcrate_rel}\n{src}"

    if ws.is_workspace:
        for member_path in resolve_workspace_members(ws):
            if member_path == ws.primary_crate_path:
                continue
            member_src = member_path / "src"
            member_tests = member_path / "tests"
            if member_src.exists() and member_path not in [ws.path / t for t in ws.test_subcrates]:
                uf, uc, usrc = _collect_unit_tests(member_src, max_chars=5000)
                if uc > 0:
                    inv.unit_test_files += uf
                    inv.unit_test_count += uc
                    if usrc:
                        inv.unit_test_source += f"\n\n// from workspace member: {member_path.name}\n{usrc}"
            if member_tests.exists() and member_path not in [ws.path / t for t in ws.test_subcrates]:
                f, c, src = _collect_integ_tests(member_tests, max_chars=5000)
                inv.integ_test_files += f
                inv.integ_test_count += c
                if src:
                    inv.integ_test_source += f"\n\n// from workspace member: {member_path.name}\n{src}"

    benches_dir = primary / "benches"
    if benches_dir.exists():
        inv.bench_files = len(list(benches_dir.rglob("*.rs")))

    if ws.is_workspace and ws.primary_crate_path != ws.path:
        ws_tests = ws.path / "tests"
        ws_benches = ws.path / "benches"
        if ws_tests.exists():
            f, c, _ = _collect_integ_tests(ws_tests, max_chars=5000)
            inv.integ_test_files += f
            inv.integ_test_count += c
        if ws_benches.exists():
            inv.bench_files += len(list(ws_benches.rglob("*.rs")))

    inv.total_test_count = inv.unit_test_count + inv.integ_test_count
    return inv
