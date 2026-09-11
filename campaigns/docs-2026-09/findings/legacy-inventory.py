#!/usr/bin/env python3
"""legacy-inventory.py -- P0-C4 (spike A0.13).

Mechanical inventory of assertions made by the legacy docs/**/*.md tree in
the vibevm-docs worktree, each labelled against the live target/debug/vibe.exe
binary and the crates/vibe-core manifest source.

Usage:
    python legacy-inventory.py [--tree PATH] [--check]

Writes (next to this script):
    legacy-inventory.md

Both a bare run and --check do the same work: scan, verify, write the table,
print a one-line machine-readable summary, exit 0. --check exists only to
satisfy the packet's self-check step name; there is no separate mode.

Assertion kinds extracted per file:
  cmd     a `vibe ...` invocation, from a fenced code block or inline code,
          where the line/span starts with "vibe ".
  path    a vibevm/..., crates/..., vibedeps/..., ~/.vibe/... path, or a bare
          *.toml / *.lock filename, found anywhere in the line.
  section a TOML `[table]` / `[[array-table]]` header, or a snake_case
          manifest field name, found in a fenced code block or inline code.
  flag    a `--flag` token, found anywhere in the line.

Labels: zhivo (alive) | ustarelo (outdated) | neizvestno (unknown) -- written
in Russian in the output table per the packet's requested column values
(zhivo/ustarelo/neizvestno below are the storage constants; the strings
written to the table are the actual Cyrillic words).

Verification methods (see WORKER-REPORT-P0-C4.md for the full rationale):
  cmd     - tokenize after "vibe"; collect leading tokens that are neither a
            flag (-...) nor a placeholder ([...], <...>, containing "...");
            walk that path one token at a time via `vibe <path> --help`,
            stopping as soon as a level's own Usage line has no "<COMMAND>"
            (i.e. it is a leaf, so further words are positional arguments,
            not more subcommand path); a walk that fails partway is
            "ustarelo"; an immediate failure on the first token is
            "ustarelo"; a bare leading flag (e.g. `vibe --version`) is
            checked against the top-level --help Options text instead.
  path    - vibevm/... and crates/... and vibedeps/... resolved against the
            tree root with Path.exists(); ~/.vibe/... resolved against this
            machine's real home directory (the only mechanical proxy
            available for a runtime path -- flagged "neizvestno" when
            absent, not "ustarelo", since absence on this machine does not
            mean the path is unsupported); a bare filename (no "/") is
            searched for anywhere under the tree (pruning .git/target/
            vibedeps/node_modules/.cargo).
  section - a `[table]` / `[[array]]` header or snake_case field is searched
            for verbatim across crates/vibe-core/src/manifest/**/*.rs
            (recursive -- see the deviation note in the worker report), then
            across vibevm/vibepacks/**/vibe.toml.
  flag    - checked against the union of every `--flag` token appearing in
            the live CLI's own --help output, crawled two levels deep
            (top-level, then every group's children).
"""
from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

DEFAULT_TREE = r"C:\Users\olegc\git\v\vibevm-docs"
SCRIPT_DIR = Path(__file__).resolve().parent  # .../vibe-docs-vision/findings

PRUNE_DIRS = {".git", "target", "vibedeps", "node_modules", ".cargo"}

ZHIVO, USTARELO, NEIZVESTNO = "\u0436\u0438\u0432\u043e", "\u0443\u0441\u0442\u0430\u0440\u0435\u043b\u043e", "\u043d\u0435\u0438\u0437\u0432\u0435\u0441\u0442\u043d\u043e"
# zhivo = живо, ustarelo = устарело, neizvestno = неизвестно

PATH_PATTERNS = [
    re.compile(r"vibevm/[A-Za-z0-9_./\-]+"),
    re.compile(r"crates/[A-Za-z0-9_./\-]+"),
    re.compile(r"vibedeps/[A-Za-z0-9_./\-]*"),
    re.compile(r"~/\.vibe/[A-Za-z0-9_./\-]*"),
    re.compile(r"\b[A-Za-z0-9_.\-]*\.(?:toml|lock)\b"),
]

FLAG_PATTERN = re.compile(r"--[a-z][a-z0-9]*(?:-[a-z0-9]+)*")

