#!/usr/bin/env python3
"""How much judging debt the corpus carries — the three kinds, with their files.

Usage:
    python tasks/judging-debt.py            # summary + the files behind it
    python tasks/judging-debt.py --files    # every file, not just the worst 15

Why this exists as a program rather than as care. Editing the corpus while a
campaign judges it is the normal case, and three different things can happen to
a fact — but only ONE of them announces itself (PROP-047 §6.2):

  * a JUDGED fact whose text moves comes due for re-judgement, and
    `text-stability.py` names it;
  * a fact ADDED to an already-judged file is unjudged and NOTHING says so — it
    enters no total and fires no gate;
  * a fact REMOVED leaves its verdict behind, still counted, pointing at an
    anchor that no longer exists.

The second and third are invisible to every shipped command, which is why the
same five orphan verdicts were measured and written into a phase batch plan on
2026-07-28 and were still there, untouched, on 2026-08-06 — recorded in a zone
the project's own rules call disposable.

The debt is a LIST, not a ratio (PROP-047 `##DEBT-IS-A-LIST-NOT-A-RATIO`): it is
paid one file at a time, because sealing is a whole-file assertion, and the
cheapest file to clear is the one you were going to open anyway.

Documentation is the exception the debt has to make: a `doc` package is
observed like every authored text here, but its genre is non-normative, so its
facts owe no verdict. `[judging] exempt` in `facts.toml` says exactly that and
nothing more — the files stay scanned, checked and mapped (PROP-057
`##OBS-NOT-JUDGED`).

**This script is a stopgap.** The durable home is `vibe progress` itself — see
PROP-047 `##DEBT-MUST-BE-ASKABLE`. The three anchors above moved there with the
tool half of PROP-043 at the facts/progress split of 2026-08-22; the tombstone
at modules/vibe-progress/PROP-043-progress-markup.xml records the move.
"""

import json
import pathlib
import re
import sys
import tomllib

ZONE = pathlib.Path(__file__).resolve().parent.parent
ROOT = ZONE.parent.parent
CACHE = ZONE / "run" / "cache.json"
MIRROR = ZONE / "run" / "mirror"
FACTS_CONFIG = ROOT / "facts.toml"

# Verdict keys that are document-level bundles rather than facts; they have no
# addressable anchor by construction and are not orphans.
DOCUMENT_KEYS = {"_elements"}


def glob_to_regex(pattern):
    """One `facts.toml` glob as a regex, in the engine's own semantics.

    `**` is a whole component and stands for ZERO or more of them. A plain
    `*` and a `?` cross separators as well — surprising, and not a choice
    made here: `glob::Pattern::matches` is called with the crate's default
    `MatchOptions`, which do not require a literal separator, and the
    `exclude` key of the same file has been read that way since DRIFT-024.
    One configuration file must not hold two readings of `*`.

    The parity matters because the debt moves homes: this script is the
    stopgap and the shipped verb is the durable home (PROP-047
    `##DEBT-MUST-BE-ASKABLE`), and a glob that meant one thing here and
    another there would silently move the debt on the day of the swap.
    `the_exempt_globs_speak_the_dialect_the_exclude_key_speaks`
    (crates/progress-core/src/scope.rs) pins the same cases on the engine.
    """
    segments = pattern.split("/")
    out = []
    for index, segment in enumerate(segments):
        last = index == len(segments) - 1
        if segment == "**":
            out.append(".*" if last else "(?:.+/)*")
            continue
        for ch in segment:
            if ch == "*":
                out.append(".*")
            elif ch == "?":
                out.append(".")
            else:
                out.append(re.escape(ch))
        if not last:
            out.append("/")
    return re.compile("^" + "".join(out) + "$")


def exemptions():
    """The `[judging] exempt` globs of `facts.toml`, compiled.

    «Observed, never judged» (PROP-057 `##OBS-NOT-JUDGED`): a documentation
    package is authored here and its pages belong in the corpus, but its
    genre is non-normative, so its facts owe no verdict and must not be
    counted as debt. This is NOT `exclude` — every exempt file is still
    scanned, still checked and still on the map.
    """
    if not FACTS_CONFIG.exists():
        return []
    config = tomllib.loads(FACTS_CONFIG.read_text(encoding="utf-8"))
    return [glob_to_regex(p) for p in config.get("judging", {}).get("exempt", [])]


