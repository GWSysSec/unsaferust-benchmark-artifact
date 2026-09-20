#!/usr/bin/env python3
"""Rebuild the 100-crate corpus from corpus_lock.csv.

Reconstructs, for every crate, the exact source tree the measurements were
taken on: fetch the recorded commit or released version, apply the recorded
overlay, and leave the result where the measurement harness expects to find it.
Run scripts/make_corpus_lock.py first, or use the lock and overlay shipped with
the artifact.

Each crate is rebuilt in three steps.

  1. Fetch. A crate whose `source` column says `git` is cloned from its
     repository and checked out at the recorded commit. A crate whose `source`
     says `crates.io` is downloaded as the recorded released version from
     static.crates.io, which is immutable, and unpacked.

  2. Overlay. corpus_overlay/<dir_name>.patch holds every difference between
     that upstream tree and the measured tree; corpus_overlay/<dir_name>/ holds
     every file the upstream tree does not have. Most of this is uninteresting:
     an empty [workspace] table added to Cargo.toml so cargo treats the crate as
     its own workspace, and symbolic links that became ordinary files when the
     tree was copied. A few crates carry real source edits.

  3. Verify. With --verify-against DIR, the rebuilt tree is compared file by
     file against a reference copy, ignoring build output. This is how the lock
     itself was checked.

The generated coverage workloads are not fetched here. They live in
recipes/<crate>/tests/ and the harness plants them into its own working copy.

Usage:
    python3 scripts/fetch_corpus.py --out /path/to/corpus
    python3 scripts/fetch_corpus.py --out /tmp/c --crate bytes --crate chrono
    python3 scripts/fetch_corpus.py --out /tmp/c --verify-against ~/…/bootcamp
"""

from __future__ import annotations

import argparse
import csv
import shutil
import subprocess
import sys
import tarfile
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LOCK = ROOT / "corpus_lock.csv"
OVERLAY = ROOT / "corpus_overlay"

# Output a build or a test run leaves behind, never part of a source tree.
# Checked on every path component: bytemuck keeps a second cargo target
# directory at derive/target/.
ARTEFACT_DIRS = {"target", ".git", ".fingerprint"}
# Cargo.lock is deliberately NOT here. It is not build output: it pins the
# exact version of every dependency that was compiled and measured. Upstream
# commits one for only 14 of the corpus crates; for 81 more the lockfile was
# generated on this machine at measurement time, from whatever crates.io served
# then. Drop it and a rebuild re-resolves dependencies to newer versions and
# measures a different program.
ARTEFACT_NAMES = {".cargo-ok", "Cargo.toml.orig", ".cargo_vcs_info.json"}
ARTEFACT_SUFFIXES = (".log",)


def is_artefact(rel: Path) -> bool:
    if any(part in ARTEFACT_DIRS for part in rel.parts):
        return True
    if rel.name in ARTEFACT_NAMES:
        return True
    return rel.name.endswith(ARTEFACT_SUFFIXES)


def run(cmd: list[str], cwd: Path | None = None) -> tuple[int, str]:
    r = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)
    return r.returncode, (r.stdout + r.stderr)


def fetch_git(url: str, commit: str, dst: Path, submodules: str) -> str:
    """Clone `url` into `dst` and check out `commit`. Returns "" on success."""
    if dst.exists():
        shutil.rmtree(dst)
    code, out = run(["git", "clone", "--quiet", "--no-checkout", url, str(dst)])
    if code != 0:
        return f"clone failed: {out.strip().splitlines()[-1] if out.strip() else code}"
    code, out = run(["git", "checkout", "--quiet", commit], cwd=dst)
    if code != 0:
        # A commit on no branch is still fetchable from most servers by SHA.
        code2, out2 = run(["git", "fetch", "--quiet", "origin", commit], cwd=dst)
        if code2 == 0:
            code, out = run(["git", "checkout", "--quiet", commit], cwd=dst)
    if code != 0:
        return f"commit {commit[:12]} not reachable: {out.strip()[:200]}"
    # Check out only the submodules the measured tree actually had content in,
    # which the lock records per crate. curl and git2 vendor their C library
    # this way and cannot build without it. iri-string and lexical-core declare
    # submodules holding test data that were never checked out, so the tests
    # needing that data did not run; initialising them would change the
    # workload. Each submodule lands on the commit the superproject records.
    for path in [p for p in submodules.split(";") if p]:
        code, out = run(["git", "submodule", "update", "--init", "--recursive",
                         "--quiet", "--", path], cwd=dst)
        if code != 0:
            return f"submodule {path} failed: {out.strip()[:200]}"
    return ""


