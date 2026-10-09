#!/usr/bin/env python3
"""Pure deterministic model of selected multi-user-planning decisions.

The model consumes only normalized synthetic state, ordered synthetic events and
answers supplied after questions. It has no filesystem, clock, process, network
or personal-state integration.
"""

from __future__ import annotations

import copy
import hashlib
import json
from typing import Any, Callable


SCHEMA = 1
TERMINAL = {"accepted", "deferred", "dropped", "superseded"}
QuestionHandler = Callable[[dict[str, Any]], dict[str, Any] | None]


class SimulationError(RuntimeError):
    """A bounded deterministic simulation refusal."""

    def __init__(self, code: str, message: str):
        super().__init__(message)
        self.code = code


def canonical_hash(value: Any) -> str:
    return hashlib.sha256(
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()


def subset_copy(value: Any) -> Any:
    return copy.deepcopy(value)


class Trace:
    def __init__(self, case_id: str, revision: int, max_steps: int):
        self.case_id = case_id
        self.revision = revision
        self.max_steps = max_steps
        self.entries: list[dict[str, Any]] = []

    def add(self, kind: str, **fields: Any) -> dict[str, Any]:
        sequence = len(self.entries) + 1
        if sequence > self.max_steps:
            raise SimulationError("STEP_LIMIT", f"simulation exceeded {self.max_steps} steps")
        entry = {
            "seq": sequence,
            "logical_time": f"T+{sequence:04d}",
            "entry_id": f"{self.case_id}:trace:{sequence:04d}",
            "kind": kind,
            **subset_copy(fields),
        }
        self.entries.append(entry)
        return entry


class CampaignModel:
    """Rule model intentionally independent from every case's expected oracle."""

    def initialize(self, initial: dict[str, Any]) -> dict[str, Any]:
        state = copy.deepcopy(initial)
        state["plan_history"] = []
        state["unresolved_obligations"] = list(state["obligations"])
        state["answered_questions"] = []
        state["priority_question_pending"] = (
            state["owner_instruction"]["selected_campaign"] is None
        )
        state["processed_events"] = []
        state["actions"] = []
        state["panel_results"] = {}
        return state

    def run(
        self,
        *,
        case_id: str,
        revision: int,
        initial: dict[str, Any],
        events: list[dict[str, Any]],
        limits: dict[str, int],
        question_handler: QuestionHandler,
    ) -> dict[str, Any]:
        if len(events) > limits["max_events"]:
            raise SimulationError("EVENT_LIMIT", "case events exceed the declared bound")
        state = self.initialize(initial)
        trace = Trace(case_id, revision, limits["max_steps"])
        self._begin_decision_epoch(state)
        self._record_initial_state(state, trace)
        self._resolve_questions(state, trace, question_handler, after_event=None)
        self._evaluate(state, trace)
        for event in events:
            self._begin_decision_epoch(state)
            trace.add("event", event_id=event["id"], event_type=event["type"], data=event)
            state["processed_events"].append(event["id"])
            self._apply_event(state, trace, event)
            self._resolve_questions(state, trace, question_handler, after_event=event["id"])
            self._evaluate(state, trace)
        self._finalize(state, trace)
        return {
            "schema": SCHEMA,
            "case_id": case_id,
            "case_revision": revision,
            "trace_id": f"{case_id}:r{revision}:{canonical_hash(initial)[:16]}",
            "fixed_time_origin": "T+0000",
            "entries": trace.entries,
            "final": self._public_final(state),
        }

    def _record_initial_state(self, state: dict[str, Any], trace: Trace) -> None:
        for actor in sorted(state["roles"]):
            trace.add("authority", actor=actor, grants=sorted(state["roles"][actor]))
        for subject in sorted(state["holds"]):
            trace.add("hold", subject=subject, active=True)
        for evidence in sorted(state["evidence"], key=lambda row: row["id"]):
            trace.add("evidence", evidence=evidence, disposition="observed")
        for obligation in state["unresolved_obligations"]:
            trace.add("obligation", obligation=obligation, status="unresolved")
        self._action(state, trace, "inspect_complete_plan", [state["plan"]["plan_id"]])

    def _question(
        self,
        state: dict[str, Any],
        trace: Trace,
        handler: QuestionHandler,
        *,
        intent: str,
        subject: list[str],
        recipient: str,
        required_information: list[str],
        after_event: str | None,
    ) -> tuple[dict[str, Any] | None, str]:
        semantic = {
            "intent": intent,
            "subject": list(subject),
            "recipient": recipient,
            "required_information": list(required_information),
            "after_event": after_event,
        }
        question_key = canonical_hash(semantic)[:12]
        question_id = f"{state['plan']['plan_id']}:q:{intent}:{question_key}"
        trace.add("question", question_id=question_id, **semantic)
        answer = handler(subset_copy(semantic))
        if answer is None:
            obligation = f"unanswered-question:{intent}"
            self._obligation(state, trace, obligation)
            return None, question_id
        trace.add("answer", question_id=question_id, recipient=recipient, answer=answer)
        state["answered_questions"].append(intent)
        return subset_copy(answer), question_id

    def _resolve_questions(
        self,
        state: dict[str, Any],
        trace: Trace,
        handler: QuestionHandler,
        *,
        after_event: str | None,
    ) -> None:
        for operation in state["operations"]:
            if operation.get("status") == "unknown" and not operation.get("question_asked"):
                operation["question_asked"] = True
                answer, _ = self._question(
                    state,
                    trace,
                    handler,
                    intent="observe_external_effect",
                    subject=[operation["id"]],
                    recipient="environment",
                    required_information=["observed_status", "evidence_subject"],
                    after_event=after_event,
                )
                if answer is not None:
                    old = operation["status"]
                    operation["status"] = answer.get("observed_status", "unknown")
                    operation["evidence_subject"] = answer.get("evidence_subject")
                    self._transition(trace, operation["id"], "status", old, operation["status"])
                    if operation["status"] == "complete":
                        self._action(
                            state,
                            trace,
                            "reconcile_external_effect",
                            [operation["id"]],
                            details={"result": "complete", "evidence_subject": operation["evidence_subject"]},
                        )
                        self._action(state, trace, "avoid_replay", [operation["id"]])

        instruction = state["owner_instruction"]
        open_campaigns = [
            row["id"] for row in state["plan"]["node"]
            if row["kind"] == "campaign" and row["state"] not in TERMINAL
        ]
        if (instruction["selected_campaign"] is None and len(open_campaigns) > 1
                and state["priority_question_pending"]):
            state["priority_question_pending"] = False
            answer, question_id = self._question(
                state,
                trace,
                handler,
                intent="resolve_owner_priority",
                subject=sorted(open_campaigns),
                recipient="owner",
                required_information=["selected_campaign", "execution_mode"],
                after_event=after_event,
            )
            if answer is not None:
                selected = answer.get("selected_campaign")
                mode = answer.get("execution_mode")
                if selected in open_campaigns and mode in {"execute", "planning", "status"}:
                    instruction["selected_campaign"] = selected
                    instruction["mode"] = mode
                    instruction["authorization"] = mode == "execute"
                    self._action(
                        state,
                        trace,
                        "select_campaign",
                        [selected],
                        actor="owner",
                        authority="owner",
                        details={"authority_source": answer.get("authority_source", "owner-answer")},
                    )
                    self._plan_transition(
                        state,
                        trace,
                        selected,
                        "owner priority resolved",
                        cause={"kind": "question", "id": question_id},
                    )
                else:
                    self._obligation(state, trace, "owner-priority-answer-invalid")

        if "worker" in state["roles"].get(state["actor"], []):
            for packet in state["packets"]:
                if packet.get("integration_owner") is None and not packet.get("question_asked"):
                    packet["question_asked"] = True
                    answer, _ = self._question(
                        state,
                        trace,
                        handler,
                        intent="missing_integration_owner",
                        subject=[packet["task_id"], packet["shared_path"]],
                        recipient="central",
                        required_information=["integration_owner"],
                        after_event=after_event,
                    )
                    if answer is not None and isinstance(answer.get("integration_owner"), str):
                        packet["integration_owner"] = answer["integration_owner"]
                        self._action(
                            state,
                            trace,
                            "bind_integration_owner",
                            [packet["shared_path"]],
                            actor=packet["integration_owner"],
                            authority="contributor-central",
                            details={"owner": packet["integration_owner"]},
                        )
                        self._action(
                            state,
                            trace,
                            "revise_bounded_packet",
                            [packet["task_id"]],
                            details={"worker_scope_expanded": False},
                        )

    def _apply_event(self, state: dict[str, Any], trace: Trace, event: dict[str, Any]) -> None:
        kind = event["type"]
        if kind == "owner_message":
            instruction = state["owner_instruction"]
            before = copy.deepcopy(instruction)
            instruction.update({
                "mode": event["mode"],
                "selected_campaign": event["selected_campaign"],
                "authorization": event["authorization"],
            })
            state["priority_question_pending"] = event["selected_campaign"] is None
            self._transition(trace, "owner_instruction", "value", before, instruction)
            self._action(
                state,
                trace,
                "record_owner_instruction",
                [event["id"]],
                details={"authority_source": "owner_message"},
            )
        elif kind == "job_observation":
            job = next((row for row in state["jobs"] if row["id"] == event["job_id"]), None)
            if job is None:
                self._obligation(state, trace, f"unknown-job:{event['job_id']}")
            else:
                old = job["status"]
                job["status"] = event["status"]
                self._transition(trace, job["id"], "status", old, job["status"])
        elif kind == "gate_result":
            selected = event["selected_cases"]
            reported_required = event["required_cases"]
            step = next(
                (row for row in state["panel"].get("steps", []) if row["id"] == event["gate_id"]),
                None,
            )
            required_case_ids = list(step["required_cases"]) if step is not None else []
            required = len(required_case_ids) if step is not None else reported_required
            denominator_matches = reported_required == required
            bound_input_ids = sorted(
                input_id for input_id, subject in state["input_identities"].items()
                if subject == event["subject"]
            )
            valid = (
                event["status"] == "pass"
                and selected > 0
                and selected == required
                and denominator_matches
            )
            result_identity = {
                "event_id": event["id"],
                "selected_cases": selected,
                "required_cases": required,
                "required_case_ids": required_case_ids,
                "subject": event["subject"],
                "input_ids": bound_input_ids,
            }
            if step is not None:
                state["panel_results"][event["gate_id"]] = copy.deepcopy(result_identity)
            if valid:
                self._action(
                    state, trace, "record_gate_result", [event["gate_id"]],
                    details=result_identity,
                )
                self._resolve_obligation(
                    state, trace, f"gate-incomplete:{event['gate_id']}"
                )
            else:
                if selected == 0:
                    reason = "zero-selected-tests"
                elif not denominator_matches:
                    reason = "panel-denominator-mismatch"
                elif selected != required:
                    reason = "skipped-required-tests"
                else:
                    reason = "gate-did-not-pass"
                self._action(
                    state, trace, "reject_gate", [event["gate_id"]],
                    details={"event_id": event["id"],
                             "reason": reason, "selected_cases": selected,
                             "required_cases": required,
                             "reported_required_cases": reported_required,
                             "required_case_ids": required_case_ids},
                )
                self._obligation(state, trace, f"gate-incomplete:{event['gate_id']}")
            if step is not None:
                old_status = step["status"]
                old_subject_valid = step["subject_valid"]
                step["status"] = "pass" if valid else "failed"
                step["subject_valid"] = valid
                cause = {"kind": "event", "id": event["id"]}
                self._transition(
                    trace, step["id"], "status", old_status, step["status"],
                    cause=cause,
                )
                self._transition(
                    trace,
                    step["id"],
                    "subject_valid",
                    old_subject_valid,
                    step["subject_valid"],
                    cause=cause,
                )
        elif kind == "input_change":
            state["input_identities"][event["input_id"]] = event["after"]
            self._transition(trace, event["input_id"], "identity", event["before"], event["after"])
            if event["expected"]:
                self._action(
                    state, trace, "recapture_input_snapshot", [event["input_id"]],
                    details={"identity": event["after"], "permission_recheck": False},
                )
            for evidence in state["evidence"]:
                if evidence.get("input_id") == event["input_id"] and evidence.get("subject") != event["after"]:
                    evidence["valid"] = False
                    self._action(
                        state, trace, "invalidate_evidence", [evidence["id"]],
                        details={"expected_subject": event["after"],
                                 "recorded_subject": evidence.get("subject")},
                    )
                    self._obligation(state, trace, f"rerun-evidence:{evidence['id']}")
            for step in state["panel"].get("steps", []):
                result = state["panel_results"].get(step["id"])
                if not (
                    result
                    and step["status"] == "pass"
                    and step.get("subject_valid") is True
                    and event["input_id"] in result.get("input_ids", [])
                    and result.get("subject") == event["before"]
                ):
                    continue
                old_status = step["status"]
                old_subject_valid = step["subject_valid"]
                step["status"] = "invalidated"
                step["subject_valid"] = False
                del state["panel_results"][step["id"]]
                self._action(
                    state,
                    trace,
                    "invalidate_panel_result",
                    [step["id"]],
                    details={
                        "gate_event_id": result["event_id"],
                        "input_change_event_id": event["id"],
                        "input_id": event["input_id"],
                        "before_subject": event["before"],
                        "after_subject": event["after"],
                    },
                )
                cause = {"kind": "event", "id": event["id"]}
                self._transition(
                    trace, step["id"], "status", old_status, step["status"],
                    cause=cause,
                )
                self._transition(
                    trace,
                    step["id"],
                    "subject_valid",
                    old_subject_valid,
                    step["subject_valid"],
                    cause=cause,
                )
                self._obligation(state, trace, f"gate-incomplete:{step['id']}")
        elif kind == "worker_report":
            self._action(
                state, trace, "record_worker_report", [event["node_id"]],
                actor=event["worker_id"], authority="worker",
                details={"status": event["status"], "evidence_id": event["evidence_id"]},
            )
            if event["status"] == "PASS":
                self._set_node_state(
                    state,
                    trace,
                    event["node_id"],
                    "candidate",
                    "worker PASS recorded as candidate",
                    cause={"kind": "event", "id": event["id"]},
                )
                self._action(state, trace, "require_central_review", [event["node_id"]])
                self._obligation(state, trace, f"central-review:{event['node_id']}")
        elif kind == "acceptance_attempt":
            grants = state["roles"].get(event["actor"], [])
            if "worker" in grants and event["tier"] in {"central-reviewed", "local-gated", "project-accepted"}:
                self._action(
                    state, trace, "reject_unauthorized_acceptance", [event["node_id"]],
                    actor=event["actor"], authority="worker",
                    details={"attempted_tier": event["tier"]},
                )
                self._obligation(state, trace, f"central-review:{event['node_id']}")
            else:
                self._action(state, trace, "record_acceptance_attempt", [event["node_id"]])
        elif kind == "closure_request":
            state["closure_requested"] = True

    def _evaluate(self, state: dict[str, Any], trace: Trace) -> None:
        self._recovery(state, trace)
        self._holds(state, trace)
        self._instruction(state, trace)
        self._dependencies(state, trace)
        self._parallel(state, trace)
        self._panel(state, trace)
        self._closure(state, trace)

    def _recovery(self, state: dict[str, Any], trace: Trace) -> None:
        for job in state["jobs"]:
            if job.get("status") != "live":
                continue
            conflicting_writer = (
                job.get("writer_role") == "contributor-central"
                and bool(job.get("conflicts_with"))
            )
            if not state["recovery"] and not conflicting_writer:
                continue
            self._action(state, trace, "reconcile_live_job", [job["id"]])
            if conflicting_writer:
                conflict = job["conflicts_with"]
                if conflict not in state["holds"]:
                    state["holds"].append(conflict)
                    trace.add("hold", subject=conflict, active=True,
                              reason="observed-live-central-conflict")
                self._action(
                    state, trace, "hold_affected_writes", [conflict],
                    details={"writer": job["actor_id"], "job_id": job["id"]},
                )
                self._obligation(state, trace, f"owner-resolution:live-writer:{job['actor_id']}")
            else:
                self._action(state, trace, "avoid_duplicate_worker", [job["id"]])
        if not state["recovery"]:
            return
        if state["candidates"]:
            self._action(state, trace, "preserve_candidates", sorted(state["candidates"]))
        valid_evidence = sorted(row["id"] for row in state["evidence"] if row.get("valid", True))
        if valid_evidence:
            self._action(state, trace, "preserve_accepted_evidence", valid_evidence)
        self._action(
            state,
            trace,
            "preserve_complete_route",
            [row["id"] for row in self._ordered_nodes(state["plan"])],
        )
        self._action(state, trace, "avoid_custody_ritual", [state["plan"]["plan_id"]])

    def _holds(self, state: dict[str, Any], trace: Trace) -> None:
        if state["holds"]:
            for hold in sorted(state["holds"]):
                self._action(state, trace, "preserve_execution_hold", [hold])

    def _instruction(self, state: dict[str, Any], trace: Trace) -> None:
        instruction = state["owner_instruction"]
        selected = instruction["selected_campaign"]
        if not selected:
            return
        if instruction["mode"] == "planning":
            if self._actor_can_coordinate(state):
                self._action(
                    state,
                    trace,
                    "refine_plan",
                    [selected],
                    details={"authority_source": "owner_instruction"},
                )
            else:
                self._obligation(state, trace, f"coordination-role-required:{selected}")
        elif instruction["mode"] == "status":
            self._action(state, trace, "report_status", [selected])
        elif self._execution_allowed(state, selected):
            self._action(
                state,
                trace,
                "continue_authorized_campaign",
                [selected],
                details={"authority_source": "owner_instruction"},
            )
        else:
            self._obligation(state, trace, f"execution-not-authorized:{selected}")

    def _dependencies(self, state: dict[str, Any], trace: Trace) -> None:
        by_id = {row["id"]: row for row in state["plan"]["node"]}
        for node in self._ordered_nodes(state["plan"]):
            required_tier = node.get("required_prerequisite_tier")
            if required_tier != "accepted" or node["state"] in TERMINAL:
                continue
            unmet = [dep for dep in node["depends_on"] if by_id[dep]["state"] != "accepted"]
            if unmet:
                self._action(
                    state, trace, "block_dependent_execution", [node["id"]],
                    details={"required_tier": "accepted", "unmet": unmet},
                )
                self._obligation(state, trace, f"accepted-prerequisite:{node['id']}:{','.join(unmet)}")
                if node.get("preparation_inputs_available"):
                    self._action(state, trace, "allow_read_only_preparation", [node["id"]])

    def _parallel(self, state: dict[str, Any], trace: Trace) -> None:
        parallel = state["parallel"]
        if state.get("parallel_evaluated") or not parallel.get("packets"):
            return
        state["parallel_evaluated"] = True
        for conflict in parallel.get("conflicts", []):
            self._action(
                state, trace, "serialize_parallel_conflict", conflict["packets"],
                details={"code": conflict["code"], "subject": conflict["subject"]},
            )
        if parallel.get("advisory_batches"):
            self._action(
                state, trace, "treat_frontier_as_advisory",
                sorted(item for batch in parallel["advisory_batches"] for item in batch),
                details={"dispatch_authorized": False},
            )
        if parallel.get("review_queue"):
            self._action(state, trace, "preserve_review_backlog", parallel["review_queue"])

    def _panel(self, state: dict[str, Any], trace: Trace) -> None:
        panel = state["panel"]
        if not panel.get("steps"):
            return
        step_ids = [row["id"] for row in panel["steps"]]
        self._action(
            state, trace, "preserve_panel_denominator", [panel["id"]],
            details={"steps": step_ids, "required_count": len(step_ids)},
        )
        selected = state["owner_instruction"].get("selected_campaign")
        may_schedule = bool(selected and self._execution_allowed(state, selected))
        for step in panel["steps"]:
            if step["status"] == "pass" and step.get("subject_valid"):
                result_identity = state["panel_results"].get(step["id"])
                self._action(
                    state,
                    trace,
                    "reuse_panel_step",
                    [step["id"]],
                    details=result_identity,
                )
            elif step["status"] == "scheduled":
                self._action(
                    state,
                    trace,
                    "report_panel_step_owed",
                    [step["id"]],
                    details={"status": "scheduled", "execution_authorized": may_schedule},
                )
            elif not may_schedule:
                self._action(
                    state,
                    trace,
                    "report_panel_step_owed",
                    [step["id"]],
                    details={
                        "status": step["status"],
                        "execution_authorized": False,
                        "reason": self._execution_block_reason(state, selected),
                    },
                )
            else:
                self._action(
                    state, trace, "run_panel_step", [step["id"]],
                    details={"previous_status": step["status"]},
                )
                old_status = step["status"]
                step["status"] = "scheduled"
                self._transition(trace, step["id"], "status", old_status, step["status"])
        self._action(
            state, trace, "report_aggregate_panel", [panel["id"]],
            details={"uninterrupted_script_claim": False},
        )

    def _closure(self, state: dict[str, Any], trace: Trace) -> None:
        if not state["closure_requested"]:
            return
        open_nodes = [
            row["id"] for row in self._ordered_nodes(state["plan"])
            if row["state"] not in TERMINAL
        ]
        panel_open = any(step["status"] != "pass" for step in state["panel"].get("steps", []))
        if open_nodes or state["unresolved_obligations"] or panel_open:
            self._action(
                state,
                trace,
                "preserve_complete_route",
                [row["id"] for row in self._ordered_nodes(state["plan"])],
            )
            self._action(
                state, trace, "refuse_campaign_closure", [state["plan"]["plan_id"]],
                details={"open_nodes": open_nodes, "panel_open": panel_open},
            )
            for node_id in open_nodes:
                self._obligation(state, trace, f"open-node:{node_id}")
        else:
            self._action(state, trace, "accept_campaign_closure", [state["plan"]["plan_id"]])

    def _finalize(self, state: dict[str, Any], trace: Trace) -> None:
        for obligation in state["unresolved_obligations"]:
            trace.add("obligation", obligation=obligation, status="unresolved-final")
        trace.add(
            "summary",
            route=[row["id"] for row in self._ordered_nodes(state["plan"])],
            unresolved_obligations=list(state["unresolved_obligations"]),
            accepted=[row["id"] for row in state["plan"]["node"] if row["state"] == "accepted"],
            candidates=[row["id"] for row in state["plan"]["node"] if row["state"] == "candidate"],
        )

    def _action(
        self,
        state: dict[str, Any],
        trace: Trace,
        intent: str,
        subject: list[str],
        *,
        actor: str | None = None,
        authority: str | None = None,
        details: dict[str, Any] | None = None,
    ) -> None:
        action = {
            "intent": intent,
            "subject": list(subject),
            "actor": actor or state["actor"],
            "authority": authority or self._primary_authority(state, actor or state["actor"]),
            "details": details or {},
        }
        if action not in state["actions"]:
            state["actions"].append(copy.deepcopy(action))
            trace.add("action", **action)

    @staticmethod
    def _begin_decision_epoch(state: dict[str, Any]) -> None:
        """Deduplicate only within one initial/event-driven decision epoch."""
        state["actions"] = []

    def _obligation(self, state: dict[str, Any], trace: Trace, obligation: str) -> None:
        if obligation not in state["unresolved_obligations"]:
            state["unresolved_obligations"].append(obligation)
            trace.add("obligation", obligation=obligation, status="opened")

    def _resolve_obligation(
        self, state: dict[str, Any], trace: Trace, obligation: str
    ) -> None:
        if obligation in state["unresolved_obligations"]:
            state["unresolved_obligations"].remove(obligation)
            trace.add("obligation", obligation=obligation, status="resolved")

    def _transition(
        self,
        trace: Trace,
        subject: str,
        field: str,
        before: Any,
        after: Any,
        *,
        cause: dict[str, str] | None = None,
        before_revision: int | None = None,
        after_revision: int | None = None,
    ) -> None:
        if before != after:
            fields: dict[str, Any] = {
                "subject": subject,
                "field": field,
                "before": before,
                "after": after,
            }
            if cause is not None:
                fields["cause"] = subset_copy(cause)
            if before_revision is not None:
                fields["before_revision"] = before_revision
            if after_revision is not None:
                fields["after_revision"] = after_revision
            trace.add("transition", **fields)

    def _plan_transition(
        self,
        state: dict[str, Any],
        trace: Trace,
        current: str,
        reason: str,
        *,
        cause: dict[str, str],
    ) -> None:
        plan = state["plan"]
        before = plan["current_node"]
        if before == current:
            return

        def mutate() -> None:
            plan["current_node"] = current

        self._record_plan_mutation(
            state,
            trace,
            subject=plan["plan_id"],
            field="current_node",
            before=before,
            after=current,
            rationale=reason,
            cause=cause,
            mutate=mutate,
        )

    def _set_node_state(
        self,
        state: dict[str, Any],
        trace: Trace,
        node_id: str,
        target: str,
        rationale: str,
        *,
        cause: dict[str, str],
    ) -> None:
        node = next((row for row in state["plan"]["node"] if row["id"] == node_id), None)
        if node is None:
            self._obligation(state, trace, f"unknown-node:{node_id}")
            return
        before = node["state"]
        if before == target:
            return

        def mutate() -> None:
            node["state"] = target

        self._record_plan_mutation(
            state,
            trace,
            subject=node_id,
            field="state",
            before=before,
            after=target,
            rationale=rationale,
            cause=cause,
            mutate=mutate,
        )
        if target == "candidate" and node_id not in state["candidates"]:
            state["candidates"].append(node_id)

    def _record_plan_mutation(
        self,
        state: dict[str, Any],
        trace: Trace,
        *,
        subject: str,
        field: str,
        before: Any,
        after: Any,
        rationale: str,
        cause: dict[str, str],
        mutate: Callable[[], None],
    ) -> None:
        plan = state["plan"]
        before_revision = plan["revision"]
        before_hash = canonical_hash(plan)
        mutate()
        plan["revision"] += 1
        after_hash = canonical_hash(plan)
        state["plan_history"].append({
            "subject": subject,
            "field": field,
            "before": subset_copy(before),
            "after": subset_copy(after),
            "before_revision": before_revision,
            "after_revision": plan["revision"],
            "before_hash": before_hash,
            "after_hash": after_hash,
            "rationale": rationale,
            "cause": subset_copy(cause),
        })
        self._transition(
            trace,
            subject,
            field,
            before,
            after,
            cause=cause,
            before_revision=before_revision,
            after_revision=plan["revision"],
        )
        trace.add(
            "transition",
            subject=plan["plan_id"],
            field="plan_revision",
            before=before_revision,
            after=plan["revision"],
            reason=rationale,
            cause=subset_copy(cause),
            before_revision=before_revision,
            after_revision=plan["revision"],
        )

    def _actor_can_coordinate(self, state: dict[str, Any]) -> bool:
        grants = set(state["roles"].get(state["actor"], []))
        return bool(grants & {"owner", "integrator", "contributor-central"})

    def _execution_allowed(self, state: dict[str, Any], subject: str) -> bool:
        instruction = state["owner_instruction"]
        return (
            instruction.get("mode") == "execute"
            and instruction.get("authorization") is True
            and instruction.get("selected_campaign") == subject
            and subject not in state["holds"]
            and self._actor_can_coordinate(state)
        )

    def _execution_block_reason(self, state: dict[str, Any], subject: str | None) -> str:
        instruction = state["owner_instruction"]
        if instruction.get("mode") != "execute":
            return f"instruction-{instruction.get('mode')}"
        if instruction.get("authorization") is not True:
            return "missing-owner-authorization"
        if subject in state["holds"]:
            return "execution-hold"
        if not self._actor_can_coordinate(state):
            return "actor-grant-missing"
        return "campaign-selection-mismatch"

    @staticmethod
    def _primary_authority(state: dict[str, Any], actor: str) -> str:
        grants = state["roles"].get(actor, [])
        order = ("owner", "integrator", "contributor-central", "worker")
        return next((role for role in order if role in grants), "none")

    @staticmethod
    def _ordered_nodes(plan: dict[str, Any]) -> list[dict[str, Any]]:
        children: dict[str, list[dict[str, Any]]] = {}
        by_id = {row["id"]: row for row in plan["node"]}
        for row in plan["node"]:
            children.setdefault(row["parent"], []).append(row)
        for rows in children.values():
            rows.sort(key=lambda row: (row["order"], row["id"]))
        result: list[dict[str, Any]] = []

        def walk(key: str) -> None:
            result.append(by_id[key])
            for child in children.get(key, []):
                walk(child["id"])

        walk(plan["root_node"])
        return result

    def _public_final(self, state: dict[str, Any]) -> dict[str, Any]:
        return {
            "plan": copy.deepcopy(state["plan"]),
            "plan_history": copy.deepcopy(state["plan_history"]),
            "owner_instruction": copy.deepcopy(state["owner_instruction"]),
            "holds": copy.deepcopy(state["holds"]),
            "input_identities": copy.deepcopy(state["input_identities"]),
            "candidates": sorted(set(state["candidates"])),
            "evidence": copy.deepcopy(state["evidence"]),
            "jobs": copy.deepcopy(state["jobs"]),
            "operations": copy.deepcopy(state["operations"]),
            "packets": copy.deepcopy(state["packets"]),
            "panel": copy.deepcopy(state["panel"]),
            "unresolved_obligations": copy.deepcopy(state["unresolved_obligations"]),
            "complete_route": [row["id"] for row in self._ordered_nodes(state["plan"])],
        }