BRACKET_SECTION = re.compile(r"\[\[?[A-Za-z_][A-Za-z0-9_.\-]*\]\]?(?!\()")
ALLCAPS_ID = re.compile(r"^[A-Z][A-Z0-9-]*$")  # PROP-002, D-17, R-12, E-BUG-001 -> not a section
SNAKE_FIELD_FULL = re.compile(r"^[a-z][a-z0-9]*(?:_[a-z0-9]+)+$")
SNAKE_FIELD_KV = re.compile(r"\b([a-z][a-z0-9]*(?:_[a-z0-9]+)+)\b(?=\s*=)")

INLINE_CODE = re.compile(r"`([^`\n]+)`")
HEADING = re.compile(r"^(#{1,6})\s+(.*\S)\s*$")
FENCE = re.compile(r"^\s*```")
LEADS_WITH_DIGIT = re.compile(r"^\d")

TRAILING_PUNCT = ".,;:)]}'\"`"
LEADING_PUNCT = "'\"`("


@dataclass
class Assertion:
    file: str
    heading: str
    line: int
    kind: str  # cmd | path | section | flag
    text: str


# ---------------------------------------------------------------------------
# Extraction
# ---------------------------------------------------------------------------

def strip_prompt(line: str) -> str:
    s = line.strip()
    if s.startswith("$ "):
        s = s[2:].strip()
    elif s.startswith("> "):
        s = s[2:].strip()
    return s


def clean_path_token(tok: str) -> str:
    tok = tok.strip()
    while tok and tok[-1] in TRAILING_PUNCT:
        tok = tok[:-1]
    while tok and tok[0] in LEADING_PUNCT:
        tok = tok[1:]
    return tok


def looks_like_cmd_candidate(rest: str) -> bool:
    """rest = text right after 'vibe ' (already stripped). Reject sample
    OUTPUT text like 'vibe 0.1.0-dev' / 'vibe 1.0.0' that happens to match
    the naive 'starts with vibe ' rule but is not an invocation."""
    first = rest.split(" ", 1)[0] if rest else ""
    return not LEADS_WITH_DIGIT.match(first)


def extract_from_file(path: Path, rel: str) -> list[Assertion]:
    text = path.read_text(encoding="utf-8", errors="replace")
    lines = text.split("\n")
    out: list[Assertion] = []
    heading = "(\u0431\u0435\u0437 \u0437\u0430\u0433\u043e\u043b\u043e\u0432\u043a\u0430)"  # (без заголовка)
    in_code = False
    for i, raw in enumerate(lines, start=1):
        if FENCE.match(raw):
            in_code = not in_code
            continue
        m = HEADING.match(raw)
        if m and not in_code:
            heading = m.group(2)

        # --- cmd: fenced code block lines ---
        if in_code:
            s = strip_prompt(raw)
            if s.startswith("vibe "):
                rest = s[len("vibe "):].strip()
                if looks_like_cmd_candidate(rest):
                    out.append(Assertion(rel, heading, i, "cmd", s[:200]))
        else:
            # --- cmd: inline code spans ---
            for span in INLINE_CODE.findall(raw):
                sp = span.strip()
                if sp.startswith("vibe "):
                    rest = sp[len("vibe "):].strip()
                    if looks_like_cmd_candidate(rest):
                        out.append(Assertion(rel, heading, i, "cmd", sp[:200]))

        # --- section/field: code-block lines OR inline-code spans only ---
        code_texts = [raw] if in_code else INLINE_CODE.findall(raw)
        for ct in code_texts:
            ctt = ct.strip()
            for bm in BRACKET_SECTION.finditer(ctt):
                bare = bm.group(0).strip("[]")
                if not ALLCAPS_ID.match(bare):
                    out.append(Assertion(rel, heading, i, "section", bm.group(0)))
            if SNAKE_FIELD_FULL.match(ctt):
                out.append(Assertion(rel, heading, i, "section", ctt))
            else:
                for fm in SNAKE_FIELD_KV.finditer(ctt):
                    out.append(Assertion(rel, heading, i, "section", fm.group(1)))

        # --- path: anywhere in the raw line ---
        for pat in PATH_PATTERNS:
            for pm in pat.finditer(raw):
                tok = clean_path_token(pm.group(0))
                if len(tok) >= 5:
                    out.append(Assertion(rel, heading, i, "path", tok))

        # --- flag: anywhere in the raw line ---
        for fm in FLAG_PATTERN.finditer(raw):
            out.append(Assertion(rel, heading, i, "flag", fm.group(0)))

    return out


