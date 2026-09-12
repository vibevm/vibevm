#!/usr/bin/env python3
"""P3-M4 — mark the documentation obligations (A3.13) in the fourteen specs of PACKET-P3-M4.

For every (file, anchor, audience) below the script finds the opening tag

    <ANCHOR fact="true" status="<stage>/<state>"

exactly once, and inserts

    action="continue" actionstage="doc" audience="<audience>"

immediately after the status attribute. Nothing else in the file changes.

It fails loudly when an anchor is missing, occurs more than once, is not
`.../done`, or already carries an `action=` / `actionstage=` attribute.
After the edits every touched file is re-parsed with xml.etree; any parse
error is fatal.

Run from the repository root:  python campaigns/docs-2026-09/findings/P3-M4-mark.py
"""

from __future__ import annotations

import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

COM = "vibevm/vibespecs/common"
FACTS = "vibevm/vibespecs/modules/vibe-facts"
CLI = "vibevm/vibespecs/modules/vibe-cli"
MCP = "vibevm/vibespecs/modules/vibe-mcp"
ACT = "vibevm/vibespecs/modules/vibe-actions"
SET = "vibevm/vibespecs/modules/vibe-settings"

# (file, anchor, audiences)
MARKS: list[tuple[str, str, str]] = [
    # ---------------------------------------------------------------- PROP-057
    # (eighteen obligations were already marked as the campaign's sample; these
    #  are the rest of the document's own contracts.)
    (f"{COM}/PROP-057-documentation-packages-and-site.xml", "KIND-APP-VS-TOOL", "author"),
    (f"{COM}/PROP-057-documentation-packages-and-site.xml", "LEVEL-ZERO", "author"),
    (f"{COM}/PROP-057-documentation-packages-and-site.xml", "REL-OFFICIAL-IS-CONVERGENCE", "author"),
    (f"{COM}/PROP-057-documentation-packages-and-site.xml", "REL-DEFAULT-CONVENTION", "author"),
    (f"{COM}/PROP-057-documentation-packages-and-site.xml", "LOC-LANGUAGE-FIELD", "author"),
    (f"{COM}/PROP-057-documentation-packages-and-site.xml", "LOC-DOCUMENTS-MATCH", "author"),
    (f"{COM}/PROP-057-documentation-packages-and-site.xml", "LOC-OFFICIAL-TRANSLATION", "author"),
    (f"{COM}/PROP-057-documentation-packages-and-site.xml", "SITE-MOUNT", "user"),
    (f"{COM}/PROP-057-documentation-packages-and-site.xml", "SITE-VERSION-SHOWS-CURRENT", "user"),
    (f"{COM}/PROP-057-documentation-packages-and-site.xml", "READER-NUMBERED-BLOCKS", "user,agent"),
    (f"{COM}/PROP-057-documentation-packages-and-site.xml", "SEO-RAW-PROJECTIONS", "agent"),
    (f"{COM}/PROP-057-documentation-packages-and-site.xml", "SEO-LLMS-FILES", "agent"),
    (f"{COM}/PROP-057-documentation-packages-and-site.xml", "SEO-MANIFEST-AND-RESOLVER", "agent"),
    (f"{COM}/PROP-057-documentation-packages-and-site.xml", "OBS-RULE-EDGE-UNPINNED", "author"),
    (f"{COM}/PROP-057-documentation-packages-and-site.xml", "OBS-COVERAGE-GATE", "author"),
    (f"{COM}/PROP-057-documentation-packages-and-site.xml", "STYLE-SOURCE-LANGUAGE", "author"),
    (f"{COM}/PROP-057-documentation-packages-and-site.xml", "STYLE-PROMPT-FIRST", "author"),
    (f"{COM}/PROP-057-documentation-packages-and-site.xml", "STYLE-LINT", "author"),
    # ------------------------------------------------- PROP-043 (facts markup)
    (f"{FACTS}/PROP-043-facts-markup.xml", "STATUS-ELEMENT", "author"),
    (f"{FACTS}/PROP-043-facts-markup.xml", "POINT-SELF-CLOSING", "author"),
    (f"{FACTS}/PROP-043-facts-markup.xml", "FENCE-AWARE", "author"),
    (f"{FACTS}/PROP-043-facts-markup.xml", "VOCAB-CLOSED", "author"),
    (f"{FACTS}/PROP-043-facts-markup.xml", "MULTI-MARKERS", "author"),
    (f"{FACTS}/PROP-043-facts-markup.xml", "ACTIONSTAGE-NARROWS", "author"),
    (f"{FACTS}/PROP-043-facts-markup.xml", "AUDIENCE-DOC-USE", "author"),
    (f"{FACTS}/PROP-043-facts-markup.xml", "SHORTHAND-FORMS", "author"),
    (f"{FACTS}/PROP-043-facts-markup.xml", "SHORTHAND-BARE", "author"),
    (f"{FACTS}/PROP-043-facts-markup.xml", "PLACE-DOCUMENT", "author"),
    (f"{FACTS}/PROP-043-facts-markup.xml", "PLACE-SECTION", "author"),
    (f"{FACTS}/PROP-043-facts-markup.xml", "PLACE-PARAGRAPH", "author"),
    (f"{FACTS}/PROP-043-facts-markup.xml", "PLACE-LIST-ITEM", "author"),
    (f"{FACTS}/PROP-043-facts-markup.xml", "NO-ORPHAN-MARKER", "author"),
    (f"{FACTS}/PROP-043-facts-markup.xml", "FACT-ANCHOR-SYNTAX", "author"),
    (f"{FACTS}/PROP-043-facts-markup.xml", "ANCHORED-WHEN-MARKED", "author"),
    (f"{FACTS}/PROP-043-facts-markup.xml", "FACT-ID-GRAMMAR", "author"),
    (f"{FACTS}/PROP-043-facts-markup.xml", "ROLLUP-DOWNWARD", "author"),
    (f"{FACTS}/PROP-043-facts-markup.xml", "ROLLUP-UPWARD", "author"),
    (f"{FACTS}/PROP-043-facts-markup.xml", "REQUIRES-GRAMMAR", "author"),
    (f"{FACTS}/PROP-043-facts-markup.xml", "REQUIRED-ARTIFACT-KINDS", "author"),
    (f"{FACTS}/PROP-043-facts-markup.xml", "CONFIG-FILE", "author"),
    (f"{FACTS}/PROP-043-facts-markup.xml", "INCLUDE-STYLE", "author"),
    (f"{FACTS}/PROP-043-facts-markup.xml", "BOUNDARY-CLI", "author"),
    # ---------------------------------------------------------------- PROP-036
    (f"{CLI}/PROP-036-package-tree.xml", "TREE-ANSWER", "user"),
    (f"{CLI}/PROP-036-package-tree.xml", "TREE-READ-ONLY", "user"),
    (f"{CLI}/PROP-036-package-tree.xml", "TREE-INPUTS", "user"),
    (f"{CLI}/PROP-036-package-tree.xml", "OUT-TUI", "user"),
    (f"{CLI}/PROP-036-package-tree.xml", "OUT-JSON", "user"),
    (f"{CLI}/PROP-036-package-tree.xml", "OUT-PLAIN", "user"),
    (f"{CLI}/PROP-036-package-tree.xml", "ROW-PER-PACKAGE", "user"),
    (f"{CLI}/PROP-036-package-tree.xml", "EFFECTIVE-FROM-ARTIFACTS", "user"),
    (f"{CLI}/PROP-036-package-tree.xml", "STATIC-SIZE-INDICATOR", "user"),
    (f"{CLI}/PROP-036-package-tree.xml", "JSON-CONTRACT", "user"),
    (f"{CLI}/PROP-036-package-tree.xml", "DIAG-STALE-ARTIFACTS", "user"),
    (f"{CLI}/PROP-036-package-tree.xml", "DIAG-ROOT-DRIFT", "user"),
    (f"{CLI}/PROP-036-package-tree.xml", "DAG-RENDERING", "user"),
    # ---------------------------------------------------------------- PROP-037
    (f"{CLI}/PROP-037-tree-tui.xml", "TIER-DETECTION", "user"),
    (f"{CLI}/PROP-037-tree-tui.xml", "THREE-SHAPES", "user"),
    (f"{CLI}/PROP-037-tree-tui.xml", "THREE-MODES", "user"),
    (f"{CLI}/PROP-037-tree-tui.xml", "MODE-SELECT-REQ", "user"),
    (f"{CLI}/PROP-037-tree-tui.xml", "F-KEY-SCHEME", "user"),
    (f"{CLI}/PROP-037-tree-tui.xml", "TREE-KEYS-REQ", "user"),
    (f"{CLI}/PROP-037-tree-tui.xml", "FOCUS-GROUPS-REQ", "user"),
    (f"{CLI}/PROP-037-tree-tui.xml", "MODAL-STACK-REQ", "user"),
    (f"{CLI}/PROP-037-tree-tui.xml", "F2-SORT-MENU-REQ", "user"),
    (f"{CLI}/PROP-037-tree-tui.xml", "F1-SEARCH-REQ", "user"),
    (f"{CLI}/PROP-037-tree-tui.xml", "QUIT-CONFIRM-REQ", "user"),
    (f"{CLI}/PROP-037-tree-tui.xml", "SETTINGS-PERSISTENCE", "user"),
    (f"{CLI}/PROP-037-tree-tui.xml", "COPY-PROVIDERS-REQ", "user"),
    (f"{CLI}/PROP-037-tree-tui.xml", "COPY-FLOW-REQ", "user"),
    (f"{CLI}/PROP-037-tree-tui.xml", "PNG-RESERVED", "user"),
    # ---------------------------------------------------------------- PROP-042
    (f"{CLI}/PROP-042-aiui-observation.xml", "SIDE-EFFECT-KEYS-REFUSED", "agent"),
    (f"{CLI}/PROP-042-aiui-observation.xml", "SNAPSHOT-FORMATS", "agent"),
    (f"{CLI}/PROP-042-aiui-observation.xml", "FMT-CELLS", "agent"),
    (f"{CLI}/PROP-042-aiui-observation.xml", "KEY-SCRIPT-GRAMMAR", "agent"),
    (f"{CLI}/PROP-042-aiui-observation.xml", "AIUI-FAMILY", "agent"),
    (f"{CLI}/PROP-042-aiui-observation.xml", "RENDER-VERB-SEMANTICS", "agent"),
    (f"{CLI}/PROP-042-aiui-observation.xml", "TERMINAL-VERBS", "agent"),
    (f"{CLI}/PROP-042-aiui-observation.xml", "MODEL-VERB", "agent"),
    (f"{CLI}/PROP-042-aiui-observation.xml", "TERM-LAUNCHER", "user"),
    (f"{CLI}/PROP-042-aiui-observation.xml", "IN-PLACE-UPGRADE", "user"),
    # ---------------------------------------------------------------- PROP-015
    (f"{MCP}/PROP-015-mcp-integration.xml", "SURFACE-SERVER", "user"),
    (f"{MCP}/PROP-015-mcp-integration.xml", "SURFACE-INSTALL", "user"),
    (f"{MCP}/PROP-015-mcp-integration.xml", "TOOL-QUERY-PACKAGE", "agent"),
    (f"{MCP}/PROP-015-mcp-integration.xml", "TOOL-READ-SUBSKILL", "agent"),
    (f"{MCP}/PROP-015-mcp-integration.xml", "TOOL-MATERIALISE-SUBSKILL", "agent"),
    (f"{MCP}/PROP-015-mcp-integration.xml", "MAP-QUERY-ANSWERS-A-DIFFERENT-QUESTION", "agent"),
    (f"{MCP}/PROP-015-mcp-integration.xml", "SELECT-SEVEN-PREDICATES-JOINED-BY-AND", "agent"),
    (f"{MCP}/PROP-015-mcp-integration.xml", "SELECT-AN-UNKNOWN-PREDICATE-IS-AN-ERROR", "agent"),
    (f"{MCP}/PROP-015-mcp-integration.xml", "AGENT-SET", "user"),
    (f"{MCP}/PROP-015-mcp-integration.xml", "CONFIG-PATH", "user"),
    (f"{MCP}/PROP-015-mcp-integration.xml", "CONFIG-MERGE", "user"),
    (f"{MCP}/PROP-015-mcp-integration.xml", "SKILL-MANIFEST", "user"),
    (f"{MCP}/PROP-015-mcp-integration.xml", "LIFECYCLE-MATRIX", "user"),
    (f"{MCP}/PROP-015-mcp-integration.xml", "INCLUDE-SELECTIVE", "author"),
    # ---------------------------------------------------------------- PROP-027
    (f"{MCP}/PROP-027-mcp-packages.xml", "MCP-KIND-DEF", "author"),
    (f"{MCP}/PROP-027-mcp-packages.xml", "TABLE-ONLY-IN-KIND", "author"),
    (f"{MCP}/PROP-027-mcp-packages.xml", "KIND-PROMISES-SERVER", "author"),
    (f"{MCP}/PROP-027-mcp-packages.xml", "SERVER-IS-BINARY", "author"),
    (f"{MCP}/PROP-027-mcp-packages.xml", "ARGS-CLOSED-SET", "author"),
    (f"{MCP}/PROP-027-mcp-packages.xml", "EXACT-PIN-LAW", "author"),
    (f"{MCP}/PROP-027-mcp-packages.xml", "REG-PACKAGE-DISCOVERY", "user"),
    (f"{MCP}/PROP-027-mcp-packages.xml", "REG-MANAGED-SIDECAR", "user"),
    (f"{MCP}/PROP-027-mcp-packages.xml", "REG-PROJECT-SCOPE", "user"),
    (f"{MCP}/PROP-027-mcp-packages.xml", "REG-STATUS", "user"),
    (f"{MCP}/PROP-027-mcp-packages.xml", "CONSENT-GATE-INHERITED", "user"),
    # ---------------------------------------------------------------- PROP-026
    (f"{MCP}/PROP-026-tcg-tool-family.xml", "TOOLS-NEW-HOME", "agent"),
    (f"{MCP}/PROP-026-tcg-tool-family.xml", "FOUR-TOOLS", "agent"),
    (f"{MCP}/PROP-026-tcg-tool-family.xml", "PARAM-LANGUAGE", "agent"),
    (f"{MCP}/PROP-026-tcg-tool-family.xml", "PARAMS-PASSTHROUGH", "agent"),
    (f"{MCP}/PROP-026-tcg-tool-family.xml", "ENRICHED-RESPONSES", "agent"),
    # ---------------------------------------------------------------- PROP-039
    (f"{ACT}/PROP-039-action-system.xml", "ADDRESS-GRAMMAR", "agent"),
    (f"{ACT}/PROP-039-action-system.xml", "I18N-FALLBACK-LAW", "author"),
    (f"{ACT}/PROP-039-action-system.xml", "MODEL-VIEW-DEF", "agent"),
    (f"{ACT}/PROP-039-action-system.xml", "AIUI-REFERENCE", "agent"),
    # ---------------------------------------------------------------- PROP-040
    (f"{SET}/PROP-040-settings.xml", "app-prefs-not-project", "user"),
    (f"{SET}/PROP-040-settings.xml", "L1-USER-MACHINE", "user"),
    (f"{SET}/PROP-040-settings.xml", "L2-REPO-SHARED", "user"),
    (f"{SET}/PROP-040-settings.xml", "L3-USER-PROJECT", "user"),
    (f"{SET}/PROP-040-settings.xml", "precedence-law", "user"),
    (f"{SET}/PROP-040-settings.xml", "MERGE-ARRAYS", "user"),
    (f"{SET}/PROP-040-settings.xml", "missing-is-default", "user"),
    (f"{SET}/PROP-040-settings.xml", "schema-first", "user"),
    (f"{SET}/PROP-040-settings.xml", "prefs-command", "user"),
    (f"{SET}/PROP-040-settings.xml", "gitignore-autogen", "user"),
    (f"{SET}/PROP-040-settings.xml", "no-secrets-in-committed", "user"),
    # ---------------------------------------------------------------- PROP-041
    (f"{SET}/PROP-041-settings-ui.xml", "tree-widget-req", "user"),
    (f"{SET}/PROP-041-settings-ui.xml", "tree-context", "user"),
    (f"{SET}/PROP-041-settings-ui.xml", "write-layer-choice", "user"),
    (f"{SET}/PROP-041-settings-ui.xml", "provenance-view", "user"),
    (f"{SET}/PROP-041-settings-ui.xml", "provenance-edit", "user"),
    (f"{SET}/PROP-041-settings-ui.xml", "settings-search", "user"),
]

