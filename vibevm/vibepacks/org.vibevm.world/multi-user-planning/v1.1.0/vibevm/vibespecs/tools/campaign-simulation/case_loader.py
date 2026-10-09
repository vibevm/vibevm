#!/usr/bin/env python3
"""Strict, read-only loaders for deterministic campaign simulation inputs."""

from __future__ import annotations

import copy
import hashlib
import json
import math
from pathlib import Path
import re
import tomllib
from typing import Any


SCHEMA = 1
ID_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:-]*$")
SHA_RE = re.compile(r"^[0-9a-f]{64}$")
CASE_FIELDS = {
    "schema", "id", "revision", "spec_refs", "initial", "events", "dialogue",
    "expected", "semantic_rubric", "limits",
}
INITIAL_FIELDS = {
    "plan_fixture", "plan_state", "actor", "roles", "owner_instruction", "holds",
    "input_identities", "candidates", "evidence", "jobs", "operations", "packets",
    "parallel", "panel", "obligations", "recovery", "closure_requested",
}
EXPECTED_FIELDS = {
    "required_actions", "forbidden_actions", "required_action_order", "final",
    "required_trace_kinds",
}
QUESTION_FIELDS = {
    "id", "intent", "subject", "recipient", "required_information", "after_event", "answer",
}
LIMIT_FIELDS = {"max_steps", "max_events"}
RUBRIC_FIELDS = {
    "meaning", "authority", "questions", "obligations", "actions", "state",
    "evidence", "ordering", "uncertainty", "closure",
}
EVIDENCE_FIELDS = {"id", "input_id", "subject", "claims", "cases", "required_cases", "valid", "tier"}
JOB_FIELDS = {"id", "node_id", "status", "write_paths", "actor_id", "writer_role", "conflicts_with"}
OPERATION_FIELDS = {"id", "kind", "status", "subject"}
PACKET_FIELDS = {
    "task_id", "read_paths", "write_paths", "shared_path", "integration_owner",
    "checks", "report_path", "safe_stop",
}
PARALLEL_FIELDS = {"packets", "conflicts", "advisory_batches", "review_queue"}
PARALLEL_PACKET_FIELDS = {"id", "reads", "writes"}
CONFLICT_FIELDS = {"packets", "code", "subject"}
PANEL_FIELDS = {"id", "steps"}
PANEL_STEP_FIELDS = {"id", "status", "subject_valid", "required_cases"}
ALLOWED_PLAN_STATES = {
    "planned", "ready", "active", "candidate", "accepted", "blocked", "deferred",
    "dropped", "superseded",
}
ALLOWED_ROLES = {"owner", "maintainer", "integrator", "contributor-central", "worker"}
ALLOWED_ACCEPTANCE_TIERS = {
    "worker-reported", "central-reviewed", "local-gated", "PR-reviewed", "merged",
    "project-accepted",
}
EVENT_FIELDS = {
    "owner_message": {"id", "type", "mode", "selected_campaign", "authorization"},
    "job_observation": {"id", "type", "job_id", "status"},
    "gate_result": {
        "id", "type", "gate_id", "status", "selected_cases", "required_cases", "subject",
    },
    "input_change": {"id", "type", "input_id", "before", "after", "expected"},
    "worker_report": {"id", "type", "worker_id", "node_id", "status", "evidence_id"},
    "acceptance_attempt": {"id", "type", "actor", "node_id", "tier"},
    "closure_request": {"id", "type"},
}
CORRECTION_FIELDS = {
    "schema", "id", "immutable", "classification", "case_ids", "operations",
    "original_real_result", "evidence_refs", "rationale", "governing_specs", "reviewer",
    "coverage_impact", "simulator_run_ids", "real_recheck_run_ids",
}
CORRECTION_CLASSES = {
    "agent_failure", "model_defect", "case_defect", "ambiguous_specification",
    "evidence_harness_failure",
}
TOMBSTONE_FIELDS = {
    "schema", "id", "immutable", "retired_case_id", "retired_revision",
    "prior_case_sha256", "reason", "replacement_coverage", "governing_spec",
}
HISTORY_FIELDS = {
    "schema", "id", "immutable", "bundle", "baseline_cases", "manual_limits",
}
HISTORY_CASE_FIELDS = {"id", "revision", "sha256", "original_real_result"}
ORIGINAL_RESULT_FIELDS = {
    "verdict", "run_id", "transcript_sha256", "structured_trace_sha256",
    "fixture_sha256", "spec_identity",
}


class InputError(ValueError):
    """A typed refusal for malformed or unsafe simulation input."""

    def __init__(self, code: str, message: str):
        super().__init__(message)
        self.code = code


def require(condition: bool, code: str, message: str) -> None:
    if not condition:
        raise InputError(code, message)


def _reject_constant(value: str) -> None:
    raise InputError("JSON_NONFINITE", f"non-finite JSON number {value!r}")


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise InputError("JSON_DUPLICATE_KEY", f"duplicate JSON key {key!r}")
        result[key] = value
    return result


