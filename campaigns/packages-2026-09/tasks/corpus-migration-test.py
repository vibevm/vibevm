#!/usr/bin/env python3
"""Focused tests for `corpus-migration.py`, on a miniature Markdown→XML pair.

Usage:
    python tasks/corpus-migration-test.py          # every case
    python tasks/corpus-migration-test.py mapper   # cases whose name contains this

Two layers, and the split is the point.

The MAPPER cases are pure: `map_path` takes its `exists` predicate as an
argument, so the PROP-052 rules — root rewrite, package inner root, the `.md`
→ `.xml` twin only as a fallback — are pinned against a dictionary rather than
against a tree. They are the cases that must never need a checkout to run. The
inverse direction (`unrelayout` / `historical_candidates`, which
`text-stability.py` walks back to a verdict revision) is pinned here too: the
two directions live in one module precisely so one test file can prove they are
inverses.

The MIGRATION cases run the real thing: a two-document corpus is written to
disk in both serialisations and parsed by `vibe progress mirror`, the shipped
parser, exactly as the live migration parses the corpus. Nothing here
re-implements a reader of either format — a test that hand-parsed the fixture
would agree with itself and with nothing else, which is precisely the failure
`text-stability.py`'s `#recipe-drift` was built to catch.

The miniature pair is a scale model of the real move: `ALPHA` and `GAMMA` are
carried across untouched, and `BETA` cites a sibling document by name, so the
conversion rewrites `other.md` → `other.xml` inside its prose and its content
hash moves. That single fact is the POSITIVE CONTROL — a migration that
reported "nothing moved" over this fixture would be broken, and the assertion
below fails loudly when it does.

Needs the shipped CLI: `target/debug/vibe`, `$VIBE_BIN`, or `cargo` on PATH.
Exit code 0 when every case passes, 1 otherwise.
"""

import io
import json
import pathlib
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parent.parent.parent
sys.path.insert(0, str(HERE))

import corpus_migration as mig  # noqa: E402
from corpus_migration.cli import main as cli_main  # noqa: E402

# --------------------------------------------------------------------------
# The miniature corpus, in both serialisations
# --------------------------------------------------------------------------

MINI_MD = """# Mini spec {#root}

<status stage="doc" state="done"/>

@fact:ALPHA The first claim, carried across the move unchanged. @status:doc/done

@fact:BETA The second claim, which cites `other.md` and moves with it. @status:doc/done

## A named section {#named}

<status stage="doc" state="done"/>

@fact:GAMMA A claim inside an anchored unit. @status:doc/done
"""

MINI_XML = """<?xml version="1.0" encoding="UTF-8"?>
<spec xmlns="https://vibevm.org/spec/1">
  <title id="root">Mini spec</title>
  <status stage="doc" state="done"/>
  <p><ALPHA fact="true" status="doc/done">The first claim, carried across the move unchanged.</ALPHA></p>
  <p><BETA fact="true" status="doc/done">The second claim, which cites `other.xml` and moves with it.</BETA></p>
  <named title="A named section">
    <status stage="doc" state="done"/>
    <p><GAMMA fact="true" status="doc/done">A claim inside an anchored unit.</GAMMA></p>
  </named>
</spec>
"""

# The same document with `GAMMA` deleted — a verdict with nowhere to land.
MINI_XML_NO_GAMMA = MINI_XML.replace(
    '    <p><GAMMA fact="true" status="doc/done">A claim inside an anchored unit.</GAMMA></p>\n', "")

# …and the same document minting one id the PRE-migration parse never had — a
# historical gap: the anchor is there once at the target, so no verdict is
# orphaned, but there is nothing behind it to compare against.
# It goes in the PREAMBLE, before the first section: this dialect requires that
# ordering and the engine refuses a `<p>` after a top-level section.
MINI_XML_PLUS_DELTA = MINI_XML.replace(
    '  <named title="A named section">',
    '  <p><DELTA fact="true" status="doc/done">A claim the live document mints and the '
    'pre-migration one never did.</DELTA></p>\n  <named title="A named section">')

