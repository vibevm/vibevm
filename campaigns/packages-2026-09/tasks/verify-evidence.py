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
# A path qualifies if it contains a `/` or a `.` — not if it ends in an extension.
# The old pattern required `\.[A-Za-z0-9]+` before the colon and so could not read
# two whole classes of real, checkable ref:
#   * extensionless DOTFILES — `.gitignore:37  /refs/` (W4b, three times, on
#     evidence that the reference sources are deliberately not in the repository);
#   * extensionless FILES — `vibedeps/flow-dev-runtime-docs/0.1.0/LICENSE:3`
#     (W6d, twice). Six of 27 world packages ship `LICENSE` rather than
#     `LICENSE.md`, so this is a recurring class, not a one-off.
# The `/`-or-`.` lookahead keeps bare prose words (`note:12`) from parsing while
# admitting any real repository path. A path that parses and does not exist is
# reported as PATH, which is louder than UNPARSED and therefore the safer failure.
# Regression: the three already-trusted tables re-run to the identical counts they
# returned before this and the previous widening (C45-rust 2, C45-go 10, C6 0).
REF = re.compile(r"^\s*((?=[^\s:]*[/.])[^\s:]+):(\d+)\s*(.*)$")
FUZZ = 3
SPAN = 4          # how many lines a single quote may legitimately span
BLOCK_CAP = 40    # and the ceiling on the block an elided quote may range over


def norm(s):
    """Whitespace-folded, case-folded, and stripped of inline markdown punctuation.

    The fourth narrowing, and the same lesson as the first three. A worker that
    quotes a fenced line as `` `type(scope): subject` `` has added a code span
    around content it copied verbatim; the file carries the line without the
    backticks, and comparing raw text called 152 honest refs fictions.
    Backticks, asterisks and underscores are MARKUP — how a quote is presented,
    never what it says — so they come off both sides before the comparison.

    **Stated honestly: the table that showed this was still being written.** The
    152 were counted mid-write, and on the finished tables every ref passes with
    or without the strip. The narrowing stays because the failure class is real
    and will recur, and because it loosens nothing measurable: re-run over the
    three already-trusted C4/C5/C6 tables returns the identical 12 unresolvable
    refs it returned before.
    """
    return re.sub(r"[`*_]", "", re.sub(r"\s+", " ", s).strip().lower())


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
    # A quote may be elided anywhere — leading (`... third supported language,
    # after Rust`), trailing (`markdown unit parser, ...`) or in the middle. The
    # rule is stated once, generally: every non-empty segment between ellipses must
    # appear IN ORDER inside the window. Two earlier versions of this check handled
    # the trailing case only and then the trailing-plus-spanning case, and each
    # reported honest quotes as fictions — the rule was narrower than the
    # convention twice before it was written down as one rule instead of a list of
    # cases.
    if re.search(r"\.\.\.|…", snippet):
        segs = [s.strip() for s in re.split(r"\.\.\.|…", snippet) if len(s.strip()) >= 8]
        if segs:
            # The window for an elided quote is the BLOCK it starts in, not a fixed
            # number of lines: an ATLAS record wraps over eleven lines and a quote
            # spanning it is one quote. Capped so a file with no blank lines cannot
            # turn the check into a whole-file substring search.
            end = line - 1
            while end < len(lines) and lines[end].strip() and end - line < BLOCK_CAP:
                end += 1
            window = norm(" ".join(lines[line - 1: max(end, line - 1 + SPAN)]))
            pos, ok_all = 0, True
            for s in segs:
                at = window.find(norm(s), pos)
                if at < 0:
                    ok_all = False
                    break
                pos = at + len(norm(s))
            if ok_all:
                shown = sum(len(s) for s in segs)
                return "ELIDED", f"{path}:{line} — {len(segs)} segment(s), {shown} chars verified around the ellipsis"
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