def read_utf8(path: Path, kind: str) -> tuple[bytes, str]:
    try:
        raw = path.read_bytes()
    except OSError as exc:
        raise InputError(f"{kind}_READ_ERROR", f"cannot read {path}: {exc}") from exc
    require(not raw.startswith(b"\xef\xbb\xbf"), f"{kind}_INVALID", "UTF-8 BOM is forbidden")
    try:
        return raw, raw.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise InputError(f"{kind}_INVALID", f"invalid UTF-8 in {path}: {exc}") from exc


def load_json(path: Path) -> tuple[bytes, Any]:
    raw, text = read_utf8(path, "JSON")
    try:
        value = json.loads(
            text,
            object_pairs_hook=_unique_object,
            parse_constant=_reject_constant,
        )
    except InputError:
        raise
    except (json.JSONDecodeError, ValueError) as exc:
        raise InputError("JSON_INVALID", f"invalid JSON in {path}: {exc}") from exc
    reject_nonfinite(value, "JSON")
    return raw, value


def reject_nonfinite(value: Any, owner: str) -> None:
    if isinstance(value, float):
        require(math.isfinite(value), f"{owner}_NONFINITE", "non-finite number is forbidden")
    elif isinstance(value, dict):
        for child in value.values():
            reject_nonfinite(child, owner)
    elif isinstance(value, list):
        for child in value:
            reject_nonfinite(child, owner)


def exact_fields(row: dict[str, Any], allowed: set[str], owner: str) -> None:
    unknown = sorted(set(row) - allowed)
    require(not unknown, "UNKNOWN_CONTROL_FIELD", f"{owner} has unknown fields: {unknown}")


def valid_id(value: Any, owner: str) -> str:
    require(isinstance(value, str) and ID_RE.fullmatch(value) is not None,
            "IDENTIFIER_INVALID", f"{owner} must be a stable identifier")
    return value


def string_list(value: Any, owner: str, *, nonempty: bool = False) -> list[str]:
    require(isinstance(value, list) and all(isinstance(item, str) and item for item in value),
            "FIELD_INVALID", f"{owner} must be an array of non-empty strings")
    require(not nonempty or bool(value), "FIELD_INVALID", f"{owner} must not be empty")
    return value


def validate_plan(plan: dict[str, Any]) -> None:
    """Validate controls while deliberately preserving opaque extension fields."""
    require(isinstance(plan, dict), "PLAN_INVALID", "fixture plan must be a table")
    require(type(plan.get("schema")) is int and plan["schema"] == SCHEMA,
            "PLAN_INVALID", "plan schema must be integer 1")
    valid_id(plan.get("plan_id"), "plan_id")
    require(type(plan.get("revision")) is int and plan["revision"] >= 0,
            "PLAN_INVALID", "plan revision must be a non-negative integer")
    root = valid_id(plan.get("root_node"), "root_node")
    current = valid_id(plan.get("current_node"), "current_node")
    nodes = plan.get("node")
    require(isinstance(nodes, list) and bool(nodes) and all(isinstance(row, dict) for row in nodes),
            "PLAN_INVALID", "plan.node must be a non-empty array of tables")
    by_id: dict[str, dict[str, Any]] = {}
    for row in nodes:
        key = valid_id(row.get("id"), "node.id")
        require(key not in by_id, "PLAN_INVALID", f"duplicate node id {key}")
        by_id[key] = row
        require(isinstance(row.get("parent"), str), "PLAN_INVALID", f"{key}.parent must be a string")
        require(type(row.get("order")) is int, "PLAN_INVALID", f"{key}.order must be an integer")
        for field in ("kind", "title", "state"):
            require(isinstance(row.get(field), str) and bool(row[field]),
                    "PLAN_INVALID", f"{key}.{field} must be a non-empty string")
        for field in ("depends_on", "mandates", "acceptance", "evidence"):
            string_list(row.get(field), f"{key}.{field}")
    require(root in by_id and current in by_id, "PLAN_INVALID", "root/current node must exist")
    require(by_id[root]["parent"] == "", "PLAN_INVALID", "root node must have no parent")
    parent_edges: dict[str, list[str]] = {}
    dep_edges: dict[str, list[str]] = {}
    for key, row in by_id.items():
        parent = row["parent"]
        require(not parent or parent in by_id, "PLAN_INVALID", f"{key} has missing parent {parent}")
        parent_edges[key] = [parent] if parent else []
        missing = [dep for dep in row["depends_on"] if dep not in by_id]
        require(not missing, "PLAN_INVALID", f"{key} has missing dependencies {missing}")
        dep_edges[key] = row["depends_on"]
    detect_cycle(parent_edges, "parent")
    detect_cycle(dep_edges, "dependency")
    for key in by_id:
        cursor = key
        while cursor != root:
            parent = by_id[cursor]["parent"]
            require(bool(parent), "PLAN_INVALID", f"node {key} is outside root {root}")
            cursor = parent
    reject_nonfinite(plan, "TOML")


def detect_cycle(edges: dict[str, list[str]], owner: str) -> None:
    visiting: set[str] = set()
    visited: set[str] = set()

    def walk(key: str) -> None:
        require(key not in visiting, "PLAN_INVALID", f"{owner} cycle reaches {key}")
        if key in visited:
            return
        visiting.add(key)
        for child in edges.get(key, []):
            walk(child)
        visiting.remove(key)
        visited.add(key)

    for key in edges:
        walk(key)


