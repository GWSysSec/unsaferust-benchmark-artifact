"""audit_duplicate_stats.py -- does a rebench directory count the same test
binary more than once, and how much do the published numbers move if it does?

WHY THIS EXISTS

A rebench output directory holds one JSON stat file per test binary process, per
metric, named '<bin>-<cargohash>__<pid>.<bin>-<cargohash>.<metric>.json'. The
published aggregator (aggregate_rebench._aggregate_per_bin_dir) sums every
*.json it finds. Three different things can put several files in one directory
for what looks like "the same binary", and they need opposite treatment:

  1. SEVERAL PROCESSES OF ONE BUILD -- same cargo hash, different pids. The
     binary genuinely ran that many times. Summing these is CORRECT; collapsing
     them would throw away real executions. cc-rs runs its `test` binary in 336
     processes off two builds.

  2. SEVERAL BUILDS OF ONE BINARY -- different cargo hashes. These are repeated
     measurement PASSES over the same work. Summing them is the defect: it
     counts the same workload twice. They should be averaged.

  3. A BROKEN PASS -- a build whose counters are not credible, e.g. zopfli's
     `zopfli` binary recording 78 instructions in one pass against 114 billion
     in the other. Averaging a broken pass with a good one is still wrong, but
     deciding a pass is broken is a judgement about validity, not arithmetic.

This tool therefore groups by (logical binary, cargo hash) = one PASS, sums the
processes inside a pass, drops any pass contributing under 1% of that binary's
largest pass, and averages what remains. It reports every pass it drops and
every pass disagreement, so the judgement stays visible rather than buried.

BEWARE THE REPAIR MORE THAN THE DEFECT, ON THE HEAP PATH. Dropping failed passes
is not optional politeness. All three of the heap binaries with a second pass in
rebench_v2 have a FAILED second pass, so averaging without dropping moves the
corpus heap share by -1.44 percentage points and manufactures a correction out
of three broken measurements. Averaging blindly is worse than leaving the heap
data alone.

A NOTE ON THE METRIC FIELD. The metric a heap file carries is 'heap', not
'heap_tracker'; only the DIRECTORY is named heap_tracker. A check keyed on the
directory name matches nothing and reads as a clean run. This tool now treats
"files present, none matching the expected metric" as a hard error rather than
a zero.

WHAT THE DEFECT DOES TO rebench_v2

That directory is not one measurement pass. Its stat files span 2026-05-18 to
2026-05-30: roughly two full passes over the corpus, plus a top-up on 05-25 that
added one extra workload binary to 16 crates. 1,085 of 1,271 logical binaries
were measured in two passes and 186 in one. The once-measured binaries include
the added workload binaries, which execute the most unsafe code, so summing
weights the low-unsafe binaries double against the high-unsafe binaries. The
reported unsafe share comes out too low.

THE CORPUS RESULT IS ROBUST; PER-CRATE REPAIRS ARE NOT

Every defensible repair rule puts the corpus unsafe-instruction share within a
hundredth of a point of the same answer: 8.1862% dropping failed passes, 8.1976%
including them, 8.2009% under an independent implementation. Against 6.4075% as
published, the correction is +1.78 percentage points.

Individual crates are the opposite and must never be quoted from a repaired
number. brotli-decompressor reads 0.0162% if its two passes are averaged and
4.4601% if only the newest pass is kept, because one of its passes attributes
zero unsafe instructions to 35.8 billion executed ones. That spread is a
validity judgement about which pass to believe, not a deduplication result.

HOW TO USE IT

    python3 audit_duplicate_stats.py --validate    # gate, then audit rebench_v2
    python3 audit_duplicate_stats.py <run-dir>
    python3 audit_duplicate_stats.py <run-dir> --per-crate

Run it on any NEW run directory before comparing that run against a published
number. A run reporting one pass per binary needs no correction at all and can
be compared directly.

VALIDATION GATE

--validate re-runs the untouched published aggregator and compares it against
the committed RQ3 dataset in this repository. The corrected figures are only
worth reading if that gate passes, because it is what proves this script reads
the same files and fields the published pipeline did.
"""

from __future__ import annotations

import collections
import json
import re
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import aggregate_rebench as agg

DEFAULT_ROOT = Path.home() / "Projects/unsafebench/rusttest-gen/rebench_v2"

# '<bin>-<16 hex cargo hash>__<pid>.'
_STAT_RE = re.compile(r"^(.+?)-([0-9a-f]{16})__(\d+)\.")

FEATURE_METRIC = {"unsafe_counter": "unsafe_counter", "heap_tracker": "heap"}
FIELDS = {"unsafe_counter": ("total_instructions", "unsafe_instructions"),
          "heap_tracker": ("total_heap_usage", "unsafe_heap_memory")}