# A document that stays Markdown across the move — 98 real ones do.
KEEP_MD = """# Kept in Markdown {#root}

<status stage="doc" state="done"/>

@fact:EPSILON A claim in a document the conversion did not touch. @status:doc/done
"""

# …and the same document minting its one id twice.
KEEP_MD_DUPLICATE = KEEP_MD + """
@fact:EPSILON A second paragraph minting the same id. @status:doc/done
"""

OLD_SCOPE = ('schema = 1\ninclude = ["spec/**/*.md", "spec/**/*.xml"]\n'
             '\n[progress]\ncache_dir = ".payloads"\n')
LIVE_SCOPE = ('schema = 1\ninclude = ["vibevm/vibespecs/**/*.md", "vibevm/vibespecs/**/*.xml"]\n'
              '\n[progress]\ncache_dir = ".payloads"\n')


def write_tree(base, scope, files):
    base.mkdir(parents=True, exist_ok=True)
    (base / "facts.toml").write_text(scope, encoding="utf-8")
    for rel, body in files.items():
        target = base / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(body, encoding="utf-8")
    return base


def mirror_of(vibe, tree):
    """Parse `tree` with the shipped parser and hand back its `Mirror`.

    Goes through `corpus_migration.mirror_tree`, the same call the migration
    makes, so a fixture is never parsed by a path the product does not use.
    """
    return mig.mirror_tree(vibe, tree, tree)


def fixture(tmp, live_files=None):
    """The pre-migration tree, the live tree and both mirrors.

    `live_files` overrides the live side so a case can break exactly one thing
    — a deleted anchor, a duplicated id — and leave the rest identical.
    """
    vibe = mig.vibe_command(ROOT, None)
    old = write_tree(tmp / "old", OLD_SCOPE, {"spec/mini.md": MINI_MD, "spec/keep.md": KEEP_MD})
    live = write_tree(tmp / "live", LIVE_SCOPE, live_files or {
        "vibevm/vibespecs/mini.xml": MINI_XML,
        "vibevm/vibespecs/keep.md": KEEP_MD,
    })
    return old, live, mirror_of(vibe, old), mirror_of(vibe, live)


# --------------------------------------------------------------------------
# Synthetic campaign state over the miniature corpus
# --------------------------------------------------------------------------


def verdict(anchor):
    return {"v": "confirmed",
            "ev": [f"spec/mini.md:5  @fact:{anchor} evidence prose citing the OLD path",
                   "an existence-and-reachability claim, resolved"]}


