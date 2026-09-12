#!/usr/bin/env python3
"""A3.13 / packet P3-M5 — mark documentation obligations in nine host PROPs.

The only edit this script performs is inserting the literal string

    ` action="continue" actionstage="doc" audience="<audiences>"`

into the opening tag of a named fact, immediately after its `status="…"`
attribute.  No other byte of any file changes: not the text, not the line
breaks, not the indentation, not the order of facts, not an anchor name.

The script refuses to do anything at all unless every planned anchor is
found exactly once, is a fact, and carries no `action=` yet; after the
insertions it re-parses every touched file with `xml.etree` and restores
the originals on any failure.

Run from the repository root:  python campaigns/docs-2026-09/findings/P3-M5-mark.py
"""

from __future__ import annotations

import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

COMMON = Path("vibevm/vibespecs/common")

# (file, anchor, audiences, why — <= 12 words)
PLAN: list[tuple[str, str, str, str]] = [
    # ---- PROP-018 — agentic and standalone modes (MVP shipped) -------------
    ("PROP-018-agentic-standalone-modes.xml", "MODE-INFERRED", "user,agent",
     "Mode is inferred per operation; there is no mode flag"),
    ("PROP-018-agentic-standalone-modes.xml", "REACH-STANDALONE-NO-ENGINE", "user",
     "The loud refusal a user meets when running without an agent"),
    ("PROP-018-agentic-standalone-modes.xml", "SKILL-SECTION-NOT-KIND", "author",
     "Skills are declared in a manifest section, not a package kind"),
    ("PROP-018-agentic-standalone-modes.xml", "SKILL-TABLE-SHAPE", "author",
     "The `[[skill]]` fields an author writes by hand"),
    ("PROP-018-agentic-standalone-modes.xml", "SKILL-EXTERNAL-SHAPE", "author",
     "The declaration form for external-source skills and their resources"),
    ("PROP-018-agentic-standalone-modes.xml", "SKILL-EXTERNAL-LAWS", "author",
     "The constraints that make an external skill declaration valid"),
    ("PROP-018-agentic-standalone-modes.xml", "PROJECTION-DEF", "user,author",
     "What installing a skill does, and into which agent directories"),
    ("PROP-018-agentic-standalone-modes.xml", "CMD-SKILL-LIST", "user",
     "The command that lists declared skills"),
    ("PROP-018-agentic-standalone-modes.xml", "CMD-SKILL-INSTALL", "user",
     "The install command, its defaults, flags and report"),
    ("PROP-018-agentic-standalone-modes.xml", "CMD-SKILL-UNINSTALL", "user",
     "The inverse command, and that foreign skills survive it"),
    ("PROP-018-agentic-standalone-modes.xml", "RELAY-PARKS", "user,agent",
     "A reasoning command parks an intent instead of acting"),
    ("PROP-018-agentic-standalone-modes.xml", "DRAIN-VERB", "user,agent",
     "`vibe command` is the one drain verb, consume-on-read"),
    ("PROP-018-agentic-standalone-modes.xml", "EMPTY-SLOT-OK", "agent",
     "An empty slot is success, not an error"),
    ("PROP-018-agentic-standalone-modes.xml", "NO-WRITE-BACK", "agent",
     "There is no return channel; do not wait for one"),
    ("PROP-018-agentic-standalone-modes.xml", "ONE-OP-TWO-TRANSPORTS", "agent",
     "The same operation reaches the agent by CLI or MCP"),
    ("PROP-018-agentic-standalone-modes.xml", "EXPLAIN-DEMONSTRATOR", "user,agent",
     "`vibe agentic explain` exists and what running it produces"),

    # ---- PROP-050 — dependency visibility (BUILT) --------------------------
    ("PROP-050-dependency-visibility.xml", "ACCESS-LEVELS", "user,author",
     "The `access` property and its three values on an edge"),
    ("PROP-050-dependency-visibility.xml", "ACCESS-PUBLIC", "user,author",
     "The default; what seeps to every consumer above"),
    ("PROP-050-dependency-visibility.xml", "ACCESS-PRIVATE", "user,author",
     "The mark that keeps a dependency in the declarant's world"),
    ("PROP-050-dependency-visibility.xml", "ACCESS-FRIENDS", "user,author",
     "The curated middle: seepage only to friends"),
    ("PROP-050-dependency-visibility.xml", "FRIEND-EDGE-FLAG", "user,author",
     "Per-edge `friend`, default false; friendship is opt-in"),
    ("PROP-050-dependency-visibility.xml", "FRIENDS-ONLY-IMPLIES-FRIEND", "author",
     "One mark per hop; explicit `friend = false` still wins"),
    ("PROP-050-dependency-visibility.xml", "VISIBILITY-SECTION", "user,author",
     "Where the node-level visibility vocabulary lives in a manifest"),
    ("PROP-050-dependency-visibility.xml", "PUBLIC-PLUS-FRIEND", "author",
     "Friendship never travels onward through a public edge"),
    ("PROP-050-dependency-visibility.xml", "TRANSIT-WITHOUT-DECLARATION", "user,author",
     "Befriending pulls a bundle; the voucher picks its versions"),
    ("PROP-050-dependency-visibility.xml", "UNFRIEND-IS-NODE-SCOPED", "user,author",
     "`unfriend` prunes the closure only through the declaring node"),
    ("PROP-050-dependency-visibility.xml", "EXCLUDE-IS-EDGE-SCOPED", "user,author",
     "`exclude` kills a subtree per edge, not globally"),
    ("PROP-050-dependency-visibility.xml", "ALLOW-FRIENDS-DESIGN", "author",
     "The provider's permits list and how it is written"),
    ("PROP-050-dependency-visibility.xml", "ALLOW-FRIENDS-STATES", "author",
     "Absent, empty and listed mean three different things"),
    ("PROP-050-dependency-visibility.xml", "ALLOW-FRIENDS-CHECKPOINT", "author",
     "A rejected grant warns; it never fails the build"),
    ("PROP-050-dependency-visibility.xml", "SEAL-GATES-FRIENDSHIP-NOT-DELIVERY", "author",
     "Sealing shuts the circle, never the package's arrival"),
    ("PROP-050-dependency-visibility.xml", "OVERRIDE-ANYWHERE", "user,author",
     "`[override]` is lawful in any manifest, not only the root"),
    ("PROP-050-dependency-visibility.xml", "OVERRIDE-PATH-SEMANTICS", "author",
     "Nearer-to-root overrides win; effective attributes are per chain"),
    ("PROP-050-dependency-visibility.xml", "OVERRIDE-KEY-COEXISTENCE", "author",
     "Two lawful `override` shapes; both in one manifest errors"),
    ("PROP-050-dependency-visibility.xml", "PRIVATE-IS-THE-DEV-WORLD", "user,author",
     "A root traverses all its own edges; no dev-dependencies section"),
    ("PROP-050-dependency-visibility.xml", "RESOLVE-EFFECTIVE-ONLY", "user",
     "The lock file records the effective set, nothing else"),
    ("PROP-050-dependency-visibility.xml", "MATERIALISE-EFFECTIVE-ONLY", "user",
     "`vibedeps/` holds exactly the effective set"),
    ("PROP-050-dependency-visibility.xml", "VIBE-WHY", "user",
     "The command that explains why a package is present or absent"),
    ("PROP-050-dependency-visibility.xml", "VERIFY-LOCK-DIFF", "user",
     "`vibe update` prints the closure diff and its cost"),
    ("PROP-050-dependency-visibility.xml", "REEXPORT-USAGE-NORM", "author",
     "The authoring duty: narrow deliberately, answer for every public edge"),

    # ---- PROP-046 — the adoption-facts registry ----------------------------
    ("PROP-046-adoption-facts-registry.xml", "REGISTRY-HOME", "user",
     "A tracked directory vibe writes at the project root"),
    ("PROP-046-adoption-facts-registry.xml", "LAW-SOVEREIGNTY", "user",
     "Imported statuses are ignored; adoption is a deliberate act"),
    ("PROP-046-adoption-facts-registry.xml", "CLI-FACTS", "user",
     "`vibe facts` is the only lawful lever over the registry"),

    # ---- PROP-048 — tokenomics --------------------------------------------
    ("PROP-048-tokenomics.xml", "THE-LAYER-LAW", "author,agent",
     "Reading order is stability order; where new text belongs"),
    ("PROP-048-tokenomics.xml", "STATIC-ROLE", "author,agent",
     "What the static lane is and why it stays byte-identical"),
    ("PROP-048-tokenomics.xml", "STATIC-PREFIX-SHARING", "author",
     "Nothing per-session or per-agent may enter the prefix"),

    # ---- PROP-051 — vibe refactor -----------------------------------------
    ("PROP-051-refactor-umbrella.xml", "CONVERT-SOURCE-SURFACE", "author",
     "The converting verb, what it selects and what it skips"),
    ("PROP-051-refactor-umbrella.xml", "ONE-DOCUMENT-ONE-FORM-ON-CONVERT", "author",
     "It deletes the original and keeps no backups"),
    ("PROP-051-refactor-umbrella.xml", "HONESTY-BY-REVERSE", "author",
     "Three loss classes; when the verb refuses to convert"),
    ("PROP-051-refactor-umbrella.xml", "FORCE-AND-PROMPT", "author",
     "What `--force` waives, what `--dry-run` does, the exit codes"),
]

