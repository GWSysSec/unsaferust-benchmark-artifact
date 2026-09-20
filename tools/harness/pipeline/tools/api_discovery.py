"""
api_discovery.py — public api surface discovery via cargo doc html

scans target/doc/<crate>/ for fn.*.html and struct.*.html to find
all public api entry points. no rust-analyzer dependency.
"""

import html
import re
import subprocess
import logging
from dataclasses import dataclass, field
from pathlib import Path

import toml as _toml

log = logging.getLogger(__name__)


@dataclass
class FunctionRecord:
    module_path: str
    fn_name: str
    signature: str
    doc_comment: str
    has_examples: bool
    # one of: "fn" | "method" | "trait" | "trait_method" | "struct" | "enum" | "macro"
    kind: str = "fn"


# kinds that have no callable runtime symbol of their own and shouldn't count
# toward api coverage. records of these kinds are still kept in the surface
# (the LLM uses them for prompting / type context) — they're just dropped from
# the denominator. matches the intent of origin/master 00fd37b ("only count
# public callable API"), but preserved as a taxonomy so we can re-include any
# subset later without touching discovery code.
#   - trait, trait_method: reachable only via concrete implementors; the
#     implementor's symbols are what llvm-cov sees.
#   - struct, enum: types themselves aren't called; coverage flows through
#     their inherent methods (kind="method").
#   - macro: macro_rules! and #[macro_export] expand at compile time; llvm-cov
#     sees the expansion site, not the macro item.
NON_COUNTABLE_KINDS: frozenset[str] = frozenset({
    "trait", "trait_method", "struct", "enum", "macro",
})


@dataclass
class ApiSurface:
    crate_name: str = ""
    functions: list[FunctionRecord] = field(default_factory=list)
    total: int = 0
    # paths known covered without llvm-cov data (used for macro-only crates
    # where coverage comes from static test-source analysis, not runtime).
    pre_covered: set[str] = field(default_factory=set)

    def countable_functions(self) -> list[FunctionRecord]:
        """records that should count toward the api coverage denominator.

        narrow fallback: if no records pass the standard NON_COUNTABLE_KINDS
        filter but the surface still has trait_methods (byteorder is the
        canonical case — its entire public API is `ReadBytesExt`/`WriteBytesExt`
        provided methods, callable via blanket impls on `io::Read`/`io::Write`),
        count those trait_methods. without this the denominator is 0 and the
        crate is unprocessable.
        """
        primary = [r for r in self.functions if r.kind not in NON_COUNTABLE_KINDS]
        if primary:
            return primary
        return [r for r in self.functions if r.kind == "trait_method"]

    @property
    def countable_total(self) -> int:
        return len(self.countable_functions())


def _macro_names_in_source(crate_path: Path) -> set[str]:
    """find all `macro_rules! NAME` declarations across the crate's source.

    rust-analyzer's LSP reports macro_rules items as `pub fns`, but they have
    no runtime symbols — they expand at compile time. counting them in the API
    denominator is wrong: they're structurally uncoverable via llvm-cov.
    """
    src = crate_path / "src"
    names: set[str] = set()
    if not src.exists():
        return names
    pat = re.compile(r"\bmacro_rules!\s+(\w+)")
    for rs in src.rglob("*.rs"):
        try:
            text = rs.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        for m in pat.finditer(text):
            names.add(m.group(1))
    return names


def _macros_used_in_tests(crate_path: Path, macro_names: set[str]) -> set[str]:
    """return macros that are invoked from the crate's own test files.

    for macro-only crates (e.g. pin-project-lite) llvm-cov can't tell us which
    macros run — they're compile-time. but if a `pin_project!` invocation
    appears in tests/*.rs, the macro IS being exercised by the test suite, and
    that's the closest analogue to "covered" for macro APIs.
    """
    if not macro_names:
        return set()
    test_dirs = [crate_path / "tests", crate_path / "examples"]
    used: set[str] = set()
    for tdir in test_dirs:
        if not tdir.exists():
            continue
        for rs in tdir.rglob("*.rs"):
            try:
                text = rs.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue
            for name in macro_names - used:
                # match `name!` as a macro invocation; \b prevents matching
                # `not_the_name!` substrings.
                if re.search(rf"\b{re.escape(name)}\s*!", text):
                    used.add(name)
    return used