def make_zone(zone, old_mirror):
    """A campaign zone whose records name the pre-migration paths."""
    (zone / "run" / "state").mkdir(parents=True, exist_ok=True)
    mini, keep = old_mirror.doc("spec/mini.md"), old_mirror.doc("spec/keep.md")

    def record(doc, anchors, extra=None):
        campaign = {"processed_hash": doc["content_hash"], "verified_at": "2026-08-01T12:00:00Z",
                    "verify_batch": "MINI-1",
                    "verdicts": {a: verdict(a) for a in anchors}}
        campaign["verdicts"].update(extra or {})
        return {"content_hash": doc["content_hash"],
                "rollup": {"explicit": ["doc", "done"], "computed": ["doc", "done"],
                           "effective": ["doc", "done"], "marker_count": len(doc["markers"]),
                           "fact_count": doc["fact_count"], "unmarked_facts": 0},
                "marker_count": len(doc["markers"]), "unit_count": len(doc["units"]),
                "issue_count": len(doc["issues"]), "campaign": campaign}

    cache = {"schema": 2, "updated_at": "2026-08-02T00:00:00Z", "files": {
        "spec/mini.md": record(mini, ["ALPHA", "BETA", "GAMMA"],
                               {"_elements": {"v": "confirmed", "ev": ["document-level bundle"]}}),
        "spec/keep.md": record(keep, ["EPSILON"]),
    }}
    units = {}
    for addr, unit_hash in (("spec/mini.md#root", mini["units"][0]["content_hash"]),
                            # A LINE address, the form an anchor-less unit takes.
                            ("spec/mini.md#L%d" % mini["units"][1]["line_start"],
                             mini["units"][1]["content_hash"]),
                            ("spec/keep.md#root", keep["units"][0]["content_hash"])):
        units[addr] = {"addr": addr, "unit_hash": unit_hash, "verdict": "confirmed",
                       "evidence": ["spec/mini.md:5  evidence prose citing the OLD path"],
                       "verified_at": "2026-08-01T12:00:00Z", "crates": [],
                       "marker": "doc/done"}
    baseline = {"schema": 1, "written_at": "2026-08-02T00:00:00Z",
                "campaign_id": "mini", "units": units}
    corpus = {"schema": 1, "updated_at": "2026-08-02T00:00:00Z",
              "files": [{"path": p, "markers": 1, "units": 1, "facts": 1,
                         "campaign": {"processed_hash": "x"}}
                        for p in ("spec/mini.md", "spec/keep.md")]}
    (zone / "run" / "cache.json").write_bytes(mig.dumps(cache))
    (zone / "baseline.json").write_bytes(mig.dumps(baseline))
    (zone / "run" / "state" / "corpus.json").write_bytes(mig.dumps(corpus))
    (zone / "run" / "journal.jsonl").write_text(
        '{"at":"2026-08-01T12:00:00Z","kind":"phase","phase":"C"}\n', encoding="utf-8")
    return zone


def run_tool(zone, live, old_mirror, new_mirror, *extra):
    out = io.StringIO()
    code = cli_main(["--root", str(live), "--zone", str(zone),
                     "--old-mirror", str(old_mirror.dir), "--new-mirror", str(new_mirror.dir),
                     *extra], out=out)
    return code, out.getvalue()


def state_bytes(zone):
    return {p: (zone / p).read_bytes() for p in
            ("run/cache.json", "baseline.json", "run/state/corpus.json", "run/journal.jsonl")}


# --------------------------------------------------------------------------
# Cases
# --------------------------------------------------------------------------

CASES = []


def case(fn):
    CASES.append(fn)
    return fn


def tree(*paths):
    """An `exists` predicate over a fixed set of live paths."""
    live = set(paths)
    return lambda p: p in live


@case
def mapper_rewrites_both_roots_and_the_package_inner_root():
    assert mig.relayout("spec/common/PROP-000.md") == "vibevm/vibespecs/common/PROP-000.md"
    assert (mig.relayout("packages/org.x/pkg/v1.0.0/spec/A.md")
            == "vibevm/vibepacks/org.x/pkg/v1.0.0/vibevm/vibespecs/A.md")
    assert (mig.relayout("packages/org.x/pkg/v1.0.0/README.md")
            == "vibevm/vibepacks/org.x/pkg/v1.0.0/README.md")


@case
def mapper_treats_a_deeper_spec_segment_as_ordinary_text():
    # Only the segment AT the slot depth is the package's own spec root; a
    # directory called `spec` further down is part of the document's path.
    assert (mig.relayout("packages/org.x/pkg/v1.0.0/spec/spec/A.md")
            == "vibevm/vibepacks/org.x/pkg/v1.0.0/vibevm/vibespecs/spec/A.md")


@case
def mapper_leaves_a_live_layout_path_alone():
    # The signal a second `--apply` reads: nothing left to move.
    assert mig.relayout("vibevm/vibespecs/common/PROP-000.xml") is None


@case
def mapper_inverts_exactly_for_every_shape_the_corpus_holds():
    # `text-stability.py` walks this backwards to reach a verdict revision, so
    # a direction that is not a true inverse would silently read the wrong file.
    for legacy in ("spec/common/PROP-000.md",
                   "spec/boot/00-core.md",
                   "packages/org.x/pkg/v1.0.0/spec/A.md",
                   "packages/org.x/pkg/v1.0.0/spec/deep/spec/B.md",
                   "packages/org.x/pkg/v1.0.0/README.md"):
        assert mig.unrelayout(mig.relayout(legacy)) == legacy, legacy
    assert mig.unrelayout("Cargo.toml") is None


