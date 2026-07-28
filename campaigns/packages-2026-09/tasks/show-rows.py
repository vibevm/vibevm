#!/usr/bin/env python3
"""Print a delegated evidence table's rows joined with the verdict already in cache.

Usage:
    python tasks/show-rows.py <ev-*.json> [--found partial,not-found] [--file <substr>]
                              [--start N] [--count N]

Reviewing a delegated table means holding three things side by side: what the
document claims, what the worker found, and what the cache currently says about
it. Opening `cache.json` by hand for each anchor is the arithmetic that makes a
reviewer skim — and skimming 138 `partial` rows into two filename buckets is
exactly the debt this script was written to pay off.

It prints, per row: the anchor and its marker, the claim, every evidence ref the
worker returned, and the verdict standing in `run/cache.json` right now. It
writes nothing.
"""

import json, sys, pathlib

ZONE = pathlib.Path(__file__).resolve().parent.parent
CACHE = ZONE / "run" / "cache.json"


def arg(name, default=None):
    return sys.argv[sys.argv.index(name) + 1] if name in sys.argv else default


def main():
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    rows = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
    cache = json.loads(CACHE.read_text(encoding="utf-8"))

    want = arg("--found")
    if want:
        keep = set(want.split(","))
        rows = [r for r in rows if r.get("found") in keep]
    sub = arg("--file")
    if sub:
        rows = [r for r in rows if sub in r.get("file", "")]
    start = int(arg("--start", 0))
    count = int(arg("--count", 10**6))
    total = len(rows)
    rows = rows[start:start + count]

    # `--brief` drops the quoted snippet from each evidence ref and keeps its
    # address. The quote has already been resolved by `verify-evidence.py`
    # before a reviewer opens the table at all, so re-reading it is the one
    # part of the row that buys nothing — while the claim, the marker, the
    # source classes and the `searched` field are the whole judgement. This is
    # an economy on machine-verified text, NOT on rows: every row is still
    # printed and still read, because reading fewer rows is what created the
    # 138-row debt.
    brief = "--brief" in sys.argv

    print(f"# {pathlib.Path(sys.argv[1]).name} — rows {start}..{start + len(rows) - 1} of {total}\n")
    for i, r in enumerate(rows, start):
        path = r.get("file", "")
        cur = (cache["files"].get(path, {}).get("campaign", {})
               .get("verdicts", {}).get(r["anchor"]))
        print(f"--- [{i}] {r['anchor']}  ({r.get('found')})")
        print(f"    file:    {path}")
        print(f"    marker:  {r.get('marker')}")
        print(f"    claim:   {r.get('claim')}")
        if r.get("src"):
            print(f"    src:     {r['src']}")
        for e in r.get("evidence", []) or []:
            print(f"    ev:      {e.split('  ', 1)[0] if brief else e}")
        if r.get("searched"):
            print(f"    searched: {r['searched']}")
        if cur:
            print(f"    CACHE:   {cur.get('v')}  {cur.get('ev')}")
        else:
            print("    CACHE:   (none)")
        print()
    return 0


if __name__ == "__main__":
    sys.exit(main())