def fetch_release(name: str, version: str, dst: Path, cache: Path) -> str:
    cache.mkdir(parents=True, exist_ok=True)
    tarball = cache / f"{name}-{version}.crate"
    if not tarball.is_file():
        url = f"https://static.crates.io/crates/{name}/{name}-{version}.crate"
        try:
            with urllib.request.urlopen(url, timeout=120) as r:
                tarball.write_bytes(r.read())
        except Exception as e:                        # noqa: BLE001
            return f"download failed: {e}"
    if dst.exists():
        shutil.rmtree(dst)
    dst.parent.mkdir(parents=True, exist_ok=True)
    tmp = dst.parent / f".{dst.name}.unpack"
    if tmp.exists():
        shutil.rmtree(tmp)
    with tarfile.open(tarball) as tf:
        tf.extractall(tmp)
    inner = next((p for p in tmp.iterdir() if p.is_dir()), None)
    if inner is None:
        return "tarball held no directory"
    shutil.move(str(inner), str(dst))
    shutil.rmtree(tmp, ignore_errors=True)
    return ""


def apply_overlay(dir_name: str, dst: Path) -> str:
    patch = OVERLAY / f"{dir_name}.patch"
    if patch.is_file():
        code, out = run(["patch", "-p1", "--silent", "--force",
                         "-i", str(patch)], cwd=dst)
        if code != 0:
            return f"patch failed: {out.strip()[:200]}"
    extra = OVERLAY / dir_name
    if extra.is_dir():
        shutil.copytree(extra, dst, dirs_exist_ok=True)
    return ""


def tree_files(base: Path) -> dict[str, bytes]:
    """Every source file under `base`, by contents.

    Symbolic links are read through rather than skipped. Copying a tree with
    `cp -L` turns a link into an ordinary file holding the same bytes, and
    several crates carry links to LICENSE files or, in libgit2's test
    fixtures, to test data. Comparing contents makes those trees equal, which
    is what matters: nothing that compiles differs.
    """
    out: dict[str, bytes] = {}
    for p in base.rglob("*"):
        rel = p.relative_to(base)
        if is_artefact(rel):
            continue
        try:
            if p.is_file():
                out[str(rel)] = p.read_bytes()
        except OSError:
            continue        # a broken link, or a file we may not read
    return out


def verify(dst: Path, ref: Path) -> tuple[int, int, list[str]]:
    a, b = tree_files(dst), tree_files(ref)
    same = sum(1 for k in a.keys() & b.keys() if a[k] == b[k])
    differing = sorted(k for k in a.keys() & b.keys() if a[k] != b[k])
    only = sorted((a.keys() ^ b.keys()))
    return same, len(differing) + len(only), (differing + only)[:6]


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--out", type=Path, required=True,
                    help="directory to rebuild the corpus into")
    ap.add_argument("--crate", action="append", default=[],
                    help="rebuild only this crate (repeatable)")
    ap.add_argument("--verify-against", type=Path,
                    help="a reference copy of the corpus to compare against")
    ap.add_argument("--lock", type=Path, default=LOCK)
    args = ap.parse_args()

    rows = list(csv.DictReader(args.lock.open()))
    if args.crate:
        want = set(args.crate)
        rows = [r for r in rows if r["crate"] in want or r["dir_name"] in want]
        if not rows:
            raise SystemExit(f"no crate in the lock matches {sorted(want)}")

    args.out.mkdir(parents=True, exist_ok=True)
    cache = args.out / ".crate_cache"
    ok = failed = 0
    problems: list[str] = []

    for r in rows:
        dir_name = r["dir_name"]
        dst = args.out / dir_name
        print(f"{dir_name} ... ", end="", flush=True)

        if r["source"] == "crates.io":
            name = r["repository"].rsplit("/", 1)[-1]
            err = fetch_release(name, r["commit"], dst, cache)
        else:
            err = fetch_git(r["repository"], r["commit"], dst,
                            r.get("submodules", ""))
        if err:
            print(f"FAILED  {err}")
            problems.append(f"{dir_name}: {err}")
            failed += 1
            continue

        err = apply_overlay(dir_name, dst)
        if err:
            print(f"FAILED  {err}")
            problems.append(f"{dir_name}: {err}")
            failed += 1
            continue

        if args.verify_against:
            ref = args.verify_against / dir_name
            if ref.is_dir():
                same, bad, examples = verify(dst, ref)
                if bad:
                    print(f"rebuilt, {bad} files differ from the reference "
                          f"({same} match): {', '.join(examples)}")
                    problems.append(f"{dir_name}: {bad} files differ")
                else:
                    print(f"rebuilt and verified ({same} files)")
            else:
                print("rebuilt (no reference copy to compare against)")
        else:
            print("rebuilt")
        ok += 1

    print(f"\n{ok} crates rebuilt, {failed} failed")
    if problems:
        print("\nproblems:")
        for p in problems:
            print(f"  {p}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
