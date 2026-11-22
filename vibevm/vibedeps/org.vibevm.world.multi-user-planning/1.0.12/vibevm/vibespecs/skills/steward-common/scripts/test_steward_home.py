#!/usr/bin/env python3
"""Hermetic tests of steward-home location, repository identity, resolution and bind."""

from __future__ import annotations

import json
import os
from pathlib import Path
import runpy
import shutil
import subprocess
import sys
import tempfile
import tomllib
import unittest
from unittest import mock


HERE = Path(__file__).resolve().parent
steward = runpy.run_path(str(HERE / "steward_home.py"))

CTX_A = "aaaaaaaa-0000-4000-8000-000000000001"
CTX_B = "bbbbbbbb-0000-4000-8000-000000000002"
CTX_C = "cccccccc-0000-4000-8000-000000000003"


def git_env(home: Path) -> dict[str, str]:
    env = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
    env.update({
        "HOME": str(home), "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": os.devnull,
        "GIT_AUTHOR_NAME": "Fixture", "GIT_AUTHOR_EMAIL": "fixture@example.invalid",
        "GIT_COMMITTER_NAME": "Fixture", "GIT_COMMITTER_EMAIL": "fixture@example.invalid",
        "GIT_AUTHOR_DATE": "2026-01-01T00:00:00Z", "GIT_COMMITTER_DATE": "2026-01-01T00:00:00Z",
    })
    return env


