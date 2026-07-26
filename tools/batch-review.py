#!/usr/bin/env python3
"""batch-review — the mechanical half of a Phase-B markup batch review.

WHAT THIS IS FOR
    A markup batch (PROP-043 wave-2, Phase B) arrives as a diff of N markdown
    files.  Reviewing it has two halves: a mechanical one (did the words
    survive, did the gate move by the predicted amount, is anything outside
    scope) and a judgement one (is the split sense-preserving, is the anchor
    name good, is an `@unknown` honest).  This tool is the FIRST HALF ONLY.

    It has been written from scratch at least four times as throwaway
    scratchpad code -- twice by batch executors verifying their own work, once
    by the B5 executor, once by the reviewer of B6.  This is that code, kept.

WHAT IT DELIBERATELY DOES NOT DO
    It does not run `vibe`.  Every parsing `vibe progress` subcommand writes
    the real `~/.vibe`, and the floor tripwires on that, so the sequencing has
    to stay visible to a human.  Run the gate yourself and pass the log:

        cargo run -q -p vibe-cli --bin vibe -- progress check --exhaustive \
            --no-cache --campaign campaigns/<zone> > gate.log 2>&1
        python tools/batch-review.py --gate-log gate.log --scope scope.txt ...

    It also does not pass judgement.  Its last section is the list of things it
    did not check, and that list is the actual review.

CALIBRATION
    `--selftest` replays the two landed batches (B5 go, B6 typescript) and the
    planted-defect controls.  A checker that has not been shown to agree with a
    known-good corpus, and to go red on a known defect, is a guess about a
    guess.  Do not trust a run of this tool on a tree where `--selftest` fails.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path

# ---------------------------------------------------------------- vocabulary
# spec://vibevm/modules/vibe-progress/PROP-043#stages / #states / #actions
STAGES = {"idea", "spec", "impl", "test", "doc", "freeze", "unknown"}
STATES = {"plan", "work", "done", "hold", "void"}
ACTIONS = {"continue", "drift", "rework", "remove"}
AUDIENCES = {"user", "author", "dev"}

# ---------------------------------------------------------------- lexemes
RE_STATUS = re.compile(r"<status\b[^>]*?/?>|</status\s*>")
RE_ATTR = re.compile(r'(\w+)\s*=\s*"([^"]*)"')
# A shorthand is recognised ONLY as a standalone, delimited token
# (`##SHORTHAND-STANDALONE`: "never mid-sentence, never inside code or links").
# The first version matched bare `@word` anywhere and produced 45 false
# positives on B6 -- `@ts-ignore`, `@typescript-eslint`, and `@implements
# spec://…` quoted inside code spans -- because it reimplemented the parser's
# rule loosely instead of reading it.  Delimiting the token kills the hyphenated
# ones; blanking inline code kills the quoted ones.
RE_SHORT = re.compile(r"(?<![\w/-])@(?P<stage>[a-z]+)(?:/(?P<state>[a-z]+))?(?![\w/-])")
RE_CODESPAN = re.compile(r"(`+)(?:(?!\1).)*?\1", re.S)
RE_FACT = re.compile(r"##(?P<id>[A-Za-z][A-Za-z0-9_-]*)")
RE_HEAD = re.compile(r"\{#(?P<id>[A-Za-z][A-Za-z0-9_-]*)\}")
RE_ORDINAL = re.compile(r"(?m)^[ \t]*\d+\.[ \t]+")
BULLET_TOKENS = {"-", "+", "*", "•"}
RE_FENCE = re.compile(r"^[ \t]*(?:```|~~~)")
RE_BULLET_LINE = re.compile(r"^[ \t]*(?:[-*+]|\d+\.)[ \t]+")
RE_HEADING = re.compile(r"^#{1,6}[ \t]")   # a heading has a SPACE; `##ANCHOR` does not


def git(*args: str) -> str:
    return subprocess.check_output(["git", *args], text=True, encoding="utf-8")


def blank_code_spans(text: str) -> str:
    """Blank inline `code`, keeping length so line numbers survive."""
    return RE_CODESPAN.sub(lambda m: " " * len(m.group(0)), text)


def marker_shorthands(text: str) -> list[re.Match]:
    """Shorthand tokens in MARKER position: standalone, at a line's start or end.

    `##SHORTHAND-FORMS` allows a shorthand only as the first or last token of a
    unit's text.  Restricting to line edges is a deliberate approximation of
    "unit edges" -- it is tighter than matching anywhere and looser than a real
    block parse, and the looseness only ever ADMITS a candidate for validation,
    never suppresses one.
    """
    out = []
    for line in text.split("\n"):
        stripped = line.strip()
        for m in RE_SHORT.finditer(line):
            head = line[:m.start()].strip()
            tail = line[m.end():].strip()
            if (head == "" or tail == "") and stripped:
                out.append(m)
    return out


def blank_fences(text: str) -> str:
    """Replace fenced-code lines with empty ones, keeping line numbering."""
    out, in_fence, marker = [], False, ""
    for line in text.split("\n"):
        t = line.lstrip()
        if in_fence:
            out.append("")
            if t.startswith(marker) and set(t.strip()) <= set(marker):
                in_fence = False
            continue
        if t.startswith("```") or t.startswith("~~~"):
            marker = "```" if t.startswith("```") else "~~~"
            in_fence = True
            out.append("")
            continue
        out.append(line)
    return "\n".join(out)


def word_stream(text: str) -> list[str]:
    """The author's words, with everything a markup pass may add removed.

    BLIND SPOT 1, DECLARED: emphasis asterisks are stripped, so this cannot see
    an emphasis change.  Ruling 12 licenses re-applying `*` when an italic
    paragraph is split, which is why they are stripped -- and why the asterisk
    DELTA is reported separately (a decrease is suspicious, an increase is
    ruling 12 working).  If ruling 12 is ever withdrawn, this function is wrong.

    BLIND SPOT 2, DECLARED, and it was found by calibration rather than
    designed: bullet characters are dropped as STANDALONE TOKENS, anywhere,
    never by line position.  The first version stripped `^[-*+] ` and went red
    on B5, whose text reads "the four queries + lifecycle" -- in the pre-batch
    revision that `+` sat at the start of a wrapped line, so the stripper ate it
    as a list marker, and B5's (documented, legal) reflow moved it inline where
    it survived.  ANY line-position-sensitive rule is unsafe here, because
    reflow is legal.  The cost of the fix is that a standalone `-`, `+` or `*`
    token added or removed in prose is invisible to this check; the cost of not
    fixing it was a red light on every legal reflow, which is worse -- a checker
    that cries wolf is a checker that gets ignored.

    Note the shape: the real parser mis-read a wrapped `+` as a bullet, which is
    why B5 reflowed the line at all, and this function then reproduced the same
    bug independently.  Fourth time this campaign has caught its own instrument.
    """
    t = RE_STATUS.sub(" ", text)
    t = RE_SHORT.sub(" ", t)
    t = RE_FACT.sub(" ", t)
    t = RE_HEAD.sub(" ", t)
    t = RE_ORDINAL.sub(" ", t)
    t = t.replace("*", " ")
    return [tok for tok in t.split() if tok not in BULLET_TOKENS]


@dataclass
class Report:
    checks: list[tuple[str, bool, str]] = field(default_factory=list)
    surfaced: list[str] = field(default_factory=list)

    def ok(self, name: str, detail: str = "") -> None:
        self.checks.append((name, True, detail))

    def fail(self, name: str, detail: str) -> None:
        self.checks.append((name, False, detail))

    def note(self, line: str) -> None:
        self.surfaced.append(line)

    @property
    def failed(self) -> bool:
        return any(not ok for _, ok, _ in self.checks)


# ---------------------------------------------------------------- checks
def c1_scope(files: list[str], scope: list[str] | None, r: Report) -> None:
    if scope is None:
        r.ok("C1 scope", "no --scope given; containment not checked")
        return
    if not scope:
        # An empty list must never read as "everything is out of scope".
        # A checker whose denominator silently became zero is this campaign's
        # single most repeated defect; refusing is the only safe reading.
        r.fail("C1 scope", "the --scope file is EMPTY -- refusing to check against a zero "
                           "denominator (a shell that produced no paths is the usual cause)")
        return
    fs, sc = set(files), set(scope)
    stray = sorted(fs - sc)
    untouched = sorted(sc - fs)
    if stray:
        r.fail("C1 scope", f"{len(stray)} file(s) outside the declared scope: " + ", ".join(stray))
    else:
        r.ok("C1 scope", f"{len(fs)} changed file(s), all inside scope")
    if untouched:
        # Not a failure: a file whose every fact came back untestable/unmarkable
        # is legitimately untouched.  Reconcile by name against the report.
        r.note(f"C1b {len(untouched)} scoped file(s) not touched: " + ", ".join(untouched))


def lazy_signature(text: str) -> list[tuple[int, str]]:
    """Paragraphs sitting directly after a list item: (line-no, first words).

    Matched on the RESULT, then diffed against the base -- a paragraph that
    already sat after a list in the source is the author's own layout, not a
    repair.  Without the diff this reported 19 candidates for B6's 2 real
    cases; with it, 2.
    """
    out = []
    lines = blank_fences(text).split("\n")
    for i in range(2, len(lines)):
        cur, blank, prev = lines[i], lines[i - 1], lines[i - 2]
        if blank.strip() != "" or not RE_BULLET_LINE.match(prev):
            continue
        # A HEADING is `#{1,6} ` -- hashes then a space.  A FACT ANCHOR is
        # `##NAME` with no space, and it is exactly what a marked ruling-30
        # paragraph starts with.  Excluding everything beginning with `#` made
        # this check silently find nothing on B6, where both real cases exist.
        # A false negative in a checker is worse than a false positive: it
        # reports clean.
        if cur[:1] in ("", " ", "\t", ">") or RE_HEADING.match(cur) \
                or RE_BULLET_LINE.match(cur) or RE_FENCE.match(cur):
            continue
        out.append((i + 1, " ".join(word_stream(cur)[:8])))
    return out


def c2_lazy_continuation(files: list[str], r: Report, base: str | None = None) -> None:
    """Find the ruling-30 signature, and claim nothing wider.

    A ruling-30 repair leaves a recognisable local pattern in the RESULT: a
    list item, then a blank line, then a line at column 0 that is not a
    heading, list item or fence -- i.e. a paragraph that used to be that item's
    lazy continuation.

    The first version flagged "any file with an added blank line", which was
    every file in the batch, because splitting a paragraph into a list adds
    blank lines by definition.  A signal that fires on 18 of 18 is not a signal.
    Detecting this properly in the DIFF would need block parsing -- a
    reimplementation of the parser, which is the thing that keeps going wrong --
    so this matches the result-side pattern instead and says so.
    """
    hits = []
    for f in files:
        new = lazy_signature(Path(f).read_text(encoding="utf-8"))
        old_keys = set()
        if base:
            try:
                old_keys = {k for _, k in lazy_signature(git("show", f"{base}:{f}"))}
            except subprocess.CalledProcessError:
                pass  # new file: everything in it is new
        for lineno, key in new:
            if key not in old_keys:
                hits.append(f"{f}:{lineno}  {key[:80]}")
    if hits:
        r.note(f"C2 {len(hits)} paragraph(s) sit directly after a list item (ruling-30 shape):")
        for h in hits:
            r.note(f"     {h}")
        r.note("     -> confirm each is a lazy-continuation repair, not a content edit")
    r.ok("C2 lazy continuation", f"{len(hits)} candidate(s) surfaced for judgement")


def c3_words(files: list[str], base: str, r: Report) -> None:
    diverged, emphasis_lost = [], []
    for f in files:
        old = git("show", f"{base}:{f}")
        new = Path(f).read_text(encoding="utf-8")
        a, b = word_stream(old), word_stream(new)
        if a != b:
            where = next((i for i, (x, y) in enumerate(zip(a, b)) if x != y), min(len(a), len(b)))
            ctx_a = " ".join(a[max(0, where - 6):where + 6])
            ctx_b = " ".join(b[max(0, where - 6):where + 6])
            diverged.append(f"{f} @word {where}\n       HEAD: {ctx_a}\n       WORK: {ctx_b}")
        if new.count("*") < old.count("*"):
            emphasis_lost.append(f"{f} ({old.count('*')} -> {new.count('*')})")
    if diverged:
        r.fail("C3 words", f"{len(diverged)} file(s) diverge:\n     " + "\n     ".join(diverged))
    else:
        r.ok("C3 words", f"{len(files)} file(s) word-identical to {base}")
    if emphasis_lost:
        r.fail("C3b emphasis", "asterisk count DECREASED (ruling 12 only permits increase): "
               + ", ".join(emphasis_lost))
    else:
        r.ok("C3b emphasis", "no file lost emphasis characters")


def c4_gate(gate: str, files: list[str], expect_unmarked: int | None,
            expect_files: list[str] | None, expect_total: int | None, r: Report) -> None:
    rows = [ln for ln in gate.split("\n") if re.match(r"^(packages|spec)/", ln)]
    total = len(rows)
    fset = set(files)
    mine = [ln for ln in rows if ln.split(":")[0] in fset]
    if expect_total is not None:
        if total == expect_total:
            r.ok("C4 corpus total", f"{total} unmarked, as predicted")
        else:
            r.fail("C4 corpus total", f"{total} unmarked, predicted {expect_total} "
                                      f"(delta {total - expect_total:+d})")
    else:
        r.ok("C4 corpus total", f"{total} unmarked (no prediction given)")

    if expect_unmarked is not None:
        if len(mine) == expect_unmarked:
            r.ok("C4b batch residual", f"{len(mine)} unmarked in the batch, as predicted")
        else:
            r.fail("C4b batch residual",
                   f"{len(mine)} unmarked in the batch, predicted {expect_unmarked}")
    if expect_files is not None:
        got = sorted({ln.split(":")[0] for ln in mine})
        want = sorted(set(expect_files))
        if got == want:
            r.ok("C4c residual files", "residual sits exactly in the predicted file(s)")
        else:
            r.fail("C4c residual files", f"residual in {got}, predicted {want}")


def c5_error_classes(gate: str, files: list[str], r: Report) -> None:
    fset = set(files)
    classes = Counter()
    for ln in gate.split("\n"):
        if not re.match(r"^(packages|spec)/", ln):
            continue
        m = re.search(r"\[([a-z-]+)\]", ln)
        if m and ln.split(":")[0] in fset:
            classes[m.group(1)] += 1
    bad = {k: v for k, v in classes.items() if k != "unmarked"}
    if bad:
        r.fail("C5 error classes", f"unexpected classes in batch files: {dict(bad)}")
    else:
        r.ok("C5 error classes", f"batch files carry only [unmarked] ({classes.get('unmarked', 0)})")


def c6_vocabulary(files: list[str], r: Report) -> None:
    bad = []
    for f in files:
        text = blank_fences(Path(f).read_text(encoding="utf-8"))
        for m in RE_STATUS.finditer(text):
            for k, v in RE_ATTR.findall(m.group(0)):
                if k == "stage" and v not in STAGES:
                    bad.append(f"{f}: stage={v!r}")
                elif k == "state" and v not in STATES:
                    bad.append(f"{f}: state={v!r}")
                elif k == "action" and v not in ACTIONS:
                    bad.append(f"{f}: action={v!r}")
                elif k == "actionstage" and v not in STAGES:
                    bad.append(f"{f}: actionstage={v!r}")
                elif k == "audience" and v not in AUDIENCES:
                    bad.append(f"{f}: audience={v!r}")
        for m in marker_shorthands(blank_code_spans(text)):
            st, sv = m.group("stage"), m.group("state")
            if st not in STAGES:
                bad.append(f"{f}: @{st}")
            elif sv is not None and sv not in STATES:
                bad.append(f"{f}: @{st}/{sv}")
    if bad:
        r.fail("C6 vocabulary", f"{len(bad)} value(s) outside the closed vocabulary: "
               + ", ".join(bad[:8]))
    else:
        r.ok("C6 vocabulary", "every stage/state/action/audience is inside PROP-043 §3.3-3.6")


def c7_anchors(files: list[str], r: Report) -> None:
    """Duplicate ids per file, across BOTH ##FACT and {#heading} forms.

    Deliberately redundant with the gate's own anchor laws.  A cross-check
    written from the spec rather than from the parser is what catches a parser
    that is blind to its own grammar -- which this campaign has now found
    three times.
    """
    bad = []
    for f in files:
        text = blank_fences(Path(f).read_text(encoding="utf-8"))
        ids = [m.group("id") for m in RE_FACT.finditer(text)]
        ids += [m.group("id") for m in RE_HEAD.finditer(text)]
        dupes = [i for i, n in Counter(ids).items() if n > 1]  # case-SENSITIVE, per F-085
        if dupes:
            bad.append(f"{f}: {', '.join(sorted(dupes)[:5])}")
    if bad:
        r.fail("C7 anchors", f"duplicate id(s) in {len(bad)} file(s): " + "; ".join(bad))
    else:
        r.ok("C7 anchors", "no id collides with another in its file (case-sensitive)")


def c8_encoding(files: list[str], r: Report) -> None:
    bad = []
    for f in files:
        raw = Path(f).read_bytes()
        if raw.startswith(b"\xef\xbb\xbf"):
            bad.append(f"{f}: BOM")
        if b"\r\n" in raw:
            bad.append(f"{f}: CRLF")
    if bad:
        r.fail("C8 encoding", ", ".join(bad))
    else:
        r.ok("C8 encoding", "no BOM, no CRLF")


def c9_markers_in_fences(files: list[str], r: Report) -> None:
    bad = []
    for f in files:
        text = blank_code_spans(Path(f).read_text(encoding="utf-8"))
        blanked = blank_fences(text)
        for name, rx in (("status", RE_STATUS), ("anchor", RE_FACT)):
            if len(rx.findall(text)) != len(rx.findall(blanked)):
                bad.append(f"{f}: {name} inside a fence")
        if len(marker_shorthands(text)) != len(marker_shorthands(blanked)):
            bad.append(f"{f}: shorthand inside a fence")
    if bad:
        r.fail("C9 fences", ", ".join(bad))
    else:
        r.ok("C9 fences", "no marker or anchor inside a fenced block")


def c10_unknowns(files: list[str], r: Report) -> None:
    """Not pass/fail.  The judgement queue."""
    found = []
    for f in files:
        for n, line in enumerate(Path(f).read_text(encoding="utf-8").split("\n"), 1):
            if RE_SHORT.search(line) and "@unknown" in line:
                found.append(f"{f}:{n}  {line.strip()[:110]}")
            elif 'state="hold"' in line:
                found.append(f"{f}:{n}  {line.strip()[:110]}")
    r.ok("C10 unknowns", f"{len(found)} unit(s) held for triage")
    for line in found:
        r.note(f"C10 {line}")


NOT_CHECKED = """\
  - whether a split preserved SENSE (words survive; meaning is not a token stream)
  - whether an anchor NAME is good, or its register (UPPER vs kebab) is right
  - whether an @unknown is honest or an evasion
  - whether a structural insertion is a repair or a content edit
  - whether a stage/state is CORRECT -- only that it is spellable
  - whether a reported semantic problem is real
  - whether the BRIEF was right: its scope, its counts, its predictions
  - emphasis changes (see word_stream's declared blind spot)
"""


def run_checks(base: str, files: list[str], gate: str, scope, expect_unmarked,
               expect_files, expect_total) -> Report:
    r = Report()
    c1_scope(files, scope, r)
    c2_lazy_continuation(files, r, base)
    c3_words(files, base, r)
    if gate:
        c4_gate(gate, files, expect_unmarked, expect_files, expect_total, r)
        c5_error_classes(gate, files, r)
    c6_vocabulary(files, r)
    c7_anchors(files, r)
    c8_encoding(files, r)
    c9_markers_in_fences(files, r)
    c10_unknowns(files, r)
    return r


def emit(r: Report) -> int:
    print("=" * 72)
    for name, ok, detail in r.checks:
        print(f"  {'PASS' if ok else 'FAIL'}  {name:<22} {detail}")
    if r.surfaced:
        print("\n  SURFACED (not judged):")
        for line in r.surfaced:
            print(f"    {line}")
    print("\n  THIS TOOL DID NOT CHECK:")
    print(NOT_CHECKED, end="")
    print("=" * 72)
    verdict = "MECHANICAL CHECKS FAILED" if r.failed else "mechanical checks clean -- now read the diff"
    print(f"  {verdict}")
    return 1 if r.failed else 0


def read_list(p: str | None) -> list[str] | None:
    if not p:
        return None
    return [ln.strip() for ln in Path(p).read_text(encoding="utf-8").split("\n") if ln.strip()]


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--base", default="HEAD", help="commit the batch started from")
    ap.add_argument("--commit", help="review a landed batch at this commit instead of the worktree")
    ap.add_argument("--gate-log", help="output of `vibe progress check --exhaustive`")
    ap.add_argument("--scope", help="file with the batch's declared path list")
    ap.add_argument("--expect-unmarked", type=int, help="predicted residual in the batch")
    ap.add_argument("--expect-residual", help="file listing paths allowed to carry residual")
    ap.add_argument("--expect-total", type=int, help="predicted corpus-wide unmarked total")
    ap.add_argument("--selftest", action="store_true", help="calibrate against landed batches")
    a = ap.parse_args()

    if a.selftest:
        return selftest()

    if a.commit:
        base = f"{a.commit}~1"
        files = git("diff", "--name-only", base, a.commit).split()
    else:
        base = a.base
        files = git("diff", "--name-only", base).split()
    files = [f for f in files if f.endswith(".md")]
    if not files:
        print("no markdown files changed -- nothing to review")
        return 0
    if a.commit:
        print("NOTE: --commit reviews a landed batch; C3 reads the worktree, so this is\n"
              "      only meaningful when the worktree still matches that commit.")

    gate = Path(a.gate_log).read_text(encoding="utf-8", errors="replace") if a.gate_log else ""
    if not gate:
        print("NOTE: no --gate-log; C4 and C5 skipped. Run the gate and pass its output.")

    r = run_checks(base, files, gate, read_list(a.scope), a.expect_unmarked,
                   read_list(a.expect_residual), a.expect_total)
    return emit(r)


# ---------------------------------------------------------------- calibration
def selftest() -> int:
    """Replay landed batches, then plant defects and require red.

    A checker shown only to pass is a checker shown nothing.
    """
    import tempfile

    failures = 0
    print("=== calibration: landed batches must come back clean ===")
    for name, commit in (("B5 go", "d3242f99"), ("B6 typescript", "12e12d4c")):
        try:
            base = f"{commit}~1"
            files = [f for f in git("diff", "--name-only", base, commit).split()
                     if f.endswith(".md")]
        except subprocess.CalledProcessError:
            print(f"  SKIP {name}: commit {commit} not in this history")
            continue
        r = Report()
        # C3 against the landed blobs, not the worktree.
        diverged = []
        for f in files:
            a = word_stream(git("show", f"{base}:{f}"))
            b = word_stream(git("show", f"{commit}:{f}"))
            if a != b:
                diverged.append(f)
        if diverged:
            print(f"  FAIL {name}: word stream diverges in {diverged}")
            failures += 1
        else:
            print(f"  ok   {name}: {len(files)} files word-identical across the batch")

    print("\n=== negative controls: each planted defect must be caught ===")
    sample = "# T {#root}\n\n<status stage=\"spec\" state=\"done\"/>\n\n" \
             "##FACT-ONE The *quick* brown fox jumps. @impl/done\n\n" \
             "##fact-two It landed cleanly. @spec/done\n"
    controls = [
        ("reworded sentence", sample.replace("jumps", "leaps"), "C3 words"),
        ("dropped word", sample.replace("brown ", ""), "C3 words"),
        ("lost emphasis", sample.replace("*quick*", "quick"), "C3b emphasis"),
        ("bad stage", sample.replace("@impl/done", "@impll/done"), "C6 vocabulary"),
        ("bad state", sample.replace("@spec/done", "@spec/finished"), "C6 vocabulary"),
        ("duplicate id", sample.replace("##fact-two", "##FACT-ONE"), "C7 anchors"),
        ("CRLF", sample.replace("\n", "\r\n"), "C8 encoding"),
    ]
    with tempfile.TemporaryDirectory() as d:
        for label, mutated, expect in controls:
            p = Path(d) / "probe.md"
            p.write_text(mutated, encoding="utf-8", newline="")
            r = Report()
            a, b = word_stream(sample), word_stream(mutated)
            if a != b:
                r.fail("C3 words", "diverges")
            else:
                r.ok("C3 words", "identical")
            if mutated.count("*") < sample.count("*"):
                r.fail("C3b emphasis", "decreased")
            c6_vocabulary([str(p)], r)
            c7_anchors([str(p)], r)
            c8_encoding([str(p)], r)
            caught = [n for n, ok, _ in r.checks if not ok]
            if expect in caught:
                print(f"  ok   {label:<20} -> caught by {expect}")
            else:
                print(f"  FAIL {label:<20} -> expected {expect}, got {caught or 'nothing'}")
                failures += 1

        # C2 has its own control, because its first version reported CLEAN on a
        # tree containing two real cases -- it excluded every line starting
        # with `#`, which is also how a marked paragraph starts.
        lazy = "# T {#root}\n\n- ##ITEM-ONE first item @impl/done\n" \
               "- ##ITEM-TWO second item @impl/done\n\n" \
               "##RULE-SOMETHING *Rule:* the law over the list. @impl/done\n"
        p = Path(d) / "lazy.md"
        p.write_text(lazy, encoding="utf-8", newline="")
        r = Report()
        c2_lazy_continuation([str(p)], r)
        if any("1 candidate" in det for _, _, det in r.checks):
            print("  ok   lazy continuation      -> C2 surfaces the ruling-30 shape")
        else:
            print(f"  FAIL lazy continuation      -> C2 found nothing")
            failures += 1

        # And the control that matters most: a CLEAN sample must stay clean.
        p = Path(d) / "clean.md"
        p.write_text(sample, encoding="utf-8", newline="")
        r = Report()
        c6_vocabulary([str(p)], r)
        c7_anchors([str(p)], r)
        c8_encoding([str(p)], r)
        c9_markers_in_fences([str(p)], r)
        if r.failed:
            print(f"  FAIL clean sample went red: {[n for n, ok, _ in r.checks if not ok]}")
            failures += 1
        else:
            print("  ok   clean sample                -> stays green")

    print()
    print("calibration FAILED" if failures else "calibration clean -- the tool may be trusted")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