# A pass this small is a candidate failed measurement, but only when the same
# binary has a far larger pass to compare against -- plenty of test binaries are
# legitimately tiny.
TINY_PASS = 1_000
TINY_PARTNER = 1_000_000


def scan(directory: Path, want_metric: str):
    """-> (logical bin -> cargo hash -> [stats]), stray-file count, metrics seen.

    Stray files are those whose "metric" field does not match the feature
    directory they sit in. rebench_v2 has 665 stdlib_api files inside
    unsafe_counter directories; they share no field name with the counter
    fields so they add zero to every total, but they do inflate any bin count
    taken by globbing, which is why they are reported rather than ignored.
    """
    tree: dict = collections.defaultdict(lambda: collections.defaultdict(list))
    stray = 0
    seen: set = set()
    if not directory.is_dir():
        return tree, stray, seen
    for f in sorted(directory.glob("*.json")):
        try:
            j = json.loads(f.read_text())
        except (json.JSONDecodeError, OSError):
            continue
        seen.add(j.get("metric"))
        if j.get("metric") != want_metric:
            stray += 1
            continue
        m = _STAT_RE.match(f.name)
        lb = m.group(1) if m else f.name.split("__", 1)[0]
        hh = m.group(2) if m else "-"
        tree[lb][hh].append(j.get("stats", {}))
    return tree, stray, seen


def pass_totals(passes: dict, field: str) -> list[int]:
    """One number per pass: the sum over the processes inside that pass."""
    return [sum(int(s.get(field, 0) or 0) for s in files)
            for files in passes.values()]


def declared_bins(crate_dir: Path, feature: str, variant: str) -> list[dict]:
    """The binaries the run SUMMARY says it attempted, with exit code and the
    stat files it expected each to write.

    Read the summary rather than inferring absence by subtraction from a file
    listing. A binary killed by the timeout writes no stat file at all, so
    there is nothing to subtract and a listing-based check reports zero
    failures for exactly the worst case. The summary is the only record that a
    binary was ever attempted.

    The summary is NOT a complete inventory of what is on disk -- it reflects
    the last pass, while the directory can hold several -- so it is used to
    find what is MISSING, never to bound what is present.
    """
    sp = crate_dir / "rebench_summary.json"
    if not sp.exists():
        return []
    try:
        j = json.loads(sp.read_text())
    except (json.JSONDecodeError, OSError):
        return []
    feat = j.get("features", {}).get(feature)
    if isinstance(feat, list):
        feat = feat[0] if feat else None
    if not isinstance(feat, dict):
        return []
    var = feat.get("variants", {}).get(variant)
    return var.get("bins", []) if isinstance(var, dict) else []


