#!/usr/bin/env python3
"""P3-M3 — mark the documentation obligations of the M3 document set (A3.13).

For every (file, anchor, audiences) row below the script finds the opening tag
`<ANCHOR fact="true" status="...">` exactly once and inserts

    ` action="continue" actionstage="doc" audience="<audiences>"`

directly after the `status="..."` attribute. Nothing else in the file changes.

It fails loudly when an anchor is missing, found more than once, or already
carries `action=` / `actionstage=`, and it re-parses every touched file with
xml.etree afterwards.

Run from the repository root:  python campaigns/docs-2026-09/findings/P3-M3-mark.py
"""

from __future__ import annotations

import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

# (file, anchor, audiences, why <= 12 words)
MARKS: list[tuple[str, str, str, str]] = [
    # ---- vibevm/vibespecs/common/PROP-000.xml -----------------------------
    ("vibevm/vibespecs/common/PROP-000.xml", "IDENTITY-FORM", "user,author",
     "the coordinate every command and manifest entry is written in"),
    ("vibevm/vibespecs/common/PROP-000.xml", "CF-LATEST", "user",
     "bare pkgref on the CLI means latest stable"),
    ("vibevm/vibespecs/common/PROP-000.xml", "CF-EXACT", "user",
     "the exact-version spelling a user types"),
    ("vibevm/vibespecs/common/PROP-000.xml", "CF-RANGE", "user",
     "the semver-range spelling a user types"),
    ("vibevm/vibespecs/common/PROP-000.xml", "INIT-DEFAULT-REGISTRY", "user",
     "what registry a fresh project talks to by default"),
    ("vibevm/vibespecs/common/PROP-000.xml", "LICENSE-EULA", "user",
     "the licence under which the tool is adopted"),
    ("vibevm/vibespecs/common/PROP-000.xml", "OBS-OUTPUT-MODES", "user",
     "the global output modes every command honours"),
    ("vibevm/vibespecs/common/PROP-000.xml", "PLATFORMS-TRIO", "user",
     "which operating systems the tool supports"),
    ("vibevm/vibespecs/common/PROP-000.xml", "INV-USER-FILES", "user",
     "the boot files vibe never overwrites"),
    ("vibevm/vibespecs/common/PROP-000.xml", "VIBE-DIR-IGNORED", "user",
     "the cache directory is per-project and not committed"),
    ("vibevm/vibespecs/common/PROP-000.xml", "TOKEN-SURFACE-SECRET", "user",
     "how a publish token is treated and must be supplied"),
    ("vibevm/vibespecs/common/PROP-000.xml", "WORKSPACE-LAYOUT", "dev",
     "the crate layout an architecture page must state"),
    ("vibevm/vibespecs/common/PROP-000.xml", "JTD-SSOT", "dev",
     "the wire-format law: schemas generate types, never hand-written"),
    ("vibevm/vibespecs/common/PROP-000.xml",
     "SURFACE-DISCIPLINE-IS-THE-OMNICHANNEL-FLOW", "dev",
     "the library-first seam every surface sits on"),
    ("vibevm/vibespecs/common/PROP-000.xml", "GIT-PRACTICES-FAMILY", "dev",
     "the commit-and-push discipline binding every contributor"),

    # ---- vibevm/vibespecs/common/PROP-052-directory-layout.xml ------------
    ("vibevm/vibespecs/common/PROP-052-directory-layout.xml", "THE-LAYOUT",
     "user,author",
     "the directory layout every project and package carries"),
    ("vibevm/vibespecs/common/PROP-052-directory-layout.xml",
     "ADDRESSES-SURVIVE-THE-MOVE", "author",
     "how a file path maps onto the address that cites it"),
    ("vibevm/vibespecs/common/PROP-052-directory-layout.xml",
     "NO-LEGACY-LAYOUT", "user",
     "an old-layout project fails loudly with a migration recipe"),
    ("vibevm/vibespecs/common/PROP-052-directory-layout.xml",
     "PACKAGES-CARRY-THE-LAYOUT-TOO", "author",
     "a package root mirrors the same layout"),

    # ---- vibevm/vibespecs/common/PROP-028-package-families.xml ------------
    ("vibevm/vibespecs/common/PROP-028-package-families.xml", "FAMILY-DEF",
     "author", "what a package family is and what it delivers"),
    ("vibevm/vibespecs/common/PROP-028-package-families.xml",
     "ROLE-AGGREGATOR", "user,author",
     "requiring the aggregator installs the whole family"),
    ("vibevm/vibespecs/common/PROP-028-package-families.xml", "UNISON-LAW",
     "user,author", "family members share one version; reading one tells all"),
    ("vibevm/vibespecs/common/PROP-028-package-families.xml",
     "SURFACE-NAMING-LAW", "author",
     "the family stem reaches crates, binaries, skills, servers"),

    # ---- vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml",
     "TWO-TREES", "user",
     "install never writes into the authored spec tree"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml",
     "TREE-VIBEDEPS", "user",
     "where dependencies land and how the slots are named"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml",
     "VIBEDEPS-COMMITTED", "user",
     "the dependency tree is committed; a clone boots without install"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml",
     "ARTIFACTS-PAIR", "user",
     "install generates two boot artifacts per entry-point node"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml",
     "ARTIFACT-INDEX-MD", "user",
     "the generated manifest a session reads at boot"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml",
     "ARTIFACTS-GENERATED", "user",
     "generated and git-tracked; never hand-edited"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml",
     "SESSION-START-ORDER", "user,agent",
     "the exact order a session reads its boot lane"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml",
     "PURE-FILE-READING", "agent",
     "boot is file-reading only; never run a tool"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml",
     "SCHEMA-LINK-FIELD", "user,author",
     "the link field, its values, its default, where valid"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml",
     "LINK-DYNAMIC", "user,author",
     "the default inclusion type and its when gate"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml",
     "LINK-STATIC", "user,author",
     "the priority lane and when to spend it"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml",
     "WHEN-FORCES-DYNAMIC", "author",
     "a condition forces the dynamic form whatever link says"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml",
     "SCHEMA-BOOT-SNIPPET", "author",
     "the boot-snippet declaration an author must write"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml",
     "CATEGORY-ORDER", "author",
     "where a declared category places the contribution"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml",
     "INSTALL-UNIFIED", "user",
     "what install does: one resolution, one lock, regenerated lanes"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml",
     "REINSTALL", "user",
     "the command that regenerates materialised state"),

    # ---- vibevm/vibespecs/modules/vibe-workspace/PROP-007-workspace.xml ---
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-007-workspace.xml",
     "WORKSPACE-TABLE", "user",
     "how a multi-package project declares its members"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-007-workspace.xml",
     "EXPLICIT-MEMBERSHIP", "user",
     "membership is declared; nothing is auto-discovered"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-007-workspace.xml",
     "ONE-MANIFEST", "user,author",
     "one manifest file per node; the role is its section set"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-007-workspace.xml",
     "PACKAGE-XOR-PROJECT", "user,author",
     "the two role sections are mutually exclusive"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-007-workspace.xml",
     "NESTING-PRINCIPLE", "user",
     "nesting groups; it never creates a resolution domain"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-007-workspace.xml",
     "ONE-LOCKFILE", "user",
     "one lockfile at the absolute root, none per member"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-007-workspace.xml",
     "COMMAND-BUBBLING", "user",
     "a command run inside a member acts on the root"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-007-workspace.xml",
     "PATH-SOURCE", "user",
     "how a cross-member dependency is spelled"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-007-workspace.xml",
     "DUAL-FORM", "user,author",
     "a publishable member's path dependency needs a version too"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-007-workspace.xml",
     "VERSION-PLACEHOLDERS", "user",
     "how one version is written once and referenced by name"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-007-workspace.xml",
     "ONE-SOURCE-TREE", "user",
     "publishing copies a member out; the tree is untouched"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-007-workspace.xml",
     "PUBLISH-POSTURE", "user,author",
     "how a node declares whether and where it publishes"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-007-workspace.xml",
     "PUBLISH-TOPOLOGICAL", "user",
     "publish order and which members are skipped"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-007-workspace.xml",
     "PUBLISH-NOT-ATOMIC", "user",
     "a failed publish stops and reports partial progress"),

    # ---- .../PROP-038-hybrid-boot-linking.xml -----------------------------
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-038-hybrid-boot-linking.xml",
     "UNIT-PER-PACKAGE", "user",
     "every materialised package carries its own boot artifacts"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-038-hybrid-boot-linking.xml",
     "EDGE-IS-INSTRUCTION", "user,author",
     "link belongs to the edge, never to the pulled package"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-038-hybrid-boot-linking.xml",
     "EDGE-STATIC-TRANSITIVE", "author",
     "the one mode that forces a whole subtree static"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-038-hybrid-boot-linking.xml",
     "MODE-STATIC-SOFT", "author",
     "what a bare static means: hoisted, deduplicated, the default"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-038-hybrid-boot-linking.xml",
     "MODE-STATIC-HARD", "author",
     "the opt-in that keeps a package local to its consumer"),

    # ---- .../PROP-034-transitive-links-boot-graph.xml ---------------------
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-034-transitive-links-boot-graph.xml",
     "STEP-REJECT-CYCLES", "user",
     "a dependency cycle is a hard error naming the cycle"),

    # ---- .../PROP-035-spec-compiler.xml -----------------------------------
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-035-spec-compiler.xml",
     "FORMAT-SIMPLE", "author",
     "the default package format and what it carries"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-035-spec-compiler.xml",
     "FORMAT-NORMAL", "author",
     "what opting into the native format buys"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-035-spec-compiler.xml",
     "DIR-CONTRACT", "author",
     "the surface directory of a native package"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-035-spec-compiler.xml",
     "DIR-SOURCE", "author",
     "the heavy directory reached only through the surface"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-035-spec-compiler.xml",
     "USE-TREE-SHAKING", "author",
     "a native package nobody uses participates in nothing"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-035-spec-compiler.xml",
     "USE-ANCESTOR-RULE", "author",
     "a use pulls the whole enclosing anchored section"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-035-spec-compiler.xml",
     "EMBED-EXACT-RULE", "author",
     "an embed splices exactly the addressed node"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-035-spec-compiler.xml",
     "DIRECTIVE-MANIFEST-AGREE", "author",
     "every file a directive names must be declared"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-035-spec-compiler.xml",
     "MERGE-DEFAULT-ADD", "author",
     "the default merge mode between contract and source"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-035-spec-compiler.xml",
     "NO-DEADLOCK-INVARIANT", "author",
     "where a cycle is legal and where it is an error"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-035-spec-compiler.xml",
     "AT-SPEC-MANDATORY", "author,agent",
     "the sigil that makes a reference a mandatory read"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-035-spec-compiler.xml",
     "BARE-SPEC-DISCRETIONARY", "author,agent",
     "a bare address is read at the reader's discretion"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-035-spec-compiler.xml",
     "READ-ONCE", "agent",
     "a mandatory target is read on first encounter only"),
    ("vibevm/vibespecs/modules/vibe-workspace/PROP-035-spec-compiler.xml",
     "URI-VERSION-OPTIONAL", "author",
     "an address needs no version; pinning is a feature"),

    # ---- .../PROP-003-dep-evolution.xml -----------------------------------
    ("vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml",
     "FEATURES-TABLE", "author",
     "how optional components are declared in a package"),
    ("vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml",
     "KEEP-ADDITIVE", "author",
     "a feature may only add; it never removes"),
    ("vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml",
     "KEEP-DEFAULT", "author",
     "which features are on when nothing is asked"),
    ("vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml",
     "KEEP-UNIFICATION", "user,author",
     "features unify across the graph as a union"),
    ("vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml",
     "CLI-INSTALL-FEATURES", "user",
     "the flags that select features at install time"),
    ("vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml",
     "SUBSKILL-DEFINITION", "author",
     "the smallest activatable content unit in a package"),
    ("vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml",
     "DELIVERY-PRIMARY-AXIS", "author",
     "the delivery axis every subskill must choose"),
    ("vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml",
     "ACTIVATION-ANY-MATCH", "author",
     "a subskill activates when any channel matches"),
    ("vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml",
     "DESCRIPTION-REQUIRED", "author",
     "lazy delivery modes require a description"),
    ("vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml",
     "DESCRIBES-ON-SUBSKILLS", "author",
     "how a package or subskill names what it describes"),
    ("vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml",
     "I18N-DECISION", "user,author",
     "how a language preference and localised files work"),
    ("vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml",
     "SIDECAR-PATTERN", "author",
     "the exact filename a translated file carries"),
    ("vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml",
     "I18N-CANONICAL-INVARIANT", "author",
     "every package must ship the canonical form of every file"),
    ("vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml",
     "PREF-CLI-FLAG", "user",
     "the flag that overrides the language for one run"),
    ("vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml",
     "DENY-UNKNOWN-FIELDS", "author",
     "an unknown manifest key fails loudly, never silently"),

    # ---- vibevm/vibespecs/common/PROP-045-xml-spec-sources.xml ------------
    ("vibevm/vibespecs/common/PROP-045-xml-spec-sources.xml",
     "NAMED-SECTION-ELEMENTS", "author",
     "how a section is spelled in the dialect"),
    ("vibevm/vibespecs/common/PROP-045-xml-spec-sources.xml",
     "NAMED-FACT-ELEMENTS", "author",
     "how a fact is spelled and recognised in the dialect"),
    ("vibevm/vibespecs/common/PROP-045-xml-spec-sources.xml",
     "FACTS-GROUP-ELEMENT", "author",
     "which list element to use when items are facts"),
    # ##DOC-VOCAB-BY-KIND states the same law but already carries an action=
    # marker (see the report), so the limit clause carries the obligation.
    ("vibevm/vibespecs/common/PROP-045-xml-spec-sources.xml",
     "DOC-VOCAB-README-LIMIT", "author",
     "the documentation vocabulary opens only in doc packages"),

    # ---- vibevm/vibespecs/common/PROP-044-change-native-formats.xml -------
    ("vibevm/vibespecs/common/PROP-044-change-native-formats.xml",
     "THE-FREEZE-MODEL", "user,author",
     "a version is mutable until its author freezes it"),
    ("vibevm/vibespecs/common/PROP-044-change-native-formats.xml",
     "THE-PUBLIC-SWITCH", "user",
     "pre-publication: breaks are unmigrated, the recipe is regenerate"),
    ("vibevm/vibespecs/common/PROP-044-change-native-formats.xml",
     "M-WIDE-INTEGERS-AS-STRINGS", "dev",
     "the wire encoding of any integer wider than 32 bits"),
]

