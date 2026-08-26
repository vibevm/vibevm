#!/usr/bin/env python3
"""Focused tests for `text-stability.py`, on a miniature corpus with real history.

Usage:
    python tasks/text-stability-test.py           # every case
    python tasks/text-stability-test.py refus     # cases whose name contains this

The fixture is a THROWAWAY git repository under the system temp directory, built
and thrown away inside each case. Nothing here touches the repository this file
lives in — no index, no branch, no commit of the project's own. It has to be a
real repository because the bug being fixed was a bug about git: `git show` of a
path that does not exist at a revision prints zero bytes, and the old program
read that silence as "the document was empty and therefore unchanged". Only real
`git cat-file` behaviour proves that absence is now detected instead of inferred.

Its timeline is the real corpus in miniature:

    C1  2026-01-01   spec/*.md                      — the pre-relayout corpus
    C2  2026-02-01   vibevm/vibespecs/*.xml         — relayout + Markdown→XML,
                                                      plus one document born here
    C3  2026-03-01   one more document born         — after every verdict

and the seven documents cover one case each: a fact carried across MD→XML
untouched, a fact whose citation the conversion rewrote, a fact whose own words
changed, a fact whose MARKUP was respelled and whose words were not, a document
whose live path already existed at its verdict revision, a document that did not
exist at its verdict revision under any shape, and a document whose historical
revision holds no marked facts at all — the shape that used to certify 404 files
as sealable over text nobody read.

Needs the shipped CLI: `target/debug/vibe`, `$VIBE_BIN`, or `cargo` on PATH.
Exit code 0 when every case passes, 1 otherwise.
"""

import io
import json
import pathlib
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parent.parent.parent
sys.path.insert(0, str(HERE))

import corpus_migration as mig  # noqa: E402
from corpus_migration import stability  # noqa: E402

LIVE_SCOPE = ('schema = 1\ninclude = ["spec/**/*.md", "spec/**/*.xml",\n'
              '           "vibevm/vibespecs/**/*.md", "vibevm/vibespecs/**/*.xml"]\n'
              '\n[progress]\ncache_dir = ".payloads"\n')


def md(anchor, body):
    return f"# Mini {{#root}}\n\n<status stage=\"doc\" state=\"done\"/>\n\n@fact:{anchor} {body} @status:doc/done\n"


def legacy_md(anchor, body):
    """The pre-2026-08-06 spelling: `##ID …  @stage/state`. Same claim, other
    markup — and the engine's content hash is defined to be blind to exactly
    this difference (`progress_core::parse::canonical_markup`)."""
    return f"# Mini {{#root}}\n\n<status stage=\"doc\" state=\"done\"/>\n\n##{anchor} {body} @doc/done\n"


def xml(anchor, body):
    return ('<?xml version="1.0" encoding="UTF-8"?>\n'
            '<spec xmlns="https://vibevm.org/spec/1">\n'
            '  <title id="root">Mini</title>\n'
            '  <status stage="doc" state="done"/>\n'
            f'  <p><{anchor} fact="true" status="doc/done">{body}</{anchor}></p>\n'
            '</spec>\n')


STABLE = "The claim that the conversion carried across untouched."
CITED_MD = "The claim that points at `other.md` and nothing else."
CITED_XML = "The claim that points at `other.xml` and nothing else."
PROSE_OLD = "The registry keeps three copies of every index."
PROSE_NEW = "The registry keeps four copies of every index."
# At C1 this document is a stub with NO marked fact at all — the empty domain.
EMPTY_MD = "# Mini {#root}\n\n<status stage=\"doc\" state=\"done\"/>\n"

RESPELLED = "The claim whose markup changed and whose words did not."

C1 = {"spec/stable.md": md("ALPHA", STABLE),
      "spec/moved-citation.md": md("BETA", CITED_MD),
      "spec/moved-prose.md": md("GAMMA", PROSE_OLD),
      "spec/empty-domain.md": EMPTY_MD,
      "spec/respelled.md": legacy_md("ETA", RESPELLED)}
