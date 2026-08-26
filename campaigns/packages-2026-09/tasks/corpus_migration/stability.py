"""Which judged facts actually moved since their verdicts were formed.

The question is per FACT, and answering it needs the fact's text twice: as it was
at the revision the verdict was formed against, and as it is now. This module
gets both from `vibe progress mirror` — the shipped parser — and from nothing
else. Its predecessor got the historical side from a regex over `git show` and
the live side from a re-implemented `sha256(canonical_markup(bytes))`, and after
the B-107 migration both were wrong in the dangerous direction: `git show` of a
path that did not exist at the revision returns zero bytes, the regex found no
facts in raw XML either, every anchor compared equal to nothing, and 404 files
carrying 17 627 verdicts were reported SEALABLE over text the program had never
read.

Four rules follow from that, and they are the design:

* the parse is the engine's, on BOTH sides — no hand-parsing, no second hash
  recipe, and therefore no `#recipe-drift` heuristic to guess when the recipe has
  drifted (there is only one recipe now);
* absence is proven, never inferred from emptiness — `git cat-file -e` decides
  whether a blob is there, and a missing historical identity REFUSES the file;
* the historical identity is found by walking the PROP-052 law backwards
  (`historical_candidates`) and proving each shape at the verdict revision;
* a file is SEALABLE only when every judged anchor was found exactly once on both
  sides and compares equal. Missing or duplicated on either side refuses the
  file, so `--sealable` can never name a document nobody compared.
"""

import collections
import pathlib

from .mapping import DOCUMENT_KEYS, historical_candidates
from .mirror import Mirror, blob_exists, git, materialise, mirror_tree
from .refusal import Refusal


class FileVerdict:
    """One judged document's comparison outcome."""

    def __init__(self, path, judged):
        self.path = path
        self.judged = judged          # verdict count on the record
        self.compared = 0             # anchors actually compared
        self.moved = []               # anchors whose content hash moved
        self.respelled = 0            # anchors whose MARKUP moved and content did not
        self.refusals = []            # why this file cannot be trusted
        self.rev = None
        self.historical = None

    @property
    def sealable(self):
        return not self.refusals and not self.moved and self.compared > 0


def verdict_revision(root, stamp, memo):
    """The tree as of a verdict's `verified_at` — one `rev-list` per stamp.

    The memo is passed in rather than kept in a default argument: two runs in one
    process may address different roots, and a cache keyed only by the stamp
    would hand the second one the first one's history.
    """
    key = (str(root), stamp)
    if key not in memo:
        rev = git(root, "rev-list", "-1", f"--before={stamp}", "HEAD").strip()
        memo[key] = rev or None
    return memo[key]


def historical_identity(root, live, rev):
    """The shape `live` had at `rev`, proven to be a blob there — or `None`.

    Candidates come from the PROP-052 law walked backwards and are tried newest
    shape first (`historical_candidates`); each one is PROVEN with
    `git cat-file -e` before it is accepted. `None` means the document did not
    exist at that revision under any shape this law can name — which is a
    refusal, never an empty document.
    """
    for candidate in historical_candidates(live):
        if blob_exists(root, rev, candidate):
            return candidate
    return None


def plan(root, cache, only=None):
    """Group every judged record by the revision its verdicts were formed at.

    Returns `(groups, outcomes)`: `groups[rev] = {live path: historical path}`,
    and one `FileVerdict` per judged record — already carrying its refusal when
    the record has no usable stamp or no identity at that revision.
    """
    groups, outcomes, memo = collections.defaultdict(dict), {}, {}
    for path, record in sorted(cache["files"].items()):
        campaign = record.get("campaign") or {}
        verdicts = campaign.get("verdicts") or {}
        if not verdicts or (only and path not in only):
            continue
        outcome = FileVerdict(path, len(verdicts))
        outcomes[path] = outcome
        stamp = campaign.get("verified_at")
        if not stamp:
            outcome.refusals.append("the record carries no `verified_at`, so there is no "
                                    "revision to compare against")
            continue
        rev = verdict_revision(root, stamp, memo)
        if not rev:
            outcome.refusals.append(f"no commit at or before {stamp}")
            continue
        outcome.rev = rev
        historical = historical_identity(root, path, rev)
        if historical is None:
            shapes = ", ".join(f"`{c}`" for c in historical_candidates(path))
            outcome.refusals.append(
                f"no identity at {rev[:12]} ({stamp}): none of {shapes} is a blob there — the "
                "verdict was not formed against this file, so nothing can be compared")
            continue
        outcome.historical = historical
        groups[rev][path] = historical
    return groups, outcomes