def discover_public_api(
    crate_path: Path,
    rusttest_gen_binary: str = "",
    lib_name: str = "",
    workspace_root: Path | None = None,
) -> ApiSurface:
    """discover all public api functions/methods. LSP-first, then cargo doc, then source scan.

    `lib_name` should be the [lib].name (or package name with - -> _) — this is the
    prefix used both in symbols (for coverage matching) and rustdoc dirs.
    """
    crate_path = Path(crate_path)
    pkg_name = _get_crate_name(crate_path)
    canonical = lib_name or pkg_name.replace("-", "_")
    macro_names = _macro_names_in_source(crate_path)

    records = []

    # primary: rust-analyzer LSP via rusttest-gen analyze
    if rusttest_gen_binary:
        records = _discover_from_rust_analyzer(
            rusttest_gen_binary, crate_path, canonical, workspace_root,
        )
        if records:
            log.info(f"  discovered {len(records)} public api items from rust-analyzer LSP")
            # B2: filter LSP records to those reachable through pub-use chains.
            # rust-analyzer's HIR reports every `pub` item, including ones inside
            # private modules (e.g. hashbrown::raw::* when `mod raw;` is private).
            # cargo doc only renders pages for items reachable from the crate
            # root, so its file tree is a clean ground-truth for "user-importable
            # public surface". for pub use reexports, rustdoc emits redirect
            # stubs we can use to rewrite source paths to canonical paths.
            reach = _build_doc_reachability(crate_path, canonical)
            if reach:
                before = len(records)
                records = _filter_to_doc_reachable(records, reach)
                log.info(f"  doc-reachability filter: {before} -> {len(records)} items kept")
            else:
                log.warning("  doc-reachability filter skipped (cargo doc unavailable) "
                            "— LSP records may include unreachable items")

    # fallback when LSP is empty/sparse: union cargo doc + source scan.
    # cargo doc skips #[doc(hidden)] modules (e.g. ouroboros::macro_help) that DO
    # produce real symbols at runtime; source scan catches them by reading pub fn
    # at any indent. union both rather than cascading because each misses items
    # the other catches (cargo doc gets re-exported macros; source scan gets
    # doc-hidden runtime helpers).
    if not records:
        log.info("  LSP yielded 0 items — supplementing with cargo doc + source scan...")
        _ensure_docs(crate_path)
        doc_dir = _find_doc_dir(crate_path, canonical)
        if doc_dir:
            doc_records = _scan_doc_html(doc_dir, canonical)
            log.info(f"  cargo doc found {len(doc_records)} public api items")
            records.extend(doc_records)
        src_records = _discover_from_source(crate_path, canonical)
        log.info(f"  source scan found {len(src_records)} public items")
        seen_paths = {r.module_path for r in records}
        for r in src_records:
            if r.module_path not in seen_paths:
                seen_paths.add(r.module_path)
                records.append(r)

    # drop entries that are actually macro_rules! definitions (LSP reports
    # them as pub fns even though they have no runtime symbols).
    if macro_names:
        records = [r for r in records if r.fn_name not in macro_names]

    # macro-only crates (pin-project-lite, etc.) have no callable API after
    # the filter above. for those, fall back to treating macros AS the API
    # surface — the only meaningful coverage signal is "is this macro used by
    # the test suite?", which we pre-compute here so compute_api_coverage can
    # use it directly without llvm-cov data. drop `__`-prefixed names: rust
    # convention for internal helpers (e.g. pin-project-lite's
    # __pin_project_expand) which are only called transitively from the public
    # macro — counting them in the denominator is unfair.
    pre_covered: set[str] = set()
    if not records and macro_names:
        public_macros = {n for n in macro_names if not n.startswith("__")}
        used = _macros_used_in_tests(crate_path, public_macros)
        for name in sorted(public_macros):
            path = f"{canonical}::{name}"
            records.append(FunctionRecord(
                module_path=path,
                fn_name=name,
                signature=f"macro_rules! {name}",
                doc_comment="",
                has_examples=False,
                kind="macro",
            ))
            if name in used:
                pre_covered.add(path)
        log.info(f"  macro-only crate: {len(public_macros)} public macros "
                 f"(filtered {len(macro_names) - len(public_macros)} private), "
                 f"{len(pre_covered)} exercised by tests/examples")

    return ApiSurface(
        crate_name=canonical,
        functions=records,
        total=len(records),
        pre_covered=pre_covered,
    )