@case
def mapper_orders_historical_candidates_newest_shape_first():
    live = "vibevm/vibepacks/org.x/pkg/v1.0.0/vibevm/vibespecs/A.xml"
    assert mig.historical_candidates(live) == [
        live,
        "packages/org.x/pkg/v1.0.0/spec/A.xml",
        "packages/org.x/pkg/v1.0.0/spec/A.md",
        "vibevm/vibepacks/org.x/pkg/v1.0.0/vibevm/vibespecs/A.md",
    ]
    # A document that never left Markdown offers exactly two shapes.
    assert mig.historical_candidates("vibevm/vibespecs/design/README.md") == [
        "vibevm/vibespecs/design/README.md", "spec/design/README.md"]


@case
def mapper_prefers_the_kept_markdown_over_an_xml_twin():
    exists = tree("vibevm/vibespecs/a.md", "vibevm/vibespecs/a.xml")
    assert mig.map_path("spec/a.md", exists) == ("vibevm/vibespecs/a.md", "kept")


@case
def mapper_falls_back_to_the_xml_twin_only_when_the_markdown_is_gone():
    exists = tree("vibevm/vibespecs/a.xml")
    assert mig.map_path("spec/a.md", exists) == ("vibevm/vibespecs/a.xml", "xml")


@case
def mapper_refuses_an_absent_target_and_names_it():
    exists = tree("vibevm/vibespecs/other.xml")
    try:
        mig.build_map(["spec/a.md"], exists)
    except mig.Refusal as refusal:
        assert "vibevm/vibespecs/a.md" in " ".join(refusal.details), refusal.details
        return
    raise AssertionError("an absent target must refuse")


@case
def mapper_refuses_a_collision_and_names_both_sources():
    # `spec/a.md` maps onto the `.xml` twin; `spec/a.xml` maps onto it
    # directly. One live file, two verdict maps — the loss would look
    # exactly like success, so it must never be silent.
    exists = tree("vibevm/vibespecs/a.xml")
    try:
        mig.build_map(["spec/a.md", "spec/a.xml"], exists)
    except mig.Refusal as refusal:
        joined = " ".join(refusal.details)
        assert "collision" in joined and "spec/a.md" in joined and "spec/a.xml" in joined
        return
    raise AssertionError("a collision must refuse")


@case
def migration_carries_every_anchor_and_detects_the_one_fact_that_moved():
    with tempfile.TemporaryDirectory() as tmp:
        tmp = pathlib.Path(tmp)
        _old, live, old_mirror, new_mirror = fixture(tmp)
        zone = make_zone(tmp / "zone", old_mirror)
        cache = json.loads((zone / "run" / "cache.json").read_text(encoding="utf-8"))
        mapping = mig.build_map(sorted(cache["files"]), lambda p: (live / p).is_file())
        assert mapping == {"spec/mini.md": "vibevm/vibespecs/mini.xml",
                           "spec/keep.md": "vibevm/vibespecs/keep.md"}, mapping
        assert dict(mig.split_kinds(mapping)) == {"xml": 1, "kept": 1}

        cmp_ = mig.compare_corpus(mapping, cache, old_mirror, new_mirror)
        assert cmp_.anchors == 4, cmp_.anchors
        # `_elements` is a document-level bundle: counted, never chased for an
        # anchor it has never had.
        assert cmp_.elements == 1
        assert not cmp_.absent and not cmp_.duplicated and not cmp_.structure
        assert not cmp_.historical_gap, cmp_.historical_gap
        assert cmp_.compared == 4, cmp_.compared
        # THE POSITIVE CONTROL: exactly the fact whose prose the conversion
        # rewrote is reported, and the two that were carried untouched are not.
        assert [a for _o, _l, a in cmp_.moved] == ["BETA"], cmp_.moved