def load_plan(path: Path, expected_sha: str) -> tuple[bytes, dict[str, Any]]:
    require(SHA_RE.fullmatch(expected_sha) is not None,
            "FIXTURE_HASH_INVALID", "fixture SHA-256 must be lowercase hex")
    raw, text = read_utf8(path, "TOML")
    actual = hashlib.sha256(raw).hexdigest()
    require(actual == expected_sha, "FIXTURE_HASH_MISMATCH",
            f"fixture hash mismatch for {path}: expected {expected_sha}, got {actual}")
    try:
        plan = tomllib.loads(text)
    except tomllib.TOMLDecodeError as exc:
        raise InputError("TOML_INVALID", f"invalid fixture TOML in {path}: {exc}") from exc
    validate_plan(plan)
    return raw, plan


def apply_plan_state(plan: dict[str, Any], overrides: dict[str, str]) -> dict[str, Any]:
    result = copy.deepcopy(plan)
    require(isinstance(overrides, dict)
            and all(isinstance(key, str) and isinstance(value, str) for key, value in overrides.items()),
            "CASE_INVALID", "initial.plan_state must map node ids to states")
    by_id = {row["id"]: row for row in result["node"]}
    require(set(overrides) <= set(by_id), "CASE_INVALID", "plan_state names an unknown node")
    for key, state in overrides.items():
        require(state in ALLOWED_PLAN_STATES, "CASE_INVALID", f"invalid plan state {state!r}")
        by_id[key]["state"] = state
    return result


