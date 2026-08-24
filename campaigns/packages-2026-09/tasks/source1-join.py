#!/usr/bin/env python3
"""§3.1 source 1, the mechanical half: does every cited document exist, and does it
carry the anchor it is cited by?

Usage:
    python tasks/source1-join.py [<path-prefix> …]        # default: vibevm/vibepacks/org.vibevm.world

§3.1's first source is «the package's own shipped artifacts» — «a protocol document
a snippet cites must exist and say what the snippet says it says». The second half
is judgement and stays the boss's. The first half is not: a link either resolves or
it does not, and deciding that by reading 121 files is how a reader misses one.

What it resolves. Relative Markdown links to `.md` files, with or without a
`#fragment`, resolved against the linking file's own directory. A fragment matches
if the target carries it either as a heading anchor `{#name}` or as a fact anchor
`##name` — the two forms this corpus addresses by (addressable-specs §placement).
Fragment matching is case-sensitive, because the anchors are.

What it deliberately does NOT resolve. `spec://` URIs: measured over
`vibevm/vibepacks/org.vibevm.world/**`, 55 occurrences of which all but two are
ILLUSTRATIVE — `spec://com.example.shop/PROP-001#…`, `spec://oproto/PROP-002#…`,
a bare `spec://…` — teaching the grammar rather than citing a document. A resolver
over those would report a wall of failures about examples that are correct, and a
check whose output is mostly noise is a check nobody reads.

Exit code is 0 when nothing is broken, 1 otherwise — so it can be a gate later.
"""

import re, sys, pathlib, collections

ROOT = pathlib.Path(__file__).resolve().parents[3]
LINK = re.compile(r"\[[^\]]*\]\(([^)\s]+?)(?:#([^)\s]+))?\)")
HEAD_ANCHOR = re.compile(r"\{#([A-Za-z0-9_.:-]+)\}")
FACT_ANCHOR = re.compile(r"##([A-Za-z0-9][A-Za-z0-9_.:-]*)")


def anchors_of(path):
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return set()
    return set(HEAD_ANCHOR.findall(text)) | set(FACT_ANCHOR.findall(text))


def observed():
    """The files the campaign actually observes — the cache is the authority on the
    `exclude` globs, which this script does not itself read."""
    import json
    cache = ROOT / "campaigns" / "packages-2026-09" / "run" / "cache.json"
    return {(ROOT / p).resolve() for p in json.loads(cache.read_text(encoding="utf-8"))["files"]}


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    corpus_only = "--corpus" in sys.argv
    prefixes = args or ["vibevm/vibepacks/org.vibevm.world"]
    files = sorted(f for p in prefixes for f in (ROOT / p).rglob("*.md"))
    if corpus_only:
        keep = observed()
        files = [f for f in files if f.resolve() in keep]
    # A run over nothing reports «0 broken», which reads exactly like a clean
    # tree. The first run of this script did precisely that, on a path bug.
    if not files:
        print(f"REFUSED: no .md files under {', '.join(prefixes)} (root {ROOT}) — "
              f"an empty sweep is not a clean one")
        return 1
    anchor_cache, rows = {}, []
    for f in files:
        text = f.read_text(encoding="utf-8")
        # Links inside fenced code are examples, not citations.
        text = re.sub(r"(?ms)^([`~]{3,}).*?^\1\s*$", "", text)
        for target, frag in LINK.findall(text):
            if not target.endswith(".md") or "://" in target:
                continue
            dest = (f.parent / target).resolve()
            rel_src = f.relative_to(ROOT).as_posix()
            rel_dst = target
            if not dest.is_file():
                rows.append((rel_src, rel_dst, frag or "", "MISSING FILE"))
                continue
            if frag:
                if dest not in anchor_cache:
                    anchor_cache[dest] = anchors_of(dest)
                if frag not in anchor_cache[dest]:
                    rows.append((rel_src, rel_dst, frag, "MISSING ANCHOR"))

    total = sum(
        len([1 for t, _ in LINK.findall(re.sub(r"(?ms)^([`~]{3,}).*?^\1\s*$", "", f.read_text(encoding="utf-8")))
             if t.endswith(".md") and "://" not in t])
        for f in files
    )
    print(f"source-1 join over {len(files)} file(s) under {', '.join(prefixes)}")
    print(f"  relative .md citations resolved: {total}")
    print(f"  broken: {len(rows)}")
    if rows:
        print()
        by_kind = collections.Counter(r[3] for r in rows)
        for k, n in by_kind.most_common():
            print(f"  {k}: {n}")
        print()
        for src, dst, frag, kind in rows:
            print(f"  {kind:15} {src}")
            print(f"  {'':15}   -> {dst}{('#' + frag) if frag else ''}")
    return 1 if rows else 0


if __name__ == "__main__":
    sys.exit(main())
