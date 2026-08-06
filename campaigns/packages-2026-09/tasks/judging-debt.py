#!/usr/bin/env python3
"""How much judging debt the corpus carries — the three kinds, with their files.

Usage:
    python tasks/judging-debt.py            # summary + the files behind it
    python tasks/judging-debt.py --files    # every file, not just the worst 15

Why this exists as a program rather than as care. Editing the corpus while a
campaign judges it is the normal case, and three different things can happen to
a fact — but only ONE of them announces itself (PROP-043 §10.1):

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

The debt is a LIST, not a ratio (PROP-043 `##DEBT-IS-A-LIST-NOT-A-RATIO`): it is
paid one file at a time, because sealing is a whole-file assertion, and the
cheapest file to clear is the one you were going to open anyway.

**This script is a stopgap.** The durable home is `vibe progress` itself — see
PROP-043 `##DEBT-MUST-BE-ASKABLE`.
"""

import json
import pathlib
import sys

ZONE = pathlib.Path(__file__).resolve().parent.parent
CACHE = ZONE / "run" / "cache.json"
MIRROR = ZONE / "run" / "mirror"

# Verdict keys that are document-level bundles rather than facts; they have no
# addressable anchor by construction and are not orphans.
DOCUMENT_KEYS = {"_elements"}


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

    missing_mirror = []
    facts = verdicts = 0
    unjudged_rows = []   # (path, marked, judged, unjudged, [anchors])
    orphan_rows = []     # (path, [anchors])
    stale = []

    for path, entry in cache["files"].items():
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
