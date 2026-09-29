#!/usr/bin/env python3
"""Read-only observations of an existing schema-1 plan's parallel frontier.

Candidate batches describe only observed path/resource compatibility. They are
neither a scheduler nor authority to dispatch; semantic, verification, resource
capacity and actual live-worker bindings remain the coordinator's responsibility.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PureWindowsPath
import re
import runpy
import sys
import tomllib
from typing import Any


SCHEMA = 1
TERMINAL = {"accepted", "deferred", "dropped", "superseded"}
WAIVED = {"deferred", "dropped", "superseded"}
SUPPRESSING_ANCESTOR_STATES = {"blocked", "deferred", "dropped", "superseded"}
RENDERER = Path(__file__).resolve().parents[2] / "steward-goal/scripts/render_goal.py"


class FrontierError(ValueError):
    """An invalid input, never an empty or permissive frontier."""

    def __init__(self, code: str, message: str):
        super().__init__(message)
        self.code = code


def require(condition: bool, code: str, message: str) -> None:
    if not condition:
        raise FrontierError(code, message)


def effective_dependencies(by_id: dict, key: str) -> list[str]:
    result = []
    while key:
        result.extend(by_id[key]["depends_on"])
        key = by_id[key]["parent"]
    return list(dict.fromkeys(result))


def ancestor_constraints(by_id: dict, key: str) -> list[dict]:
    """Keep suppressed subtrees out of new dispatch without hiding their work."""
    result = []
    key = by_id[key]["parent"]
    while key:
        if by_id[key]["state"] in SUPPRESSING_ANCESTOR_STATES:
            result.append({"id": key, "state": by_id[key]["state"]})
        key = by_id[key]["parent"]
    return result


def validate_plan(plan: dict) -> tuple[dict, dict, dict]:
    require(type(plan.get("schema")) is int and plan["schema"] == SCHEMA,
            "PLAN_INVALID", "plan schema must be integer 1")
    renderer = runpy.run_path(str(RENDERER))
    try:
        _, _, by_id = renderer["validate_plan"](plan)
    except renderer["GoalError"] as exc:
        raise FrontierError("PLAN_INVALID", str(exc)) from exc
    root = plan["root_node"]
    require(not by_id[root]["parent"], "PLAN_INVALID", "root must have no parent")
    children = {key: [] for key in by_id}
    for key, node in by_id.items():
        if node["parent"]:
            children[node["parent"]].append(key)
        cursor = key
        while cursor != root:
            cursor = by_id[cursor]["parent"]
            require(cursor in by_id, "PLAN_INVALID", f"node outside root: {key}")
    dependencies = {key: effective_dependencies(by_id, key) for key in by_id}
    completion = {
        key: deps + [child for child in children[key] if by_id[child]["state"] not in WAIVED]
        for key, deps in dependencies.items()
    }
    try:
        renderer["detect_cycle"](completion, "completion/dependency")
    except renderer["GoalError"] as exc:
        raise FrontierError("PLAN_INVALID", str(exc)) from exc
    for key, node in by_id.items():
        if node["state"] == "accepted":
            require(bool(node["evidence"]), "PLAN_INVALID", f"accepted without evidence: {key}")
            required = dependencies[key] + children[key]
            require(all(by_id[dep]["state"] in TERMINAL for dep in required),
                    "PLAN_INVALID", f"accepted with open prerequisites/children: {key}")
    return by_id, children, dependencies


def path_key(raw: str, repo_root: Path) -> tuple[str, ...]:
    """Normalize aliases and follow existing reparse points without creating paths.

    Every path is a conservative subtree perimeter, including nonexistent paths.
    Unsupported/glob/external paths fail closed instead of proving disjointness.
    """
    require(isinstance(raw, str) and bool(raw), "PATH_UNKNOWN", "empty/non-string path")
    portable = raw.replace("\\", "/")
    windows = PureWindowsPath(portable)
    require(not portable.startswith("/") and not windows.drive,
            "PATH_UNKNOWN", "absolute paths are outside repository-relative scope")
    require(not any(character in portable for character in "*?[]:\x00"),
            "PATH_UNKNOWN", "wildcards, streams or unsupported path syntax")
    components: list[str] = []
    for part in portable.split("/"):
        if part in {"", "."}:
            continue
        if part == "..":
            require(bool(components), "PATH_UNKNOWN", "path escapes repository")
            components.pop()
        else:
            if os.name == "nt":
                require(not part.endswith((".", " ")) and not PureWindowsPath(part).is_reserved(),
                        "PATH_UNKNOWN", "ambiguous Windows path component")
            components.append(part)
    try:
        # Preserve '..' until resolution: a reparse point before it can change
        # the actual target. Lexical containment alone cannot prove isolation.
        resolved = repo_root.joinpath(*portable.split("/")).resolve(strict=False)
        root_parts = tuple(os.path.normcase(part) for part in repo_root.parts)
        parts = tuple(os.path.normcase(part) for part in resolved.parts)
        require(parts[:len(root_parts)] == root_parts,
                "PATH_UNKNOWN", "resolved path escapes repository")
        return parts[len(root_parts):]
    except (OSError, RuntimeError) as exc:
        raise FrontierError("PATH_UNKNOWN", "cannot resolve path aliases") from exc


def display_path(key: tuple[str, ...]) -> str:
    return "/".join(key) or "."


def overlaps(left: tuple, right: tuple) -> bool:
    return left[:len(right)] == right or right[:len(left)] == left


def path_list(row: dict, name: str, repo: Path) -> list[tuple]:
    raw = row.get(name)
    require(isinstance(raw, list) and all(isinstance(item, str) for item in raw),
            "PATH_UNKNOWN", f"{name} is missing or not a string array")
    return sorted(set(path_key(item, repo) for item in raw))


def declared_scope(task: Any, repo: Path) -> dict:
    result = {"origin": "unknown", "reads": [], "writes": [], "resources": None,
              "issues": []}
    if not isinstance(task, dict):
        result["issues"].append("TASK_METADATA_MISSING")
        return result
    try:
        result["reads"] = path_list(task, "read_paths", repo)
        result["writes"] = path_list(task, "write_paths", repo)
        result["origin"] = "declared"
    except FrontierError as exc:
        result["issues"].append(exc.code)
    return result


def bind_scopes(bindings: dict | None, scopes: dict, plan: dict,
                repo: Path, plan_sha256: str | None) -> None:
    if bindings is None:
        return
    require(isinstance(bindings, dict) and type(bindings.get("schema")) is int
            and bindings["schema"] == SCHEMA, "BINDINGS_INVALID", "binding schema must be 1")
    require(bindings.get("plan_id") == plan["plan_id"]
            and type(bindings.get("plan_revision")) is int
            and bindings["plan_revision"] == plan["revision"],
            "BINDINGS_STALE", "binding plan identity/revision differs")
    require(isinstance(plan_sha256, str) and re.fullmatch(r"[0-9a-f]{64}", plan_sha256) is not None
            and bindings.get("plan_sha256") == plan_sha256,
            "BINDINGS_STALE", "binding must match the supplied hash of raw plan bytes")
    rows = bindings.get("tasks")
    require(isinstance(rows, list), "BINDINGS_INVALID", "binding tasks must be an array")
    seen = set()
    for row in rows:
        require(isinstance(row, dict), "BINDINGS_INVALID", "binding task must be an object")
        key = row.get("id")
        require(isinstance(key, str) and key in scopes and key not in seen,
                "BINDINGS_INVALID", "unknown or duplicate binding task id")
        seen.add(key)
        outer = scopes[key]
        require(outer["origin"] == "declared", "BINDINGS_INVALID",
                f"cannot narrow an unknown declared perimeter: {key}")
        try:
            reads = path_list(row, "read_paths", repo)
            writes = path_list(row, "write_paths", repo)
        except FrontierError as exc:
            raise FrontierError("BINDINGS_INVALID", f"invalid bound paths: {key}") from exc
        for actual, permitted in ((reads, outer["reads"] + outer["writes"]),
                                  (writes, outer["writes"])):
            require(all(any(path[:len(bound)] == bound for bound in permitted) for path in actual),
                    "BINDINGS_INVALID", f"bound perimeter exceeds declared paths: {key}")
        resources = row.get("resources")
        if resources is not None:
            require(isinstance(resources, list), "BINDINGS_INVALID", "resources must be an array")
            resource_ids = set()
            for resource in resources:
                require(isinstance(resource, dict) and isinstance(resource.get("id"), str)
                        and bool(resource["id"].strip()) and resource["id"] not in resource_ids
                        and isinstance(resource.get("mode"), str)
                        and resource["mode"] in {"shared", "exclusive"},
                        "BINDINGS_INVALID", "invalid/duplicate resource demand")
                resource_ids.add(resource["id"])
            resources = sorted((dict(id=item["id"], mode=item["mode"]) for item in resources),
                               key=lambda item: item["id"])
        scopes[key] = {"origin": "bound", "reads": reads, "writes": writes,
                       "resources": resources, "issues": []}


def pair_conflicts(left_id: str, left: dict, right_id: str, right: dict) -> list[dict]:
    """Report conservative conflicts on two normalized scope observations."""
    base = {"left": left_id, "right": right_id}
    if left["origin"] == "unknown" or right["origin"] == "unknown":
        return [dict(base, code="PERIMETER_UNKNOWN")]
    result = []
    for code, a_paths, b_paths in (
        ("WRITE_WRITE", left["writes"], right["writes"]),
        ("WRITE_READ", left["writes"], right["reads"]),
        ("READ_WRITE", left["reads"], right["writes"]),
    ):
        hits = [{"left_path": display_path(a), "right_path": display_path(b)}
                for a in a_paths for b in b_paths if overlaps(a, b)]
        if hits:
            result.append(dict(base, code=code, paths=hits))
    a_resources = {item["id"]: item["mode"] for item in left["resources"] or []}
    b_resources = {item["id"]: item["mode"] for item in right["resources"] or []}
    exclusive = sorted(key for key in a_resources.keys() & b_resources.keys()
                       if "exclusive" in {a_resources[key], b_resources[key]})
    if exclusive:
        result.append(dict(base, code="RESOURCE_EXCLUSIVE", resources=exclusive))
    return result


def public_scope(key: str, scope: dict) -> dict:
    resources = scope["resources"]
    return {
        "id": key,
        "perimeter_origin": scope["origin"],
        "read_paths": [display_path(path) for path in scope["reads"]],
        "write_paths": [display_path(path) for path in scope["writes"]],
        "issues": scope["issues"],
        "resources": resources,
        "perimeter_binding_required": scope["origin"] != "bound",
        "semantic_binding_required": True,
        "verification_binding_required": True,
        "resource_binding_required": resources is None,
        "resource_capacity_unknown": resources is None or bool(resources),
        "source_capture_verification_required": True,
        "live_jobs_unverified": True,
    }


def analyze(plan: dict, tasks: dict, repo_root: str | Path, bindings: dict | None = None,
            *, plan_sha256: str | None = None) -> dict:
    """Derive observations without changing inputs or authorizing execution.

    tasks maps existing node ids to metadata with read_paths/write_paths. Missing
    metadata is unknown. Binding hashes refer to raw TOML bytes, not reserialization.
    No worker status, Git state, capacity or authority is inferred from this view.
    This advisory view also excludes suppressed ancestor subtrees; it can be more
    conservative than a legacy ready-nodes projection that checks only dependencies.
    """
    require(isinstance(plan, dict), "PLAN_INVALID", "plan must be an object")
    by_id, children, dependencies = validate_plan(plan)
    require(isinstance(tasks, dict) and all(key in by_id for key in tasks),
            "TASKS_INVALID", "task metadata ids must name existing nodes")
    repo = Path(repo_root).resolve()
    require(repo.is_dir(), "REPO_INVALID", "repository root must be an existing directory")
    ordered = sorted(by_id, key=lambda key: (by_id[key]["order"], key))
    constraints = {key: ancestor_constraints(by_id, key) for key in by_id}
    ready, active, review, blocked = [], [], [], []
    for key in ordered:
        node = by_id[key]
        if node["state"] == "candidate":
            review.append(key)
        if children[key]:
            continue
        unmet = [dep for dep in dependencies[key] if by_id[dep]["state"] not in TERMINAL]
        if node["state"] == "active":
            active.append(key)
        elif node["state"] in {"planned", "ready"} and not unmet and not constraints[key]:
            ready.append(key)
        elif node["state"] in {"planned", "ready", "blocked"}:
            reasons = (["DEPENDENCIES_UNSATISFIED"] if unmet else [])
            if node["state"] == "blocked":
                reasons.append("NODE_BLOCKED")
            if constraints[key]:
                reasons.append("ANCESTOR_STATE_SUPPRESSES_DISPATCH")
            blocked.append({"id": key, "state": node["state"], "unsatisfied_dependencies": unmet,
                            "ancestor_constraints": constraints[key], "reasons": reasons})
    needed = set(ready + active)
    if isinstance(bindings, dict) and isinstance(bindings.get("tasks"), list):
        needed.update(row["id"] for row in bindings["tasks"]
                      if isinstance(row, dict) and isinstance(row.get("id"), str)
                      and row["id"] in by_id)
    scopes = {key: declared_scope(tasks.get(key) if key in needed else None, repo)
              for key in by_id}
    bind_scopes(bindings, scopes, plan, repo, plan_sha256)
    conflicts, collisions, blocked_by_active = [], set(), set()
    for index, left in enumerate(ready):
        for right in ready[index + 1:] + active:
            observed = pair_conflicts(left, scopes[left], right, scopes[right])
            for item in observed:
                item["right_state"] = by_id[right]["state"]
            conflicts.extend(observed)
            if observed:
                collisions.add(frozenset((left, right)))
                if right in active:
                    blocked_by_active.add(left)
    batches: list[list[str]] = []
    excluded = []
    for key in ready:
        reasons = []
        if scopes[key]["origin"] == "unknown":
            reasons.append("PERIMETER_UNKNOWN")
        if key in blocked_by_active:
            reasons.append("ACTIVE_CONFLICT")
        if reasons:
            excluded.append({"id": key, "reasons": reasons})
            continue
        for batch in batches:
            if all(frozenset((key, peer)) not in collisions for peer in batch):
                batch.append(key)
                break
        else:
            batches.append([key])
    return {
        "schema": SCHEMA,
        "plan_id": plan["plan_id"], "plan_revision": plan["revision"],
        "plan_sha256": plan_sha256,
        "policy_state": "requires-binding", "dispatch_authorized": False,
        "observation_scope": "declared-or-bound-paths-and-resources-only",
        "runtime_observed": False,
        "batch_order_is_execution_order": False,
        "counts": {"nodes": len(by_id), "ready": len(ready), "active": len(active),
                   "review": len(review), "blocked": len(blocked),
                   "proposed_batches": len(batches)},
        "ready": [public_scope(key, scopes[key]) for key in ready],
        "active": [dict(public_scope(key, scopes[key]),
                        unsatisfied_dependencies=[dep for dep in dependencies[key]
                                                  if by_id[dep]["state"] not in TERMINAL],
                        ancestor_constraints=constraints[key],
                        reasons=["ANCESTOR_STATE_SUPPRESSES_DISPATCH"] if constraints[key] else [])
                   for key in active],
        "review_queue": review,
        "review_before_expanding": bool(review),
        "blocked": blocked,
        "conflicts": conflicts,
        "excluded_from_batches": excluded,
        "proposed_batches": batches,
        "limitations": [
            "Dependency eligibility is not current owner execution authority.",
            "Declared paths are outer bounds; bound paths still need semantic and actual input review.",
            "Batches have no observed path/exclusive-resource collision, not proven independence.",
            "Active nodes are plan observations, not live-worker status or ownership reservations.",
            "Suppressed ancestor states exclude new dispatch; surviving active work and candidates remain visible.",
            "Unknown or shared resource capacity requires observation; no concurrency limit is inferred.",
            "External documentation and service scopes need explicit intake and coordination.",
        ],
    }


def main(argv: list[str] | None = None) -> int:
    class JsonErrors(argparse.ArgumentParser):
        def error(self, message):
            raise FrontierError("ARGUMENTS_INVALID", message)

    parser = JsonErrors(description=__doc__)
    parser.add_argument("--plan", type=Path, required=True)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--task-file", type=Path, action="append", default=[])
    parser.add_argument("--bindings", type=Path)
    try:
        args = parser.parse_args(argv)
        raw = args.plan.read_bytes()
        plan = tomllib.loads(raw.decode("utf-8"))
        tasks = {}
        for path in args.task_file:
            group = json.loads(path.read_text(encoding="utf-8"))
            require(isinstance(group, dict) and isinstance(group.get("tasks"), list),
                    "TASKS_INVALID", "task file must contain a tasks array")
            for task in group["tasks"]:
                require(isinstance(task, dict) and isinstance(task.get("id"), str)
                        and task["id"] not in tasks, "TASKS_INVALID", "invalid/duplicate task metadata")
                tasks[task["id"]] = task
        bindings = json.loads(args.bindings.read_text(encoding="utf-8")) if args.bindings else None
        result = analyze(plan, tasks, args.repo, bindings, plan_sha256=hashlib.sha256(raw).hexdigest())
    except (OSError, UnicodeError, ValueError, RuntimeError) as exc:
        result = {"schema": SCHEMA, "dispatch_authorized": False,
                  "error": {"code": getattr(exc, "code", "INPUT_INVALID"), "message": str(exc)}}
        print(json.dumps(result, ensure_ascii=True, sort_keys=True))
        return 2
    # ASCII-escaped JSON preserves Unicode values even through a legacy console
    # code page; callers decode the same strings without a locale-dependent crash.
    print(json.dumps(result, ensure_ascii=True, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
