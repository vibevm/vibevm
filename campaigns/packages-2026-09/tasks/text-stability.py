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
working tree, and compare them byte for byte.

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

import json
import pathlib
import re
import subprocess
import sys

ZONE = pathlib.Path(__file__).resolve().parent.parent
ROOT = ZONE.parent.parent
CACHE = ZONE / "run" / "cache.json"

# A fact's own paragraph: the `##<id>` line and every line up to the next
# blank one. Deliberately NOT the anchor set — that is the mirror's job
# (`merge-verdicts.addressable`); this only compares the text of anchors the
# cache already names as judged, so a regex cannot widen the perimeter.
#
# The optional list bullet is load-bearing, not tidiness: a great many facts
# in this corpus are written as `- ##ID …` list items, and a pattern anchored
# on `##` alone silently folds them into the PRECEDING fact's paragraph (or
# drops them when there is no preceding one). The first cut of this program
# had that bug, and its effect is the dangerous direction — a list-item fact
# that moved could compare equal and be sealed as stable. Caught 2026-08-05
# by a seal refusal that named facts this extractor reported as absent.
FACT_RE = re.compile(r"^\s*(?:[-*+]\s+)?##([A-Za-z0-9_-]+)\b")


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


def main():
    only_sealable = "--sealable" in sys.argv
    cache = json.loads(CACHE.read_text(encoding="utf-8"))
    clean, dirty, judged_total, moved_total = [], [], 0, 0

    for path, verdicts, at in stale_files(cache):
        if not at:
            dirty.append((path, len(verdicts), ["<undated: cannot locate the judging revision>"]))
            continue
        base = sh("git", "rev-list", "-1", f"--before={at}", "HEAD").strip()
        old = fact_paragraphs(sh("git", "show", f"{base}:{path}"))
        new = fact_paragraphs((ROOT / path).read_text(encoding="utf-8"))
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
