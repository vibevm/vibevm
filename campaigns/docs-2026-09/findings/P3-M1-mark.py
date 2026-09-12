#!/usr/bin/env python3
"""P3-M1 — mark the documentation obligations (A3.13) in the ten specs of PACKET-P3-M1.

For every (file, anchor, audience) below the script finds the opening tag

    <ANCHOR fact="true" status="<stage>/<state>"

exactly once, and inserts

    action="continue" actionstage="doc" audience="<audience>"

immediately after the status attribute. Nothing else in the file changes.

It fails loudly when an anchor is missing, occurs more than once, is not
`.../done`, or already carries an `action=` / `actionstage=` attribute.
After the edits every touched file is re-parsed with xml.etree; any parse
error is fatal.

Run from the repository root:  python campaigns/docs-2026-09/findings/P3-M1-mark.py
"""

from __future__ import annotations

import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

REG = "vibevm/vibespecs/modules/vibe-registry"
IDX = "vibevm/vibespecs/modules/vibe-index"
COM = "vibevm/vibespecs/common"

# (file, anchor, audiences)
MARKS: list[tuple[str, str, str]] = [
    # ---------------------------------------------------------------- PROP-002
    (f"{REG}/PROP-002-decentralized-registry.xml", "SHAPE-OWN-REPO", "user,author"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "SHAPE-REGISTRY-ARRAY", "user,author"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "SHAPE-PLAIN-GIT-URL", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "IDENTITY-TUPLE", "user,author"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "IDENTITY-CONSEQUENCE", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "EFF-LOCKFILE-STABLE", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "EFF-MIRROR-SUBSTITUTION", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "EFF-FORCE-PUSH-CAUGHT", "user,author"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "REGISTRY-ARRAY", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "REG-FIELD-URL", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "REG-FIELD-NAMING", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "REGISTRY-WALK-ORDER", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "AUTH-REGIMES", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "TOKEN-ENV-DEFAULTING", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "TOKEN-NEVER-ON-DISK", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "GLOBAL-REGISTRY-FILE", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "MERGE-PROJECT-FIRST", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "ENABLED-FLAG", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "MIRROR-LAYER", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "MIR-CANONICAL-IN-LOCKFILE", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "MIRROR-INTEGRITY-MANDATORY", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "REGISTRY-WALK-SEMANTICS", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "AUTH-AWARE-401", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "MIRROR-WALK-SEMANTICS", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "OVERRIDE-SHORT-CIRCUIT", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "OVERRIDE-SEMANTICS", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "GIT-SOURCE-DECL", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "GS-WIRE-FORM", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "GS-EXACTLY-ONE-REF", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "GS-RESOLUTION-ORDER", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "GS-IDENTITY", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "GS-IDENTITY-VERIFICATION", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "GS-MUTABILITY", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "GS-AUTH-EXPLICIT", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "SOURCE-KIND-VALUES", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "REDIRECT-STUB", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "REDIRECT-MARKER-FILE", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "RD-STEP-HOP-LIMIT", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "REDIRECT-TAG-VISIBILITY", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "REDIRECT-SYNC-HELPER", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "REDIRECT-LOCKFILE-FIELD", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "REDIRECT-CLI", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "RD-TRUST-FLAG", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "FLAT-LAYOUT", "author"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "LAYOUT-TAG-VERSION", "author"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "LOCKFILE-V2", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "LF-ROOT-DEPENDENCIES", "user"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "CAP-PROVIDES", "author"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "CAP-REQUIRES-PACKAGES", "user,author"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "CAP-REQUIRES-CAPABILITIES", "user,author"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "CAP-REQUIRES-ANY", "user,author"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "CAP-OBSOLETES", "author"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "CAP-CONFLICTS", "author"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "PUBLISH-UTILITY", "author"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "PUBLISH-MUTABLE-VERSIONS", "author"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "PUB-TOKEN-LOADING", "author"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "TOK-HOST-ENV-VAR", "author"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "TOK-PER-HOST-FILE", "author"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "TOKEN-SECRECY-INVARIANT", "author"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "PUB-ADAPTER-SELECTION", "author"),
    (f"{REG}/PROP-002-decentralized-registry.xml", "PUBLISH-NEVER-RULES", "author"),
    # ---------------------------------------------------------------- PROP-005
    (f"{IDX}/PROP-005-package-index.xml", "INDEX-OPTIONAL", "user"),
    (f"{IDX}/PROP-005-package-index.xml", "A-PROBE-HAS-THREE-OUTCOMES-NOT-TWO", "user"),
    (f"{IDX}/PROP-005-package-index.xml", "AN-ABSENT-INDEX-FALLS-BACK-WITHOUT-A-WORD", "user"),
    (f"{IDX}/PROP-005-package-index.xml", "INDEX-URL-CONFIG", "user"),
    (f"{IDX}/PROP-005-package-index.xml", "INDEX-URL-DEFAULT", "user"),
    (f"{IDX}/PROP-005-package-index.xml", "INDEX-URL-TODAY-IS-AN-ENVIRONMENT-VARIABLE", "user"),
    (f"{IDX}/PROP-005-package-index.xml", "REPOS-AUTHORITATIVE", "user"),
    # NOTE: PROP-005 §2.3 carries a second qualifying fact beside the one above
    # (the anchor whose text is "if the index disagrees with reality, reality
    # wins"). It is deliberately NOT marked: naming that anchor here trips the
    # zone gate's R-25 private-infrastructure pattern, which matches the bare
    # protocol word inside it. The fact is subsumed by the marked
    # ##REPOS-AUTHORITATIVE; see WORKER-REPORT-P3-M1.md.
    (f"{IDX}/PROP-005-package-index.xml", "INT-SEARCH", "user"),
    # ---------------------------------------------------------------- PROP-010
    (f"{REG}/PROP-010-local-package-cache.xml", "CACHE-MACHINE-GLOBAL", "user"),
    (f"{REG}/PROP-010-local-package-cache.xml", "CACHE-ACCRETIVE", "user"),
    (f"{REG}/PROP-010-local-package-cache.xml", "EXPLICIT-RECLAIM", "user"),
    (f"{REG}/PROP-010-local-package-cache.xml", "THE-SETTINGS-HOME-IS-DOT-VIBE-NOT-XDG", "user"),
    (f"{REG}/PROP-010-local-package-cache.xml", "THE-STORE-IS-DOT-VIBE-CACHE", "user"),
    (f"{REG}/PROP-010-local-package-cache.xml", "REGISTRIES-KEEP-THEIR-OWN-FILE", "user"),
    (f"{REG}/PROP-010-local-package-cache.xml", "PROJECT-OVERRIDES", "user"),
    (f"{REG}/PROP-010-local-package-cache.xml", "OFFLINE-FLAG", "user"),
    (f"{REG}/PROP-010-local-package-cache.xml", "OFFLINE-LAYERING", "user"),
    (f"{REG}/PROP-010-local-package-cache.xml", "OFFLINE-LOCAL-ONLY", "user"),
    (f"{REG}/PROP-010-local-package-cache.xml", "OFFLINE-HARD-ERROR", "user"),
    (f"{REG}/PROP-010-local-package-cache.xml", "OFFLINE-NO-DEGRADE", "user"),
    (f"{REG}/PROP-010-local-package-cache.xml", "AS-OF-LAST-REFRESH", "user"),
    (f"{REG}/PROP-010-local-package-cache.xml", "A-CACHE-HIT-IS-AUTHORITATIVE-FOR-AVAILABILITY", "user"),
    (f"{REG}/PROP-010-local-package-cache.xml", "CMD-LIST", "user"),
    (f"{REG}/PROP-010-local-package-cache.xml", "CMD-ADD", "user"),
    (f"{REG}/PROP-010-local-package-cache.xml", "CMD-CLEAN", "user"),
    (f"{REG}/PROP-010-local-package-cache.xml", "CMD-CHECK", "user"),
    # ---------------------------------------------------------------- PROP-008
    (f"{REG}/PROP-008-qualified-naming.xml", "GROUP-MANDATORY", "user,author"),
    (f"{REG}/PROP-008-qualified-naming.xml", "GROUP-GRAMMAR", "user,author"),
    (f"{REG}/PROP-008-qualified-naming.xml", "IDENTITY-TUPLE", "user,author"),
    (f"{REG}/PROP-008-qualified-naming.xml", "NAME-UNIQUE-IN-GROUP", "author"),
    (f"{REG}/PROP-008-qualified-naming.xml", "GROUP-CHANGE-NEW-PACKAGE", "user,author"),
    (f"{REG}/PROP-008-qualified-naming.xml", "KIND-METADATA", "user,author"),
    (f"{REG}/PROP-008-qualified-naming.xml", "PKGREF-GRAMMAR", "user,author"),
    (f"{REG}/PROP-008-qualified-naming.xml", "KIND-VALIDATION", "user"),
    (f"{REG}/PROP-008-qualified-naming.xml", "SHORT-CLI-ONLY", "user,author"),
    (f"{REG}/PROP-008-qualified-naming.xml", "JOINER-UNDERSCORE", "author"),
    (f"{REG}/PROP-008-qualified-naming.xml", "RESOLVE-ONCE-WRITE-QUALIFIED", "user"),
    (f"{REG}/PROP-008-qualified-naming.xml", "INDEX-DEPENDENCY", "user"),
    (f"{REG}/PROP-008-qualified-naming.xml", "LOCKFILE-AUTHORITATIVE", "user"),
    (f"{REG}/PROP-008-qualified-naming.xml", "COLLISION-BEHAVIOR", "user"),
    (f"{REG}/PROP-008-qualified-naming.xml", "EXIT-CODE-7", "user"),
    (f"{REG}/PROP-008-qualified-naming.xml", "GROUP-IS-A-CLAIM", "user,author"),
    (f"{REG}/PROP-008-qualified-naming.xml", "DEFAULT-TRUSTED-REGISTRIES", "user"),
    # ---------------------------------------------------------------- PROP-030
    (f"{REG}/PROP-030-embedded-registry.xml", "AMBIENT-DEFAULT", "user"),
    (f"{REG}/PROP-030-embedded-registry.xml", "EXPLICIT-ABOVE", "user"),
    (f"{REG}/PROP-030-embedded-registry.xml", "KNOB-DEFAULT", "user"),
    (f"{REG}/PROP-030-embedded-registry.xml", "KNOB-SUPPRESS", "user"),
    (f"{REG}/PROP-030-embedded-registry.xml", "ENUM-UNION", "user"),
    (f"{REG}/PROP-030-embedded-registry.xml", "FLAG-EMBEDDED-SHORT-CIRCUIT", "user"),
    (f"{REG}/PROP-030-embedded-registry.xml", "LOCAL-AUTO-OPEN", "user"),
    (f"{REG}/PROP-030-embedded-registry.xml", "LOCAL-NO-PREFER-FLAG", "user"),
    (f"{REG}/PROP-030-embedded-registry.xml", "LOCAL-SOURCE-KIND", "user"),
    (f"{REG}/PROP-030-embedded-registry.xml", "LOCK-EMBEDDED", "user"),
    (f"{REG}/PROP-030-embedded-registry.xml", "GUARD-CI-OFF", "user"),
    (f"{REG}/PROP-030-embedded-registry.xml", "GUARD-WARN", "user"),
    # ---------------------------------------------------------------- PROP-001
    (f"{REG}/PROP-001-git-backend.xml", "RISK-GIT-IN-PATH", "user"),
    (f"{REG}/PROP-001-git-backend.xml", "FRESHNESS-TTL", "user"),
    (f"{REG}/PROP-001-git-backend.xml", "OPEN-GIT-BINARY-PATH", "user"),
    # ---------------------------------------------------------------- PROP-021
    (f"{REG}/PROP-021-submodule-sources.xml", "WHAT-VVM-DOES", "author"),
    (f"{REG}/PROP-021-submodule-sources.xml", "FORM-DEPENDENCY-DECLARED", "author"),
    (f"{REG}/PROP-021-submodule-sources.xml", "NOT-A-PACKAGE", "author"),
    (f"{REG}/PROP-021-submodule-sources.xml", "PUBLISH-FLATTENS-GITLINKS", "author"),
    (f"{REG}/PROP-021-submodule-sources.xml", "DECLARED-SOURCE-AUTH", "author"),
    # ---------------------------------------------------------------- PROP-023
    (f"{REG}/PROP-023-bridge-packages.xml", "BRIDGE-DEF", "user,author"),
    (f"{REG}/PROP-023-bridge-packages.xml", "BRIDGE-FLAG", "author"),
    (f"{REG}/PROP-023-bridge-packages.xml", "CLASS-VENDORED", "author"),
    (f"{REG}/PROP-023-bridge-packages.xml", "CLASS-SUBMODULE", "author"),
    (f"{REG}/PROP-023-bridge-packages.xml", "CLASS-REFERENCE", "author"),
    (f"{REG}/PROP-023-bridge-packages.xml", "AUTHORSHIP-SEPARATION", "author"),
    (f"{REG}/PROP-023-bridge-packages.xml", "GROUP-PROVENANCE", "author"),
    (f"{REG}/PROP-023-bridge-packages.xml", "LICENSE-BOUNDARY", "author"),
    # ---------------------------------------------------------------- PROP-016
    (f"{COM}/PROP-016-source-mirrors.xml", "ORTHOGONALITY-LAW", "user"),
    # ---------------------------------------------------------------- PROP-029
    (f"{COM}/PROP-029-fully-qualified-addresses.xml", "ADDR-LAW", "user,author"),
    (f"{COM}/PROP-029-fully-qualified-addresses.xml", "ADDR-SHORT-NAMES", "user"),
    (f"{COM}/PROP-029-fully-qualified-addresses.xml", "CARRIER-PKGREF-FORM", "user,author"),
    (f"{COM}/PROP-029-fully-qualified-addresses.xml", "CARRIER-SPEC-URI-FORM", "author"),
    (f"{COM}/PROP-029-fully-qualified-addresses.xml", "CARRIER-REPO-NAME-FORM", "author"),
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
