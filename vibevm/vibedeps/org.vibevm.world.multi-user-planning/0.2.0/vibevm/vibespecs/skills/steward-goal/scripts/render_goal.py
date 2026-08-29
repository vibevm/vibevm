#!/usr/bin/env python3
"""Deterministic reference renderer for multi-user-planning GOAL files."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import tempfile
import tomllib
from typing import Any


SCHEMA = 1
CLAUDE_CONDITION_MAX_UTF16 = 4000
TERMINAL_SELECTION_STATES = {"accepted", "dropped", "superseded", "deferred"}
ROUTE_TERMINAL_STATES = {"accepted", "dropped", "superseded", "deferred"}
ALLOWED_STATES = {
    "planned",
    "ready",
    "active",
    "candidate",
    "accepted",
    "blocked",
    "deferred",
    "dropped",
    "superseded",
}
ALLOWED_KINDS = {
    "portfolio",
    "campaign",
    "phase",
    "workstream",
    "group",
    "atom",
    "gate",
    "horizon",
}
ID_RE = re.compile(r"^[A-Za-z0-9._:-]+$")
MARKER_RE = re.compile(
    r'^<!-- steward-goal schema=(?P<schema>\d+) '
    r'plan_id="(?P<plan_id>[^"]+)" '
    r'plan_revision=(?P<revision>\d+) '
    r'goal_node="(?P<goal_node>[^"]+)" '
    r'planning_profile="(?P<profile>[^"]+)" '
    r'plan_sha256="(?P<plan_sha>[0-9a-f]{64})" '
    r'claude_condition_sha256="(?P<condition_sha>[0-9a-f]{64}|unavailable)" -->$'
)

PROFILE_TEXT = {
    "ultra": (
        "Use extended reasoning across the full route; larger coherent atoms are "
        "allowed only while review and evidence remain bounded. The completeness, "
        "review and gate floor is unchanged."
    ),
    "standard": (
        "Use smaller atoms, explicit per-claim evidence and a backscan after every "
        "completed parent. The completeness, review and gate floor is unchanged."
    ),
}
CONTINUITY_TEXT = (
    "After compact or resume, re-read the complete plan and refresh this goal if "
    "stale. Continue across accepted intermediate atoms; a short UI plan, context "
    "loss, difficulty, or worker termination is not completion. Stop only for an "
    "owner-only decision, an external blocker, or a central custody handoff. "
    "Declare completion only when every completion proof below is satisfied."
)
CLAUDE_CONTINUITY = (
    "After compact/resume re-read plan.toml and GOAL.md; do not stop at a short UI "
    "plan, an intermediate atom, difficulty or a worker/process ending; stop only "
    "for an owner-only decision, external blocker or central handoff; the goal is "
    "met only when the selected node is accepted with all required descendants/"
    "gates and owner inspection."
)


class GoalError(RuntimeError):
    """A typed projection refusal."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def utf16_units(text: str) -> int:
    return len(text.encode("utf-16-le")) // 2


def collapse_ws(text: str) -> str:
    return " ".join(str(text).split())


def md_text(text: str) -> str:
    return str(text).replace("\r\n", "\n").replace("\r", "\n").replace("\n", "\n  ")


def read_bytes(path: Path, code: str) -> bytes:
    try:
        return path.read_bytes()
    except OSError as exc:
        raise GoalError(code, f"cannot read {path}: {exc}") from exc


def parse_toml(raw: bytes, path: Path) -> dict[str, Any]:
    try:
        return tomllib.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, tomllib.TOMLDecodeError) as exc:
        raise GoalError("GOAL_PLAN_INVALID", f"invalid UTF-8 TOML {path}: {exc}") from exc


def require_str(row: dict[str, Any], key: str, owner: str) -> str:
    value = row.get(key)
    if not isinstance(value, str):
        raise GoalError("GOAL_PLAN_INVALID", f"{owner}.{key} must be a string")
    return value


def require_str_list(row: dict[str, Any], key: str, owner: str) -> list[str]:
    value = row.get(key)
    if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
        raise GoalError("GOAL_PLAN_INVALID", f"{owner}.{key} must be a string array")
    return value


