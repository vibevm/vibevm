#!/usr/bin/env python3
"""Shared steward-home helpers: home location, repository identity, bindings.

Implements `user-local-state.xml#STATE-HOME`, `#REPOSITORY-IDENTITY`,
`#BINDING-SCHEMA-2` and `#CONTEXT-RESOLUTION`. Every function here is read-only
except `write_binding` and `release_default`, which only the explicit `bind`
command calls. Other scripts load this file by path (`runpy.run_path`), so it
needs no packaging and no third-party module.
"""

from __future__ import annotations

import os
from pathlib import Path
import re
import subprocess
import tempfile
import tomllib
from typing import Any


SETTINGS_ENV = "VIBE_SETTINGS"
BINDING_NAME = "binding.toml"
CONTEXTS_NAME = "contexts"
REPOSITORY_ID_RE = re.compile(r"^(?:[0-9a-f]{40}|[0-9a-f]{64})$")
CONTEXT_ID_RE = re.compile(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")
SCHEMA1_TUPLE = ("repository_root", "worktree_root", "git_common_dir", "revision_selector")


class StewardError(RuntimeError):
    """A typed refusal from home, repository or binding handling."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


# --- B1: the steward home -------------------------------------------------


def settings_dir(environ: dict[str, str] | None = None) -> Path:
    """`$VIBE_SETTINGS` when set and non-empty, otherwise `~/.vibe`."""
    env = os.environ if environ is None else environ
    value = env.get(SETTINGS_ENV, "")
    if value:
        return Path(value).expanduser()
    return Path.home() / ".vibe"


def steward_home(override: Path | str | None = None, environ: dict[str, str] | None = None) -> Path:
    """The steward home: an explicit `--home`, else `<settings dir>/steward`."""
    if override is not None and str(override) != "":
        return Path(override).expanduser()
    return settings_dir(environ) / "steward"


def contexts_dir(home: Path) -> Path:
    return home / CONTEXTS_NAME


# --- repository identity ---------------------------------------------------


def git(repo: Path, *args: str) -> str:
    try:
        completed = subprocess.run(
            ["git", "-C", str(repo), *args],
            capture_output=True, text=True, encoding="utf-8", check=False,
        )
    except OSError as exc:
        raise StewardError("GIT_UNAVAILABLE", f"cannot run git: {exc}") from exc
    if completed.returncode != 0:
        detail = completed.stderr.strip() or f"exit {completed.returncode}"
        raise StewardError("NOT_A_REPOSITORY", f"git {' '.join(args)} failed in {repo}: {detail}")
    return completed.stdout


def repository_id(repo: Path) -> str:
    """The lexicographically smallest root commit id reachable from HEAD.

    A shallow clone is refused: its boundary commits look like roots, so the id
    it would yield differs from every complete clone of the same repository.
    """
    if git(repo, "rev-parse", "--is-shallow-repository").strip() == "true":
        raise StewardError("REPOSITORY_SHALLOW",
                           f"shallow clone has no stable repository identity; unshallow it first: {repo}")
    roots = sorted(line.strip() for line in git(repo, "rev-list", "--max-parents=0", "HEAD").splitlines()
                   if line.strip())
    if not roots:
        raise StewardError("NO_ROOT_COMMIT", f"repository has no root commit: {repo}")
    return roots[0]


def canonical(path: str | Path) -> str:
    """A comparable spelling of a local path; used only for schema-1 matching."""
    return os.path.normcase(os.path.normpath(str(Path(path).expanduser().resolve())))


class Repository:
    """The facts of one checkout that resolution needs."""

    def __init__(self, repo: Path) -> None:
        self.repository_id = repository_id(repo)
        self.worktree_root = git(repo, "rev-parse", "--show-toplevel").strip()
        common = git(repo, "rev-parse", "--path-format=absolute", "--git-common-dir").strip()
        self.git_common_dir = common
        common_path = Path(common)
        self.repository_root = str(common_path.parent if common_path.name == ".git" else common_path)
        try:
            self.revision = git(repo, "symbolic-ref", "-q", "HEAD").strip()
        except StewardError:
            self.revision = git(repo, "rev-parse", "HEAD").strip()

    def path_tuple(self) -> tuple[str, str, str, str]:
        return (canonical(self.repository_root), canonical(self.worktree_root),
                canonical(self.git_common_dir), self.revision)


# --- bindings --------------------------------------------------------------


class Binding:
    """One parsed `binding.toml` of schema 1 or 2."""

    def __init__(self, context_dir: Path, data: dict[str, Any]) -> None:
        self.context_dir = context_dir
        self.data = data
        self.schema = data.get("schema")
        if self.schema not in (1, 2):
            raise StewardError("BINDING_INVALID", f"{context_dir.name}: unsupported binding schema {self.schema!r}")
        self.context_id = data.get("context_id")
        if not isinstance(self.context_id, str) or self.context_id != context_dir.name:
            raise StewardError("BINDING_INVALID",
                               f"{context_dir.name}: context_id must equal the directory name")
        project = data.get("project_name", "")
        self.project_name = project if isinstance(project, str) else ""
        self.repository_id = ""
        self.default = False
        self.revisions: list[str] = []
        self.seen: list[dict[str, Any]] = []
        if self.schema == 2:
            value = data.get("repository_id")
            if not isinstance(value, str) or not REPOSITORY_ID_RE.match(value):
                raise StewardError("BINDING_INVALID", f"{context_dir.name}: repository_id must be a commit id")
            self.repository_id = value
            default = data.get("default", False)
            if not isinstance(default, bool):
                raise StewardError("BINDING_INVALID", f"{context_dir.name}: default must be a boolean")
            self.default = default
            revisions = data.get("revisions", [])
            if not isinstance(revisions, list) or not all(isinstance(item, str) for item in revisions):
                raise StewardError("BINDING_INVALID", f"{context_dir.name}: revisions must be a string list")
            self.revisions = list(revisions)
            seen = data.get("seen", [])
            if not isinstance(seen, list) or not all(isinstance(item, dict) for item in seen):
                raise StewardError("BINDING_INVALID", f"{context_dir.name}: seen must be an array of tables")
            self.seen = [dict(item) for item in seen]
        else:
            for key in SCHEMA1_TUPLE:
                if not isinstance(data.get(key), str) or not data.get(key):
                    raise StewardError("BINDING_INVALID", f"{context_dir.name}: schema-1 binding lacks {key}")

    def schema1_tuple(self) -> tuple[str, str, str, str]:
        return (canonical(self.data["repository_root"]), canonical(self.data["worktree_root"]),
                canonical(self.data["git_common_dir"]), self.data["revision_selector"])

    def serves(self, revision: str) -> bool:
        return any(revision_matches(selector, revision) for selector in self.revisions)

    def summary(self) -> dict[str, Any]:
        return {
            "context_id": self.context_id,
            "context_dir": str(self.context_dir),
            "project_name": self.project_name,
            "schema": self.schema,
            "default": self.default,
            "revisions": list(self.revisions),
        }


def revision_matches(selector: str, revision: str) -> bool:
    """`main` and `refs/heads/main` both select the branch `refs/heads/main`."""
    return selector == revision or f"refs/heads/{selector}" == revision


def read_binding(context_dir: Path) -> Binding:
    path = context_dir / BINDING_NAME
    try:
        raw = path.read_bytes()
    except OSError as exc:
        raise StewardError("BINDING_UNREADABLE", f"cannot read {path}: {exc}") from exc
    try:
        data = tomllib.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, tomllib.TOMLDecodeError) as exc:
        raise StewardError("BINDING_INVALID", f"{context_dir.name}: invalid UTF-8 TOML: {exc}") from exc
    return Binding(context_dir, data)


def scan_bindings(home: Path) -> tuple[list[Binding], list[dict[str, str]]]:
    """Every readable binding under `contexts/*/`, plus the ones that were not."""
    bindings: list[Binding] = []
    invalid: list[dict[str, str]] = []
    root = contexts_dir(home)
    if not root.is_dir():
        return bindings, invalid
    for context_dir in sorted(root.iterdir()):
        if not context_dir.is_dir() or not (context_dir / BINDING_NAME).is_file():
            continue
        try:
            bindings.append(read_binding(context_dir))
        except StewardError as exc:
            invalid.append({"context_dir": str(context_dir), "code": exc.code, "message": str(exc)})
    return bindings, invalid


# --- B2: context resolution ------------------------------------------------


class Resolution:
    """The typed outcome: `found`, `ambiguous` (with candidates) or `none`."""

    def __init__(self, status: str, *, context_dir: Path | None = None, context_id: str | None = None,
                 candidates: list[Binding] | None = None, rule: str | None = None,
                 invalid: list[dict[str, str]] | None = None) -> None:
        self.status = status
        self.context_dir = context_dir
        self.context_id = context_id
        self.candidates = list(candidates or [])
        self.rule = rule
        self.invalid = list(invalid or [])

    def as_json(self) -> dict[str, Any]:
        return {
            "status": self.status,
            "context_id": self.context_id,
            "context_dir": str(self.context_dir) if self.context_dir is not None else None,
            "rule": self.rule,
            "candidates": [candidate.summary() for candidate in self.candidates],
            "invalid": self.invalid,
        }


def found(binding_or_dir: Binding | Path, rule: str, invalid: list[dict[str, str]]) -> Resolution:
    if isinstance(binding_or_dir, Binding):
        return Resolution("found", context_dir=binding_or_dir.context_dir,
                          context_id=binding_or_dir.context_id, rule=rule, invalid=invalid)
    return Resolution("found", context_dir=binding_or_dir, context_id=binding_or_dir.name,
                      rule=rule, invalid=invalid)


def resolve(home: Path, repo: Path | None = None, *, context_id: str | None = None,
            repository: Repository | None = None) -> Resolution:
    """Resolve one context without creating, repairing or reading `[[seen]]`."""
    if context_id is not None:
        if not CONTEXT_ID_RE.match(context_id):
            raise StewardError("CONTEXT_ID_INVALID", f"not an opaque context id: {context_id!r}")
        context_dir = contexts_dir(home) / context_id
        if context_dir.is_dir():
            return found(context_dir, "explicit-id", [])
        return Resolution("none", rule="explicit-id")

    if repository is None:
        repository = Repository(repo if repo is not None else Path.cwd())
    bindings, invalid = scan_bindings(home)

    same_repository = [b for b in bindings if b.schema == 2 and b.repository_id == repository.repository_id]
    if len(same_repository) == 1:
        return found(same_repository[0], "repository", invalid)
    if same_repository:
        serving = [b for b in same_repository if b.serves(repository.revision)]
        if len(serving) == 1:
            return found(serving[0], "revision", invalid)
        if len(serving) > 1:
            return Resolution("ambiguous", candidates=serving, rule="revision", invalid=invalid)
        defaults = [b for b in same_repository if b.default]
        if len(defaults) == 1:
            return found(defaults[0], "default", invalid)
        return Resolution("ambiguous", candidates=same_repository, rule="default", invalid=invalid)

    current = repository.path_tuple()
    exact = [b for b in bindings if b.schema == 1 and b.schema1_tuple() == current]
    if len(exact) == 1:
        return found(exact[0], "schema-1-path", invalid)
    if exact:
        return Resolution("ambiguous", candidates=exact, rule="schema-1-path", invalid=invalid)
    return Resolution("none", rule="no-match", invalid=invalid)


# --- explicit binding writes (the `bind` command only) ---------------------


def toml_value(value: Any) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, str):
        escaped = value.replace("\\", "\\\\").replace('"', '\\"')
        escaped = "".join(ch if ch >= " " and ch != "\x7f" else f"\\u{ord(ch):04x}" for ch in escaped)
        return f'"{escaped}"'
    if isinstance(value, list):
        return "[" + ", ".join(toml_value(item) for item in value) + "]"
    raise StewardError("BINDING_INVALID", f"cannot serialise {type(value).__name__}")


def binding_text(data: dict[str, Any]) -> str:
    """Schema-2 binding bytes in a fixed key order."""
    lines = [f"{key} = {toml_value(data[key])}" for key in
             ("schema", "context_id", "project_name", "repository_id", "default", "revisions")]
    for entry in data.get("seen", []):
        lines.append("")
        lines.append("[[seen]]")
        lines.extend(f"{key} = {toml_value(value)}" for key, value in entry.items()
                     if isinstance(value, (str, bool, int)))
    text = "\n".join(lines) + "\n"
    tomllib.loads(text)
    return text


def atomic_write(path: Path, text: str) -> None:
    handle, temporary = tempfile.mkstemp(prefix=f".{path.name}.new-", dir=path.parent)
    temp_path = Path(temporary)
    try:
        with os.fdopen(handle, "w", encoding="utf-8", newline="\n") as stream:
            stream.write(text)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temp_path, path)
    finally:
        temp_path.unlink(missing_ok=True)


DEFAULT_TRUE_RE = re.compile(r"(?m)^(default\s*=\s*)true(\s*(?:#.*)?)$")


def release_default(binding: Binding) -> None:
    """Set `default = false` in another context's binding, touching nothing else."""
    path = binding.context_dir / BINDING_NAME
    text = path.read_text(encoding="utf-8")
    updated, count = DEFAULT_TRUE_RE.subn(r"\1false\2", text, count=1)
    if count != 1:
        raise StewardError("BINDING_INVALID", f"{binding.context_id}: cannot locate `default = true`")
    atomic_write(path, updated)
