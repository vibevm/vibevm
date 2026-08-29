#!/usr/bin/env python3
"""Focused behavioral tests for the deterministic goal renderer."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import shutil
from types import SimpleNamespace
import tempfile
import unittest
from unittest import mock


HERE = Path(__file__).resolve().parent
VIBESPECS = HERE.parents[2]
FIXTURE = VIBESPECS / "examples" / "steward-goal"
SPEC = importlib.util.spec_from_file_location("render_goal", HERE / "render_goal.py")
assert SPEC and SPEC.loader
render_goal = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(render_goal)


class RendererTests(unittest.TestCase):
    def context(self) -> tuple[tempfile.TemporaryDirectory[str], Path]:
        temporary = tempfile.TemporaryDirectory()
        home = Path(temporary.name) / "home"
        shutil.copytree(FIXTURE / "home", home)
        context = home / "contexts" / "00000000-0000-4000-8000-000000000001"
        (context / "GOAL.md").unlink(missing_ok=True)
        (context / "GOAL-CLAUDE.txt").unlink(missing_ok=True)
        return temporary, context

    @staticmethod
    def args(context: Path, *, check: bool = False, holder: str = "fixture-holder") -> SimpleNamespace:
        return SimpleNamespace(
            context=context,
            holder_id=holder,
            session_id="fixture-session",
            check=check,
        )

    def test_golden_and_second_render_are_byte_identical(self) -> None:
        temporary, context = self.context()
        self.addCleanup(temporary.cleanup)
        first = render_goal.run(self.args(context))
        self.assertTrue(first["current"])
        self.assertEqual((context / "GOAL.md").read_bytes(), (FIXTURE / "expected" / "GOAL.txt").read_bytes())
        self.assertEqual(
            (context / "GOAL-CLAUDE.txt").read_bytes(),
            (FIXTURE / "expected" / "GOAL-CLAUDE.txt").read_bytes(),
        )
        goal = (context / "GOAL.md").read_text(encoding="utf-8")
        accepted_section = goal.split("## Accepted boundary\n", 1)[1].split("\n## Current candidates", 1)[0]
        candidate_section = goal.split("## Current candidates\n", 1)[1].split("\n## Remaining route", 1)[0]
        self.assertNotIn("[R2]", accepted_section)
        self.assertIn("[R2] Candidate implementation", candidate_section)
        second = render_goal.run(self.args(context))
        self.assertFalse(second["goal_changed"])
        self.assertFalse(second["claude_goal_changed"])

    def test_ambiguous_campaigns_refuse_without_writing(self) -> None:
        temporary, context = self.context()
        self.addCleanup(temporary.cleanup)
        settings = (context / "settings.toml").read_text(encoding="utf-8")
        (context / "settings.toml").write_text(
            settings.replace('goal_node = "BUILD"\n', ""), encoding="utf-8", newline="\n"
        )
        with (context / "plan.toml").open("a", encoding="utf-8", newline="\n") as stream:
            stream.write(
                "\n[[node]]\n"
                'id = "OTHER"\nparent = "ROOT"\norder = 20\nkind = "campaign"\n'
                'title = "Other campaign"\nstate = "active"\nzoom = "summary"\n'
                'depends_on = []\nmandates = []\nacceptance = ["other done"]\nevidence = []\n'
            )
        with self.assertRaises(render_goal.GoalError) as caught:
            render_goal.run(self.args(context))
        self.assertEqual(caught.exception.code, "GOAL_NODE_AMBIGUOUS")
        self.assertFalse((context / "GOAL.md").exists())

    def test_staleness_tracks_semantic_inputs_not_interaction_mode(self) -> None:
        temporary, context = self.context()
        self.addCleanup(temporary.cleanup)
        render_goal.run(self.args(context))
        settings_path = context / "settings.toml"
        settings = settings_path.read_text(encoding="utf-8")
        settings_path.write_text(settings.replace('interaction_mode = "auto"', 'interaction_mode = "collab"'), encoding="utf-8", newline="\n")
        self.assertTrue(render_goal.run(self.args(context, check=True))["current"])
        with (context / "plan.toml").open("a", encoding="utf-8", newline="\n") as stream:
            stream.write("\n# raw-byte freshness probe\n")
        self.assertFalse(render_goal.run(self.args(context, check=True))["current"])

    def test_revision_selection_and_profile_each_make_goal_stale(self) -> None:
        mutations = {
            "revision": ("plan.toml", "revision = 7", "revision = 8"),
            "selection": ("settings.toml", 'goal_node = "BUILD"', 'goal_node = "ROOT"'),
            "profile": (
                "settings.toml",
                'planning_profile = "ultra"',
                'planning_profile = "standard"',
            ),
        }
        for name, (filename, old, new) in mutations.items():
            with self.subTest(name=name):
                temporary, context = self.context()
                try:
                    render_goal.run(self.args(context))
                    path = context / filename
                    path.write_text(
                        path.read_text(encoding="utf-8").replace(old, new),
                        encoding="utf-8",
                        newline="\n",
                    )
                    self.assertFalse(render_goal.run(self.args(context, check=True))["current"])
                finally:
                    temporary.cleanup()

    def test_non_holder_and_offering_never_replace_sentinel(self) -> None:
        temporary, context = self.context()
        self.addCleanup(temporary.cleanup)
        sentinel = b"sentinel\n"
        (context / "GOAL.md").write_bytes(sentinel)
        with self.assertRaises(render_goal.GoalError) as wrong_holder:
            render_goal.run(self.args(context, holder="another-holder"))
        self.assertEqual(wrong_holder.exception.code, "GOAL_NOT_HOLDER")
        self.assertEqual((context / "GOAL.md").read_bytes(), sentinel)
        custody_path = context / "custody.toml"
        custody = custody_path.read_text(encoding="utf-8").replace('state = "held"', 'state = "offering"')
        custody_path.write_text(custody, encoding="utf-8", newline="\n")
        with self.assertRaises(render_goal.GoalError) as offering:
            render_goal.run(self.args(context))
        self.assertEqual(offering.exception.code, "GOAL_NOT_HOLDER")
        self.assertEqual((context / "GOAL.md").read_bytes(), sentinel)

    def test_two_moved_snapshots_refuse_without_output(self) -> None:
        temporary, context = self.context()
        self.addCleanup(temporary.cleanup)
        with mock.patch.object(render_goal, "inputs_unchanged", return_value=False):
            with self.assertRaises(render_goal.GoalError) as caught:
                render_goal.run(self.args(context))
        self.assertEqual(caught.exception.code, "GOAL_INPUT_MOVED")
        self.assertFalse((context / "GOAL.md").exists())
        self.assertFalse((context / "GOAL-CLAUDE.txt").exists())

    def test_large_route_uses_exact_bounded_fallback(self) -> None:
        temporary, context = self.context()
        self.addCleanup(temporary.cleanup)
        with (context / "plan.toml").open("a", encoding="utf-8", newline="\n") as stream:
            for index in range(600):
                stream.write(
                    "\n[[node]]\n"
                    f'id = "C{index:03d}"\nparent = "BUILD"\norder = {1000 + index}\n'
                    'kind = "atom"\n'
                    f'title = "Large candidate number {index:03d} with deterministic text"\n'
                    'state = "candidate"\nzoom = "summary"\ndepends_on = ["R1"]\n'
                    'mandates = ["M-001"]\nacceptance = ["candidate accepted"]\nevidence = []\n'
                )
        result = render_goal.run(self.args(context))
        command = (context / "GOAL-CLAUDE.txt").read_text(encoding="utf-8").rstrip("\n")
        condition = command.removeprefix("/goal ")
        self.assertLessEqual(render_goal.utf16_units(condition), 4000)
        self.assertIn("more in GOAL.md", condition)
        self.assertEqual(result["claude_condition_utf16_units"], render_goal.utf16_units(condition))


if __name__ == "__main__":
    unittest.main()