INSERT = ' action="continue" actionstage="doc" audience="{audiences}"'


def main() -> int:
    root = Path.cwd()
    if not (root / COMMON).is_dir():
        print(f"error: run from the repository root (no {COMMON})", file=sys.stderr)
        return 2

    # Group the plan per file, preserving order.
    per_file: dict[str, list[tuple[str, str, str]]] = {}
    for name, anchor, audiences, why in PLAN:
        per_file.setdefault(name, []).append((anchor, audiences, why))

    originals: dict[Path, str] = {}
    updated: dict[Path, str] = {}
    errors: list[str] = []
    inserted = 0

    for name, entries in per_file.items():
        path = root / COMMON / name
        if not path.is_file():
            errors.append(f"{name}: file not found")
            continue
        with open(path, "r", encoding="utf-8", newline="") as fh:
            text = fh.read()
        originals[path] = text

        for anchor, audiences, _why in entries:
            pattern = re.compile(
                r'<' + re.escape(anchor) + r'\s+fact="true"\s+status="([^"]*)"'
            )
            matches = list(pattern.finditer(text))
            if not matches:
                errors.append(f"{name}#{anchor}: opening fact tag not found")
                continue
            if len(matches) > 1:
                errors.append(
                    f"{name}#{anchor}: opening fact tag found {len(matches)} times"
                )
                continue
            m = matches[0]
            # The remainder of the opening tag must not already act.
            tail_end = text.find(">", m.end())
            if tail_end == -1:
                errors.append(f"{name}#{anchor}: unterminated opening tag")
                continue
            if "action=" in text[m.end():tail_end]:
                errors.append(f"{name}#{anchor}: already carries action=")
                continue
            if not m.group(1).endswith("/done"):
                errors.append(
                    f"{name}#{anchor}: status {m.group(1)!r} is not a done status"
                )
                continue
            text = (
                text[: m.end()]
                + INSERT.format(audiences=audiences)
                + text[m.end():]
            )
            inserted += 1

        updated[path] = text

    if errors:
        for e in errors:
            print(f"error: {e}", file=sys.stderr)
        print("no file was written", file=sys.stderr)
        return 1

    for path, text in updated.items():
        with open(path, "w", encoding="utf-8", newline="") as fh:
            fh.write(text)

    # Re-parse every touched file; restore everything on any failure.
    for path in updated:
        try:
            ET.parse(path)
        except ET.ParseError as exc:
            for restore, text in originals.items():
                with open(restore, "w", encoding="utf-8", newline="") as fh:
                    fh.write(text)
            print(f"error: {path} does not parse after the edit: {exc}",
                  file=sys.stderr)
            print("all files restored", file=sys.stderr)
            return 1

    for name, entries in per_file.items():
        print(f"{name}: {len(entries)} marked")
    print(f"total: {inserted} marks across {len(updated)} files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
