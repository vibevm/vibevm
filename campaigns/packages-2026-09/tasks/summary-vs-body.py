#!/usr/bin/env python3
"""Find summary anchors that drift while the body they summarise is confirmed.

Wave 7 found a failure mode §3.7 does not cover and that costs one cache read to
detect: **the same fact, measured once, confirming a body row and drifting its
own summary.** One batch decided ten of its thirty anchors that way, seven of
them on this exact shape. The disproof was already inside `run/cache.json`.

The convention this leans on is the corpus's own: a summary anchor is named
`##SUM-…` and restates rules stated as body rows in the same document.  So a
`##SUM-…` reading `drift` in a file whose ordinary anchors are entirely (or
overwhelmingly) `confirmed` is **internally inconsistent** — either the body
rows are wrong, or the summary is, and one of the two verdicts is free to
correct.

This is a **candidate finder, not a judge.** It cannot know whether a summary
restates the rows it sits near, and W1's summary-restatement precedent says a
summary sometimes carries a clause its body does not. Every hit is read by hand.
That is the same contract `make-slice.py` and `merge-verdicts.py` hold: refuse
and report, never decide.

Usage:

    python campaigns/packages-2026-09/tasks/summary-vs-body.py
    python campaigns/packages-2026-09/tasks/summary-vs-body.py --all   # incl. clean files
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

ZONE = Path(__file__).resolve().parent.parent
CACHE = ZONE / "run" / "cache.json"
ROUTING = ZONE / "run" / "state" / "routing.json"


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--all", action="store_true", help="show files with no suspect summary too")
    args = ap.parse_args()

    cache = json.loads(CACHE.read_text(encoding="utf-8"))
    routed = {
        e["anchor"] for e in json.loads(ROUTING.read_text(encoding="utf-8"))["entries"]
    }

    suspects: list[tuple[str, str, int, int]] = []
    checked = 0

    for path, entry in cache["files"].items():
        verdicts = entry.get("campaign", {}).get("verdicts", {})
        if not verdicts:
            continue
        checked += 1

        body = {a: v for a, v in verdicts.items() if not a.upper().startswith("SUM-")}
        sums = {a: v for a, v in verdicts.items() if a.upper().startswith("SUM-")}
        if not sums or not body:
            continue

        body_conf = sum(1 for v in body.values() if v["v"] == "confirmed")
        body_drift = sum(1 for v in body.values() if v["v"] == "drift")

        for anchor, v in sums.items():
            if v["v"] != "drift":
                continue
            if f"{path}#{anchor}" in routed:
                continue
            suspects.append((path, anchor, body_conf, body_drift))

    # Strongest signal first: the more of the body is confirmed, the harder it is
    # for its own summary to be drifting on the same measurement.
    suspects.sort(key=lambda s: (s[3] / max(s[2] + s[3], 1), -s[2]))

    strong = [s for s in suspects if s[3] == 0]
    print(f"files with verdicts examined                   : {checked}")
    print(f"summary anchors still reading drift, not routed : {len(suspects)}")
    print(f"  of which over a body with ZERO drift         : {len(strong)}")
    print()
    print("Each is a CANDIDATE, not a finding. Read the summary against the rows it")
    print("restates: either those body verdicts are wrong, or this one is. W1's")
    print("summary-restatement precedent allows a summary to carry a clause its body")
    print("does not — so a hit is a question, never an answer.")
    print()
    for path, anchor, conf, drift in suspects:
        ratio = drift / max(conf + drift, 1)
        mark = "!!" if drift == 0 else ("! " if ratio < 0.25 else "  ")
        print(f"{mark} ##{anchor}")
        print(f"      {path}")
        print(f"      body in this file: {conf} confirmed, {drift} drift")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
