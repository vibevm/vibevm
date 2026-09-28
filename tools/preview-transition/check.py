#!/usr/bin/env python3
"""Read-only CLI for the generated preview transition checker."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[2]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--transition", required=True)
    parser.add_argument("--mode", required=True)
    parser.add_argument("--stage")
    parser.add_argument("--format", default="json")
    args = parser.parse_args()

    request = {
        "schema": 1,
        "transition": str(Path(args.transition).resolve()),
        "mode": args.mode,
        "format": args.format,
    }
    if args.stage is not None:
        request["stage"] = args.stage

    command = [
        "cargo",
        "run",
        "--quiet",
        "-p",
        "vibe-wire",
        "--bin",
        "preview-transition-check",
    ]
    result = subprocess.run(
        command,
        cwd=ROOT,
        input=json.dumps(request, sort_keys=True).encode("utf-8"),
        capture_output=True,
        check=False,
    )
    sys.stdout.buffer.write(result.stdout)
    if result.stderr:
        sys.stderr.buffer.write(result.stderr)
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
