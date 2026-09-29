"""Pure temporary routing checks (PROP-081#TEMPORARY-CHANGE-BRIDGE).

Expected scope and record IDs are supplied declarations, not verified product
subjects. Permanent record meaning, bytes and actual delta remain Rust-owned.
"""
from __future__ import annotations

import json
from pathlib import Path


def need(condition, reason):
    if not condition:
        raise ValueError(reason)


def strict_pairs(pairs):
    result = {}
    for key, value in pairs:
        need(key not in result, "duplicate bridge JSON object key")
        result[key] = value
    return result


def read_bridge(path: Path):
    return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=strict_pairs)


def fields(value, expected, subject):
    need(isinstance(value, dict) and set(value) == set(expected),
         f"invalid {subject} fields")


def nonblank(value, subject):
    need(isinstance(value, str) and bool(value.strip()), f"blank or invalid {subject}")
    return value


def members(value, subject, nonempty=False):
    need(isinstance(value, list), f"invalid {subject} list")
    if nonempty:
        need(bool(value), f"empty {subject}")
    result = set()
    for identity in value:
        nonblank(identity, subject)
        need(identity not in result, f"duplicate {subject}")
        result.add(identity)
    return result


def validate_change_coverage(bridge, tasks, before_capture_id, after_capture_id, record_ids):
    """Check references only; exact input text is preserved without normalization."""
    nonblank(before_capture_id, "expected before capture ID")
    nonblank(after_capture_id, "expected after capture ID")
    selected = members(record_ids, "expected record IDs", nonempty=True)
    fields(bridge, ("schema", "scope", "tasks", "unplanned"), "bridge")
    need(type(bridge["schema"]) is int and bridge["schema"] == 1, "unsupported bridge schema")
    scope = bridge["scope"]
    fields(scope, ("before_capture_id", "after_capture_id"), "bridge scope")
    for key, expected in (("before_capture_id", before_capture_id),
                          ("after_capture_id", after_capture_id)):
        nonblank(scope[key], "bridge capture ID")
        need(scope[key] == expected, "bridge scope differs from supplied expected scope")
    need(isinstance(bridge["tasks"], list), "invalid bridge task rows")
    need(isinstance(bridge["unplanned"], list), "invalid bridge unplanned rows")
    task_ids, contribution_ids, covered = set(), set(), set()
    bound_task_rows = 0
    for row in bridge["tasks"]:
        fields(row, ("task_id", "change_ids"), "task row")
        task_id = nonblank(row["task_id"], "task ID")
        need(task_id in tasks, "unknown child task ID")
        need(task_id not in task_ids, "duplicate task row")
        task_ids.add(task_id)
        changes = members(row["change_ids"], "task change IDs")
        need(changes.issubset(selected), "task references an unselected record ID")
        covered.update(changes)
        bound_task_rows += bool(changes)
    for row in bridge["unplanned"]:
        fields(row, ("contribution_id", "reason", "change_ids"), "unplanned row")
        identity = nonblank(row["contribution_id"], "unplanned contribution ID")
        nonblank(row["reason"], "unplanned contribution reason")
        need(identity not in contribution_ids, "duplicate unplanned row")
        contribution_ids.add(identity)
        changes = members(row["change_ids"], "unplanned change IDs", nonempty=True)
        need(changes.issubset(selected), "unplanned entry references an unselected record ID")
        covered.update(changes)
    need(covered == selected, "selected record ID is omitted from bridge coverage")
    return {
        "ok": True,
        "mode": "temporary-bridge-structure",
        "bridge_references_validated": True,
        "scope": dict(scope),
        "record_ids": sorted(selected),
        "task_ids": sorted(task_ids),
        "unplanned_contribution_ids": sorted(contribution_ids),
        "task_rows": len(task_ids),
        "bound_task_rows": bound_task_rows,
        "unplanned_rows": len(contribution_ids),
        "selected_record_count": len(selected),
        "product_gates_executed": False,
        "preview_change_records_verified": False,
        "central_acceptance": False,
        "supplied_scope_and_record_ids_verified": False,
    }
