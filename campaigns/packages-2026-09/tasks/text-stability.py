#!/usr/bin/env python3
"""Which judged facts actually moved since their verdicts were formed.

Usage:
    python tasks/text-stability.py            # report every stale file
    python tasks/text-stability.py --sealable # print only the sealable paths

A file is STALE when the campaign cache's `processed_hash` disagrees with the
document's current `content_hash` — the projection reports those, because a
close-out that ships them carries units forward on a judgment made about
different text (`progress_core::baseline::project`, field `stale`).

Staleness is a DOCUMENT-level signal, and it over-reports badly: a file goes
stale when any byte moves, including a `<status>` comment, a line inside a
fenced code block, or prose no marked fact owns. Measured 2026-08-05 over the
20 stale files of `packages-2026-09`: **1214 judged verdicts, of which 13
facts had actually changed.** Re-reading 1214 to find 13 is not diligence, it
is theatre — this program finds the 13.

What it does, per stale file: take the commit that was HEAD at `verified_at`,
extract every judged fact's own paragraph from that revision and from the
working tree, and compare them byte for byte — in CANONICAL markup form, which
is the form the engine hashes, so a fact whose spelling was rewritten and whose
content was not compares equal (`canonical_markup`).

**What a clean result licenses, and what it does not.** Byte-identical text
means each verdict was formed against exactly the text on disk today, so the
re-derivation `vibe progress seal` asks for has been performed — mechanically
and exhaustively, over every judged fact, which is stronger than a re-read.
It does NOT re-assert the verdict against the TREE: a claim whose wording
never moved can still have gone stale because the code under it moved, and
that is `drift`, caught by the campaign's ongoing judging and never by a
seal. Seal what this reports clean; do not read it as freshness.

Exit code is 0 always — this is a measuring instrument, not a gate.
"""

import hashlib
import json
import pathlib
import re
import subprocess
import sys

ZONE = pathlib.Path(__file__).resolve().parent.parent
ROOT = ZONE.parent.parent
CACHE = ZONE / "run" / "cache.json"

def canonical_markup(s):
    """Mirror of `progress_core::parse::canonical_markup` — reduce both marker
    spellings to the legacy one.

    The engine hashes and compares text in this form, not in its raw one, so a
    document that changed only its markup SPELLING is the same content. This
    program must ask the same question the engine answers; a comparison of raw
    spellings would report the 2026-08-06 migration as though every fact in the
    corpus had been rewritten. It did report exactly that, over 274 files and
    11 870 verdicts, until this function existed.

    It is a second implementation of one law and that is a cost, not a design:
    `#recipe-drift` below is what makes the divergence loud instead of silent.
    Only `@fact:` and `@status:` are folded — `@fact/<type>:` is a form with no
    legacy equivalent and hashes as itself, which is the engine's behaviour too.
    """
    return s.replace("@fact:", "##").replace("@status:", "@")


# A fact's own paragraph: the `##<id>` line and every line up to the next
# blank one. Deliberately NOT the anchor set — that is the mirror's job
# (`merge-verdicts.addressable`); this only compares the text of anchors the
# cache already names as judged, so a regex cannot widen the perimeter.
#
# The pattern reads the LEGACY spelling only, and correctly so: every blob it
# is handed has been through `canonical_markup` first. Feeding it raw text
# matches nothing at all on a migrated corpus — measured 2026-08-06 over three
# spec documents carrying 377 facts between them: 0 matches.
#
# The optional list marker is load-bearing, not tidiness: a great many facts
# in this corpus are written as `- ##ID …` bullets or `5. ##ID …` numbered
# steps, and a pattern anchored on `##` alone silently folds them into the
# PRECEDING fact's paragraph (or drops them when there is no preceding one).
# The first cut of this program matched bare `##` and the second only bullets;
# both had the same failure in the dangerous direction — a list fact that
# moved compares equal and gets sealed as stable. Both gaps were caught the
# same day: the bullet one by a seal refusal naming facts this extractor
# reported absent, the numbered one by `PROP-035 ##PIPE-QUALIFY`, a `5.` step.
# Widen this pattern before trusting a clean run over a new corpus.
FACT_RE = re.compile(r"^\s*(?:[-*+]\s+|\d+\.\s+)?##([A-Za-z0-9_-]+)\b")


def sh(*args):
    return subprocess.run(
        args, cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace"
    ).stdout


def fact_paragraphs(blob):
    out, cur, buf = {}, None, []
    for line in blob.splitlines():
        m = FACT_RE.match(line)
        if m:
            if cur:
                out[cur] = "\n".join(buf)
            cur, buf = m.group(1), [line]
        elif cur is not None:
            if line.strip() == "":
                out[cur] = "\n".join(buf)
                cur, buf = None, []
            else:
                buf.append(line)
    if cur:
        out[cur] = "\n".join(buf)
    return out


def stale_files(cache):
    for path, rec in cache["files"].items():
        camp = rec.get("campaign") or {}
        verdicts = camp.get("verdicts") or {}
        processed = camp.get("processed_hash")
        if verdicts and processed and processed != rec.get("content_hash"):
            yield path, verdicts, camp.get("verified_at")


