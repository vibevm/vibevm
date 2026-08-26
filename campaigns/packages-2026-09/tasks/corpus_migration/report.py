"""The migration's own printed report."""

import collections
import sys

from .mapping import split_kinds


def head(items, n):
    rows = list(items)
    return rows[:n], max(0, len(rows) - n)


def report(max_report, rev, mapping, cmp_, how, unresolved, out=sys.stdout):
    def say(line=""):
        print(line, file=out)

    kinds = split_kinds(mapping)
    say("corpus migration — old physical identities onto live ones\n")
    say(f"  pre-migration revision            {rev}")
    say(f"  judged records                    {len(mapping):>6}")
    say(f"    kept their serialisation        {kinds['kept']:>6}")
    say(f"    Markdown -> XML                 {kinds['xml']:>6}")
    say()
    say(f"  judged fact anchors asked about    {cmp_.anchors:>6}")
    say(f"  document-level `_elements`         {cmp_.elements:>6}")
    say(f"  anchors absent at the mapped path  {len(cmp_.absent):>6}")
    say(f"  anchors duplicated there           {len(cmp_.duplicated):>6}")
    say(f"  unit structure differing           {len(cmp_.structure):>6}")
    say(f"  documents whose own hash moved     {cmp_.doc_hash_changed:>6}")
    say(f"  anchors compared on BOTH sides     {cmp_.compared:>6}")
    say(f"  JUDGED FACTS WHOSE TEXT MOVED      {len(cmp_.moved):>6}"
        "   <- owed a re-judgement, never re-judged here")
    say(f"  HISTORICAL GAPS                    {len(cmp_.historical_gap):>6}"
        "   <- nothing to compare against; NOT counted as moved")
    say(f"  (compared {cmp_.compared} = asked {cmp_.anchors} - absent {len(cmp_.absent)} "
        f"- duplicated {len(cmp_.duplicated)} - gaps {len(cmp_.historical_gap)})")
    say()
    say("  baseline addresses, by how each was resolved:")
    for kind, n in how.most_common():
        say(f"    {n:>5}  {kind}")
    if unresolved:
        dead = sum(1 for _a, _n, kind in unresolved if "already dead" in kind)
        say(f"\n  {len(unresolved)} baseline address(es) resolve to no live unit — carried with "
            "the path remapped and the fragment untouched")
        say(f"  ({dead} of them resolved to nothing BEFORE the migration too — pre-existing "
            "baseline rot, not damage this move did):")
        shown, rest = head(unresolved, max_report)
        for addr, new_addr, kind in shown:
            say(f"    {addr}\n        -> {new_addr}   ({kind})")
        if rest:
            say(f"    … {rest} more")
    if cmp_.moved:
        by_file = collections.Counter(old for old, _live, _a in cmp_.moved)
        say(f"\n=== facts owed a re-judgement ({len(cmp_.moved)} in {len(by_file)} file(s)) ===")
        shown, rest = head(by_file.most_common(), max_report)
        for path, n in shown:
            say(f"  {n:>4}  {path}")
            if n <= 8:
                say("        " + ", ".join(sorted(a for o, _l, a in cmp_.moved if o == path)))
        if rest:
            say(f"  … {rest} more file(s) — raise --max-report")
    if cmp_.historical_gap:
        say(f"\n=== historical gaps ({len(cmp_.historical_gap)}) — the anchor is at the mapped "
            "path but the pre-migration parse offers nothing to compare it with ===")
        say("    Their verdicts are carried unchanged; whether the text moved is UNKNOWN, so "
            "they are\n    not in the moved count above and must be acknowledged on their own.")
        shown, rest = head(cmp_.historical_gap, max_report)
        for old, live, anchor, why in shown:
            say(f"    {old}#{anchor}\n        -> {live}   ({why})")
        if rest:
            say(f"    … {rest} more")
    for label, rows in (("anchors absent at the mapped path", cmp_.absent),
                        ("anchors duplicated at the mapped path", cmp_.duplicated),
                        ("unit structure differing", cmp_.structure)):
        if rows:
            say(f"\n=== {label} ===")
            shown, rest = head(rows, max_report)
            for row in shown:
                say(f"    {row}")
            if rest:
                say(f"    … {rest} more")
