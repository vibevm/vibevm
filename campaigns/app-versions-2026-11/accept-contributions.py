#!/usr/bin/env python3
"""The `slot:` contribution rows of an install's own documents.

`campaigns/app-versions-2026-11/accept.sh` leg 13 asks what an ordinary
package's judged `slot:post-install` contribution did. A project install emits
several JSON documents on one stream, so this reads them all and prints one
tuple per contribution row — point, handler kind, status — which the leg then
matches as a string.

usage: accept-contributions.py <log-file>
"""

import json
import pathlib
import sys


def documents(text: str):
    """Every top-level JSON object of a stream that carries several."""
    decoder = json.JSONDecoder()
    at = 0
    while at < len(text):
        while at < len(text) and text[at] not in "{[":
            at += 1
        if at >= len(text):
            return
        try:
            value, end = decoder.raw_decode(text, at)
        except ValueError:
            at += 1
            continue
        yield value
        at = end


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    text = pathlib.Path(argv[1]).read_text(errors="replace")
    rows = [
        (row.get("point"), row.get("handler"), row.get("status"))
        for document in documents(text)
        if isinstance(document, dict)
        for row in document.get("contributions", [])
        if isinstance(row, dict)
    ]
    print(rows)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
