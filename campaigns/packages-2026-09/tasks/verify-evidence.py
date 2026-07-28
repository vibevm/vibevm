#!/usr/bin/env python3
"""Check that a delegated evidence table points where it says it points.

Usage:
    python tasks/verify-evidence.py <ev-*.json> [<ev-*.json> …]

A worker returns rows of `{anchor, claim, marker, evidence[], found, searched}`,
where each evidence string is `"<path>:<line>  <snippet>"`. Reviewing that by hand
means opening every path and counting lines, which is the cost that makes people
skim instead. This does the counting.

Every ref lands in exactly one outcome:

  OK      the quote is at the cited line, or spans the few lines from it
  OFF-BY  the quote is real but sits at a neighbouring line — worth knowing,
          not a fiction
  ELIDED  the quote ends in `...` and matches up to the ellipsis — honest
          quoting, named rather than passed silently, because an ellipsis is
          exactly where a fabricated tail would hide
  PATH    the file does not exist
  LINE    the line number is past end-of-file
  TEXT    the quote is nowhere near the cited line

The first three pass; the last three fail. Two of these outcomes exist because
the first version of this checker did not have them and reported three accurate
refs as fictions — a multi-line quote and an elided one. **A checker that cries
wolf is worse than no checker**: it gets skipped, and then it protects nothing.
The rule was approximated instead of read, which is the same mistake this
campaign has now caught in four separate instruments including two of mine.

What it cannot check is whether the located code *settles* the claim. That is the
review, and the review is not delegated — this only removes the part of it that is
arithmetic, so the judgement gets the attention.

Exit code 0 when every ref resolves (OFF-BY and ELIDED tolerated but named), 1 otherwise.
"""

import json, re, sys, pathlib, collections

ROOT = pathlib.Path(__file__).resolve().parents[3]
REF = re.compile(r"^\s*([^\s:]+(?::[^\s:]*)??[^\s:]*\.[A-Za-z0-9]+):(\d+)\s*(.*)$")
FUZZ = 3
SPAN = 4          # how many lines a single quote may legitimately span


def norm(s):
    return re.sub(r"\s+", " ", s).strip().lower()


def check_ref(ref):
    m = REF.match(ref)
    if not m:
        return "UNPARSED", ref
    path, line, snippet = m.group(1), int(m.group(2)), m.group(3)
    f = ROOT / path
    if not f.is_file():
        return "PATH", f"{path} does not exist"
    try:
        lines = f.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError as e:
        return "PATH", f"{path}: {e}"
    if not (1 <= line <= len(lines)):
        return "LINE", f"{path}:{line} — file has {len(lines)} line(s)"
    if not snippet or len(snippet.strip()) < 4:
        return "OK", ""            # a bare path:line with no quote is not a text claim
    want = norm(snippet)
    if want in norm(lines[line - 1]):
        return "OK", ""
    # A quote may legitimately span more than one source line, anchored at the
    # first — `#[derive(…)]  pub struct Edge {` is two lines and one claim. The
    # first version of this check assumed one line per quote and reported two
    # accurate refs as fictions, which is the more expensive failure: a checker
    # that cries wolf gets skipped, and then it protects nothing.
    if want in norm(" ".join(lines[line - 1: line - 1 + SPAN])):
        return "OK", ""
    # A quote may be elided — `description = "The specmap engine (PROP-014 §2.5):
    # markdown unit parser, ..."`. That is honest quoting and must not read as a
    # fiction, but it is also where a fabrication could hide, so it gets its own
    # name rather than passing silently as OK.
    head = re.split(r"\.\.\.|…", snippet, maxsplit=1)[0].strip()
    if head != snippet.strip() and len(head) >= 12:
        window = norm(" ".join(lines[line - 1: line - 1 + SPAN]))
        if norm(head) in window:
            return "ELIDED", f"{path}:{line} — verified to the ellipsis only ({len(head)} chars)"
    for d in range(1, FUZZ + 1):
        for cand in (line - 1 - d, line - 1 + d):
            if 0 <= cand < len(lines) and want in norm(lines[cand]):
                return "OFF-BY", f"{path}:{line} — the quote is at line {cand + 1} ({cand + 1 - line:+d})"
    return "TEXT", f"{path}:{line} — the quote is not at that line, within ±{FUZZ}, nor across the {SPAN} lines from it"


def main():
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    bad = 0
    for arg in sys.argv[1:]:
        rows = json.loads(pathlib.Path(arg).read_text(encoding="utf-8"))
        kinds = collections.Counter()
        found = collections.Counter(r.get("found", "?") for r in rows)
        problems = []
        for r in rows:
            for ref in r.get("evidence", []) or []:
                kind, detail = check_ref(ref)
                kinds[kind] += 1
                if kind not in ("OK", "OFF-BY", "ELIDED"):
                    problems.append((r["anchor"], kind, detail))
                elif kind in ("OFF-BY", "ELIDED"):
                    problems.append((r["anchor"], kind, detail))
        print(f"=== {pathlib.Path(arg).name}: {len(rows)} row(s)")
        print("    found:   " + ", ".join(f"{k}={v}" for k, v in sorted(found.items())))
        print("    refs:    " + ", ".join(f"{k}={v}" for k, v in sorted(kinds.items())))
        hard = sum(v for k, v in kinds.items() if k not in ("OK", "OFF-BY", "ELIDED"))
        if problems:
            print()
            for anchor, kind, detail in problems:
                print(f"    {kind:8} {anchor}: {detail}")
        if hard:
            bad += hard
        print()
    print("unresolvable refs:", bad)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
