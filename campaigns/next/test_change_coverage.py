"""Bounded temporary bridge cases; no permanent product gates or plan reads."""
import contextlib
import copy
import io
import json
from pathlib import Path
import runpy
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch

HERE = Path(__file__).parent
HELPER = runpy.run_path(str(HERE / "change_coverage.py"))
CAMPAIGN = runpy.run_path(str(HERE / "campaign.py"))


def fixture():
    return {"schema": 1, "scope": {"before_capture_id": " before:/α ",
                                  "after_capture_id": "after opaque"},
            "tasks": [{"task_id": "task.1", "change_ids": ["change-b", "change-a"]},
                      {"task_id": "task.2", "change_ids": ["change-a"]},
                      {"task_id": "task.3", "change_ids": []}],
            "unplanned": [{"contribution_id": "repair", "reason": "Out-of-plan repair",
                           "change_ids": ["change-c"]}]}


class BridgeReferences(unittest.TestCase):
    def validate(self, bridge=None, records=None, before=" before:/α ", after="after opaque"):
        return HELPER["validate_change_coverage"](
            fixture() if bridge is None else bridge, {"task.1": {}, "task.2": {}, "task.3": {}},
            before, after, ["change-c", "change-b", "change-a"] if records is None else records)

    def test_many_to_many_unplanned_and_explicitly_unbound_rows(self):
        result = self.validate()
        self.assertEqual(result["record_ids"], ["change-a", "change-b", "change-c"])
        self.assertEqual(result["task_ids"], ["task.1", "task.2", "task.3"])
        self.assertEqual(result["unplanned_contribution_ids"], ["repair"])
        self.assertEqual((result["task_rows"], result["bound_task_rows"],
                          result["unplanned_rows"], result["selected_record_count"]), (3, 2, 1, 3))
        self.assertEqual(result["scope"]["before_capture_id"], " before:/α ")
        for key in ("product_gates_executed", "preview_change_records_verified",
                    "central_acceptance", "supplied_scope_and_record_ids_verified"):
            self.assertIs(result[key], False)

    def test_input_order_does_not_change_sorted_result(self):
        changed = fixture()
        changed["tasks"].reverse()
        changed["tasks"][-1]["change_ids"].reverse()
        self.assertEqual(self.validate(), self.validate(changed, ["change-a", "change-c", "change-b"]))

    def test_empty_planned_rows_cannot_cover_selected_records(self):
        bridge = fixture()
        bridge["tasks"] = [{"task_id": "task.1", "change_ids": []}]
        bridge["unplanned"] = []
        with self.assertRaisesRegex(ValueError, "omitted"):
            self.validate(bridge)

    def test_unknown_task_record_and_omitted_record_refuse(self):
        for case in range(4):
            bridge = fixture()
            if case == 0:
                bridge["tasks"][0]["task_id"] = "unknown.1"
            elif case == 1:
                bridge["tasks"][0]["change_ids"].append("unknown")
            elif case == 2:
                bridge["unplanned"][0]["change_ids"].append("unknown")
            else:
                bridge["unplanned"] = []
            with self.subTest(case=case), self.assertRaises(ValueError):
                self.validate(bridge)

    def test_duplicate_rows_and_members_refuse_but_cross_task_sharing_is_legal(self):
        for case in range(4):
            bridge = fixture()
            if case == 0:
                bridge["tasks"].append(copy.deepcopy(bridge["tasks"][0]))
            elif case == 1:
                bridge["unplanned"].append(copy.deepcopy(bridge["unplanned"][0]))
            elif case == 2:
                bridge["tasks"][0]["change_ids"].append("change-a")
            else:
                bridge["unplanned"][0]["change_ids"].append("change-c")
            with self.subTest(case=case), self.assertRaisesRegex(ValueError, "duplicate"):
                self.validate(bridge)

    def test_expected_scope_and_unique_nonempty_expected_set_are_required(self):
        for kwargs in ({"before": "before:/α"}, {"after": "wrong"}, {"records": []},
                       {"records": ["change-a", "change-a"]}, {"before": " "},
                       {"records": [False]}):
            with self.subTest(kwargs=kwargs), self.assertRaises(ValueError):
                self.validate(**kwargs)

    def test_strict_fields_types_and_smuggled_authority_refuse(self):
        for case in range(12):
            bridge = fixture()
            if case == 0:
                bridge["schema"] = True
            elif case == 1:
                bridge["schema"] = 2
            elif case == 2:
                bridge["tasks"] = {}
            elif case == 3:
                bridge["tasks"][0]["change_ids"] = "change-a"
            elif case == 4:
                bridge["scope"]["after_capture_id"] = None
            elif case == 5:
                del bridge["scope"]["before_capture_id"]
            elif case == 6:
                bridge["unplanned"][0]["reason"] = " "
            elif case == 7:
                bridge["unplanned"][0]["change_ids"] = []
            elif case == 8:
                bridge["unplanned"][0]["contribution_id"] = " "
            elif case == 9:
                bridge["acceptance"] = "accepted"
            elif case == 10:
                bridge["tasks"][0]["no_impact"] = True
            else:
                bridge["unplanned"][0]["completed"] = True
            with self.subTest(case=case), self.assertRaises(ValueError):
                self.validate(bridge)

    def test_duplicate_json_keys_at_root_and_nested_rows_refuse(self):
        for raw in ('{"schema":1,"schema":1}',
                    '{"scope":{"before_capture_id":"a","before_capture_id":"b"}}',
                    '{"tasks":[{"task_id":"a","task_id":"b"}]}'):
            with tempfile.TemporaryDirectory() as owned:
                path = Path(owned) / "bridge.json"
                path.write_text(raw, encoding="utf-8")
                with self.subTest(raw=raw), self.assertRaisesRegex(ValueError, "duplicate"):
                    HELPER["read_bridge"](path)