def dedup_per_file(items: list[Assertion]) -> list[Assertion]:
    seen: dict[tuple, Assertion] = {}
    for a in items:
        key = (a.file, a.kind, a.text)
        if key not in seen:
            seen[key] = a
    return list(seen.values())


# ---------------------------------------------------------------------------
# vibe.exe live CLI verification
# ---------------------------------------------------------------------------

def looks_like_placeholder(tok: str) -> bool:
    if tok.startswith(("-", "[", "<")):
        return True
    if "..." in tok or "…" in tok:
        return True
    if tok.endswith(":"):
        # output-text label, e.g. the "add:" in a rendered
        # "vibe registry add: `private` registered ..." log line, not a
        # real subcommand token.
        return True
    return False


class VibeCli:
    def __init__(self, exe: Path):
        self.exe = exe
        self.cache: dict[tuple, tuple[int, str]] = {}

    def help(self, path_tokens: tuple) -> tuple:
        if path_tokens in self.cache:
            return self.cache[path_tokens]
        args = [str(self.exe), *path_tokens, "--help"]
        try:
            proc = subprocess.run(args, capture_output=True, text=True, timeout=20)
            result = (proc.returncode, (proc.stdout or "") + (proc.stderr or ""))
        except Exception as exc:  # pragma: no cover - defensive
            result = (-1, f"<exec error: {exc}>")
        self.cache[path_tokens] = result
        return result

    @staticmethod
    def is_group(help_text: str) -> bool:
        for line in help_text.splitlines():
            if line.strip().startswith("Usage:"):
                return "<COMMAND>" in line
        return False


def parse_commands_section(help_text: str) -> list[str]:
    names = []
    in_section = False
    for line in help_text.splitlines():
        if line.strip() == "Commands:":
            in_section = True
            continue
        if in_section:
            if line.strip() == "":
                continue
            if not line.startswith("  "):
                break
            m = re.match(r"^\s{2}([a-z][a-z0-9-]*)\s", line)
            if m:
                names.append(m.group(1))
    return [n for n in names if n != "help"]


def crawl_cli_tree(cli: VibeCli) -> dict:
    rc, top = cli.help(())
    tree = {(): top}
    for c in parse_commands_section(top):
        rc1, text1 = cli.help((c,))
        if rc1 == 0:
            tree[(c,)] = text1
            if cli.is_group(text1):
                for c2 in parse_commands_section(text1):
                    rc2, text2 = cli.help((c, c2))
                    if rc2 == 0:
                        tree[(c, c2)] = text2
    return tree


def resolve_vibe_command(cli: VibeCli, statement: str) -> tuple:
    body = statement.split("|", 1)[0].split("&&", 1)[0].strip()
    tokens = body.split()
    if not tokens or tokens[0] not in ("vibe", "vibe.exe"):
        return NEIZVESTNO, "\u043d\u0435 \u0443\u0434\u0430\u043b\u043e\u0441\u044c \u0440\u0430\u0437\u043e\u0431\u0440\u0430\u0442\u044c \u043a\u043e\u043c\u0430\u043d\u0434\u0443"
    tokens = tokens[1:]

    path_tokens: list[str] = []
    for t in tokens:
        if looks_like_placeholder(t):
            break
        path_tokens.append(t)

    if not path_tokens:
        flag = tokens[0] if tokens else ""
        rc, top_help = cli.help(())
        if flag.startswith("-") and flag in top_help:
            return ZHIVO, f"\u0433\u043b\u043e\u0431\u0430\u043b\u044c\u043d\u044b\u0439 \u0444\u043b\u0430\u0433 {flag} \u043d\u0430\u0439\u0434\u0435\u043d \u0432 `vibe --help`"
        return NEIZVESTNO, "\u043d\u0435\u0442 \u043f\u043e\u0434\u043a\u043e\u043c\u0430\u043d\u0434\u044b, \u0444\u043b\u0430\u0433 \u043d\u0435 \u043e\u043f\u043e\u0437\u043d\u0430\u043d \u0432 `vibe --help`"

    path: list[str] = []
    for tok in path_tokens:
        trial = tuple(path + [tok])
        rc, text = cli.help(trial)
        if rc != 0:
            joined = " ".join(path) if path else "(\u043a\u043e\u0440\u0435\u043d\u044c)"
            return USTARELO, (
                f"vibe.exe {' '.join(trial)} --help -> exit {rc}; "
                f"\u0440\u0435\u0437\u043e\u043b\u0432\u0438\u0442\u0441\u044f \u0442\u043e\u043b\u044c\u043a\u043e \u0434\u043e '{joined}'"
            )
        path = path + [tok]
        if not cli.is_group(text):
            break
    return ZHIVO, f"vibe.exe {' '.join(path)} --help -> exit 0"