@case
def migration_names_a_historical_gap_instead_of_calling_it_content_drift():
    # B-107 repair 3. `DELTA` is minted once at the mapped path and never in the
    # pre-migration parse. The shape this code had before folded that into
    # "the fact's text moved" through a `.get(anchor, [None])[0]` default — a
    # `None` reported as content drift. On the live corpus it inflated the
    # headline from 1082 genuinely moved facts to 1083.
    with tempfile.TemporaryDirectory() as tmp:
        tmp = pathlib.Path(tmp)
        _old, live, old_mirror, new_mirror = fixture(tmp, {
            "vibevm/vibespecs/mini.xml": MINI_XML_PLUS_DELTA,
            "vibevm/vibespecs/keep.md": KEEP_MD,
        })
        zone = make_zone(tmp / "zone", old_mirror)
        cache_path = zone / "run" / "cache.json"
        cache = json.loads(cache_path.read_text(encoding="utf-8"))
        cache["files"]["spec/mini.md"]["campaign"]["verdicts"]["DELTA"] = verdict("DELTA")
        cache_path.write_bytes(mig.dumps(cache))

        mapping = mig.build_map(sorted(cache["files"]), lambda p: (live / p).is_file())
        cmp_ = mig.compare_corpus(mapping, cache, old_mirror, new_mirror)
        # No verdict is orphaned, so this is NOT structural and the move may go on…
        assert not cmp_.structural_failures, cmp_.structural_failures
        # …but it is its own class, named, and it is not a move.
        assert [(a, w) for _o, _l, a, w in cmp_.historical_gap] \
            == [("DELTA", "absent from the pre-migration parse")], cmp_.historical_gap
        assert [a for _o, _l, a in cmp_.moved] == ["BETA"], cmp_.moved
        assert cmp_.anchors == 5 and cmp_.compared == 4, (cmp_.anchors, cmp_.compared)

        # The gate needs its own admission: pinning the moved count is not enough.
        before = state_bytes(zone)
        try:
            run_tool(zone, live, old_mirror, new_mirror, "--apply", "--accept-moved-facts", "1")
        except mig.Refusal as refusal:
            assert "--accept-historical-gaps 1" in refusal.headline, refusal.headline
            assert state_bytes(zone) == before, "a refusal must write nothing"
        else:
            raise AssertionError("an unacknowledged historical gap must refuse")

        code, text = run_tool(zone, live, old_mirror, new_mirror, "--apply",
                              "--accept-moved-facts", "1", "--accept-historical-gaps", "1")
        assert code == 0, text
        after = json.loads((zone / "run" / "cache.json").read_text(encoding="utf-8"))
        # The gap's verdict is CARRIED, never dropped and never re-judged.
        assert (after["files"]["vibevm/vibespecs/mini.xml"]["campaign"]["verdicts"]["DELTA"]
                == cache["files"]["spec/mini.md"]["campaign"]["verdicts"]["DELTA"])
        assert state_bytes(zone)["run/journal.jsonl"] == before["run/journal.jsonl"]


@case
def migration_refuses_when_an_anchor_has_nowhere_to_land():
    with tempfile.TemporaryDirectory() as tmp:
        tmp = pathlib.Path(tmp)
        _old, live, old_mirror, new_mirror = fixture(tmp, {
            "vibevm/vibespecs/mini.xml": MINI_XML_NO_GAMMA,
            "vibevm/vibespecs/keep.md": KEEP_MD,
        })
        zone = make_zone(tmp / "zone", old_mirror)
        before = state_bytes(zone)
        try:
            run_tool(zone, live, old_mirror, new_mirror, "--apply", "--accept-moved-facts", "1")
        except mig.Refusal as refusal:
            assert "structural" in refusal.headline, refusal.headline
            assert state_bytes(zone) == before, "a refusal must write nothing"
            return
        raise AssertionError("an orphaned verdict must refuse")


