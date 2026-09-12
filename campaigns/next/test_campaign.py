"""Counterexamples for the temporary campaign's coverage and retirement checks."""
import copy
import json
from pathlib import Path
import runpy
from types import SimpleNamespace
import unittest
from unittest.mock import patch

MODULE = runpy.run_path(str(Path(__file__).with_name("campaign.py")))
ROOT = MODULE["ROOT"]
MANIFEST = MODULE["read_json"](ROOT / "campaigns/next/manifest.json")


class VerificationRecipes(unittest.TestCase):
    def validate(self, commands, notes=()):
        MODULE["validate_check_recipes"]({"fixture": {"checks": commands, "notes": notes}})

    def test_crate_filter_and_list_do_not_narrow_compilation(self):
        for command in ("cargo test -p example", "cargo test -p example selected_name",
                        "cargo test -p example -- --list",
                        "cargo test -p example -- --list --lib"):
            with self.subTest(command=command):
                with self.assertRaisesRegex(MODULE["Refusal"], "compile target"):
                    self.validate([command])

    def test_compile_target_selection_is_accepted_without_claiming_a_run(self):
        self.validate(["cargo test -p example --lib module::tests:: -- --list",
                       "cargo test -p example --test integration named_case -- --exact",
                       "cargo test -p example --test=integration -- --list",
                       "cargo +stable test -p example --bin cli commands::tests::",
                       "cargo test -p example --doc"])

    def test_broad_selectors_cannot_hide_beside_a_narrow_selector(self):
        for flag in ("--workspace", "--all", "--all-targets", "--all-features", "--tests"):
            with self.subTest(flag=flag):
                with self.assertRaisesRegex(MODULE["Refusal"], "broad Cargo"):
                    self.validate([f"cargo test -p example --lib {flag}"])

    def test_wildcard_or_missing_named_target_is_not_a_concrete_selection(self):
        for suffix in ("--test", "--test -- --list", "--test=*", "--bin cli?", "--lib --test=*"):
            with self.subTest(suffix=suffix):
                with self.assertRaisesRegex(MODULE["Refusal"], "concrete named"):
                    self.validate([f"cargo test -p example {suffix}"])

    def test_unbound_or_conditional_recipe_is_not_misread_as_executed_command(self):
        self.validate(["SELECT BEFORE DISPATCH: name the target before cargo test -p example.",
                       "CONDITIONAL: full panel only for an explicitly justified scope.",
                       "TO CREATE: a permanent behavioral test for the required refusal case."])

    def test_obsolete_imperatives_refuse_but_explanations_do_not(self):
        for note in ("Use full listed crate tests by default. Optional narrowing follows.",
                     "Inspect tests; listed checks intentionally use unfiltered crate suites."):
            with self.subTest(note=note):
                with self.assertRaisesRegex(MODULE["Refusal"], "obsolete broad"):
                    self.validate([], [note])
        self.validate([], ["Do not use full listed crate tests by default.",
                           "The old phrase 'listed checks intentionally use unfiltered crate suites' was removed."])

    def test_ordinary_leaf_cannot_prescribe_full_product_panel(self):
        with self.assertRaisesRegex(MODULE["Refusal"], "ordinary task"):
            self.validate(["bash tools/self-check.sh"])