# ---------------------------------------------------------------------------
# path / manifest-field verification
# ---------------------------------------------------------------------------

_bare_filename_cache: dict[str, bool] = {}


def _bare_filename_exists(tree_root: Path, name: str) -> bool:
    if name in _bare_filename_cache:
        return _bare_filename_cache[name]
    found = False
    for dirpath, dirnames, filenames in os.walk(tree_root):
        dirnames[:] = [d for d in dirnames if d not in PRUNE_DIRS]
        if name in filenames:
            found = True
            break
    _bare_filename_cache[name] = found
    return found


def verify_path(tree_root: Path, home: Path, tok: str) -> tuple:
    if tok.startswith("vibedeps/") or tok == "vibedeps":
        return NEIZVESTNO, (
            "\u0433\u0435\u043d\u0435\u0440\u0438\u0440\u0443\u0435\u043c\u044b\u0439 \u043a\u0430\u0442\u0430\u043b\u043e\u0433 (\u043c\u0430\u0442\u0435\u0440\u0438\u0430\u043b\u0438\u0437\u0443\u0435\u0442\u0441\u044f `vibe install`), "
            "\u043c\u0435\u0445\u0430\u043d\u0438\u0447\u0435\u0441\u043a\u0430\u044f \u043f\u0440\u043e\u0432\u0435\u0440\u043a\u0430 \u043f\u043e \u0434\u0435\u0440\u0435\u0432\u0443 \u043d\u0435\u0438\u043d\u0444\u043e\u0440\u043c\u0430\u0442\u0438\u0432\u043d\u0430 \u0434\u043b\u044f \u043d\u0435\u043f\u0440\u043e\u0438\u043d\u0441\u0442\u0430\u043b\u043b\u0438\u0440\u043e\u0432\u0430\u043d\u043d\u043e\u0433\u043e \u0447\u0435\u043a\u0430\u0443\u0442\u0430"
        )

    if tok.startswith("~/.vibe"):
        if tok.endswith(".token"):
            return NEIZVESTNO, (
                "\u0441\u0435\u043a\u0440\u0435\u0442-\u043f\u043e\u0434\u043e\u0431\u043d\u044b\u0439 \u043f\u0443\u0442\u044c (R-12) \u2014 \u0441\u0443\u0449\u0435\u0441\u0442\u0432\u043e\u0432\u0430\u043d\u0438\u0435 \u043d\u0430 \u0434\u0438\u0441\u043a\u0435 \u043d\u0435 \u043f\u0440\u043e\u0432\u0435\u0440\u044f\u043b\u043e\u0441\u044c \u043d\u0430\u043c\u0435\u0440\u0435\u043d\u043d\u043e"
            )
        rest = tok[len("~/"):]
        candidate = home / Path(rest)
        if candidate.exists():
            return ZHIVO, f"\u043d\u0430\u0439\u0434\u0435\u043d \u043d\u0430 \u0434\u0438\u0441\u043a\u0435 (\u044d\u0442\u0430 \u043c\u0430\u0448\u0438\u043d\u0430): {candidate}"
        return NEIZVESTNO, f"\u0440\u0430\u043d\u0442\u0430\u0439\u043c-\u043f\u0443\u0442\u044c, \u043d\u0435 \u043d\u0430\u0439\u0434\u0435\u043d \u043d\u0430 \u044d\u0442\u043e\u0439 \u043c\u0430\u0448\u0438\u043d\u0435: {tok}"

    if "/" not in tok:
        if _bare_filename_exists(tree_root, tok):
            return ZHIVO, f"\u043d\u0430\u0439\u0434\u0435\u043d \u0432 \u0434\u0435\u0440\u0435\u0432\u0435 (\u0438\u043c\u044f \u0444\u0430\u0439\u043b\u0430): {tok}"
        return USTARELO, f"\u043d\u0438 \u043e\u0434\u0438\u043d \u0444\u0430\u0439\u043b \u0441 \u0438\u043c\u0435\u043d\u0435\u043c {tok} \u043d\u0435 \u043d\u0430\u0439\u0434\u0435\u043d \u0432 \u0434\u0435\u0440\u0435\u0432\u0435"

    candidate = tree_root / tok
    if candidate.exists():
        return ZHIVO, f"\u043f\u0443\u0442\u044c \u0441\u0443\u0449\u0435\u0441\u0442\u0432\u0443\u0435\u0442: {tok}"
    return USTARELO, f"\u043f\u0443\u0442\u044c \u043d\u0435 \u0441\u0443\u0449\u0435\u0441\u0442\u0432\u0443\u0435\u0442: {tok}"