@case
def migration_refuses_when_the_target_mints_the_anchor_twice():
    with tempfile.TemporaryDirectory() as tmp:
        tmp = pathlib.Path(tmp)
        _old, live, old_mirror, new_mirror = fixture(tmp, {
            "vibevm/vibespecs/mini.xml": MINI_XML,
            "vibevm/vibespecs/keep.md": KEEP_MD_DUPLICATE,
        })
        zone = make_zone(tmp / "zone", old_mirror)
        cache = json.loads((zone / "run" / "cache.json").read_text(encoding="utf-8"))
        mapping = mig.build_map(sorted(cache["files"]), lambda p: (live / p).is_file())
        cmp_ = mig.compare_corpus(mapping, cache, old_mirror, new_mirror)
        assert [(a, n) for _o, _l, a, n in cmp_.duplicated] == [("EPSILON", 2)], cmp_.duplicated
        assert cmp_.structural_failures, "a duplicate target is structural"


@case
def migration_remaps_a_line_address_through_the_unit_ordinal():
    with tempfile.TemporaryDirectory() as tmp:
        tmp = pathlib.Path(tmp)
        _old, live, old_mirror, new_mirror = fixture(tmp)
        zone = make_zone(tmp / "zone", old_mirror)
        baseline = json.loads((zone / "baseline.json").read_text(encoding="utf-8"))
        cache = json.loads((zone / "run" / "cache.json").read_text(encoding="utf-8"))
        mapping = mig.build_map(sorted(cache["files"]), lambda p: (live / p).is_file())
        units, how, unresolved = mig.remap_baseline(baseline, mapping, old_mirror, new_mirror)
        assert not unresolved, unresolved
        assert len(units) == len(baseline["units"])
        # `#L9` names a line, which no move preserves; the ordinal does, and
        # the live unit at that ordinal carries an anchor — a better address
        # than the one it replaces.
        assert "vibevm/vibespecs/mini.xml#named" in units, sorted(units)
        assert units["vibevm/vibespecs/mini.xml#named"]["addr"] \
            == "vibevm/vibespecs/mini.xml#named"
        assert how["line-by-ordinal"] == 1 and how["anchor"] == 2, how


@case
def migration_refuses_moved_facts_until_the_count_is_pinned():
    with tempfile.TemporaryDirectory() as tmp:
        tmp = pathlib.Path(tmp)
        _old, live, old_mirror, new_mirror = fixture(tmp)
        zone = make_zone(tmp / "zone", old_mirror)
        before = state_bytes(zone)
        for extra in (("--apply",), ("--apply", "--accept-moved-facts", "7")):
            try:
                run_tool(zone, live, old_mirror, new_mirror, *extra)
            except mig.Refusal as refusal:
                assert "--accept-moved-facts 1" in refusal.headline, refusal.headline
            else:
                raise AssertionError(f"{extra} must refuse")
            assert state_bytes(zone) == before, "a refusal must write nothing"
        code, _ = run_tool(zone, live, old_mirror, new_mirror,
                           "--check", "--accept-moved-facts", "1")
        assert code == 0
        assert state_bytes(zone) == before, "--check must write nothing"