def detect_cycle(edges: dict[str, list[str]], label: str) -> None:
    visiting: set[str] = set()
    visited: set[str] = set()

    def walk(node_id: str) -> None:
        if node_id in visiting:
            raise GoalError("GOAL_PLAN_INVALID", f"{label} cycle reaches {node_id}")
        if node_id in visited:
            return
        visiting.add(node_id)
        for target in edges.get(node_id, []):
            walk(target)
        visiting.remove(node_id)
        visited.add(node_id)

    for candidate in edges:
        walk(candidate)


def validate_plan(plan: dict[str, Any]) -> tuple[list[dict[str, Any]], list[dict[str, Any]], dict[str, dict[str, Any]]]:
    if plan.get("schema") != 1:
        raise GoalError("GOAL_PLAN_INVALID", "plan schema must be 1")
    plan_id = require_str(plan, "plan_id", "plan")
    if not ID_RE.fullmatch(plan_id):
        raise GoalError("GOAL_PLAN_INVALID", "plan_id is not marker-safe")
    revision = plan.get("revision")
    if not isinstance(revision, int) or isinstance(revision, bool) or revision < 0:
        raise GoalError("GOAL_PLAN_INVALID", "plan.revision must be a non-negative integer")
    mandates = plan.get("mandate")
    nodes = plan.get("node")
    if not isinstance(mandates, list) or not all(isinstance(row, dict) for row in mandates):
        raise GoalError("GOAL_PLAN_INVALID", "plan.mandate must be an array of tables")
    if not isinstance(nodes, list) or not all(isinstance(row, dict) for row in nodes):
        raise GoalError("GOAL_PLAN_INVALID", "plan.node must be an array of tables")

    mandate_ids: set[str] = set()
    for row in mandates:
        mid = require_str(row, "id", "mandate")
        if not ID_RE.fullmatch(mid) or mid in mandate_ids:
            raise GoalError("GOAL_PLAN_INVALID", f"invalid or duplicate mandate id {mid!r}")
        mandate_ids.add(mid)
        require_str(row, "text", mid)
        require_str(row, "disposition", mid)
        require_str_list(row, "nodes", mid)

    by_id: dict[str, dict[str, Any]] = {}
    for row in nodes:
        node_id = require_str(row, "id", "node")
        if not ID_RE.fullmatch(node_id) or node_id in by_id:
            raise GoalError("GOAL_PLAN_INVALID", f"invalid or duplicate node id {node_id!r}")
        by_id[node_id] = row

    parent_edges: dict[str, list[str]] = {}
    dep_edges: dict[str, list[str]] = {}
    for node_id, row in by_id.items():
        parent = require_str(row, "parent", node_id)
        if parent and parent not in by_id:
            raise GoalError("GOAL_PLAN_INVALID", f"{node_id} has missing parent {parent}")
        parent_edges[node_id] = [parent] if parent else []
        deps = require_str_list(row, "depends_on", node_id)
        missing_deps = [dep for dep in deps if dep not in by_id]
        if missing_deps:
            raise GoalError("GOAL_PLAN_INVALID", f"{node_id} has missing dependencies {missing_deps}")
        dep_edges[node_id] = deps
        refs = require_str_list(row, "mandates", node_id)
        missing_mandates = [mid for mid in refs if mid not in mandate_ids]
        if missing_mandates:
            raise GoalError("GOAL_PLAN_INVALID", f"{node_id} has missing mandates {missing_mandates}")
        require_str_list(row, "acceptance", node_id)
        require_str_list(row, "evidence", node_id)
        require_str(row, "title", node_id)
        state = require_str(row, "state", node_id)
        kind = require_str(row, "kind", node_id)
        if state not in ALLOWED_STATES:
            raise GoalError("GOAL_PLAN_INVALID", f"{node_id} has unknown state {state}")
        if kind not in ALLOWED_KINDS:
            raise GoalError("GOAL_PLAN_INVALID", f"{node_id} has unknown kind {kind}")
        order = row.get("order")
        if not isinstance(order, int) or isinstance(order, bool):
            raise GoalError("GOAL_PLAN_INVALID", f"{node_id}.order must be an integer")

    detect_cycle(parent_edges, "parent")
    detect_cycle(dep_edges, "dependency")
    root_node = require_str(plan, "root_node", "plan")
    current_node = require_str(plan, "current_node", "plan")
    if root_node not in by_id or current_node not in by_id:
        raise GoalError("GOAL_PLAN_INVALID", "root_node/current_node must exist")
    for mandate in mandates:
        missing_nodes = [node for node in mandate["nodes"] if node not in by_id]
        if missing_nodes:
            raise GoalError("GOAL_PLAN_INVALID", f"{mandate['id']} names missing nodes {missing_nodes}")
    return mandates, nodes, by_id