ALLOWED = {"user", "author", "dev", "agent"}


def die(msg: str) -> None:
    print(f"FATAL: {msg}", file=sys.stderr)
    raise SystemExit(1)


def main() -> None:
    root = Path(__file__).resolve().parents[3]
    if not (root / "vibevm" / "vibespecs").is_dir():
        die(f"repository root not found (looked at {root})")

    seen: set[tuple[str, str]] = set()
    for rel, anchor, audience in MARKS:
        key = (rel, anchor)
        if key in seen:
            die(f"{rel}: anchor {anchor} listed twice in MARKS")
        seen.add(key)
        for part in audience.split(","):
            if part not in ALLOWED:
                die(f"{rel}#{anchor}: illegal audience {part!r}")

    by_file: dict[str, list[tuple[str, str]]] = {}
    for rel, anchor, audience in MARKS:
        by_file.setdefault(rel, []).append((anchor, audience))

    total = 0
    for rel, entries in by_file.items():
        path = root / rel
        if not path.is_file():
            die(f"missing file {rel}")
        with open(path, encoding="utf-8", newline="") as fh:
            src = fh.read()
        for anchor, audience in entries:
            pat = re.compile(
                r"<" + re.escape(anchor) + r'(\s+fact="true"\s+status="([a-z]+)/([a-z]+)")([^>]*)>'
            )
            hits = list(pat.finditer(src))
            if not hits:
                die(f"{rel}: anchor {anchor} not found as an opening fact tag")
            if len(hits) > 1:
                die(f"{rel}: anchor {anchor} found {len(hits)} times")
            m = hits[0]
            if m.group(3) != "done":
                die(f"{rel}#{anchor}: status is {m.group(2)}/{m.group(3)}, not done")
            if "action=" in m.group(4) or "actionstage=" in m.group(4):
                die(f"{rel}#{anchor}: already carries an action marker")
            insert = f' action="continue" actionstage="doc" audience="{audience}"'
            src = src[: m.end(1)] + insert + src[m.end(1) :]
            total += 1
        with open(path, "w", encoding="utf-8", newline="") as fh:
            fh.write(src)
        try:
            ET.parse(path)
        except ET.ParseError as exc:
            die(f"{rel}: XML no longer parses after the edit: {exc}")
        print(f"{rel}: {len(entries)} marker(s)")

    print(f"TOTAL: {total} marker(s) across {len(by_file)} file(s)")


if __name__ == "__main__":
    main()
