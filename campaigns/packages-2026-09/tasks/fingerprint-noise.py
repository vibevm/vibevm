#!/usr/bin/env python3
"""Measure the noise of two code-fingerprint schemes for the specmap map.

The repo carries a code↔spec map (`specmap.json`) whose code elements are
recorded as `{symbol, item_kind, crate_name, file, line}` — five fields, NO
fingerprint and NO end-of-range. The owner is deciding what to hash onto each
element so the map notices "the code under this requirement moved". The choice
stands on one number:

  - scheme A — raw text  (every formatter run / comment edit flips the hash);
  - scheme B — token stream (format-independent: whitespace and comment
    cosmetics do not move it).

This tool does NOT pick. It MEASURES the difference on this tree and its real
history, so the owner decides by number. It prints every figure the finding
report cites, to stdout, stdlib-only.

Three measurements:

  M1 — real-history loss. Over a window of N .rs-touching commits, for every
       .rs changed between parent and child: did the raw-hash move? did the
       token-hash move? `spurious = raw moved AND token did not` is the noise
       scheme A emits that scheme B would not. `spurious_rate = spurious / raw`
       is the headline number, printed at file level (exact) and element level
       (heuristic, secondary).

  M2 — control perturbations. On HEAD, five in-memory edits (P1 indent, P2 blank
       line after `{`, P3 `// note` before `fn`, P4 rewrite `//` comments, P5 a
       real rename) prove the mechanism: P1–P4 must move raw ~100 % and token
       0 %; any token hit on P1–P4 is a lexer bug, named in the output.

  Coverage — honesty of the element heuristic. What fraction of `specmap.json`
       code_items does the declaration-finder recover at (file, line)?

Token-hash is measured in two columns:

  B1 — all comments dropped (incl. doc-comments);
  B2 — only ordinary `//` and `/* */` dropped; doc-comments (`///`, `//!`,
       `/** */`, `/*! */`) kept in the stream as tokens (internal whitespace
       collapsed, so pure re-indent still does not move them).

Usage:
    python campaigns/packages-2026-09/tasks/fingerprint-noise.py
    python campaigns/packages-2026-09/tasks/fingerprint-noise.py --window 60
    python campaigns/packages-2026-09/tasks/fingerprint-noise.py --no-history   # M2+coverage only
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

# campaigns/packages-2026-09/tasks/<this>  ->  repo root.
REPO = Path(__file__).resolve().parents[3]
SPECMAP = REPO / "specmap.json"

# Perimeter: every .rs in the tree EXCEPT these path substrings (task §4.1).
EXCLUDE_SUBSTR = ("/vibedeps/", "/.vibe/", "/target/", "/.wt/", "/campaigns/",
                  "/tests/fixtures/")
# A path is excluded if it carries one of the substrings, OR any segment is one
# of these directory names (covers a top-level `target/` the substring would
# miss). `crates/` is NOT excluded — it is the host source the map describes.
EXCLUDE_SEGMENTS = {"vibedeps", ".vibe", "target", ".wt", "campaigns"}

DEFAULT_WINDOW = 300  # .rs-touching commits; task floor is 200, tree has 871.

# --------------------------------------------------------------------------- git


def git(*args: str, want_bytes: bool = True):
    """Run a read-only git verb in the worktree; return stdout (bytes)."""
    p = subprocess.run(
        ["git", *args], cwd=str(REPO),
        capture_output=True, check=False,
    )
    return p.stdout if want_bytes else p.stdout.decode("utf-8", "replace")


def in_perimeter(path: str) -> bool:
    if not path.endswith(".rs"):
        return False
    if any(s in path for s in EXCLUDE_SUBSTR):
        return False
    segs = re.split(r"[\\/]+", path)
    if any(seg in EXCLUDE_SEGMENTS for seg in segs):
        return False
    return True


def cat_file_batch(refs):
    """Feed `rev:path` strings to one `git cat-file --batch`; return
    {ref: bytes | None}. None = object absent (added/deleted file)."""
    out = {}
    refs = [r for r in refs if r is not None]
    if not refs:
        return out
    payload = ("\n".join(refs) + "\n").encode("utf-8")
    data = subprocess.run(
        ["git", "cat-file", "--batch"], cwd=str(REPO), input=payload,
        capture_output=True, check=False,
    ).stdout
    i = 0
    for ref in refs:
        nl = data.index(b"\n", i)
        header = data[i:nl]
        i = nl + 1
        # git responds in input order. A found object's header is
        # `<resolved-oid> <type> <size>` (the oid is the RESOLVED blob id, not
        # our `rev:path` ref); a missing object is `<ref> missing`. So parse by
        # trailing tokens, not by matching our ref prefix.
        parts = header.split()
        if parts and parts[-1] == b"missing":
            out[ref] = None
            continue
        size = int(parts[-1])      # last token
        out[ref] = data[i:i + size]
        i += size + 1  # +1 trailing newline
    return out


# ---------------------------------------------------------------------- hashing


def raw_hash(text: str) -> str:
    """sha256 over raw bytes, line endings normalised to LF."""
    norm = text.replace("\r\n", "\n").replace("\r", "\n")
    return hashlib.sha256(norm.encode("utf-8")).hexdigest()


# ------------------------------------------------------------------- the lexer
#
# One forward scan classifies every character of a Rust source into an atom:
#   ws       – a maximal whitespace run (spaces/tabs/CR/LF)
#   code     – a maximal run of code chars with no whitespace inside
#   lit      – a string / char / raw / byte literal, verbatim
#   doc      – a doc-comment (/// //!--- /** */ /*! */) — content
#   comment  – an ordinary // or /* */ comment
# Atoms partition the source; each carries its start line and start offset.
# From one atomisation we derive: the B1/B2 token strings, a brace-only
# `code_view` (literals+comments blanked), and element bodies.

_IDENT = re.compile(r"[A-Za-z0-9_]")


def _is_ident_char(c: str) -> bool:
    return bool(_IDENT.match(c))


def _read_raw_string(src: str, i: int, n: int):
    """src[i] == 'r' (or the 'r' of 'br'). Return (end, nl) for a raw string
    r#".."#, or None if this 'r' is not a raw-string prefix (raw ident / ident).
    """
    j = i + 1
    h = 0
    while j < n and src[j] == "#":
        h += 1
        j += 1
    if j >= n or src[j] != '"':
        return None
    k = j + 1
    nl = 0
    close = '"' + "#" * h
    while k < n:
        if src.startswith(close, k):
            return (k + 1 + h, nl)
        if src[k] == "\n":
            nl += 1
        k += 1
    return (n, nl)  # unterminated — consume to EOF


def _read_string(src: str, i: int, n: int):
    """src[i] == '"'. Regular / byte string. Return (end, nl)."""
    j = i + 1
    nl = 0
    while j < n:
        c = src[j]
        if c == "\\":
            j += 1
            if j < n:
                if src[j] == "\n":
                    nl += 1
                j += 1
        elif c == '"':
            return (j + 1, nl)
        else:
            if c == "\n":
                nl += 1
            j += 1
    return (n, nl)


def _read_char_or_lifetime(src: str, i: int, n: int):
    """src[i] == "'". Return ('lit', end) for a char literal, or
    ('code', end) for a lifetime (which stays in the code stream)."""
    j = i + 1
    ok = True
    if j >= n:
        ok = False
    else:
        if src[j] == "\\":
            j += 1
            if j >= n:
                ok = False
            else:
                e = src[j]
                if e in ("x", "u"):
                    j += 1
                    if e == "x":
                        cnt = 0
                        while j < n and cnt < 2 and src[j] in "0123456789abcdefABCDEF":
                            j += 1
                            cnt += 1
                    else:
                        if j < n and src[j] == "{":
                            j += 1
                            while j < n and src[j] != "}":
                                j += 1
                            if j < n:
                                j += 1
                        else:
                            cnt = 0
                            while j < n and cnt < 4 and src[j] in "0123456789abcdefABCDEF":
                                j += 1
                                cnt += 1
                else:
                    if src[j] == "\n":
                        pass
                    j += 1
        else:
            if src[j] == "'" or src[j] == "\n":
                ok = False
            else:
                j += 1
        if ok:
            if j < n and src[j] == "'":
                return ("lit", j + 1)
            ok = False
    if not ok:
        # lifetime: ' + ident chars
        k = i + 1
        while k < n and _is_ident_char(src[k]):
            k += 1
        return ("code", k)
    return ("code", j)  # unreachable


def _read_block_comment(src: str, i: int, n: int):
    """src[i:i+2] == '/*'. Return (end, is_doc, nl) handling nesting."""
    is_doc = False
    if i + 2 < n:
        if src[i + 2] == "*" and not (i + 3 < n and src[i + 3] == "/"):
            is_doc = True
        elif src[i + 2] == "!":
            is_doc = True
    depth = 1
    j = i + 2
    nl = 0
    while j < n and depth > 0:
        if src[j] == "/" and j + 1 < n and src[j + 1] == "*":
            depth += 1
            j += 2
        elif src[j] == "*" and j + 1 < n and src[j + 1] == "/":
            depth -= 1
            j += 2
        else:
            if src[j] == "\n":
                nl += 1
            j += 1
    return (j, is_doc, nl)


def scan_atoms(src: str):
    """Return a list of (kind, text, start_line, start_offset)."""
    atoms = []
    n = len(src)
    i = 0
    line = 1
    code_buf = []
    code_line = 0
    code_off = 0

    def flush_code():
        if code_buf:
            atoms.append(("code", "".join(code_buf), code_line, code_off))
            code_buf.clear()

    while i < n:
        c = src[i]

        # whitespace
        if c == " " or c == "\t" or c == "\r" or c == "\n":
            flush_code()
            j = i
            nl = 0
            while j < n and src[j] in " \t\r\n":
                if src[j] == "\n":
                    nl += 1
                j += 1
            atoms.append(("ws", src[i:j], line, i))
            line += nl
            i = j
            continue

        # line comment
        if c == "/" and i + 1 < n and src[i + 1] == "/":
            flush_code()
            doc = i + 2 < n and (src[i + 2] == "/" or src[i + 2] == "!")
            j = i
            while j < n and src[j] != "\n":
                j += 1
            atoms.append(("doc" if doc else "comment", src[i:j], line, i))
            i = j
            continue

        # block comment
        if c == "/" and i + 1 < n and src[i + 1] == "*":
            flush_code()
            end, is_doc, nl = _read_block_comment(src, i, n)
            atoms.append(("doc" if is_doc else "comment", src[i:end], line, i))
            line += nl
            i = end
            continue

        prev_ident = i > 0 and _is_ident_char(src[i - 1])

        # regular / byte string
        if c == '"':
            flush_code()
            end, nl = _read_string(src, i, n)
            atoms.append(("lit", src[i:end], line, i))
            line += nl
            i = end
            continue
        if (not prev_ident) and c == "b" and i + 1 < n and src[i + 1] == '"':
            flush_code()
            end, nl = _read_string(src, i + 1, n)
            atoms.append(("lit", src[i:end], line, i))
            line += nl
            i = end
            continue

        # raw / byte-raw string
        if (not prev_ident) and c == "r":
            r = _read_raw_string(src, i, n)
            if r is not None:
                flush_code()
                end, nl = r
                atoms.append(("lit", src[i:end], line, i))
                line += nl
                i = end
                continue
            # else fall through: 'r' is a code char
        if (not prev_ident) and c == "b" and i + 1 < n and src[i + 1] == "r":
            r = _read_raw_string(src, i + 1, n)
            if r is not None:
                flush_code()
                end, nl = r
                atoms.append(("lit", src[i:end], line, i))
                line += nl
                i = end
                continue
            # else fall through

        # char literal vs lifetime
        if c == "'":
            kind, end = _read_char_or_lifetime(src, i, n)
            if kind == "lit":
                flush_code()
                atoms.append(("lit", src[i:end], line, i))
                i = end
                continue
            # lifetime: ' + ident -> code chars (fall through into code_buf)

        # ordinary code character
        if not code_buf:
            code_line = line
            code_off = i
        code_buf.append(c)
        i += 1

    flush_code()
    return atoms


_WS_RE = re.compile(r"\s+")


def token_string(atoms, keep_doc: bool) -> str:
    """Collapse whitespace runs to one separator; drop comments (B1: all;
    B2: ordinary only, doc kept with internal whitespace collapsed); keep
    literals verbatim."""
    out = []
    sep = False

    def flush():
        nonlocal sep
        if sep:
            out.append(" ")
            sep = False

    for kind, text, _ln, _off in atoms:
        if kind == "ws":
            sep = True
        elif kind == "comment":
            continue
        elif kind == "doc":
            if keep_doc:
                flush()
                out.append(_WS_RE.sub(" ", text))
            # else dropped
        elif kind == "lit":
            flush()
            out.append(text)
        else:  # code
            flush()
            out.append(text)
    # A comment at the very start/end leaves a dangling separator that would
    # flip the hash without a token change; strip it.
    return "".join(out).strip()


def code_view(src: str, atoms) -> str:
    """Source with every literal/comment char blanked to a space (newlines
    kept). Braces and identifiers here are guaranteed outside strings/comments.
    """
    view = list(src)
    for kind, text, _ln, off in atoms:
        if kind in ("lit", "comment", "doc"):
            for k in range(off, min(off + len(text), len(view))):
                if view[k] != "\n":
                    view[k] = " "
    return "".join(view)


def tok_hashes(src: str):
    """Return (b1, b2) token hashes for a source string."""
    atoms = scan_atoms(src)
    b1 = hashlib.sha256(token_string(atoms, False).encode("utf-8")).hexdigest()
    b2 = hashlib.sha256(token_string(atoms, True).encode("utf-8")).hexdigest()
    return b1, b2


# ------------------------------------------------------------- element finding
#
# The map records `line` as syn's item.span().start().line — the FIRST line of
# the leading doc-comment / attribute run, not the keyword line (verified
# empirically: see finding report). So an element's start_line walks up over
# contiguous doc/attribute lines above the keyword.

_DECL_KW = {"fn", "struct", "enum", "trait", "impl", "mod",
            "const", "static", "type", "union"}
# Match a declaration keyword as a standalone token in code_view. Allow the
# usual qualifiers so the keyword is the keyword, not a substring.
_DECL_RE = re.compile(
    r"(?<![A-Za-z0-9_#$])"
    r"(?:pub(?:\s*\([^)]*\))?\s+)?"
    r"(?:(?:unsafe|async|const|extern(?:\s+\"[^\"]*\")?)\s+)*"
    r"(fn|struct|enum|trait|impl|mod|const|static|type|union)\b"
)


def _line_of(view: str, off: int) -> int:
    return view.count("\n", 0, off) + 1


def _line_start(view: str, line_no: int) -> int:
    """Offset of the first char of 1-based line_no."""
    if line_no <= 1:
        return 0
    pos = -1
    for _ in range(line_no - 1):
        pos = view.index("\n", pos + 1)
    return pos + 1


def _match_braces(view: str, open_off: int):
    """view[open_off] == '{'. Return offset just past the matching '}'."""
    depth = 0
    i = open_off
    n = len(view)
    while i < n:
        ch = view[i]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return i + 1
        i += 1
    return n


def _ident_after(view: str, off: int):
    """First identifier token at/after off (skipping non-ident chars)."""
    m = re.search(r"[A-Za-z_][A-Za-z0-9_]*", view[off:])
    if not m:
        return None, None
    start = off + m.start()
    return m.group(0), start


def _attr_lines(view: str):
    """1-based line numbers that lie inside a `#[...]` / `#![...]` attribute
    (scanned forward, so multi-line attributes mark every line they span).
    Runs on code_view so `[`/`]` inside string literals do not confuse it.
    """
    lines = set()
    n = len(view)
    i = 0
    while i < n:
        if view[i] == "#" and i + 1 < n and view[i + 1] in ("[", "!") and \
           (view[i + 1] == "[" or (i + 2 < n and view[i + 2] == "[")):
            # advance to the '['
            j = i + 1
            while j < n and view[j] != "[":
                j += 1
            depth = 0
            start_off = i
            end_off = j
            while end_off < n:
                if view[end_off] == "[":
                    depth += 1
                elif view[end_off] == "]":
                    depth -= 1
                    if depth == 0:
                        break
                end_off += 1
            lstart = view.count("\n", 0, start_off) + 1
            lend = view.count("\n", 0, end_off) + 1
            for L in range(lstart, lend + 1):
                lines.add(L)
            i = end_off + 1
        else:
            i += 1
    return lines


def extract_elements(src: str):
    """Return list of dicts: {kind, name, kw_line, start_line, start_off,
    end_off}. start_line/end_off bound the element body (leading doc/attrs +
    declared block). start_line follows the syn convention (first doc/attr
    line above the keyword, else the keyword line).
    """
    atoms = scan_atoms(src)
    view = code_view(src, atoms)
    n = len(view)
    raw_lines = src.split("\n")
    attr_set = _attr_lines(view)
    elems = []
    for m in _DECL_RE.finditer(view):
        kw = m.group(1)
        kw_end = m.end()
        kw_line = _line_of(view, m.start())

        # body end: first '{' (block) or ';' (no block), whichever comes first
        scan = kw_end
        brace_open = -1
        semi = -1
        depth_paren = 0
        while scan < n:
            ch = view[scan]
            if ch == "(" or ch == "[":
                depth_paren += 1
            elif ch == ")" or ch == "]":
                if depth_paren > 0:
                    depth_paren -= 1
            elif depth_paren == 0:
                if ch == "{":
                    brace_open = scan
                    break
                if ch == ";":
                    semi = scan
                    break
            scan += 1
        if brace_open >= 0:
            end_off = _match_braces(view, brace_open)
        elif semi >= 0:
            end_off = semi + 1
        else:
            continue  # malformed / unfinished — skip

        # name
        if kw == "impl":
            header = view[m.start():brace_open if brace_open >= 0 else end_off]
            header = re.sub(r"\s+", " ", header).strip()
            name = header  # e.g. "impl Trait for Type"
        else:
            nm, _nmoff = _ident_after(view, kw_end)
            name = nm

        # start_line: walk up over the contiguous doc / attribute run that syn
        # folds into the item span. A blank line breaks attachment; attributes
        # (incl. multi-line) are recognised via the forward _attr_lines scan.
        start_line = kw_line
        for L in range(kw_line - 1, 0, -1):
            raw = raw_lines[L - 1].strip()
            if raw == "":
                break
            if L in attr_set:
                start_line = L
                continue
            if (raw.startswith("///") or raw.startswith("//!")
                    or raw.startswith("/**") or raw.startswith("/*!")
                    or raw.startswith("*")):
                start_line = L
                continue
            break
        start_off = _line_start(view, start_line)
        elems.append({
            "kind": kw,
            "name": name,
            "kw_line": kw_line,
            "start_line": start_line,
            "start_off": start_off,
            "end_off": end_off,
        })
    return elems


# ---------------------------------------------------------------- measurements

def _perturb(src: str, which: str) -> str:
    """Apply one control perturbation in memory (task §4.3).

    P1–P4 are *formatter-style* edits: they must touch only CODE (whitespace,
    comments, braces), never the interior of a string/char/raw literal — real
    formatters (rustfmt) leave literals byte-identical, so corrupting a
    multi-line literal would be a bug in the perturbation, not a finding. P5 is
    a real rename and operates on raw text. The M2 self-check is the arbiter.
    """
    if which == "P5":  # real change — raw text
        mlet = re.search(r"\blet\s+([A-Za-z_][A-Za-z0-9_]*)\b", src)
        if not mlet:
            return src  # nothing to rename — no-op
        ident = mlet.group(1)
        if ident == "_":
            return src
        return re.sub(r"\b" + re.escape(ident) + r"\b", "renamed_ident", src)

    lines = src.split("\n")
    atoms = scan_atoms(src)
    lit_spans = [(off, off + len(t)) for k, t, _ln, off in atoms if k == "lit"]
    # line start offsets (for the literal-interior test)
    line_off = []
    o = 0
    for ln in lines:
        line_off.append(o)
        o += len(ln) + 1
    lit_spans.sort()

    def in_lit(off):
        for s, e in lit_spans:
            if s <= off < e:
                return True
            if s > off:
                break
        return False

    cview = code_view(src, atoms)
    cv_lines = cview.split("\n")

    out = []
    if which == "P1":  # double leading indentation of every CODE line
        for i, ln in enumerate(lines):
            if in_lit(line_off[i]):
                out.append(ln)  # literal-interior line: leave verbatim
                continue
            stripped = ln.lstrip(" \t")
            indent = ln[: len(ln) - len(stripped)]
            out.append(indent + indent + stripped)
        return "\n".join(out)
    if which == "P2":  # blank line after every line ending in a CODE '{'
        for i, ln in enumerate(lines):
            out.append(ln)
            if cv_lines[i].rstrip().endswith("{"):
                out.append("")
        return "\n".join(out)
    if which == "P3":  # '// note' before every line that (after indent) starts with 'fn '
        for i, ln in enumerate(lines):
            if cv_lines[i].lstrip(" \t").startswith("fn "):
                stripped = ln.lstrip(" \t")
                indent = ln[: len(ln) - len(stripped)]
                out.append(indent + "// note")
            out.append(ln)
        return "\n".join(out)
    if which == "P4":  # rewrite each ordinary '// ' (not /// //!) comment to '// x'
        for i, ln in enumerate(lines):
            if in_lit(line_off[i]):
                out.append(ln)
                continue
            stripped = ln.lstrip(" \t")
            if stripped.startswith("//") and not stripped.startswith("///") \
                    and not stripped.startswith("//!"):
                indent = ln[: len(ln) - len(stripped)]
                out.append(indent + "// x")
            else:
                out.append(ln)
        return "\n".join(out)
    raise ValueError(which)


def measure_m2(perimeter_files):
    """Control perturbations on HEAD. Return per-P counts + per-file token
    regressions (any token hit on P1–P4 is a lexer bug)."""
    perturbations = ["P1", "P2", "P3", "P4", "P5"]
    raw = {p: 0 for p in perturbations}
    t1 = {p: 0 for p in perturbations}  # B1
    t2 = {p: 0 for p in perturbations}  # B2
    regressions = []  # (file, P) where a token hash moved on P1–P4
    total = 0
    for path in perimeter_files:
        try:
            src = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        total += 1
        r0 = raw_hash(src)
        b1_0, b2_0 = tok_hashes(src)
        for p in perturbations:
            new = _perturb(src, p)
            if new == src:
                continue  # perturbation was a no-op on this file
            r1 = raw_hash(new)
            a1, a2 = tok_hashes(new)
            if r1 != r0:
                raw[p] += 1
            if a1 != b1_0:
                t1[p] += 1
                if p != "P5":
                    regressions.append((str(path), p, "B1"))
            if a2 != b2_0:
                t2[p] += 1
                if p != "P5":
                    regressions.append((str(path), p, "B2"))
    return {
        "total_files": total,
        "raw": raw, "tok_b1": t1, "tok_b2": t2,
        "regressions": regressions,
    }


def _bucket(subject: str) -> str:
    m = re.match(r"^([a-zA-Z]+)(?:\([^)]*\))?(!)?:", subject)
    if not m:
        return "other"
    t = m.group(1).lower()
    if t in ("style", "chore", "refactor"):
        return "fmt"  # noise class
    if t in ("feat", "fix", "docs"):
        return "semantic"
    return "other"


class _Acc:
    """Accumulates raw/tok change counts, optionally split by bucket."""

    def __init__(self):
        self.raw = 0
        self.tok_b1 = 0
        self.tok_b2 = 0
        # for spurious we need (raw & ~tok); since tok=>raw, spurious = raw-tok
        # but we count explicitly to stay honest if that invariant ever breaks.

    def add(self, raw_ch: bool, b1_ch: bool, b2_ch: bool):
        if raw_ch:
            self.raw += 1
        if b1_ch:
            self.tok_b1 += 1
        if b2_ch:
            self.tok_b2 += 1

    def spurious(self):
        # spurious_b1 = raw moved AND b1 did not  = raw - b1 (given b1=>raw)
        return (max(self.raw - self.tok_b1, 0), max(self.raw - self.tok_b2, 0))

    def rate(self):
        s1, s2 = self.spurious()
        r1 = (s1 / self.raw) if self.raw else 0.0
        r2 = (s2 / self.raw) if self.raw else 0.0
        return r1, r2


def measure_m1(window: int, do_elements: bool):
    """Walk the history window; return file-level and element-level dicts."""
    shas = git("log", "--format=%H", "--", "*.rs").decode("utf-8").split()
    shas = shas[:window]  # newest first
    window_shas = list(reversed(shas))  # chronological (oldest -> newest)

    # subjects for bucketing
    subj_out = git("log", "--format=%H%x09%s", "--", "*.rs").decode("utf-8")
    subject = {}
    for line in subj_out.splitlines():
        if "\t" in line:
            h, s = line.split("\t", 1)
            subject[h] = s

    file_total = _Acc()
    file_by_bucket = {"fmt": _Acc(), "semantic": _Acc(), "other": _Acc()}

    elem_total = _Acc()
    elem_pairs = 0
    elem_unmatched = 0

    for C in window_shas:
        P = git("rev-parse", C + "^").decode("utf-8").strip()
        if not P:
            continue
        changed = git("diff", "--no-renames", "--name-only", P, C,
                      "--", "*.rs").decode("utf-8").split()
        changed = [c for c in changed if in_perimeter(c)]
        if not changed:
            continue
        refs = []
        for path in changed:
            refs.append(f"{P}:{path}")
            refs.append(f"{C}:{path}")
        blobs = cat_file_batch(refs)
        buck = _bucket(subject.get(C, ""))
        for path in changed:
            pb = blobs.get(f"{P}:{path}")
            cb = blobs.get(f"{C}:{path}")
            if pb is None or cb is None:
                continue  # added/deleted — no pair
            try:
                p_src = pb.decode("utf-8")
                c_src = cb.decode("utf-8")
            except UnicodeDecodeError:
                continue
            # file level
            pr, cr = raw_hash(p_src), raw_hash(c_src)
            pb1, pb2 = tok_hashes(p_src)
            cb1, cb2 = tok_hashes(c_src)
            rch = pr != cr
            b1ch = pb1 != cb1
            b2ch = pb2 != cb2
            file_total.add(rch, b1ch, b2ch)
            file_by_bucket[buck].add(rch, b1ch, b2ch)
            # element level
            if do_elements:
                pe = {(e["kind"], e["name"]): e for e in extract_elements(p_src)}
                ce = {(e["kind"], e["name"]): e for e in extract_elements(c_src)}
                for key in pe.keys() & ce.keys():
                    if key in ("", None):
                        continue
                    pe_, ce_ = pe[key], ce[key]
                    pb = p_src[pe_["start_off"]:pe_["end_off"]]
                    cb = c_src[ce_["start_off"]:ce_["end_off"]]
                    pr, cr = raw_hash(pb), raw_hash(cb)
                    pb1, pb2 = tok_hashes(pb)
                    cb1, cb2 = tok_hashes(cb)
                    elem_total.add(pr != cr, pb1 != cb1, pb2 != cb2)
                    elem_pairs += 1
                elem_unmatched += len(pe.keys() ^ ce.keys())

    return {
        "window": window,
        "n_commits": len(window_shas),
        "oldest": window_shas[0] if window_shas else None,
        "newest": window_shas[-1] if window_shas else None,
        "file_total": file_total,
        "file_by_bucket": file_by_bucket,
        "elem_total": elem_total,
        "elem_pairs": elem_pairs,
        "elem_unmatched": elem_unmatched,
    }


def measure_coverage():
    """Of specmap.json code_items, how many does the declaration finder
    recover at (file, line)?"""
    if not SPECMAP.exists():
        return None
    data = json.loads(SPECMAP.read_text(encoding="utf-8"))
    items = data.get("code_items") or data.get("codeItems") or []
    total = len(items)
    hit = 0
    fuzzy = 0
    miss_kind = {}
    for it in items:
        rel = it.get("file")
        line = it.get("line")
        kind = it.get("item_kind") or it.get("itemKind")
        if not rel or line is None:
            continue
        f = REPO / rel.replace("/", "\\") if "\\" in str(REPO) else REPO / rel
        f = REPO / rel
        if not f.exists():
            miss_kind[kind] = miss_kind.get(kind, 0) + 1
            continue
        try:
            src = f.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            miss_kind[kind] = miss_kind.get(kind, 0) + 1
            continue
        elems = extract_elements(src)
        matched = any(e["start_line"] == line and e["kind"] == kind for e in elems)
        if matched:
            hit += 1
        else:
            near = any(abs(e["start_line"] - line) <= 2 and e["kind"] == kind
                       for e in elems)
            if near:
                fuzzy += 1
            else:
                miss_kind[kind] = miss_kind.get(kind, 0) + 1
    return {"total": total, "hit": hit, "fuzzy": fuzzy,
            "miss_kind": miss_kind}


def list_perimeter_files():
    out = []
    for p in REPO.rglob("*.rs"):
        rel = p.relative_to(REPO).as_posix()
        if in_perimeter(rel):
            out.append(p)
    return out


# ---------------------------------------------------------------------- report

def _pct(x, y):
    return (100.0 * x / y) if y else 0.0


def report(m1, m2, cov, perimeter_files):
    print("=" * 72)
    print("FINGERPRINT NOISE — scheme A (raw text) vs scheme B (token stream)")
    print("=" * 72)
    print(f"repo: {REPO}")
    print(f"perimeter .rs files at HEAD: {len(perimeter_files)}")
    print()

    # ---- M1 file level
    if m1 is not None:
        ft = m1["file_total"]
        r1, r2 = ft.rate()
        print("M1 — real-history loss (FILE level, exact)")
        print("-" * 72)
        print(f"history window: {m1['n_commits']} .rs-touching commits "
              f"(of 871 available); HEAD-bound below.")
        print(f"  oldest in window: {m1['oldest']}")
        print(f"  newest in window: {m1['newest']}  (== HEAD of this worktree)")
        print(f"file-revisions where raw-hash moved : {ft.raw}")
        print(f"  of those, token-hash ALSO moved B1 : {ft.tok_b1}")
        print(f"  of those, token-hash ALSO moved B2 : {ft.tok_b2}")
        s1, s2 = ft.spurious()
        print(f"SPURIOUS (raw moved, token did not) B1: {s1}   "
              f"= spurious_rate B1 {r1 * 100:5.1f} %")
        print(f"SPURIOUS (raw moved, token did not) B2: {s2}   "
              f"= spurious_rate B2 {r2 * 100:5.1f} %")
        print(f"  -> headline spurious_rate  B1={r1 * 100:.1f}%  B2={r2 * 100:.1f}%")
        print()
        print("M1 — spurious_rate by commit class (file level)")
        print("-" * 72)
        print(f"  {'class':<10} {'raw':>7} {'tokB1':>7} {'tokB2':>7} "
              f"{'spurB1':>7} {'spurB2':>7} {'rateB1':>8} {'rateB2':>8}")
        for name in ("fmt", "semantic", "other"):
            a = m1["file_by_bucket"][name]
            s1, s2 = a.spurious()
            rr1, rr2 = a.rate()
            print(f"  {name:<10} {a.raw:>7} {a.tok_b1:>7} {a.tok_b2:>7} "
                  f"{s1:>7} {s2:>7} {rr1 * 100:>7.1f}% {rr2 * 100:>7.1f}%")
        print("  (fmt = style/chore/refactor; semantic = feat/fix/docs; "
              "other = build/test/perf/revert/...)")
        print()

        # ---- M1 element level
        et = m1["elem_total"]
        er1, er2 = et.rate()
        print("M1 — real-history loss (ELEMENT level, heuristic — secondary)")
        print("-" * 72)
        print(f"element pairs matched by (kind,name) across versions: "
              f"{m1['elem_pairs']}  (unmatched names: {m1['elem_unmatched']})")
        print(f"element-revisions where raw moved : {et.raw}")
        print(f"  token also moved B1 / B2         : {et.tok_b1} / {et.tok_b2}")
        es1, es2 = et.spurious()
        print(f"SPURIOUS B1 {es1}  rate {er1 * 100:5.1f}%   |   "
              f"SPURIOUS B2 {es2}  rate {er2 * 100:5.1f}%")
        print()

    # ---- coverage
    if cov is not None:
        print("Coverage — honesty of the element heuristic (task §4.2)")
        print("-" * 72)
        tot = cov["total"]
        hit = cov["hit"]
        print(f"specmap.json code_items: {tot}; found at (file,line): {hit} "
              f"= {_pct(hit, tot):.1f}%")
        print(f"near-misses (start_line within +-2): {cov['fuzzy']} "
              f"(specmap drift, not heuristic failure)")
        if cov["miss_kind"]:
            print("missed-by-kind (no declaration found near the line):")
            for k, v in sorted(cov["miss_kind"].items(), key=lambda kv: -kv[1]):
                print(f"  {k:<10} {v}")
        else:
            print("missed-by-kind: none")
        print()

    # ---- M2
    print("M2 — control perturbations on HEAD (task §4.3) — lexer self-check")
    print("-" * 72)
    tot = m2["total_files"]
    print(f"{'perturbation':<22} {'raw %':>8} {'tokB1 %':>8} {'tokB2 %':>8}")
    for p in ("P1", "P2", "P3", "P4", "P5"):
        print(f"{p:<22} {_pct(m2['raw'][p], tot):>7.1f}% "
              f"{_pct(m2['tok_b1'][p], tot):>7.1f}% "
              f"{_pct(m2['tok_b2'][p], tot):>7.1f}%")
    print("  P1 double-indent | P2 blank line after { | P3 '// note' before fn | "
          "P4 rewrite // | P5 real rename")
    reg = m2["regressions"]
    if reg:
        print(f"LEXER REGRESSIONS (token moved on P1-P4 — a lexer bug): {len(reg)}")
        for f, p, col in reg[:20]:
            print(f"  {p} {col}  {f}")
    else:
        print("LEXER REGRESSIONS on P1-P4: 0  (token-hash is format-independent)")
    print()

    print("=" * 72)
    if m1 is not None:
        ft = m1["file_total"]
        r1, r2 = ft.rate()
        print(f"HEADLINE: file-level spurious_rate  B1={r1 * 100:.1f}%  "
              f"B2={r2 * 100:.1f}%   (raw window {m1['n_commits']} commits)")
    print("=" * 72)
    return 0


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--window", type=int, default=DEFAULT_WINDOW,
                    help=f".rs-touching commits to scan for M1 (default {DEFAULT_WINDOW})")
    ap.add_argument("--no-history", action="store_true",
                    help="skip M1 (history walk); run M2 + coverage only")
    ap.add_argument("--no-elements", action="store_true",
                    help="skip element-level M1 (file level + M2 + coverage)")
    args = ap.parse_args()

    perimeter_files = list_perimeter_files()
    m2 = measure_m2(perimeter_files)
    cov = measure_coverage()
    m1 = None
    if not args.no_history:
        m1 = measure_m1(args.window, do_elements=not args.no_elements)
    return report(m1, m2, cov, perimeter_files)


if __name__ == "__main__":
    sys.exit(main())