def load_corpus(root: Path, glob_pattern: str) -> list:
    out = []
    if not root.exists():
        return out
    for f in sorted(root.glob(glob_pattern)):
        if not f.is_file():
            continue
        try:
            out.append((f.relative_to(root).as_posix(), f.read_text(encoding="utf-8", errors="ignore")))
        except OSError:
            continue
    return out


def _grep_corpus(corpus: list, needle: str) -> str | None:
    for relpath, content in corpus:
        idx = content.find(needle)
        if idx != -1:
            line_no = content.count("\n", 0, idx) + 1
            return f"{relpath}:{line_no}"
    return None


def verify_manifest_item(manifest_corpus: list, toml_corpus: list, tok: str) -> tuple:
    if tok.startswith("["):
        bare = tok.strip("[]")
        hit = _grep_corpus(manifest_corpus, tok) or _grep_corpus(manifest_corpus, bare)
        if hit:
            return ZHIVO, f"\u043d\u0430\u0439\u0434\u0435\u043d\u043e \u0432 crates/vibe-core/src/manifest/{hit}"
        hit2 = _grep_corpus(toml_corpus, tok)
        if hit2:
            return ZHIVO, f"\u043d\u0430\u0439\u0434\u0435\u043d\u043e \u0432 vibevm/vibepacks/{hit2}"
        return USTARELO, "\u043d\u0435 \u043d\u0430\u0439\u0434\u0435\u043d\u043e \u043d\u0438 \u0432 manifest source, \u043d\u0438 \u0432 \u043f\u0440\u0438\u043c\u0435\u0440\u0430\u0445 vibe.toml"

    hit = _grep_corpus(manifest_corpus, tok)
    if hit:
        return ZHIVO, f"\u043d\u0430\u0439\u0434\u0435\u043d\u043e \u0432 crates/vibe-core/src/manifest/{hit}"
    hit2 = _grep_corpus(toml_corpus, tok + " ") or _grep_corpus(toml_corpus, tok + "=")
    if hit2:
        return ZHIVO, f"\u043d\u0430\u0439\u0434\u0435\u043d\u043e \u0432 vibevm/vibepacks/{hit2}"
    return USTARELO, "\u043f\u043e\u043b\u0435 \u043d\u0435 \u043d\u0430\u0439\u0434\u0435\u043d\u043e \u043d\u0438 \u0432 manifest source, \u043d\u0438 \u0432 \u043f\u0440\u0438\u043c\u0435\u0440\u0430\u0445 vibe.toml"


def verify_flag(flag_universe: set, tok: str) -> tuple:
    if tok in flag_universe:
        return ZHIVO, "\u043d\u0430\u0439\u0434\u0435\u043d \u0432 \u0441\u043e\u0431\u0440\u0430\u043d\u043d\u043e\u043c \u043a\u043e\u0440\u043f\u0443\u0441\u0435 `--help`"
    return USTARELO, "\u043d\u0435 \u043d\u0430\u0439\u0434\u0435\u043d \u043d\u0438 \u0432 \u043e\u0434\u043d\u043e\u043c \u0441\u043e\u0431\u0440\u0430\u043d\u043d\u043e\u043c `--help`"


def build_flag_universe(tree: dict) -> set:
    universe: set = set()
    for text in tree.values():
        for m in FLAG_PATTERN.finditer(text):
            universe.add(m.group(0))
    return universe


# ---------------------------------------------------------------------------
# docs/** file map (part 4 of the packet)
# ---------------------------------------------------------------------------