def discover_workspace_member_apis(
    ws,
    rusttest_gen_binary: str = "",
) -> tuple[list["FunctionRecord"], list[str], set[str]]:
    """discover api for the primary plus any explicitly-listed coverage extras.

    returns (deduped_records, list_of_member_labels, pre_covered_paths).

    we used to iterate every workspace member with a lib, which contaminates
    the denominator. e.g. ouroboros workspace has ouroboros (primary, 1 item),
    ouroboros_examples (55), ouroboros_macro (61) — but tests only cover
    ouroboros::* symbols, so the 116 phantom items always count as "uncovered"
    and drag api% to ~0. now we only pick up extras explicitly named in
    EXTRA_COVERAGE_PKGS (the same set we run coverage against), so the API
    surface stays aligned with what the tests can actually hit.
    """
    # imported here to avoid a circular import (workspace -> api_discovery via stages)
    from pipeline.tools.workspace import (
        WorkspaceInfo, resolve_workspace_members, _extract_lib_name,
    )

    seen_paths: set[str] = set()
    all_records: list[FunctionRecord] = []
    profiled: list[str] = []
    pre_covered: set[str] = set()

    def add_from(crate_dir: Path, lib_name: str, label: str):
        api = discover_public_api(
            crate_dir,
            rusttest_gen_binary=rusttest_gen_binary,
            lib_name=lib_name,
            workspace_root=ws.path if ws.is_workspace else None,
        )
        added = 0
        for rec in api.functions:
            if rec.module_path not in seen_paths:
                seen_paths.add(rec.module_path)
                all_records.append(rec)
                added += 1
        pre_covered.update(api.pre_covered)
        if added:
            profiled.append(f"{label}({added})")
        log.info(f"    member api: {label} -> {added} new items (lib={lib_name})")

    # primary always counts
    add_from(ws.primary_crate_path, ws.primary_lib_name, ws.primary_package_name)

    # extras: only members we explicitly run coverage against (e.g. msgpack-rust:
    # rmp + rmp-serde + rmpv). map package_name -> member_path by scanning members.
    if ws.is_workspace and ws.extra_coverage_pkgs:
        extra_set = set(ws.extra_coverage_pkgs)
        for member_path in resolve_workspace_members(ws):
            if member_path == ws.primary_crate_path:
                continue
            member_toml = member_path / "Cargo.toml"
            if not member_toml.exists():
                continue
            try:
                mdata = _toml.loads(member_toml.read_text())
            except Exception:
                continue
            pkg = mdata.get("package", {})
            mpkg_name = pkg.get("name", member_path.name)
            if mpkg_name not in extra_set:
                continue
            mlib_name = _extract_lib_name(mdata, mpkg_name)
            # skip bin-only members (no src/lib.rs and no [lib])
            if not (member_path / "src" / "lib.rs").exists() and "lib" not in mdata:
                log.info(f"    member api: {mpkg_name} skipped (bin-only)")
                continue
            add_from(member_path, mlib_name, mpkg_name)

    return all_records, profiled, pre_covered


def compute_api_coverage(
    api: ApiSurface,
    covered_functions: set[str],
) -> tuple[int, int, float, list[str]]:
    """compute api coverage from llvm-cov covered functions vs discovered api.

    cargo doc shows re-exported paths (e.g., crate::Builder::new) while
    llvm-cov shows internal paths (e.g., crate::runnable::Builder::new).
    we match on suffix keys (Type::method, or bare fn_name) to handle re-exports.

    trait declarations and trait method declarations are excluded from the
    denominator — see NON_COUNTABLE_KINDS for the rationale.
    """
    countable = api.countable_functions()
    if not countable:
        return 0, 0, 0.0, []

    public_paths = {rec.module_path for rec in countable}
    public_paths = {p for p in public_paths if not _looks_platform_specific(p)}
    # seed covered set with paths known to be covered without runtime data
    # (e.g. macros invoked in test files).
    pre_covered = api.pre_covered & public_paths

    # build suffix index from llvm-cov covered functions
    # for each covered fn, generate multiple suffix keys:
    #   async_task::runnable::Builder::new -> {"Builder::new", "new"}
    cov_suffixes: dict[str, set[str]] = {}
    cov_bare_names: set[str] = set()
    for fn_path in covered_functions:
        parts = fn_path.split("::")
        for depth in range(1, len(parts)):
            suffix = "::".join(parts[-depth:])
            cov_suffixes.setdefault(suffix, set()).add(fn_path)
        if parts:
            cov_bare_names.add(parts[-1])

    covered_public = set(pre_covered)
    for pub_path in public_paths - covered_public:
        parts = pub_path.split("::")
        matched = False
        # try progressively shorter suffixes from the public path
        for depth in range(len(parts), 0, -1):
            suffix = "::".join(parts[-depth:])
            if suffix in cov_suffixes:
                covered_public.add(pub_path)
                matched = True
                break
        # last resort: bare function name match (only for standalone functions)
        if not matched and parts and parts[-1] in cov_bare_names:
            covered_public.add(pub_path)

    total = len(public_paths)
    hit = len(covered_public)
    pct = (hit / total * 100.0) if total > 0 else 0.0
    uncovered = sorted(public_paths - covered_public)

    return hit, total, pct, uncovered


