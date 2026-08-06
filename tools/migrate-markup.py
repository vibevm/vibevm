#!/usr/bin/env python3
"""Rewrite the progress markup into its qualified spelling.

    ##ID            ->  @fact:ID
    @stage/state    ->  @status:stage/state
    @stage          ->  @status:stage

Usage:
    python tools/migrate-markup.py                 # dry run, report only
    python tools/migrate-markup.py --apply         # write the files
    python tools/migrate-markup.py --reverse       # dry run of the inverse
    python tools/migrate-markup.py --reverse --apply
    python tools/migrate-markup.py --self-test     # prove it is not blind

Why a script and not a worker: this is ~25 000 substitutions across ~550
files. A language model would produce a small, silent error rate that is
indistinguishable from success; a program either handles a case or visibly
does not.

Why not `sed`: 201 markers live inside inline code spans — they are
QUOTATIONS of the syntax inside documents that define it, and rewriting them
would corrupt exactly the files that explain the format. This program is
markdown-aware: it never edits inside a fenced block or an inline code span.

What it deliberately does NOT do: it does not touch `@spec://…` (a foreign
citation grammar), it does not touch real ATX headings (`## ` with a space —
a heading needs that space, which is why this markup is written closed up),
and it refuses to guess. An `@word` whose word is not a known stage is left
exactly as it was; it is reported, never rewritten.
"""

from __future__ import annotations

import argparse
import pathlib
import re
import sys
import collections

# Vocabularies, copied from crates/progress-core/src/model.rs. If the reader
# and this program ever disagree, the migration means something different
# from what the parser will read back — so they are asserted equal by the
# corpus counts, not trusted.
STAGES = ("idea", "spec", "impl", "test", "doc", "freeze", "unknown")
STATES = ("plan", "work", "done", "hold", "void")

ROOTS = ("spec", "packages", "campaigns", "docs")
EXTRA_FILES = ("CLAUDE.md", "AGENTS.md", "GEMINI.md", "SPECSPACES.md",
               "BACKLOG.md", "TASKS.md", "AUDIT.md", "CONTINUE.md",
               "ROADMAP.md", "TOOLING-MAP.md", "README.md")

# Generated / vendored trees: regenerated from the sources above, never
# migrated in place (that is how a source and its copy drift apart).
SKIP_DIRS = {"vibedeps", ".vibe", "target", ".git", "node_modules", ".wt",
             "refs", "fixtures"}

FACT = re.compile(r"##([A-Za-z][A-Za-z0-9_-]*)(?=\s|$)")
FACT_BACK = re.compile(r"@fact:([A-Za-z][A-Za-z0-9_-]*)(?=\s|$)")
STATUS = re.compile(
    r"@(" + "|".join(STAGES) + r")(/(" + "|".join(STATES) + r"))?\b(?!:)"
)
STATUS_BACK = re.compile(
    r"@status:(" + "|".join(STAGES) + r")(/(" + "|".join(STATES) + r"))?\b"
)
# Any `@word` at all, so the program can REPORT what it chose not to touch.
ANY_AT = re.compile(r"(?<![\w.:/-])@([A-Za-z][\w-]*)")
FENCE = re.compile(r"^\s*(`{3,}|~{3,})")
INLINE = re.compile(r"(`+)(.*?)\1", re.S)


def segments(line: str):
    """Split a line into (text, is_code) pieces; code is never rewritten."""
    out, last = [], 0
    for m in INLINE.finditer(line):
        if m.start() > last:
            out.append((line[last:m.start()], False))
        out.append((m.group(0), True))
        last = m.end()
    out.append((line[last:], False))
    return out


def rewrite_text(s: str, reverse: bool) -> str:
    if reverse:
        s = FACT_BACK.sub(r"##\1", s)
        return STATUS_BACK.sub(lambda m: "@" + m.group(1) + (m.group(2) or ""), s)
    s = FACT.sub(r"@fact:\1", s)
    # `@spec://` must survive: the negative lookahead on `:` in STATUS keeps
    # `@spec:` out, but `@spec://` is `@spec` + `://`, so it is excluded here.
    def status_sub(m: re.Match) -> str:
        if s[m.end():m.end() + 3] == "://":
            return m.group(0)
        return "@status:" + m.group(1) + (m.group(2) or "")
    return STATUS.sub(status_sub, s)