def first_heading_and_blurb(path: Path) -> tuple:
    text = path.read_text(encoding="utf-8", errors="replace")
    lines = text.split("\n")
    heading = "(\u043d\u0435\u0442 \u0437\u0430\u0433\u043e\u043b\u043e\u0432\u043a\u0430)"
    blurb = ""
    h_idx = None
    for i, raw in enumerate(lines):
        m = HEADING.match(raw)
        if m:
            heading = m.group(2)
            h_idx = i
            break
    if h_idx is not None:
        in_code = False
        for raw in lines[h_idx + 1:]:
            if FENCE.match(raw):
                in_code = not in_code
                continue
            s = raw.strip()
            if in_code or not s or s.startswith("#") or set(s) <= {"-"} or s.startswith("|"):
                continue
            blurb = s
            break
    if len(blurb) > 200:
        blurb = blurb[:197] + "..."
    return heading, blurb


def md_escape(s: str) -> str:
    return s.replace("|", "\\|").replace("\n", " ")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--tree", default=DEFAULT_TREE)
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()

    tree_root = Path(args.tree).resolve()
    docs_root = tree_root / "docs"
    exe = tree_root / "target" / "debug" / "vibe.exe"
    home = Path.home()

    if not exe.exists():
        print(f"ERROR: binary not found: {exe}", file=sys.stderr)
        return 1
    if not docs_root.exists():
        print(f"ERROR: docs/ not found under {tree_root}", file=sys.stderr)
        return 1

    print("PROGRESS: scanning docs/**/*.md", flush=True)
    md_files = sorted(docs_root.rglob("*.md"))
    all_assertions: list[Assertion] = []
    for f in md_files:
        rel = "docs/" + f.relative_to(docs_root).as_posix()
        items = extract_from_file(f, rel)
        all_assertions.extend(dedup_per_file(items))

    print("PROGRESS: crawling vibe.exe --help command tree", flush=True)
    cli = VibeCli(exe)
    cli_tree = crawl_cli_tree(cli)
    flag_universe = build_flag_universe(cli_tree)

    print("PROGRESS: loading manifest source and vibe.toml corpus", flush=True)
    manifest_root = tree_root / "crates" / "vibe-core" / "src" / "manifest"
    manifest_corpus = load_corpus(manifest_root, "**/*.rs")
    toml_corpus = load_corpus(tree_root / "vibevm" / "vibepacks", "**/vibe.toml")

    print(f"PROGRESS: verifying {len(all_assertions)} assertions", flush=True)
    verify_cache: dict[tuple, tuple] = {}

    def verify(a: Assertion) -> tuple:
        key = (a.kind, a.text)
        if key in verify_cache:
            return verify_cache[key]
        if a.kind == "cmd":
            res = resolve_vibe_command(cli, a.text)
        elif a.kind == "path":
            res = verify_path(tree_root, home, a.text)
        elif a.kind == "section":
            res = verify_manifest_item(manifest_corpus, toml_corpus, a.text)
        elif a.kind == "flag":
            res = verify_flag(flag_universe, a.text)
        else:
            res = (NEIZVESTNO, "\u043d\u0435\u0438\u0437\u0432\u0435\u0441\u0442\u043d\u044b\u0439 \u0442\u0438\u043f \u0443\u0442\u0432\u0435\u0440\u0436\u0434\u0435\u043d\u0438\u044f")
        verify_cache[key] = res
        return res

    rows = [(a, *verify(a)) for a in all_assertions]

    total = len(rows)
    counts = {ZHIVO: 0, USTARELO: 0, NEIZVESTNO: 0}
    for _, label, _ in rows:
        counts[label] = counts.get(label, 0) + 1
    pct = {k: (100.0 * v / total if total else 0.0) for k, v in counts.items()}
    files_in_table = {a.file for a, _, _ in rows}
    unknown_ok = pct[NEIZVESTNO] < 20.0

    print("PROGRESS: writing legacy-inventory.md", flush=True)
    out_path = SCRIPT_DIR / "legacy-inventory.md"
    L = []
    L.append("# \u0418\u043d\u0432\u0435\u043d\u0442\u0430\u0440\u044c \u0443\u0442\u0432\u0435\u0440\u0436\u0434\u0435\u043d\u0438\u0439 legacy-\u0434\u043e\u043a\u0443\u043c\u0435\u043d\u0442\u0430\u0446\u0438\u0438 (P0-C4 / \u0441\u043f\u0430\u0439\u043a A0.13)")
    L.append("")
    L.append(f"\u0414\u0435\u0440\u0435\u0432\u043e: `{tree_root}`. \u0411\u0438\u043d\u0430\u0440\u043d\u0438\u043a: `target/debug/vibe.exe` (`vibe --version` = см. WORKER-REPORT-P0-C4.md).")
    L.append("")
    L.append("\u0422\u0438\u043f\u044b: `cmd` \u2014 \u043a\u043e\u043c\u0430\u043d\u0434\u0430 `vibe \u2026`; `path` \u2014 \u043f\u0443\u0442\u044c \u0444\u0430\u0439\u043b\u0430/\u043a\u0430\u0442\u0430\u043b\u043e\u0433\u0430; "
             "`section` \u2014 \u0441\u0435\u043a\u0446\u0438\u044f/\u043f\u043e\u043b\u0435 \u043c\u0430\u043d\u0438\u0444\u0435\u0441\u0442\u0430; `flag` \u2014 \u0444\u043b\u0430\u0433 CLI \u0438\u0437 \u0442\u0435\u043a\u0441\u0442\u0430.")
    L.append("")
    L.append("| файл | заголовок раздела | утверждение | тип | метка | как проверено |")
    L.append("| --- | --- | --- | --- | --- | --- |")
    for a, label, how in rows:
        L.append("| " + " | ".join(md_escape(x) for x in (a.file, a.heading, a.text, a.kind, label, how)) + " |")
    L.append("")
    L.append("## \u0421\u0432\u043e\u0434\u043a\u0430")
    L.append("")
    L.append(f"- \u0424\u0430\u0439\u043b\u043e\u0432 \u043f\u0440\u043e\u0441\u043a\u0430\u043d\u0438\u0440\u043e\u0432\u0430\u043d\u043e: {len(md_files)}")
    L.append(f"- \u0424\u0430\u0439\u043b\u043e\u0432, \u0434\u0430\u0432\u0448\u0438\u0445 \u0445\u043e\u0442\u044f \u0431\u044b \u043e\u0434\u043d\u043e \u0443\u0442\u0432\u0435\u0440\u0436\u0434\u0435\u043d\u0438\u0435: {len(files_in_table)}")
    L.append(f"- \u0423\u0442\u0432\u0435\u0440\u0436\u0434\u0435\u043d\u0438\u0439 \u0432\u0441\u0435\u0433\u043e (\u043f\u043e\u0441\u043b\u0435 dedup \u043f\u043e \u0444\u0430\u0439\u043b\u0443): {total}")
    for k in (ZHIVO, USTARELO, NEIZVESTNO):
        L.append(f"- {k}: {counts[k]} ({pct[k]:.1f}%)")
    L.append("")
    below_word = "\u043d\u0438\u0436\u0435" if unknown_ok else "\u041d\u0415 \u043d\u0438\u0436\u0435"
    L.append(f"\u0414\u043e\u043b\u044f `{NEIZVESTNO}` {below_word} 20%: {pct[NEIZVESTNO]:.1f}%.")
    L.append("")
    L.append("## \u041a\u0430\u0440\u0442\u0430 \u0444\u0430\u0439\u043b\u043e\u0432 docs/** (\u043f\u0435\u0440\u0432\u044b\u0439 \u0437\u0430\u0433\u043e\u043b\u043e\u0432\u043e\u043a \u0438 \u0441\u043e\u0434\u0435\u0440\u0436\u0430\u043d\u0438\u0435)")
    L.append("")
    L.append("| файл | первый заголовок | о чём (авто) |")
    L.append("| --- | --- | --- |")
    for f in md_files:
        rel = "docs/" + f.relative_to(docs_root).as_posix()
        heading, blurb = first_heading_and_blurb(f)
        L.append(f"| {md_escape(rel)} | {md_escape(heading)} | {md_escape(blurb)} |")

    out_path.write_text("\n".join(L) + "\n", encoding="utf-8", newline="\n")

    summary = (
        f"files={len(md_files)} assertions={total} "
        f"zhivo={counts[ZHIVO]} ustarelo={counts[USTARELO]} neizvestno={counts[NEIZVESTNO]} "
        f"neizvestno_pct={pct[NEIZVESTNO]:.1f} unknown_under_20={unknown_ok}"
    )
    print(summary)
    print(f"OK: written {out_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