def addressable(path):
    """Marked, anchored facts of one file, per the campaign's own mirror."""
    f = MIRROR / (path.replace("/", "__") + ".json")
    if not f.exists():
        return None
    d = json.loads(f.read_text(encoding="utf-8"))
    return {
        fact["id"]
        for b in d["blocks"]
        for fact in b.get("facts", [])
        if fact.get("marked") and fact.get("id")
    }


def main():
    show_all = "--files" in sys.argv
    if not CACHE.exists():
        raise SystemExit(f"no cache at {CACHE} — run `vibe progress scan` first")
    cache = json.loads(CACHE.read_text(encoding="utf-8"))

    exempt_globs = exemptions()
    missing_mirror = []
    exempt = []
    facts = verdicts = 0
    unjudged_rows = []   # (path, marked, judged, unjudged, [anchors])
    orphan_rows = []     # (path, [anchors])
    stale = []

    for path, entry in cache["files"].items():
        # Observed, never judged — before the mirror is even consulted, so
        # an exempt file owes nothing whether or not one was written for it.
        if any(g.match(path) for g in exempt_globs):
            exempt.append(path)
            continue
        camp = entry.get("campaign", {})
        vmap = camp.get("verdicts", {})
        marked = addressable(path)
        if marked is None:
            missing_mirror.append(path)
            continue
        facts += len(marked)
        verdicts += len(vmap)

        unjudged = sorted(a for a in marked if a not in vmap)
        if unjudged:
            unjudged_rows.append((path, len(marked), len(vmap), unjudged))

        orphans = sorted(
            a for a in vmap if a not in marked and a not in DOCUMENT_KEYS
        )
        if orphans:
            orphan_rows.append((path, orphans))

        if vmap and camp.get("processed_hash") != entry.get("content_hash"):
            stale.append(path)

    n_unjudged = sum(len(r[3]) for r in unjudged_rows)
    n_orphans = sum(len(r[1]) for r in orphan_rows)

    print("judging debt — what the corpus owes\n")
    print(f"  addressable marked facts      {facts:>6}")
    print(f"  facts carrying a verdict      {verdicts:>6}")
    print()
    print(f"  UNJUDGED  facts with no verdict at all      {n_unjudged:>5}"
          f"   in {len(unjudged_rows)} file(s)")
    print(f"  ORPHANED  verdicts whose anchor is gone     {n_orphans:>5}"
          f"   in {len(orphan_rows)} file(s)")
    print(f"  STALE     files whose bytes moved since judging  {len(stale):>5}")
    print()
    print("  STALE is not the same question as «a judged fact moved» — a file"
          " goes stale when facts are\n  ADDED too. For the per-fact answer run"
          " `text-stability.py`; it names every fact owed a re-judgement.")
    if exempt:
        print(f"\n  {len(exempt)} observed file(s) are exempt from judgement"
              " by `[judging] exempt` in facts.toml —\n  documentation is"
              " observed, checked and mapped, and owes no verdict"
              " (PROP-057 ##OBS-NOT-JUDGED)")
    if missing_mirror:
        print(f"\n  ! {len(missing_mirror)} file(s) have no mirror —"
              " run `vibe progress mirror` before trusting the numbers above")

    if unjudged_rows:
        print("\n=== files carrying unjudged facts (clear one file at a time) ===")
        rows = sorted(unjudged_rows, key=lambda r: -len(r[3]))
        for path, m, j, u in (rows if show_all else rows[:15]):
            print(f"  {len(u):>4} unjudged of {m:>4} marked ({j} judged)  {path}")
            if len(u) <= 8:
                print(f"        {', '.join(u)}")
        if not show_all and len(rows) > 15:
            print(f"  … {len(rows) - 15} more — pass --files")

    if orphan_rows:
        print("\n=== verdicts pointing at anchors that no longer exist ===")
        for path, anchors in sorted(orphan_rows, key=lambda r: -len(r[1])):
            print(f"  {len(anchors):>4}  {path}")
            print(f"        {', '.join(anchors)}")

    if not n_unjudged and not n_orphans:
        print("\nno debt: every marked fact carries a verdict and every verdict"
              " has its anchor.")


if __name__ == "__main__":
    main()
