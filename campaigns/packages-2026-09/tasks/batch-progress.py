#!/usr/bin/env python3
"""The phase's progress bar, per batch: owed, judged, remaining.

Usage:
    python tasks/batch-progress.py [--cluster world|ai-native]

`summary.py` answers «what did the verdicts say» — confirmed / drift /
unverifiable per cluster. It cannot answer «how much is left», because it
counts what exists rather than what is owed. This joins the two sides:
`tasks/PHASE-C-BATCHES.json` carries the anchor count each batch owes
(generated from `run/mirror/`), and `run/cache.json` carries the verdicts
written so far. A batch is closed when the second reaches the first.

The join is per FILE, not per batch, because the per-file slice is the unit of
work: a batch three files into five reports 60 %, and the two open files are
named. That is the number a session picks its next slice from.

It writes nothing.
"""

import json, sys, pathlib

ZONE = pathlib.Path(__file__).resolve().parent.parent
BATCHES = ZONE / "tasks" / "PHASE-C-BATCHES.json"
CACHE = ZONE / "run" / "cache.json"


def bar(done, owed, width=24):
    if owed <= 0:
        return "?" * width
    filled = round(width * min(done, owed) / owed)
    return "#" * filled + "." * (width - filled)


def main():
    want = None
    if "--cluster" in sys.argv:
        want = sys.argv[sys.argv.index("--cluster") + 1]

    batches = json.loads(BATCHES.read_text(encoding="utf-8"))["batches"]
    cache = json.loads(CACHE.read_text(encoding="utf-8"))["files"]

    def judged(path):
        entry = cache.get(path, {}).get("campaign", {}).get("verdicts", {})
        # `_elements` is the per-file document/section bundle, not an anchor
        # verdict; the exit gate counts anchors, so it is excluded here too.
        return sum(1 for k in entry if k != "_elements")

    grand_owed = grand_done = 0
    rows = []
    for b in batches:
        if want and b["cluster"] != want:
            continue
        owed = b["anchors"]
        done = sum(judged(f) for f in b["files"])
        open_files = [f for f in b["files"] if judged(f) == 0]
        part_files = [f for f in b["files"] if 0 < judged(f)]
        rows.append((b, owed, done, open_files, part_files))
        grand_owed += owed
        grand_done += done

    print(f"Phase C — progress by batch{'' if not want else ' (' + want + ')'}")
    print()
    for b, owed, done, open_files, part_files in rows:
        left = owed - done
        state = "CLOSED" if left <= 0 else ("open" if done else "NOT STARTED")
        pct = 100.0 * done / owed if owed else 0.0
        print(f"  {b['id']:<4} {bar(done, owed)}  {done:>5}/{owed:<5} {pct:5.1f} %  "
              f"{len(b['files']) - len(open_files)}/{len(b['files'])} files  {state}")
        print(f"       {b['title']}")
        if 0 < len(open_files) < len(b["files"]):
            for f in open_files:
                print(f"         unopened: {f}")
    print()
    left = grand_owed - grand_done
    print(f"  {'ALL':<4} {bar(grand_done, grand_owed)}  {grand_done:>5}/{grand_owed:<5} "
          f"{100.0 * grand_done / grand_owed:5.1f} %  {left} anchors remain")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
