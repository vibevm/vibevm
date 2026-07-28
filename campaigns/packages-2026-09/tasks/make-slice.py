#!/usr/bin/env python3
"""Build one file's verdict batch from a delegated evidence table plus a rulings map.

Usage:
    python tasks/make-slice.py <ev-*.json> --file <substr> --batch <id> --out <path>
                               [--rulings <rulings.json>] [--cluster world]

The per-file slice is the unit of work, and seventeen of them were built by
seventeen throwaway scripts that each re-implemented the same three steps: take
the worker's rows for one file, apply a hand-written map of the anchors the
reviewer disagreed with, default the rest to `confirmed` on the worker's own
evidence. Re-implementing that is where a transcribed anchor goes wrong, and a
wrong anchor is the one error `merge-verdicts.py` cannot catch — it is a valid
anchor of a real file, just not the one the reason belongs to.

The rulings file is the reviewer's whole contribution:

    {
      "ANCHOR-ID": {"v": "drift", "why": "DRIFT: …the reason…", "src": [1, 2]},
      "OTHER-ID":  {"v": "unverifiable", "why": "UNVERIFIABLE: …"}
    }

`v` and `why` are required per entry; `src` is optional and defaults to the
worker's own. `why` is appended as the LAST evidence ref, which is where every
batch in this campaign has carried the reviewer's reason — a reader who opens
the cache meets the refs first and the judgement last.

**Every row gets a reason, not only the ruled ones.** A row with no ruling
closes with `confirmed: <the worker's own `searched`>`, which is what the
seventeen hand-written slices did and what the campaign's own recipe
prescribes. The rule behind it is a constraint of this campaign: a wrong REASON
is worse than a wrong verdict, because the next reader acts on the reason and a
verdict at least pointed at a real defect. A row that reaches the cache with
refs and no reason gives that reader nothing to act on either way.

## What it refuses, and why each refusal exists

  * **a `--file` matching zero or more than one file in the table** — a slice
    is one file by definition, and a substring that matches two silently merges
    two reviews;
  * **a ruling for an anchor the table does not carry** — a typo'd anchor would
    otherwise be dropped in silence, taking the reason with it;
  * **an anchor the mirror declares addressable that the table does not cover**
    — this is the one that keeps a slice from becoming a debt: it prints the
    missing anchors and exits, so the gap is a decision rather than a discovery
    two batches later;
  * **an empty `why`, a `v` outside the vocabulary, a row with no evidence.**

It writes only the batch file. `merge-verdicts.py` re-validates everything
independently — this is a generator, not a gate, and the gate stays downstream.
"""

import json, sys, pathlib

ZONE = pathlib.Path(__file__).resolve().parent.parent
MIRROR = ZONE / "run" / "mirror"
VOCAB = {"confirmed", "drift", "unverifiable"}


def arg(name, default=None):
    return sys.argv[sys.argv.index(name) + 1] if name in sys.argv else default


def addressable(path):
    """The anchors a verdict map may key for one file — the mirror is the definition."""
    f = MIRROR / (path.replace("/", "__") + ".json")
    if not f.exists():
        raise SystemExit(f"REFUSED: no mirror for {path}")
    d = json.loads(f.read_text(encoding="utf-8"))
    return {fact["id"] for b in d["blocks"] for fact in b.get("facts", [])
            if fact.get("marked") and fact.get("id")}


def main():
    if len(sys.argv) < 2 or "--file" not in sys.argv or "--batch" not in sys.argv \
            or "--out" not in sys.argv:
        raise SystemExit(__doc__)

    rows = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
    sub, batch, out = arg("--file"), arg("--batch"), arg("--out")
    cluster = arg("--cluster", "world")

    paths = sorted({r["file"] for r in rows if sub in r["file"]})
    if len(paths) != 1:
        raise SystemExit(f"REFUSED: --file {sub!r} matches {len(paths)} files, not 1"
                         + ("\n  " + "\n  ".join(paths) if paths else ""))
    path = paths[0]
    mine = [r for r in rows if r["file"] == path]

    rulings = {}
    rf = arg("--rulings")
    if rf:
        rulings = json.loads(pathlib.Path(rf).read_text(encoding="utf-8"))

    by_anchor = {}
    for r in mine:                       # last row wins if a worker repeated an anchor
        by_anchor[r["anchor"]] = r

    stray = [a for a in rulings if a not in by_anchor]
    if stray:
        raise SystemExit("REFUSED: ruling for anchor(s) the table does not carry:\n  "
                         + "\n  ".join(stray))

    addr = addressable(path)
    uncovered = sorted(addr - set(by_anchor))
    if uncovered:
        raise SystemExit(f"REFUSED: {len(uncovered)} addressable anchor(s) of {path}\n"
                         "         are not in the evidence table — a slice that skips\n"
                         "         them lands as a debt. Missing:\n  "
                         + "\n  ".join(uncovered))

    verdicts, counts = {}, {"confirmed": 0, "drift": 0, "unverifiable": 0}
    for anchor, r in by_anchor.items():
        if anchor not in addr:           # the table may carry non-addressable extras
            continue
        ev = [e for e in r.get("evidence", []) if isinstance(e, str) and e.strip()]
        src = r.get("src") or [1]
        rule = rulings.get(anchor)
        v = "confirmed"
        if rule:
            v = rule.get("v")
            why = (rule.get("why") or "").strip()
            if v not in VOCAB:
                raise SystemExit(f"REFUSED: {anchor}: v={v!r} is outside the vocabulary")
            if not why:
                raise SystemExit(f"REFUSED: {anchor}: a ruling without a `why` is a "
                                 "verdict nobody can act on")
            src = rule.get("src", src)
        else:
            searched = (r.get("searched") or "").strip()
            if not searched:
                raise SystemExit(f"REFUSED: {anchor}: the worker left `searched` empty, "
                                 "so an unruled row would reach the cache with no reason "
                                 "— rule it by hand or send the row back")
            why = f"confirmed: {searched}"
        ev = ev + [why]                  # the reason is ALWAYS the last ref
        if not any(len(e.strip()) > 8 for e in ev):
            raise SystemExit(f"REFUSED: {anchor}: no usable evidence ref")
        verdicts[anchor] = {"v": v, "ev": ev, "src": src}
        counts[v] += 1

    doc = {"batch": batch, "cluster": cluster, "files": {path: verdicts}}
    pathlib.Path(out).write_text(json.dumps(doc, ensure_ascii=False, indent=1),
                                 encoding="utf-8")
    total = sum(counts.values())
    pct = 100.0 * counts["confirmed"] / total if total else 0.0
    print(f"{out}\n  {path}\n  {total} anchors  "
          f"confirmed {counts['confirmed']}  drift {counts['drift']}  "
          f"unverifiable {counts['unverifiable']}  {pct:.1f} %")
    self_ref = sum(1 for v in verdicts.values() if v["src"] == [1])
    print(f"  src == [1] (self-referential): {self_ref}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
