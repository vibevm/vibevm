#!/usr/bin/env python3
"""Focused tests of the static plan-tree viewer: buckets, counts, safety, bytes."""

from __future__ import annotations

import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest


HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("render_tree", HERE / "render_tree.py")
assert SPEC and SPEC.loader
tree = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(tree)

FIXTURE = HERE.parents[2] / "examples/steward-goal/home/contexts/00000000-0000-4000-8000-000000000001/plan.toml"
STATES = ("planned", "ready", "active", "candidate", "accepted", "blocked",
          "deferred", "dropped", "superseded")


def node(key, *, parent="ROOT", state="planned", kind="atom", order=10, title=None,
         depends=(), acceptance=("review",), evidence=None, **extra):
    row = {
        "id": key, "parent": parent, "order": order, "kind": kind,
        "title": title if title is not None else f"Node {key}", "state": state,
        "zoom": "expanded", "depends_on": list(depends), "mandates": [],
        "acceptance": list(acceptance),
        "evidence": list(evidence if evidence is not None
                         else (["commit:fixture"] if state == "accepted" else [])),
    }
    row.update(extra)
    return row


def plan(*nodes, **overrides):
    result = {
        "schema": 1, "plan_id": "sample", "revision": 7, "root_node": "ROOT",
        "current_node": "ROOT", "updated_at": "2026-01-01T00:00:00Z",
        "mandate": [], "node": [node("ROOT", parent="", kind="campaign", order=0,
                                     state="active"), *nodes],
    }
    result.update(overrides)
    return result


def toml_of(value, indent=""):
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, list):
        return "[" + ", ".join(toml_of(item) for item in value) + "]"
    return json.dumps(str(value), ensure_ascii=False)


def write_plan(path: Path, data: dict) -> None:
    """Serialise a test plan without depending on a TOML writer."""
    lines = [f"{key} = {toml_of(value)}" for key, value in data.items()
             if key not in {"node", "mandate"}]
    for name in ("mandate", "node"):
        if not data.get(name):
            lines.append(f"{name} = []")
        for row in data.get(name, []):
            lines.append(f"\n[[{name}]]")
            lines.extend(f"{key} = {toml_of(value)}" for key, value in row.items())
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


class TreeViewerTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.sandbox = Path(self.temporary.name).resolve()
        self.out = self.sandbox / "out"
        self.plan_path = self.sandbox / "plan.toml"

    def render(self, data=None, *, lang=None, path=None):
        if data is not None:
            write_plan(self.plan_path, data)
        report = tree.render_once(path or self.plan_path, self.out, lang)
        payload = json.loads((self.out / "plan-tree.json").read_text(encoding="utf-8"))
        html = (self.out / "index.html").read_text(encoding="utf-8")
        return report, payload, html

    def nodes_of(self, payload):
        return {row["id"]: row for row in payload["nodes"]}

    # ---- buckets and counts ------------------------------------------------

    def test_shipped_synthetic_fixture_renders_with_counted_leaves(self):
        self.out.mkdir()
        report, payload, html = self.render(path=FIXTURE)
        self.assertTrue(report["ok"])
        self.assertEqual(payload["validation"]["status"], "valid")
        self.assertEqual(payload["plan"]["plan_id"], "fixture-plan")
        rows = self.nodes_of(payload)
        self.assertEqual(rows["BUILD"]["counts"],
                         {"accepted": 1, "candidate": 1, "planned": 2, "deferred": 1})
        self.assertEqual(rows["BUILD"]["leaves"], 5)
        self.assertEqual(payload["totals"], rows["ROOT"]["counts"])
        self.assertIn('role="tree"', html)

    def test_every_protocol_state_maps_to_its_own_display_bucket(self):
        rows = [node(state.upper(), state=state, order=index)
                for index, state in enumerate(STATES)]
        _, payload, _ = self.render(plan(*rows))
        buckets = {row["id"]: row["bucket"] for row in payload["nodes"]}
        self.assertEqual(buckets["ACCEPTED"], "accepted")
        self.assertEqual(buckets["CANDIDATE"], "candidate")
        self.assertEqual(buckets["DROPPED"], "closed")
        self.assertEqual(buckets["SUPERSEDED"], "closed")
        for state in ("planned", "ready", "active", "blocked", "deferred"):
            self.assertEqual(buckets[state.upper()], state)
        self.assertEqual(payload["totals"],
                         {"accepted": 1, "candidate": 1, "active": 1, "ready": 1,
                          "planned": 1, "blocked": 1, "deferred": 1, "closed": 2})
        self.assertEqual(sum(payload["totals"].values()), len(STATES))

    def test_accepted_and_candidate_are_never_merged(self):
        _, payload, html = self.render(plan(node("A", state="accepted"),
                                            node("B", state="candidate", order=20)))
        rows = self.nodes_of(payload)
        self.assertNotEqual(rows["A"]["bucket"], rows["B"]["bucket"])
        labels = payload["strings"]["ru"]
        self.assertEqual(labels["bucket.candidate"], "Сделано · ждёт приёмки")
        self.assertNotEqual(labels["bucket.accepted"], labels["bucket.candidate"])
        self.assertEqual(payload["strings"]["en"]["bucket.candidate"],
                         "Done · awaiting acceptance")
        self.assertEqual(labels["bucket.closed"], "Закрыто без выполнения")
        for bucket in tree.BUCKETS:
            self.assertIn("--b-" + bucket, html)

    def test_counts_cover_leaf_descendants_only_and_sum_exactly(self):
        rows = [
            node("P", kind="phase", state="active"),
            node("P.1", parent="P", state="accepted"),
            node("P.2", parent="P", state="candidate", order=20),
            node("Q", kind="phase", state="blocked", order=20),
        ]
        _, payload, _ = self.render(plan(*rows))
        found = self.nodes_of(payload)
        self.assertEqual(found["P"]["counts"], {"accepted": 1, "candidate": 1})
        self.assertEqual(found["P"]["leaves"], 2)
        self.assertNotIn("active", found["P"]["counts"])
        self.assertEqual(found["Q"]["counts"], {"blocked": 1})
        self.assertEqual(found["ROOT"]["leaves"], 3)
        for row in payload["nodes"]:
            self.assertEqual(sum(row["counts"].values()), row["leaves"])

    def test_unknown_state_is_a_visible_warning_not_a_silent_readiness(self):
        _, payload, _ = self.render(plan(node("A", state="in-progress")))
        rows = self.nodes_of(payload)
        self.assertEqual(rows["A"]["bucket"], "unknown")
        self.assertEqual(rows["A"]["state"], "in-progress")
        codes = {item["code"] for item in payload["warnings"]}
        self.assertIn("NODE_STATE_UNKNOWN", codes)
        self.assertEqual(payload["validation"]["status"], "invalid")
        self.assertIn("unknown state", payload["validation"]["message"])

    # ---- validation banner -------------------------------------------------

    def test_invalid_plan_still_renders_a_tree_under_a_banner(self):
        data = plan(node("A", state="accepted", evidence=[]))
        report, payload, html = self.render(data)
        self.assertFalse(report["ok"])
        self.assertEqual(payload["validation"]["code"], "PLAN_INVALID")
        self.assertIn("accepted", payload["validation"]["message"])
        self.assertEqual(len(payload["nodes"]), 2)
        self.assertIn('id="banner"', html)

    def test_unparsable_plan_reports_the_parse_error_in_the_banner(self):
        self.plan_path.write_text("schema = = 1\n", encoding="utf-8")
        report, payload, _ = self.render()
        self.assertFalse(report["ok"])
        self.assertEqual(payload["validation"]["code"], "PLAN_UNPARSABLE")
        self.assertIn("PLAN_NO_NODES", {item["code"] for item in payload["warnings"]})

    def test_orphans_cycles_and_dangling_edges_become_warnings(self):
        data = plan(node("A", parent="GHOST"), node("B", depends=["NOWHERE"], order=20),
                    node("C", parent="D", order=30), node("D", parent="C", order=40))
        _, payload, _ = self.render(data)
        codes = {item["code"] for item in payload["warnings"]}
        self.assertIn("NODE_PARENT_MISSING", codes)
        self.assertIn("EDGE_TARGET_MISSING", codes)
        self.assertIn("NODE_PARENT_CYCLE", codes)
        self.assertIn("PLAN_MULTIPLE_ROOTS", codes)
        self.assertEqual(payload["roots"][0], "ROOT")
        self.assertEqual(len(payload["nodes"]), len(payload["order"]))

    # ---- safety ------------------------------------------------------------

    def test_plan_text_is_data_and_never_markup(self):
        hostile = '</script><script>alert("x")</script>&<img src=x onerror=1>'
        data = plan(node("A", title=hostile, acceptance=[hostile],
                         evidence=[hostile], state="accepted"))
        _, payload, html = self.render(data)
        self.assertEqual(self.nodes_of(payload)["A"]["title"], hostile)
        self.assertNotIn("<script>alert", html)
        self.assertNotIn("</script><script>", html)
        self.assertNotIn("<img", html)
        self.assertIn("\\u003c/script\\u003e", html)
        head, _, tail = html.partition('<script type="application/json" id="plan-data">')
        block, _, _ = tail.partition("</script>")
        self.assertNotIn("<", block)
        self.assertEqual(json.loads(block.replace("\\u003c", "<").replace("\\u003e", ">")
                                    .replace("\\u0026", "&"))["plan"]["plan_id"], "sample")

    def test_only_the_two_output_files_are_written(self):
        keep = self.sandbox / "untouched.txt"
        keep.write_text("keep", encoding="utf-8")
        write_plan(self.plan_path, plan(node("A")))
        before = sorted(path.name for path in self.sandbox.iterdir())
        plan_bytes = self.plan_path.read_bytes()
        self.render()
        self.assertEqual(sorted(path.name for path in self.sandbox.iterdir()),
                         sorted(before + ["out"]))
        self.assertEqual(sorted(path.name for path in self.out.iterdir()),
                         ["index.html", "plan-tree.json"])
        self.assertEqual(keep.read_text(encoding="utf-8"), "keep")
        self.assertEqual(self.plan_path.read_bytes(), plan_bytes)  # the plan is read-only

    def test_rendering_is_deterministic_and_skips_identical_bytes(self):
        data = plan(node("A", state="candidate"), node("B", order=20))
        first_report, _, first_html = self.render(data)
        first_json = (self.out / "plan-tree.json").read_bytes()
        second_report, _, second_html = self.render()
        self.assertTrue(first_report["changed"])
        self.assertFalse(second_report["changed"])
        self.assertEqual(first_html, second_html)
        self.assertEqual(first_json, (self.out / "plan-tree.json").read_bytes())
        self.assertNotIn("2026-11", first_html.split('id="plan-data"')[0])

    # ---- navigation surfaces ----------------------------------------------

    def test_dependencies_are_typed_edges_rather_than_one_hardcoded_field(self):
        _, payload, _ = self.render(plan(node("A", state="accepted"),
                                         node("B", depends=["A"], order=20)))
        rows = self.nodes_of(payload)
        self.assertEqual(rows["B"]["edges"], [{"kind": "depends_on", "to": "A"}])
        self.assertEqual(rows["A"]["edges"], [])
        self.assertEqual(payload["edge_kinds"], ["depends_on", "completes_after"])

    def test_the_two_dependency_kinds_are_shown_distinctly(self):
        data = plan(node("P", kind="phase", completes_after=["X"]),
                    node("P.1", parent="P", depends=["X"]),
                    node("X", order=90, state="active"))
        _, payload, _ = self.render(data)
        rows = self.nodes_of(payload)
        self.assertEqual(rows["P"]["edges"], [{"kind": "completes_after", "to": "X"}])
        self.assertEqual(rows["P.1"]["edges"], [{"kind": "depends_on", "to": "X"}])
        self.assertNotIn("completes_after", rows["P"]["extra"])
        labels = payload["strings"]["ru"]
        self.assertEqual(labels["edge.depends_on.out"], "Предпосылка старта")
        self.assertEqual(labels["edge.completes_after.out"], "Завершается после")
        self.assertNotEqual(labels["edge.depends_on.in"], labels["edge.completes_after.in"])
        for kind in payload["edge_kinds"]:
            for side in ("out", "in"):
                self.assertIn(f"edge.{kind}.{side}", payload["strings"]["en"])

    def test_extra_node_and_plan_fields_are_surfaced_generically(self):
        data = plan(node("A", owner="team-x", attempts=3, flags=["a", "b"]),
                    note="a plan-level note")
        _, payload, _ = self.render(data)
        self.assertEqual(self.nodes_of(payload)["A"]["extra"],
                         {"attempts": 3, "flags": ["a", "b"], "owner": "team-x"})
        self.assertEqual(payload["plan"]["extra"], {"note": "a plan-level note"})

    def test_current_node_and_initial_expansion_follow_the_plan(self):
        data = plan(node("P", kind="phase", state="active"),
                    node("P.1", parent="P", state="active"),
                    node("DONE", kind="phase", state="accepted", order=20),
                    node("DONE.1", parent="DONE", state="accepted"),
                    node("SUM", kind="phase", state="planned", order=30, zoom="summary"),
                    node("SUM.1", parent="SUM"),
                    current_node="P.1")
        _, payload, _ = self.render(data)
        rows = self.nodes_of(payload)
        self.assertEqual(payload["plan"]["current_node"], "P.1")
        self.assertTrue(rows["ROOT"]["open"])
        self.assertTrue(rows["P"]["open"])
        self.assertFalse(rows["DONE"]["open"], "a fully accepted branch starts folded")
        self.assertFalse(rows["SUM"]["open"], "a summarised branch starts folded")
        self.assertFalse(rows["P.1"]["open"], "a leaf is never openable")
        self.assertEqual(rows["P.1"]["path"], ["ROOT", "P"])
        self.assertEqual(rows["P"]["children"], ["P.1"])

    def test_locale_comes_from_the_flag_then_the_plan_then_settings(self):
        data = plan(node("A"))
        self.assertEqual(self.render(data, lang="en")[0]["lang"], "en")
        self.assertEqual(self.render(dict(data, lang="en"))[0]["lang"], "en")
        (self.sandbox / "settings.toml").write_text('lang = "en"\n', encoding="utf-8")
        self.assertEqual(self.render(data)[0]["lang"], "en")
        (self.sandbox / "settings.toml").write_text('lang = "klingon"\n', encoding="utf-8")
        self.assertEqual(self.render(data)[0]["lang"], "ru")
        self.assertEqual(set(self.render(data)[1]["strings"]), set(tree.LANGS))

    # ---- command interface -------------------------------------------------

    def run_cli(self, *arguments, timeout=60):
        command = [sys.executable, "-B", "-I", str(HERE / "render_tree.py"), *arguments]
        return subprocess.run(command, cwd=self.sandbox, text=True, timeout=timeout,
                              capture_output=True, encoding="utf-8",
                              env={**os.environ, "PYTHONIOENCODING": "utf-8"})

    def test_cli_exit_codes_separate_valid_invalid_and_unreadable(self):
        write_plan(self.plan_path, plan(node("A")))
        run = self.run_cli("--plan", "plan.toml", "--out", "out")
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        self.assertTrue(json.loads(run.stdout)["ok"])
        write_plan(self.plan_path, plan(node("A", state="accepted", evidence=[])))
        run = self.run_cli("--plan", "plan.toml", "--out", "out")
        self.assertEqual(run.returncode, 3, run.stdout + run.stderr)
        self.assertEqual(json.loads(run.stdout)["validation"]["code"], "PLAN_INVALID")
        run = self.run_cli("--plan", "absent.toml", "--out", "out")
        self.assertEqual(run.returncode, 2)
        self.assertEqual(json.loads(run.stdout)["code"], "PLAN_UNREADABLE")

    def test_watch_rerenders_changed_plan_bytes_without_touching_the_plan(self):
        write_plan(self.plan_path, plan(node("A")))
        command = [sys.executable, "-B", "-I", str(HERE / "render_tree.py"),
                   "--plan", "plan.toml", "--out", "out", "--watch", "--interval", "0.05"]
        process = subprocess.Popen(command, cwd=self.sandbox, text=True,
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                   encoding="utf-8")
        try:
            first = json.loads(process.stdout.readline())
            self.assertEqual(first["event"], "rendered")
            self.assertEqual(first["nodes"], 2)
            time.sleep(0.2)
            write_plan(self.plan_path, plan(node("A"), node("B", order=20)))
            deadline = time.monotonic() + 30
            second = None
            while second is None and time.monotonic() < deadline:
                second = json.loads(process.stdout.readline())
            self.assertEqual(second["nodes"], 3)
            self.assertNotEqual(second["plan_sha256"], first["plan_sha256"])
        finally:
            process.terminate()
            process.wait(timeout=30)
            process.stdout.close()
            process.stderr.close()


if __name__ == "__main__":
    unittest.main()
