#!/usr/bin/env python3
"""Focused tests of conservative frontier observations; no product execution."""

from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("parallel_frontier", HERE / "parallel_frontier.py")
assert SPEC and SPEC.loader
frontier = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(frontier)
RAW_HASH = hashlib.sha256(b"test plan bytes").hexdigest()


def node(key, *, parent="ROOT", state="planned", depends=(), order=1, kind="atom"):
    return dict(id=key, parent=parent, state=state, depends_on=list(depends),
                order=order, kind=kind, title=key, mandates=[], acceptance=["review"],
                evidence=["verified:fixture"] if state == "accepted" else [])


def plan(*extra):
    return dict(schema=1, plan_id="sample", revision=3, root_node="ROOT", current_node="A",
                mandate=[], node=[node("ROOT", parent="", kind="campaign"),
                                  node("A"), node("B", order=2), *extra])


def metadata(reads=(), writes=()):
    return dict(read_paths=list(reads), write_paths=list(writes))


class ParallelFrontierTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.repo = Path(self.temporary.name).resolve()
        self.plan = plan()
        self.tasks = {"A": metadata(writes=["src/a.py"]),
                      "B": metadata(writes=["src/b.py"])}

    def analyze(self, bindings=None):
        return frontier.analyze(self.plan, self.tasks, self.repo, bindings,
                                plan_sha256=RAW_HASH)

    def binding(self, **overrides):
        result = dict(schema=1, plan_id="sample", plan_revision=3, plan_sha256=RAW_HASH,
                      tasks=[dict(id=key, **copy.deepcopy(value), resources=[])
                             for key, value in self.tasks.items()])
        result.update(overrides)
        return result

    def codes(self):
        return {row["code"] for row in self.analyze()["conflicts"]}

    def test_disjoint_declared_scopes_form_observation_batch_without_authority(self):
        result = self.analyze()
        self.assertEqual(result["proposed_batches"], [["A", "B"]])
        self.assertEqual(result["policy_state"], "requires-binding")
        self.assertFalse(result["dispatch_authorized"])
        self.assertFalse(result["runtime_observed"])
        self.assertFalse(result["batch_order_is_execution_order"])
        self.assertTrue(all(row["perimeter_binding_required"] for row in result["ready"]))
        self.assertTrue(all(row["resource_capacity_unknown"] for row in result["ready"]))

    def test_exact_write_and_read_conflicts_are_reported(self):
        self.tasks["B"] = metadata(reads=["src/a.py"], writes=["src/a.py"])
        self.assertEqual(self.codes(), {"WRITE_READ", "WRITE_WRITE"})
        self.assertEqual(self.analyze()["proposed_batches"], [["A"], ["B"]])
        self.tasks["A"] = metadata(reads=["src/a.py"])
        self.tasks["B"] = metadata(writes=["src/a.py"])
        self.assertEqual(self.codes(), {"READ_WRITE"})

    def test_directory_ancestry_is_conservative_without_prefix_false_positive(self):
        self.tasks["A"] = metadata(writes=["src/a/"])
        self.tasks["B"] = metadata(reads=["src/a/future.py"])
        self.assertEqual(self.codes(), {"WRITE_READ"})
        self.tasks["B"] = metadata(writes=["src/ab/future.py"])
        self.assertEqual(self.codes(), set())
        self.assertEqual(self.analyze()["proposed_batches"], [["A", "B"]])

    def test_dot_backslash_and_host_case_aliases_are_normalized(self):
        self.tasks["B"] = metadata(writes=[r"src\sub\..\.\a.py"])
        self.assertEqual(self.codes(), {"WRITE_WRITE"})
        self.tasks["B"] = metadata(writes=["SRC/A.PY"])
        self.assertEqual(self.codes(), {"WRITE_WRITE"} if os.name == "nt" else set())

    def test_external_wildcard_and_unknown_paths_do_not_prove_disjointness(self):
        for path in ("../outside", "src/../../outside", "/tmp/outside", "C:/outside",
                     r"\\server\share", "src/*.py", "src/[ab].py", "src/a.py:stream"):
            with self.subTest(path=path):
                self.tasks["B"] = metadata(writes=[path])
                result = self.analyze()
                self.assertEqual(result["ready"][1]["perimeter_origin"], "unknown")
                self.assertEqual(result["proposed_batches"], [["A"]])
                self.assertEqual(result["conflicts"][0]["code"], "PERIMETER_UNKNOWN")
        self.tasks["B"] = {"write_paths": []}
        self.assertEqual(self.analyze()["ready"][1]["perimeter_origin"], "unknown")

    def test_reparse_alias_inside_repo_collides_and_outside_repo_is_unknown(self):
        (self.repo / "real").mkdir()
        def make_alias(path, target):
            if os.name == "nt":
                # Junctions exercise the Windows reparse boundary without the
                # administrator privilege needed for a symbolic link. Both
                # names are controlled paths under this test's temporary root.
                result = subprocess.run(["cmd.exe", "/d", "/c", "mklink", "/J",
                                         str(path), str(target)], capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            else:
                path.symlink_to(target, target_is_directory=True)
        make_alias(self.repo / "alias", self.repo / "real")
        self.tasks["A"] = metadata(writes=["real/future.py"])
        self.tasks["B"] = metadata(writes=["alias/future.py"])
        self.assertEqual(self.codes(), {"WRITE_WRITE"})
        # The external target is observed only as an invalid perimeter.
        make_alias(self.repo / "outside-alias", self.repo.parent)
        self.tasks["B"] = metadata(writes=["outside-alias/future.py"])
        self.assertEqual(self.analyze()["ready"][1]["perimeter_origin"], "unknown")

    def test_active_writer_is_never_redispatched_and_conflicts_exclude_admission(self):
        self.plan["node"][2]["state"] = "active"
        self.tasks["B"] = metadata(writes=["src/"])
        result = self.analyze()
        self.assertEqual([row["id"] for row in result["active"]], ["B"])
        self.assertEqual([row["id"] for row in result["ready"]], ["A"])
        self.assertEqual(result["proposed_batches"], [])
        self.assertEqual(result["excluded_from_batches"], [{"id": "A", "reasons": ["ACTIVE_CONFLICT"]}])
        self.assertTrue(result["active"][0]["live_jobs_unverified"])

    def test_unknown_active_scope_blocks_new_work_including_readers(self):
        self.plan["node"][2]["state"] = "active"
        self.tasks.pop("B")
        self.tasks["A"] = metadata(reads=["src/a.py"])
        result = self.analyze()
        self.assertEqual(result["proposed_batches"], [])
        self.assertEqual(result["conflicts"][0]["code"], "PERIMETER_UNKNOWN")

    def test_missing_auxiliary_metadata_is_unknown_not_zero_write_scope(self):
        self.plan["node"].append(node("DOCS", order=3))
        result = self.analyze()
        docs = next(row for row in result["ready"] if row["id"] == "DOCS")
        self.assertEqual(docs["issues"], ["TASK_METADATA_MISSING"])
        self.assertEqual(result["proposed_batches"], [["A", "B"]])

    def test_stale_hash_revision_identity_and_unknown_bindings_are_refused(self):
        for overrides in ({"plan_revision": 2}, {"plan_id": "other"},
                          {"plan_sha256": "0" * 64}, {"plan_revision": True}):
            with self.subTest(overrides=overrides), self.assertRaises(frontier.FrontierError):
                self.analyze(self.binding(**overrides))
        with self.assertRaises(frontier.FrontierError):
            frontier.analyze(self.plan, self.tasks, self.repo, self.binding())
        rows = self.binding()["tasks"]
        for invalid in (rows + [rows[0]], [dict(rows[0], id="absent")]):
            with self.subTest(rows=invalid), self.assertRaises(frontier.FrontierError):
                self.analyze(self.binding(tasks=invalid))

    def test_bound_scope_must_be_contained_in_original_outer_scope(self):
        for invalid in (dict(id="A", read_paths=[], write_paths=["src/"], resources=[]),
                        dict(id="A", read_paths=["other.py"], write_paths=[], resources=[]),
                        dict(id="A", read_paths=[], write_paths=["../outside"], resources=[])):
            with self.subTest(row=invalid), self.assertRaises(frontier.FrontierError):
                self.analyze(self.binding(tasks=[invalid]))
        self.tasks.pop("B")
        with self.assertRaises(frontier.FrontierError):
            self.analyze(self.binding(tasks=[dict(id="B", read_paths=[], write_paths=[], resources=[])]))

    def test_narrowed_scopes_remove_outer_collision_but_keep_other_obligations(self):
        self.tasks = {"A": metadata(writes=["src/"]), "B": metadata(writes=["src/"])}
        self.assertEqual(self.analyze()["proposed_batches"], [["A"], ["B"]])
        bindings = self.binding(tasks=[
            dict(id="A", read_paths=["src/a.py"], write_paths=["src/a.py"], resources=[]),
            dict(id="B", read_paths=["src/b.py"], write_paths=["src/b.py"], resources=[]),
        ])
        result = self.analyze(bindings)
        self.assertEqual(result["proposed_batches"], [["A", "B"]])
        self.assertFalse(result["dispatch_authorized"])
        for row in result["ready"]:
            self.assertEqual(row["perimeter_origin"], "bound")
            self.assertFalse(row["perimeter_binding_required"])
            self.assertTrue(row["source_capture_verification_required"])
            self.assertTrue(row["semantic_binding_required"])
            self.assertTrue(row["verification_binding_required"])

    def test_exclusive_resource_conflicts_and_shared_capacity_stays_unknown(self):
        bindings = self.binding()
        for row in bindings["tasks"]:
            row["resources"] = [dict(id="docker-daemon", mode="shared")]
        result = self.analyze(bindings)
        self.assertEqual(result["proposed_batches"], [["A", "B"]])
        self.assertTrue(all(row["resource_capacity_unknown"] for row in result["ready"]))
        bindings["tasks"][1]["resources"][0]["mode"] = "exclusive"
        result = self.analyze(bindings)
        self.assertEqual(result["proposed_batches"], [["A"], ["B"]])
        self.assertEqual(result["conflicts"][0]["code"], "RESOURCE_EXCLUSIVE")
        self.assertEqual(result["conflicts"][0]["resources"], ["docker-daemon"])
        bindings["tasks"][1]["resources"][0]["mode"] = []
        with self.assertRaises(frontier.FrontierError):
            self.analyze(bindings)

    def test_missing_resources_remain_unknown_explicit_empty_does_not_grant_authority(self):
        bindings = self.binding()
        del bindings["tasks"][0]["resources"]
        result = self.analyze(bindings)
        self.assertTrue(result["ready"][0]["resource_binding_required"])
        self.assertFalse(result["ready"][1]["resource_binding_required"])
        self.assertFalse(result["dispatch_authorized"])

    def test_candidate_review_queue_does_not_erase_independent_frontier(self):
        self.plan["node"].append(node("C", state="candidate", order=3))
        result = self.analyze()
        self.assertEqual(result["review_queue"], ["C"])
        self.assertTrue(result["review_before_expanding"])
        self.assertEqual(result["proposed_batches"], [["A", "B"]])

    def test_execution_hold_is_inherited_and_acceptance_is_not_inferred(self):
        self.plan["node"].append(node("HOLD", parent="", state="blocked", kind="gate"))
        self.plan["node"][0]["parent"] = "HOLD"
        self.plan["root_node"] = "HOLD"
        self.plan["node"][0]["depends_on"] = ["HOLD"]
        # A parent-completion/dependency loop must be refused, not made ready.
        with self.assertRaises(frontier.FrontierError):
            self.analyze()
        self.plan = plan(node("HOLD", state="blocked", kind="gate", order=3),
                         node("PHASE", kind="phase", depends=["HOLD"], order=4))
        self.plan["node"][1]["parent"] = "PHASE"
        self.plan["node"][2]["parent"] = "PHASE"
        result = self.analyze()
        self.assertEqual(result["counts"]["ready"], 0)
        self.assertEqual(result["proposed_batches"], [])
        self.assertEqual(result["blocked"][0]["unsatisfied_dependencies"], ["HOLD"])
        self.plan["node"][3]["state"] = "accepted"
        self.plan["node"][3]["evidence"] = ["owner:execution-authorized"]
        self.assertEqual(self.analyze()["proposed_batches"], [["A", "B"]])
        self.assertEqual(self.plan["node"][4]["state"], "planned")

    def test_open_predecessor_keeps_existing_sequential_dependency(self):
        self.plan["node"][2]["depends_on"] = ["A"]
        result = self.analyze()
        self.assertEqual([row["id"] for row in result["ready"]], ["A"])
        self.assertEqual(result["blocked"][0]["unsatisfied_dependencies"], ["A"])
        self.plan["node"][1]["state"] = "candidate"
        self.assertEqual(self.analyze()["proposed_batches"], [])

    def test_suppressing_parent_states_exclude_descendants_without_new_dependencies(self):
        for state in ("blocked", "deferred", "dropped", "superseded"):
            with self.subTest(state=state):
                self.plan = plan(node("PHASE", kind="phase", state=state, order=3))
                for row in self.plan["node"][1:3]:
                    row["parent"] = "PHASE"
                original = copy.deepcopy(self.plan)
                result = self.analyze()
                self.assertEqual(result["ready"], [])
                self.assertEqual(result["proposed_batches"], [])
                self.assertEqual([row["id"] for row in result["blocked"]], ["A", "B"])
                for row in result["blocked"]:
                    self.assertEqual(row["ancestor_constraints"], [{"id": "PHASE", "state": state}])
                    self.assertEqual(row["reasons"], ["ANCESTOR_STATE_SUPPRESSES_DISPATCH"])
                    self.assertEqual(row["unsatisfied_dependencies"], [])
                self.assertEqual(self.plan, original)
                # Removing the explicit ancestor suppression restores the same
                # declared observation batch; no new dependency was invented.
                self.plan["node"][3]["state"] = "planned"
                self.assertEqual(self.analyze()["proposed_batches"], [["A", "B"]])

    def test_suppressing_root_states_are_included_in_ancestor_constraints(self):
        for state in ("blocked", "deferred", "dropped", "superseded"):
            with self.subTest(state=state):
                self.plan = plan()
                self.plan["node"][0]["state"] = state
                result = self.analyze()
                self.assertEqual(result["ready"], [])
                self.assertEqual(result["proposed_batches"], [])
                self.assertEqual(result["blocked"][0]["ancestor_constraints"],
                                 [{"id": "ROOT", "state": state}])

    def test_suppressed_ancestor_preserves_active_work_and_candidate_review(self):
        for state in ("blocked", "deferred", "dropped", "superseded"):
            with self.subTest(state=state):
                self.plan = plan(node("PHASE", kind="phase", state=state, order=3))
                self.plan["node"][1].update(parent="PHASE", state="active")
                self.plan["node"][2].update(parent="PHASE", state="candidate")
                result = self.analyze()
                self.assertEqual(result["ready"], [])
                self.assertEqual(result["proposed_batches"], [])
                self.assertEqual(result["review_queue"], ["B"])
                self.assertTrue(result["review_before_expanding"])
                self.assertEqual([row["id"] for row in result["active"]], ["A"])
                self.assertTrue(result["active"][0]["live_jobs_unverified"])
                self.assertEqual(result["active"][0]["ancestor_constraints"],
                                 [{"id": "PHASE", "state": state}])
                self.assertEqual(result["active"][0]["reasons"],
                                 ["ANCESTOR_STATE_SUPPRESSES_DISPATCH"])

    def test_inconsistent_acceptance_is_refused(self):
        self.plan["node"][1]["state"] = "accepted"
        with self.assertRaises(frontier.FrontierError):
            self.analyze()
        self.plan["node"][1]["evidence"] = ["check:done"]
        self.plan["node"][1]["depends_on"] = ["B"]
        with self.assertRaises(frontier.FrontierError):
            self.analyze()
        self.plan = plan()
        self.plan["node"][0]["state"] = "accepted"
        self.plan["node"][0]["evidence"] = ["check:done"]
        with self.assertRaises(frontier.FrontierError):
            self.analyze()

    def test_deterministic_observations_leave_all_input_objects_unchanged(self):
        self.plan["node"][1]["contract"] = "preserve:extension"
        bindings = self.binding()
        originals = copy.deepcopy((self.plan, self.tasks, bindings))
        self.assertEqual(self.analyze(bindings), self.analyze(bindings))
        self.assertEqual((self.plan, self.tasks, bindings), originals)

    def test_cli_json_uses_raw_hash_refuses_stale_binding_and_writes_nothing(self):
        lines = [f"{key} = {json.dumps(value)}" for key, value in self.plan.items()
                 if key != "node"]
        for row in self.plan["node"]:
            lines += ["[[node]]"] + [f"{key} = {json.dumps(value)}" for key, value in row.items()]
        raw = ("\n".join(lines) + "\n").encode("utf-8")
        (self.repo / "plan.toml").write_bytes(raw)
        (self.repo / "tasks.json").write_text(json.dumps({"tasks": [
            dict(id=key, **value) for key, value in self.tasks.items()]}), encoding="utf-8")
        bindings = self.binding(plan_sha256=hashlib.sha256(raw).hexdigest())
        bindings["tasks"][0]["resources"] = [{"id": "сборка", "mode": "shared"}]
        (self.repo / "bindings.json").write_text(json.dumps(bindings), encoding="utf-8")
        before = {path.name: path.read_bytes() for path in self.repo.iterdir()}
        command = [sys.executable, "-B", str(HERE / "parallel_frontier.py"),
                   "--plan", str(self.repo / "plan.toml"), "--repo", str(self.repo),
                   "--task-file", str(self.repo / "tasks.json"),
                   "--bindings", str(self.repo / "bindings.json")]
        run = subprocess.run(command, cwd=self.repo, text=True, encoding="utf-8",
                             capture_output=True, env={**os.environ, "PYTHONIOENCODING": "ascii"})
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        result = json.loads(run.stdout)
        self.assertEqual(result["plan_sha256"], hashlib.sha256(raw).hexdigest())
        self.assertEqual(result["proposed_batches"], [["A", "B"]])
        self.assertEqual(result["ready"][0]["resources"][0]["id"], "сборка")
        self.assertEqual(before, {path.name: path.read_bytes() for path in self.repo.iterdir()})
        (self.repo / "plan.toml").write_bytes(raw + b"# same revision, changed bytes\n")
        run = subprocess.run(command, cwd=self.repo, text=True, capture_output=True)
        self.assertEqual(run.returncode, 2)
        self.assertEqual(json.loads(run.stdout)["error"]["code"], "BINDINGS_STALE")
        self.assertFalse(json.loads(run.stdout)["dispatch_authorized"])

    def test_cli_argument_errors_are_json_refusals(self):
        run = subprocess.run([sys.executable, "-B", str(HERE / "parallel_frontier.py")],
                             cwd=self.repo, text=True, capture_output=True)
        self.assertEqual(run.returncode, 2)
        result = json.loads(run.stdout)
        self.assertEqual(result["schema"], 1)
        self.assertFalse(result["dispatch_authorized"])
        self.assertEqual(result["error"]["code"], "ARGUMENTS_INVALID")


if __name__ == "__main__":
    unittest.main()
