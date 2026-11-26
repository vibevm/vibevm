#!/usr/bin/env python3
"""Tests of the `bin/plan-tree` launcher: discovery, shared plan across clones, refusals."""

from __future__ import annotations

import contextlib
import importlib.machinery
import importlib.util
import io
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest import mock
import uuid


HERE = Path(__file__).resolve().parent
LAUNCHER = HERE.parent / "bin" / "plan-tree"
SKILLS = HERE.parents[1]
PACKAGE = HERE.parents[2]
EXAMPLE_HOME = PACKAGE / "examples/steward-goal/home"
BIND = SKILLS / "steward-common/scripts/bind_context.py"
INSTALLED = "vibevm/vibedeps/org.vibevm.world.multi-user-planning/{version}/vibevm/vibespecs/skills"


def load_launcher():
    """Import the extension-less launcher by path, as a fresh module each time."""
    loader = importlib.machinery.SourceFileLoader("plan_tree_launcher", str(LAUNCHER))
    spec = importlib.util.spec_from_loader("plan_tree_launcher", loader)
    assert spec
    module = importlib.util.module_from_spec(spec)
    loader.exec_module(module)
    return module


class PlanTreeLauncherTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.sandbox = Path(self.temporary.name).resolve()
        self.settings = self.sandbox / "settings"
        env = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
        env.update({"PYTHONIOENCODING": "utf-8", "HOME": str(self.sandbox),
                    "USERPROFILE": str(self.sandbox), "GIT_CONFIG_NOSYSTEM": "1",
                    "GIT_CONFIG_GLOBAL": os.devnull,
                    "GIT_CEILING_DIRECTORIES": str(self.sandbox.parent),
                    "VIBE_SETTINGS": str(self.settings)})
        self.env = env

    # ---- world construction ------------------------------------------------

    def git(self, *args, cwd: Path):
        return subprocess.run(["git", *args], cwd=cwd, env=self.env, check=True,
                              capture_output=True, text=True)

    def make_repo(self, name: str) -> Path:
        repo = self.sandbox / name
        repo.mkdir()
        self.git("init", "-q", "-b", "main", cwd=repo)
        self.git("-c", "user.name=F", "-c", "user.email=f@example.invalid",
                 "commit", "-q", "--allow-empty", "-m", "root", cwd=repo)
        return repo

    @staticmethod
    def install(repo: Path, version: str = "1.0.12") -> Path:
        target = repo / INSTALLED.format(version=version)
        shutil.copytree(SKILLS, target, ignore=shutil.ignore_patterns("__pycache__"))
        return target

    def make_context(self) -> Path:
        home = self.settings / "steward"
        home.mkdir(parents=True)
        shutil.copyfile(EXAMPLE_HOME / "config.toml", home / "config.toml")
        example = next((EXAMPLE_HOME / "contexts").iterdir())
        context = home / "contexts" / str(uuid.uuid4())
        context.mkdir(parents=True)
        for name in ("plan.toml", "settings.toml"):
            shutil.copyfile(example / name, context / name)
        return context

    def bind(self, context: Path, repo: Path) -> None:
        done = subprocess.run([sys.executable, "-B", str(BIND), "--context", str(context),
                               "--repo", str(repo), "--default"],
                              env=self.env, capture_output=True, text=True, encoding="utf-8")
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)

    def make_world(self):
        repo = self.make_repo("repo")
        self.install(repo)
        context = self.make_context()
        self.bind(context, repo)
        return repo, context

    def launch(self, cwd: Path, *arguments):
        return subprocess.run([sys.executable, "-B", str(LAUNCHER), *arguments], cwd=cwd,
                              env=self.env, capture_output=True, text=True, encoding="utf-8",
                              timeout=120)

    # ---- cases -------------------------------------------------------------

    def test_world_prints_the_context_page_as_one_line(self):
        repo, context = self.make_world()
        run = self.launch(repo)
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        page = context / "view" / "index.html"
        self.assertEqual(run.stdout.splitlines(), [str(page)])
        self.assertTrue(page.is_file())
        nested = repo / "vibevm"
        again = self.launch(nested)
        self.assertEqual((again.returncode, again.stdout), (0, run.stdout), again.stderr)

    def test_a_second_clone_prints_the_same_page(self):
        repo, context = self.make_world()
        first = self.launch(repo)
        self.assertEqual(first.returncode, 0, first.stdout + first.stderr)
        other = self.sandbox / "other"
        self.git("clone", "-q", str(repo), str(other), cwd=self.sandbox)
        self.install(other)
        second = self.launch(self.sandbox, "--repo", str(other))
        self.assertEqual(second.returncode, 0, second.stdout + second.stderr)
        self.assertEqual(second.stdout, first.stdout)
        self.assertEqual(second.stdout.strip(), str(context / "view" / "index.html"))

    def test_outside_a_checkout_it_refuses(self):
        outside = self.sandbox / "plain"
        outside.mkdir()
        run = self.launch(outside)
        self.assertEqual(run.returncode, 1, run.stdout + run.stderr)
        self.assertTrue(run.stderr.startswith("plan-tree:"), run.stderr)
        self.assertEqual(run.stdout, "")

    def test_a_checkout_without_the_package_names_the_glob(self):
        repo = self.make_repo("bare")
        run = self.launch(repo)
        self.assertEqual(run.returncode, 1, run.stdout + run.stderr)
        self.assertTrue(run.stderr.startswith("plan-tree:"), run.stderr)
        self.assertIn(load_launcher().PACKAGE_GLOB, run.stderr)

    def test_an_old_installed_copy_asks_for_an_update(self):
        repo = self.make_repo("old")
        old = self.install(repo, "1.0.0")
        shutil.rmtree(old / "steward-common")
        run = self.launch(repo)
        self.assertEqual(run.returncode, 1, run.stdout + run.stderr)
        self.assertTrue(run.stderr.startswith("plan-tree:"), run.stderr)
        self.assertIn("1.0.0", run.stderr)
        self.assertIn("update this checkout to 1.0.12", run.stderr)

    def test_open_hands_the_page_to_the_browser_opener(self):
        repo, context = self.make_world()
        launcher = load_launcher()
        opened = []
        out, err = io.StringIO(), io.StringIO()
        with mock.patch.object(launcher, "open_in_browser", opened.append), \
                mock.patch.dict(os.environ, self.env, clear=True), \
                mock.patch.object(sys, "argv", ["plan-tree", "--open", "--repo", str(repo)]), \
                contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = launcher.main()
        page = context / "view" / "index.html"
        self.assertEqual(code, 0, err.getvalue())
        self.assertEqual(opened, [page])
        self.assertEqual(out.getvalue().splitlines(), [str(page)])
        self.assertTrue(page.is_file())


if __name__ == "__main__":
    unittest.main()