C2 = {"vibevm/vibespecs/stable.xml": xml("ALPHA", STABLE),
      "vibevm/vibespecs/moved-citation.xml": xml("BETA", CITED_XML),
      "vibevm/vibespecs/moved-prose.xml": xml("GAMMA", PROSE_NEW),
      "vibevm/vibespecs/empty-domain.xml": xml("DELTA", "A claim this document grew later."),
      "vibevm/vibespecs/respelled.xml": xml("ETA", RESPELLED),
      "vibevm/vibespecs/postmig.xml": xml("EPSILON", "A claim born already in XML.")}
C3 = {"vibevm/vibespecs/newborn.xml": xml("ZETA", "A claim younger than its own verdict.")}

# Which revision each document's verdicts were formed at.
STAMPS = {"vibevm/vibespecs/stable.xml": "2026-01-15T00:00:00Z",
          "vibevm/vibespecs/moved-citation.xml": "2026-01-15T00:00:00Z",
          "vibevm/vibespecs/moved-prose.xml": "2026-01-15T00:00:00Z",
          "vibevm/vibespecs/empty-domain.xml": "2026-01-15T00:00:00Z",
          "vibevm/vibespecs/respelled.xml": "2026-01-15T00:00:00Z",
          "vibevm/vibespecs/postmig.xml": "2026-02-15T00:00:00Z",
          "vibevm/vibespecs/newborn.xml": "2026-01-15T00:00:00Z"}
ANCHORS = {"vibevm/vibespecs/stable.xml": ["ALPHA"],
           "vibevm/vibespecs/moved-citation.xml": ["BETA"],
           "vibevm/vibespecs/moved-prose.xml": ["GAMMA"],
           "vibevm/vibespecs/empty-domain.xml": ["DELTA"],
           "vibevm/vibespecs/respelled.xml": ["ETA"],
           "vibevm/vibespecs/postmig.xml": ["EPSILON"],
           "vibevm/vibespecs/newborn.xml": ["ZETA"]}


def run_git(repo, *args, date=None):
    env = None
    if date:
        import os
        env = dict(os.environ, GIT_AUTHOR_DATE=date, GIT_COMMITTER_DATE=date)
    proc = subprocess.run(["git", "-C", str(repo), "-c", "user.name=fixture",
                           "-c", "user.email=fixture@example.invalid", *args],
                          capture_output=True, text=True, encoding="utf-8",
                          errors="replace", env=env)
    if proc.returncode != 0:
        raise AssertionError(f"git {' '.join(args)} failed in the fixture:\n{proc.stderr}")
    return proc.stdout


def write(repo, files):
    for rel, body in files.items():
        target = repo / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(body, encoding="utf-8")


def build_repo(tmp):
    """The three-commit fixture, and the live mirror of its working tree."""
    repo = tmp / "repo"
    repo.mkdir()
    run_git(repo, "init", "-q")
    (repo / "facts.toml").write_text(LIVE_SCOPE, encoding="utf-8")
    write(repo, C1)
    run_git(repo, "add", "-A")
    run_git(repo, "commit", "-q", "-m", "C1 the pre-relayout corpus",
            date="2026-01-01T00:00:00Z")
    for rel in C1:
        (repo / rel).unlink()
    write(repo, C2)
    run_git(repo, "add", "-A")
    run_git(repo, "commit", "-q", "-m", "C2 relayout and convert",
            date="2026-02-01T00:00:00Z")
    write(repo, C3)
    run_git(repo, "add", "-A")
    run_git(repo, "commit", "-q", "-m", "C3 a later document",
            date="2026-03-01T00:00:00Z")
    vibe = mig.vibe_command(ROOT)
    zone = repo / "campaigns" / "z"
    live = mig.mirror_tree(vibe, repo, repo, "campaigns/z")
    return repo, zone, live, vibe


