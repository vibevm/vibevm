#!/usr/bin/env python3
"""Resolve the steward context of a repository; read-only.

Prints one JSON object: `status` (`found` | `ambiguous` | `none` | `error`),
`context_id`, `context_dir`, `candidates` and diagnostics. Exit 0 found, 1 none,
2 ambiguous, 3 error. It never creates, repairs or rebinds anything; creating a
context and writing a binding are separate explicit actions.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import runpy
import sys


HOME_MODULE = Path(__file__).resolve().parent / "steward_home.py"
EXIT = {"found": 0, "none": 1, "ambiguous": 2}


def load_home_module() -> dict:
    return runpy.run_path(str(HOME_MODULE))


def main(argv: list[str] | None = None) -> int:
    class JsonErrors(argparse.ArgumentParser):
        def error(self, message):
            print(json.dumps({"status": "error", "code": "ARGUMENTS_INVALID", "message": message}))
            raise SystemExit(3)

    parser = JsonErrors(description=__doc__)
    parser.add_argument("--home", type=Path, help="steward home; default <settings dir>/steward")
    parser.add_argument("--repo", type=Path, default=None, help="checkout to resolve; default: current directory")
    parser.add_argument("--context-id", help="an opaque context id named by the owner")
    args = parser.parse_args(argv)
    steward = load_home_module()
    try:
        home = steward["steward_home"](args.home)
        result = steward["resolve"](home, args.repo, context_id=args.context_id)
    except steward["StewardError"] as exc:
        print(json.dumps({"status": "error", "code": exc.code, "message": str(exc)}, ensure_ascii=False))
        return 3
    print(json.dumps({"home": str(home), **result.as_json()}, ensure_ascii=False, sort_keys=True))
    return EXIT[result.status]


if __name__ == "__main__":
    sys.exit(main())