class BridgeCommand(unittest.TestCase):
    def invoke(self, arguments, tasks):
        main = CAMPAIGN["main"]
        loader = Mock(return_value=tasks)
        forbidden_plan = Mock(side_effect=AssertionError("bridge read a plan"))
        output = io.StringIO()
        with patch.dict(main.__globals__, {"load_tasks": loader, "load_plan": forbidden_plan}), \
                patch.object(sys, "argv", ["campaign.py", *arguments]), \
                contextlib.redirect_stdout(output):
            code = main()
        forbidden_plan.assert_not_called()
        return code, json.loads(output.getvalue()), loader

    def arguments(self, path):
        return ["bridge-check", "--bridge", str(path), "--before-capture-id", " before:/α ",
                "--after-capture-id", "after opaque", "--record-id", "change-c",
                "--record-id", "change-a", "--record-id", "change-b"]

    def test_cli_loads_actual_hashed_task_metadata_once_without_plan_or_product_calls(self):
        loader = CAMPAIGN["load_tasks"]
        manifest = CAMPAIGN["read_json"](HERE / "manifest.json")
        with tempfile.TemporaryDirectory() as owned:
            path = Path(owned) / "bridge.json"
            bridge = fixture()
            bridge["tasks"] = [{"task_id": "M-01-A.1", "change_ids": ["change-b", "change-a"]}]
            path.write_text(json.dumps(bridge), encoding="utf-8")
            main = CAMPAIGN["main"]
            forbidden = Mock(side_effect=AssertionError("structural command invoked plan/product"))
            observed = Mock(wraps=loader)
            output = io.StringIO()
            with patch.dict(main.__globals__, {"load_tasks": observed, "load_plan": forbidden}), \
                    patch.object(main.__globals__["subprocess"], "run", forbidden), \
                    patch.object(sys, "argv", ["campaign.py", *self.arguments(path)]), \
                    contextlib.redirect_stdout(output):
                code = main()
            self.assertEqual(code, 0)
            observed.assert_called_once_with(manifest)
            forbidden.assert_not_called()
            result = json.loads(output.getvalue())
            self.assertEqual(result["mode"], "temporary-bridge-structure")
            self.assertEqual(result["task_ids"], ["M-01-A.1"])
            self.assertEqual(result["record_ids"], ["change-a", "change-b", "change-c"])
            for key in ("product_gates_executed", "preview_change_records_verified",
                        "central_acceptance", "supplied_scope_and_record_ids_verified"):
                self.assertIs(result[key], False)

    def test_cli_refuses_missing_file_context_wrong_command_and_missing_arguments(self):
        with tempfile.TemporaryDirectory() as owned:
            missing = Path(owned) / "missing.json"
            for arguments in (self.arguments(missing), self.arguments(missing) + ["--context", owned],
                              ["check", "--record-id", "change-a"], ["bridge-check"]):
                with self.subTest(arguments=arguments):
                    code, result, _ = self.invoke(arguments, {})
                    self.assertEqual(code, 2)
                    self.assertIs(result["ok"], False)

    def test_cli_malformed_shape_and_reference_failures_use_existing_exit2_envelope(self):
        with tempfile.TemporaryDirectory() as owned:
            path = Path(owned) / "bridge.json"
            for raw in ('{', '{"schema":1,"schema":1}', json.dumps(fixture())):
                path.write_text(raw, encoding="utf-8")
                code, result, _ = self.invoke(self.arguments(path), {})
                self.assertEqual(code, 2)
                self.assertIs(result["ok"], False)
