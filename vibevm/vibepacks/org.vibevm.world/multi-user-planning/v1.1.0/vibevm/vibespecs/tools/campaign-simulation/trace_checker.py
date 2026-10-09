#!/usr/bin/env python3
"""Independent oracle checker for deterministic campaign simulation traces."""

from __future__ import annotations

import copy
import hashlib
import json
from typing import Any


SCHEMA = 1
TERMINAL = {"accepted", "deferred", "dropped", "superseded"}
EXECUTION_INTENTS = {
    "continue_authorized_campaign", "refine_plan", "run_panel_step",
    "accept_campaign_closure",
}
COORDINATING_ROLES = {"owner", "integrator", "contributor-central"}
PANEL_RESULT_INTENTS = {"record_gate_result", "accept_gate", "reuse_panel_step"}
PLAN_MUTATION_IDENTITY_FIELDS = (
    "subject", "field", "before", "after", "before_revision", "after_revision",
    "before_hash", "after_hash",
)


def canonical_hash(value: Any) -> str:
    return hashlib.sha256(
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()


def subset(expected: Any, actual: Any) -> bool:
    if isinstance(expected, dict):
        return isinstance(actual, dict) and all(
            key in actual and subset(value, actual[key]) for key, value in expected.items()
        )
    if isinstance(expected, list):
        return isinstance(actual, list) and expected == actual
    return expected == actual


def semantic_question(entry: dict[str, Any]) -> dict[str, Any]:
    return {
        "intent": entry.get("intent"),
        "subject": entry.get("subject"),
        "recipient": entry.get("recipient"),
        "required_information": entry.get("required_information"),
        "after_event": entry.get("after_event"),
    }


def ordered_route(plan: dict[str, Any]) -> list[str]:
    children: dict[str, list[dict[str, Any]]] = {}
    for row in plan["node"]:
        children.setdefault(row["parent"], []).append(row)
    for rows in children.values():
        rows.sort(key=lambda row: (row["order"], row["id"]))
    result: list[str] = []

    def walk(node_id: str) -> None:
        result.append(node_id)
        for child in children.get(node_id, []):
            walk(child["id"])

    walk(plan["root_node"])
    return result


def expected_plan(case: dict[str, Any]) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    """Project plan mutations from inputs without consulting model actions or final state."""
    plan = copy.deepcopy(case["initial"]["plan"])
    mutations: list[dict[str, Any]] = []

    def mutate(subject: str, field: str, after: Any, cause: dict[str, Any]) -> None:
        before_revision = plan["revision"]
        before_hash = canonical_hash(plan)
        if field == "current_node":
            before = plan["current_node"]
            if before == after:
                return
            plan["current_node"] = after
        else:
            node = next(row for row in plan["node"] if row["id"] == subject)
            before = node[field]
            if before == after:
                return
            node[field] = after
        plan["revision"] += 1
        mutations.append({
            "subject": subject,
            "field": field,
            "before": before,
            "after": after,
            "before_revision": before_revision,
            "after_revision": plan["revision"],
            "before_hash": before_hash,
            "after_hash": canonical_hash(plan),
            "cause": copy.deepcopy(cause),
        })

    def apply_questions(after_event: str | None) -> None:
        for question in case["dialogue"]:
            if question["after_event"] != after_event:
                continue
            if question["intent"] == "resolve_owner_priority":
                answer = question["answer"]
                selected = answer.get("selected_campaign")
                mode = answer.get("execution_mode")
                open_campaigns = {
                    row["id"] for row in plan["node"]
                    if row["kind"] == "campaign" and row["state"] not in TERMINAL
                }
                if selected in open_campaigns and mode in {"execute", "planning", "status"}:
                    mutate(
                        plan["plan_id"],
                        "current_node",
                        selected,
                        {
                            "type": "answer",
                            "dialogue_id": question["id"],
                            "after_event": after_event,
                        },
                    )

    apply_questions(None)
    for event in case["events"]:
        if event["type"] == "worker_report" and event["status"] == "PASS":
            mutate(
                event["node_id"],
                "state",
                "candidate",
                {"type": "event", "event_id": event["id"]},
            )
        apply_questions(event["id"])
    return plan, mutations


def check_trace(case: dict[str, Any], trace: dict[str, Any]) -> dict[str, Any]:
    """Compare observed behavior to the case oracle without invoking the model."""
    failures: list[dict[str, Any]] = []

    def fail(code: str, message: str, **details: Any) -> None:
        failures.append({"code": code, "message": message, **details})

    if trace.get("schema") != SCHEMA:
        fail("TRACE_SCHEMA", "trace schema must be 1")
    if trace.get("case_id") != case["id"] or trace.get("case_revision") != case["revision"]:
        fail("TRACE_IDENTITY", "trace case identity/revision does not match oracle")
    entries = trace.get("entries")
    if not isinstance(entries, list):
        entries = []
        fail("TRACE_ENTRIES", "trace entries must be an array")
    if len(entries) > case["limits"]["max_steps"]:
        fail("TRACE_STEP_LIMIT", "trace exceeds declared step bound")
    object_entries = [entry for entry in entries if isinstance(entry, dict)]
    entry_ids = [entry.get("entry_id") for entry in object_entries]
    if len(entry_ids) != len(set(entry_ids)):
        fail("TRACE_DUPLICATE_ID", "trace entry ids are not unique")
    for index, entry in enumerate(entries, 1):
        if not isinstance(entry, dict):
            fail("TRACE_ENTRY", "trace entry is not an object", index=index)
            continue
        if entry.get("seq") != index or entry.get("logical_time") != f"T+{index:04d}":
            fail("TRACE_ORDER", "trace sequence/logical time is not deterministic", index=index)

    questions = [entry for entry in object_entries if entry.get("kind") == "question"]
    answers = [entry for entry in object_entries if entry.get("kind") == "answer"]
    actions = [entry for entry in object_entries if entry.get("kind") == "action"]
    events = [entry for entry in object_entries if entry.get("kind") == "event"]
    expected_questions = case["dialogue"]
    if len(questions) != len(expected_questions):
        fail(
            "QUESTION_COUNT",
            "missing, extra or redundant questions",
            expected=len(expected_questions),
            actual=len(questions),
        )
    answer_groups: dict[Any, list[dict[str, Any]]] = {}
    for answer in answers:
        answer_groups.setdefault(answer.get("question_id"), []).append(answer)
    event_sequences = {entry.get("event_id"): entry.get("seq") for entry in events}
    question_bindings: dict[str, dict[str, Any]] = {}

    def dependent_action_patterns(
        question: dict[str, Any],
    ) -> list[tuple[str, list[str]]]:
        intent = question["intent"]
        answer = question["answer"]
        subject = question["subject"]
        if intent == "observe_external_effect" and answer.get("observed_status") == "complete":
            return [
                ("reconcile_external_effect", [subject[0]]),
                ("avoid_replay", [subject[0]]),
            ]
        if intent == "resolve_owner_priority":
            selected = answer.get("selected_campaign")
            mode = answer.get("execution_mode")
            if not isinstance(selected, str) or mode not in {"execute", "planning", "status"}:
                return []
            followup = {
                "execute": "continue_authorized_campaign",
                "planning": "refine_plan",
                "status": "report_status",
            }[mode]
            return [("select_campaign", [selected]), (followup, [selected])]
        if (intent == "missing_integration_owner"
                and isinstance(answer.get("integration_owner"), str)):
            return [
                ("bind_integration_owner", [subject[1]]),
                ("revise_bounded_packet", [subject[0]]),
            ]
        return []

    for index, expected in enumerate(expected_questions):
        if index >= len(questions):
            break
        actual = questions[index]
        oracle = {key: expected[key] for key in (
            "intent", "subject", "recipient", "required_information", "after_event"
        )}
        if semantic_question(actual) != oracle:
            fail("QUESTION_MISMATCH", "question meaning/subject/recipient/timing differs",
                 expected=oracle, actual=semantic_question(actual))
        preceding_events = [event for event in events if event.get("seq", 0) < actual.get("seq", 0)]
        actual_after = preceding_events[-1].get("event_id") if preceding_events else None
        if actual.get("after_event") != actual_after:
            fail("QUESTION_CAUSAL_POSITION", "question is not positioned after its claimed event",
                 question=expected["id"], claimed=actual.get("after_event"), actual=actual_after)
        if expected["after_event"] is not None:
            event_seq = event_sequences.get(expected["after_event"])
            if event_seq is None or actual.get("seq", 0) <= event_seq:
                fail("QUESTION_CAUSAL_POSITION", "question does not follow its causal event",
                     question=expected["id"])
        matches = answer_groups.get(actual.get("question_id"), [])
        if len(matches) != 1:
            code = "ANSWER_MISSING" if not matches else "ANSWER_COUNT"
            fail(code, "a question must have exactly one answer",
                 question=expected["id"], actual=len(matches))
            continue
        answer = matches[0]
        if answer.get("answer") != expected["answer"]:
            fail("ANSWER_MISMATCH", "released answer differs from the scripted answer",
                 question=expected["id"])
        if answer.get("recipient") != expected["recipient"]:
            fail("ANSWER_RECIPIENT", "answer recipient differs from the question recipient",
                 question=expected["id"])
        if answer.get("seq", 0) <= actual.get("seq", 0):
            fail("ANSWER_ORDER", "answer was released before its matching question",
                 question=expected["id"])
        intervening_event = any(
            event.get("seq", 0) > actual.get("seq", 0)
            and event.get("seq", 0) < answer.get("seq", 0)
            for event in events
        )
        if intervening_event:
            fail("ANSWER_CAUSAL_POSITION", "answer crossed a later event boundary",
                 question=expected["id"])
        causal_start = (
            event_sequences.get(expected["after_event"], 0)
            if expected["after_event"] is not None else 0
        )
        causal_end = min(
            (
                event.get("seq", len(entries) + 1)
                for event in events
                if event.get("seq", 0) > answer.get("seq", 0)
            ),
            default=len(entries) + 1,
        )
        question_bindings[expected["id"]] = {
            "question_id": actual.get("question_id"),
            "event_seq": causal_start,
            "question_seq": actual.get("seq", 0),
            "answer_seq": answer.get("seq", 0),
            "window_end": causal_end,
        }
        patterns = dependent_action_patterns(expected)
        premature = [
            action.get("intent") for action in actions
            if (action.get("intent"), action.get("subject")) in patterns
            and action.get("seq", 0) > causal_start
            and action.get("seq", 0) < answer.get("seq", 0)
        ]
        if premature:
            fail("ANSWER_CAUSAL_ORDER", "answer-dependent action occurred before the answer",
                 question=expected["id"], actions=premature)
        for intent, subject in patterns:
            positioned = [
                action for action in actions
                if action.get("intent") == intent
                and action.get("subject") == subject
                and answer.get("seq", 0) < action.get("seq", 0) < causal_end
            ]
            if not positioned:
                fail(
                    "ANSWER_ACTION_POSITION",
                    "answer-dependent action is absent from its causal event interval",
                    question=expected["id"],
                    intent=intent,
                    subject=subject,
                )
    question_ids = {entry.get("question_id") for entry in questions}
    for answer in answers:
        if answer.get("question_id") not in question_ids:
            fail("ANSWER_UNBOUND", "answer was released without a matching question")

    for pattern in case["expected"]["required_actions"]:
        if not any(subset(pattern, action) for action in actions):
            fail("ACTION_MISSING", "required action intent is absent", pattern=pattern)
    for pattern in case["expected"]["forbidden_actions"]:
        if any(subset(pattern, action) for action in actions):
            fail("ACTION_FORBIDDEN", "forbidden action intent is present", pattern=pattern)
    cursor = -1
    for intent in case["expected"]["required_action_order"]:
        found = next((index for index in range(cursor + 1, len(actions))
                      if actions[index].get("intent") == intent), None)
        if found is None:
            fail("ACTION_ORDER", "required causal action order is absent", intent=intent)
            break
        cursor = found

    kinds = {entry.get("kind") for entry in object_entries}
    for kind in case["expected"]["required_trace_kinds"]:
        if kind not in kinds:
            fail("TRACE_KIND_MISSING", "required structured trace kind is absent", kind=kind)
    final = trace.get("final", {})
    if not subset(case["expected"]["final"], final):
        fail("FINAL_STATE", "final state does not match the case oracle")

    roles = case["initial"]["roles"]
    source_events = {event["id"]: event for event in case["events"]}
    trace_questions = {entry.get("question_id"): entry for entry in questions}

    def execution_context(sequence: int) -> tuple[dict[str, Any], set[str]]:
        instruction = copy.deepcopy(case["initial"]["owner_instruction"])
        holds = set(case["initial"]["holds"])
        jobs = {job["id"]: job for job in case["initial"]["jobs"]}
        for entry in object_entries:
            if entry.get("seq", 0) >= sequence:
                break
            if entry.get("kind") == "event":
                event = source_events.get(entry.get("event_id"))
                if event and event["type"] == "owner_message":
                    instruction.update({
                        "mode": event["mode"],
                        "selected_campaign": event["selected_campaign"],
                        "authorization": event["authorization"],
                    })
                elif event and event["type"] == "job_observation" and event["status"] == "live":
                    job = jobs.get(event["job_id"])
                    if job and job.get("writer_role") == "contributor-central" and job.get("conflicts_with"):
                        holds.add(job["conflicts_with"])
            elif entry.get("kind") == "answer":
                question = trace_questions.get(entry.get("question_id"), {})
                answer = entry.get("answer", {})
                if question.get("intent") == "resolve_owner_priority" and isinstance(answer, dict):
                    mode = answer.get("execution_mode")
                    selected = answer.get("selected_campaign")
                    if mode in {"execute", "planning", "status"} and isinstance(selected, str):
                        instruction.update({
                            "mode": mode,
                            "selected_campaign": selected,
                            "authorization": mode == "execute",
                        })
            elif entry.get("kind") == "hold" and isinstance(entry.get("subject"), str):
                if entry.get("active") is True:
                    holds.add(entry["subject"])
                elif entry.get("active") is False:
                    holds.discard(entry["subject"])
        return instruction, holds

    for action in actions:
        actor = action.get("actor")
        authority = action.get("authority")
        grants = set(roles.get(actor, []))
        if actor not in roles:
            fail("ACTION_ACTOR_UNKNOWN", "action actor is absent from role bindings",
                 actor=actor, intent=action.get("intent"))
        if authority != "none" and authority not in grants:
            fail("AUTHORITY_UNGRANTED", "action claims an ungranted role",
                 actor=actor, authority=authority, intent=action.get("intent"))
        if action.get("intent") in EXECUTION_INTENTS and not (grants & COORDINATING_ROLES):
            fail("EXECUTION_ROLE_UNGRANTED", "campaign execution requires a coordinating actor grant",
                 actor=actor, intent=action.get("intent"))
        if action.get("intent") in {"continue_authorized_campaign", "run_panel_step"}:
            instruction, active_holds = execution_context(action.get("seq", 0))
            selected = instruction.get("selected_campaign")
            if (instruction.get("mode") != "execute"
                    or instruction.get("authorization") is not True
                    or not isinstance(selected, str)
                    or selected in active_holds):
                fail("EXECUTION_NOT_AUTHORIZED",
                     "execution action lacks active execute instruction or is held",
                     actor=actor, intent=action.get("intent"), selected=selected,
                     holds=sorted(active_holds))
            if (action.get("intent") == "continue_authorized_campaign"
                    and action.get("subject") != [selected]):
                fail("EXECUTION_SCOPE", "campaign continuation differs from selected owner scope",
                     actor=actor, subject=action.get("subject"), selected=selected)
        if action.get("intent") in {"accept_worker_result", "accept_campaign_closure"}:
            if "worker" in grants and not (COORDINATING_ROLES & grants):
                fail("WORKER_ACCEPTANCE", "worker action cannot establish central acceptance")
        if (action.get("intent") in PANEL_RESULT_INTENTS
                and not (COORDINATING_ROLES & grants)):
            fail(
                "PANEL_RESULT_ROLE_UNGRANTED",
                "panel acceptance or reuse requires a coordinating actor grant",
                actor=actor,
                intent=action.get("intent"),
            )

    event_ids = [entry.get("event_id") for entry in events]
    expected_event_ids = [event["id"] for event in case["events"]]
    if event_ids != expected_event_ids:
        fail("EVENT_COVERAGE", "ordered case events were skipped, added or reordered",
             expected=expected_event_ids, actual=event_ids)
    for entry, expected_event in zip(events, case["events"]):
        if (entry.get("event_type") != expected_event["type"]
                or entry.get("data") != expected_event):
            fail("EVENT_CONTENT", "trace event content differs from the ordered case event",
                 event_id=expected_event["id"])

    panel_steps = {
        step["id"]: step for step in case["initial"]["panel"].get("steps", [])
    }
    panel_sources: dict[str, dict[str, Any]] = {}
    panel_sources_by_gate: dict[str, list[dict[str, Any]]] = {}
    panel_input_changes: list[dict[str, Any]] = []
    rolling_panel_inputs = dict(case["initial"]["input_identities"])
    for event in case["events"]:
        if event["type"] == "input_change":
            panel_input_changes.append({
                "event": event,
                "seq": event_sequences.get(event["id"], 0),
            })
            rolling_panel_inputs[event["input_id"]] = event["after"]
            continue
        if event["type"] != "gate_result":
            continue
        panel_step = panel_steps.get(event["gate_id"])
        trusted_case_ids = list(panel_step["required_cases"]) if panel_step is not None else []
        trusted_required = (
            len(trusted_case_ids) if panel_step is not None else event["required_cases"]
        )
        complete = bool(
            panel_step is not None
            and event["status"] == "pass"
            and event["selected_cases"] == trusted_required
            and event["required_cases"] == trusted_required
            and trusted_required > 0
        )
        source = {
            "event": event,
            "seq": event_sequences.get(event["id"], 0),
            "trusted_case_ids": trusted_case_ids,
            "trusted_required": trusted_required,
            "complete": complete,
            "bound_input_ids": sorted(
                input_id for input_id, subject in rolling_panel_inputs.items()
                if subject == event["subject"]
            ),
        }
        panel_sources[event["id"]] = source
        panel_sources_by_gate.setdefault(event["gate_id"], []).append(source)

    for sources in panel_sources_by_gate.values():
        sources.sort(key=lambda source: source["seq"])

    result_actions = [
        action for action in actions if action.get("intent") in PANEL_RESULT_INTENTS
    ]
    attributed_actions: dict[str, list[dict[str, Any]]] = {
        event_id: [] for event_id in panel_sources
    }

    def invalidating_panel_change(
        source: dict[str, Any], upper_sequence: int
    ) -> dict[str, Any] | None:
        return next(
            (
                change for change in panel_input_changes
                if source["seq"] < change["seq"] < upper_sequence
                and change["event"]["input_id"] in source["bound_input_ids"]
                and change["event"]["before"] == source["event"]["subject"]
            ),
            None,
        )

    for action in result_actions:
        intent = action.get("intent")
        subject = action.get("subject")
        gate_id = subject[0] if isinstance(subject, list) and len(subject) == 1 else None
        details = action.get("details") if isinstance(action.get("details"), dict) else {}
        claimed_event_id = details.get("event_id")
        prior_sources = [
            source for source in panel_sources_by_gate.get(gate_id, [])
            if source["seq"] < action.get("seq", 0)
        ]
        source = panel_sources.get(claimed_event_id)
        if source is None and prior_sources:
            # Continue checking the latest applicable result so an omitted or forged
            # cause cannot create an unchecked interval after an unrelated event.
            source = prior_sources[-1]
        if claimed_event_id is None:
            initial_step = panel_steps.get(gate_id)
            initial_reuse = bool(
                intent == "reuse_panel_step"
                and not prior_sources
                and initial_step is not None
                and initial_step.get("status") == "pass"
                and initial_step.get("subject_valid") is True
            )
            if initial_reuse:
                continue
            fail(
                "PANEL_RESULT_CAUSE_MISSING",
                "panel acceptance or result-derived reuse lacks a gate_result event id",
                gate_id=gate_id,
                intent=intent,
            )
        elif claimed_event_id not in panel_sources:
            fail(
                "PANEL_RESULT_CAUSE_INVALID",
                "panel action claims an unknown or non-gate result event",
                event_id=claimed_event_id,
                gate_id=gate_id,
                intent=intent,
            )
        if source is None:
            continue

        event = source["event"]
        event_id = event["id"]
        if claimed_event_id == event_id:
            attributed_actions[event_id].append(action)
        if gate_id != event["gate_id"]:
            fail(
                "PANEL_RESULT_STEP_IDENTITY",
                "panel action subject differs from its claimed gate result step",
                event_id=event_id,
                expected_gate=event["gate_id"],
                actual_gate=gate_id,
            )
        if action.get("seq", 0) <= source["seq"]:
            fail(
                "PANEL_RESULT_CAUSAL_ORDER",
                "panel action does not follow its claimed gate result",
                event_id=event_id,
                gate_id=event["gate_id"],
            )
        superseding = next(
            (
                later for later in panel_sources_by_gate.get(event["gate_id"], [])
                if source["seq"] < later["seq"] < action.get("seq", 0)
            ),
            None,
        )
        if superseding is not None:
            fail(
                "PANEL_RESULT_SUPERSEDED",
                "panel action cites a result superseded by a newer result for the same step",
                event_id=event_id,
                superseding_event_id=superseding["event"]["id"],
                gate_id=event["gate_id"],
            )
        invalidating_change = invalidating_panel_change(source, action.get("seq", 0))
        if invalidating_change is not None:
            fail(
                "PANEL_RESULT_SUBJECT_CHANGED",
                "panel action reuses a result after its bound input subject changed",
                event_id=event_id,
                input_change_event_id=invalidating_change["event"]["id"],
                input_id=invalidating_change["event"]["input_id"],
                gate_id=event["gate_id"],
            )
        identity_matches = (
            claimed_event_id == event_id
            and details.get("selected_cases") == event["selected_cases"]
            and details.get("required_cases") == source["trusted_required"]
            and details.get("required_case_ids") == source["trusted_case_ids"]
            and details.get("subject") == event["subject"]
            and details.get("input_ids") == source["bound_input_ids"]
        )
        if not identity_matches:
            fail(
                "PANEL_DENOMINATOR_IDENTITY",
                "panel action does not bind its result, subject and immutable denominator",
                event_id=event_id,
                gate_id=event["gate_id"],
                intent=intent,
            )
        if not source["complete"]:
            code = (
                "PANEL_DENOMINATOR_REUSED"
                if intent == "reuse_panel_step" else "PANEL_DENOMINATOR_ACCEPTED"
            )
            fail(
                code,
                "incomplete panel result became accepted or reusable",
                event_id=event_id,
                gate_id=event["gate_id"],
                required_case_ids=source["trusted_case_ids"],
            )
        if event["selected_cases"] == 0 and intent != "reuse_panel_step":
            fail("ZERO_TESTS_ACCEPTED", "zero selected tests cannot establish a passing gate")
        if (event["selected_cases"] < event["required_cases"]
                and intent != "reuse_panel_step"):
            fail("SKIPPED_TESTS_ACCEPTED", "skipped required tests cannot establish a passing gate")

    for event_id, source in panel_sources.items():
        event = source["event"]
        event_actions = attributed_actions[event_id]
        accepted_actions = [
            action for action in event_actions
            if action.get("intent") in {"record_gate_result", "accept_gate"}
        ]
        reused_actions = [
            action for action in event_actions if action.get("intent") == "reuse_panel_step"
        ]
        if source["complete"] and not accepted_actions:
            fail(
                "PANEL_RESULT_NOT_ACCEPTED",
                "complete panel result has no acceptance bound to its event identity",
                event_id=event_id,
                gate_id=event["gate_id"],
            )
        if source["complete"] and not reused_actions:
            fail(
                "PANEL_RESULT_NOT_REUSED",
                "complete panel result has no reuse bound to its event identity",
                event_id=event_id,
                gate_id=event["gate_id"],
            )

    initial_panel = case["initial"]["panel"]
    final_panel = trace.get("final", {}).get("panel", {})
    if not isinstance(final_panel, dict) or final_panel.get("id") != initial_panel["id"]:
        fail(
            "PANEL_IDENTITY",
            "final panel identity differs from the immutable initial panel",
            expected=initial_panel["id"],
            actual=final_panel.get("id") if isinstance(final_panel, dict) else None,
        )
    final_panel_steps = final_panel.get("steps", []) if isinstance(final_panel, dict) else []
    if not isinstance(final_panel_steps, list):
        final_panel_steps = []
    initial_step_ids = [step["id"] for step in initial_panel.get("steps", [])]
    final_step_ids = [
        step.get("id") if isinstance(step, dict) else None
        for step in final_panel_steps
    ]
    if final_step_ids != initial_step_ids:
        fail(
            "PANEL_STEP_IDENTITIES",
            "final panel does not preserve the complete ordered initial step identity set",
            expected=initial_step_ids,
            actual=final_step_ids,
        )
    final_panel_by_id = {
        step.get("id"): step for step in final_panel_steps if isinstance(step, dict)
    }
    for initial_step in initial_panel.get("steps", []):
        final_step = final_panel_by_id.get(initial_step["id"])
        if final_step is None:
            continue
        if final_step.get("required_cases") != initial_step["required_cases"]:
            fail(
                "PANEL_STEP_DENOMINATOR",
                "final panel step changed its immutable required-case denominator",
                gate_id=initial_step["id"],
                expected=initial_step["required_cases"],
                actual=final_step.get("required_cases"),
            )

    projected_panel = {
        step["id"]: {
            "status": step["status"],
            "subject_valid": step["subject_valid"],
            "source": None,
        }
        for step in initial_panel.get("steps", [])
    }
    expected_panel_transitions: list[dict[str, Any]] = []
    panel_transition_boundaries = [
        entry for entry in object_entries
        if entry.get("kind") in {"event", "summary"}
        or (
            entry.get("kind") == "action"
            and entry.get("intent") == "run_panel_step"
        )
    ]

    def panel_transition_window(trigger: dict[str, Any]) -> tuple[int, int]:
        lower = trigger.get("seq", 0)
        upper = min(
            (
                boundary.get("seq", len(entries) + 1)
                for boundary in panel_transition_boundaries
                if boundary.get("seq", 0) > lower
            ),
            default=len(entries) + 1,
        )
        return lower, upper

    def project_panel_field(
        gate_id: str,
        field: str,
        after: Any,
        cause: dict[str, str] | None,
        trigger: dict[str, Any],
    ) -> None:
        before = projected_panel[gate_id][field]
        if before == after:
            return
        lower, upper = panel_transition_window(trigger)
        expected_panel_transitions.append({
            "identity": {
                "subject": gate_id,
                "field": field,
                "before": before,
                "after": after,
                "cause": copy.deepcopy(cause),
            },
            "trigger": {
                "kind": trigger.get("kind"),
                "entry_id": trigger.get("entry_id"),
                "event_id": trigger.get("event_id"),
                "intent": trigger.get("intent"),
            },
            "lower_bound": lower,
            "upper_bound": upper,
        })
        projected_panel[gate_id][field] = after

    for entry in object_entries:
        if entry.get("kind") == "event":
            event = source_events.get(entry.get("event_id"))
            if event is None:
                continue
            if event["type"] == "gate_result" and event["gate_id"] in projected_panel:
                source = panel_sources[event["id"]]
                gate_id = event["gate_id"]
                cause = {"kind": "event", "id": event["id"]}
                project_panel_field(
                    gate_id,
                    "status",
                    "pass" if source["complete"] else "failed",
                    cause,
                    entry,
                )
                project_panel_field(
                    gate_id, "subject_valid", source["complete"], cause, entry
                )
                projected_panel[gate_id]["source"] = source if source["complete"] else None
            elif event["type"] == "input_change":
                for gate_id, state in projected_panel.items():
                    source = state["source"]
                    if not (
                        source is not None
                        and event["input_id"] in source["bound_input_ids"]
                        and event["before"] == source["event"]["subject"]
                    ):
                        continue
                    cause = {"kind": "event", "id": event["id"]}
                    project_panel_field(
                        gate_id, "status", "invalidated", cause, entry
                    )
                    project_panel_field(
                        gate_id, "subject_valid", False, cause, entry
                    )
                    projected_panel[gate_id]["source"] = None
        elif entry.get("kind") == "action" and entry.get("intent") == "run_panel_step":
            subject = entry.get("subject")
            gate_id = subject[0] if isinstance(subject, list) and len(subject) == 1 else None
            if not isinstance(gate_id, str) or gate_id not in projected_panel:
                fail(
                    "PANEL_SCHEDULE_STEP",
                    "panel scheduling action does not name an immutable panel step",
                    gate_id=gate_id,
                )
                continue
            state = projected_panel[gate_id]
            details = entry.get("details") if isinstance(entry.get("details"), dict) else {}
            if details.get("previous_status") != state["status"]:
                fail(
                    "PANEL_SCHEDULE_PREVIOUS_STATE",
                    "panel scheduling action does not match the independently projected prior state",
                    gate_id=gate_id,
                    expected=state["status"],
                    actual=details.get("previous_status"),
                )
            if state["status"] == "pass" and state["subject_valid"] is True:
                fail(
                    "PANEL_VALID_RESULT_RESCHEDULED",
                    "a still-valid passing panel result cannot be scheduled again",
                    gate_id=gate_id,
                )
                continue
            project_panel_field(gate_id, "status", "scheduled", None, entry)

    observed_panel_transition_entries = [
        entry
        for entry in object_entries
        if entry.get("kind") == "transition"
        and isinstance(entry.get("subject"), str)
        and entry.get("subject") in projected_panel
        and entry.get("field") in {"status", "subject_valid"}
    ]
    observed_panel_transitions = [
        {
            "subject": entry.get("subject"),
            "field": entry.get("field"),
            "before": entry.get("before"),
            "after": entry.get("after"),
            "cause": entry.get("cause"),
        }
        for entry in observed_panel_transition_entries
    ]
    expected_panel_transition_identities = [
        expected["identity"] for expected in expected_panel_transitions
    ]
    if observed_panel_transitions != expected_panel_transition_identities:
        fail(
            "PANEL_TRANSITION_TRACE",
            "panel transition trace differs from independently projected events and scheduling",
            expected=expected_panel_transition_identities,
            actual=observed_panel_transitions,
        )
    else:
        for observed, expected in zip(
            observed_panel_transition_entries, expected_panel_transitions
        ):
            sequence = observed.get("seq", 0)
            if not expected["lower_bound"] < sequence < expected["upper_bound"]:
                fail(
                    "PANEL_TRANSITION_CAUSAL_ORDER",
                    "panel transition is outside its triggering event or scheduling interval",
                    transition=expected["identity"],
                    transition_seq=sequence,
                    trigger=expected["trigger"],
                    lower_bound=expected["lower_bound"],
                    upper_bound=expected["upper_bound"],
                )

    for gate_id, expected_state in projected_panel.items():
        final_step = final_panel_by_id.get(gate_id)
        if final_step is None:
            continue
        actual_state = {
            "status": final_step.get("status"),
            "subject_valid": final_step.get("subject_valid"),
        }
        projected_state = {
            "status": expected_state["status"],
            "subject_valid": expected_state["subject_valid"],
        }
        if actual_state.get("status") == "pass" and projected_state["status"] != "pass":
            fail(
                "PANEL_FINAL_UNSUPPORTED_PASS",
                "final panel step claims pass without an immutable initial or valid gate-result basis",
                gate_id=gate_id,
            )
        if actual_state != projected_state:
            fail(
                "PANEL_FINAL_STATE",
                "final panel step differs from independently projected valid transitions",
                gate_id=gate_id,
                expected=projected_state,
                actual=actual_state,
            )

    expected_final_plan, expected_mutations = expected_plan(case)
    plan = final.get("plan", {}) if isinstance(final, dict) else {}
    initial_plan = case["initial"]["plan"]
    initial_nodes = initial_plan["node"]
    final_nodes = plan.get("node", []) if isinstance(plan, dict) else []
    initial_ids = [row["id"] for row in initial_nodes]
    final_ids = [row.get("id") for row in final_nodes if isinstance(row, dict)]
    if final_ids != initial_ids:
        fail("PLAN_NODE_IDENTITIES", "complete ordered node identity set was not preserved",
             expected=initial_ids, actual=final_ids)
    final_by_id = {row.get("id"): row for row in final_nodes if isinstance(row, dict)}
    for initial_node in initial_nodes:
        actual = final_by_id.get(initial_node["id"])
        if actual is None:
            continue
        preserved = {key: value for key, value in initial_node.items() if key != "state"}
        actual_preserved = {key: value for key, value in actual.items() if key != "state"}
        if actual_preserved != preserved:
            fail("PLAN_NODE_PRESERVATION", "node identity or extension fields changed",
                 node=initial_node["id"])
    initial_extensions = {
        key: value for key, value in initial_plan.items()
        if key not in {"revision", "current_node", "node"}
    }
    final_extensions = {
        key: value for key, value in plan.items()
        if key not in {"revision", "current_node", "node"}
    } if isinstance(plan, dict) else {}
    if final_extensions != initial_extensions:
        fail("PLAN_EXTENSION_PRESERVATION", "top-level plan identity or extension fields changed")
    if plan != expected_final_plan:
        fail("PLAN_PROJECTION", "final plan differs from independently projected transitions")

    expected_candidates = sorted(
        row["id"] for row in final_nodes if isinstance(row, dict) and row.get("state") == "candidate"
    )
    if final.get("candidates") != expected_candidates:
        fail("CANDIDATE_PROJECTION", "candidate projection differs from node states",
             expected=expected_candidates, actual=final.get("candidates"))
    expected_accepted = [
        row["id"] for row in final_nodes if isinstance(row, dict) and row.get("state") == "accepted"
    ]
    summaries = [entry for entry in object_entries if entry.get("kind") == "summary"]
    if len(summaries) != 1:
        fail("SUMMARY_COUNT", "trace must contain exactly one summary", actual=len(summaries))
    else:
        summary = summaries[0]
        if summary.get("candidates") != expected_candidates:
            fail("SUMMARY_CANDIDATES", "summary candidates differ from final node states")
        if summary.get("accepted") != expected_accepted:
            fail("SUMMARY_ACCEPTED", "summary accepted nodes differ from final node states")
    expected_route = ordered_route(expected_final_plan)
    if final.get("complete_route") != expected_route:
        fail("COMPLETE_ROUTE", "complete route is not the independent full-plan projection",
             expected=expected_route, actual=final.get("complete_route"))

    history = final.get("plan_history", []) if isinstance(final, dict) else []

    def expected_cause_ref(mutation: dict[str, Any]) -> dict[str, str] | None:
        cause = mutation["cause"]
        if cause["type"] == "event":
            return {"kind": "event", "id": cause["event_id"]}
        binding = question_bindings.get(cause["dialogue_id"])
        if binding is None or not isinstance(binding.get("question_id"), str):
            return None
        return {"kind": "question", "id": binding["question_id"]}

    if not isinstance(history, list) or len(history) != len(expected_mutations):
        fail("PLAN_HISTORY_COUNT", "plan history does not cover every plan mutation",
             expected=len(expected_mutations), actual=len(history) if isinstance(history, list) else None)
    else:
        for index, (actual, expected) in enumerate(zip(history, expected_mutations)):
            if not isinstance(actual, dict):
                fail("PLAN_HISTORY_ENTRY", "plan history entry is not an object", index=index)
                continue
            for field in PLAN_MUTATION_IDENTITY_FIELDS:
                value = expected[field]
                if actual.get(field) != value:
                    fail("PLAN_HISTORY_IDENTITY", "plan history before/after identity differs",
                         index=index, field=field, expected=value, actual=actual.get(field))
            expected_cause = expected_cause_ref(expected)
            if expected_cause is not None and actual.get("cause") != expected_cause:
                fail(
                    "PLAN_HISTORY_CAUSE",
                    "plan history mutation does not bind its actual question or event cause",
                    index=index,
                    expected=expected_cause,
                    actual=actual.get("cause"),
                )
            if not isinstance(actual.get("rationale"), str) or not actual["rationale"]:
                fail("PLAN_HISTORY_RATIONALE", "plan mutation lacks a rationale", index=index)
    node_ids = set(initial_ids)
    plan_field_transitions = [
        entry for entry in object_entries
        if entry.get("kind") == "transition"
        and ((entry.get("subject") in node_ids and entry.get("field") == "state")
             or (entry.get("subject") == initial_plan["plan_id"]
                 and entry.get("field") == "current_node"))
    ]
    plan_revision_transitions = [
        entry for entry in object_entries
        if entry.get("kind") == "transition"
        and entry.get("subject") == initial_plan["plan_id"]
        and entry.get("field") == "plan_revision"
    ]
    for mutation in expected_mutations:
        matching = [entry for entry in plan_field_transitions if all(
            entry.get(field) == mutation[field]
            for field in (
                "subject", "field", "before", "after", "before_revision", "after_revision",
            )
        )]
        revision_matching = [
            entry for entry in plan_revision_transitions
            if entry.get("before") == mutation["before_revision"]
            and entry.get("after") == mutation["after_revision"]
            and entry.get("before_revision") == mutation["before_revision"]
            and entry.get("after_revision") == mutation["after_revision"]
        ]
        if len(matching) != 1:
            fail("PLAN_TRANSITION_TRACE", "plan mutation lacks one matching transition",
                 subject=mutation["subject"], field=mutation["field"], actual=len(matching))
        if len(revision_matching) != 1:
            fail(
                "PLAN_REVISION_TRANSITION_TRACE",
                "plan mutation lacks one matching revision transition",
                subject=mutation["subject"],
                before_revision=mutation["before_revision"],
                after_revision=mutation["after_revision"],
                actual=len(revision_matching),
            )
        if len(matching) != 1 or len(revision_matching) != 1:
            continue

        field_transition = matching[0]
        revision_transition = revision_matching[0]
        cause = mutation["cause"]
        expected_cause = expected_cause_ref(mutation)
        if expected_cause is not None:
            for transition in (field_transition, revision_transition):
                if transition.get("cause") != expected_cause:
                    fail(
                        "PLAN_TRANSITION_CAUSE",
                        "plan transition does not bind its actual question or event cause",
                        subject=mutation["subject"],
                        before_revision=mutation["before_revision"],
                        after_revision=mutation["after_revision"],
                        expected=expected_cause,
                        actual=transition.get("cause"),
                    )
        if cause["type"] == "answer":
            binding = question_bindings.get(cause["dialogue_id"])
            if binding is None:
                continue
            lower = binding["answer_seq"]
            upper = binding["window_end"]
            code = "ANSWER_TRANSITION_ORDER"
            message = "answer-dependent plan transitions are outside their causal event interval"
        else:
            lower = event_sequences.get(cause["event_id"], 0)
            upper = min(
                (
                    entry.get("seq", len(entries) + 1)
                    for entry in events
                    if entry.get("seq", 0) > lower
                ),
                default=len(entries) + 1,
            )
            code = "EVENT_TRANSITION_ORDER"
            message = "event-dependent plan transitions are outside their triggering event interval"
        sequences = [field_transition.get("seq", 0), revision_transition.get("seq", 0)]
        if (not all(lower < sequence < upper for sequence in sequences)
                or sequences[0] >= sequences[1]):
            fail(
                code,
                message,
                cause=cause,
                transitions=[
                    {
                        "subject": field_transition.get("subject"),
                        "field": field_transition.get("field"),
                        "seq": field_transition.get("seq"),
                    },
                    {
                        "subject": revision_transition.get("subject"),
                        "field": revision_transition.get("field"),
                        "seq": revision_transition.get("seq"),
                    },
                ],
                lower_bound=lower,
                upper_bound=upper,
            )
    if len(plan_field_transitions) != len(expected_mutations):
        fail("PLAN_TRANSITION_COUNT", "trace contains an unprojected or missing plan transition",
             expected=len(expected_mutations), actual=len(plan_field_transitions))
    if len(plan_revision_transitions) != len(expected_mutations):
        fail(
            "PLAN_REVISION_TRANSITION_COUNT",
            "trace contains an unprojected or missing revision transition",
            expected=len(expected_mutations),
            actual=len(plan_revision_transitions),
        )

    for node_id, node in final_by_id.items():
        if node.get("state") != "accepted":
            continue
        open_children = [row.get("id") for row in final_nodes
                         if row.get("parent") == node_id and row.get("state") not in TERMINAL]
        if open_children:
            fail("FALSE_PARENT_CLOSURE", "accepted parent has open required children",
                 node=node_id, children=open_children)

    changed_inputs = {
        event["input_id"]: event["after"] for event in case["events"]
        if event["type"] == "input_change"
    }
    for evidence in final.get("evidence", []) if isinstance(final, dict) else []:
        input_id = evidence.get("input_id")
        if input_id in changed_inputs and evidence.get("subject") != changed_inputs[input_id]:
            if evidence.get("valid", True):
                fail("STALE_EVIDENCE_REUSED", "changed evidence subject remained valid",
                     evidence=evidence.get("id"))

    verdict = "pass" if not failures else "fail"
    return {
        "schema": SCHEMA,
        "case_id": case["id"],
        "case_revision": case["revision"],
        "verdict": verdict,
        "failures": failures,
        "counts": {
            "entries": len(entries),
            "questions": len(questions),
            "answers": len(answers),
            "actions": len(actions),
            "events": len(events),
        },
    }