def compare_group(vibe, root, rev, members, live_mirror, cache, outcomes, scratch,
                  reuse=False):
    """Mirror one revision's historical blobs and compare them with the live mirror."""
    tree = scratch / f"rev-{rev[:12]}"
    zone = "campaigns/historical"
    historical_paths = sorted(set(members.values()))
    mirror_dir = tree / zone / "run" / "mirror"
    if reuse and mirror_dir.is_dir():
        mirror = Mirror(mirror_dir)
    else:
        materialise(root, rev, historical_paths, tree)
        mirror = mirror_tree(vibe, root, tree, zone)
    for live, historical in sorted(members.items()):
        outcome = outcomes[live]
        if not mirror.has(historical):
            outcome.refusals.append(
                f"`{historical}` is absent from the historical mirror of {rev[:12]} — the "
                "engine's own scope did not observe it")
            continue
        was_hashes = mirror.marked_facts(historical)
        was_bodies = mirror.fact_bodies(historical)
        now_hashes = live_mirror.marked_facts(live)
        now_bodies = live_mirror.fact_bodies(live)
        verdicts = cache["files"][live]["campaign"]["verdicts"]
        for anchor in sorted(verdicts):
            if anchor in DOCUMENT_KEYS:
                continue
            before, after = was_hashes.get(anchor, []), now_hashes.get(anchor, [])
            for side, hits in (("historical", before), ("live", after)):
                if not hits:
                    outcome.refusals.append(f"anchor `{anchor}` is absent from the {side} parse")
                elif len(hits) > 1:
                    outcome.refusals.append(
                        f"anchor `{anchor}` is minted {len(hits)} times in the {side} parse")
            if len(before) != 1 or len(after) != 1:
                continue
            outcome.compared += 1
            # THE DECISION IS THE ENGINE'S HASH, and only that. The body is kept
            # for the report, never for the verdict — it carries the raw markup
            # SPELLING, and the engine's hash canonicalises that on purpose
            # (`progress_core::parse::canonical_markup`): the same claim written
            # `##ID … @impl/done` and `@fact:ID … @status:impl/done` is the same
            # claim. Letting the body decide re-runs the 2026-08-06 accident in
            # which a corpus-wide respelling read as every fact having moved —
            # measured here as 8 275 facts on this corpus against 981 real ones.
            if before[0] != after[0]:
                outcome.moved.append(anchor)
            elif was_bodies[anchor][0] != now_bodies[anchor][0]:
                outcome.respelled += 1


def require_current_mirror(cache, live_mirror, only=None):
    """The live side must be the parse of the corpus the cache describes.

    Both come out of one `vibe progress mirror` run, so a disagreement means the
    two artifacts were written by different runs and one of them is behind. That
    is decidable here — the numbers are both the engine's — where the old
    "is the cache behind the tree?" was not, because answering it required
    re-implementing the engine's hash.
    """
    behind = []
    for path, record in cache["files"].items():
        campaign = record.get("campaign") or {}
        if not (campaign.get("verdicts") or {}) or (only and path not in only):
            continue
        if not live_mirror.has(path):
            behind.append(f"{path} — no mirror file")
        elif live_mirror.content_hash(path) != record.get("content_hash"):
            behind.append(f"{path} — mirror and cache disagree on the content hash")
    if behind:
        raise Refusal(
            f"the live mirror and the campaign cache describe different corpora "
            f"({len(behind)} judged file(s)); run "
            f"`vibe progress mirror --campaign <zone> --path .` and re-run",
            behind[:20])


def verify_live_mirror(vibe, root, live_mirror, scratch, cache):
    """Prove the live mirror is current with the TREE, not merely with the cache.

    Re-parses the working tree into scratch through the shipped CLI and compares
    the two mirrors' content hashes. Opt-in because it costs a full parse pass;
    without it this program's contract is the one its predecessor had — run after
    `vibe progress mirror` — only now the contract is checkable
    ([`require_current_mirror`]) instead of assumed.
    """
    fresh = mirror_tree(vibe, root, root, str(scratch / "verify-live"))
    drifted = [p for p in cache["files"]
               if fresh.has(p) and live_mirror.has(p)
               and fresh.content_hash(p) != live_mirror.content_hash(p)]
    missing = [p for p in cache["files"] if not fresh.has(p)]
    if drifted or missing:
        raise Refusal(
            f"the tree has moved under the mirror: {len(drifted)} file(s) parse differently and "
            f"{len(missing)} are no longer observed", (drifted + missing)[:20])
    return len(cache["files"])