def validate_case(case: dict[str, Any], case_path: Path) -> dict[str, Any]:
    require(isinstance(case, dict), "CASE_INVALID", "case must be an object")
    exact_fields(case, CASE_FIELDS, "case")
    require(type(case.get("schema")) is int and case["schema"] == SCHEMA,
            "CASE_INVALID", "case schema must be integer 1")
    valid_id(case.get("id"), "case.id")
    require(type(case.get("revision")) is int and case["revision"] > 0,
            "CASE_INVALID", "case revision must be a positive integer")
    string_list(case.get("spec_refs"), "case.spec_refs", nonempty=True)
    require(all(ref.startswith("spec://") and "#" in ref for ref in case["spec_refs"]),
            "CASE_INVALID", "every spec ref must be an anchored spec:// reference")

    initial = case.get("initial")
    require(isinstance(initial, dict), "CASE_INVALID", "case.initial must be an object")
    exact_fields(initial, INITIAL_FIELDS, "initial")
    fixture = initial.get("plan_fixture")
    require(isinstance(fixture, dict), "CASE_INVALID", "initial.plan_fixture must be an object")
    exact_fields(fixture, {"path", "sha256"}, "plan_fixture")
    require(isinstance(fixture.get("path"), str) and bool(fixture["path"]),
            "CASE_INVALID", "fixture path must be a non-empty string")
    fixture_path = (case_path.parent / fixture["path"]).resolve()
    examples_root = case_path.parent.parent.resolve()
    require(fixture_path == examples_root or examples_root in fixture_path.parents,
            "CASE_INVALID", "fixture path escapes campaign-simulation examples")
    raw_plan, plan = load_plan(fixture_path, fixture.get("sha256", ""))
    plan = apply_plan_state(plan, initial.get("plan_state"))
    valid_id(initial.get("actor"), "initial.actor")
    roles = initial.get("roles")
    require(isinstance(roles, dict) and initial["actor"] in roles,
            "CASE_INVALID", "roles must include the simulated actor")
    for actor, grants in roles.items():
        valid_id(actor, "roles actor")
        string_list(grants, f"roles.{actor}", nonempty=True)
        require(set(grants) <= ALLOWED_ROLES and len(grants) == len(set(grants)),
                "CASE_INVALID", f"roles.{actor} has unknown or duplicate grants")
    require(isinstance(initial.get("owner_instruction"), dict),
            "CASE_INVALID", "owner_instruction must be an object")
    exact_fields(initial["owner_instruction"],
                 {"mode", "selected_campaign", "authorization"}, "owner_instruction")
    require(isinstance(initial["owner_instruction"].get("mode"), str)
            and initial["owner_instruction"]["mode"] in {"execute", "planning", "status"},
            "CASE_INVALID", "owner instruction mode is invalid")
    require(isinstance(initial["owner_instruction"].get("selected_campaign"), (str, type(None))),
            "CASE_INVALID", "selected_campaign must be string or null")
    require(type(initial["owner_instruction"].get("authorization")) is bool,
            "CASE_INVALID", "owner authorization must be boolean")
    for field in ("holds", "candidates", "obligations"):
        string_list(initial.get(field), f"initial.{field}")
    require(isinstance(initial.get("input_identities"), dict)
            and all(isinstance(k, str) and isinstance(v, str) for k, v in initial["input_identities"].items()),
            "CASE_INVALID", "input_identities must map strings to strings")
    for field in ("evidence", "jobs", "operations", "packets"):
        require(isinstance(initial.get(field), list)
                and all(isinstance(row, dict) for row in initial[field]),
                "CASE_INVALID", f"initial.{field} must be an array of objects")
    by_id = {row["id"]: row for row in plan["node"]}
    selected_campaign = initial["owner_instruction"]["selected_campaign"]
    require(selected_campaign is None
            or (selected_campaign in by_id and by_id[selected_campaign]["kind"] == "campaign"),
            "CASE_INVALID", "selected_campaign must name a campaign node")
    expected_candidates = sorted(key for key, row in by_id.items() if row["state"] == "candidate")
    require(sorted(initial["candidates"]) == expected_candidates,
            "CASE_INVALID", "initial candidates must exactly match candidate plan nodes")
    evidence_ids: set[str] = set()
    for evidence in initial["evidence"]:
        exact_fields(evidence, EVIDENCE_FIELDS, "evidence")
        evidence_id = valid_id(evidence.get("id"), "evidence.id")
        require(evidence_id not in evidence_ids, "CASE_INVALID", "duplicate evidence id")
        evidence_ids.add(evidence_id)
        valid_id(evidence.get("input_id"), "evidence.input_id")
        require(evidence["input_id"] in initial["input_identities"],
                "CASE_INVALID", "evidence input_id must have an input identity")
        require(isinstance(evidence.get("subject"), str) and evidence["subject"],
                "CASE_INVALID", "evidence subject is required")
        string_list(evidence.get("claims"), "evidence.claims", nonempty=True)
        string_list(evidence.get("cases"), "evidence.cases")
        string_list(evidence.get("required_cases"), "evidence.required_cases")
        require(type(evidence.get("valid")) is bool and isinstance(evidence.get("tier"), str),
                "CASE_INVALID", "evidence valid/tier controls are invalid")
    job_ids: set[str] = set()
    for job in initial["jobs"]:
        exact_fields(job, JOB_FIELDS, "job")
        job_id = valid_id(job.get("id"), "job.id")
        require(job_id not in job_ids, "CASE_INVALID", "duplicate job id")
        job_ids.add(job_id)
        require(job.get("node_id") in by_id and job.get("status") in {"live", "complete", "failed", "unknown"},
                "CASE_INVALID", "job node/status is invalid")
        string_list(job.get("write_paths"), "job.write_paths")
        writer_controls = (job.get("actor_id"), job.get("writer_role"), job.get("conflicts_with"))
        require(all(value is None for value in writer_controls)
                or (job.get("actor_id") in roles
                    and job.get("writer_role") in roles[job["actor_id"]]
                    and isinstance(job.get("conflicts_with"), str)
                    and bool(job["conflicts_with"])),
                "CASE_INVALID", "job writer controls must be absent or a complete granted binding")
    operation_ids: set[str] = set()
    for operation in initial["operations"]:
        exact_fields(operation, OPERATION_FIELDS, "operation")
        operation_id = valid_id(operation.get("id"), "operation.id")
        require(operation_id not in operation_ids, "CASE_INVALID", "duplicate operation id")
        operation_ids.add(operation_id)
        require(operation.get("kind") == "external-effect"
                and operation.get("status") in {"unknown", "live", "complete", "failed", "partial"}
                and isinstance(operation.get("subject"), str),
                "CASE_INVALID", "operation controls are invalid")
    for packet in initial["packets"]:
        exact_fields(packet, PACKET_FIELDS, "packet")
        require(packet.get("task_id") in by_id, "CASE_INVALID", "packet task_id is unknown")
        for field in ("read_paths", "write_paths", "checks"):
            string_list(packet.get(field), f"packet.{field}")
        require(isinstance(packet.get("shared_path"), str)
                and isinstance(packet.get("report_path"), str)
                and isinstance(packet.get("safe_stop"), str)
                and (packet.get("integration_owner") is None
                     or packet["integration_owner"] in roles),
                "CASE_INVALID", "packet ownership/path controls are invalid")
    parallel = initial.get("parallel")
    require(isinstance(parallel, dict), "CASE_INVALID", "parallel must be an object")
    exact_fields(parallel, PARALLEL_FIELDS, "parallel")
    require(isinstance(parallel.get("packets"), list)
            and isinstance(parallel.get("conflicts"), list)
            and isinstance(parallel.get("advisory_batches"), list),
            "CASE_INVALID", "parallel packet/conflict/batch controls must be arrays")
    string_list(parallel.get("review_queue"), "parallel.review_queue")
    parallel_ids: set[str] = set()
    for packet in parallel["packets"]:
        require(isinstance(packet, dict), "CASE_INVALID", "parallel packet must be an object")
        exact_fields(packet, PARALLEL_PACKET_FIELDS, "parallel packet")
        packet_id = valid_id(packet.get("id"), "parallel packet id")
        require(packet_id not in parallel_ids, "CASE_INVALID", "duplicate parallel packet id")
        parallel_ids.add(packet_id)
        string_list(packet.get("reads"), "parallel packet reads")
        string_list(packet.get("writes"), "parallel packet writes")
    for conflict in parallel["conflicts"]:
        require(isinstance(conflict, dict), "CASE_INVALID", "parallel conflict must be an object")
        exact_fields(conflict, CONFLICT_FIELDS, "parallel conflict")
        packets = string_list(conflict.get("packets"), "conflict.packets", nonempty=True)
        require(set(packets) <= parallel_ids and isinstance(conflict.get("code"), str)
                and isinstance(conflict.get("subject"), str),
                "CASE_INVALID", "parallel conflict controls are invalid")
    for batch in parallel["advisory_batches"]:
        require(isinstance(batch, list) and bool(batch) and all(item in parallel_ids for item in batch),
                "CASE_INVALID", "advisory batch must name parallel packets")
    panel = initial.get("panel")
    require(isinstance(panel, dict), "CASE_INVALID", "panel must be an object")
    exact_fields(panel, PANEL_FIELDS, "panel")
    valid_id(panel.get("id"), "panel.id")
    require(isinstance(panel.get("steps"), list), "CASE_INVALID", "panel.steps must be an array")
    panel_ids: set[str] = set()
    panel_by_id: dict[str, dict[str, Any]] = {}
    for step in panel["steps"]:
        require(isinstance(step, dict), "CASE_INVALID", "panel step must be an object")
        exact_fields(step, PANEL_STEP_FIELDS, "panel step")
        step_id = valid_id(step.get("id"), "panel step id")
        require(step_id not in panel_ids, "CASE_INVALID", "duplicate panel step id")
        panel_ids.add(step_id)
        panel_by_id[step_id] = step
        require(step.get("status") in {"pass", "failed", "invalidated", "interrupted", "unrun"}
                and type(step.get("subject_valid")) is bool,
                "CASE_INVALID", "panel step status/subject validity is invalid")
        string_list(step.get("required_cases"), "panel step required_cases", nonempty=True)
    require(type(initial.get("recovery")) is bool and type(initial.get("closure_requested")) is bool,
            "CASE_INVALID", "recovery/closure_requested must be booleans")

    events = case.get("events")
    require(isinstance(events, list), "CASE_INVALID", "events must be an array")
    event_ids: set[str] = set()
    rolling_inputs = dict(initial["input_identities"])
    for event in events:
        require(isinstance(event, dict), "CASE_INVALID", "event must be an object")
        kind = event.get("type")
        require(isinstance(kind, str) and kind in EVENT_FIELDS,
                "CASE_INVALID", f"unknown event type {kind!r}")
        exact_fields(event, EVENT_FIELDS[kind], f"event {kind}")
        missing = sorted(EVENT_FIELDS[kind] - set(event))
        require(not missing, "CASE_INVALID", f"event {kind} is missing fields: {missing}")
        event_id = valid_id(event.get("id"), "event.id")
        require(event_id not in event_ids, "CASE_INVALID", f"duplicate event id {event_id}")
        event_ids.add(event_id)
        require(event.get("type") == kind, "CASE_INVALID", "event.type is invalid")
        if kind == "owner_message":
            mode = event.get("mode")
            selected = event.get("selected_campaign")
            authorization = event.get("authorization")
            require(isinstance(mode, str) and mode in {"execute", "planning", "status"}
                    and (selected is None
                         or (isinstance(selected, str)
                             and selected in by_id and by_id[selected]["kind"] == "campaign"))
                    and type(authorization) is bool,
                    "CASE_INVALID", "owner_message controls are invalid")
            require(not authorization or (mode == "execute" and selected is not None),
                    "CASE_INVALID", "owner_message authorization requires a selected execution campaign")
        elif kind == "job_observation":
            require(isinstance(event.get("job_id"), str) and event["job_id"] in job_ids
                    and isinstance(event.get("status"), str)
                    and event["status"] in {"live", "complete", "failed", "unknown"},
                    "CASE_INVALID", "job_observation reference or status is invalid")
        elif kind == "gate_result":
            selected_cases = event.get("selected_cases")
            required_cases = event.get("required_cases")
            gate_id = valid_id(event.get("gate_id"), "gate_result.gate_id")
            panel_step = panel_by_id.get(gate_id)
            require(isinstance(event.get("status"), str)
                    and event["status"] in {"pass", "fail", "interrupted"}
                    and type(selected_cases) is int and selected_cases >= 0
                    and type(required_cases) is int and required_cases > 0
                    and selected_cases <= required_cases
                    and (panel_step is None
                         or required_cases == len(panel_step["required_cases"]))
                    and isinstance(event.get("subject"), str) and bool(event["subject"])
                    and event["subject"] in set(rolling_inputs.values()),
                    "CASE_INVALID",
                    "gate_result controls or immutable panel denominator are invalid")
        elif kind == "input_change":
            input_id = event.get("input_id")
            require(isinstance(input_id, str) and input_id in rolling_inputs
                    and isinstance(event.get("before"), str) and bool(event["before"])
                    and isinstance(event.get("after"), str) and bool(event["after"])
                    and event["before"] == rolling_inputs[input_id]
                    and event["after"] != event["before"]
                    and type(event.get("expected")) is bool,
                    "CASE_INVALID", "input_change identity or controls are invalid")
            rolling_inputs[input_id] = event["after"]
        elif kind == "worker_report":
            worker_id = event.get("worker_id")
            node_id = event.get("node_id")
            require(isinstance(worker_id, str) and worker_id in roles
                    and "worker" in roles[worker_id]
                    and isinstance(node_id, str) and node_id in by_id
                    and isinstance(event.get("status"), str)
                    and event["status"] in {"PASS", "FAIL", "UNKNOWN"},
                    "CASE_INVALID", "worker_report references or status are invalid")
            valid_id(event.get("evidence_id"), "worker_report.evidence_id")
        elif kind == "acceptance_attempt":
            require(isinstance(event.get("actor"), str) and event["actor"] in roles
                    and isinstance(event.get("node_id"), str) and event["node_id"] in by_id
                    and isinstance(event.get("tier"), str)
                    and event["tier"] in ALLOWED_ACCEPTANCE_TIERS,
                    "CASE_INVALID", "acceptance_attempt references or tier are invalid")
        else:
            require(kind == "closure_request", "CASE_INVALID", "event type is invalid")

    dialogue = case.get("dialogue")
    require(isinstance(dialogue, list), "CASE_INVALID", "dialogue must be an array")
    question_ids: set[str] = set()
    for question in dialogue:
        require(isinstance(question, dict), "CASE_INVALID", "dialogue entry must be an object")
        exact_fields(question, QUESTION_FIELDS, "dialogue entry")
        question_id = valid_id(question.get("id"), "dialogue.id")
        require(question_id not in question_ids, "CASE_INVALID", "duplicate dialogue id")
        question_ids.add(question_id)
        valid_id(question.get("intent"), "dialogue.intent")
        string_list(question.get("subject"), "dialogue.subject", nonempty=True)
        valid_id(question.get("recipient"), "dialogue.recipient")
        string_list(question.get("required_information"), "dialogue.required_information", nonempty=True)
        require(question.get("after_event") is None
                or (isinstance(question["after_event"], str)
                    and question["after_event"] in event_ids),
                "CASE_INVALID", "dialogue.after_event must name an event or be null")
        require(isinstance(question.get("answer"), dict) and bool(question["answer"]),
                "CASE_INVALID", "dialogue answer must be a non-empty object")

    expected = case.get("expected")
    require(isinstance(expected, dict), "CASE_INVALID", "expected must be an object")
    exact_fields(expected, EXPECTED_FIELDS, "expected")
    for field in ("required_actions", "forbidden_actions"):
        require(isinstance(expected.get(field), list)
                and all(isinstance(row, dict) for row in expected[field]),
                "CASE_INVALID", f"expected.{field} must be an array of patterns")
    string_list(expected.get("required_action_order"), "expected.required_action_order")
    require(isinstance(expected.get("final"), dict), "CASE_INVALID", "expected.final must be an object")
    string_list(expected.get("required_trace_kinds"), "expected.required_trace_kinds", nonempty=True)

    rubric = case.get("semantic_rubric")
    require(isinstance(rubric, dict), "CASE_INVALID", "semantic_rubric must be an object")
    exact_fields(rubric, RUBRIC_FIELDS, "semantic_rubric")
    require(all(isinstance(rubric.get(field), str) and rubric[field] for field in RUBRIC_FIELDS),
            "CASE_INVALID", "every semantic rubric field must be non-empty")
    limits = case.get("limits")
    require(isinstance(limits, dict), "CASE_INVALID", "limits must be an object")
    exact_fields(limits, LIMIT_FIELDS, "limits")
    require(type(limits.get("max_steps")) is int and 1 <= limits["max_steps"] <= 1000,
            "CASE_INVALID", "max_steps must be 1..1000")
    require(type(limits.get("max_events")) is int and 0 <= limits["max_events"] <= 100,
            "CASE_INVALID", "max_events must be 0..100")
    require(len(events) <= limits["max_events"], "CASE_INVALID", "event count exceeds limit")

    result = copy.deepcopy(case)
    result["initial"]["plan"] = plan
    result["initial"]["plan_raw_sha256"] = hashlib.sha256(raw_plan).hexdigest()
    return result


