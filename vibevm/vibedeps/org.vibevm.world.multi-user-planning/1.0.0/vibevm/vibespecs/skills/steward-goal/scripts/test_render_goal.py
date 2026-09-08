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
    def args(
        context: Path,
        *,
        check: bool = False,
        holder: str | None = None,
        session: str | None = None,
    ) -> SimpleNamespace:
        return SimpleNamespace(
            context=context,
            holder_id=holder,
            session_id=session,
            check=check,
        )

    @staticmethod
    def set_offering(context: Path, offer_id: str) -> None:
        custody_path = context / "custody.toml"
        custody = custody_path.read_text(encoding="utf-8")
        custody = custody.replace('state = "held"', 'state = "offering"')
        custody = custody.replace('offer_id = ""', f'offer_id = "{offer_id}"')
        custody_path.write_text(custody, encoding="utf-8", newline="\n")

    @staticmethod
    def write_valid_offer(
        context: Path,
        *,
        bundle_name: str = "20260908T000000Z-formal",
        recorded_offer_id: str = "formal-offer",
        recorded_context_id: str | None = None,
        schema_literal: str = "1",
        source_epoch: int = 3,
        proposed_epoch: int = 4,
    ) -> Path:
        bundle = context / "handoffs" / bundle_name
        bundle.mkdir(parents=True, exist_ok=True)
        context_id = recorded_context_id or context.name
        (bundle / "offer.toml").write_text(
            f"schema = {schema_literal}\n"
            f'handoff_id = "{recorded_offer_id}"\n'
            'kind = "planned"\n'
            'status = "offered"\n'
            f'context_id = "{context_id}"\n'
            f"source_epoch = {source_epoch}\n"
            f"proposed_epoch = {proposed_epoch}\n",
            encoding="utf-8",
            newline="\n",
        )
        return bundle

    @staticmethod
    def write_terminal(bundle: Path, kind: str) -> None:
        context_id = bundle.parents[1].name
        if kind == "receipt":
            body = (
                "schema = 1\n"
                'handoff_id = "formal-offer"\n'
                f'context_id = "{context_id}"\n'
                "source_epoch = 3\n"
                "claimed_epoch = 4\n"
                'result = "accepted"\n'
            )
        else:
            body = (
                "schema = 1\n"
                'kind = "cancellation"\n'
                'handoff_id = "formal-offer"\n'
                f'context_id = "{context_id}"\n'
                "source_epoch = 3\n"
                "cancellation_epoch = 4\n"
            )
        (bundle / f"{kind}.toml").write_text(body, encoding="utf-8", newline="\n")

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
            "revision": ("plan.toml", "revision = 8", "revision = 9"),
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

    def test_different_holder_and_missing_custody_do_not_gate_publication(self) -> None:
        temporary, context = self.context()
        self.addCleanup(temporary.cleanup)
        sentinel = b"sentinel\n"
        (context / "GOAL.md").write_bytes(sentinel)
        result = render_goal.run(
            self.args(context, holder="another-holder", session="another-session")
        )
        self.assertTrue(result["current"])
        self.assertNotEqual((context / "GOAL.md").read_bytes(), sentinel)

        custody_path = context / "custody.toml"
        custody = custody_path.read_text(encoding="utf-8")
        custody_path.write_text(
            custody.replace('state = "held"', 'state = "vacant"'),
            encoding="utf-8",
            newline="\n",
        )
        self.assertTrue(render_goal.run(self.args(context))["current"])

        (context / "custody.toml").unlink()
        (context / "GOAL.md").unlink()
        (context / "GOAL-CLAUDE.txt").unlink()
        without_custody = render_goal.run(self.args(context))
        self.assertTrue(without_custody["current"])

    def test_only_valid_matching_open_offer_fences(self) -> None:
        for case in (
            "stale",
            "missing",
            "malformed",
            "boolean-schema",
            "negative-epoch",
            "mismatched",
            "duplicate",
            "cancelled",
            "receipted",
            "open",
        ):
            with self.subTest(case=case):
                temporary, context = self.context()
                try:
                    goal_sentinel = b"goal sentinel\n"
                    command_sentinel = b"command sentinel\n"
                    (context / "GOAL.md").write_bytes(goal_sentinel)
                    (context / "GOAL-CLAUDE.txt").write_bytes(command_sentinel)
                    self.set_offering(context, "" if case == "stale" else "formal-offer")
                    if case == "malformed":
                        bundle = context / "handoffs" / "20260908T000000Z-formal"
                        bundle.mkdir(parents=True)
                        (bundle / "offer.toml").write_text("[invalid", encoding="utf-8")
                    elif case == "boolean-schema":
                        self.write_valid_offer(context, schema_literal="true")
                    elif case == "negative-epoch":
                        custody_path = context / "custody.toml"
                        custody_path.write_text(
                            custody_path.read_text(encoding="utf-8").replace("epoch = 3", "epoch = -1"),
                            encoding="utf-8",
                            newline="\n",
                        )
                        self.write_valid_offer(context, source_epoch=-1, proposed_epoch=0)
                    elif case == "mismatched":
                        self.write_valid_offer(context, recorded_offer_id="another-offer")
                    elif case == "duplicate":
                        self.write_valid_offer(context)
                        self.write_valid_offer(context, bundle_name="20260908T000001Z-formal")
                    elif case in {"cancelled", "receipted", "open"}:
                        bundle = self.write_valid_offer(context)
                        if case == "cancelled":
                            self.write_terminal(bundle, "cancellation")
                        elif case == "receipted":
                            self.write_terminal(bundle, "receipt")

                    status = render_goal.run(self.args(context, check=True))
                    self.assertFalse(status["current"])
                    if case == "open":
                        self.assertFalse(status["can_write"])
                        with self.assertRaises(render_goal.GoalError) as offering:
                            render_goal.run(self.args(context))
                        self.assertEqual(offering.exception.code, "GOAL_FORMAL_HANDOFF_ACTIVE")
                        self.assertEqual((context / "GOAL.md").read_bytes(), goal_sentinel)
                        self.assertEqual((context / "GOAL-CLAUDE.txt").read_bytes(), command_sentinel)
                    else:
                        self.assertTrue(status["can_write"])
                        self.assertEqual((context / "GOAL.md").read_bytes(), goal_sentinel)
                        self.assertEqual((context / "GOAL-CLAUDE.txt").read_bytes(), command_sentinel)
                        result = render_goal.run(self.args(context))
                        self.assertTrue(result["current"])
                        self.assertTrue(result["can_write"])
                        self.assertNotEqual((context / "GOAL.md").read_bytes(), goal_sentinel)
                        self.assertNotEqual((context / "GOAL-CLAUDE.txt").read_bytes(), command_sentinel)
                finally:
                    temporary.cleanup()

    def test_held_custody_metadata_is_advisory_and_check_never_writes(self) -> None:
        temporary, context = self.context()
        self.addCleanup(temporary.cleanup)
        render_goal.run(self.args(context))
        goal_before = (context / "GOAL.md").read_bytes()
        custody_path = context / "custody.toml"
        custody = custody_path.read_text(encoding="utf-8")
        custody = custody.replace('holder_id = "fixture-holder"', 'holder_id = "later-session"')
        custody = custody.replace('session_id = "fixture-session"', 'session_id = "later-session"')
        custody = custody.replace('heartbeat_at = "2026-08-29T00:00:00Z"', 'heartbeat_at = "2030-01-01T00:00:00Z"')
        custody_path.write_text(custody, encoding="utf-8", newline="\n")

        status = render_goal.run(self.args(context, check=True))
        self.assertTrue(status["current"])
        self.assertTrue(status["can_write"])
        self.assertEqual((context / "GOAL.md").read_bytes(), goal_before)

    def test_valid_offer_appearing_during_snapshot_refuses_without_output(self) -> None:
        temporary, context = self.context()
        self.addCleanup(temporary.cleanup)
        self.set_offering(context, "formal-offer")
        original = render_goal.inputs_unchanged
        changed = False

        def begin_formal_handoff(path: Path, snapshot: dict[str, object]) -> bool:
            nonlocal changed
            if not changed:
                self.write_valid_offer(path)
                changed = True
            return original(path, snapshot)

        with mock.patch.object(render_goal, "inputs_unchanged", side_effect=begin_formal_handoff):
            with self.assertRaises(render_goal.GoalError) as caught:
                render_goal.run(self.args(context))
        self.assertEqual(caught.exception.code, "GOAL_FORMAL_HANDOFF_ACTIVE")
        self.assertFalse((context / "GOAL.md").exists())
        self.assertFalse((context / "GOAL-CLAUDE.txt").exists())

    def test_inherited_global_profile_is_rechecked_before_publication(self) -> None:
        temporary, context = self.context()
        self.addCleanup(temporary.cleanup)
        settings_path = context / "settings.toml"
        settings = settings_path.read_text(encoding="utf-8")
        settings_path.write_text(
            settings.replace('planning_profile = "ultra"\n', ""),
            encoding="utf-8",
            newline="\n",
        )
        config_path = context.parent.parent / "config.toml"
        original = render_goal.inputs_unchanged
        changed = False

        def change_global_profile(path: Path, snapshot: dict[str, object]) -> bool:
            nonlocal changed
            if not changed:
                config = config_path.read_text(encoding="utf-8")
                config_path.write_text(
                    config.replace('planning_profile = "standard"', 'planning_profile = "ultra"'),
                    encoding="utf-8",
                    newline="\n",
                )
                changed = True
            return original(path, snapshot)

        with mock.patch.object(render_goal, "inputs_unchanged", side_effect=change_global_profile):
            result = render_goal.run(self.args(context))
        self.assertEqual(result["planning_profile"], "ultra")

    def test_check_mode_never_creates_outputs(self) -> None:
        temporary, context = self.context()
        self.addCleanup(temporary.cleanup)
        status = render_goal.run(self.args(context, check=True))
        self.assertFalse(status["current"])
        self.assertTrue(status["can_write"])
        self.assertFalse((context / "GOAL.md").exists())
        self.assertFalse((context / "GOAL-CLAUDE.txt").exists())

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
