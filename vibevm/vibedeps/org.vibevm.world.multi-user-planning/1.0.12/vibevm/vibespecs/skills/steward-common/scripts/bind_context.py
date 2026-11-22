#!/usr/bin/env python3
"""Write a schema-2 `binding.toml` for an existing context, on explicit request.

A schema-1 binding is converted: `context_id` and `project_name` are kept,
`repository_id` is computed from `--repo`, and the old path tuple becomes one
advisory `[[seen]]` entry. At most one context per repository holds
`default = true`; another holder is refused unless `--take-default` moves the
default here. The steward home is the one that holds `--context` in its
`contexts/` directory; a directory anywhere else is refused. Prints one JSON object; exit 0 written, 2 refused.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import runpy
import sys


HOME_MODULE = Path(__file__).resolve().parent / "steward_home.py"


def bind(steward: dict, args: argparse.Namespace) -> dict:
    error = steward["StewardError"]
    context = Path(args.context).expanduser().resolve()
    if not context.is_dir():
        raise error("CONTEXT_NOT_FOUND", f"context directory not found: {context}")
    if context.parent.name != "contexts":
        raise error("CONTEXT_NOT_FOUND",
                    f"context directory is not inside a steward home's contexts/ directory: {context}")
    if not steward["CONTEXT_ID_RE"].match(context.name):
        raise error("CONTEXT_ID_INVALID", f"context directory is not named by an opaque id: {context.name}")
    path = context / steward["BINDING_NAME"]
    existing = steward["read_binding"](context) if path.exists() else None
    repository = steward["Repository"](Path(args.repo).expanduser())
    if existing is not None and existing.schema == 2 and existing.repository_id != repository.repository_id:
        raise error("BINDING_REPOSITORY_CHANGED",
                    f"{context.name} is bound to repository {existing.repository_id}, not {repository.repository_id}")

    schema2 = existing is not None and existing.schema == 2
    if args.project_name:
        project_name = args.project_name
    elif existing is not None and existing.project_name:
        project_name = existing.project_name
    else:
        project_name = Path(repository.repository_root).name
    default = bool(args.default or args.take_default or (schema2 and existing.default))
    if args.revision:
        revisions = list(dict.fromkeys(args.revision))
    else:
        revisions = list(existing.revisions) if schema2 else []
    if schema2:
        seen = list(existing.seen)
    elif existing is not None:
        seen = [{key: existing.data[key] for key in steward["SCHEMA1_TUPLE"]}]
    else:
        seen = []

    holders = []
    if default:
        bindings, _ = steward["scan_bindings"](context.parent.parent)
        holders = [b for b in bindings if b.schema == 2 and b.default
                   and b.repository_id == repository.repository_id and b.context_id != context.name]
        if holders and not args.take_default:
            names = ", ".join(b.context_id for b in holders)
            raise error("DEFAULT_HELD", f"context {names} already holds default = true for this repository; "
                                        "pass --take-default to move it")

    text = steward["binding_text"]({
        "schema": 2,
        "context_id": context.name,
        "project_name": project_name,
        "repository_id": repository.repository_id,
        "default": default,
        "revisions": revisions,
        "seen": seen,
    })
    steward["atomic_write"](path, text)
    for holder in holders:
        steward["release_default"](holder)
    return {
        "ok": True,
        "context_id": context.name,
        "binding": str(path),
        "converted_from_schema": existing.schema if existing is not None else None,
        "repository_id": repository.repository_id,
        "default": default,
        "revisions": revisions,
        "released_default": [holder.context_id for holder in holders],
    }


def main(argv: list[str] | None = None) -> int:
    class JsonErrors(argparse.ArgumentParser):
        def error(self, message):
            print(json.dumps({"ok": False, "code": "ARGUMENTS_INVALID", "message": message}))
            raise SystemExit(2)

    parser = JsonErrors(description=__doc__)
    parser.add_argument("--context", type=Path, required=True, help="existing context directory")
    parser.add_argument("--repo", type=Path, required=True, help="a checkout of the repository to bind")
    parser.add_argument("--default", action="store_true", help="make this the repository's default context")
    parser.add_argument("--take-default", action="store_true",
                        help="move default = true here from another context of the same repository")
    parser.add_argument("--revision", action="append", default=[],
                        help="branch selector this context serves; repeatable; replaces the stored list")
    parser.add_argument("--project-name", help="project name; default: kept, else the repository directory name")
    args = parser.parse_args(argv)
    steward = runpy.run_path(str(HOME_MODULE))
    try:
        result = bind(steward, args)
    except steward["StewardError"] as exc:
        print(json.dumps({"ok": False, "code": exc.code, "message": str(exc)}, ensure_ascii=False))
        return 2
    except OSError as exc:
        print(json.dumps({"ok": False, "code": "BINDING_UNWRITABLE", "message": str(exc)}, ensure_ascii=False))
        return 2
    print(json.dumps(result, ensure_ascii=False, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