def audit_feature(crates, feature: str, metric: str, per_crate: bool) -> dict:
    tot, unsafe_f = FIELDS[feature]
    files = logical = stray_files = 0
    metrics_seen: set = set()
    npasses = collections.Counter()
    nprocs = collections.Counter()
    sums = collections.Counter()      # as published: sum every pass
    mean_ = collections.Counter()     # average every pass, failures included
    kept_ = collections.Counter()     # average after dropping failed passes
    dropped = 0
    disagree = []
    broken = []
    rows = []
    # Coverage. A crate that produced NO stat file is a failed or timed-out
    # measurement, not an absent problem. It must never be silently skipped:
    # doing so shrinks the population the totals are computed over without
    # saying so, which reads as a clean run. Same false-all-clear shape as a
    # metric-name mismatch.
    no_feature_dir = []      # the feature directory does not exist
    empty_feature_dir = []   # it exists but yielded no usable stat file
    absent_bins = []         # summary attempted it; no stat file was written
    badexit_bins = []        # summary records a nonzero exit code
    for cd in crates:
        fdir = cd / feature / "without_native"
        if not fdir.is_dir():
            no_feature_dir.append(cd.name)
            continue
        tree, stray, seen = scan(fdir, metric)
        metrics_seen |= seen
        stray_files += stray
        ondisk = {f.name for f in fdir.glob("*.json")}
        for b in declared_bins(cd, feature, "without_native"):
            sf = b.get("stat_files") or []
            if b.get("exit_code", 0) != 0:
                badexit_bins.append((cd.name, b.get("name", "?"),
                                     b.get("exit_code")))
            if not sf or not any(x in ondisk for x in sf):
                absent_bins.append((cd.name, b.get("name", "?"),
                                    b.get("exit_code", 0)))
        if not tree:
            empty_feature_dir.append(cd.name)
            continue
        a = collections.Counter()
        c = collections.Counter()
        k = collections.Counter()
        for lb, passes in tree.items():
            logical += 1
            npasses[len(passes)] += 1
            for h, procs in passes.items():
                nprocs[len(procs)] += 1
                files += len(procs)
            hashes = list(passes)
            pt = {h: sum(int(s.get(tot, 0) or 0) for s in passes[h]) for h in hashes}
            pu = {h: sum(int(s.get(unsafe_f, 0) or 0) for s in passes[h])
                  for h in hashes}
            # A pass contributing under 1% of the binary's largest pass did not
            # run the workload. Averaging such a pass against a good one halves
            # a valid measurement, so drop it instead. This is the one validity
            # rule applied automatically; every dropped pass is reported.
            mx = max(pt.values())
            keep = [h for h in hashes if mx == 0 or pt[h] >= 0.01 * mx]
            dropped += len(hashes) - len(keep)
            a["t"] += sum(pt.values()); a["u"] += sum(pu.values())
            c["t"] += statistics.mean(pt.values())
            c["u"] += statistics.mean(pu.values())
            k["t"] += statistics.mean([pt[h] for h in keep])
            k["u"] += statistics.mean([pu[h] for h in keep])
            if len(passes) > 1:
                lo, hi = min(pt.values()), max(pt.values())
                if hi and (hi - lo) / hi > 0.01:
                    disagree.append((cd.name, lb, lo, hi))
                if lo < TINY_PASS and hi > TINY_PARTNER:
                    broken.append((cd.name, lb, lo, hi))
        sums["t"] += a["t"]; sums["u"] += a["u"]
        mean_["t"] += c["t"]; mean_["u"] += c["u"]
        kept_["t"] += k["t"]; kept_["u"] += k["u"]
        if per_crate and a["t"]:
            pa = 100 * a["u"] / a["t"]
            pk = 100 * k["u"] / k["t"] if k["t"] else float("nan")
            rows.append((cd.name, len(tree), pa, pk, pk - pa))
    return dict(files=files, logical=logical, stray_files=stray_files,
                npasses=dict(sorted(npasses.items())),
                multiproc=sum(v for kk, v in nprocs.items() if kk > 1),
                singleproc=nprocs.get(1, 0), dropped=dropped,
                sums=sums, mean=mean_, kept=kept_, disagree=disagree,
                broken=broken, rows=rows, fields=(tot, unsafe_f),
                metrics_seen=metrics_seen, crates_seen=len(crates),
                no_feature_dir=no_feature_dir,
                empty_feature_dir=empty_feature_dir,
                crates_with_data=len(crates) - len(no_feature_dir)
                                 - len(empty_feature_dir),
                absent_bins=absent_bins, badexit_bins=badexit_bins)


def validate() -> bool:
    pub = json.loads((HERE / "unsafeinstfrequency_rq3"
                      / "unsafe_counter_nativefalse.json").read_text())
    fresh = agg.aggregate_all(DEFAULT_ROOT)
    bad = checked = 0
    for crate, rec in pub["per_crate_data"].items():
        got = fresh.get(crate, {}).get("unsafe_counter", {}).get("without_native")
        if not got:
            print(f"  MISSING {crate}")
            bad += 1
            continue
        for k in ("total_instructions", "unsafe_instructions"):
            checked += 1
            if int(rec.get(k, 0)) != int(got.get(k, 0)):
                print(f"  MISMATCH {crate}.{k}: {rec.get(k)} vs {got.get(k)}")
                bad += 1
    print(f"validation: {checked} values compared over "
          f"{len(pub['per_crate_data'])} crates, {bad} mismatches")
    return bad == 0


