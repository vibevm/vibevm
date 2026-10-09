#!/usr/bin/env python3
"""Local synthetic campaign simulator: simulate, validate and report only."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
from pathlib import Path
import sys
from typing import Any

from campaign_model import CampaignModel, SimulationError
from case_loader import (
    InputError,
    exact_fields,
    load_cases,
    load_history,
    load_json,
    require,
    source_case_hash,
    validate_correction,
)
from trace_checker import check_trace


SCHEMA = 1
TRACE_ENTRY_FIELDS = {
    "authority": {"seq", "logical_time", "entry_id", "kind", "actor", "grants"},
    "hold": {"seq", "logical_time", "entry_id", "kind", "subject", "active", "reason"},
    "evidence": {"seq", "logical_time", "entry_id", "kind", "evidence", "disposition"},
    "obligation": {"seq", "logical_time", "entry_id", "kind", "obligation", "status"},
    "action": {"seq", "logical_time", "entry_id", "kind", "intent", "subject", "actor", "authority", "details"},
    "question": {"seq", "logical_time", "entry_id", "kind", "question_id", "intent", "subject", "recipient", "required_information", "after_event"},
    "answer": {"seq", "logical_time", "entry_id", "kind", "question_id", "recipient", "answer"},
    "event": {"seq", "logical_time", "entry_id", "kind", "event_id", "event_type", "data"},
    "transition": {
        "seq", "logical_time", "entry_id", "kind", "subject", "field", "before", "after",
        "reason", "cause", "before_revision", "after_revision",
    },
    "summary": {"seq", "logical_time", "entry_id", "kind", "route", "unresolved_obligations", "accepted", "candidates"},
}


class CliError(ValueError):
    def __init__(self, code: str, message: str):
        super().__init__(message)
        self.code = code


def question_provider(case: dict[str, Any]) -> tuple[Any, list[bool]]:
    used = [False] * len(case["dialogue"])

    def provide(question: dict[str, Any]) -> dict[str, Any] | None:
        for index, expected in enumerate(case["dialogue"]):
            oracle = {key: expected[key] for key in (
                "intent", "subject", "recipient", "required_information", "after_event"
            )}
            if not used[index] and oracle == question:
                used[index] = True
                return copy.deepcopy(expected["answer"])
        return None

    return provide, used


def run_case(case: dict[str, Any], model: CampaignModel | None = None) -> dict[str, Any]:
    """Run a model and then apply the separately authored trace oracle."""
    model = model or CampaignModel()
    provide, used = question_provider(case)
    # Only actual inputs and bounds enter the model. Expected actions, semantic
    # rubric and question oracle remain outside it; the provider releases an
    # answer only after the model emits a matching question.
    initial = copy.deepcopy(case["initial"])
    trace = model.run(
        case_id=case["id"],
        revision=case["revision"],
        initial=initial,
        events=copy.deepcopy(case["events"]),
        limits=copy.deepcopy(case["limits"]),
        question_handler=provide,
    )
    verdict = check_trace(case, trace)
    unused = [case["dialogue"][index]["id"] for index, was_used in enumerate(used) if not was_used]
    if unused:
        verdict["verdict"] = "fail"
        verdict["failures"].append({
            "code": "ANSWER_UNUSED",
            "message": "scripted answers were never requested by matching questions",
            "dialogue_ids": unused,
        })
    return {"case": {"id": case["id"], "revision": case["revision"]},
            "trace": trace, "verdict": verdict}


def suite_manifest(cases_path: Path, cases: list[dict[str, Any]]) -> dict[str, Any]:
    """Bind a run to the strict case inventory supplied to simulation/reporting."""
    raw, _ = load_json(cases_path)
    return {
        "name": cases_path.name,
        "sha256": hashlib.sha256(raw).hexdigest(),
        "case_identities": [
            {
                "id": case["id"],
                "revision": case["revision"],
                "sha256": source_case_hash(case),
            }
            for case in cases
        ],
    }


def simulate(cases_path: Path, selected: list[str]) -> tuple[int, dict[str, Any]]:
    cases = load_cases(cases_path)
    available = {case["id"]: case for case in cases}
    if len(selected) != len(set(selected)):
        duplicates = sorted({case_id for case_id in selected if selected.count(case_id) > 1})
        raise CliError("CASE_DUPLICATE", f"duplicate requested cases: {duplicates}")
    unknown = sorted(set(selected) - set(available))
    if unknown:
        raise CliError("CASE_UNKNOWN", f"unknown requested cases: {unknown}")
    chosen = cases if not selected else [available[key] for key in selected]
    if not chosen:
        raise CliError("ZERO_CASES", "zero cases selected is not a passing suite")
    runs = [run_case(case) for case in chosen]
    passed = sum(run["verdict"]["verdict"] == "pass" for run in runs)
    selected_ids = {case["id"] for case in chosen}
    discovered_ids = set(available)
    report = {
        "schema": SCHEMA,
        "operation": "simulate",
        "suite": suite_manifest(cases_path, cases),
        "counts": {"discovered": len(cases), "selected": len(chosen),
                   "passed": passed, "failed": len(chosen) - passed},
        "complete_selection": selected_ids == discovered_ids,
        "runs": runs,
    }
    return (0 if passed == len(chosen) else 1), report


def validate(cases_path: Path, corrections: list[Path]) -> tuple[int, dict[str, Any]]:
    cases = load_cases(cases_path)
    history_path = cases_path.with_name("campaign-case-history.json")
    history = load_history(history_path, cases_path.name)
    records = [validate_correction(path) for path in corrections]
    correction_ids = [record["id"] for record in records]
    if len(correction_ids) != len(set(correction_ids)):
        raise CliError("CORRECTION_DUPLICATE", "correction ids must be unique")
    current = {
        case["id"]: {"revision": case["revision"], "sha256": source_case_hash(case)}
        for case in cases
    }
    baseline = {row["id"]: row for row in history["baseline_cases"]}
    operations: dict[str, tuple[dict[str, Any], dict[str, Any]]] = {}
    for record in records:
        for operation in record["operations"]:
            case_id = operation["case_id"]
            if case_id in operations:
                raise CliError("CORRECTION_OVERLAP",
                               f"multiple corrections operate on case: {case_id}")
            operations[case_id] = (record, operation)

    changed: dict[str, str] = {}
    for case_id in sorted(set(baseline) | set(current)):
        before = baseline.get(case_id)
        after = current.get(case_id)
        if before is None:
            changed[case_id] = "add"
        elif after is None:
            changed[case_id] = "delete"
        elif (before["revision"], before["sha256"]) != (after["revision"], after["sha256"]):
            changed[case_id] = "modify"

    missing_records = sorted(set(changed) - set(operations))
    extra_records = sorted(set(operations) - set(changed))
    if missing_records:
        raise CliError("CORRECTION_MISSING",
                       f"bundle changes lack correction records: {missing_records}")
    if extra_records:
        raise CliError("CORRECTION_NO_CHANGE",
                       f"correction operations do not match a bundle change: {extra_records}")

    for case_id, expected_kind in changed.items():
        record, operation = operations[case_id]
        before = baseline.get(case_id)
        after = current.get(case_id)
        if operation["kind"] != expected_kind:
            raise CliError("CORRECTION_KIND_MISMATCH",
                           f"correction kind differs for {case_id}: expected {expected_kind}")
        if before is not None:
            if (operation["before_revision"], operation["before_sha256"]) != (
                    before["revision"], before["sha256"]):
                raise CliError("CORRECTION_BEFORE_MISMATCH",
                               f"correction baseline identity differs: {case_id}")
            if before["original_real_result"] is None:
                raise CliError("CORRECTION_ORIGINAL_MISSING",
                               f"history lacks the frozen original result: {case_id}")
            if record["original_real_result"] != before["original_real_result"]:
                raise CliError("CORRECTION_ORIGINAL_MISMATCH",
                               f"correction changed the frozen original result: {case_id}")
        if after is not None and (
                operation["after_revision"], operation["after_sha256"]
        ) != (after["revision"], after["sha256"]):
            raise CliError("CORRECTION_AFTER_MISMATCH",
                           f"correction after identity differs: {case_id}")
        if expected_kind == "delete":
            replacements = operation["replacement_coverage"]
            missing_replacements = sorted(set(replacements) - set(current))
            if missing_replacements:
                raise CliError("CORRECTION_REPLACEMENT_MISSING",
                               f"replacement coverage is absent: {missing_replacements}")
    return 0, {
        "schema": SCHEMA,
        "operation": "validate",
        "valid": True,
        "counts": {
            "cases": len(cases),
            "baseline_cases": len(baseline),
            "corrections": len(records),
            "recorded_changes": len(changed),
        },
        "case_identities": [{"id": row["id"], "revision": row["revision"]} for row in cases],
        "correction_ids": [row["id"] for row in records],
        "history_id": history["id"],
        "manual_limits": history["manual_limits"],
    }


def report(run_path: Path, cases_path: Path) -> tuple[int, dict[str, Any]]:
    _, value = load_json(run_path)
    if not isinstance(value, dict) or value.get("operation") != "simulate":
        raise CliError("REPORT_INVALID", "report input must be a simulator run report")
    exact_fields(value, {"schema", "operation", "suite", "counts", "complete_selection", "runs"},
                 "run report")
    require(type(value.get("schema")) is int and value["schema"] == SCHEMA
            and type(value.get("complete_selection")) is bool
            and isinstance(value.get("suite"), dict),
            "REPORT_INVALID", "run report identity fields are invalid")
    trusted_cases = load_cases(cases_path)
    trusted_suite = suite_manifest(cases_path, trusted_cases)
    require(value["suite"] == trusted_suite,
            "REPORT_INVALID", "run report suite identity differs from the trusted case inventory")
    trusted_identities = {
        (case["id"], case["revision"])
        for case in trusted_cases
    }
    counts = value.get("counts")
    if not isinstance(counts, dict) or not all(type(counts.get(key)) is int
                                               for key in ("discovered", "selected", "passed", "failed")):
        raise CliError("REPORT_INVALID", "run report counts are missing or invalid")
    exact_fields(counts, {"discovered", "selected", "passed", "failed"}, "run report counts")
    require(counts["discovered"] > 0 and counts["selected"] > 0
            and counts["selected"] <= counts["discovered"],
            "REPORT_INVALID", "run report count denominator is inconsistent")
    cases = []
    runs = value.get("runs")
    require(isinstance(runs, list) and len(runs) == counts["selected"],
            "REPORT_INVALID", "run rows do not match selected count")
    identities: set[tuple[str, int]] = set()
    case_ids: set[str] = set()
    derived_passed = 0
    derived_failed = 0
    for run in runs:
        require(isinstance(run, dict), "REPORT_INVALID", "run row must be an object")
        exact_fields(run, {"case", "trace", "verdict"}, "run row")
        case_identity = run.get("case")
        require(isinstance(case_identity, dict), "REPORT_INVALID", "case identity is missing")
        exact_fields(case_identity, {"id", "revision"}, "case identity")
        require(isinstance(case_identity.get("id"), str) and bool(case_identity["id"])
                and type(case_identity.get("revision")) is int and case_identity["revision"] > 0,
                "REPORT_INVALID", "case identity is invalid")
        identity = (case_identity["id"], case_identity["revision"])
        require(identity not in identities and case_identity["id"] not in case_ids,
                "REPORT_INVALID", "run report contains duplicate case identities")
        require(identity in trusted_identities,
                "REPORT_INVALID", "run report case is absent from the trusted case inventory")
        identities.add(identity)
        case_ids.add(case_identity["id"])
        trace = run.get("trace")
        require(isinstance(trace, dict), "REPORT_INVALID", "trace must be an object")
        exact_fields(trace, {"schema", "case_id", "case_revision", "trace_id",
                             "fixed_time_origin", "entries", "final"}, "trace")
        require(type(trace.get("schema")) is int and trace["schema"] == SCHEMA
                and trace.get("case_id") == case_identity.get("id")
                and trace.get("case_revision") == case_identity.get("revision")
                and isinstance(trace.get("trace_id"), str) and bool(trace["trace_id"])
                and trace.get("fixed_time_origin") == "T+0000"
                and isinstance(trace.get("entries"), list),
                "REPORT_INVALID", "trace identity or entries are invalid")
        require(isinstance(trace.get("final"), dict), "REPORT_INVALID", "trace final state is invalid")
        exact_fields(trace["final"], {
            "plan", "plan_history", "owner_instruction", "holds", "input_identities",
            "candidates", "evidence", "jobs", "operations", "packets", "panel",
            "unresolved_obligations", "complete_route",
        }, "trace final state")
        for entry in trace["entries"]:
            require(isinstance(entry, dict) and entry.get("kind") in TRACE_ENTRY_FIELDS,
                    "REPORT_INVALID", "unknown trace entry kind")
            allowed = TRACE_ENTRY_FIELDS[entry["kind"]]
            # Transition reasons occur only for plan transitions.
            if entry["kind"] in {"transition", "hold"} and "reason" not in entry:
                allowed = allowed - {"reason"}
            exact_fields(entry, allowed, f"trace {entry['kind']} entry")
        verdict = run.get("verdict")
        require(isinstance(verdict, dict), "REPORT_INVALID", "verdict must be an object")
        exact_fields(verdict, {"schema", "case_id", "case_revision", "verdict", "failures", "counts"},
                     "verdict")
        require(verdict.get("verdict") in {"pass", "fail"}
                and verdict.get("case_id") == case_identity["id"]
                and verdict.get("case_revision") == case_identity["revision"]
                and isinstance(verdict.get("failures"), list)
                and isinstance(verdict.get("counts"), dict),
                "REPORT_INVALID", "verdict controls are invalid")
        exact_fields(verdict["counts"], {"entries", "questions", "answers", "actions", "events"},
                     "verdict counts")
        derived_verdict_counts = {
            "entries": len(trace["entries"]),
            "questions": sum(entry.get("kind") == "question" for entry in trace["entries"]),
            "answers": sum(entry.get("kind") == "answer" for entry in trace["entries"]),
            "actions": sum(entry.get("kind") == "action" for entry in trace["entries"]),
            "events": sum(entry.get("kind") == "event" for entry in trace["entries"]),
        }
        require(verdict["counts"] == derived_verdict_counts,
                "REPORT_INVALID", "verdict counts differ from trace entries")
        require((verdict["verdict"] == "pass") == (not verdict["failures"]),
                "REPORT_INVALID", "verdict and failure list are inconsistent")
        if verdict["verdict"] == "pass":
            derived_passed += 1
        else:
            derived_failed += 1
        cases.append({
            "id": case_identity.get("id"),
            "revision": case_identity.get("revision"),
            "verdict": verdict.get("verdict"),
            "failure_codes": [row.get("code") for row in verdict.get("failures", [])],
        })
    derived_counts = {
        "discovered": len(trusted_identities),
        "selected": len(identities),
        "passed": derived_passed,
        "failed": derived_failed,
    }
    require(counts == derived_counts,
            "REPORT_INVALID", "caller-supplied aggregate counts differ from unique run verdicts")
    require(value["complete_selection"] == (identities == trusted_identities),
            "REPORT_INVALID", "complete_selection differs from the trusted case inventory")
    return (0 if derived_failed == 0 else 1), {
        "schema": SCHEMA,
        "operation": "report",
        "summary": derived_counts,
        "cases": cases,
        "limits": [
            "Simulator agreement is not proof of arbitrary prose meaning or architectural independence.",
            "A separately invoked bounded real-agent gate and semantic review remain required.",
        ],
    }


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    commands = result.add_subparsers(dest="operation", required=True)
    simulate_parser = commands.add_parser("simulate", help="run local synthetic cases")
    simulate_parser.add_argument("--cases", type=Path, required=True)
    simulate_parser.add_argument("--case", action="append", default=[])
    validate_parser = commands.add_parser("validate", help="validate cases and correction records")
    validate_parser.add_argument("--cases", type=Path, required=True)
    validate_parser.add_argument("--correction", type=Path, action="append", default=[])
    report_parser = commands.add_parser("report", help="summarize a recorded simulator JSON report")
    report_parser.add_argument("--run", type=Path, required=True)
    report_parser.add_argument("--cases", type=Path, required=True)
    return result


def main(argv: list[str] | None = None) -> int:
    try:
        args = parser().parse_args(argv)
        if args.operation == "simulate":
            exit_code, result = simulate(args.cases, args.case)
        elif args.operation == "validate":
            exit_code, result = validate(args.cases, args.correction)
        else:
            exit_code, result = report(args.run, args.cases)
    except (InputError, SimulationError, CliError, OSError, ValueError) as exc:
        result = {
            "schema": SCHEMA,
            "ok": False,
            "error": {"code": getattr(exc, "code", "INPUT_INVALID"), "message": str(exc)},
        }
        exit_code = 2
    print(json.dumps(result, ensure_ascii=True, sort_keys=True))
    return exit_code


if __name__ == "__main__":
    sys.exit(main())
