#!/usr/bin/env python3
"""Focused tests for the pure campaign simulator and independent oracle."""

from __future__ import annotations

import copy
import hashlib
import io
import json
from pathlib import Path
import socket
import subprocess
import tempfile
import unittest
from unittest import mock

from campaign_model import CampaignModel
from case_loader import InputError, load_cases, load_json, load_plan, validate_case, validate_correction
from simulate_campaign import (
    CliError,
    main,
    question_provider,
    report,
    run_case,
    simulate,
    validate as validate_suite,
)
from trace_checker import check_trace


HERE = Path(__file__).resolve().parent
VIBESPECS = HERE.parents[1]
EXAMPLES = VIBESPECS / "examples" / "campaign-simulation"
CASES = EXAMPLES / "cases" / "campaign-cases.json"
CORRECTIONS = sorted((EXAMPLES / "corrections").glob("*.json"))


def resequence(trace: dict) -> None:
    for index, entry in enumerate(trace["entries"], 1):
        entry["seq"] = index
        entry["logical_time"] = f"T+{index:04d}"
        entry["entry_id"] = f"{trace['case_id']}:trace:{index:04d}"


class CampaignSimulationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.cases = load_cases(CASES)
        cls.by_id = {case["id"]: case for case in cls.cases}

    def trace(self, case_id: str) -> tuple[dict, dict]:
        case = self.by_id[case_id]
        run = run_case(case)
        self.assertEqual(run["verdict"]["verdict"], "pass", run["verdict"]["failures"])
        return case, run["trace"]

    def panel_recovery_case(self) -> dict:
        case = copy.deepcopy(self.by_id["partial-campaign-panel-resumption"])
        case["events"] = [
            {
                "id": "E-P2-fail", "type": "gate_result", "gate_id": "P2",
                "status": "fail", "selected_cases": 1, "required_cases": 1,
                "subject": "sha-panel-1",
            },
            {
                "id": "E-P2-pass", "type": "gate_result", "gate_id": "P2",
                "status": "pass", "selected_cases": 1, "required_cases": 1,
                "subject": "sha-panel-1",
            },
        ]
        case["limits"]["max_events"] = 2
        return case

    def panel_recovery_with_unrelated_case(self) -> dict:
        case = self.panel_recovery_case()
        case["events"].insert(1, {
            "id": "E-owner-unrelated",
            "type": "owner_message",
            "mode": "execute",
            "selected_campaign": "MAIN",
            "authorization": True,
        })
        case["limits"]["max_events"] = 3
        return case

    def panel_pass_then_unrelated_case(self) -> dict:
        case = copy.deepcopy(self.by_id["partial-campaign-panel-resumption"])
        case["events"] = [
            {
                "id": "E-P2-pass", "type": "gate_result", "gate_id": "P2",
                "status": "pass", "selected_cases": 1, "required_cases": 1,
                "subject": "sha-panel-1",
            },
            {
                "id": "E-owner-unrelated", "type": "owner_message",
                "mode": "execute", "selected_campaign": "MAIN",
                "authorization": True,
            },
        ]
        case["limits"]["max_events"] = 2
        return case

    def panel_pass_then_input_change_case(self) -> dict:
        case = copy.deepcopy(self.by_id["partial-campaign-panel-resumption"])
        case["events"] = [
            {
                "id": "E-P2-pass", "type": "gate_result", "gate_id": "P2",
                "status": "pass", "selected_cases": 1, "required_cases": 1,
                "subject": "sha-panel-1",
            },
            {
                "id": "E-panel-subject-changed", "type": "input_change",
                "input_id": "panel-subject", "before": "sha-panel-1",
                "after": "sha-panel-2", "expected": True,
            },
        ]
        case["limits"]["max_events"] = 2
        return case

    def panel_pass_then_superseding_failure_case(self) -> dict:
        case = copy.deepcopy(self.by_id["partial-campaign-panel-resumption"])
        case["events"] = [
            {
                "id": "E-P2-pass", "type": "gate_result", "gate_id": "P2",
                "status": "pass", "selected_cases": 1, "required_cases": 1,
                "subject": "sha-panel-1",
            },
            {
                "id": "E-P2-later-fail", "type": "gate_result", "gate_id": "P2",
                "status": "fail", "selected_cases": 1, "required_cases": 1,
                "subject": "sha-panel-1",
            },
        ]
        case["limits"]["max_events"] = 2
        return case

    def repeated_priority_case(self) -> dict:
        case = copy.deepcopy(self.by_id["owner-priority-planning-only"])
        selections = ["ALT", "MAIN", "ALT", "MAIN"]
        events = [
            {
                "id": f"E-owner-priority-ambiguous-{index}",
                "type": "owner_message",
                "mode": "planning",
                "selected_campaign": None,
                "authorization": False,
            }
            for index in range(1, 4)
        ]
        dialogue = []
        for index, selected in enumerate(selections):
            after_event = None if index == 0 else events[index - 1]["id"]
            dialogue.append({
                "id": f"Q-priority-{index + 1}",
                "intent": "resolve_owner_priority",
                "subject": ["ALT", "MAIN"],
                "recipient": "owner",
                "required_information": ["selected_campaign", "execution_mode"],
                "after_event": after_event,
                "answer": {
                    "selected_campaign": selected,
                    "execution_mode": "planning",
                    "authority_source": f"owner-message:{index + 1}",
                },
            })
        case["events"] = events
        case["dialogue"] = dialogue
        case["expected"] = {
            "required_actions": [
                {"intent": "select_campaign", "subject": ["ALT"]},
                {"intent": "select_campaign", "subject": ["MAIN"]},
                *[
                    {"intent": "record_owner_instruction", "subject": [event["id"]]}
                    for event in events
                ],
            ],
            "forbidden_actions": [{"intent": "continue_authorized_campaign"}],
            "required_action_order": [
                "select_campaign", "refine_plan",
                "record_owner_instruction", "select_campaign", "refine_plan",
                "record_owner_instruction", "select_campaign", "refine_plan",
                "record_owner_instruction", "select_campaign", "refine_plan",
            ],
            "final": {
                "owner_instruction": {
                    "mode": "planning",
                    "selected_campaign": "MAIN",
                    "authorization": False,
                },
                "plan": {"revision": 11, "current_node": "MAIN"},
            },
            "required_trace_kinds": [
                "authority", "event", "question", "answer", "transition", "action", "summary",
            ],
        }
        case["limits"] = {"max_steps": 220, "max_events": 3}
        return case

    def test_complete_suite_runs_all_nineteen_cases_and_is_deterministic(self) -> None:
        code, first = simulate(CASES, [])
        second_code, second = simulate(CASES, [])
        self.assertEqual(code, 0)
        self.assertEqual(second_code, 0)
        self.assertEqual(first["counts"], {"discovered": 19, "selected": 19, "passed": 19, "failed": 0})
        self.assertTrue(first["complete_selection"])
        self.assertEqual(first, second)

    def test_model_input_excludes_expected_actions_and_semantic_rubric(self) -> None:
        case = self.by_id["explicit-continue-no-redundant-question"]
        provider, _ = question_provider(case)
        first = CampaignModel().run(
            case_id=case["id"], revision=case["revision"], initial=case["initial"],
            events=case["events"], limits=case["limits"], question_handler=provider,
        )
        altered = copy.deepcopy(case)
        altered["expected"] = {"not": "a model input"}
        altered["semantic_rubric"] = {"not": "a model input"}
        provider, _ = question_provider(altered)
        second = CampaignModel().run(
            case_id=altered["id"], revision=altered["revision"], initial=altered["initial"],
            events=altered["events"], limits=altered["limits"], question_handler=provider,
        )
        self.assertEqual(first, second)

    def test_questions_are_exact_and_answers_follow_matching_questions(self) -> None:
        case, trace = self.trace("owner-priority-planning-only")
        entries = trace["entries"]
        question = next(row for row in entries if row["kind"] == "question")
        answer = next(row for row in entries if row["kind"] == "answer")
        self.assertLess(question["seq"], answer["seq"])
        self.assertEqual(question["question_id"], answer["question_id"])
        self.assertEqual(question["intent"], "resolve_owner_priority")
        self.assertEqual(answer["answer"]["selected_campaign"], "ALT")
        self.assertEqual(check_trace(case, trace)["verdict"], "pass")

    def test_missing_extra_and_misbound_question_controls_turn_red(self) -> None:
        case, trace = self.trace("owner-priority-planning-only")
        question_index = next(i for i, row in enumerate(trace["entries"]) if row["kind"] == "question")
        missing = copy.deepcopy(trace)
        missing["entries"].pop(question_index)
        self.assertIn("QUESTION_COUNT", {row["code"] for row in check_trace(case, missing)["failures"]})

        extra = copy.deepcopy(trace)
        extra["entries"].append(copy.deepcopy(next(row for row in trace["entries"] if row["kind"] == "question")))
        self.assertIn("QUESTION_COUNT", {row["code"] for row in check_trace(case, extra)["failures"]})

        wrong = copy.deepcopy(trace)
        next(row for row in wrong["entries"] if row["kind"] == "answer")["answer"]["selected_campaign"] = "MAIN"
        self.assertIn("ANSWER_MISMATCH", {row["code"] for row in check_trace(case, wrong)["failures"]})

    def test_unmatched_question_never_receives_scripted_answer(self) -> None:
        case = copy.deepcopy(self.by_id["owner-priority-planning-only"])
        case["dialogue"][0]["subject"] = ["MAIN"]
        run = run_case(case)
        self.assertEqual(run["verdict"]["verdict"], "fail")
        self.assertFalse(any(row["kind"] == "answer" for row in run["trace"]["entries"]))
        codes = {row["code"] for row in run["verdict"]["failures"]}
        self.assertTrue({"QUESTION_MISMATCH", "ANSWER_MISSING", "ANSWER_UNUSED"} <= codes)

    def test_explicit_continue_and_casual_recovery_have_no_questions(self) -> None:
        for case_id in ("explicit-continue-no-redundant-question", "casual-interruption-no-repeat-approval"):
            with self.subTest(case_id=case_id):
                _, trace = self.trace(case_id)
                self.assertFalse(any(row["kind"] == "question" for row in trace["entries"]))

    def test_worker_pass_and_worker_acceptance_never_advance_candidate(self) -> None:
        for case_id in ("worker-pass-awaits-central-review", "unauthorized-worker-acceptance-rejected"):
            with self.subTest(case_id=case_id):
                _, trace = self.trace(case_id)
                node = next(row for row in trace["final"]["plan"]["node"] if row["id"] == "MAIN-CANDIDATE")
                self.assertEqual(node["state"], "candidate")
        case, trace = self.trace("unauthorized-worker-acceptance-rejected")
        damaged = copy.deepcopy(trace)
        action = next(row for row in damaged["entries"] if row.get("intent") == "reject_unauthorized_acceptance")
        action["intent"] = "accept_worker_result"
        codes = {row["code"] for row in check_trace(case, damaged)["failures"]}
        self.assertTrue({"ACTION_MISSING", "ACTION_FORBIDDEN", "WORKER_ACCEPTANCE"} <= codes)

    def test_candidate_and_deferred_prerequisites_do_not_satisfy_accepted_tier(self) -> None:
        for case_id in ("dependency-candidate-does-not-satisfy-tier",
                        "dependency-deferred-does-not-satisfy-tier"):
            with self.subTest(case_id=case_id):
                _, trace = self.trace(case_id)
                blocks = [row for row in trace["entries"] if row.get("intent") == "block_dependent_execution"]
                self.assertEqual(blocks[0]["details"]["unmet"], ["MAIN-ACCEPTED"])

    def test_missing_panel_tail_is_detected_by_independent_oracle(self) -> None:
        case, trace = self.trace("partial-campaign-panel-resumption")
        damaged = copy.deepcopy(trace)
        damaged["entries"] = [row for row in damaged["entries"]
                              if not (row.get("intent") == "run_panel_step" and row.get("subject") == ["P4"])]
        failures = check_trace(case, damaged)["failures"]
        self.assertIn("ACTION_MISSING", {row["code"] for row in failures})

    def test_zero_and_skipped_test_acceptance_are_detected(self) -> None:
        for case_id in ("zero-selected-tests-rejected", "skipped-required-tests-rejected"):
            with self.subTest(case_id=case_id):
                case, trace = self.trace(case_id)
                damaged = copy.deepcopy(trace)
                action = next(row for row in damaged["entries"] if row.get("intent") == "reject_gate")
                action["intent"] = "record_gate_result"
                codes = {row["code"] for row in check_trace(case, damaged)["failures"]}
                expected = "ZERO_TESTS_ACCEPTED" if case_id.startswith("zero") else "SKIPPED_TESTS_ACCEPTED"
                self.assertIn(expected, codes)

    def test_changed_subject_cannot_remain_valid(self) -> None:
        case, trace = self.trace("changed-evidence-subject-invalidates-reuse")
        self.assertTrue(any(row.get("intent") == "recapture_input_snapshot"
                            for row in trace["entries"]))
        self.assertFalse(any(row["kind"] == "question" for row in trace["entries"]))
        damaged = copy.deepcopy(trace)
        damaged["final"]["evidence"][0]["valid"] = True
        codes = {row["code"] for row in check_trace(case, damaged)["failures"]}
        self.assertIn("STALE_EVIDENCE_REUSED", codes)

    def test_observed_live_central_conflict_holds_writes_without_replacement(self) -> None:
        _, trace = self.trace("observed-live-central-conflict-holds-writes")
        intents = [row.get("intent") for row in trace["entries"] if row["kind"] == "action"]
        self.assertIn("hold_affected_writes", intents)
        self.assertNotIn("continue_authorized_campaign", intents)
        self.assertNotIn("avoid_duplicate_worker", intents)
        self.assertEqual(trace["final"]["holds"], ["MAIN"])

    def test_complete_route_and_extension_fields_are_preserved_without_input_mutation(self) -> None:
        case = self.by_id["casual-interruption-no-repeat-approval"]
        before = copy.deepcopy(case)
        run = run_case(case)
        self.assertEqual(case, before)
        final_plan = run["trace"]["final"]["plan"]
        main = next(row for row in final_plan["node"] if row["id"] == "MAIN")
        candidate = next(row for row in final_plan["node"] if row["id"] == "MAIN-CANDIDATE")
        self.assertEqual(main["route_extension"], {"lane": "primary", "preserve": True})
        self.assertEqual(candidate["candidate_extension"], "must-survive")
        self.assertEqual(run["trace"]["final"]["complete_route"],
                         ["ROOT", "MAIN", "MAIN-ACCEPTED", "MAIN-CANDIDATE",
                          "MAIN-DEPENDENT", "MAIN-DISTANT", "MAIN-FINAL", "ALT", "ALT-A"])

    def test_strict_json_rejects_duplicate_unknown_and_nonfinite_controls(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            duplicate = root / "duplicate.json"
            duplicate.write_text('{"schema":1,"schema":1}', encoding="utf-8")
            with self.assertRaises(InputError) as caught:
                load_json(duplicate)
            self.assertEqual(caught.exception.code, "JSON_DUPLICATE_KEY")

            nonfinite = root / "nonfinite.json"
            nonfinite.write_text('{"value":NaN}', encoding="utf-8")
            with self.assertRaises(InputError) as caught:
                load_json(nonfinite)
            self.assertEqual(caught.exception.code, "JSON_NONFINITE")

            _, raw_bundle = load_json(CASES)
            unknown = copy.deepcopy(raw_bundle["cases"][0])
            unknown["live_mode"] = True
            with self.assertRaises(InputError) as caught:
                validate_case(unknown, CASES)
            self.assertEqual(caught.exception.code, "UNKNOWN_CONTROL_FIELD")

    def test_strict_toml_rejects_duplicate_keys_nonfinite_and_hash_drift(self) -> None:
        template = (
            'schema = 1\nplan_id = "p"\nrevision = 1\nroot_node = "R"\ncurrent_node = "R"\n'
            '[[node]]\nid = "R"\nparent = ""\norder = 0\nkind = "campaign"\n'
            'title = "R"\nstate = "active"\ndepends_on = []\nmandates = []\nacceptance = []\nevidence = []\n'
        )
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "plan.toml"
            for suffix, code in (("id = \"R2\"\n", "TOML_INVALID"),
                                 ("extension = nan\n", "TOML_NONFINITE")):
                with self.subTest(suffix=suffix):
                    path.write_text(template + suffix, encoding="utf-8")
                    sha = hashlib.sha256(path.read_bytes()).hexdigest()
                    with self.assertRaises(InputError) as caught:
                        load_plan(path, sha)
                    self.assertEqual(caught.exception.code, code)
            path.write_text(template, encoding="utf-8")
            with self.assertRaises(InputError) as caught:
                load_plan(path, "0" * 64)
            self.assertEqual(caught.exception.code, "FIXTURE_HASH_MISMATCH")

    def test_correction_examples_validate_and_deletion_needs_replacement_or_reason(self) -> None:
        self.assertEqual(len(CORRECTIONS), 2)
        for path in CORRECTIONS:
            self.assertTrue(validate_correction(path)["immutable"])
        code, validation = validate_suite(CASES, CORRECTIONS)
        self.assertEqual(code, 0)
        self.assertEqual(validation["counts"], {
            "cases": 19, "baseline_cases": 20, "corrections": 2, "recorded_changes": 2,
        })
        deletion = validate_correction(next(path for path in CORRECTIONS if "retire" in path.name))
        deletion["operations"][0]["replacement_coverage"] = []
        deletion["operations"][0]["authorized_reason"] = None
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "bad-correction.json"
            path.write_text(json.dumps(deletion), encoding="utf-8")
            with self.assertRaises(InputError) as caught:
                validate_correction(path)
        self.assertEqual(caught.exception.code, "CORRECTION_INVALID")

    def test_cli_surface_is_only_simulate_validate_report_and_zero_unknown_refuse(self) -> None:
        from simulate_campaign import parser
        choices = next(action for action in parser()._actions if action.dest == "operation").choices
        self.assertEqual(set(choices), {"simulate", "validate", "report"})
        with mock.patch("sys.stdout", new_callable=io.StringIO) as output:
            code = main(["simulate", "--cases", str(CASES), "--case", "not-a-case"])
        self.assertEqual(code, 2)
        self.assertEqual(json.loads(output.getvalue())["error"]["code"], "CASE_UNKNOWN")

    def test_report_summarizes_recorded_run_without_changing_its_verdict(self) -> None:
        code, run = simulate(CASES, ["explicit-continue-no-redundant-question"])
        self.assertEqual(code, 0)
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "run.json"
            path.write_text(json.dumps(run), encoding="utf-8")
            report_code, summary = report(path, CASES)
            run["live_mode"] = True
            path.write_text(json.dumps(run), encoding="utf-8")
            with self.assertRaises(InputError) as caught:
                report(path, CASES)
        self.assertEqual(report_code, 0)
        self.assertEqual(summary["summary"]["selected"], 1)
        self.assertEqual(summary["cases"][0]["verdict"], "pass")
        self.assertIn("bounded real-agent gate", summary["limits"][1])
        self.assertEqual(caught.exception.code, "UNKNOWN_CONTROL_FIELD")

    def test_ordinary_simulation_attempts_no_process_network_home_or_file_write(self) -> None:
        case = self.by_id["unknown-effect-observed-complete"]
        before = copy.deepcopy(case)
        with mock.patch.object(subprocess, "Popen", side_effect=AssertionError("process attempted")), \
             mock.patch.object(socket, "socket", side_effect=AssertionError("network attempted")), \
             mock.patch.object(Path, "home", side_effect=AssertionError("personal state attempted")), \
             mock.patch.object(Path, "write_text", side_effect=AssertionError("write attempted")), \
             mock.patch.object(Path, "write_bytes", side_effect=AssertionError("write attempted")):
            run = run_case(case)
        self.assertEqual(run["verdict"]["verdict"], "pass")
        self.assertEqual(case, before)

    def test_p2_01_worker_owner_instruction_cannot_grant_campaign_execution(self) -> None:
        case, trace = self.trace("worker-packet-missing-integration-owner")
        self.assertFalse(any(row.get("intent") == "continue_authorized_campaign"
                             for row in trace["entries"]))
        damaged = copy.deepcopy(trace)
        action = next(row for row in damaged["entries"]
                      if row.get("intent") == "revise_bounded_packet")
        action.update({
            "intent": "continue_authorized_campaign",
            "subject": ["MAIN"],
            "actor": "worker-1",
            "authority": "owner",
            "details": {"authority_source": "owner_instruction"},
        })
        codes = {row["code"] for row in check_trace(case, damaged)["failures"]}
        self.assertIn("AUTHORITY_UNGRANTED", codes)
        self.assertIn("EXECUTION_ROLE_UNGRANTED", codes)

    def test_p2_02_panel_scheduling_requires_execute_mode_no_hold_and_actor_grant(self) -> None:
        status_case, status_trace = self.trace("false-closure-complete-panel-and-route-required")
        self.assertFalse(any(row.get("intent") == "run_panel_step"
                             for row in status_trace["entries"]))
        self.assertTrue(any(row.get("intent") == "report_panel_step_owed"
                            for row in status_trace["entries"]))
        self.assertEqual(status_trace["final"]["panel"]["steps"][0]["status"], "unrun")
        damaged = copy.deepcopy(status_trace)
        owed = next(row for row in damaged["entries"]
                    if row.get("intent") == "report_panel_step_owed")
        owed["intent"] = "run_panel_step"
        self.assertIn("EXECUTION_NOT_AUTHORIZED", {
            row["code"] for row in check_trace(status_case, damaged)["failures"]
        })

        for label, mutate in (
            ("hold", lambda initial: initial["holds"].append("MAIN")),
            ("missing-grant", lambda initial: initial.update({"actor": "worker-1"})),
        ):
            with self.subTest(label=label):
                case = copy.deepcopy(self.by_id["partial-campaign-panel-resumption"])
                mutate(case["initial"])
                provider, _ = question_provider(case)
                trace = CampaignModel().run(
                    case_id=case["id"], revision=case["revision"], initial=case["initial"],
                    events=case["events"], limits=case["limits"], question_handler=provider,
                )
                self.assertFalse(any(row.get("intent") == "run_panel_step"
                                     for row in trace["entries"]))
                self.assertTrue(any(row.get("intent") == "report_panel_step_owed"
                                    for row in trace["entries"]))
                self.assertEqual([row["status"] for row in trace["final"]["panel"]["steps"]],
                                 ["pass", "failed", "unrun", "unrun"])

    def test_p2_03_duplicate_selection_and_forged_report_totals_are_rejected(self) -> None:
        case_id = "explicit-continue-no-redundant-question"
        with self.assertRaises(CliError) as caught:
            simulate(CASES, [case_id] * 19)
        self.assertEqual(caught.exception.code, "CASE_DUPLICATE")

        code, run = simulate(CASES, [case_id])
        self.assertEqual(code, 0)
        run["runs"][0]["verdict"]["verdict"] = "fail"
        run["runs"][0]["verdict"]["failures"] = [{"code": "FORGED-FAIL"}]
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "run.json"
            path.write_text(json.dumps(run), encoding="utf-8")
            with self.assertRaises(InputError) as caught:
                report(path, CASES)
        self.assertEqual(caught.exception.code, "REPORT_INVALID")

    def test_p2_04_event_schema_rejects_missing_bool_integer_and_bad_reference(self) -> None:
        _, bundle = load_json(CASES)
        by_id = {case["id"]: case for case in bundle["cases"]}

        missing = copy.deepcopy(by_id["zero-selected-tests-rejected"])
        missing["events"][0].pop("required_cases")
        with self.assertRaises(InputError):
            validate_case(missing, CASES)

        bool_integer = copy.deepcopy(by_id["zero-selected-tests-rejected"])
        bool_integer["events"][0]["selected_cases"] = True
        with self.assertRaises(InputError):
            validate_case(bool_integer, CASES)

        bad_subject = copy.deepcopy(by_id["zero-selected-tests-rejected"])
        bad_subject["events"][0]["subject"] = "sha-unreferenced"
        with self.assertRaises(InputError):
            validate_case(bad_subject, CASES)

        bad_reference = copy.deepcopy(by_id["job-observation-new-live-conflict-recomputes"])
        bad_reference["events"][0]["job_id"] = "MISSING-JOB"
        with self.assertRaises(InputError):
            validate_case(bad_reference, CASES)

        bad_type = copy.deepcopy(by_id["zero-selected-tests-rejected"])
        bad_type["events"][0]["type"] = ["gate_result"]
        with self.assertRaises(InputError):
            validate_case(bad_type, CASES)

    def test_p2_05_oracle_rejects_leaf_acceptance_and_distant_node_loss(self) -> None:
        case, trace = self.trace("worker-pass-awaits-central-review")
        accepted = copy.deepcopy(trace)
        node = next(row for row in accepted["final"]["plan"]["node"]
                    if row["id"] == "MAIN-CANDIDATE")
        node["state"] = "accepted"
        codes = {row["code"] for row in check_trace(case, accepted)["failures"]}
        self.assertIn("PLAN_PROJECTION", codes)
        self.assertIn("CANDIDATE_PROJECTION", codes)

        dropped = copy.deepcopy(trace)
        dropped["final"]["plan"]["node"] = [
            row for row in dropped["final"]["plan"]["node"] if row["id"] != "MAIN-DISTANT"
        ]
        codes = {row["code"] for row in check_trace(case, dropped)["failures"]}
        self.assertIn("PLAN_NODE_IDENTITIES", codes)
        self.assertIn("PLAN_PROJECTION", codes)

    def test_p2_06_answers_are_unique_correctly_bound_and_causally_prior(self) -> None:
        case, trace = self.trace("owner-priority-planning-only")
        answer = next(row for row in trace["entries"] if row["kind"] == "answer")

        duplicate = copy.deepcopy(trace)
        answer_index = next(i for i, row in enumerate(duplicate["entries"])
                            if row["kind"] == "answer")
        duplicate["entries"].insert(answer_index + 1, copy.deepcopy(duplicate["entries"][answer_index]))
        resequence(duplicate)
        self.assertIn("ANSWER_COUNT", {
            row["code"] for row in check_trace(case, duplicate)["failures"]
        })

        wrong_recipient = copy.deepcopy(trace)
        next(row for row in wrong_recipient["entries"]
             if row["kind"] == "answer")["recipient"] = "environment"
        self.assertIn("ANSWER_RECIPIENT", {
            row["code"] for row in check_trace(case, wrong_recipient)["failures"]
        })

        premature = copy.deepcopy(trace)
        causal = [row for row in premature["entries"]
                  if row.get("intent") in {"select_campaign", "refine_plan"}]
        premature["entries"] = [row for row in premature["entries"] if row not in causal]
        answer_index = next(i for i, row in enumerate(premature["entries"])
                            if row["kind"] == "answer")
        premature["entries"][answer_index:answer_index] = causal
        resequence(premature)
        self.assertIn("ANSWER_CAUSAL_ORDER", {
            row["code"] for row in check_trace(case, premature)["failures"]
        })

        false_position = copy.deepcopy(trace)
        next(row for row in false_position["entries"]
             if row["kind"] == "question")["after_event"] = "E-never-observed"
        self.assertIn("QUESTION_CAUSAL_POSITION", {
            row["code"] for row in check_trace(case, false_position)["failures"]
        })

    def test_p2_07_corrections_require_history_approval_coverage_and_frozen_failure(self) -> None:
        with self.assertRaises(CliError) as caught:
            validate_suite(CASES, [])
        self.assertEqual(caught.exception.code, "CORRECTION_MISSING")

        changed_path = next(path for path in CORRECTIONS if "changed-evidence" in path.name)
        retired_path = next(path for path in CORRECTIONS if "retire" in path.name)
        _, changed = load_json(changed_path)
        _, retired = load_json(retired_path)

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            rejected_path = root / "rejected.json"
            rejected = copy.deepcopy(changed)
            rejected["reviewer"]["verdict"] = "rejected"
            rejected_path.write_text(json.dumps(rejected), encoding="utf-8")
            with self.assertRaises(InputError):
                validate_correction(rejected_path)

            changed_original = copy.deepcopy(changed)
            changed_original["original_real_result"]["verdict"] = "fail"
            original_path = root / "changed-original.json"
            original_path.write_text(json.dumps(changed_original), encoding="utf-8")
            with self.assertRaises(CliError) as caught:
                validate_suite(CASES, [original_path, retired_path])
            self.assertEqual(caught.exception.code, "CORRECTION_ORIGINAL_MISMATCH")

            correction_dir = root / "corrections"
            retired_dir = root / "retired"
            correction_dir.mkdir()
            retired_dir.mkdir()
            _, tombstone = load_json(EXAMPLES / "retired" / "redundant-permission-question.tombstone.json")
            bad_retired = copy.deepcopy(retired)
            bad_tombstone = copy.deepcopy(tombstone)
            bad_retired["operations"][0]["replacement_coverage"] = ["missing-replacement"]
            bad_tombstone["replacement_coverage"] = ["missing-replacement"]
            tombstone_text = json.dumps(bad_tombstone, sort_keys=True)
            tombstone_path = retired_dir / "bad-tombstone.json"
            tombstone_path.write_text(tombstone_text, encoding="utf-8")
            bad_retired["operations"][0]["tombstone_path"] = "../retired/bad-tombstone.json"
            bad_retired["operations"][0]["tombstone_sha256"] = hashlib.sha256(
                tombstone_text.encode("utf-8")
            ).hexdigest()
            bad_retired_path = correction_dir / "bad-retired.json"
            bad_retired_path.write_text(json.dumps(bad_retired), encoding="utf-8")
            with self.assertRaises(CliError) as caught:
                validate_suite(CASES, [changed_path, bad_retired_path])
            self.assertEqual(caught.exception.code, "CORRECTION_REPLACEMENT_MISSING")

        _, validation = validate_suite(CASES, CORRECTIONS)
        self.assertTrue(any("reviewer identity" in limit for limit in validation["manual_limits"]))

    def test_p2_08_events_recompute_instruction_job_and_gate_decisions(self) -> None:
        _, owner_trace = self.trace("owner-message-status-to-execute-recomputes")
        event_seq = next(row["seq"] for row in owner_trace["entries"]
                         if row.get("event_id") == "E-owner-execute")
        continue_seq = next(row["seq"] for row in owner_trace["entries"]
                            if row.get("intent") == "continue_authorized_campaign")
        self.assertGreater(continue_seq, event_seq)

        _, job_trace = self.trace("job-observation-new-live-conflict-recomputes")
        job_event_seq = next(row["seq"] for row in job_trace["entries"]
                             if row.get("event_id") == "E-job-live")
        hold_seq = next(row["seq"] for row in job_trace["entries"]
                        if row.get("intent") == "hold_affected_writes")
        self.assertGreater(hold_seq, job_event_seq)
        self.assertFalse(any(row.get("intent") == "continue_authorized_campaign"
                             and row["seq"] > job_event_seq for row in job_trace["entries"]))

        gate_case = copy.deepcopy(self.by_id["partial-campaign-panel-resumption"])
        gate_case["events"] = [{
            "id": "E-P2-pass", "type": "gate_result", "gate_id": "P2", "status": "pass",
            "selected_cases": 1, "required_cases": 1, "subject": "sha-panel-1",
        }]
        gate_run = run_case(gate_case)
        self.assertEqual(gate_run["verdict"]["verdict"], "pass", gate_run["verdict"]["failures"])
        gate_event_seq = next(row["seq"] for row in gate_run["trace"]["entries"]
                              if row.get("event_id") == "E-P2-pass")
        reused_seq = next(row["seq"] for row in gate_run["trace"]["entries"]
                          if row.get("intent") == "reuse_panel_step" and row["subject"] == ["P2"])
        self.assertGreater(reused_seq, gate_event_seq)

    def test_p2_09_plan_mutations_revision_history_and_projection_only_stability(self) -> None:
        mutation_case = copy.deepcopy(self.by_id["worker-pass-awaits-central-review"])
        node = next(row for row in mutation_case["initial"]["plan"]["node"]
                    if row["id"] == "MAIN-CANDIDATE")
        node["state"] = "planned"
        mutation_case["initial"]["plan_state"] = {"MAIN-CANDIDATE": "planned"}
        mutation_case["initial"]["candidates"] = []
        run = run_case(mutation_case)
        self.assertEqual(run["verdict"]["verdict"], "pass", run["verdict"]["failures"])
        final = run["trace"]["final"]
        self.assertEqual(final["plan"]["revision"], 8)
        self.assertEqual(len(final["plan_history"]), 1)
        history = final["plan_history"][0]
        self.assertEqual((history["subject"], history["field"], history["before"], history["after"]),
                         ("MAIN-CANDIDATE", "state", "planned", "candidate"))
        self.assertTrue(history["before_hash"])
        self.assertTrue(history["after_hash"])
        self.assertTrue(history["rationale"])

        stable_case = self.by_id["explicit-continue-no-redundant-question"]
        stable_run = run_case(stable_case)
        self.assertEqual(stable_run["trace"]["final"]["plan"], stable_case["initial"]["plan"])
        self.assertEqual(stable_run["trace"]["final"]["plan_history"], [])

    def test_r1_report_rejects_forged_truncated_full_suite_but_accepts_honest_partial(self) -> None:
        code, full = simulate(CASES, [])
        self.assertEqual(code, 0)
        forged = copy.deepcopy(full)
        forged["runs"] = forged["runs"][:1]
        forged["counts"] = {"discovered": 1, "selected": 1, "passed": 1, "failed": 0}
        forged["complete_selection"] = True
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "forged-full.json"
            path.write_text(json.dumps(forged), encoding="utf-8")
            with self.assertRaises(InputError) as caught:
                report(path, CASES)
        self.assertEqual(caught.exception.code, "REPORT_INVALID")

        partial_code, partial = simulate(
            CASES, ["explicit-continue-no-redundant-question"]
        )
        self.assertEqual(partial_code, 0)
        self.assertFalse(partial["complete_selection"])
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "honest-partial.json"
            path.write_text(json.dumps(partial), encoding="utf-8")
            report_code, summary = report(path, CASES)
        self.assertEqual(report_code, 0)
        self.assertEqual(
            summary["summary"],
            {"discovered": 19, "selected": 1, "passed": 1, "failed": 0},
        )

    def test_r2_panel_gate_uses_immutable_step_case_denominator(self) -> None:
        _, raw_bundle = load_json(CASES)
        raw_case = copy.deepcopy(next(
            row for row in raw_bundle["cases"]
            if row["id"] == "partial-campaign-panel-resumption"
        ))
        raw_p2 = next(
            step for step in raw_case["initial"]["panel"]["steps"] if step["id"] == "P2"
        )
        raw_p2["required_cases"].append("case-extra")
        raw_case["events"] = [{
            "id": "E-P2-forged-pass", "type": "gate_result", "gate_id": "P2",
            "status": "pass", "selected_cases": 1, "required_cases": 1,
            "subject": "sha-panel-1",
        }]
        with self.assertRaises(InputError) as caught:
            validate_case(raw_case, CASES)
        self.assertEqual(caught.exception.code, "CASE_INVALID")

        case = copy.deepcopy(self.by_id["partial-campaign-panel-resumption"])
        p2 = next(step for step in case["initial"]["panel"]["steps"] if step["id"] == "P2")
        p2["required_cases"].append("case-extra")
        case["events"] = [{
            "id": "E-P2-forged-pass", "type": "gate_result", "gate_id": "P2",
            "status": "pass", "selected_cases": 1, "required_cases": 1,
            "subject": "sha-panel-1",
        }]
        run = run_case(case)
        self.assertEqual(run["verdict"]["verdict"], "pass", run["verdict"]["failures"])
        event_seq = next(
            entry["seq"] for entry in run["trace"]["entries"]
            if entry.get("event_id") == "E-P2-forged-pass"
        )
        rejection = next(
            entry for entry in run["trace"]["entries"]
            if entry.get("intent") == "reject_gate" and entry.get("subject") == ["P2"]
        )
        self.assertEqual(rejection["details"]["reason"], "panel-denominator-mismatch")
        self.assertFalse(any(
            entry.get("intent") == "reuse_panel_step"
            and entry.get("subject") == ["P2"]
            and entry["seq"] > event_seq
            for entry in run["trace"]["entries"]
        ))
        final_p2 = next(
            step for step in run["trace"]["final"]["panel"]["steps"] if step["id"] == "P2"
        )
        self.assertNotEqual(final_p2["status"], "pass")
        self.assertFalse(final_p2["subject_valid"])

        damaged = copy.deepcopy(run["trace"])
        forged_action = next(
            entry for entry in damaged["entries"]
            if entry.get("intent") == "reject_gate" and entry.get("subject") == ["P2"]
        )
        forged_action["intent"] = "record_gate_result"
        forged_action["details"] = {
            "selected_cases": 1,
            "required_cases": 1,
            "required_case_ids": ["case-2"],
            "subject": "sha-panel-1",
        }
        final_p2 = next(
            step for step in damaged["final"]["panel"]["steps"] if step["id"] == "P2"
        )
        final_p2["status"] = "pass"
        final_p2["subject_valid"] = True
        codes = {row["code"] for row in check_trace(case, damaged)["failures"]}
        self.assertTrue(
            {"PANEL_DENOMINATOR_ACCEPTED", "PANEL_DENOMINATOR_IDENTITY",
             "PANEL_FINAL_UNSUPPORTED_PASS", "PANEL_FINAL_STATE"} <= codes
        )

    def test_p2b_panel_failure_then_pass_recovers_in_distinct_event_intervals(self) -> None:
        case = self.panel_recovery_case()
        run = run_case(case)
        self.assertEqual(run["verdict"]["verdict"], "pass", run["verdict"]["failures"])
        entries = run["trace"]["entries"]
        fail_seq = next(entry["seq"] for entry in entries
                        if entry.get("event_id") == "E-P2-fail")
        pass_seq = next(entry["seq"] for entry in entries
                        if entry.get("event_id") == "E-P2-pass")
        reject_seq = next(entry["seq"] for entry in entries
                          if entry.get("intent") == "reject_gate"
                          and entry.get("subject") == ["P2"]
                          and entry["seq"] > fail_seq)
        reschedule_seq = next(entry["seq"] for entry in entries
                              if entry.get("intent") == "run_panel_step"
                              and entry.get("subject") == ["P2"]
                              and entry["seq"] > fail_seq)
        accept_seq = next(entry["seq"] for entry in entries
                          if entry.get("intent") == "record_gate_result"
                          and entry.get("subject") == ["P2"])
        reuse_seq = next(entry["seq"] for entry in entries
                         if entry.get("intent") == "reuse_panel_step"
                         and entry.get("subject") == ["P2"]
                         and entry["seq"] > pass_seq)
        self.assertTrue(fail_seq < reject_seq < reschedule_seq < pass_seq)
        self.assertTrue(pass_seq < accept_seq < reuse_seq)
        final_p2 = next(
            step for step in run["trace"]["final"]["panel"]["steps"]
            if step["id"] == "P2"
        )
        self.assertEqual(final_p2["status"], "pass")
        self.assertTrue(final_p2["subject_valid"])
        self.assertNotIn(
            "gate-incomplete:P2", run["trace"]["final"]["unresolved_obligations"]
        )

    def test_p2b_panel_failure_interval_rejects_forged_early_acceptance(self) -> None:
        case = self.panel_recovery_case()
        run = run_case(case)
        self.assertEqual(run["verdict"]["verdict"], "pass", run["verdict"]["failures"])
        damaged = copy.deepcopy(run["trace"])
        forged = next(
            entry for entry in damaged["entries"]
            if entry.get("intent") == "reject_gate" and entry.get("subject") == ["P2"]
        )
        forged["intent"] = "record_gate_result"
        forged["details"] = {
            "event_id": "E-P2-fail",
            "selected_cases": 1,
            "required_cases": 1,
            "required_case_ids": ["case-2"],
            "subject": "sha-panel-1",
        }
        verdict = check_trace(case, damaged)
        self.assertEqual(verdict["verdict"], "fail")
        self.assertIn(
            "PANEL_DENOMINATOR_ACCEPTED",
            {failure["code"] for failure in verdict["failures"]},
        )

    def test_p2c_panel_recovery_keeps_result_cause_across_unrelated_event(self) -> None:
        case = self.panel_recovery_with_unrelated_case()
        run = run_case(case)
        self.assertEqual(run["verdict"]["verdict"], "pass", run["verdict"]["failures"])
        actions = [
            entry for entry in run["trace"]["entries"]
            if entry.get("intent") in {"record_gate_result", "reuse_panel_step"}
            and entry.get("subject") == ["P2"]
        ]
        self.assertEqual(
            [action["details"]["event_id"] for action in actions],
            ["E-P2-pass", "E-P2-pass"],
        )
        for action in actions:
            self.assertEqual(action["details"]["subject"], "sha-panel-1")
            self.assertEqual(action["details"]["required_cases"], 1)
            self.assertEqual(action["details"]["required_case_ids"], ["case-2"])

    def test_p2e_panel_pass_transitions_cannot_precede_their_gate_event(self) -> None:
        case = self.panel_recovery_with_unrelated_case()
        run = run_case(case)
        self.assertEqual(run["verdict"]["verdict"], "pass", run["verdict"]["failures"])
        damaged = copy.deepcopy(run["trace"])
        pass_transitions = [
            entry for entry in damaged["entries"]
            if entry.get("kind") == "transition"
            and entry.get("subject") == "P2"
            and entry.get("cause") == {"kind": "event", "id": "E-P2-pass"}
        ]
        self.assertEqual(
            [
                (entry["field"], entry["before"], entry["after"])
                for entry in pass_transitions
            ],
            [("status", "scheduled", "pass"), ("subject_valid", False, True)],
        )
        damaged["entries"] = [
            entry for entry in damaged["entries"] if entry not in pass_transitions
        ]
        pass_event_index = next(
            index for index, entry in enumerate(damaged["entries"])
            if entry.get("event_id") == "E-P2-pass"
        )
        damaged["entries"][pass_event_index:pass_event_index] = pass_transitions
        resequence(damaged)
        self.assertEqual(damaged["final"], run["trace"]["final"])
        verdict = check_trace(case, damaged)
        self.assertEqual(verdict["verdict"], "fail")
        self.assertIn(
            "PANEL_TRANSITION_CAUSAL_ORDER",
            {failure["code"] for failure in verdict["failures"]},
        )

    def test_p2e_panel_scheduling_transition_must_follow_its_action(self) -> None:
        case = self.panel_recovery_case()
        run = run_case(case)
        self.assertEqual(run["verdict"]["verdict"], "pass", run["verdict"]["failures"])
        damaged = copy.deepcopy(run["trace"])
        fail_event_seq = next(
            entry["seq"] for entry in damaged["entries"]
            if entry.get("event_id") == "E-P2-fail"
        )
        scheduling_action = next(
            entry for entry in damaged["entries"]
            if entry.get("intent") == "run_panel_step"
            and entry.get("subject") == ["P2"]
            and entry["seq"] > fail_event_seq
        )
        scheduling_transition = next(
            entry for entry in damaged["entries"]
            if entry.get("kind") == "transition"
            and entry.get("subject") == "P2"
            and entry.get("field") == "status"
            and entry.get("before") == "failed"
            and entry.get("after") == "scheduled"
            and entry["seq"] > scheduling_action["seq"]
        )
        damaged["entries"].remove(scheduling_transition)
        action_index = damaged["entries"].index(scheduling_action)
        damaged["entries"].insert(action_index, scheduling_transition)
        resequence(damaged)
        verdict = check_trace(case, damaged)
        self.assertEqual(verdict["verdict"], "fail")
        self.assertIn(
            "PANEL_TRANSITION_CAUSAL_ORDER",
            {failure["code"] for failure in verdict["failures"]},
        )

    def test_p2c_panel_forgery_after_unrelated_event_has_no_unchecked_interval(self) -> None:
        case = self.panel_recovery_with_unrelated_case()
        run = run_case(case)
        self.assertEqual(run["verdict"]["verdict"], "pass", run["verdict"]["failures"])
        damaged = copy.deepcopy(run["trace"])
        forged = copy.deepcopy(next(
            entry for entry in damaged["entries"]
            if entry.get("intent") == "record_gate_result"
            and entry.get("subject") == ["P2"]
        ))
        forged["details"] = {
            "event_id": "E-P2-fail",
            "selected_cases": 1,
            "required_cases": 1,
            "required_case_ids": ["case-2"],
            "subject": "sha-panel-1",
        }
        pass_index = next(
            index for index, entry in enumerate(damaged["entries"])
            if entry.get("event_id") == "E-P2-pass"
        )
        damaged["entries"].insert(pass_index, forged)
        resequence(damaged)
        verdict = check_trace(case, damaged)
        self.assertEqual(verdict["verdict"], "fail")
        self.assertIn(
            "PANEL_DENOMINATOR_ACCEPTED",
            {failure["code"] for failure in verdict["failures"]},
        )

    def test_p2d_passing_panel_result_reuses_after_unrelated_event(self) -> None:
        case = self.panel_pass_then_unrelated_case()
        run = run_case(case)
        self.assertEqual(run["verdict"]["verdict"], "pass", run["verdict"]["failures"])
        entries = run["trace"]["entries"]
        unrelated_seq = next(
            entry["seq"] for entry in entries
            if entry.get("event_id") == "E-owner-unrelated"
        )
        later_reuse = next(
            entry for entry in entries
            if entry.get("intent") == "reuse_panel_step"
            and entry.get("subject") == ["P2"]
            and entry["seq"] > unrelated_seq
        )
        self.assertEqual(later_reuse["details"], {
            "event_id": "E-P2-pass",
            "selected_cases": 1,
            "required_cases": 1,
            "required_case_ids": ["case-2"],
            "subject": "sha-panel-1",
            "input_ids": ["panel-subject"],
        })
        self.assertEqual(check_trace(case, run["trace"])["verdict"], "pass")

    def test_p2d_changed_bound_input_invalidates_model_and_forged_reuse(self) -> None:
        case = self.panel_pass_then_input_change_case()
        run = run_case(case)
        self.assertEqual(run["verdict"]["verdict"], "pass", run["verdict"]["failures"])
        entries = run["trace"]["entries"]
        change_seq = next(
            entry["seq"] for entry in entries
            if entry.get("event_id") == "E-panel-subject-changed"
        )
        self.assertTrue(any(
            entry.get("intent") == "invalidate_panel_result"
            and entry.get("subject") == ["P2"]
            and entry["seq"] > change_seq
            for entry in entries
        ))
        self.assertFalse(any(
            entry.get("intent") == "reuse_panel_step"
            and entry.get("subject") == ["P2"]
            and entry["seq"] > change_seq
            for entry in entries
        ))
        final_p2 = next(
            step for step in run["trace"]["final"]["panel"]["steps"]
            if step["id"] == "P2"
        )
        self.assertEqual(final_p2["status"], "scheduled")
        self.assertFalse(final_p2["subject_valid"])

        damaged = copy.deepcopy(run["trace"])
        forged = copy.deepcopy(next(
            entry for entry in damaged["entries"]
            if entry.get("intent") == "reuse_panel_step"
            and entry.get("subject") == ["P2"]
            and entry["seq"] < change_seq
        ))
        summary_index = next(
            index for index, entry in enumerate(damaged["entries"])
            if entry.get("kind") == "summary"
        )
        damaged["entries"].insert(summary_index, forged)
        resequence(damaged)
        verdict = check_trace(case, damaged)
        self.assertEqual(verdict["verdict"], "fail")
        self.assertIn(
            "PANEL_RESULT_SUBJECT_CHANGED",
            {failure["code"] for failure in verdict["failures"]},
        )

    def test_p2e_invalidation_transitions_cannot_precede_input_change(self) -> None:
        case = self.panel_pass_then_input_change_case()
        run = run_case(case)
        self.assertEqual(run["verdict"]["verdict"], "pass", run["verdict"]["failures"])
        damaged = copy.deepcopy(run["trace"])
        invalidation_transitions = [
            entry for entry in damaged["entries"]
            if entry.get("kind") == "transition"
            and entry.get("subject") == "P2"
            and entry.get("cause")
            == {"kind": "event", "id": "E-panel-subject-changed"}
        ]
        self.assertEqual(
            [
                (entry["field"], entry["before"], entry["after"])
                for entry in invalidation_transitions
            ],
            [("status", "pass", "invalidated"), ("subject_valid", True, False)],
        )
        damaged["entries"] = [
            entry for entry in damaged["entries"]
            if entry not in invalidation_transitions
        ]
        change_event_index = next(
            index for index, entry in enumerate(damaged["entries"])
            if entry.get("event_id") == "E-panel-subject-changed"
        )
        damaged["entries"][change_event_index:change_event_index] = invalidation_transitions
        resequence(damaged)
        verdict = check_trace(case, damaged)
        self.assertEqual(verdict["verdict"], "fail")
        self.assertIn(
            "PANEL_TRANSITION_CAUSAL_ORDER",
            {failure["code"] for failure in verdict["failures"]},
        )

    def test_p2e_result_transition_cannot_cross_unrelated_event_boundary(self) -> None:
        case = self.panel_pass_then_unrelated_case()
        run = run_case(case)
        self.assertEqual(run["verdict"]["verdict"], "pass", run["verdict"]["failures"])
        damaged = copy.deepcopy(run["trace"])
        pass_transitions = [
            entry for entry in damaged["entries"]
            if entry.get("kind") == "transition"
            and entry.get("subject") == "P2"
            and entry.get("cause") == {"kind": "event", "id": "E-P2-pass"}
        ]
        self.assertEqual(len(pass_transitions), 2)
        damaged["entries"] = [
            entry for entry in damaged["entries"] if entry not in pass_transitions
        ]
        unrelated_event_index = next(
            index for index, entry in enumerate(damaged["entries"])
            if entry.get("event_id") == "E-owner-unrelated"
        )
        damaged["entries"][unrelated_event_index + 1:unrelated_event_index + 1] = (
            pass_transitions
        )
        resequence(damaged)
        verdict = check_trace(case, damaged)
        self.assertEqual(verdict["verdict"], "fail")
        self.assertIn(
            "PANEL_TRANSITION_CAUSAL_ORDER",
            {failure["code"] for failure in verdict["failures"]},
        )

    def test_p2d_superseded_passing_result_cannot_be_reused(self) -> None:
        case = self.panel_pass_then_superseding_failure_case()
        run = run_case(case)
        self.assertEqual(run["verdict"]["verdict"], "pass", run["verdict"]["failures"])
        entries = run["trace"]["entries"]
        failure_seq = next(
            entry["seq"] for entry in entries
            if entry.get("event_id") == "E-P2-later-fail"
        )
        self.assertFalse(any(
            entry.get("intent") == "reuse_panel_step"
            and entry.get("subject") == ["P2"]
            and entry["seq"] > failure_seq
            for entry in entries
        ))

        damaged = copy.deepcopy(run["trace"])
        forged = copy.deepcopy(next(
            entry for entry in damaged["entries"]
            if entry.get("intent") == "reuse_panel_step"
            and entry.get("subject") == ["P2"]
            and entry["seq"] < failure_seq
        ))
        summary_index = next(
            index for index, entry in enumerate(damaged["entries"])
            if entry.get("kind") == "summary"
        )
        damaged["entries"].insert(summary_index, forged)
        resequence(damaged)
        verdict = check_trace(case, damaged)
        self.assertEqual(verdict["verdict"], "fail")
        self.assertIn(
            "PANEL_RESULT_SUPERSEDED",
            {failure["code"] for failure in verdict["failures"]},
        )

    def test_p2d_final_panel_rejects_unsupported_passes_and_denominator_forgery(self) -> None:
        case = self.panel_recovery_with_unrelated_case()
        run = run_case(case)
        self.assertEqual(run["verdict"]["verdict"], "pass", run["verdict"]["failures"])
        honest_steps = {
            step["id"]: step for step in run["trace"]["final"]["panel"]["steps"]
        }
        self.assertEqual(honest_steps["P3"]["status"], "scheduled")
        self.assertEqual(honest_steps["P4"]["status"], "scheduled")
        self.assertEqual(honest_steps["P2"]["required_cases"], ["case-2"])

        damaged = copy.deepcopy(run["trace"])
        forged_steps = {
            step["id"]: step for step in damaged["final"]["panel"]["steps"]
        }
        for gate_id in ("P3", "P4"):
            forged_steps[gate_id]["status"] = "pass"
            forged_steps[gate_id]["subject_valid"] = True
        forged_steps["P2"]["required_cases"] = []
        verdict = check_trace(case, damaged)
        self.assertEqual(verdict["verdict"], "fail")
        codes = {failure["code"] for failure in verdict["failures"]}
        self.assertIn("PANEL_STEP_DENOMINATOR", codes)
        self.assertIn("PANEL_FINAL_UNSUPPORTED_PASS", codes)
        self.assertIn("PANEL_FINAL_STATE", codes)

    def test_r3_answer_dependent_plan_transitions_cannot_precede_question(self) -> None:
        case, trace = self.trace("owner-priority-planning-only")
        damaged = copy.deepcopy(trace)
        transitions = [
            entry for entry in damaged["entries"]
            if entry.get("kind") == "transition"
            and entry.get("field") in {"current_node", "plan_revision"}
        ]
        damaged["entries"] = [
            entry for entry in damaged["entries"] if entry not in transitions
        ]
        question_index = next(
            index for index, entry in enumerate(damaged["entries"])
            if entry["kind"] == "question"
        )
        damaged["entries"][question_index:question_index] = transitions
        resequence(damaged)
        codes = {row["code"] for row in check_trace(case, damaged)["failures"]}
        self.assertIn("ANSWER_TRANSITION_ORDER", codes)

    def test_r4_real_recheck_id_cannot_reuse_frozen_original_run(self) -> None:
        changed_path = next(path for path in CORRECTIONS if "changed-evidence" in path.name)
        _, changed = load_json(changed_path)
        changed["real_recheck_run_ids"] = [changed["original_real_result"]["run_id"]]
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "reused-run-id.json"
            path.write_text(json.dumps(changed), encoding="utf-8")
            with self.assertRaises(InputError) as caught:
                validate_correction(path)
        self.assertEqual(caught.exception.code, "CORRECTION_INVALID")

    def test_r5_live_conflict_ignores_recovery_latch_and_actions_repeat_by_epoch(self) -> None:
        conflict = copy.deepcopy(self.by_id["job-observation-new-live-conflict-recomputes"])
        conflict["initial"]["recovery"] = False
        conflict_run = run_case(conflict)
        self.assertEqual(
            conflict_run["verdict"]["verdict"], "pass", conflict_run["verdict"]["failures"]
        )
        conflict_event_seq = next(
            entry["seq"] for entry in conflict_run["trace"]["entries"]
            if entry.get("event_id") == "E-job-live"
        )
        self.assertTrue(any(
            entry.get("intent") == "hold_affected_writes"
            and entry["seq"] > conflict_event_seq
            for entry in conflict_run["trace"]["entries"]
        ))
        missing_hold = copy.deepcopy(conflict_run["trace"])
        missing_hold["entries"].remove(next(
            entry for entry in missing_hold["entries"]
            if entry.get("intent") == "hold_affected_writes"
        ))
        resequence(missing_hold)
        self.assertIn("ACTION_MISSING", {
            row["code"] for row in check_trace(conflict, missing_hold)["failures"]
        })

        renewed = copy.deepcopy(self.by_id["owner-message-status-to-execute-recomputes"])
        renewed["initial"]["owner_instruction"] = {
            "mode": "execute", "selected_campaign": "MAIN", "authorization": True,
        }
        renewed["events"] = [
            {"id": "E-owner-status", "type": "owner_message", "mode": "status",
             "selected_campaign": "MAIN", "authorization": False},
            {"id": "E-owner-execute", "type": "owner_message", "mode": "execute",
             "selected_campaign": "MAIN", "authorization": True},
        ]
        renewed_run = run_case(renewed)
        self.assertEqual(
            renewed_run["verdict"]["verdict"], "pass", renewed_run["verdict"]["failures"]
        )
        continuations = [
            entry for entry in renewed_run["trace"]["entries"]
            if entry.get("intent") == "continue_authorized_campaign"
        ]
        self.assertEqual(len(continuations), 2)
        final_event_seq = next(
            entry["seq"] for entry in renewed_run["trace"]["entries"]
            if entry.get("event_id") == "E-owner-execute"
        )
        self.assertLess(continuations[0]["seq"], final_event_seq)
        self.assertGreater(continuations[1]["seq"], final_event_seq)

        damaged = copy.deepcopy(renewed_run["trace"])
        damaged["entries"].remove(next(
            entry for entry in damaged["entries"]
            if entry.get("intent") == "continue_authorized_campaign"
            and entry["seq"] > final_event_seq
        ))
        resequence(damaged)
        self.assertIn("ACTION_ORDER", {
            row["code"] for row in check_trace(renewed, damaged)["failures"]
        })

    def test_r6_owner_event_can_reopen_priority_dialogue_with_causal_answer(self) -> None:
        case = copy.deepcopy(self.by_id["owner-priority-planning-only"])
        first = copy.deepcopy(case["dialogue"][0])
        first["answer"]["selected_campaign"] = "ALT"
        later = copy.deepcopy(first)
        later["id"] = "Q-priority-after-owner-event"
        later["after_event"] = "E-owner-priority-ambiguous"
        later["answer"] = {
            "selected_campaign": "MAIN",
            "execution_mode": "planning",
            "authority_source": "owner-message:later",
        }
        case["events"] = [{
            "id": "E-owner-priority-ambiguous",
            "type": "owner_message",
            "mode": "planning",
            "selected_campaign": None,
            "authorization": False,
        }]
        case["dialogue"] = [first, later]
        case["expected"] = {
            "required_actions": [
                {"intent": "select_campaign", "subject": ["ALT"]},
                {"intent": "refine_plan", "subject": ["ALT"]},
                {"intent": "record_owner_instruction",
                 "subject": ["E-owner-priority-ambiguous"]},
                {"intent": "select_campaign", "subject": ["MAIN"]},
                {"intent": "refine_plan", "subject": ["MAIN"]},
            ],
            "forbidden_actions": [{"intent": "continue_authorized_campaign"}],
            "required_action_order": [
                "select_campaign", "refine_plan", "record_owner_instruction",
                "select_campaign", "refine_plan",
            ],
            "final": {
                "owner_instruction": {
                    "mode": "planning", "selected_campaign": "MAIN",
                    "authorization": False,
                },
                "plan": {"revision": 9, "current_node": "MAIN"},
            },
            "required_trace_kinds": [
                "authority", "event", "question", "answer", "transition", "action", "summary"
            ],
        }
        run = run_case(case)
        self.assertEqual(run["verdict"]["verdict"], "pass", run["verdict"]["failures"])
        questions = [
            entry for entry in run["trace"]["entries"] if entry["kind"] == "question"
        ]
        self.assertEqual(len(questions), 2)
        event_seq = next(
            entry["seq"] for entry in run["trace"]["entries"]
            if entry.get("event_id") == "E-owner-priority-ambiguous"
        )
        self.assertEqual(questions[1]["after_event"], "E-owner-priority-ambiguous")
        self.assertGreater(questions[1]["seq"], event_seq)
        later_answer = next(
            entry for entry in run["trace"]["entries"]
            if entry["kind"] == "answer" and entry["question_id"] == questions[1]["question_id"]
        )
        self.assertGreater(later_answer["seq"], questions[1]["seq"])

        premature_transition = copy.deepcopy(run["trace"])
        later_transitions = [
            entry for entry in premature_transition["entries"]
            if entry.get("kind") == "transition"
            and (
                (entry.get("field") == "current_node"
                 and entry.get("before") == "ALT" and entry.get("after") == "MAIN")
                or
                (entry.get("field") == "plan_revision"
                 and entry.get("before") == 8 and entry.get("after") == 9)
            )
        ]
        self.assertEqual(len(later_transitions), 2)
        premature_transition["entries"] = [
            entry for entry in premature_transition["entries"]
            if entry not in later_transitions
        ]
        event_index = next(
            index for index, entry in enumerate(premature_transition["entries"])
            if entry.get("event_id") == "E-owner-priority-ambiguous"
        )
        premature_transition["entries"][event_index:event_index] = later_transitions
        resequence(premature_transition)
        self.assertEqual(premature_transition["final"], run["trace"]["final"])
        transition_verdict = check_trace(case, premature_transition)
        self.assertEqual(transition_verdict["verdict"], "fail")
        self.assertIn(
            "ANSWER_TRANSITION_ORDER",
            {failure["code"] for failure in transition_verdict["failures"]},
        )

        damaged = copy.deepcopy(run["trace"])
        damaged_questions = [
            entry for entry in damaged["entries"] if entry["kind"] == "question"
        ]
        damaged_questions[1]["after_event"] = None
        codes = {row["code"] for row in check_trace(case, damaged)["failures"]}
        self.assertTrue({"QUESTION_MISMATCH", "QUESTION_CAUSAL_POSITION"} <= codes)

    def test_p2c_repeated_switches_have_distinct_question_revision_causes(self) -> None:
        case = self.repeated_priority_case()
        run = run_case(case)
        self.assertEqual(run["verdict"]["verdict"], "pass", run["verdict"]["failures"])
        transitions = [
            entry for entry in run["trace"]["entries"]
            if entry.get("kind") == "transition"
            and entry.get("field") == "current_node"
            and entry.get("before") == "ALT"
            and entry.get("after") == "MAIN"
        ]
        self.assertEqual(len(transitions), 2)
        self.assertEqual(
            [(entry["before_revision"], entry["after_revision"]) for entry in transitions],
            [(8, 9), (10, 11)],
        )
        self.assertEqual(len({entry["cause"]["id"] for entry in transitions}), 2)
        history = run["trace"]["final"]["plan_history"]
        self.assertEqual(len(history), 4)
        self.assertEqual(len({entry["cause"]["id"] for entry in history}), 4)

    def test_p2c_final_repeated_switch_cannot_precede_its_own_answer(self) -> None:
        case = self.repeated_priority_case()
        run = run_case(case)
        self.assertEqual(run["verdict"]["verdict"], "pass", run["verdict"]["failures"])
        damaged = copy.deepcopy(run["trace"])
        final_transitions = [
            entry for entry in damaged["entries"]
            if entry.get("kind") == "transition"
            and entry.get("before_revision") == 10
            and entry.get("after_revision") == 11
        ]
        self.assertEqual(len(final_transitions), 2)
        damaged["entries"] = [
            entry for entry in damaged["entries"] if entry not in final_transitions
        ]
        final_question = next(
            entry for entry in damaged["entries"]
            if entry.get("kind") == "question"
            and entry.get("after_event") == "E-owner-priority-ambiguous-3"
        )
        answer_index = next(
            index for index, entry in enumerate(damaged["entries"])
            if entry.get("kind") == "answer"
            and entry.get("question_id") == final_question["question_id"]
        )
        damaged["entries"][answer_index:answer_index] = final_transitions
        resequence(damaged)
        self.assertEqual(damaged["final"], run["trace"]["final"])
        verdict = check_trace(case, damaged)
        self.assertEqual(verdict["verdict"], "fail")
        self.assertIn(
            "ANSWER_TRANSITION_ORDER",
            {failure["code"] for failure in verdict["failures"]},
        )


if __name__ == "__main__":
    unittest.main()