def children_map(nodes: list[dict[str, Any]]) -> dict[str, list[dict[str, Any]]]:
    children: dict[str, list[dict[str, Any]]] = {}
    for node in nodes:
        children.setdefault(node["parent"], []).append(node)
    for rows in children.values():
        rows.sort(key=lambda row: (row["order"], row["id"]))
    return children


def preorder(root_id: str, children: dict[str, list[dict[str, Any]]], by_id: dict[str, dict[str, Any]]) -> list[dict[str, Any]]:
    result: list[dict[str, Any]] = []

    def walk(node_id: str) -> None:
        result.append(by_id[node_id])
        for child in children.get(node_id, []):
            walk(child["id"])

    walk(root_id)
    return result


def campaign_ancestor(node: dict[str, Any], by_id: dict[str, dict[str, Any]]) -> bool:
    parent = node["parent"]
    while parent:
        row = by_id[parent]
        if row["kind"] == "campaign":
            return True
        parent = row["parent"]
    return False


def select_goal(settings: dict[str, Any], nodes: list[dict[str, Any]], by_id: dict[str, dict[str, Any]]) -> dict[str, Any]:
    selected = settings.get("goal_node")
    if selected is not None:
        if not isinstance(selected, str) or selected not in by_id:
            raise GoalError("GOAL_NODE_INVALID", f"unknown goal_node {selected!r}")
        node = by_id[selected]
        if node["kind"] not in {"campaign", "portfolio"} or node["state"] in TERMINAL_SELECTION_STATES:
            raise GoalError("GOAL_NODE_INVALID", f"goal_node {selected} is not an open campaign/portfolio")
        return node
    candidates = [
        node
        for node in nodes
        if node["kind"] == "campaign"
        and node["state"] not in TERMINAL_SELECTION_STATES
        and not campaign_ancestor(node, by_id)
    ]
    if len(candidates) != 1:
        raise GoalError("GOAL_NODE_AMBIGUOUS", f"expected one inferable campaign, found {len(candidates)}")
    return candidates[0]


def unique_texts(items: list[str]) -> list[str]:
    seen: set[str] = set()
    result: list[str] = []
    for item in items:
        if item not in seen:
            seen.add(item)
            result.append(item)
    return result


def section_with_entries(label: str, entries: list[str], id_entries: list[str], budget: int) -> str:
    if not entries:
        return ""
    full = f" {label}: " + "; ".join(entries) + "."
    if utf16_units(full) <= budget:
        return full
    compact = f" {label}: " + "; ".join(id_entries) + "."
    if utf16_units(compact) <= budget:
        return compact
    prefix = f" {label}: "
    chosen: list[str] = []
    for index, entry in enumerate(id_entries):
        trial = prefix + "; ".join(chosen + [entry])
        remaining = len(id_entries) - index - 1
        suffix = f"; +{remaining} more in GOAL.md." if remaining else "."
        if utf16_units(trial + suffix) > budget:
            break
        chosen.append(entry)
    remaining = len(id_entries) - len(chosen)
    if chosen:
        suffix = f"; +{remaining} more in GOAL.md." if remaining else "."
        return prefix + "; ".join(chosen) + suffix
    fallback = f" {label}: {len(id_entries)} entries in GOAL.md."
    return fallback if utf16_units(fallback) <= budget else ""