def analyse(vibe, root, zone, cache, live_mirror, only=None, reuse=False, verify_live=False):
    """The whole comparison. Returns `(outcomes, stats)`."""
    scratch = pathlib.Path(zone) / "run" / "mirror" / ".stability-scratch"
    scratch.mkdir(parents=True, exist_ok=True)
    require_current_mirror(cache, live_mirror, only)
    verified = verify_live_mirror(vibe, root, live_mirror, scratch, cache) if verify_live else 0
    groups, outcomes = plan(root, cache, only)
    for rev, members in sorted(groups.items()):
        compare_group(vibe, root, rev, members, live_mirror, cache, outcomes, scratch, reuse)
    stats = {"revisions": len(groups), "scratch": scratch, "verified_live": verified}
    return outcomes, stats


# --------------------------------------------------------------------------
# Reporting
# --------------------------------------------------------------------------


def buckets(outcomes):
    """The four buckets, and they PARTITION the judged records.

    Disjoint by construction: `refused` is every outcome carrying a refusal, and
    the other three are all `not o.refusals`. Nothing may be counted twice, and
    `tally` below asserts the partition rather than trusting this comment.
    """
    sealable = [o for o in outcomes.values() if o.sealable]
    rejudge = [o for o in outcomes.values() if not o.refusals and o.moved]
    refused = [o for o in outcomes.values() if o.refusals]
    nothing = [o for o in outcomes.values()
               if not o.refusals and not o.moved and o.compared == 0]
    return sealable, rejudge, refused, nothing


def tally(outcomes):
    """Every number the report prints, each labelled with the domain it is over.

    B-107 repair 3 exists because one number was not: `moved` was summed over ALL
    outcomes and then printed as the RE-JUDGE bucket's fact count. On the live
    corpus that headline read "191 files, 981 facts" over 191 rows enumerating
    859 anchors — the other 122 sitting inside REFUSED files, whose verdicts the
    same report had already counted in a bucket labelled "cannot be compared".
    A fact belonged to two mutually exclusive buckets at once.

    So every total here is per bucket, the corpus-wide sums are named
    `*_corpus`, and the two are never interchanged. The assertions are the point:
    they fail loudly the moment a bucket stops being disjoint.
    """
    sealable, rejudge, refused, nothing = buckets(outcomes)
    t = {
        "files": len(outcomes),
        "sealable_files": len(sealable), "rejudge_files": len(rejudge),
        "refused_files": len(refused), "nothing_files": len(nothing),
        "sealable_verdicts": sum(o.judged for o in sealable),
        "rejudge_verdicts": sum(o.judged for o in rejudge),
        "refused_verdicts": sum(o.judged for o in refused),
        "nothing_verdicts": sum(o.judged for o in nothing),
        # The three domains a moved fact can live in. `sealable` has none by
        # definition and `nothing` compared none, so the corpus-wide sum splits
        # in two — and it is SPLIT, never reused for either half.
        "moved_rejudge": sum(len(o.moved) for o in rejudge),
        "moved_refused": sum(len(o.moved) for o in refused),
        "moved_corpus": sum(len(o.moved) for o in outcomes.values()),
        "refused_with_moved": len([o for o in refused if o.moved]),
        "compared_rejudge": sum(o.compared for o in rejudge),
        "compared_sealable": sum(o.compared for o in sealable),
        "compared_refused": sum(o.compared for o in refused),
        "compared_corpus": sum(o.compared for o in outcomes.values()),
        "respelled_rejudge": sum(o.respelled for o in rejudge),
        "respelled_sealable": sum(o.respelled for o in sealable),
        "respelled_refused": sum(o.respelled for o in refused),
        "respelled_corpus": sum(o.respelled for o in outcomes.values()),
    }
    assert t["sealable_files"] + t["rejudge_files"] + t["refused_files"] \
        + t["nothing_files"] == t["files"], "the buckets must partition the judged records"
    assert t["moved_rejudge"] + t["moved_refused"] == t["moved_corpus"], \
        "a moved fact lives in exactly one bucket"
    assert t["compared_sealable"] + t["compared_rejudge"] + t["compared_refused"] \
        == t["compared_corpus"], "a compared anchor lives in exactly one bucket"
    return t