class CampaignChecks(unittest.TestCase):
    def setUp(self):
        self.plan = MODULE["load_plan"](None)
        self.tasks = MODULE["load_tasks"](MANIFEST)

    def test_real_seed_has_complete_acyclic_coverage(self):
        result = MODULE["check"](MANIFEST, None)
        self.assertEqual(result["baseline_work_packages"], 86)
        self.assertEqual(result["baseline_implementation_tasks"], 201)
        self.assertEqual(result["supplemental_tasks"], 11)
        self.assertEqual(result["implementation_tasks"], len(self.tasks))
        self.assertGreaterEqual(result["implementation_tasks"], 212)
        self.assertFalse(result["product_gates_executed"])
        self.assertFalse(result["preview_change_records_verified"])
        self.assertFalse(result["verification_bindings_executed"])

    def test_missing_task_cannot_be_hidden_by_short_local_plan(self):
        self.plan["node"] = [n for n in self.plan["node"] if n["id"] != "M-01-A.1"]
        with self.assertRaises(MODULE["Refusal"]):
            MODULE["validate_coverage"](MANIFEST, self.tasks, self.plan)

    def test_targeted_policy_cannot_hide_old_unconditional_panel(self):
        for key, requirement in (
            ("M-13", "Pass the coherent full panel and accept the workstream evidence before retirement."),
            ("NEXT-GATE-M-13", "Run actual full tools/self-check.sh from this unchanged checkout."),
            ("NEXT-P0.2", "From the next checkout run Git Bash tools/self-check.sh --quiet; preserve exit status."),
        ):
            with self.subTest(node=key):
                plan = copy.deepcopy(self.plan)
                next(n for n in plan["node"] if n["id"] == key)["acceptance"].append(requirement)
                with self.assertRaisesRegex(MODULE["Refusal"], "obsolete unconditional full panel"):
                    MODULE["validate_coverage"](MANIFEST, self.tasks, plan)

    def test_inherited_prerequisite_deadlock_is_rejected(self):
        nodes = {
            "R": {"parent": "", "depends_on": [], "state": "planned"},
            "P": {"parent": "R", "depends_on": ["Q"], "state": "planned"},
            "p": {"parent": "P", "depends_on": [], "state": "planned"},
            "Q": {"parent": "R", "depends_on": [], "state": "planned"},
            "q": {"parent": "Q", "depends_on": ["p"], "state": "planned"},
        }
        with self.assertRaisesRegex(MODULE["Refusal"], "cycle"):
            MODULE["acyclic"](MODULE["combined_graph"](nodes))

    def test_child_order_cannot_disappear(self):
        next(n for n in self.plan["node"] if n["id"] == "M-01-A.2")["depends_on"] = []
        with self.assertRaisesRegex(MODULE["Refusal"], "predecessor"):
            MODULE["validate_coverage"](MANIFEST, self.tasks, self.plan)

    def test_required_work_cannot_be_marked_deferred(self):
        next(n for n in self.plan["node"] if n["id"] == "M-17-D.1")["state"] = "deferred"
        with self.assertRaisesRegex(MODULE["Refusal"], "silently disappear"):
            MODULE["validate_coverage"](MANIFEST, self.tasks, self.plan)

    def test_parent_cannot_accept_open_children(self):
        root = next(n for n in self.plan["node"] if n["id"] == "NEXT")
        root["state"], root["evidence"] = "accepted", ["candidate assertion"]
        with self.assertRaisesRegex(MODULE["Refusal"], "open child"):
            MODULE["validate_state"](self.plan)

    def test_pending_clause_refuses_retirement(self):
        with self.assertRaisesRegex(MODULE["Refusal"], "unpromoted"):
            MODULE["validate_promotion"]({"id": "pending", "state": "pending"}, MANIFEST)

    def test_planning_hold_forbids_phase_zero_start(self):
        next(n for n in self.plan["node"] if n["id"] == "NEXT-P0.1")["state"] = "active"
        with self.assertRaisesRegex(MODULE["Refusal"], "planning-only hold"):
            MODULE["validate_coverage"](MANIFEST, self.tasks, self.plan)

    def test_unrelated_ancestral_commit_cannot_certify_current_bytes(self):
        path = "vibevm/vibespecs/common/PROP-044-change-native-formats.xml"
        target = {"path": path, "anchor": "root", "sha256": MODULE["digest"]((ROOT / path).read_bytes())}
        evidence = {"path": "Cargo.toml", "sha256": MODULE["digest"]((ROOT / "Cargo.toml").read_bytes())}
        clause = {"id": "test", "state": "promoted", "meaning_review": "review",
                  "reviewer": "central", "acceptance_commit": "a" * 40,
                  "targets": [target], "evidence": [evidence]}
        with patch.object(MODULE["subprocess"], "run", side_effect=[SimpleNamespace(returncode=0), SimpleNamespace(returncode=0, stdout=b"unrelated")]):
            with self.assertRaisesRegex(MODULE["Refusal"], "not bound"):
                MODULE["validate_promotion"](clause, MANIFEST)

    def test_file_zone_does_not_hide_permanent_prefix_sibling(self):
        path = "vibevm/vibespecs/terraforms/NEXT-IMPLEMENTATION-CAMPAIGN.xml-extra"
        self.assertFalse(MODULE["temporary"](path, MANIFEST))

    def test_repository_escape_is_rejected(self):
        with self.assertRaisesRegex(MODULE["Refusal"], "outside repository"):
            MODULE["within"]("../outside-campaign")

    def test_dotted_alias_cannot_hide_temporary_target(self):
        self.assertTrue(MODULE["temporary"]("./campaigns/next/README.md", MANIFEST))
        self.assertTrue(MODULE["temporary"]("campaigns/next/../next/README.md", MANIFEST))

    def test_missing_parent_is_rejected_independently_of_dependencies(self):
        next(n for n in self.plan["node"] if n["id"] == "M-01-A.1")["parent"] = "MISSING-PARENT"
        with self.assertRaises(MODULE["Refusal"]):
            MODULE["validate_state"](self.plan)

    def test_parent_cycle_is_rejected_independently_of_dependencies(self):
        next(n for n in self.plan["node"] if n["id"] == "M-01")["parent"] = "M-01-A"
        with self.assertRaises(MODULE["Refusal"]):
            MODULE["validate_state"](self.plan)

    def test_missing_supplemental_task_is_not_hidden_by_baseline_coverage(self):
        self.plan["node"] = [n for n in self.plan["node"] if n["id"] != "NEXT-PREVIEW-BASE.1"]
        with self.assertRaises(MODULE["Refusal"]):
            MODULE["validate_coverage"](MANIFEST, self.tasks, self.plan)

    def test_supplemental_task_cannot_start_during_planning_hold(self):
        next(n for n in self.plan["node"] if n["id"] == "NEXT-PREVIEW-BASE.1")["state"] = "active"
        with self.assertRaisesRegex(MODULE["Refusal"], "planning-only hold"):
            MODULE["validate_coverage"](MANIFEST, self.tasks, self.plan)

    def test_bootstrap_dependency_cannot_disappear_from_both_plan_and_extra_map(self):
        manifest = copy.deepcopy(MANIFEST)
        key, gate = "M-13-A.1", "NEXT-PREVIEW-BASE.3"
        next(n for n in self.plan["node"] if n["id"] == key)["depends_on"].remove(gate)
        manifest["extra_dependencies"][key].remove(gate)
        with self.assertRaisesRegex(MODULE["Refusal"], "bootstrap prerequisite"):
            MODULE["validate_coverage"](manifest, self.tasks, self.plan)

    def test_supplemental_packet_preserves_old_task_contracts(self):
        before = copy.deepcopy(self.tasks["M-13-A.1"])
        packet = MODULE["task_packet"](MANIFEST, "NEXT-PREVIEW-BASE.1", None)
        self.assertEqual(packet["supplemental_parent"]["owner"], "NEXT-PREVIEW")
        self.assertFalse(packet["ready"])
        self.assertEqual(packet["change_control"]["protocol"]["id"], "NEXT-A26")
        old_packet = MODULE["task_packet"](MANIFEST, "M-13-A.1", None)
        self.assertEqual({key: old_packet[key] for key in MODULE["TASK_FIELDS"]}, before)
        self.assertEqual(old_packet["change_control"]["to_release"], "developer-preview-2")
        self.assertEqual(MODULE["load_tasks"](MANIFEST)["M-13-A.1"], before)

    def test_phase_zero_and_closure_packets_include_change_control(self):
        for key in ("NEXT-P0.1", "NEXT-CLOSE.3"):
            with self.subTest(key=key):
                packet = MODULE["task_packet"](MANIFEST, key, None)
                self.assertEqual(packet["change_control"]["bootstrap_gate"], "NEXT-PREVIEW-BASE.3")
                self.assertIn("planned obligations", packet["change_control"]["status_note"])

    def test_refined_task_resolves_its_declared_baseline_or_supplemental_owner(self):
        self.assertEqual(MODULE["package_ancestor"](MANIFEST, "M-17-A.2.1")["milestone"], "M-17")
        self.assertEqual(MODULE["package_ancestor"](MANIFEST, "NEXT-PREVIEW-BIND.2.1")["owner"], "NEXT-PREVIEW")
        with self.assertRaisesRegex(MODULE["Refusal"], "no declared"):
            MODULE["package_ancestor"](MANIFEST, "UNKNOWN.1.1")

    def test_refinement_still_requires_an_existing_task_parent(self):
        manifest = copy.deepcopy(MANIFEST)
        manifest["refinements"] = [{"id": "MISSING-TASK", "tasks_file": "campaigns/next/tasks/missing.json"}]
        with self.assertRaisesRegex(MODULE["Refusal"], "already declared task"):
            MODULE["load_tasks"](manifest)

    def test_ordered_refinement_still_loads_after_supplemental_packages(self):
        manifest = copy.deepcopy(MANIFEST)
        parent = "NEXT-PREVIEW-BIND.2"
        group = {"id": parent, "tasks": []}
        for number in (1, 2):
            task = copy.deepcopy(self.tasks[parent])
            task["id"] = f"{parent}.{number}"
            group["tasks"].append(task)
        payload = json.dumps(group).encode()
        path = ROOT / "campaigns/next/tasks/fixture-refinement.json"
        manifest["refinements"] = [{"id": parent, "tasks_file": path.relative_to(ROOT).as_posix()}]
        manifest["task_files_sha256"][parent] = MODULE["digest"](payload)
        original_read = Path.read_bytes

        def read_bytes(candidate):
            return payload if candidate == path else original_read(candidate)

        with patch.object(Path, "read_bytes", read_bytes):
            tasks = MODULE["load_tasks"](manifest)
        self.assertEqual(len(tasks), len(self.tasks) + 2)
        self.assertEqual(tasks[parent], self.tasks[parent])
        self.assertEqual(tasks[parent + ".1"]["id"], parent + ".1")

    def test_retired_preview_protocol_resolves_only_to_permanent_targets(self):
        unit = next(unit for unit in MANIFEST["units"] if unit["id"] == "NEXT-A26")
        target = {"path": "vibevm/vibespecs/common/PROP-068-preview-change-accounting.xml", "anchor": "root"}
        clauses = {anchor: {"targets": [target]} for anchor in unit["anchors"]}
        states = {unit["id"]: {"state": "retired"}}
        with patch.dict(MODULE["resolved_contract_units"].__globals__,
                        {"source_contracts": lambda manifest, ledger: ({}, states, clauses)}):
            resolved = MODULE["resolved_contract_units"](MANIFEST, [unit])[0]
        self.assertNotIn("path", resolved)
        self.assertEqual(resolved["state"], "retired")
        self.assertEqual(resolved["permanent_targets"], [target] * len(unit["anchors"]))

    def assert_bad_preview_control(self, control, message):
        nodes = {node["id"]: node for node in self.plan["node"]}
        with patch.dict(MODULE["preview_control"].__globals__, {"read_json": lambda path: control}):
            with self.assertRaisesRegex(MODULE["Refusal"], message):
                MODULE["validate_preview_control"](MANIFEST, nodes, self.tasks)

    def test_dangling_preview_gate_is_rejected(self):
        control = MODULE["preview_control"](MANIFEST)
        control["final_gate"] = "MISSING-PREVIEW-GATE"
        self.assert_bad_preview_control(control, "preview gate missing")

    def test_preview_permanent_home_cannot_escape_or_target_scaffolding(self):
        for path, reason in (("../release.json", "outside repository"),
                             ("campaigns/next/release.json", "targets scaffolding"),
                             ("./campaigns/next/../next/release.json", "targets scaffolding")):
            with self.subTest(path=path):
                control = MODULE["preview_control"](MANIFEST)
                control["permanent_homes"]["transition_index"] = path
                self.assert_bad_preview_control(control, reason)

    def test_future_permanent_homes_need_not_exist_during_planning(self):
        control = MODULE["preview_control"](MANIFEST)
        control["permanent_homes"]["transition_index"] = "docs/releases/future-uncreated-transition/index.json"
        nodes = {node["id"]: node for node in self.plan["node"]}
        with patch.dict(MODULE["preview_control"].__globals__, {"read_json": lambda path: control}):
            self.assertEqual(MODULE["validate_preview_control"](MANIFEST, nodes, self.tasks), control)

    def test_documentation_input_can_arrive_without_starting_implementation(self):
        gate = next(n for n in self.plan["node"] if n["id"] == "NEXT-PREVIEW-DOCS-INPUT")
        self.assertEqual(gate["state"], "blocked")
        gate["state"], gate["evidence"] = "accepted", ["Reviewed exact owner-supplied documentation subject"]
        MODULE["validate_coverage"](MANIFEST, self.tasks, self.plan)
        self.assertNotIn("NEXT-PREVIEW-DOCS.1", {n["id"] for n in MODULE["ready_nodes"](self.plan)})

    def test_original_frozen_inputs_keep_exact_revision_four_bytes(self):
        expected = {
            "campaigns/next/inputs/REFINED-IMPLEMENTATION-PLAN.md": "11cd76253638800fc90897220984912eb7d737ad12a423f9207299f2b70cb9e7",
            "campaigns/next/inputs/PROJECT-REVIEW.md": "084cf62c72427b65dd8b41241a841ca7b5ac521c7936333d9662d78b70986486",
        }
        self.assertEqual({row["path"]: row["sha256"] for row in MANIFEST["baseline_files"]}, expected)
        for path, sha256 in expected.items():
            self.assertEqual(MODULE["digest"]((ROOT / path).read_bytes()), sha256)

    def test_verification_policy_binding_cannot_disappear(self):
        manifest = copy.deepcopy(MANIFEST)
        manifest.pop("verification_policy", None)
        with self.assertRaisesRegex(MODULE["Refusal"], "verification-policy binding missing"):
            MODULE["validate_coverage"](manifest, self.tasks, self.plan)

    def test_workstream_baseline_and_final_verification_obligations_are_required(self):
        policy = MODULE["verification_policy"](MANIFEST)
        for key, acceptance, reason in (("M-01", "workstream_acceptance", "workstream acceptance"),
                                        ("NEXT-GATE-M-01", "workstream_acceptance", "workstream acceptance"),
                                        ("NEXT-P0.2", "baseline_acceptance", "baseline acceptance"),
                                        ("NEXT-P0-GATE", "baseline_acceptance", "baseline acceptance"),
                                        ("NEXT-CLOSE.3", "final_acceptance", "final acceptance")):
            with self.subTest(key=key):
                nodes = {node["id"]: copy.deepcopy(node) for node in self.plan["node"]}
                nodes[key]["acceptance"].remove(policy[acceptance])
                with self.assertRaisesRegex(MODULE["Refusal"], reason):
                    MODULE["validate_verification_policy"](MANIFEST, nodes, self.tasks)

    def test_phase_zero_cannot_become_the_full_panel_gate(self):
        policy = MODULE["verification_policy"](MANIFEST)
        policy["full_panel_gate"] = "NEXT-P0.2"
        with patch.dict(MODULE["verification_policy"].__globals__, {"read_json": lambda path: policy}):
            with self.assertRaisesRegex(MODULE["Refusal"], "gate identity differs"):
                MODULE["verification_policy"](MANIFEST)

    def test_packets_require_binding_without_starting_work_or_claiming_checks(self):
        before = copy.deepcopy(self.plan)
        for key in ("M-01-A.1", "NEXT-P0.2", "NEXT-CLOSE.3"):
            with self.subTest(key=key):
                packet = MODULE["task_packet"](MANIFEST, key, None)
                verification = packet["verification"]
                self.assertEqual(verification["state"], "requires-binding")
                self.assertIn("package_and_compile_target", verification["binding_fields"])
                self.assertIn("affected_claims_and_contract_consumers", verification["binding_fields"])
                self.assertFalse(verification["bindings_executed"])
                self.assertFalse(packet["ready"])
                self.assertEqual(verification["mode"], "full-product" if key == "NEXT-CLOSE.3" else "affected-targets")
        MODULE["validate_coverage"](MANIFEST, self.tasks, self.plan)
        self.assertEqual(self.plan, before)
        self.assertEqual(MODULE["ready_nodes"](self.plan), [])


if __name__ == "__main__":
    unittest.main()