_PLATFORM_HINTS = {
    "backends", "linux", "android", "apple", "darwin", "ios", "macos",
    "windows", "unix", "wasi", "wasm", "emscripten", "solaris",
    "freebsd", "netbsd", "openbsd", "dragonfly",
}


def _looks_platform_specific(path: str) -> bool:
    segments = [seg.lower() for seg in path.split("::") if seg]
    return any(seg in _PLATFORM_HINTS for seg in segments)


def _get_crate_name(crate_path: Path) -> str:
    toml_path = crate_path / "Cargo.toml"
    if not toml_path.exists():
        return crate_path.name
    for line in toml_path.read_text().splitlines():
        line = line.strip()
        if line.startswith("name"):
            return line.split("=")[-1].strip().strip('"').strip("'")
    return crate_path.name


def _ensure_docs(crate_path: Path):
    doc_index = crate_path / "target" / "doc"
    if doc_index.exists():
        return
    log.info("  generating docs with cargo doc --no-deps...")
    result = subprocess.run(
        ["cargo", "doc", "--no-deps"],
        cwd=crate_path, capture_output=True, text=True, timeout=120,
    )
    if result.returncode != 0:
        log.warning(f"  cargo doc failed: {result.stderr[:200]}")


def _find_doc_dir(crate_path: Path, crate_name: str) -> Path | None:
    # check both crate local target/ and workspace root target/
    candidates = [
        crate_path / "target" / "doc",
        crate_path.parent / "target" / "doc",
        crate_path.parent.parent / "target" / "doc"
    ]
    
    doc_root = None
    for cand in candidates:
        if cand.exists() and cand.is_dir():
            doc_root = cand
            break
            
    if not doc_root:
        return None

    direct = doc_root / crate_name.replace("-", "_")
    if direct.exists() and (direct / "index.html").exists():
        return direct

    for d in doc_root.iterdir():
        if d.is_dir() and list(d.glob("fn.*.html")):
            return d

    return None


# item-page kinds emitted by rustdoc whose stem starts with `<kind>.`
# struct/enum/trait/type are "method holders" — methods on these are
# kept iff the holder is doc-reachable. fn/macro/constant/static are
# standalone items.
_DOC_KINDS: frozenset[str] = frozenset({
    "fn", "struct", "enum", "trait", "type", "macro", "constant", "static",
})

_DOC_REDIRECT_RE = re.compile(
    r'<meta[^>]+http-equiv="refresh"[^>]+URL=([^"]+)"',
    re.IGNORECASE,
)

# rustdoc emits one anchor per documented method on type/trait pages:
#   <section id="method.NAME"> for inherent methods and default trait impls
#   <section id="tymethod.NAME"> for required trait methods
# we harvest both — the union is the full callable method surface for the type.
_DOC_METHOD_ANCHOR_RE = re.compile(r'id="(?:ty)?method\.([A-Za-z_][A-Za-z0-9_]*)"')