class Sandbox(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve()
        self.env = git_env(self.root / "user")
        self.env["GIT_CEILING_DIRECTORIES"] = str(self.root.parent)
        (self.root / "user").mkdir()
        patcher = mock.patch.dict(os.environ, self.env, clear=True)
        patcher.start()
        self.addCleanup(patcher.stop)
        self.home = self.root / "settings" / "steward"
        (self.home / "contexts").mkdir(parents=True)

    def git(self, cwd: Path, *args: str) -> str:
        return subprocess.run(["git", *args], cwd=cwd, env=self.env, check=True,
                              capture_output=True, text=True).stdout.strip()

    def make_repo(self, name: str, message: str = "root") -> Path:
        repo = self.root / name
        repo.mkdir()
        self.git(repo, "init", "-q", "-b", "main")
        (repo / "README").write_text(message + "\n", encoding="utf-8")
        self.git(repo, "add", "README")
        self.git(repo, "commit", "-q", "-m", message)
        return repo

    def clone(self, source: Path, name: str) -> Path:
        self.git(self.root, "clone", "-q", str(source), name)
        return self.root / name

    def context(self, context_id: str, **binding) -> Path:
        directory = self.home / "contexts" / context_id
        directory.mkdir()
        if binding:
            data = {"schema": 2, "context_id": context_id, "project_name": "sample",
                    "default": False, "revisions": [], "seen": [], **binding}
            (directory / "binding.toml").write_text(steward["binding_text"](data), encoding="utf-8")
        return directory

    def resolve(self, repo: Path, **kwargs):
        return steward["resolve"](self.home, repo, **kwargs)


class HomeTests(unittest.TestCase):
    def test_settings_dir_follows_vibe_settings_then_home(self):
        self.assertEqual(steward["steward_home"](None, {"VIBE_SETTINGS": "/x/settings"}),
                         Path("/x/settings/steward"))
        self.assertEqual(steward["steward_home"](None, {}), Path.home() / ".vibe" / "steward")
        self.assertEqual(steward["steward_home"](None, {"VIBE_SETTINGS": ""}),
                         Path.home() / ".vibe" / "steward")

    def test_explicit_home_overrides_the_environment(self):
        self.assertEqual(steward["steward_home"]("/explicit", {"VIBE_SETTINGS": "/x"}), Path("/explicit"))


class RepositoryIdTests(Sandbox):
    def test_two_clones_share_the_id_and_an_unrelated_repository_differs(self):
        origin = self.make_repo("origin")
        first, second = self.clone(origin, "first"), self.clone(origin, "second")
        unrelated = self.make_repo("unrelated", "another root")
        identity = steward["repository_id"](first)
        self.assertRegex(identity, r"^[0-9a-f]{40}$")
        self.assertEqual(identity, steward["repository_id"](second))
        self.assertEqual(identity, steward["repository_id"](origin))
        self.assertNotEqual(identity, steward["repository_id"](unrelated))

    def test_several_roots_choose_the_smallest(self):
        repo = self.make_repo("multi")
        first_root = self.git(repo, "rev-parse", "HEAD")
        self.git(repo, "checkout", "-q", "--orphan", "other")
        (repo / "OTHER").write_text("other\n", encoding="utf-8")
        self.git(repo, "add", "OTHER")
        self.git(repo, "commit", "-q", "-m", "second root")
        second_root = self.git(repo, "rev-parse", "HEAD")
        self.git(repo, "merge", "-q", "--allow-unrelated-histories", "-m", "join", "main")
        self.assertEqual(steward["repository_id"](repo), min(first_root, second_root))

    def test_a_directory_outside_git_is_refused(self):
        plain = self.root / "plain"
        plain.mkdir()
        with self.assertRaises(steward["StewardError"]) as caught:
            steward["repository_id"](plain)
        self.assertEqual(caught.exception.code, "NOT_A_REPOSITORY")

    def test_a_shallow_clone_is_refused_not_given_another_id(self):
        origin = self.make_repo("origin")
        (origin / "README").write_text("second\n", encoding="utf-8")
        self.git(origin, "commit", "-q", "-am", "second")
        self.git(self.root, "clone", "-q", "--depth", "1", origin.as_uri(), "shallow")
        shallow = self.root / "shallow"
        self.assertEqual(self.git(shallow, "rev-parse", "--is-shallow-repository"), "true")
        with self.assertRaises(steward["StewardError"]) as caught:
            steward["repository_id"](shallow)
        self.assertEqual(caught.exception.code, "REPOSITORY_SHALLOW")
        self.context(CTX_A, repository_id=steward["repository_id"](origin))
        run = subprocess.run([sys.executable, "-B", "-I", str(HERE / "resolve_context.py"),
                              "--home", str(self.home), "--repo", str(shallow)],
                             cwd=self.root, env=self.env, capture_output=True, text=True, timeout=60)
        self.assertEqual(run.returncode, 3, run.stdout + run.stderr)
        self.assertEqual(json.loads(run.stdout)["code"], "REPOSITORY_SHALLOW")


class ResolutionTests(Sandbox):
    def setUp(self):
        super().setUp()
        self.repo = self.make_repo("project")
        self.rid = steward["repository_id"](self.repo)

    def test_single_repository_match_wins_whatever_the_branch(self):
        self.context(CTX_A, repository_id=self.rid, revisions=["elsewhere"])
        self.context(CTX_B, repository_id="f" * 40, default=True)
        result = self.resolve(self.repo)
        self.assertEqual((result.status, result.context_id, result.rule), ("found", CTX_A, "repository"))
        self.assertEqual(result.context_dir, self.home / "contexts" / CTX_A)

    def test_branch_selector_chooses_among_several_contexts(self):
        self.context(CTX_A, repository_id=self.rid, default=True)
        self.context(CTX_B, repository_id=self.rid, revisions=["feature"])
        self.git(self.repo, "checkout", "-q", "-b", "feature")
        result = self.resolve(self.repo)
        self.assertEqual((result.status, result.context_id, result.rule), ("found", CTX_B, "revision"))

    def test_full_ref_selector_also_matches(self):
        self.context(CTX_A, repository_id=self.rid)
        self.context(CTX_B, repository_id=self.rid, revisions=["refs/heads/main"])
        self.assertEqual(self.resolve(self.repo).context_id, CTX_B)

    def test_default_is_used_when_no_selector_matches(self):
        self.context(CTX_A, repository_id=self.rid, default=True)
        self.context(CTX_B, repository_id=self.rid, revisions=["feature"])
        result = self.resolve(self.repo)
        self.assertEqual((result.status, result.context_id, result.rule), ("found", CTX_A, "default"))

    def test_ambiguity_is_refused_with_every_candidate_listed(self):
        self.context(CTX_A, repository_id=self.rid)
        self.context(CTX_B, repository_id=self.rid)
        result = self.resolve(self.repo)
        self.assertEqual(result.status, "ambiguous")
        self.assertIsNone(result.context_id)
        self.assertEqual(sorted(c.context_id for c in result.candidates), [CTX_A, CTX_B])
        self.assertEqual(sorted(p.name for p in (self.home / "contexts").iterdir()), [CTX_A, CTX_B])

    def test_schema_1_binding_matches_its_exact_path_tuple(self):
        repository = steward["Repository"](self.repo)
        directory = self.context(CTX_C)
        (directory / "binding.toml").write_text(
            f'schema = 1\ncontext_id = "{CTX_C}"\nproject_name = "legacy"\n'
            f'repository_root = {json.dumps(repository.repository_root)}\n'
            f'worktree_root = {json.dumps(repository.worktree_root)}\n'
            f'git_common_dir = {json.dumps(repository.git_common_dir)}\n'
            f'revision_selector = "refs/heads/main"\n', encoding="utf-8")
        result = self.resolve(self.repo)
        self.assertEqual((result.status, result.context_id, result.rule), ("found", CTX_C, "schema-1-path"))
        self.git(self.repo, "checkout", "-q", "-b", "other")
        self.assertEqual(self.resolve(self.repo).status, "none")

    def test_copied_home_resolves_although_seen_paths_are_gone(self):
        self.context(CTX_A, repository_id=self.rid, default=True,
                     seen=[{"worktree_root": str(self.root / "old-machine" / "project")}])
        copied_home = self.root / "other-machine" / "steward"
        shutil.copytree(self.home, copied_home)
        clone = self.clone(self.repo, "elsewhere")
        result = steward["resolve"](copied_home, clone)
        self.assertEqual((result.status, result.context_id), ("found", CTX_A))
        self.assertEqual(result.context_dir, copied_home / "contexts" / CTX_A)

    def test_explicit_id_wins_and_no_match_creates_nothing(self):
        self.context(CTX_A, repository_id="e" * 40)
        self.assertEqual(self.resolve(self.repo, context_id=CTX_A).context_id, CTX_A)
        result = self.resolve(self.repo)
        self.assertEqual(result.status, "none")
        self.assertEqual([p.name for p in (self.home / "contexts").iterdir()], [CTX_A])

    def test_invalid_binding_is_reported_not_silently_used(self):
        directory = self.context(CTX_A)
        (directory / "binding.toml").write_text('schema = 2\ncontext_id = "wrong"\n', encoding="utf-8")
        result = self.resolve(self.repo)
        self.assertEqual(result.status, "none")
        self.assertEqual(result.invalid[0]["code"], "BINDING_INVALID")

    def run_cli(self, script: str, *arguments: str, cwd: Path) -> subprocess.CompletedProcess:
        return subprocess.run([sys.executable, "-B", "-I", str(HERE / script), *arguments], cwd=cwd,
                              env=self.env, capture_output=True, text=True, timeout=60)

    def test_cli_reports_json_and_exit_codes(self):
        run = self.run_cli("resolve_context.py", "--home", str(self.home), cwd=self.repo)
        self.assertEqual(run.returncode, 1, run.stdout + run.stderr)
        self.assertEqual(json.loads(run.stdout)["status"], "none")
        self.context(CTX_A, repository_id=self.rid)
        run = self.run_cli("resolve_context.py", "--home", str(self.home), cwd=self.repo)
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        report = json.loads(run.stdout)
        self.assertEqual((report["status"], report["context_id"]), ("found", CTX_A))
        self.assertEqual(report["context_dir"], str(self.home / "contexts" / CTX_A))
        self.context(CTX_B, repository_id=self.rid)
        run = self.run_cli("resolve_context.py", "--home", str(self.home), cwd=self.repo)
        self.assertEqual(run.returncode, 2, run.stdout + run.stderr)
        self.assertEqual(len(json.loads(run.stdout)["candidates"]), 2)
        settings = self.home.parent
        env = {**self.env, "VIBE_SETTINGS": str(settings)}
        run = subprocess.run([sys.executable, "-B", "-I", str(HERE / "resolve_context.py"),
                              "--repo", str(self.repo)], cwd=self.root, env=env,
                             capture_output=True, text=True, timeout=60)
        self.assertEqual(json.loads(run.stdout)["home"], str(self.home))
        run = self.run_cli("resolve_context.py", "--home", str(self.home), cwd=self.root)
        self.assertEqual(run.returncode, 3, run.stdout + run.stderr)


class BindTests(Sandbox):
    def setUp(self):
        super().setUp()
        self.repo = self.make_repo("project")
        self.rid = steward["repository_id"](self.repo)

    def bind(self, *arguments: str) -> subprocess.CompletedProcess:
        return subprocess.run([sys.executable, "-B", "-I", str(HERE / "bind_context.py"), *arguments],
                              cwd=self.root, env=self.env, capture_output=True, text=True, timeout=60)

    def test_schema_1_converts_to_schema_2_keeping_identity(self):
        directory = self.context(CTX_A)
        legacy = {"repository_root": "/gone/project", "worktree_root": "/gone/project",
                  "git_common_dir": "/gone/project/.git", "revision_selector": "refs/heads/main"}
        (directory / "binding.toml").write_text(
            f'schema = 1\ncontext_id = "{CTX_A}"\nproject_name = "legacy-name"\n'
            + "".join(f"{key} = {json.dumps(value)}\n" for key, value in legacy.items()), encoding="utf-8")
        run = self.bind("--context", str(directory), "--repo", str(self.repo), "--default")
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        self.assertEqual(json.loads(run.stdout)["converted_from_schema"], 1)
        data = tomllib.loads((directory / "binding.toml").read_text(encoding="utf-8"))
        self.assertEqual(data["schema"], 2)
        self.assertEqual((data["context_id"], data["project_name"]), (CTX_A, "legacy-name"))
        self.assertEqual(data["repository_id"], self.rid)
        self.assertIs(data["default"], True)
        self.assertEqual(data["revisions"], [])
        self.assertEqual(data["seen"], [legacy])
        self.assertFalse(set(legacy) & set(data))
        self.assertEqual(self.resolve(self.repo).context_id, CTX_A)

    def test_a_second_default_is_refused_unless_taken(self):
        first = self.context(CTX_A, repository_id=self.rid, default=True)
        second = self.context(CTX_B)
        before = (second / "binding.toml").exists()
        run = self.bind("--context", str(second), "--repo", str(self.repo), "--default")
        self.assertEqual(run.returncode, 2, run.stdout + run.stderr)
        self.assertEqual(json.loads(run.stdout)["code"], "DEFAULT_HELD")
        self.assertEqual((second / "binding.toml").exists(), before)
        run = self.bind("--context", str(second), "--repo", str(self.repo), "--revision", "feature")
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        run = self.bind("--context", str(second), "--repo", str(self.repo), "--take-default")
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        self.assertEqual(json.loads(run.stdout)["released_default"], [CTX_A])
        self.assertIs(tomllib.loads((first / "binding.toml").read_text(encoding="utf-8"))["default"], False)
        data = tomllib.loads((second / "binding.toml").read_text(encoding="utf-8"))
        self.assertEqual((data["default"], data["revisions"]), (True, ["feature"]))
        self.assertEqual(self.resolve(self.repo).context_id, CTX_B)

    def test_bind_refuses_a_missing_context_and_a_non_repository(self):
        run = self.bind("--context", str(self.home / "contexts" / CTX_C), "--repo", str(self.repo))
        self.assertEqual(json.loads(run.stdout)["code"], "CONTEXT_NOT_FOUND")
        directory = self.context(CTX_C)
        run = self.bind("--context", str(directory), "--repo", str(self.root))
        self.assertEqual(json.loads(run.stdout)["code"], "NOT_A_REPOSITORY")
        self.assertFalse((directory / "binding.toml").exists())

    def test_bind_refuses_a_context_outside_a_contexts_directory(self):
        stray = self.root / "elsewhere" / CTX_C
        stray.mkdir(parents=True)
        run = self.bind("--context", str(stray), "--repo", str(self.repo), "--default")
        self.assertEqual(run.returncode, 2, run.stdout + run.stderr)
        self.assertEqual(json.loads(run.stdout)["code"], "CONTEXT_NOT_FOUND")
        self.assertFalse((stray / "binding.toml").exists())

    def test_bind_refuses_a_shallow_clone(self):
        (self.repo / "README").write_text("second\n", encoding="utf-8")
        self.git(self.repo, "commit", "-q", "-am", "second")
        self.git(self.root, "clone", "-q", "--depth", "1", self.repo.as_uri(), "shallow")
        directory = self.context(CTX_C)
        run = self.bind("--context", str(directory), "--repo", str(self.root / "shallow"))
        self.assertEqual(run.returncode, 2, run.stdout + run.stderr)
        self.assertEqual(json.loads(run.stdout)["code"], "REPOSITORY_SHALLOW")
        self.assertFalse((directory / "binding.toml").exists())


if __name__ == "__main__":
    unittest.main()
