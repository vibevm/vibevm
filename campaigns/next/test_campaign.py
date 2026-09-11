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


class CampaignChecks(unittest.TestCase):
    def setUp(self):
        self.plan = MODULE["load_plan"](None)
        self.tasks = MODULE["load_tasks"](MANIFEST)

    def test_real_seed_has_complete_acyclic_coverage(self):
        result = MODULE["check"](MANIFEST, None)
        self.assertEqual(result["baseline_work_packages"], 86)
        self.assertEqual(result["implementation_tasks"], 201)
        self.assertFalse(result["product_gates_executed"])

    def test_missing_task_cannot_be_hidden_by_short_local_plan(self):
        self.plan["node"] = [n for n in self.plan["node"] if n["id"] != "M-01-A.1"]
        with self.assertRaises(MODULE["Refusal"]):
            MODULE["validate_coverage"](MANIFEST, self.tasks, self.plan)

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


if __name__ == "__main__":
    unittest.main()