def build_cache(live):
    """A campaign cache over the fixture, agreeing with the live mirror by
    construction — the currency guard is a separate case, not a trap here."""
    files = {}
    for path, anchors in ANCHORS.items():
        digest = live.content_hash(path)
        files[path] = {
            "content_hash": digest,
            "rollup": {"fact_count": 1, "unmarked_facts": 0},
            "marker_count": 1, "unit_count": 1, "issue_count": 0,
            "campaign": {"processed_hash": digest, "verified_at": STAMPS[path],
                         "verify_batch": "MINI",
                         "verdicts": {a: {"v": "confirmed", "ev": ["…"]} for a in anchors}},
        }
    return {"schema": 2, "updated_at": "2026-03-02T00:00:00Z", "files": files}


def analyse_fixture(tmp, **kw):
    repo, zone, live, vibe = build_repo(tmp)
    cache = build_cache(live)
    outcomes, stats = stability.analyse(vibe, repo, zone, cache, live, **kw)
    return repo, zone, live, cache, outcomes, stats


def outcome_of(outcomes, stem):
    return outcomes[f"vibevm/vibespecs/{stem}.xml"]


CASES = []


def case(fn):
    CASES.append(fn)
    return fn


@case
def a_fact_carried_across_md_to_xml_is_sealable():
    with tempfile.TemporaryDirectory() as tmp:
        _r, _z, _l, _c, outcomes, _s = analyse_fixture(pathlib.Path(tmp))
        got = outcome_of(outcomes, "stable")
        assert got.historical == "spec/stable.md", got.historical
        assert got.compared == 1 and not got.moved and not got.refusals, vars(got)
        assert got.sealable


@case
def a_rewritten_citation_moves_the_fact():
    with tempfile.TemporaryDirectory() as tmp:
        _r, _z, _l, _c, outcomes, _s = analyse_fixture(pathlib.Path(tmp))
        got = outcome_of(outcomes, "moved-citation")
        assert got.historical == "spec/moved-citation.md"
        assert got.moved == ["BETA"], got.moved
        assert not got.sealable and not got.refusals


@case
def substantive_prose_moves_the_fact():
    with tempfile.TemporaryDirectory() as tmp:
        _r, _z, _l, _c, outcomes, _s = analyse_fixture(pathlib.Path(tmp))
        got = outcome_of(outcomes, "moved-prose")
        assert got.moved == ["GAMMA"], got.moved
        assert not got.sealable


@case
def a_post_migration_live_path_resolves_to_itself():
    # Its verdicts were formed AFTER the relayout, so the identity search stops
    # on the first candidate — the live path — without inventing a legacy shape.
    with tempfile.TemporaryDirectory() as tmp:
        _r, _z, _l, _c, outcomes, _s = analyse_fixture(pathlib.Path(tmp))
        got = outcome_of(outcomes, "postmig")
        assert got.historical == got.path, got.historical
        assert got.compared == 1 and got.sealable, vars(got)


@case
def an_absent_historical_path_refuses_and_names_every_shape_it_tried():
    with tempfile.TemporaryDirectory() as tmp:
        _r, _z, _l, _c, outcomes, _s = analyse_fixture(pathlib.Path(tmp))
        got = outcome_of(outcomes, "newborn")
        assert got.refusals and not got.sealable, vars(got)
        why = got.refusals[0]
        assert "no identity at" in why, why
        for shape in ("vibevm/vibespecs/newborn.xml", "spec/newborn.xml", "spec/newborn.md",
                      "vibevm/vibespecs/newborn.md"):
            assert shape in why, (shape, why)
        assert got.compared == 0


@case
def an_empty_historical_domain_refuses_instead_of_certifying_silence():
    # THE POSITIVE CONTROL for the bug this program was rewritten to fix. At its
    # verdict revision the document holds no marked fact at all; the old
    # implementation found nothing on either side, compared the two nothings
    # equal, and called the file SEALABLE. Absence must refuse.
    with tempfile.TemporaryDirectory() as tmp:
        _r, _z, _l, _c, outcomes, _s = analyse_fixture(pathlib.Path(tmp))
        got = outcome_of(outcomes, "empty-domain")
        assert got.historical == "spec/empty-domain.md", got.historical
        assert got.compared == 0, "an absent anchor is never a comparison"
        assert any("absent from the historical parse" in r for r in got.refusals), got.refusals
        assert not got.sealable