def build_condition(selected: dict[str, Any], subtree: list[dict[str, Any]], revision: int) -> str:
    base = collapse_ws(
        f"Complete campaign [{selected['id']}] {selected['title']} exactly as the current "
        f"stewardship plan.toml revision {revision} defines, including every non-terminal "
        "descendant and every completion proof recorded in GOAL.md."
    )
    continuity = collapse_ws(CLAUDE_CONTINUITY)
    mandatory = base + " " + continuity
    if utf16_units(mandatory) > CLAUDE_CONDITION_MAX_UTF16:
        raise GoalError("GOAL_CLAUDE_BASE_TOO_LONG", "mandatory Claude goal clauses exceed 4000 UTF-16 units")

    candidates = [node for node in subtree if node["state"] == "candidate"]
    direct_route = [
        node
        for node in subtree
        if node["parent"] == selected["id"] and node["state"] not in ROUTE_TERMINAL_STATES
    ]
    remaining_budget = CLAUDE_CONDITION_MAX_UTF16 - utf16_units(base + " " + continuity)
    candidate_full = [collapse_ws(f"[{node['id']}] {node['title']}") for node in candidates]
    candidate_ids = [f"[{node['id']}]" for node in candidates]
    route_reserve = (
        utf16_units(f" Remaining top-level route: {len(direct_route)} entries in GOAL.md.")
        if direct_route
        else 0
    )
    candidate_text = section_with_entries(
        "Current candidates",
        candidate_full,
        candidate_ids,
        max(0, remaining_budget - route_reserve),
    )
    remaining_budget -= utf16_units(candidate_text)
    route_full = [collapse_ws(f"[{node['id']}] {node['title']} ({node['state']})") for node in direct_route]
    route_ids = [f"[{node['id']}]" for node in direct_route]
    route_text = section_with_entries("Remaining top-level route", route_full, route_ids, remaining_budget)
    condition = base + candidate_text + route_text + " " + continuity
    if utf16_units(condition) > CLAUDE_CONDITION_MAX_UTF16:
        raise GoalError("GOAL_CLAUDE_BASE_TOO_LONG", "bounded Claude condition unexpectedly exceeds limit")
    return condition


def render(snapshot: dict[str, Any]) -> tuple[bytes, bytes | None, dict[str, Any]]:
    plan = snapshot["plan"]
    settings = snapshot["settings"]
    mandates, nodes, by_id = validate_plan(plan)
    selected = select_goal(settings, nodes, by_id)
    children = children_map(nodes)
    subtree = preorder(selected["id"], children, by_id)
    subtree_ids = {node["id"] for node in subtree}
    profile = settings.get("planning_profile", snapshot["global_profile"])
    if profile in {"normal", "medium"}:
        profile = "standard"
    if profile not in PROFILE_TEXT:
        raise GoalError("GOAL_PLAN_INVALID", f"unknown planning profile {profile!r}")

    try:
        condition = build_condition(selected, subtree, plan["revision"])
        condition_hash = sha256(condition.encode("utf-8"))
        claude_bytes: bytes | None = ("/goal " + condition + "\n").encode("utf-8")
    except GoalError as exc:
        if exc.code != "GOAL_CLAUDE_BASE_TOO_LONG":
            raise
        condition = None
        condition_hash = "unavailable"
        claude_bytes = None

    marker = (
        f'<!-- steward-goal schema={SCHEMA} plan_id="{plan["plan_id"]}" '
        f'plan_revision={plan["revision"]} goal_node="{selected["id"]}" '
        f'planning_profile="{profile}" plan_sha256="{snapshot["plan_sha"]}" '
        f'claude_condition_sha256="{condition_hash}" -->'
    )
    lines = [marker, f"# Goal — [{selected['id']}] {selected['title']}", "", "## Outcome"]
    lines.append(f"Complete [{selected['id']}] {md_text(selected['title'])}.")
    for acceptance in selected["acceptance"]:
        lines.append(f"- Acceptance: {md_text(acceptance)}")

    referenced_mandates = {
        mid
        for node in subtree
        if node["state"] not in {"dropped", "superseded"}
        for mid in node["mandates"]
    }
    lines.extend(["", "## Governing mandates"])
    for mandate in mandates:
        if mandate["id"] in referenced_mandates:
            lines.append(
                f"- [{mandate['id']}] {mandate['disposition']} — {md_text(mandate['text'])}"
            )
    if not referenced_mandates:
        lines.append("- None.")

    accepted = [
        node
        for node in subtree
        if node["state"] == "accepted"
        and (node["parent"] not in subtree_ids or by_id[node["parent"]]["state"] != "accepted")
    ]
    lines.extend(["", "## Accepted boundary"])
    lines.extend(
        [f"- [{node['id']}] {md_text(node['title'])}" for node in accepted] or ["- None."]
    )

    candidates = [node for node in subtree if node["state"] == "candidate"]
    lines.extend(["", "## Current candidates"])
    if candidates:
        for node in candidates:
            lines.append(f"- [{node['id']}] {md_text(node['title'])}")
            for acceptance in node["acceptance"]:
                lines.append(f"  - Acceptance: {md_text(acceptance)}")
    else:
        lines.append("- None.")

    remaining = [node for node in subtree if node["state"] not in ROUTE_TERMINAL_STATES]
    lines.extend(["", "## Remaining route"])
    for node in remaining:
        deps = ", ".join(node["depends_on"]) if node["depends_on"] else "none"
        lines.append(
            f"- [{node['id']}] {node['state']} — {md_text(node['title'])}; depends on: {deps}"
        )
    if not remaining:
        lines.append("- None.")

    deferred = [node for node in subtree if node["state"] == "deferred"]
    lines.extend(["", "## Deferred horizon"])
    if deferred:
        for node in deferred:
            lines.append(f"- [{node['id']}] {md_text(node['title'])}")
            for acceptance in node["acceptance"]:
                lines.append(f"  - Revisit/acceptance: {md_text(acceptance)}")
    else:
        lines.append("- None.")

    closure = list(selected["acceptance"])
    for node in subtree:
        if node["kind"] == "gate" and node["state"] != "accepted":
            closure.extend(node["acceptance"])
    closure = unique_texts(closure)
    lines.extend(["", "## Completion proof"])
    lines.extend([f"- {md_text(item)}" for item in closure] or ["- No explicit proof declared."])
    lines.extend(["", "## Execution profile", PROFILE_TEXT[profile]])
    lines.extend(["", "## Continuation law", CONTINUITY_TEXT, ""])
    goal_bytes = "\n".join(lines).encode("utf-8")
    metadata = {
        "schema": SCHEMA,
        "plan_id": plan["plan_id"],
        "plan_revision": plan["revision"],
        "goal_node": selected["id"],
        "planning_profile": profile,
        "plan_sha256": snapshot["plan_sha"],
        "claude_condition_sha256": condition_hash,
        "claude_condition_utf16_units": utf16_units(condition) if condition is not None else None,
    }
    return goal_bytes, claude_bytes, metadata