def main() -> None:
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    flags = {a for a in sys.argv[1:] if a.startswith("--")}
    root = Path(args[0]).expanduser() if args else DEFAULT_ROOT

    if "--validate" in flags:
        if not validate():
            print("GATE FAIL")
            sys.exit(1)
        print("GATE PASS\n")

    if not root.is_dir():
        print(f"no such run directory: {root}")
        sys.exit(2)

    crates = sorted(p for p in root.iterdir()
                    if p.is_dir() and not p.name.startswith("_"))
    print(f"run directory: {root}")
    for feature, metric in FEATURE_METRIC.items():
        r = audit_feature(crates, feature, metric, "--per-crate" in flags)
        print(f"\n=== {feature} (variant without_native) ===")
        if not r["files"]:
            # A directory holding files of which NONE carried the expected
            # metric is a naming mismatch, not a clean result, and it must not
            # read as one. The metric field is 'heap', not 'heap_tracker'; a
            # check keyed on the feature name silently reports zero.
            if r["stray_files"]:
                print(f"  WARNING: found {r['stray_files']:,} stat files but NONE "
                      f"carry metric == {metric!r}.")
                print(f"  This is a metric-name mismatch, NOT a clean run. "
                      f"Observed metric values: "
                      f"{', '.join(sorted(map(repr, r['metrics_seen']))) or 'none'}")
                sys.exit(3)
            print("  no stat files")
            continue
        tot, unsafe_f = r["fields"]
        # Coverage first: a shrunken population invalidates every total below.
        miss = r["no_feature_dir"]; empty = r["empty_feature_dir"]
        print(f"  crates with usable data           {r['crates_with_data']} "
              f"of {r['crates_seen']}")
        if empty:
            print(f"  !! crates whose {feature} directory is EMPTY (measurement "
                  f"failed or timed out): {len(empty)}")
            print(f"     {', '.join(sorted(empty)[:12])}"
                  f"{' ...' if len(empty) > 12 else ''}")
        if miss:
            print(f"  !! crates with NO {feature} directory at all: {len(miss)}")
            print(f"     {', '.join(sorted(miss)[:12])}"
                  f"{' ...' if len(miss) > 12 else ''}")
        if empty or miss:
            print(f"     A crate contributing nothing is an ABSENT measurement, "
                  f"not a zero. Every total below is computed over "
                  f"{r['crates_with_data']} crates, not {r['crates_seen']}.")
        ab, be = r["absent_bins"], r["badexit_bins"]
        if ab:
            print(f"  !! binaries the run summary ATTEMPTED but which wrote no "
                  f"stat file: {len(ab)}")
            for cn, bn, ec in sorted(ab)[:10]:
                print(f"     {cn}/{bn} (exit {ec}"
                      f"{'; timeout' if ec == 124 else ''})")
            print(f"     These are absent measurements. They cannot be found by "
                  f"inspecting the files on disk, only by reading the summary.")
        if be:
            print(f"  !! binaries with a nonzero exit code in the summary: "
                  f"{len(be)}")
            for cn, bn, ec in sorted(be)[:8]:
                print(f"     {cn}/{bn} (exit {ec})")
        print(f"  stat files (one per process)      {r['files']:,}")
        print(f"  logical test binaries             {r['logical']:,}")
        print(f"  measurement passes per binary     {r['npasses']}")
        print(f"  passes holding >1 process         {r['multiproc']:,} "
              f"(these are real re-executions; summed, not collapsed)")
        print(f"  stray files of another metric     {r['stray_files']:,}")
        a, c, k = r["sums"], r["mean"], r["kept"]
        if a["t"]:
            pa = 100 * a["u"] / a["t"]
            pc = 100 * c["u"] / c["t"] if c["t"] else float("nan")
            pk = 100 * k["u"] / k["t"] if k["t"] else float("nan")
            print(f"\n  {tot}:")
            print(f"      summing every pass (as published)  {int(a['t']):>20,}")
            print(f"      averaging, failed passes dropped   {int(k['t']):>20,}"
                  f"   <- corrected")
            print(f"      averaging, failures included       {int(c['t']):>20,}"
                  f"   (sensitivity only)")
            if k["t"]:
                print(f"      inflation of the published total    {a['t']/k['t']:.4f}x")
            print(f"      passes dropped as failed            {r['dropped']}")
            print(f"  {unsafe_f} as a share of {tot}:")
            print(f"      summing every pass (as published)  {pa:8.4f}%")
            print(f"      averaging, failed passes dropped   {pk:8.4f}%  "
                  f"({pk-pa:+.4f} pp)   <- corrected")
            print(f"      averaging, failures included       {pc:8.4f}%  "
                  f"({pc-pa:+.4f} pp)   (sensitivity only)")
        if r["disagree"]:
            print(f"\n  binaries whose passes disagree on {tot} by >1%: "
                  f"{len(r['disagree'])}")
            for cn, lb, lo, hi in sorted(r["disagree"],
                                         key=lambda t: -(t[3] / max(t[2], 1)))[:5]:
                print(f"      {cn}/{lb}: {lo:,} vs {hi:,}")
        if r["broken"]:
            print(f"\n  CANDIDATE FAILED PASSES (one pass <{TINY_PASS:,} insts, "
                  f"another >{TINY_PARTNER:,}): {len(r['broken'])}")
            for cn, lb, lo, hi in r["broken"]:
                print(f"      {cn}/{lb}: {lo:,} vs {hi:,}  <- decide which to believe")
        if r["rows"]:
            print(f"\n  per crate (largest moves first; see the header on why "
                  f"these must not be published as repairs):")
            print(f"      {'crate':24s} {'bins':>5s} {'summed%':>9s} "
                  f"{'averaged%':>10s} {'delta pp':>9s}")
            for cn, nb, pa_, pc_, dl in sorted(r["rows"],
                                               key=lambda t: -abs(t[4]))[:20]:
                print(f"      {cn:24s} {nb:5d} {pa_:9.4f} {pc_:10.4f} {dl:+9.4f}")


if __name__ == "__main__":
    main()