INSERT = ' action="continue" actionstage="doc" audience="{aud}"'


def main() -> int:
    root = Path(__file__).resolve().parents[3]
    by_file: dict[str, list[tuple[str, str]]] = {}
    for path, anchor, aud, _why in MARKS:
        by_file.setdefault(path, []).append((anchor, aud))

    errors: list[str] = []
    total = 0
    staged: dict[str, str] = {}
    for rel, rows in by_file.items():
        fp = root / rel
        if not fp.is_file():
            errors.append(f"{rel}: file not found")
            continue
        with fp.open("r", encoding="utf-8", newline="") as fh:  # keep line endings
            text = fh.read()
        for anchor, aud in rows:
            open_tag = re.compile(
                r"<" + re.escape(anchor) + r' fact="true" status="([a-z]+/[a-z]+)"([^>]*)>'
            )
            found = list(open_tag.finditer(text))
            if len(found) == 0:
                errors.append(f"{rel}#{anchor}: opening tag not found")
                continue
            if len(found) > 1:
                errors.append(f"{rel}#{anchor}: opening tag found {len(found)} times")
                continue
            m = found[0]
            if "action=" in m.group(2) or "actionstage=" in m.group(2):
                errors.append(f"{rel}#{anchor}: already carries action= / actionstage=")
                continue
            if not m.group(1).endswith("/done"):
                errors.append(f"{rel}#{anchor}: status is {m.group(1)}, not a done fact")
                continue
            head = m.group(0)
            cut = head.index('status="') + len('status="') + len(m.group(1)) + 1
            text = (
                text[: m.start()]
                + head[:cut]
                + INSERT.format(aud=aud)
                + head[cut:]
                + text[m.end():]
            )
            total += 1
        staged[rel] = text

    if errors:
        for e in errors:
            print(f"ERROR {e}", file=sys.stderr)
        print(f"aborted: {len(errors)} error(s); no file written", file=sys.stderr)
        return 1

    for rel, text in staged.items():
        with (root / rel).open("w", encoding="utf-8", newline="") as fh:
            fh.write(text)

    for rel in by_file:
        try:
            ET.parse(root / rel)
        except ET.ParseError as exc:
            print(f"ERROR {rel}: XML parse failed after edit: {exc}", file=sys.stderr)
            return 1

    print(f"marked {total} fact(s) across {len(by_file)} file(s); all parse clean")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