def _build_doc_reachability(
    crate_path: Path, crate_name: str,
) -> dict:
    """walk target/doc/<crate>/ and build doc-reachability maps.

    returns:
      {
        "fn"|"struct"|"enum"|...: {source_path: canonical_path},
        "__methods": {canonical_type_path: set(method_name, ...)},
      }

    real (non-redirect) pages map to themselves; redirect stubs map source
    path (file location) to canonical (URL target) path. method anchors are
    harvested from real type/trait pages and indexed by the type's canonical
    path so the filter can drop records like `crate::HashMap::get_inner`
    where the type is reachable but the method isn't documented.

    if cargo doc fails or no doc dir is found, returns empty dict — caller
    treats that as "filter unavailable" and keeps records unfiltered.
    """
    _ensure_docs(crate_path)
    doc_dir = _find_doc_dir(crate_path, crate_name)
    if not doc_dir:
        return {}

    out: dict = {k: {} for k in _DOC_KINDS}
    out["__methods"] = {}
    doc_root = doc_dir.resolve()

    for html_file in doc_dir.rglob("*.html"):
        stem = html_file.stem
        if "." not in stem:
            continue
        kind, item = stem.split(".", 1)
        if kind not in _DOC_KINDS:
            continue
        rel = html_file.relative_to(doc_dir)
        module_segs = list(rel.parts[:-1])
        source_path = "::".join([crate_name, *module_segs, item])

        try:
            content = html_file.read_text(encoding="utf-8", errors="ignore")
        except OSError:
            continue

        m = _DOC_REDIRECT_RE.search(content[:1024])
        if m:
            target = m.group(1)
            try:
                target_full = (html_file.parent / target).resolve()
                target_rel = target_full.relative_to(doc_root)
            except (ValueError, OSError):
                continue
            target_stem = target_rel.stem
            if "." not in target_stem:
                continue
            tkind, titem = target_stem.split(".", 1)
            if tkind not in _DOC_KINDS:
                continue
            target_segs = list(target_rel.parts[:-1])
            canonical_path = "::".join([crate_name, *target_segs, titem])
            out[kind][source_path] = canonical_path
        else:
            out[kind][source_path] = source_path
            # only struct/enum/trait/type pages carry method anchors
            if kind in ("struct", "enum", "trait", "type"):
                methods = set(_DOC_METHOD_ANCHOR_RE.findall(content))
                if methods:
                    out["__methods"][source_path] = methods

    return out


def _filter_to_doc_reachable(
    records: list[FunctionRecord],
    reach: dict[str, dict[str, str]],
) -> list[FunctionRecord]:
    """drop LSP records that aren't doc-reachable; rewrite paths of records
    on reexported types/fns to their canonical (rustdoc) paths.

    method/trait_method records are reachable iff their parent type/trait is
    doc-reachable. coverage matching uses suffix matching on module_path, so
    rewriting `crate::map::HashMap::insert` to `crate::HashMap::insert` is
    safe — the `HashMap::insert` suffix still hits the llvm-cov symbol.
    """
    type_holders: dict[str, str] = {}
    for k in ("struct", "enum", "trait", "type"):
        type_holders.update(reach.get(k, {}))
    doc_methods: dict[str, set[str]] = reach.get("__methods", {})

    out: list[FunctionRecord] = []
    for rec in records:
        path = rec.module_path
        kind = rec.kind

        if kind in ("method", "trait_method"):
            type_path = "::".join(path.split("::")[:-1])
            method = path.split("::")[-1]
            canonical_type = type_holders.get(type_path)
            if canonical_type is None:
                continue
            # require the method itself to be documented on the canonical type.
            # without this check, LSP-reported private helpers (`get_inner`,
            # `*_unchecked_*_inner`, etc.) on a doc-reachable type sneak through
            # because rustdoc only renders public methods.
            type_methods = doc_methods.get(canonical_type)
            if type_methods is not None and method not in type_methods:
                continue
            if canonical_type == type_path:
                out.append(rec)
            else:
                out.append(FunctionRecord(
                    module_path=f"{canonical_type}::{method}",
                    fn_name=rec.fn_name, signature=rec.signature,
                    doc_comment=rec.doc_comment, has_examples=rec.has_examples,
                    kind=rec.kind,
                ))
        else:
            # type-like records (struct/enum/trait): pool across kinds because
            # LSP and rustdoc can disagree on the kind label — e.g. LSP marks
            # `pub type X = ...` aliases as `struct` while rustdoc renders them
            # as `type.X.html`. fn/macro have stricter shapes; look up by kind.
            if kind in ("struct", "enum", "trait"):
                canonical = type_holders.get(path)
            else:
                kind_map = reach.get(kind, {})
                canonical = kind_map.get(path)
            if canonical is None:
                continue
            if canonical == path:
                out.append(rec)
            else:
                out.append(FunctionRecord(
                    module_path=canonical,
                    fn_name=rec.fn_name, signature=rec.signature,
                    doc_comment=rec.doc_comment, has_examples=rec.has_examples,
                    kind=rec.kind,
                ))

    return out


