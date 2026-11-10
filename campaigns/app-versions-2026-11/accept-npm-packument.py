#!/usr/bin/env python3
"""One npm packument for the acceptance script's loopback npm registry.

`campaigns/app-versions-2026-11/accept.sh` leg 6 carries a **real** global npm
package from one Node version into another, which means npm has to be able to
reinstall it **by name**. The documented mirror this repository pins serves an
empty `npm-registry/` on purpose — it is digest-locked, so nothing may be
written into it — and so the acceptance serves a registry of its own holding
exactly one package with no dependencies.

This writes the one document that registry needs: the packument, named exactly
as npm asks for it (`<root>/<name>`), carrying the tarball's own sha1 so that
npm's own integrity check is what admits it rather than a flag.

usage: accept-npm-packument.py <registry-root> <registry-url>
"""

import hashlib
import json
import pathlib
import sys

NAME = "accept-npm-fixture"
VERSION = "1.0.0"


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    root = pathlib.Path(argv[1])
    url = argv[2].rstrip("/")
    tarball = root / f"{NAME}-{VERSION}.tgz"
    if not tarball.is_file():
        print(f"accept-npm-packument.py: no tarball at {tarball}", file=sys.stderr)
        return 2
    shasum = hashlib.sha1(tarball.read_bytes()).hexdigest()
    packument = {
        "_id": NAME,
        "name": NAME,
        "dist-tags": {"latest": VERSION},
        "versions": {
            VERSION: {
                "_id": f"{NAME}@{VERSION}",
                "name": NAME,
                "version": VERSION,
                "license": "UPL-1.0",
                "dist": {"shasum": shasum, "tarball": f"{url}/{tarball.name}"},
            }
        },
    }
    (root / NAME).write_text(json.dumps(packument, indent=2) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
