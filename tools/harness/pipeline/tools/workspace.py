"""
workspace.py — crate structure discovery

parses Cargo.toml to detect workspace vs single crate layout,
resolve primary library crate, and expand workspace member globs.
"""

import glob
import logging
import toml
from dataclasses import dataclass, field
from pathlib import Path

log = logging.getLogger(__name__)

# manual overrides for well-known workspace crates
PRIMARY_CRATE_MAP = {
    "tokio": "tokio",
    "curl": ".", "curl-rust": ".",
    "h2": ".", "headers": ".", "http": ".",
    "http-body": "http-body",
    "parking_lot": ".", "portable-atomic": ".",
    "borsh-rs": "borsh",
    "msgpack-rust": "rmp",
    "ouroboros": "ouroboros",
    "rust-cpp": "cpp",
    "rust-openssl": "openssl",
    "deranged": "deranged",
    "os_info": "os_info",
    "serde_with": "serde_with",
    "lexical-core": "lexical-core",
    "pulldown-cmark": "pulldown-cmark",
    "metrics": "metrics",
    "pin-project-lite": ".",
    "parity-scale-codec": ".",
}

TEST_SUBCRATE_MAP = {
    "h2": ["tests/h2-tests"],
    "tokio": ["tests-integration"],
    "serde_with": ["serde_with_test"],
    "ouroboros": ["examples"],
    "msgpack-rust": ["rmpv-tests"],
    "pin-project-lite": ["tests/no-core", "tests/no-std", "tests/lint"],
}

EXTRA_COVERAGE_PKGS = {
    "rust-cpp": ["cpp_test"],
    "serde_with": ["serde_with_test"],
    "msgpack-rust": ["rmp-serde", "rmpv", "rmpv-tests"],
}


@dataclass
class WorkspaceInfo:
    name: str = ""
    path: Path = field(default_factory=Path)
    is_workspace: bool = False
    root_has_package: bool = False
    primary_crate_path: Path = field(default_factory=Path)
    primary_package_name: str = ""
    primary_lib_name: str = ""  # name as it appears in symbols & rustdoc dirs
    workspace_members: list = field(default_factory=list)
    test_subcrates: list = field(default_factory=list)
    extra_coverage_pkgs: list = field(default_factory=list)


def _extract_lib_name(cargo_data: dict, package_name: str) -> str:
    """resolve the library name as it appears in symbols / rustdoc dirs.

    rules:
      - explicit [lib].name wins
      - else package.name.replace('-', '_')
      - rust crate names are normalized: '-' -> '_' in symbols
    """
    lib_section = cargo_data.get("lib", {})
    explicit = lib_section.get("name", "") if isinstance(lib_section, dict) else ""
    if explicit:
        return explicit
    return (package_name or "").replace("-", "_")


def discover_workspace(crate_path: Path) -> WorkspaceInfo:
    crate_path = Path(crate_path)
    ws = WorkspaceInfo(name=crate_path.name, path=crate_path)

    cargo_toml = crate_path / "Cargo.toml"
    if not cargo_toml.exists():
        log.error(f"no Cargo.toml found at {crate_path}")
        return ws

    try:
        data = toml.loads(cargo_toml.read_text())
    except Exception as e:
        log.error(f"failed to parse Cargo.toml: {e}")
        return ws

    workspace = data.get("workspace", {})
    members = workspace.get("members", [])
    has_package = "package" in data

    if members:
        ws.is_workspace = True
        ws.workspace_members = members
        ws.root_has_package = has_package
        log.info(f"  workspace with {len(members)} members: {members}")
    elif "workspace" in data and not members:
        ws.is_workspace = "members" in workspace
        ws.root_has_package = has_package

    if ws.is_workspace:
        if ws.name in PRIMARY_CRATE_MAP:
            # explicit override
            primary_rel = PRIMARY_CRATE_MAP[ws.name]
            ws.primary_crate_path = crate_path if primary_rel == "." else crate_path / primary_rel
        elif has_package:
            # workspace root has [package] — root is the primary crate
            ws.primary_crate_path = crate_path
        else:
            # workspace root has NO [package] — auto-detect primary from members
            ws.primary_crate_path = _auto_detect_primary(crate_path, members, ws.name)

        primary_toml = ws.primary_crate_path / "Cargo.toml"
        if primary_toml.exists():
            try:
                pdata = toml.loads(primary_toml.read_text())
                ws.primary_package_name = pdata.get("package", {}).get("name", ws.name)
                ws.primary_lib_name = _extract_lib_name(pdata, ws.primary_package_name)
            except Exception:
                ws.primary_package_name = ws.name
                ws.primary_lib_name = ws.name.replace("-", "_")
        else:
            ws.primary_package_name = ws.name
            ws.primary_lib_name = ws.name.replace("-", "_")
    else:
        ws.primary_crate_path = crate_path
        pkg = data.get("package", {})
        ws.primary_package_name = pkg.get("name", ws.name)
        ws.primary_lib_name = _extract_lib_name(data, ws.primary_package_name)
        if "workspace" not in data:
            with open(cargo_toml, "a") as f:
                f.write("\n\n[workspace]\n")

    ws.test_subcrates = TEST_SUBCRATE_MAP.get(ws.name, [])
    ws.extra_coverage_pkgs = EXTRA_COVERAGE_PKGS.get(ws.name, [])

    log.info(f"  type={'WORKSPACE' if ws.is_workspace else 'SINGLE'}, "
             f"primary={ws.primary_package_name} (lib={ws.primary_lib_name}) "
             f"at {ws.primary_crate_path}")
    return ws


