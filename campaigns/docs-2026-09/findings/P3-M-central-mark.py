#!/usr/bin/env python3
"""A3.13, second pass of the central session — two obligations the workers
left out for reasons that were not the criterion: `REALITY-WINS` because the
zone gate of the day matched its name, `LAW-LIFECYCLE` because the worker's
document had passed the size corridor.

Same edit as the P3-M<n> scripts: insert the literal string

    ` action="continue" actionstage="doc" audience="<audiences>"`

into the opening tag of a named fact, right after its `status="…"`
attribute, and nothing else. Refuses unless every anchor is found exactly
once and carries no `action=` yet; re-parses every touched file afterwards.

Run from the repository root:  python campaigns/docs-2026-09/findings/P3-M-central-mark.py
"""

from __future__ import annotations

import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

PLAN: list[tuple[str, str, str, str]] = [
    (
        "vibevm/vibespecs/modules/vibe-index/PROP-005-package-index.xml",
        "REALITY-WINS",
        "user",
        "an index that disagrees with the repository loses; the user relies on it",
    ),
    (
        "vibevm/vibespecs/common/PROP-046-adoption-facts-registry.xml",
        "LAW-LIFECYCLE",
        "user",
        "uninstall keeps the adoption file; the user sees the clean dialog",
    ),
]


def main() -> int:
    originals: dict[Path, str] = {}
    for file, anchor, audiences, _why in PLAN:
        path = Path(file)
        text = originals.get(path) or path.read_text(encoding="utf-8")
        pattern = re.compile(rf'<{re.escape(anchor)} fact="true" status="[^"]*"')
        hits = pattern.findall(text)
        if len(hits) != 1:
            print(f"{file}#{anchor}: expected exactly one fact tag, found {len(hits)}", file=sys.stderr)
            return 1
        start = text.index(hits[0])
        end = text.index(">", start)
        if "action=" in text[start:end]:
            print(f"{file}#{anchor}: already carries an action marker", file=sys.stderr)
            return 1
        insert = f' action="continue" actionstage="doc" audience="{audiences}"'
        originals.setdefault(path, path.read_text(encoding="utf-8"))
        text = text[: start + len(hits[0])] + insert + text[start + len(hits[0]) :]
        path.write_text(text, encoding="utf-8", newline="\n")
        originals[path] = text
    for path in originals:
        try:
            ET.fromstring(path.read_bytes())
        except ET.ParseError as error:
            print(f"{path}: not well-formed after the edit: {error}", file=sys.stderr)
            return 1
    print(f"marked {len(PLAN)} obligations")
    return 0


if __name__ == "__main__":
    sys.exit(main())