@case
def a_markup_respelling_is_not_a_move():
    # `##ETA … @doc/done` became `@fact:ETA … @status:doc/done` and the words did
    # not change. The engine's content hash canonicalises exactly that, so the
    # fact did not move — and a comparison that decided on the raw BODY would
    # report the whole 2026-08-06 respelling as a corpus-wide content change.
    # Measured on the real corpus while this program was being written: 8 275
    # facts respelled, 981 actually moved.
    with tempfile.TemporaryDirectory() as tmp:
        _r, _z, _l, _c, outcomes, _s = analyse_fixture(pathlib.Path(tmp))
        got = outcome_of(outcomes, "respelled")
        assert got.historical == "spec/respelled.md", got.historical
        assert got.compared == 1 and not got.moved, vars(got)
        assert got.respelled == 1, "the respelling must be seen, and named as not a move"
        assert got.sealable


@case
def sealable_names_only_documents_that_were_actually_compared():
    with tempfile.TemporaryDirectory() as tmp:
        _r, _z, _l, _c, outcomes, _s = analyse_fixture(pathlib.Path(tmp))
        sealable, rejudge, refused, _nothing = stability.buckets(outcomes)
        assert sorted(o.path for o in sealable) == ["vibevm/vibespecs/postmig.xml",
                                                    "vibevm/vibespecs/respelled.xml",
                                                    "vibevm/vibespecs/stable.xml"]
        assert sorted(o.path for o in rejudge) == ["vibevm/vibespecs/moved-citation.xml",
                                                   "vibevm/vibespecs/moved-prose.xml"]
        assert sorted(o.path for o in refused) == ["vibevm/vibespecs/empty-domain.xml",
                                                   "vibevm/vibespecs/newborn.xml"]
        for outcome in sealable:
            assert outcome.compared > 0 and not outcome.refusals


@case
def a_corpus_that_changed_serialisation_wholesale_still_reports():
    # Four of six documents changed serialisation between their verdict and now.
    # The old program answered that shape with a 90 %-of-files heuristic and a
    # REFUSING TO REPORT banner, because it could not tell "the corpus moved"
    # from "my hash recipe drifted". There is one recipe now — the engine's — so
    # the question cannot arise and the banner does not exist.
    with tempfile.TemporaryDirectory() as tmp:
        _r, _z, _l, _c, outcomes, stats = analyse_fixture(pathlib.Path(tmp))
        text = io.StringIO()
        stability.render(outcomes, stats, 25, text)
        body = text.getvalue()
        assert "REFUSING TO REPORT" not in body
        assert "SEALABLE" in body and "RE-JUDGE" in body and "REFUSED" in body
        # …and there is nothing left that COULD drift: no second hash, no
        # canonicalisation of its own, no ratio deciding whom to disbelieve.
        # Checked on the namespace and on the code, so a mention of the old
        # recipe in the prose that explains it does not read as a relapse.
        for banned in ("canonical_markup", "recipe_has_drifted", "hashlib", "sha256"):
            assert not hasattr(stability, banned), f"`{banned}` is back in the module"
        source = (HERE / "corpus_migration" / "stability.py").read_text(encoding="utf-8")
        code = "\n".join(line for line in source.splitlines()
                         if not line.lstrip().startswith(("#", "*")))
        code = code.split('"""')[0] + '"""'.join(code.split('"""')[2:])
        for banned in ("hashlib", "sha256", "9 // 10", '.replace("@fact:'):
            assert banned not in code, f"the rewrite must not carry `{banned}` back in"