def _scan_doc_html(doc_dir: Path, crate_name: str) -> list[FunctionRecord]:
    records = []
    seen_paths: set[str] = set()

    def module_prefix_for(path: Path) -> list[str]:
        rel = path.relative_to(doc_dir)
        if len(rel.parts) <= 1:
            return []
        return list(rel.parts[:-1])

    def add_record(rec: FunctionRecord):
        if rec.module_path in seen_paths:
            return
        seen_paths.add(rec.module_path)
        records.append(rec)

    # macros and standalone functions. attr/derive are proc-macro pages — group
    # them under "macro" so the coverage filter treats them uniformly.
    standalone_files = (
        [(p, "fn") for p in sorted(doc_dir.rglob("fn.*.html"))] +
        [(p, "macro") for p in sorted(doc_dir.rglob("macro.*.html"))] +
        [(p, "macro") for p in sorted(doc_dir.rglob("attr.*.html"))] +
        [(p, "macro") for p in sorted(doc_dir.rglob("derive.*.html"))]
    )
    for html_file, item_kind in standalone_files:
        prefix = html_file.stem.split(".")[0] + "."
        fn_name = _normalize_item_name(html_file.stem.replace(prefix, "", 1))
        content = html_file.read_text(encoding="utf-8", errors="ignore")
        module_prefix = module_prefix_for(html_file)
        module_path = "::".join([crate_name, *module_prefix, fn_name])

        add_record(FunctionRecord(
            module_path=module_path,
            fn_name=fn_name,
            signature=_extract_signature(content),
            doc_comment=_extract_docblock(content),
            has_examples=len(_extract_examples(content)) > 0,
            kind=item_kind,
        ))

    # struct and enum methods
    method_pages = sorted(doc_dir.rglob("struct.*.html")) + sorted(doc_dir.rglob("enum.*.html"))
    for html_file in method_pages:
        type_name = html_file.stem.split(".", 1)[1]
        content = html_file.read_text(encoding="utf-8", errors="ignore")
        module_prefix = module_prefix_for(html_file)
        impl_content = _extract_inherent_impl_methods(content)

        sections = re.split(r'<section id="', impl_content)
        for sec in sections:
            if not sec.startswith("method."):
                continue

            method_name = _normalize_item_name(sec[len("method."):].split('"', 1)[0])
            signature = _extract_signature(sec)

            if not re.search(r"\bpub\b.*\bfn\b", signature):
                continue

            add_record(FunctionRecord(
                module_path="::".join([crate_name, *module_prefix, type_name, method_name]),
                fn_name=method_name,
                signature=signature,
                doc_comment=_extract_docblock(sec),
                has_examples=len(_extract_examples(sec)) > 0,
                kind="method",
            ))

    return records


def _extract_signature(content: str) -> str:
    code_matches = re.findall(r"<code[^>]*>(.*?)</code>", content, re.DOTALL)
    h4_matches = re.findall(r'<h4[^>]*class="code-header"[^>]*>(.*?)</h4>', content, re.DOTALL)
    for m in code_matches + h4_matches:
        text = html.unescape(re.sub(r"<[^>]+>", "", m)).strip()
        text = re.sub(r"\s+", " ", text)
        if re.search(r"(?:pub\s+)?(?:async\s+)?(?:unsafe\s+)?fn\s+", text) or \
           re.search(r"macro_rules!\s+", text) or \
           re.search(r"pub\s+macro\s+", text) or \
           text.startswith("#["):
            return text
    return "// signature not found"


def _extract_docblock(content: str) -> str:
    match = re.search(r'<div[^>]*class="docblock"[^>]*>(.*?)</div>', content, re.DOTALL)
    if not match:
        return ""
    text = html.unescape(re.sub(r"<[^>]+>", "", match.group(1)))
    return re.sub(r"\s+", " ", text).strip()


def _extract_examples(content: str) -> list[str]:
    blocks = re.findall(
        r'<div class="example-wrap">.*?<pre[^>]*>(.*?)</pre>', content, re.DOTALL
    )
    return [html.unescape(re.sub(r"<[^>]+>", "", b)).strip() for b in blocks if b]


def _extract_inherent_impl_methods(content: str) -> str:
    start = content.find('<div id="implementations-list">')
    if start == -1:
        return content

    tail = content[start:]
    end_markers = [
        '<h2 id="deref-methods',
        '<h2 id="trait-implementations"',
        '<h2 id="synthetic-implementations"',
        '<h2 id="auto-trait-implementations"',
        '<h2 id="blanket-implementations"',
        '<h2 id="implementors"',
    ]
    end = len(tail)
    for marker in end_markers:
        idx = tail.find(marker)
        if idx != -1:
            end = min(end, idx)
    return tail[:end]