def load_cases(path: Path) -> list[dict[str, Any]]:
    _, bundle = load_json(path)
    require(isinstance(bundle, dict), "CASE_INVALID", "case bundle must be an object")
    exact_fields(bundle, {"schema", "cases"}, "case bundle")
    require(type(bundle.get("schema")) is int and bundle["schema"] == SCHEMA,
            "CASE_INVALID", "case bundle schema must be integer 1")
    rows = bundle.get("cases")
    require(isinstance(rows, list) and bool(rows), "CASE_INVALID", "case bundle must not be empty")
    cases = [validate_case(row, path) for row in rows]
    identities = [(row["id"], row["revision"]) for row in cases]
    require(len(identities) == len(set(identities)), "CASE_INVALID", "duplicate case id/revision")
    require(len({row["id"] for row in cases}) == len(cases),
            "CASE_INVALID", "bundle may contain only one revision of each case")
    return cases


def source_case_hash(case: dict[str, Any]) -> str:
    """Hash the strict source case, excluding loader-only normalized fields."""
    source_case = copy.deepcopy(case)
    source_case["initial"].pop("plan", None)
    source_case["initial"].pop("plan_raw_sha256", None)
    return hashlib.sha256(
        json.dumps(source_case, ensure_ascii=False, sort_keys=True,
                   separators=(",", ":")).encode("utf-8")
    ).hexdigest()