@case
def the_live_mirror_must_describe_the_cache_it_is_read_beside():
    with tempfile.TemporaryDirectory() as tmp:
        tmp = pathlib.Path(tmp)
        repo, zone, live, vibe = build_repo(tmp)
        cache = build_cache(live)
        cache["files"]["vibevm/vibespecs/stable.xml"]["content_hash"] = "0" * 64
        try:
            stability.analyse(vibe, repo, zone, cache, live)
        except mig.Refusal as refusal:
            assert "different corpora" in refusal.headline, refusal.headline
            assert any("stable.xml" in d for d in refusal.details), refusal.details
            return
        raise AssertionError("a mirror that disagrees with its cache must refuse")


@case
def absence_is_proven_on_the_production_path_and_leaves_no_empty_file():
    # The two calls the shipping programs actually make — `blob_exists` for one
    # path, `materialise` for a revision's worth. There is no third absence
    # parser to test (B-107 repair 3 removed the unused `read_blob`, whose proof
    # certified a helper neither program ran). The second assertion is the one
    # that matters: an absent path leaves NO file behind, so a later parse can
    # never read an empty file as an empty document — the exact accident that
    # certified 404 files as sealable over text nobody had read.
    with tempfile.TemporaryDirectory() as tmp:
        tmp = pathlib.Path(tmp)
        repo, _z, _l, _v = build_repo(tmp)
        head = mig.git(repo, "rev-parse", "HEAD").strip()
        assert mig.blob_exists(repo, head, "vibevm/vibespecs/stable.xml")
        assert not mig.blob_exists(repo, head, "spec/stable.md")
        dest = tmp / "out"
        try:
            mig.materialise(repo, head, ["vibevm/vibespecs/stable.xml", "spec/gone.md"], dest)
        except mig.Refusal as refusal:
            assert "spec/gone.md" in refusal.details, refusal.details
            assert (dest / "vibevm/vibespecs/stable.xml").is_file()
            assert not (dest / "spec/gone.md").exists(), \
                "an absent path must leave no file at all, not an empty one"
            return
        raise AssertionError("materialising an absent path must refuse")


def build_spaced_repo(tmp):
    """A throwaway repository holding one document whose path carries spaces.

    Its own repository rather than a file added to `build_repo`, so the shared
    fixture's corpus — which several cases name path by path — stays exactly as
    it was.
    """
    repo = tmp / "spaced"
    repo.mkdir()
    run_git(repo, "init", "-q")
    (repo / "facts.toml").write_text(LIVE_SCOPE, encoding="utf-8")
    write(repo, {SPACED: md("SPACED", STABLE)})
    run_git(repo, "add", "-A")
    run_git(repo, "commit", "-q", "-m", "a document whose path holds spaces",
            date="2026-01-01T00:00:00Z")
    return repo, run_git(repo, "rev-parse", "HEAD").strip()


SPACED = "spec/a document with spaces.md"
SPACED_GONE = "spec/a document that is gone.md"


@case
def a_blob_path_with_spaces_materialises_instead_of_reading_as_absent():
    # `git cat-file --batch` answers a missing object by ECHOING the query and
    # appending a word, so `<rev>:a document with spaces.md missing` splits into
    # the same three tokens a real `<oid> blob <size>` header does. Counting
    # tokens therefore decides absence WRONGLY in both directions; the suffix
    # decides it correctly.
    with tempfile.TemporaryDirectory() as tmp:
        tmp = pathlib.Path(tmp)
        repo, head = build_spaced_repo(tmp)
        assert mig.blob_exists(repo, head, SPACED)
        dest = tmp / "out"
        mig.materialise(repo, head, [SPACED], dest)
        assert (dest / SPACED).read_text(encoding="utf-8") == md("SPACED", STABLE)