def _normalize_item_name(name: str) -> str:
    return re.sub(r"-\d+$", "", name.strip())


# --- fallback discovery strategies ---

_PUB_FN_RE = re.compile(
    r"^\s*pub\s+(?:async\s+)?(?:unsafe\s+)?(?:extern\s+\"[^\"]*\"\s+)?fn\s+(\w+)",
    re.MULTILINE,
)
_PUB_STRUCT_RE = re.compile(r"^\s*pub\s+struct\s+(\w+)", re.MULTILINE)
_PUB_ENUM_RE = re.compile(r"^\s*pub\s+enum\s+(\w+)", re.MULTILINE)
_PUB_TRAIT_RE = re.compile(r"^\s*pub\s+trait\s+(\w+)", re.MULTILINE)
_IMPL_METHOD_RE = re.compile(
    r"^\s*pub\s+(?:async\s+)?(?:unsafe\s+)?fn\s+(\w+)\s*[(<]",
    re.MULTILINE,
)
_MACRO_EXPORT_RE = re.compile(r"#\[macro_export\]\s*macro_rules!\s+(\w+)", re.DOTALL)
_PROC_MACRO_RE = re.compile(r"#\[proc_macro(?:_derive|_attribute)?\s*(?:\([^)]*\))?\]")


def _discover_from_source(crate_path: Path, crate_name: str) -> list[FunctionRecord]:
    """scan source files for pub fn/struct/enum + macro_export fallback."""
    src_dir = crate_path / "src"
    if not src_dir.exists():
        return []

    records = []
    seen = set()

    for rs_file in sorted(src_dir.rglob("*.rs")):
        try:
            text = rs_file.read_text(encoding="utf-8", errors="replace")
        except Exception:
            continue

        rel = rs_file.relative_to(src_dir)
        module_parts = list(rel.parts[:-1])
        stem = rel.stem
        if stem != "mod" and stem != "lib":
            module_parts.append(stem)

        # pub fn at module level
        for m in _PUB_FN_RE.finditer(text):
            fn_name = m.group(1)
            # extract the full line as a rough signature
            line_start = text.rfind("\n", 0, m.start()) + 1
            line_end = text.find("\n", m.end())
            sig = text[line_start:line_end].strip() if line_end > 0 else text[line_start:].strip()

            module_path = "::".join([crate_name, *module_parts, fn_name])
            if module_path not in seen:
                seen.add(module_path)
                records.append(FunctionRecord(
                    module_path=module_path,
                    fn_name=fn_name,
                    signature=sig,
                    doc_comment="",
                    has_examples=False,
                    kind="fn",
                ))

        # pub struct/enum — record their methods via impl blocks
        for pattern, type_kind in [(_PUB_STRUCT_RE, "struct"), (_PUB_ENUM_RE, "enum")]:
            for m in pattern.finditer(text):
                type_name = m.group(1)
                type_path = "::".join([crate_name, *module_parts, type_name])
                if type_path not in seen:
                    seen.add(type_path)
                    records.append(FunctionRecord(
                        module_path=type_path,
                        fn_name=type_name,
                        signature=f"pub {type_kind} {type_name}",
                        doc_comment="",
                        has_examples=False,
                        kind=type_kind,
                    ))

        # pub trait
        for m in _PUB_TRAIT_RE.finditer(text):
            trait_name = m.group(1)
            trait_path = "::".join([crate_name, *module_parts, trait_name])
            if trait_path not in seen:
                seen.add(trait_path)
                records.append(FunctionRecord(
                    module_path=trait_path,
                    fn_name=trait_name,
                    signature=f"pub trait {trait_name}",
                    doc_comment="",
                    has_examples=False,
                    kind="trait",
                ))


        # macro_export macros
        for m in _MACRO_EXPORT_RE.finditer(text):
            macro_name = m.group(1)
            macro_path = "::".join([crate_name, macro_name])
            if macro_path not in seen:
                seen.add(macro_path)
                records.append(FunctionRecord(
                    module_path=macro_path,
                    fn_name=macro_name,
                    signature=f"macro_rules! {macro_name}",
                    doc_comment="",
                    has_examples=False,
                    kind="macro",
                ))

    return records