def validate_original_result(value: Any, owner: str) -> dict[str, Any]:
    require(isinstance(value, dict), "CORRECTION_INVALID", f"{owner} is required")
    exact_fields(value, ORIGINAL_RESULT_FIELDS, owner)
    require(value.get("verdict") in {"fail", "uncertain"},
            "CORRECTION_INVALID", "original verdict must remain fail or uncertain")
    for field in ("transcript_sha256", "structured_trace_sha256", "fixture_sha256"):
        require(isinstance(value.get(field), str) and SHA_RE.fullmatch(value[field]) is not None,
                "CORRECTION_INVALID", f"{owner}.{field} must be a SHA-256")
    for field in ("run_id", "spec_identity"):
        require(isinstance(value.get(field), str) and value[field],
                "CORRECTION_INVALID", f"{owner}.{field} is required")
    return value


def load_history(path: Path, bundle_name: str) -> dict[str, Any]:
    """Load the declared immutable baseline used to detect unrecorded bundle changes."""
    _, history = load_json(path)
    require(isinstance(history, dict), "HISTORY_INVALID", "history inventory must be an object")
    exact_fields(history, HISTORY_FIELDS, "history inventory")
    require(type(history.get("schema")) is int and history["schema"] == SCHEMA
            and history.get("immutable") is True
            and history.get("bundle") == bundle_name,
            "HISTORY_INVALID", "history inventory identity is invalid")
    valid_id(history.get("id"), "history.id")
    string_list(history.get("manual_limits"), "history.manual_limits", nonempty=True)
    rows = history.get("baseline_cases")
    require(isinstance(rows, list) and bool(rows) and all(isinstance(row, dict) for row in rows),
            "HISTORY_INVALID", "baseline_cases must be a non-empty object array")
    identities: set[str] = set()
    for row in rows:
        exact_fields(row, HISTORY_CASE_FIELDS, "history case")
        case_id = valid_id(row.get("id"), "history case id")
        require(case_id not in identities, "HISTORY_INVALID", "duplicate history case id")
        identities.add(case_id)
        require(type(row.get("revision")) is int and row["revision"] >= 0
                and isinstance(row.get("sha256"), str)
                and SHA_RE.fullmatch(row["sha256"]) is not None,
                "HISTORY_INVALID", "history case revision/hash is invalid")
        original = row.get("original_real_result")
        if original is not None:
            validate_original_result(original, f"history case {case_id} original_real_result")
    return history