def _auto_detect_primary(workspace_root: Path, members: list[str], ws_name: str) -> Path:
    """auto-detect the primary crate in a workspace when root has no [package].

    strategy:
    1. look for a member whose directory name matches the workspace name
    2. look for a member whose package name matches the workspace name
    3. fall back to the first member that has a [lib] or src/lib.rs
    4. last resort: first member
    """
    import glob as _glob

    # expand globs to get actual member dirs
    expanded = []
    for m in members:
        m_clean = m.rstrip("/")
        if "*" in m_clean:
            for match in sorted(_glob.glob(str(workspace_root / m_clean))):
                p = Path(match)
                if p.is_dir():
                    expanded.append(p)
        else:
            p = workspace_root / m_clean
            if p.is_dir():
                expanded.append(p)

    if not expanded:
        log.warning(f"  no workspace members found, defaulting to root")
        return workspace_root

    # strategy 1: directory name matches workspace name
    for p in expanded:
        if p.name == ws_name or p.name == ws_name.replace("-", "_"):
            log.info(f"  auto-detected primary crate by name match: {p.name}")
            return p

    # strategy 2: package name matches workspace name
    for p in expanded:
        member_toml = p / "Cargo.toml"
        if member_toml.exists():
            try:
                mdata = toml.loads(member_toml.read_text())
                pkg_name = mdata.get("package", {}).get("name", "")
                if pkg_name == ws_name or pkg_name == ws_name.replace("-", "_"):
                    log.info(f"  auto-detected primary crate by package name: {pkg_name}")
                    return p
            except Exception:
                continue

    # strategy 3: first member with a lib target
    for p in expanded:
        if (p / "src" / "lib.rs").exists():
            log.info(f"  auto-detected primary crate by lib.rs presence: {p.name}")
            return p

    # strategy 4: first member
    log.warning(f"  could not auto-detect primary crate, using first member: {expanded[0].name}")
    return expanded[0]


def build_member_lib_map(ws: WorkspaceInfo) -> dict[str, tuple[Path, str]]:
    """resolve each workspace member's lib name → (member_path, package_name).

    used by `generate_tests` to route a batch's test file into the correct
    workspace member when the targeted module lives in a non-primary member
    (e.g. msgpack-rust: tests for `rmpv::*` must land in `rmpv/tests/`, not
    in the primary member `rmp/tests/`, otherwise `use rmpv::*` fails with
    `E0432: unresolved import`).

    only members that expose a library target (`src/lib.rs` or `[lib]`) are
    included; bin-only members like `rmpv-tests` cannot host integ tests
    that import their own crate.
    """
    import glob as _glob
    out: dict[str, tuple[Path, str]] = {}
    if not ws.is_workspace or not ws.workspace_members:
        return out

    expanded: list[Path] = []
    for m in ws.workspace_members:
        m_clean = m.rstrip("/")
        if "*" in m_clean:
            for match in sorted(_glob.glob(str(ws.path / m_clean))):
                p = Path(match)
                if p.is_dir():
                    expanded.append(p)
        else:
            p = ws.path / m_clean
            if p.is_dir():
                expanded.append(p)

    for p in expanded:
        cargo = p / "Cargo.toml"
        if not cargo.exists():
            continue
        try:
            data = toml.loads(cargo.read_text())
        except Exception:
            continue
        pkg = data.get("package", {})
        pkg_name = pkg.get("name", "")
        if not pkg_name:
            continue
        # only libraries — bin-only members can't host integ tests for
        # types they don't export
        has_lib = bool(data.get("lib")) or (p / "src" / "lib.rs").exists()
        if not has_lib:
            continue
        lib_name = _extract_lib_name(data, pkg_name)
        out[lib_name] = (p, pkg_name)
    return out


def derive_workspace_pathway(ws: WorkspaceInfo, crate_name: str) -> str:
    """derive the workspace pathway string for the csv "Workspace pathway" column."""
    if not ws.is_workspace:
        return ""

    parts: list[str] = []

    # primary crate relative path
    try:
        primary_rel = ws.primary_crate_path.relative_to(ws.path)
        if str(primary_rel) != ".":
            parts.append(f"{crate_name}/{primary_rel}")
    except ValueError:
        pass

    # test subdirectory
    primary_tests = ws.primary_crate_path / "tests"
    if primary_tests.exists():
        try:
            rel = primary_tests.relative_to(ws.path)
            test_path = f"{crate_name}/{rel}"
            if test_path not in parts:
                parts.append(test_path)
        except ValueError:
            pass

    # test subcrates
    for sc in ws.test_subcrates:
        sc_path = f"{crate_name}/{sc}"
        if sc_path not in parts:
            parts.append(sc_path)

    if not parts:
        return "Run in Workspace"

    return ", ".join(parts)


def resolve_workspace_members(ws: WorkspaceInfo) -> list[Path]:
    """expand workspace member globs into actual directories."""
    resolved = []
    seen = set()
    for member in ws.workspace_members:
        member_clean = member.rstrip("/")
        if "*" in member_clean:
            for match in sorted(glob.glob(str(ws.path / member_clean))):
                p = Path(match)
                if p.is_dir() and p not in seen:
                    seen.add(p)
                    resolved.append(p)
        else:
            p = ws.path / member_clean
            if p.is_dir() and p not in seen:
                seen.add(p)
                resolved.append(p)
    return resolved