def _discover_from_rust_analyzer(
    binary: str, crate_path: Path, lib_name: str,
    workspace_root: Path | None = None,
) -> list[FunctionRecord]:
    """use rusttest-gen analyze (rust-analyzer LSP) for api discovery.

    walks the structured CrateAnalysis output: top-level pub fns and
    struct/enum methods (from inherent impls). trait items are intentionally
    excluded from API denominator accounting.
    """
    import json as _json

    cmd = [binary, "analyze", str(crate_path), "--format", "json"]
    if workspace_root and workspace_root != crate_path:
        cmd.extend(["--workspace-root", str(workspace_root)])

    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=180)
    except subprocess.TimeoutExpired:
        log.warning("  rust-analyzer analyze timed out")
        return []
    except Exception as e:
        log.warning(f"  rust-analyzer analyze failed: {e}")
        return []

    if result.returncode != 0:
        log.warning(f"  rust-analyzer returned non-zero: {result.stderr[:200]}")
        return []

    try:
        data = _json.loads(result.stdout)
    except _json.JSONDecodeError as e:
        log.warning(f"  rust-analyzer output not valid JSON: {e}")
        return []

    records: list[FunctionRecord] = []
    seen: set[str] = set()

    def emit(module_segments: list[str], item_name: str, signature: str, kind: str = "fn"):
        # drop "mod" segments — the rust-analyzer extractor includes the file
        # name, so items in `de/mod.rs` come back with module="de::mod". the
        # canonical Rust path is just `de::`, and that's what coverage symbols
        # use, so a trailing/embedded "mod" makes suffix matching miss.
        cleaned_segments = [s for s in module_segments if s and s != "mod"]
        path_parts = [lib_name] + cleaned_segments + [item_name]
        module_path = "::".join(path_parts)
        if module_path in seen:
            return
        seen.add(module_path)
        records.append(FunctionRecord(
            module_path=module_path,
            fn_name=item_name,
            signature=signature,
            doc_comment="",
            has_examples=False,
            kind=kind,
        ))

    # top-level pub fns
    for fn in data.get("all_pub_fns", []):
        module = fn.get("module", "") or ""
        emit(module.split("::") if module else [],
             fn.get("name", ""),
             f"pub fn {fn.get('name', '')}({fn.get('params', '')}) -> {fn.get('return_type', '()')}",
             kind="fn")

    # inherent impl methods (skip trait impls — `trait` is set, `type` empty)
    # NB: ImplInfo uses serde rename: fields are "trait" and "type", not "trait_name"/"type_name"
    for impl in data.get("impl_blocks", []):
        if impl.get("trait"):
            continue  # skip trait impls
        type_name = impl.get("type") or ""
        if not type_name:
            continue
        type_base = type_name.split("<")[0].strip()
        if not type_base:
            continue
        module = impl.get("module", "") or ""
        module_segs = (module.split("::") if module else []) + [type_base]
        for sig in impl.get("method_sigs", []):
            mname = sig.get("name", "")
            if not mname:
                continue
            emit(module_segs, mname,
                 f"pub fn {mname}({sig.get('params', '')}) -> {sig.get('return_type', '()')}",
                 kind="method")

    # trait method declarations: tagged "trait_method" so they're excluded from
    # the coverage denominator via NON_COUNTABLE_KINDS (no symbol of their own —
    # llvm-cov reports `<Impl as Trait>::method` under each implementor). they
    # ARE kept in the surface so the LLM still sees them when prompting.
    for tr in data.get("pub_traits", []):
        module = tr.get("module", "") or ""
        tname = tr.get("name", "")
        if not tname:
            continue
        module_segs = (module.split("::") if module else []) + [tname]
        for m in tr.get("methods", []):
            mname = m.get("name", "")
            if mname:
                emit(module_segs, mname,
                     f"pub fn {mname}({m.get('params', '')}) -> {m.get('return_type', '()')}",
                     kind="trait_method")

    # struct/enum/trait names themselves — kept in the API surface for context
    # but excluded from the coverage denominator (kind in NON_COUNTABLE_KINDS).
    for st in data.get("pub_structs", []):
        sname = st.get("name", "")
        if sname:
            module = st.get("module", "") or ""
            emit(module.split("::") if module else [], sname, f"pub struct {sname}", kind="struct")
    for en in data.get("pub_enums", []):
        ename = en.get("name", "")
        if ename:
            module = en.get("module", "") or ""
            emit(module.split("::") if module else [], ename, f"pub enum {ename}", kind="enum")

    return records
