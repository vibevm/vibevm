"""Semantic and filesystem integration cases for deterministic draft assembly."""

from __future__ import annotations

import copy
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[3]
NAMES = ("MIGRATION-DRAFT.md", "WORKLIST-DRAFT.md")


class AssembleTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        target = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
        cls.binary = target / "debug" / ("preview-transition-assemble.exe" if os.name == "nt" else "preview-transition-assemble")
        # One build per suite, never per case. A supplied path reuses the bound build.
        if os.environ.get("PREVIEW_TRANSITION_ASSEMBLER_BIN"):
            cls.binary = Path(os.environ["PREVIEW_TRANSITION_ASSEMBLER_BIN"])
        else:
            subprocess.run(["cargo", "build", "-p", "xtask", "--bin", "preview-transition-assemble"], cwd=ROOT, check=True)
        cls.base = json.loads((ROOT / "formats/corpora/preview-transition/e1/valid/captured.json").read_text())
        cls.example = json.loads((ROOT / "formats/corpora/release-impact/e1/valid/internal-refactor.json").read_text())

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="preview-assemble-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def project(self, name="project", records=None, transition=None):
        root = self.root / name
        root.mkdir()
        (root / "vibe.toml").write_text('[project]\nname = "fixture"\nversion = "1.0.0"\n', encoding="utf-8")
        transition = copy.deepcopy(self.base if transition is None else transition)
        records = [] if records is None else copy.deepcopy(records)
        transition["change_records"] = sorted(record["change_id"] for record in records)
        for record in records:
            path = root / "vibevm/vibespecs/changes" / record["change_id"]
            path.mkdir(parents=True, exist_ok=True)
            (path / "release-impact.json").write_text(json.dumps(record), encoding="utf-8")
        path = root / "release/transition.json"
        path.parent.mkdir()
        path.write_text(json.dumps(transition), encoding="utf-8")
        return path

    def record(self, identity="change-a"):
        record = copy.deepcopy(self.example)
        record["change_id"] = identity
        record["before_capture_id"] = self.base["dp1"]["capture_id"]
        return record

    def run_binary(self, transition, output=None):
        output = transition.parent / "drafts" if output is None else output
        result = subprocess.run([str(self.binary), str(transition), str(output)], capture_output=True, check=False)
        return result, output

    def fail_without_writes(self, transition):
        result, output = self.run_binary(transition)
        self.assertNotEqual(result.returncode, 0, result.stderr.decode())
        self.assertFalse(output.exists())
        return result

    def rewrite(self, transition, mutate):
        value = json.loads(transition.read_text())
        mutate(value)
        transition.write_text(json.dumps(value), encoding="utf-8")

    def test_no_records_keeps_net_noop_unresolved(self):
        result, output = self.run_binary(self.project())
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        draft = (output / NAMES[0]).read_text()
        self.assertIn("No change records supplied", draft)
        self.assertIn("Net transition effects remain unresolved", draft)
        self.assertIn("DP2 has no capture", draft)

    def test_two_checkout_paths_produce_identical_bytes(self):
        records = [self.record()]
        outputs = []
        for name in ("first-checkout", "second-checkout"):
            result, output = self.run_binary(self.project(name, records))
            self.assertEqual(result.returncode, 0, result.stderr.decode())
            outputs.append([(output / file).read_bytes() for file in NAMES])
        self.assertEqual(*outputs)

    def test_add_remove_same_version_changed_content_and_noop_effects(self):
        for kind, before, after in (("introduced", "absent", "available"), ("removed", "available", "absent"), ("changed", "1.0.0 content aaaa", "1.0.0 content bbbb"), ("unchanged", "same", "same")):
            with self.subTest(kind=kind):
                record = self.record()
                record["effects"] = [{"interface_id": "cli", "capability_id": "cap", "audience": "developer", "platform": "Linux", "kind": kind, "before": before, "after": after, "contract_anchors": ["spec://org.example/tool/cap#root"]}]
                result, output = self.run_binary(self.project(kind, [record]))
                self.assertEqual(result.returncode, 0, result.stderr.decode())
                draft = (output / NAMES[0]).read_text()
                self.assertIn(f"Kind: {kind}", draft)
                self.assertIn(before.replace(".", "\\."), draft)
                self.assertIn(after.replace(".", "\\."), draft)
                self.assertIn("Recorded acceptance: candidate", draft)

    def test_missing_record_refuses_without_creating_output_directory(self):
        transition = self.project()
        self.rewrite(transition, lambda value: value.update(change_records=["missing"]))
        self.fail_without_writes(transition)

    def test_user_noimpact_does_not_erase_documentation_action(self):
        record = self.record()
        record["documentation_impact"] = {"state": "impact", "documentation_action": "update", "reason": "Correct the documented example."}
        result, output = self.run_binary(self.project(records=[record]))
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        for name in NAMES:
            draft = (output / name).read_text()
            self.assertIn("User impact: no\\-impact; migration action: none", draft)
            self.assertIn("Documentation impact: impact; action: update", draft)
            self.assertIn("Correct the documented example\\.", draft)

    def test_missing_noimpact_reason_is_invalid_input(self):
        for name in ("user_impact", "documentation_impact"):
            record = self.record()
            record[name]["reason"] = ""
            self.fail_without_writes(self.project(name, [record]))

    def test_unknown_transition_and_record_epochs(self):
        transition = self.project("transition-epoch")
        self.rewrite(transition, lambda value: value.update(schema=2))
        self.fail_without_writes(transition)
        record = self.record()
        record["schema"] = 2
        self.fail_without_writes(self.project("record-epoch", [record]))

    def test_generated_reader_rejects_unknown_fields_and_wrong_types(self):
        for name, mutate in (("extra-field", lambda value: value.update(extra=True)), ("wrong-type", lambda value: value.update(schema="1"))):
            transition = self.project(name)
            self.rewrite(transition, mutate)
            self.fail_without_writes(transition)

    def test_duplicate_ids_and_unsafe_identity(self):
        for name, ids in (("duplicate", ["change-a", "change-a"]), ("unsafe", ["../outside"])):
            transition = self.project(name, [self.record()])
            self.rewrite(transition, lambda value: value.update(change_records=ids))
            self.fail_without_writes(transition)

    def test_dangling_and_duplicate_relations(self):
        for name, relations in (("dangling", [{"change_id": "missing", "kind": "reverts"}]), ("duplicate-relation", [{"change_id": "change-b", "kind": "reverts"}, {"change_id": "change-b", "kind": "supersedes"}])):
            record = self.record()
            record["relations"] = relations
            self.fail_without_writes(self.project(name, [record]))

    def test_record_identity_mismatch(self):
        transition = self.project(records=[self.record()])
        path = transition.parents[1] / "vibevm/vibespecs/changes/change-a/release-impact.json"
        value = json.loads(path.read_text()); value["change_id"] = "different"
        path.write_text(json.dumps(value))
        self.fail_without_writes(transition)

    def test_pending_unknown_capture_ids_are_explicit_escaped_unresolved(self):
        record = self.record()
        record["before_capture_id"] = "unknown\n# before"
        record["after_capture_id"] = "unknown-after"
        result, output = self.run_binary(self.project(records=[record]))
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        for name in NAMES:
            draft = (output / name).read_text()
            self.assertIn("UNRESOLVED before capture: unknown\\\\u{a}\\# before", draft)
            self.assertIn("UNRESOLVED after capture: unknown\\-after", draft)
            self.assertNotIn("\n# before", draft)

    def reference(self, identity, source=None):
        reference = copy.deepcopy(self.base["dp1"])
        reference["capture_id"] = identity
        reference["source"] = source or f"evidence/{identity}.json"
        return reference

    def test_captured_transition_accepts_intermediate_contribution_subjects(self):
        transition = copy.deepcopy(self.base)
        transition["dp2"] = dict(self.reference("final-dp2"), state="sealed")
        transition["captures"] = [self.reference("intermediate-after"), self.reference("intermediate-before")]
        record = self.record()
        record["before_capture_id"] = "intermediate-before"
        record["after_capture_id"] = "intermediate-after"
        outputs = []
        for name in ("first-intermediate", "second-intermediate"):
            result, output = self.run_binary(self.project(name, [record], transition))
            self.assertEqual(result.returncode, 0, result.stderr.decode())
            drafts = [(output / filename).read_bytes() for filename in NAMES]
            outputs.append(drafts)
            for draft in drafts:
                self.assertIn(b"Additional supplied capture (unverified by assembly)", draft)
                self.assertIn(transition["captures"][0]["capture_file_sha256"].encode(), draft)
                self.assertIn(b"intermediate\\-before", draft)
                self.assertNotIn(b"UNRESOLVED before capture", draft)
                self.assertNotIn(b"UNRESOLVED after capture", draft)
        self.assertEqual(*outputs)

    def test_captured_unknown_before_refuses_before_outputs(self):
        transition = copy.deepcopy(self.base)
        transition["dp2"] = dict(self.reference("final-dp2"), state="candidate")
        record = self.record()
        record["before_capture_id"] = "unknown"
        record["after_capture_id"] = "final-dp2"
        self.fail_without_writes(self.project(records=[record], transition=transition))

    def test_absent_empty_supplied_and_wrong_type_catalogues(self):
        for name, captures in (("absent", None), ("empty", []), ("supplied", [self.reference("intermediate")])):
            transition = copy.deepcopy(self.base)
            if captures is not None:
                transition["captures"] = captures
            result, output = self.run_binary(self.project(name, transition=transition))
            self.assertEqual(result.returncode, 0, result.stderr.decode())
            if name == "empty":
                self.assertIn("explicitly supplied empty", (output / NAMES[0]).read_text())
        transition = copy.deepcopy(self.base)
        transition["captures"] = "wrong"
        self.fail_without_writes(self.project("wrong", transition=transition))

    def test_invalid_conflicting_duplicate_and_unsorted_catalogues_refuse(self):
        malformed = self.reference("intermediate"); malformed["source"] = "../outside.json"
        conflict = copy.deepcopy(self.base["dp1"]); conflict["git_tree_sha1"] = "a" * 40
        for name, references in (("malformed", [malformed]), ("conflict", [conflict]), ("duplicate", [self.reference("same"), self.reference("same")]), ("unsorted", [self.reference("z"), self.reference("a")])):
            transition = copy.deepcopy(self.base)
            transition["captures"] = references
            self.fail_without_writes(self.project(name, transition=transition))

    def test_captured_dp2_mismatch_and_matching_capture(self):
        transition = copy.deepcopy(self.base)
        transition["dp2"] = dict(transition["dp1"], state="candidate", capture_id="exact-after")
        record = self.record()
        self.fail_without_writes(self.project("mismatch", [record], transition))
        record["after_capture_id"] = "exact-after"
        result, output = self.run_binary(self.project("matching", [record], transition))
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertIn("DP2 capture: exact\\-after", (output / NAMES[0]).read_text())

    def test_prose_cannot_inject_markdown_html_or_control(self):
        record = self.record()
        record["user_impact"]["reason"] = "line\n# Accepted\r<script>*yes* [link](url) `code`\x1b\u202e"
        result, output = self.run_binary(self.project(records=[record]))
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        draft = (output / NAMES[0]).read_text()
        self.assertNotIn("\n# Accepted", draft)
        self.assertNotIn("<script>", draft)
        self.assertNotIn("\x1b", draft)
        self.assertNotIn("\u202e", draft)
        self.assertIn("&lt;script&gt;", draft)
        self.assertIn("\\*yes\\*", draft)
        self.assertIn("\\\\u{a}", draft)

    def test_existing_editorial_outputs_are_preserved(self):
        for name in NAMES:
            transition = self.project(name)
            output = transition.parent / "drafts"; output.mkdir()
            (output / name).write_text("editorial prose")
            result, _ = self.run_binary(transition)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual((output / name).read_text(), "editorial prose")
            self.assertEqual(len(list(output.iterdir())), 1)

    def test_existing_and_dangling_symlink_outputs_are_preserved(self):
        for index, name in enumerate(NAMES):
            transition = self.project(str(index))
            output = transition.parent / "drafts"; output.mkdir()
            target = self.root / "editorial"
            if index == 0: target.write_text("editorial target")
            else: target = self.root / "absent"
            link = output / name; link.symlink_to(target)
            result, _ = self.run_binary(transition)
            self.assertNotEqual(result.returncode, 0)
            self.assertTrue(link.is_symlink())
            self.assertEqual(len(list(output.iterdir())), 1)
            if index == 0: self.assertEqual(target.read_text(), "editorial target")
            else: self.assertFalse(target.exists())

    def test_symlink_output_directory_is_refused(self):
        transition = self.project()
        target = self.root / "elsewhere"; target.mkdir()
        output = self.root / "linked"; output.symlink_to(target, target_is_directory=True)
        result, _ = self.run_binary(transition, output)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(list(target.iterdir()), [])

    def test_record_final_and_ancestor_symlinks_are_refused(self):
        for placement in ("final", "ancestor"):
            transition = self.project(placement, [self.record()])
            change = transition.parents[1] / "vibevm/vibespecs/changes/change-a"
            original = change / "release-impact.json"
            if placement == "final":
                retained = self.root / "retained-record.json"
                original.rename(retained)
                original.symlink_to(retained)
            else:
                retained = self.root / "retained-change"
                change.rename(retained)
                change.symlink_to(retained, target_is_directory=True)
            self.fail_without_writes(transition)

    def test_explicit_transition_and_manifest_symlinks_are_refused(self):
        for placement in ("transition", "manifest"):
            transition = self.project(placement)
            source = transition if placement == "transition" else transition.parents[1] / "vibe.toml"
            retained = self.root / (placement + "-retained")
            source.rename(retained)
            source.symlink_to(retained)
            self.fail_without_writes(transition)

    def test_wire_shape_error_does_not_echo_source_values(self):
        record = self.record()
        secret = "arbitrary-input-content-that-must-not-be-echoed"
        record["acceptance"]["state"] = secret
        result = self.fail_without_writes(self.project(records=[record]))
        self.assertNotIn(secret.encode(), result.stderr)
        self.assertIn(b"invalid generated release-impact shape", result.stderr)

    def test_cli_is_only_transport_and_preserves_caller_paths(self):
        spec = importlib.util.spec_from_file_location("preview_assemble", ROOT / "tools/preview-transition/assemble.py")
        module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
        with patch("sys.argv", ["assemble.py", "--transition", "relative.json", "--output-dir", "owned"]), patch.object(module.subprocess, "run") as run:
            run.return_value.returncode = 7
            self.assertEqual(module.main(), 7)
            command = run.call_args.args[0]
            self.assertEqual(command[-2:], [str(Path("relative.json").absolute()), str(Path("owned").absolute())])
            self.assertEqual(run.call_args.kwargs["cwd"], ROOT)


if __name__ == "__main__":
    unittest.main()