def read_snapshot(context: Path) -> dict[str, Any]:
    settings_raw = read_bytes(context / "settings.toml", "GOAL_PLAN_INVALID")
    custody_raw = read_bytes(context / "custody.toml", "GOAL_NOT_HOLDER")
    plan_raw = read_bytes(context / "plan.toml", "GOAL_PLAN_INVALID")
    config_path = context.parent.parent / "config.toml"
    config_raw = read_bytes(config_path, "GOAL_PLAN_INVALID")
    settings = parse_toml(settings_raw, context / "settings.toml")
    custody = parse_toml(custody_raw, context / "custody.toml")
    plan = parse_toml(plan_raw, context / "plan.toml")
    config = parse_toml(config_raw, config_path)
    return {
        "settings_raw": settings_raw,
        "custody_raw": custody_raw,
        "plan_raw": plan_raw,
        "settings": settings,
        "custody": custody,
        "plan": plan,
        "plan_sha": sha256(plan_raw),
        "global_profile": config.get("planning_profile", "standard"),
    }


def assert_holder(snapshot: dict[str, Any], holder_id: str | None, session_id: str | None) -> None:
    custody = snapshot["custody"]
    if (
        custody.get("state") != "held"
        or holder_id is None
        or session_id is None
        or custody.get("holder_id") != holder_id
        or custody.get("session_id") != session_id
    ):
        raise GoalError("GOAL_NOT_HOLDER", "current session does not hold this context")


def inputs_unchanged(context: Path, snapshot: dict[str, Any]) -> bool:
    return (
        read_bytes(context / "settings.toml", "GOAL_INPUT_MOVED") == snapshot["settings_raw"]
        and read_bytes(context / "custody.toml", "GOAL_INPUT_MOVED") == snapshot["custody_raw"]
        and read_bytes(context / "plan.toml", "GOAL_INPUT_MOVED") == snapshot["plan_raw"]
    )