@case
def a_missing_path_with_spaces_refuses_and_never_raises_a_parse_error():
    with tempfile.TemporaryDirectory() as tmp:
        tmp = pathlib.Path(tmp)
        repo, head = build_spaced_repo(tmp)
        assert not mig.blob_exists(repo, head, SPACED_GONE)
        dest = tmp / "out"
        try:
            mig.materialise(repo, head, [SPACED, SPACED_GONE], dest)
        except mig.Refusal as refusal:
            assert SPACED_GONE in refusal.details, refusal.details
            assert (dest / SPACED).is_file(), "the blob that IS there still lands"
            assert not (dest / SPACED_GONE).exists()
            return
        except ValueError as exc:  # noqa: F841 — named so the failure reads
            raise AssertionError(f"absence must be typed, not a parse error: {exc}")
        raise AssertionError("an absent spaced path must refuse")


@case
def a_batch_header_this_module_cannot_read_refuses_with_its_own_diagnostic():
    assert mig.batch_header(b"a1b2c3 blob 42\n", "q") == 42
    assert mig.batch_header(b"deadbeef:spec/a b.md missing\n", "q") is None
    assert mig.batch_header(b"deadbeef:spec/a b.md ambiguous\n", "q") is None
    for bad in (b"a1b2c3 blob notanumber\n", b"nonsense\n", b"\n"):
        try:
            mig.batch_header(bad, "q")
        except mig.Refusal as refusal:
            assert "cannot read" in refusal.headline, refusal.headline
            continue
        raise AssertionError(f"{bad!r} must refuse")
    try:
        mig.batch_header(b"a1b2c3 tree 42\n", "q")
    except mig.Refusal as refusal:
        assert "not a blob" in refusal.headline, refusal.headline
        return
    raise AssertionError("a non-blob type must refuse")


@case
def a_refused_file_that_carries_moved_anchors_stays_out_of_the_rejudge_total():
    # THE SYNTHETIC RED for B-107 repair 3. Before it, `render` summed `moved`
    # over ALL outcomes and printed the result as the RE-JUDGE bucket's fact
    # count: on the live corpus, "191 files, 981 facts" over 191 rows listing
    # 859 anchors, the other 122 living inside files the same report called
    # "cannot be compared". This fixture is that shape in miniature, and it
    # fails loudly if the two domains are ever merged again.
    clean = stability.FileVerdict("live/clean.xml", 3)
    clean.compared, clean.moved = 3, ["ONE", "TWO"]
    poisoned = stability.FileVerdict("live/poisoned.xml", 40)
    poisoned.compared, poisoned.moved = 20, ["A", "B", "C", "D", "E", "F", "G"]
    poisoned.refusals.append("anchor `GONE` is absent from the historical parse")
    outcomes = {o.path: o for o in (clean, poisoned)}

    t = stability.tally(outcomes)
    assert (t["moved_rejudge"], t["moved_refused"], t["moved_corpus"]) == (2, 7, 9), t
    assert (t["rejudge_files"], t["refused_files"]) == (1, 1), t
    text = io.StringIO()
    stability.render(outcomes, {"revisions": 1, "scratch": "-"}, 25, text)
    body = text.getvalue()
    assert "RE-JUDGE — the fact's own text moved (1 files, 2 facts)" in body, body
    assert "MOVED INSIDE A REFUSED FILE (1 files, 7 facts)" in body, body
    assert "facts owed a re-judgement (RE-JUDGE bucket): 2" in body, body
    assert "+ 7 moved fact(s) inside refused files" in body, body
    # The corpus-wide number may appear, but never as the RE-JUDGE headline.
    assert "(1 files, 9 facts)" not in body and "(RE-JUDGE bucket): 9" not in body, body


def main(argv):
    only = argv[0] if argv else ""
    if not mig.have_vibe(ROOT):
        print("SKIP — no shipped CLI (build it, or set VIBE_BIN); this suite needs the engine")
        return 0
    failed = 0
    selected = [fn for fn in CASES if not only or only in fn.__name__]
    for fn in selected:
        try:
            fn()
            print(f"  ok    {fn.__name__}")
        except Exception as exc:  # noqa: BLE001 — a test runner reports, never raises
            failed += 1
            print(f"  FAIL  {fn.__name__}: {type(exc).__name__}: {exc}")
    print(f"\n{len(selected) - failed}/{len(selected)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
