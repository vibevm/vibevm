#!/usr/bin/env python3
"""A0.12 — mock of the documentation example runner.

One example = one directory holding `example.toml`, an optional fixture
tree, and the blessed `expect.out` / `expect.err`.  The runner copies the
fixture into a fresh sandbox, runs the declared command with an isolated
per-user home, captures stdout/stderr/exit code, normalises both streams
by the rules the fixture declares, and compares them with the blessed
text (exact equality after normalisation).  `--json` documents are also
validated against the JTD schema the fixture names.

Usage:
    python runner.py --examples <dir> --repo <vibevm checkout> [--bless]
                     [--only <id> ...]

Exit code 0 = every example matched; 1 = at least one did not.

This is a SPIKE.  It exists to measure what a real runner must handle,
not to be that runner.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shlex
import shutil
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path

# --------------------------------------------------------------- isolation

# The three per-user state roots, spelled exactly as the product reads
# them.  Names and semantics: crates/vibe-core/src/settings.rs:36,
# crates/vibe-test-support/src/isolate.rs:52 and :56; the load-time
# constructor that sets all three is isolate.rs:104-127.
SETTINGS_DIR_ENV = "VIBE_SETTINGS"
REGISTRY_CACHE_ENV = "VIBE_REGISTRY_CACHE"
SEARCH_CACHE_ENV = "VIBEVM_SEARCH_CACHE_DIR"
# Suppresses first-run seeding of a global registry.toml
# (crates/vibe-cli/src/main.rs:84).
NO_DEFAULT_REGISTRY_ENV = "VIBE_NO_DEFAULT_REGISTRY"


def isolated_env(home: Path, extra: dict[str, str]) -> dict[str, str]:
    """The child's environment: the real one, with the per-user state
    roots moved into `home` and every network path closed."""
    env = dict(os.environ)
    # Anything inherited that could reach the operator's real home goes
    # first, so a stale value can never survive into the child.
    for key in (SETTINGS_DIR_ENV, REGISTRY_CACHE_ENV, SEARCH_CACHE_ENV):
        env.pop(key, None)
    settings = home / "settings"
    registries = home / "registries"
    search = home / "search-cache"
    for path in (settings, registries, search):
        path.mkdir(parents=True, exist_ok=True)
    env[SETTINGS_DIR_ENV] = win(settings)
    env[REGISTRY_CACHE_ENV] = win(registries)
    env[SEARCH_CACHE_ENV] = win(search)
    # Presentation only, and the one deliberate exception to "isolation
    # never changes behaviour": a captured stream must not depend on
    # whether the harness had a terminal.  ANSI stripping backs it up.
    env["NO_COLOR"] = "1"
    # Behaviour-changing inherited values are REMOVED rather than set:
    # an example must succeed for the reason the reader can see in its
    # own command line, not because the harness pre-set a flag.
    # `VIBE_INVOKED_BY` additionally stamps an `invoked_by` member onto
    # every JSON envelope that no JTD schema declares.
    # `VIBETERM` / `VIBEFRAME` make `vibe tree` write a bare OSC icon
    # sequence to stdout (crates/vibe-cli/src/commands/tree/host.rs:39),
    # so an inherited value would put escape bytes in the capture.
    for key in ("VIBE_OFFLINE", "VIBE_UNATTENDED", "VIBE_INVOKED_BY",
                NO_DEFAULT_REGISTRY_ENV, "VIBETERM", "VIBEFRAME"):
        env.pop(key, None)
    env.update(extra)
    return env


def win(path: Path) -> str:
    """A native Windows spelling.  An MSYS-style `/c/...` value handed to
    a native binary is read as a path off the current drive root, which
    silently lands the state somewhere else."""
    return str(path).replace("/", os.sep)


# ------------------------------------------------------------ normalisation

ANSI = re.compile(r"\x1b\[[0-9;?]*[ -/]*[@-~]|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)")
PRODUCT_VERSION = re.compile(r"\bvibe (\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.\-]+)?)")


def path_variants(path: Path) -> list[str]:
    """Every spelling of `path` a captured stream can carry on Windows."""
    native = str(path).replace("/", "\\")
    drive, rest = (native[:2], native[2:]) if native[1:2] == ":" else ("", native)
    msys = ""
    if drive:
        msys = "/" + drive[0].lower() + rest.replace("\\", "/")
    out = [
        "\\\\?\\" + native,  # long-path prefix
        native,  # C:\a\b
        native.replace("\\", "/"),  # C:/a/b
    ]
    if msys:
        out.append(msys)  # /c/a/b
    # Longest first: `C:\a\b` must not be eaten by a shorter prefix.
    return sorted(out, key=len, reverse=True)


def substitute_paths(text: str, subs: list[tuple[Path, str]]) -> str:
    """Replace real directories with their placeholders, longest first.

    Case-insensitive: Windows hands back `C:\\Users` or `c:\\users`
    depending on who built the string.
    """
    pairs: list[tuple[str, str]] = []
    for path, token in subs:
        for variant in path_variants(path):
            pairs.append((variant, token))
    pairs.sort(key=lambda pair: len(pair[0]), reverse=True)
    for variant, token in pairs:
        text = re.sub(re.escape(variant), token, text, flags=re.IGNORECASE)
    return text


def sort_blocks(text: str, patterns: list[str]) -> str:
    """Sort every maximal run of consecutive lines matching a pattern.

    The escape hatch for output whose ORDER is not a promise.  Declared
    per fixture and per line-shape, never globally: sorting a whole
    stream would hide a real ordering regression.
    """
    if not patterns:
        return text
    compiled = [re.compile(p) for p in patterns]
    lines = text.split("\n")
    out: list[str] = []
    index = 0
    while index < len(lines):
        matched = next((c for c in compiled if c.search(lines[index])), None)
        if matched is None:
            out.append(lines[index])
            index += 1
            continue
        run = []
        while index < len(lines) and matched.search(lines[index]):
            run.append(lines[index])
            index += 1
        out.extend(sorted(run))
    return "\n".join(out)


def normalise(raw: bytes, rules: dict, subs: list[tuple[Path, str]]) -> str:
    """Apply the declared rules, in the one order that works."""
    # 1. bytes -> text.  Strict: a decode failure is a finding about the
    #    product's output, not something to paper over.
    text = raw.decode("utf-8")
    # 2. ANSI escapes.  NO_COLOR is already set, so this only catches
    #    sequences a command emits unconditionally.
    if rules.get("ansi", True):
        text = ANSI.sub("", text)
    # 3. line endings, before anything matches on line shape.
    if rules.get("crlf", True):
        text = text.replace("\r\n", "\n").replace("\r", "\n")
    # 4. real directories -> placeholders, BEFORE slashes are unified:
    #    the substitution has to see both spellings to catch both.
    if rules.get("paths", True):
        text = substitute_paths(text, subs)
    # 5. any backslash left is a path separator we did not anticipate.
    if rules.get("slashes", True):
        text = text.replace("\\", "/")
    # 6. the product's own version.
    if rules.get("product_version", True):
        text = PRODUCT_VERSION.sub("vibe <VERSION>", text)
    # 7. extra fixture-local rules.  MULTILINE by default: these streams
    #    are line-oriented, so `^`/`$` mean "a line", not "the stream".
    for rule in rules.get("replace", []):
        text = re.sub(rule["pattern"], rule["with"], text, flags=re.MULTILINE)
    # 8. order-insensitive blocks.
    text = sort_blocks(text, rules.get("sort_blocks", []))
    # 9. trailing whitespace and the final newline.
    if rules.get("trailing_space", True):
        text = "\n".join(line.rstrip() for line in text.split("\n"))
    return text.rstrip("\n") + "\n" if text.strip() else ""


# ------------------------------------------------------------ JTD validation

_TYPES = {
    "boolean": lambda v: isinstance(v, bool),
    "string": lambda v: isinstance(v, str),
    "timestamp": lambda v: isinstance(v, str),
    "float32": lambda v: isinstance(v, (int, float)) and not isinstance(v, bool),
    "float64": lambda v: isinstance(v, (int, float)) and not isinstance(v, bool),
    "int8": lambda v: _int_in(v, -128, 127),
    "uint8": lambda v: _int_in(v, 0, 255),
    "int16": lambda v: _int_in(v, -32768, 32767),
    "uint16": lambda v: _int_in(v, 0, 65535),
    "int32": lambda v: _int_in(v, -(2**31), 2**31 - 1),
    "uint32": lambda v: _int_in(v, 0, 2**32 - 1),
}


def _int_in(value, low: int, high: int) -> bool:
    return isinstance(value, int) and not isinstance(value, bool) and low <= value <= high


def jtd_validate(schema: dict, instance, root: dict | None = None, at: str = "") -> list[str]:
    """RFC 8927 (JSON Type Definition), all eight forms.

    Written out because the repository ships NO runtime JTD validator:
    `schemas/*.jtd.json` is codegen input only (crates/vibe-wire/src/lib.rs:1-20),
    and the generated Rust types are the contract as *serde* sees it —
    which is strictly weaker, since none of them denies unknown fields.
    """
    root = root if root is not None else schema
    errs: list[str] = []
    if instance is None and schema.get("nullable"):
        return errs

    if "ref" in schema:
        target = root.get("definitions", {}).get(schema["ref"])
        if target is None:
            return [f"{at or '/'}: unknown definition `{schema['ref']}`"]
        return jtd_validate(target, instance, root, at)

    if "type" in schema:
        check = _TYPES.get(schema["type"])
        if check is None:
            return [f"{at or '/'}: unknown type `{schema['type']}`"]
        if not check(instance):
            errs.append(f"{at or '/'}: expected {schema['type']}, got {_shape(instance)}")
        return errs

    if "enum" in schema:
        if instance not in schema["enum"]:
            errs.append(f"{at or '/'}: {instance!r} not in enum {schema['enum']}")
        return errs

    if "elements" in schema:
        if not isinstance(instance, list):
            return [f"{at or '/'}: expected array, got {_shape(instance)}"]
        for i, item in enumerate(instance):
            errs += jtd_validate(schema["elements"], item, root, f"{at}/{i}")
        return errs

    if "values" in schema:
        if not isinstance(instance, dict):
            return [f"{at or '/'}: expected object, got {_shape(instance)}"]
        for key, item in instance.items():
            errs += jtd_validate(schema["values"], item, root, f"{at}/{key}")
        return errs

    if "discriminator" in schema:
        tag = schema["discriminator"]
        if not isinstance(instance, dict):
            return [f"{at or '/'}: expected object, got {_shape(instance)}"]
        if tag not in instance:
            return [f"{at or '/'}: missing discriminator `{tag}`"]
        arm = schema.get("mapping", {}).get(instance[tag])
        if arm is None:
            return [f"{at or '/'}: `{tag}` = {instance[tag]!r} has no mapping arm"]
        rest = {k: v for k, v in instance.items() if k != tag}
        return jtd_validate(arm, rest, root, at)

    if "properties" in schema or "optionalProperties" in schema:
        if not isinstance(instance, dict):
            return [f"{at or '/'}: expected object, got {_shape(instance)}"]
        required = schema.get("properties", {})
        optional = schema.get("optionalProperties", {})
        for key, sub in required.items():
            if key not in instance:
                errs.append(f"{at or '/'}: missing required member `{key}`")
            else:
                errs += jtd_validate(sub, instance[key], root, f"{at}/{key}")
        for key, sub in optional.items():
            if key in instance:
                errs += jtd_validate(sub, instance[key], root, f"{at}/{key}")
        if not schema.get("additionalProperties", False):
            for key in instance:
                if key not in required and key not in optional:
                    errs.append(f"{at or '/'}: unexpected member `{key}`")
        return errs

    return errs  # empty form: anything goes


def _shape(value) -> str:
    if value is None:
        return "null"
    return {bool: "boolean", int: "number", float: "number", str: "string",
            list: "array", dict: "object"}.get(type(value), type(value).__name__)


def split_json_documents(text: str) -> list:
    """`vibe … --json` writes a STREAM of pretty-printed documents, not
    one.  `vibe install --json` emits three (`install:plan`,
    `install:closure-diff`, `install`); a reader that calls
    `json.loads` on the whole stream fails on the second."""
    decoder = json.JSONDecoder()
    docs = []
    index = 0
    while index < len(text):
        while index < len(text) and text[index] in " \t\r\n":
            index += 1
        if index >= len(text):
            break
        value, index = decoder.raw_decode(text, index)
        docs.append(value)
    return docs


# ------------------------------------------------------------------ running


class Result:
    def __init__(self, example_id: str):
        self.id = example_id
        self.failures: list[str] = []
        self.exit_code: int | None = None
        self.notes: list[str] = []

    @property
    def ok(self) -> bool:
        return not self.failures


def run_example(directory: Path, repo: Path, bless: bool) -> Result:
    spec = tomllib.loads((directory / "example.toml").read_text(encoding="utf-8"))
    example = spec["example"]
    rules = spec.get("normalize", {})
    result = Result(example["id"])

    with tempfile.TemporaryDirectory(prefix="vibe-example-") as raw_sandbox:
        sandbox = Path(raw_sandbox).resolve()
        home = sandbox / "home"
        cwd = sandbox / "work"
        tree = example.get("tree")
        if tree:
            shutil.copytree(directory / tree, cwd)
        else:
            cwd.mkdir(parents=True)

        binary = repo / "target" / "debug" / "vibe.exe"
        tokens = {
            "${VIBE}": win(binary),
            "${REGISTRY}": win(repo / "fixtures" / "registry"),
            "${CWD}": win(cwd),
        }
        # Split FIRST, expand after: a Windows path substituted before
        # `shlex.split` loses every backslash to POSIX escaping.
        argv = shlex.split(example["run"], posix=True)
        for index, token in enumerate(argv):
            for name, value in tokens.items():
                token = token.replace(name, value)
            argv[index] = token
        if argv[0] == "vibe":
            argv[0] = str(binary)

        env = isolated_env(home, {k: str(v) for k, v in example.get("env", {}).items()})
        completed = subprocess.run(
            argv, cwd=cwd, env=env, capture_output=True, timeout=example.get("timeout", 300)
        )
        result.exit_code = completed.returncode

        subs = [
            (sandbox, "<TMP>"),
            (repo, "<REPO>"),
            (Path(tempfile.gettempdir()).resolve(), "<TEMP>"),
            (Path.home().resolve(), "<HOME>"),
        ]
        streams = {
            "out": normalise(completed.stdout, rules, subs),
            "err": normalise(completed.stderr, rules, subs),
        }

        expected_exit = example.get("exit", 0)
        if completed.returncode != expected_exit:
            result.failures.append(
                f"exit code: expected {expected_exit}, got {completed.returncode}"
            )

        for stream, actual in streams.items():
            blessed_file = directory / f"expect.{stream}"
            if bless:
                if actual:
                    blessed_file.write_text(actual, encoding="utf-8", newline="\n")
                elif blessed_file.exists():
                    blessed_file.unlink()
                continue
            expected = (
                blessed_file.read_text(encoding="utf-8").replace("\r\n", "\n")
                if blessed_file.exists()
                else ""
            )
            if actual != expected:
                result.failures.append(
                    f"std{stream} differs\n" + unified(expected, actual, stream)
                )

        jtd = example.get("jtd")
        if jtd and not bless:
            result.notes += validate_json(streams["out"], jtd, repo, result)

    return result


def validate_json(text: str, mapping: dict, repo: Path, result: Result) -> list[str]:
    notes = []
    try:
        docs = split_json_documents(text)
    except ValueError as exc:
        result.failures.append(f"stdout is not a JSON document stream: {exc}")
        return notes
    notes.append(f"{len(docs)} JSON document(s) on stdout")
    for doc in docs:
        command = doc.get("command") if isinstance(doc, dict) else None
        schema_path = mapping.get(command or "")
        if schema_path is None:
            notes.append(f"  `{command}`: NO schema declared - unchecked")
            continue
        schema = json.loads((repo / schema_path).read_text(encoding="utf-8"))
        errors = jtd_validate(schema, doc)
        if errors:
            result.failures.append(
                f"`{command}` fails {schema_path}:\n    " + "\n    ".join(errors)
            )
        else:
            notes.append(f"  `{command}`: valid against {schema_path}")
    return notes


def unified(expected: str, actual: str, label: str) -> str:
    import difflib

    diff = difflib.unified_diff(
        expected.splitlines(), actual.splitlines(),
        fromfile=f"expect.{label}", tofile=f"actual.{label}", lineterm="",
    )
    return "\n".join("    " + line for line in diff)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--examples", required=True, type=Path)
    parser.add_argument("--repo", required=True, type=Path)
    parser.add_argument("--bless", action="store_true")
    parser.add_argument("--only", action="append", default=[])
    args = parser.parse_args()

    repo = args.repo.resolve()
    directories = sorted(p for p in args.examples.iterdir() if (p / "example.toml").exists())
    if args.only:
        directories = [d for d in directories if d.name in args.only]

    failed = 0
    for directory in directories:
        result = run_example(directory, repo, args.bless)
        verdict = "BLESSED" if args.bless else ("MATCH " if result.ok else "DIFFER")
        print(f"[{verdict}] {result.id}  exit={result.exit_code}")
        for note in result.notes:
            print(f"    {note}")
        for failure in result.failures:
            print(f"    {failure}")
        if not result.ok:
            failed += 1
    print(f"\n{len(directories) - failed}/{len(directories)} example(s) matched")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