def cache_behind_the_tree(cache):
    """Judged files whose cached `content_hash` no longer matches their bytes.

    Both fields `stale_files` compares live INSIDE the cache, so a document
    edited since the last `vibe progress scan` is invisible to this program: the
    cache still holds the pre-edit digest in both, they agree, and the file is
    never yielded. Measured 2026-08-06 in exactly that state — one spec document
    carrying **92 verdicts** was edited, and the report read «0 stale, 0 facts
    needing re-judgement» with no sign anything had been missed.

    That is the silence this exists to cure, not a defect in the comparison:
    the program's contract is «run after a scan», and until now nothing said so
    at the moment it mattered. A clean zero over a cache that is behind the tree
    licenses nothing.

    The digest is taken over `canonical_markup`, because that is what the engine
    hashes. Taking it over raw bytes accused a freshly-scanned cache of being
    stale for every migrated file — the accusation `#recipe-drift` now catches.
    """
    behind, judged_files = [], 0
    for path, rec in cache["files"].items():
        camp = rec.get("campaign") or {}
        verdicts = camp.get("verdicts") or {}
        cached = rec.get("content_hash")
        if not verdicts or not cached:
            continue
        judged_files += 1
        target = ROOT / path
        if not target.is_file():
            behind.append((path, len(verdicts), "absent from the tree"))
            continue
        text = canonical_markup(target.read_bytes().decode("utf-8"))
        digest = hashlib.sha256(text.encode("utf-8")).hexdigest()
        if cached not in (digest, "sha256:" + digest):
            behind.append((path, len(verdicts), "edited since the last scan"))
    return behind, judged_files


def recipe_has_drifted(behind, judged_files):
    """Is «every judged file is stale» a claim about the tree, or about us?

    A corpus where nearly every judged file was edited since the last scan is
    possible; a corpus where nearly every judged file was edited and none was
    not is a hash recipe that no longer matches the engine's. The second is
    what happened on 2026-08-06, and the program announced the first — loudly,
    with a file list, and wrongly. Distinguishing them costs one ratio, and
    guessing wrong costs a session's worth of re-judgement that is not owed.
    """
    return judged_files > 10 and len(behind) >= judged_files * 9 // 10


def main():
    only_sealable = "--sealable" in sys.argv
    cache = json.loads(CACHE.read_text(encoding="utf-8"))

    behind, judged_files = cache_behind_the_tree(cache)
    if recipe_has_drifted(behind, judged_files) and not only_sealable:
        print(
            f"REFUSING TO REPORT — {len(behind)} of {judged_files} judged files "
            f"disagree with their cached hash. That is not a corpus that was "
            f"edited; that is this program computing the hash differently from "
            f"the engine. Teach `canonical_markup` here whatever "
            f"`progress_core::parse::canonical_markup` now does, then re-run. "
            f"Everything below would be about the recipe, not about the tree."
        )
        return
    if behind and not only_sealable:
        judged = sum(n for _, n, _ in behind)
        print(
            f"WARNING — the cache is behind the tree for {len(behind)} judged "
            f"file(s) carrying {judged} verdict(s). Everything below compares two "
            f"fields INSIDE the cache, so those files cannot appear in it no "
            f"matter what moved in them. Run `vibe progress scan` (or `mirror`) "
            f"first, or read the result as being about the cache rather than "
            f"about the tree:"
        )
        for path, n, why in sorted(behind, key=lambda r: -r[1]):
            print(f"  {n:4d} verdicts  {path}  — {why}")
        print()

    clean, dirty, judged_total, moved_total = [], [], 0, 0

    for path, verdicts, at in stale_files(cache):
        if not at:
            dirty.append((path, len(verdicts), ["<undated: cannot locate the judging revision>"]))
            continue
        base = sh("git", "rev-list", "-1", f"--before={at}", "HEAD").strip()
        old = fact_paragraphs(canonical_markup(sh("git", "show", f"{base}:{path}")))
        new = fact_paragraphs(canonical_markup((ROOT / path).read_text(encoding="utf-8")))
        moved = sorted(a for a in verdicts if old.get(a) != new.get(a))
        judged_total += len(verdicts)
        moved_total += len(moved)
        (dirty if moved else clean).append((path, len(verdicts), moved))

    if only_sealable:
        for path, _, _ in sorted(clean):
            print(path)
        return

    print(f"stale files: {len(clean) + len(dirty)}   judged verdicts: {judged_total}")
    print(f"\nSEALABLE — every judged fact byte-identical to the text it was judged against ({len(clean)} files):")
    for path, n, _ in sorted(clean, key=lambda r: -r[1]):
        print(f"  {n:4d}  {path}")
    print(f"\nRE-JUDGE — the fact's own text moved ({len(dirty)} files, {moved_total} facts):")
    for path, n, moved in sorted(dirty, key=lambda r: -len(r[2])):
        print(f"  {n:4d} judged, {len(moved):3d} moved  {path}")
        for a in moved:
            print(f"           - {a}")
    print(f"\nTOTAL sealable verdicts: {sum(n for _, n, _ in clean)}   facts needing re-judgement: {moved_total}")


if __name__ == "__main__":
    main()