@case
def migration_applies_once_and_the_second_apply_is_a_no_op():
    with tempfile.TemporaryDirectory() as tmp:
        tmp = pathlib.Path(tmp)
        _old, live, old_mirror, new_mirror = fixture(tmp)
        zone = make_zone(tmp / "zone", old_mirror)
        before = state_bytes(zone)
        old_cache = json.loads(before["run/cache.json"].decode("utf-8"))

        code, _ = run_tool(zone, live, old_mirror, new_mirror,
                           "--apply", "--accept-moved-facts", "1")
        assert code == 0
        after = state_bytes(zone)
        cache = json.loads(after["run/cache.json"].decode("utf-8"))
        baseline = json.loads(after["baseline.json"].decode("utf-8"))
        corpus = json.loads(after["run/state/corpus.json"].decode("utf-8"))

        assert set(cache["files"]) == {"vibevm/vibespecs/mini.xml", "vibevm/vibespecs/keep.md"}
        assert not set(cache["files"]) & set(old_cache["files"]), "key sets must be disjoint"
        assert [row["path"] for row in corpus["files"]] == sorted(cache["files"])
        # Verdicts, evidence prose and timestamps ride through untouched —
        # including evidence that still cites the pre-migration path, which is
        # a record of what was read and not a live pointer.
        assert (cache["files"]["vibevm/vibespecs/mini.xml"]["campaign"]
                == old_cache["files"]["spec/mini.md"]["campaign"])
        assert set(baseline["units"]) == {"vibevm/vibespecs/mini.xml#root",
                                          "vibevm/vibespecs/mini.xml#named",
                                          "vibevm/vibespecs/keep.md#root"}
        assert after["run/journal.jsonl"] == before["run/journal.jsonl"], "journal is append-only"

        code, text = run_tool(zone, live, old_mirror, new_mirror,
                              "--apply", "--accept-moved-facts", "1")
        assert code == 0 and "no-op" in text, text
        assert state_bytes(zone) == after, "a second apply must change nothing"


@case
def migration_publishes_all_three_artifacts_or_none():
    with tempfile.TemporaryDirectory() as tmp:
        tmp = pathlib.Path(tmp)
        first, second = tmp / "a.json", tmp / "b.json"
        first.write_bytes(b"ORIGINAL-A")
        second.write_bytes(b"ORIGINAL-B")
        # The third target is a non-empty directory, so its rename fails after
        # the first two have already landed — the exact shape of a torn write.
        blocked = tmp / "c.json"
        (blocked / "inside").mkdir(parents=True)
        try:
            mig.publish({first: b"NEW-A", second: b"NEW-B", blocked: b"NEW-C"})
        except OSError:
            assert first.read_bytes() == b"ORIGINAL-A", "a torn write must be rolled back"
            assert second.read_bytes() == b"ORIGINAL-B", "a torn write must be rolled back"
            assert not list(tmp.glob("*.migrate~")), "staging files must not survive"
            return
        raise AssertionError("a blocked target must raise")


@case
def this_machinery_holds_the_repository_line_budget():
    # `conform.toml` `max_file_lines`. The budget is why this machinery is a
    # package rather than one script, so the package asserts it about itself —
    # a budget nothing measures is a budget already broken somewhere. Scoped to
    # the files these two programs own: the older campaign scripts beside them
    # are somebody else's debt and are reported, not silently adopted.
    budget = 600
    for line in (ROOT / "conform.toml").read_text(encoding="utf-8").splitlines():
        if line.strip().startswith("max_file_lines"):
            budget = int(line.split("=")[1].strip())
    owned = [HERE / name for name in ("corpus-migration.py", "corpus-migration-test.py",
                                      "text-stability.py", "text-stability-test.py")]
    owned += sorted((HERE / "corpus_migration").glob("*.py"))
    over = [f"{p.name} {len(p.read_text(encoding='utf-8').splitlines())}"
            for p in owned if p.is_file()
            and len(p.read_text(encoding="utf-8").splitlines()) > budget]
    assert not over, f"over the {budget}-line budget: {over}"


# --------------------------------------------------------------------------


def main(argv):
    only = argv[0] if argv else ""
    if not mig.have_vibe(ROOT):
        print("SKIP — no shipped CLI (build it, or set VIBE_BIN); the mapper cases still run")
    failed = 0
    for fn in CASES:
        if only and only not in fn.__name__:
            continue
        try:
            fn()
            print(f"  ok    {fn.__name__}")
        except Exception as exc:  # noqa: BLE001 — a test runner reports, never raises
            failed += 1
            print(f"  FAIL  {fn.__name__}: {type(exc).__name__}: {exc}")
    total = sum(1 for fn in CASES if not only or only in fn.__name__)
    print(f"\n{total - failed}/{total} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