def replace_if_changed(path: Path, data: bytes) -> bool:
    try:
        if path.exists() and path.read_bytes() == data:
            return False
    except OSError as exc:
        raise GoalError("GOAL_INPUT_MOVED", f"cannot inspect {path}: {exc}") from exc
    fd, temp_name = tempfile.mkstemp(prefix=f".{path.name}.new-", dir=path.parent)
    temp = Path(temp_name)
    try:
        with os.fdopen(fd, "wb") as stream:
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temp, path)
    finally:
        try:
            temp.unlink(missing_ok=True)
        except OSError:
            pass
    return True


def current_status(context: Path, goal_bytes: bytes, claude_bytes: bytes | None) -> tuple[bool, list[str]]:
    reasons: list[str] = []
    goal_path = context / "GOAL.md"
    command_path = context / "GOAL-CLAUDE.txt"
    if not goal_path.exists() or goal_path.read_bytes() != goal_bytes:
        reasons.append("GOAL.md missing or stale")
    if claude_bytes is None:
        if command_path.exists():
            reasons.append("GOAL-CLAUDE.txt exists for unavailable adapter")
    elif not command_path.exists() or command_path.read_bytes() != claude_bytes:
        reasons.append("GOAL-CLAUDE.txt missing or stale")
    return not reasons, reasons


def run(args: argparse.Namespace) -> dict[str, Any]:
    context = args.context.resolve()
    if not context.is_dir():
        raise GoalError("GOAL_PLAN_INVALID", f"context directory not found: {context}")
    last_moved = False
    for attempt in range(2):
        snapshot = read_snapshot(context)
        goal_bytes, claude_bytes, metadata = render(snapshot)
        current, reasons = current_status(context, goal_bytes, claude_bytes)
        can_write = False
        try:
            assert_holder(snapshot, args.holder_id, args.session_id)
            can_write = True
        except GoalError:
            can_write = False
        if args.check:
            return {
                **metadata,
                "current": current,
                "reasons": reasons,
                "can_write": can_write,
                "goal_path": str(context / "GOAL.md"),
                "claude_goal_path": str(context / "GOAL-CLAUDE.txt"),
            }
        if not can_write:
            raise GoalError("GOAL_NOT_HOLDER", "render requested by a session without held custody")
        if not inputs_unchanged(context, snapshot):
            last_moved = True
            continue
        command_changed = False
        command_path = context / "GOAL-CLAUDE.txt"
        if claude_bytes is None:
            if command_path.exists():
                command_path.unlink()
                command_changed = True
        else:
            command_changed = replace_if_changed(command_path, claude_bytes)
        goal_changed = replace_if_changed(context / "GOAL.md", goal_bytes)
        rendered_goal, rendered_command, second_meta = render(snapshot)
        if rendered_goal != goal_bytes or rendered_command != claude_bytes or second_meta != metadata:
            raise GoalError("GOAL_INPUT_MOVED", "second in-memory render was not byte-identical")
        current, reasons = current_status(context, goal_bytes, claude_bytes)
        if not current:
            raise GoalError("GOAL_INPUT_MOVED", "; ".join(reasons))
        return {
            **metadata,
            "current": True,
            "can_write": True,
            "goal_changed": goal_changed,
            "claude_goal_changed": command_changed,
            "goal_path": str(context / "GOAL.md"),
            "claude_goal_path": str(context / "GOAL-CLAUDE.txt"),
        }
    if last_moved:
        raise GoalError("GOAL_INPUT_MOVED", "inputs moved during both bounded snapshots")
    raise GoalError("GOAL_INPUT_MOVED", "unable to obtain a coherent snapshot")


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    result.add_argument("--context", type=Path, required=True)
    result.add_argument("--holder-id")
    result.add_argument("--session-id")
    result.add_argument("--check", action="store_true", help="diagnose only; never write")
    return result


def main() -> int:
    args = parser().parse_args()
    try:
        result = run(args)
    except GoalError as exc:
        print(json.dumps({"ok": False, "code": exc.code, "message": str(exc)}, ensure_ascii=False))
        return 2
    print(json.dumps({"ok": True, **result}, ensure_ascii=False, sort_keys=True))
    return 0 if result.get("current", True) else 3


if __name__ == "__main__":
    raise SystemExit(main())