def process(text: str, reverse: bool, stats: collections.Counter,
            untouched: collections.Counter):
    lines = text.split("\n")
    out = []
    fence = None
    for line in lines:
        m = FENCE.match(line)
        if fence is None and m:
            fence = m.group(1)[0] * 3
            out.append(line)
            continue
        if fence is not None:
            if m and m.group(1)[0] * 3 == fence:
                fence = None
            out.append(line)
            continue

        new_parts = []
        for seg, is_code in segments(line):
            if is_code:
                stats["skipped_inline_code"] += len(FACT.findall(seg)) + len(
                    STATUS.findall(seg)
                )
                new_parts.append(seg)
                continue
            before_f = len(FACT.findall(seg)) if not reverse else len(
                FACT_BACK.findall(seg))
            before_s = len(STATUS.findall(seg)) if not reverse else len(
                STATUS_BACK.findall(seg))
            for tok in ANY_AT.findall(seg):
                if tok not in STAGES and tok not in ("fact", "status"):
                    untouched[f"@{tok}"] += 1
            seg2 = rewrite_text(seg, reverse)
            stats["facts"] += before_f
            stats["statuses"] += before_s
            new_parts.append(seg2)
        out.append("".join(new_parts))
    return "\n".join(out)


def targets(root: pathlib.Path):
    for name in EXTRA_FILES:
        p = root / name
        if p.is_file():
            yield p
    for d in ROOTS:
        base = root / d
        if not base.is_dir():
            continue
        for p in base.rglob("*.md"):
            if any(part in SKIP_DIRS for part in p.parts):
                continue
            yield p


def self_test() -> int:
    """Prove the program is not blind: each case must behave as stated."""
    cases = [
        ("##ID text @impl/done", "@fact:ID text @status:impl/done"),
        ("## A real heading", "## A real heading"),
        ("- ##ROW cell @spec/work", "- @fact:ROW cell @status:spec/work"),
        ("see `##ID` and `@impl/done`", "see `##ID` and `@impl/done`"),
        ("@spec://org/x#y stays", "@spec://org/x#y stays"),
        ("@impl bare", "@status:impl bare"),
        ("@idea/plan legal", "@status:idea/plan legal"),
        ("@vasya untouched", "@vasya untouched"),
        ("##9bad not an id", "##9bad not an id"),
        ("##ID- trailing", "@fact:ID- trailing"),
    ]
    bad = 0
    for src, want in cases:
        got = process(src, False, collections.Counter(), collections.Counter())
        if got != want:
            print(f"  FAIL  {src!r}\n        got  {got!r}\n        want {want!r}")
            bad += 1
    # a fence must be untouched end to end
    fenced = "```bash\n##ID @impl/done\n```\n"
    got = process(fenced, False, collections.Counter(), collections.Counter())
    if got != fenced:
        print(f"  FAIL fence: {got!r}")
        bad += 1
    # round trip
    doc = "##A one @impl/done\n\n- ##B two @spec/work\n"
    fwd = process(doc, False, collections.Counter(), collections.Counter())
    back = process(fwd, True, collections.Counter(), collections.Counter())
    if back != doc:
        print(f"  FAIL round-trip:\n  {doc!r}\n  {back!r}")
        bad += 1
    # idempotence
    twice = process(fwd, False, collections.Counter(), collections.Counter())
    if twice != fwd:
        print(f"  FAIL idempotence:\n  {fwd!r}\n  {twice!r}")
        bad += 1
    print(f"self-test: {len(cases) + 3} checks, {bad} failure(s)")
    return 1 if bad else 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--apply", action="store_true")
    ap.add_argument("--reverse", action="store_true")
    ap.add_argument("--self-test", action="store_true")
    ap.add_argument("--root", default=".")
    a = ap.parse_args()

    if a.self_test:
        return self_test()

    root = pathlib.Path(a.root).resolve()
    stats = collections.Counter()
    untouched = collections.Counter()
    changed, scanned = [], 0

    for p in targets(root):
        scanned += 1
        try:
            text = p.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            stats["unreadable"] += 1
            continue
        new = process(text, a.reverse, stats, untouched)
        if new != text:
            changed.append(p)
            if a.apply:
                p.write_text(new, encoding="utf-8", newline="")

    verb = "reverse" if a.reverse else "forward"
    mode = "APPLIED" if a.apply else "dry run"
    print(f"markup migration ({verb}, {mode})")
    print(f"  files scanned            : {scanned}")
    print(f"  files that change        : {len(changed)}")
    print(f"  fact markers rewritten   : {stats['facts']}")
    print(f"  status tokens rewritten  : {stats['statuses']}")
    print(f"  markers inside code kept : {stats['skipped_inline_code']}")
    print()
    print("  `@` tokens deliberately left alone (must stay constant):")
    for t, c in untouched.most_common(15):
        print(f"    {c:6d}  {t}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