def validate_correction(path: Path) -> dict[str, Any]:
    _, record = load_json(path)
    require(isinstance(record, dict), "CORRECTION_INVALID", "correction must be an object")
    exact_fields(record, CORRECTION_FIELDS, "correction")
    require(type(record.get("schema")) is int and record["schema"] == SCHEMA,
            "CORRECTION_INVALID", "correction schema must be integer 1")
    valid_id(record.get("id"), "correction.id")
    require(record.get("immutable") is True, "CORRECTION_INVALID", "correction must be immutable")
    require(record.get("classification") in CORRECTION_CLASSES,
            "CORRECTION_INVALID", "unknown discrepancy classification")
    string_list(record.get("case_ids"), "correction.case_ids", nonempty=True)
    string_list(record.get("evidence_refs"), "correction.evidence_refs", nonempty=True)
    string_list(record.get("governing_specs"), "correction.governing_specs", nonempty=True)
    require(all(ref.startswith("spec://") and "#" in ref for ref in record["governing_specs"]),
            "CORRECTION_INVALID", "governing_specs must be anchored spec references")
    string_list(record.get("simulator_run_ids"), "correction.simulator_run_ids", nonempty=True)
    string_list(record.get("real_recheck_run_ids"), "correction.real_recheck_run_ids", nonempty=True)
    require(isinstance(record.get("rationale"), str) and record["rationale"],
            "CORRECTION_INVALID", "correction rationale is required")
    original_result = validate_original_result(
        record.get("original_real_result"), "original_real_result"
    )
    recheck_ids = record["real_recheck_run_ids"]
    require(len(recheck_ids) == len(set(recheck_ids))
            and original_result["run_id"] not in set(recheck_ids),
            "CORRECTION_INVALID",
            "real recheck run ids must be unique and distinct from the frozen original run")
    reviewer = record.get("reviewer")
    require(isinstance(reviewer, dict), "CORRECTION_INVALID", "reviewer is required")
    exact_fields(reviewer, {"id", "verdict"}, "reviewer")
    require(isinstance(reviewer.get("id"), str) and reviewer["id"]
            and reviewer.get("verdict") == "approved",
            "CORRECTION_INVALID", "adopted correction requires recorded reviewer approval")
    coverage = record.get("coverage_impact")
    require(isinstance(coverage, dict), "CORRECTION_INVALID", "coverage impact is required")
    exact_fields(coverage, {"added", "removed", "preserved"}, "coverage_impact")
    for field in ("added", "removed", "preserved"):
        string_list(coverage.get(field), f"coverage_impact.{field}")
    operations = record.get("operations")
    require(isinstance(operations, list) and bool(operations),
            "CORRECTION_INVALID", "correction operations must not be empty")
    operation_case_ids: set[str] = set()
    for operation in operations:
        require(isinstance(operation, dict), "CORRECTION_INVALID", "operation must be an object")
        exact_fields(operation, {
            "kind", "case_id", "before_revision", "after_revision", "before_sha256",
            "after_sha256", "tombstone", "tombstone_path", "tombstone_sha256",
            "replacement_coverage", "authorized_reason",
        }, "correction operation")
        require(operation.get("kind") in {"add", "modify", "delete"},
                "CORRECTION_INVALID", "operation kind is invalid")
        operation_case_id = valid_id(operation.get("case_id"), "operation.case_id")
        require(operation_case_id not in operation_case_ids,
                "CORRECTION_INVALID", "correction repeats an operation case_id")
        operation_case_ids.add(operation_case_id)
        require(operation["case_id"] in record["case_ids"],
                "CORRECTION_INVALID", "operation case_id is absent from case_ids")
        for hash_field in ("before_sha256", "after_sha256"):
            value = operation.get(hash_field)
            require(value is None or (isinstance(value, str) and SHA_RE.fullmatch(value) is not None),
                    "CORRECTION_INVALID", f"operation.{hash_field} is invalid")
        before_revision = operation.get("before_revision")
        after_revision = operation.get("after_revision")
        require(before_revision is None or (type(before_revision) is int and before_revision >= 0),
                "CORRECTION_INVALID", "before_revision is invalid")
        require(after_revision is None or (type(after_revision) is int and after_revision > 0),
                "CORRECTION_INVALID", "after_revision is invalid")
        require(type(operation.get("tombstone")) is bool,
                "CORRECTION_INVALID", "operation.tombstone must be boolean")
        require(isinstance(operation.get("replacement_coverage"), list)
                and all(isinstance(item, str) and item for item in operation["replacement_coverage"]),
                "CORRECTION_INVALID", "replacement_coverage must be a string array")
        require(operation.get("authorized_reason") is None
                or (isinstance(operation["authorized_reason"], str)
                    and bool(operation["authorized_reason"])),
                "CORRECTION_INVALID", "authorized_reason must be null or non-empty")
        if operation["kind"] == "add":
            require(before_revision is None and operation.get("before_sha256") is None
                    and after_revision is not None and operation.get("after_sha256") is not None
                    and operation["tombstone"] is False
                    and operation.get("tombstone_path") is None
                    and operation.get("tombstone_sha256") is None,
                    "CORRECTION_INVALID", "add identity fields are inconsistent")
        elif operation["kind"] == "modify":
            require(before_revision is not None and after_revision is not None
                    and after_revision > before_revision
                    and operation.get("before_sha256") is not None
                    and operation.get("after_sha256") is not None
                    and operation["tombstone"] is False
                    and operation.get("tombstone_path") is None
                    and operation.get("tombstone_sha256") is None,
                    "CORRECTION_INVALID", "modify identity fields are inconsistent")
        else:
            require(operation.get("tombstone") is True and operation.get("before_sha256") is not None,
                    "CORRECTION_INVALID", "deletion requires tombstone and prior hash")
            require(before_revision is not None and after_revision is None
                    and operation.get("after_sha256") is None,
                    "CORRECTION_INVALID", "delete identity fields are inconsistent")
            replacements = operation.get("replacement_coverage")
            reason = operation.get("authorized_reason")
            require((isinstance(replacements, list) and bool(replacements)
                     and all(isinstance(item, str) and item for item in replacements))
                    or (isinstance(reason, str) and bool(reason)),
                    "CORRECTION_INVALID",
                    "deletion requires replacement coverage or an authorized reason")
            tombstone_path = operation.get("tombstone_path")
            tombstone_sha = operation.get("tombstone_sha256")
            require(isinstance(tombstone_path, str) and bool(tombstone_path)
                    and isinstance(tombstone_sha, str) and SHA_RE.fullmatch(tombstone_sha) is not None,
                    "CORRECTION_INVALID", "deletion requires a tombstone path and hash")
            resolved = (path.parent / tombstone_path).resolve()
            examples_root = path.parent.parent.resolve()
            require(resolved == examples_root or examples_root in resolved.parents,
                    "CORRECTION_INVALID", "tombstone path escapes campaign-simulation examples")
            tombstone_raw, tombstone = load_json(resolved)
            require(hashlib.sha256(tombstone_raw).hexdigest() == tombstone_sha,
                    "CORRECTION_INVALID", "tombstone hash mismatch")
            require(isinstance(tombstone, dict), "CORRECTION_INVALID", "tombstone must be an object")
            exact_fields(tombstone, TOMBSTONE_FIELDS, "tombstone")
            require(type(tombstone.get("schema")) is int and tombstone["schema"] == SCHEMA
                    and tombstone.get("immutable") is True
                    and tombstone.get("retired_case_id") == operation["case_id"]
                    and tombstone.get("retired_revision") == before_revision
                    and tombstone.get("prior_case_sha256") == operation["before_sha256"]
                    and tombstone.get("replacement_coverage") == replacements,
                    "CORRECTION_INVALID", "tombstone identity or replacement coverage differs")
            require(isinstance(tombstone.get("reason"), str) and bool(tombstone["reason"])
                    and isinstance(tombstone.get("governing_spec"), str)
                    and tombstone["governing_spec"].startswith("spec://")
                    and "#" in tombstone["governing_spec"],
                    "CORRECTION_INVALID", "tombstone reason/governing spec is invalid")
    require(set(record["case_ids"]) == operation_case_ids
            and len(record["case_ids"]) == len(set(record["case_ids"])),
            "CORRECTION_INVALID", "case_ids must exactly identify unique correction operations")
    return record