def render(outcomes, stats, max_report, out):
    def say(line=""):
        print(line, file=out)

    sealable, rejudge, refused, nothing = buckets(outcomes)
    t = tally(outcomes)
    say("text stability — every judged fact, compared through the shipped parser\n")
    say(f"  judged files                       {t['files']:>6}")
    say(f"  historical revisions mirrored      {stats['revisions']:>6}")
    say(f"  judged facts compared, CORPUS-WIDE {t['compared_corpus']:>6}   "
        f"({t['compared_sealable']} sealable + {t['compared_rejudge']} re-judge + "
        f"{t['compared_refused']} inside refused files)")
    say(f"  of those, respelled but unmoved    {t['respelled_corpus']:>6}   "
        f"({t['respelled_sealable']} + {t['respelled_rejudge']} + {t['respelled_refused']}; "
        "markup rewritten, the engine's hash canonicalises it, so it is not a move)")
    if stats.get("verified_live"):
        say(f"  live mirror re-verified against the tree ({stats['verified_live']} files)")
    say()
    say(f"  SEALABLE  every judged fact equal to the text it was judged against  "
        f"{t['sealable_files']:>4} file(s), {t['sealable_verdicts']} verdict(s)")
    say(f"  RE-JUDGE  the fact's own text moved                                  "
        f"{t['rejudge_files']:>4} file(s), {t['moved_rejudge']} fact(s)")
    say(f"  REFUSED   cannot be trusted as a whole — see below                   "
        f"{t['refused_files']:>4} file(s), {t['refused_verdicts']} verdict(s)")
    if t["moved_refused"]:
        say(f"            of these, {t['moved_refused']} fact(s) in {t['refused_with_moved']} "
            "file(s) were compared and DID move — counted")
        say("            here and NOT in RE-JUDGE, because the file around them is not "
            "trustworthy")
    if nothing:
        say(f"  NO TEXT   only document-level verdicts, nothing addressable to compare "
            f"{t['nothing_files']:>4} file(s), {t['nothing_verdicts']} verdict(s)")
    say(f"\n  Buckets partition the {t['files']} judged files "
        f"({t['sealable_files']}+{t['rejudge_files']}+{t['refused_files']}+"
        f"{t['nothing_files']}) and their {t['sealable_verdicts'] + t['rejudge_verdicts'] + t['refused_verdicts'] + t['nothing_verdicts']} verdicts.")
    say("  A REFUSED file is never sealable. `--sealable` prints the first bucket only.")

    if rejudge:
        say(f"\n=== RE-JUDGE — the fact's own text moved ({t['rejudge_files']} files, "
            f"{t['moved_rejudge']} facts) ===")
        rows = sorted(rejudge, key=lambda o: -len(o.moved))
        for outcome in rows[:max_report]:
            say(f"  {outcome.judged:>4} judged, {len(outcome.moved):>3} moved  {outcome.path}")
            for anchor in outcome.moved:
                say(f"           - {anchor}")
        if len(rows) > max_report:
            say(f"  … {len(rows) - max_report} more file(s) — raise --max-report")
    if t["moved_refused"]:
        say(f"\n=== MOVED INSIDE A REFUSED FILE ({t['refused_with_moved']} files, "
            f"{t['moved_refused']} facts) — not part of the RE-JUDGE total above ===")
        rows = sorted((o for o in refused if o.moved), key=lambda o: -len(o.moved))
        for outcome in rows[:max_report]:
            say(f"  {outcome.judged:>4} judged, {len(outcome.moved):>3} moved  {outcome.path}")
            say(f"           refused: {outcome.refusals[0]}")
        if len(rows) > max_report:
            say(f"  … {len(rows) - max_report} more file(s) — raise --max-report")
    if refused:
        say(f"\n=== REFUSED — not trusted, never sealable ({t['refused_files']} files) ===")
        why = collections.Counter(o.refusals[0].split(" — ")[-1].split(":")[0]
                                  for o in refused)
        for reason, n in why.most_common():
            say(f"  {n:>4}  {reason}")
        for outcome in sorted(refused, key=lambda o: -o.judged)[:max_report]:
            say(f"  {outcome.judged:>4} verdicts  {outcome.path}")
            say(f"           {outcome.refusals[0]}")
            if len(outcome.refusals) > 1:
                say(f"           (+{len(outcome.refusals) - 1} more on this file)")
        if t["refused_files"] > max_report:
            say(f"  … {t['refused_files'] - max_report} more file(s) — raise --max-report")
    say(f"\nTOTAL sealable verdicts: {t['sealable_verdicts']}   "
        f"facts owed a re-judgement (RE-JUDGE bucket): {t['moved_rejudge']}   "
        f"refused files: {t['refused_files']}")
    say(f"  + {t['moved_refused']} moved fact(s) inside refused files — real moves, but the "
        "file they sit in\n    could not be compared as a whole, so they are reported apart "
        f"from the {t['moved_rejudge']} above.")
    say(f"scratch: {stats['scratch']}")
