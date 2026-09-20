"""
crate_prep.py — copy a bootcamp crate to a tmp working dir with fixups.

handles:
- excluding only top-level target/ (not nested target/ that may be source)
- preserving symlinks (rmp/LICENSE points outside the bootcamp)
- stripping outdated numeric rust-toolchain pins (jpeg-decoder pins 1.61.0
  which can't build modern deps), keeping channel pins (nightly/beta/stable)
- planting marker .git dir for ring-style build.rs that branches on its presence
"""

import logging
import re
import shutil
from pathlib import Path

log = logging.getLogger(__name__)


def copy_crate(bootcamp_dir: Path, crate_name: str, tmp_dir: Path) -> Path:
    """copy bootcamp/<crate> -> tmp/<crate>, applying source-prep fixups.

    returns the tmp destination path.
    """
    src = Path(bootcamp_dir) / crate_name
    if not src.exists():
        raise FileNotFoundError(f"crate not found: {src}")

    dst = Path(tmp_dir) / crate_name
    if dst.exists():
        shutil.rmtree(dst)

    src_resolved = src.resolve()

    def _ignore(dirpath: str, names: list[str]) -> set[str]:
        # only exclude top-level "target" (the cargo build dir). previously we
        # used shutil.ignore_patterns("target", ...) which matches by basename
        # at any depth — that ate cc-rs/src/target/, breaking `mod apple;`.
        ignored = {".git"}
        for n in names:
            if n.endswith((".o", ".d")):
                ignored.add(n)
        if Path(dirpath).resolve() == src_resolved and "target" in names:
            ignored.add("target")
        return ignored

    # symlinks=True keeps links instead of dereferencing; rmp/LICENSE points
    # to ../LICENSE (outside the bootcamp) and would otherwise crash the copy.
    # ignore_dangling_symlinks=True handles other broken-link cases gracefully.
    shutil.copytree(src, dst, ignore=_ignore, symlinks=True, ignore_dangling_symlinks=True)

    _strip_outdated_toolchain_pins(dst)
    _plant_ring_marker_git(dst)

    return dst


def _strip_outdated_toolchain_pins(dst: Path):
    """remove rust-toolchain pins ONLY when they point to an outdated numeric
    version (e.g. jpeg-decoder pins "1.61.0", which can't build modern deps
    like libc 0.2.172 needing >=1.63). keep channel pins like "nightly" or
    "beta" — fs4 needs nightly for `extern crate test;` inside its test_mod
    macro, and stripping that pin breaks compilation.
    """
    for tc_name in ("rust-toolchain", "rust-toolchain.toml"):
        tc = dst / tc_name
        if not tc.exists():
            continue
        try:
            content = tc.read_text()
        except OSError:
            continue
        # extract channel: `channel = "..."` in toml, else first non-empty line
        chan_match = re.search(r'channel\s*=\s*[\'"]([^\'"]+)[\'"]', content)
        if chan_match:
            chan = chan_match.group(1)
        else:
            lines = [ln.strip() for ln in content.splitlines() if ln.strip()]
            chan = lines[0] if lines else ""
        # keep non-numeric channels (nightly, beta, stable, custom names)
        if chan and not re.match(r"^\d+\.\d+", chan):
            continue
        tc.unlink()


def _plant_ring_marker_git(dst: Path):
    """ring: bootcamp source is missing pregenerated/ (gitignored upstream;
    populated only by `cargo package`). its build.rs branches on .git
    existence: with .git → regenerate from perl, without .git → expect
    pregenerated/. our copy strips .git, hitting the failing branch. drop a
    marker .git dir wherever a build.rs uses this exact pattern so the perl
    path runs instead. perl is required (it is on this host).
    """
    for buildrs in dst.rglob("build.rs"):
        try:
            text = buildrs.read_text(encoding="utf-8", errors="ignore")
        except OSError:
            continue
        if "PREGENERATED" in text and "is_git" in text:
            (buildrs.parent / ".git").mkdir(exist_ok=True)
