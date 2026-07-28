#!/usr/bin/env python3
"""Check that a delegated evidence table points where it says it points.

Usage:
    python tasks/verify-evidence.py <ev-*.json> [<ev-*.json> …]

A worker returns rows of `{anchor, claim, marker, evidence[], found, searched}`,
where each evidence string is `"<path>:<line>  <snippet>"`. Reviewing that by hand
means opening every path and counting lines, which is the cost that makes people
skim instead. This does the counting.

For each evidence ref it checks three things and reports which failed:

  PATH    the file exists
  LINE    the line number is inside the file
  TEXT    the quoted snippet actually occurs at that line — or, failing that,
          anywhere within ±3 lines, which is reported as OFF-BY (a real match
          recorded against a neighbouring line, worth knowing but not a fiction)

What it cannot check is whether the located code *settles* the claim. That is the
review, and the review is not delegated — this only removes the part of it that is
arithmetic, so the judgement gets the attention.

Exit code 0 when every ref resolves (OFF-BY tolerated), 1 otherwise.
"""

import json, re, sys, pathlib, collections

ROOT = pathlib.Path(__file__).resolve().parents[3]
REF = re.compile(r"^\s*([^\s:]+(?::[^\s:]*)??[^\s:]*\.[A-Za-z0-9]+):(\d+)\s*(.*)$")
FUZZ = 3


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
    for d in range(1, FUZZ + 1):
        for cand in (line - 1 - d, line - 1 + d):
            if 0 <= cand < len(lines) and want in norm(lines[cand]):
                return "OFF-BY", f"{path}:{line} — the quote is at line {cand + 1} ({cand + 1 - line:+d})"
    return "TEXT", f"{path}:{line} — the quote is not at that line nor within ±{FUZZ}"


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
                if kind not in ("OK", "OFF-BY"):
                    problems.append((r["anchor"], kind, detail))
                elif kind == "OFF-BY":
                    problems.append((r["anchor"], kind, detail))
        print(f"=== {pathlib.Path(arg).name}: {len(rows)} row(s)")
        print("    found:   " + ", ".join(f"{k}={v}" for k, v in sorted(found.items())))
        print("    refs:    " + ", ".join(f"{k}={v}" for k, v in sorted(kinds.items())))
        hard = sum(v for k, v in kinds.items() if k not in ("OK", "OFF-BY"))
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
