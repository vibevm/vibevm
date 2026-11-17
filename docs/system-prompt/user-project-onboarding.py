#!/usr/bin/env python3
"""One offline USER-PROJECT journey. Uses synthetic data and an argv recorder only.

Run with Python 3.11+, on Linux, against a freshly built vibe binary:
  python3 -I docs/system-prompt/user-project-onboarding.py /absolute/path/to/vibe /scratch/parent
Every run creates a new directory; it never uses the caller's settings or agents.
"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    exe = Path(sys.argv[1]).resolve(strict=True)
    base = Path(tempfile.mkdtemp(prefix="user-project-onboarding-", dir=sys.argv[2])).resolve()
    project = base / "project"
    settings = base / "settings"
    registry = base / "registry"
    home = base / "home"
    for folder in (project, settings, registry, home):
        folder.mkdir()
    (settings / "registry.toml").write_text("")
    # Deliberately do not inherit credentials, proxies, provider settings or launchers.
    env = {
        "PATH": os.environ.get("PATH", "/usr/bin:/bin"),
        "HOME": str(home),
        "XDG_CONFIG_HOME": str(home / "config"),
        "XDG_CACHE_HOME": str(home / "cache"),
        "TMPDIR": str(base),
        "LANG": "C.UTF-8",
        "VIBE_SETTINGS": str(settings),
        "VIBE_REGISTRY_CACHE": str(base / "cache"),
        "VIBEVM_SEARCH_CACHE_DIR": str(base / "search"),
        "VIBE_NO_DEFAULT_REGISTRY": "1",
    }
    rows = []
    checks = []
    binary_hash = sha(exe)
    report_path = base / "RESULTS.json"

    def report(outcome):
        report_path.write_text(json.dumps({
            "outcome": outcome,
            "binary": str(exe), "binary_sha256": binary_hash,
            "scenario_sha256": sha(Path(__file__)),
            "steps": rows, "checks": checks,
            "limits": ["synthetic local fixture registry; offline",
                       "argv-recording harness, no provider request or model compliance proof",
                       "Linux only; no graphical Electron smoke",
                       "not full automatic routing/source-policy parity"],
        }, indent=2) + "\n")

    def run(*args, cwd=project):
        result = subprocess.run([str(exe), *map(str, args)], cwd=cwd, env=env,
                                text=True, capture_output=True, timeout=120)
        rows.append({"argv": list(map(str, args)), "exit": result.returncode,
                     "stdout": result.stdout, "stderr": result.stderr})
        report("running")
        if result.returncode:
            raise RuntimeError(f"step {len(rows)} failed: {args}\n{result.stderr}")
        return result.stdout

    def checked(label, condition):
        if not condition:
            raise AssertionError(label)
        checks.append(label)
        report("running")

    def package(name, version, marker):
        root = registry / f"org.onboarding/{name}/v{version}"
        (root / "boot").mkdir(parents=True)
        (root / "vibe.toml").write_text(
            f"[package]\ngroup='org.onboarding'\nname='{name}'\n"
            f"version='{version}'\nkind='flow'\nformat='simple'\n"
            "[boot_snippet]\nsource='boot/instructions.md'\n")
        (root / "boot/instructions.md").write_text(f"# {name}\n\n{marker}\n")

    def add_registry(root):
        manifest = root / "vibe.toml"
        manifest.write_text(manifest.read_text() +
                            f"\n[[registry]]\nname='onboarding'\nurl='{registry.as_uri()}'\n")

    try:
        package("personal", "1.0.0", "PERSONAL_PACKAGE_V1")
        package("project", "1.0.0", "PROJECT_STATIC")
        package("override", "1.0.0", "OVERRIDE_PACKAGE")

        # Start from an ordinary, empty selected project and separate personal root.
        run("init", "--path", project)
        run("user", "init", "--json")
        run("user", "status", "--json")
        personal = settings / "user-project"
        checked("personal root initialized separately", personal.joinpath("vibe.toml").is_file())
        add_registry(personal)
        run("install", "-g", "--user-project", "org.onboarding/personal", "--offline", "--json")
        run("list", "-g", "--json")
        run("check", "-g", "--json")
        memory = personal / "vibevm/vibespecs/boot/preferences.md"
        memory.write_text("# Authorized synthetic preference and memory\n\n"
                          "PERSONAL_AUTHORED_ONE: prefer concise examples.\n"
                          "Keep project decisions in the shared repository, never credentials here.\n")
        run("install", "-g", "--user-project", "--offline", "--json")

        boot = project / "vibevm/vibespecs/boot"
        (boot / "SYSTEM-TEMPLATE.md").write_text("PROJECT_TEMPLATE\n")
        (boot / "project.md").write_text("PROJECT_LIVE\n")
        run("install", "org.onboarding/project", "--path", project, "--registry", registry,
            "--offline", "--json")
        prefix = (boot / "SYSTEM.md").read_bytes()
        lane = (boot / "AGENT-LANE.md").read_bytes()
        checked("shared boot excludes personal text", b"PERSONAL_" not in prefix + lane)

        recorder = base / "record-agent"
        argv_file = base / "argv.txt"
        recorder.write_text('#!/bin/sh\nprintf "%s\\n" "$@" > "$ONBOARDING_ARGV"\n')
        recorder.chmod(0o755)
        # Isolated fixture launcher only, never the user's harness configuration.
        (settings / "agents.toml").write_text(
            f"[launchers.onboarding]\nkind='claude'\ncommand=['{recorder}']\n")
        env["ONBOARDING_ARGV"] = str(argv_file)

        def launch(root=project):
            run("agent", "onboarding", cwd=root)
            argv = argv_file.read_text().splitlines()
            carrier = Path(argv[argv.index("--append-system-prompt-file") + 1])
            checked("actual launcher receives private AGENT carrier",
                    carrier.parent == root / "vibevm/vibespecs/boot/local")
            return carrier.read_text()

        first = launch()
        checked("project then personal package and authored memory delivered",
                max(first.index("PROJECT_STATIC"), first.index("PROJECT_LIVE")) <
                min(first.index("PERSONAL_PACKAGE_V1"), first.index("PERSONAL_AUTHORED_ONE"))
                and first.count("PERSONAL_AUTHORED_ONE") == 1)
        checked("launch carrier omits already loaded template", "PROJECT_TEMPLATE" not in first)
        memory.write_text(memory.read_text().replace("PERSONAL_AUTHORED_ONE", "PERSONAL_AUTHORED_TWO"))
        next_launch = launch()
        checked("authored edit visible next launch without rebuild",
                "PERSONAL_AUTHORED_ONE" not in next_launch and "PERSONAL_AUTHORED_TWO" in next_launch)

        created = json.loads(run("user", "override", "init", "--alias", "work",
                                 "--path", project, "--json"))
        override = Path(created["root"])
        run("install", "org.onboarding/override", "--path", override, "--registry", registry,
            "--offline", "--json")
        override_memory = override / "vibevm/vibespecs/boot/preferences.md"
        override_memory.write_text("OVERRIDE_AUTHORED: project-local personal preference.\n")
        run("install", "--path", override, "--registry", registry, "--offline", "--json")
        enabled = launch()
        checked("override comes after personal", enabled.index("PERSONAL_AUTHORED_TWO") <
                enabled.index("OVERRIDE_PACKAGE") and "OVERRIDE_AUTHORED" in enabled)
        for state in ("disable", "enable"):
            run("user", "override", state, "--path", project, "--json")
            text = launch()
            checked(f"{state} affects next launch, preserves authored file",
                    ("OVERRIDE_AUTHORED" in text) == (state == "enable") and override_memory.is_file())

        # A distinct non-Git checkout is not inferred; explicit bind reuses the alias.
        second = base / "second-project"
        second.mkdir()
        run("init", "--path", second)
        bound = json.loads(run("user", "override", "bind", "--alias", "work",
                               "--path", second, "--json"))
        checked("explicit bind reuses root and enabled state",
                Path(bound["root"]) == override and bound["enabled"])

        run("tree", "--layers", "--plain", "--path", project)
        tree = json.loads(run("tree", "--layers", "--path", project, "--json"))
        checked("layer tree exposes three separate descriptors", len(tree["layers"]) == 3)
        composed = json.loads(run("prompt", "compose", "--path", project,
                                  "--session-target", "os=linux", "--json"))
        data = Path(composed["path"]).read_bytes()
        checked("composed SYSTEM preserves exact shared prefix", data.startswith(prefix))
        checked("composed layers ordered", data.index(b"PROJECT_LIVE") <
                data.index(b"PERSONAL_AUTHORED_TWO") < data.index(b"OVERRIDE_AUTHORED"))

        package("personal", "1.1.0", "PERSONAL_PACKAGE_V2")
        run("update", "-g", "--user-project", "--offline", "--json")
        updated = launch()
        checked("ordinary update changes next launch package",
                "PERSONAL_PACKAGE_V2" in updated and "PERSONAL_PACKAGE_V1" not in updated)
        run("uninstall", "-g", "--user-project", "org.onboarding/personal", "--json")
        removed = launch()
        checked("uninstall removes package, not authored memory or override",
                "PERSONAL_PACKAGE_V2" not in removed and "PERSONAL_AUTHORED_TWO" in removed
                and "OVERRIDE_AUTHORED" in removed)
        checked("shared artifacts unchanged across personal lifecycle",
                (boot / "SYSTEM.md").read_bytes() == prefix and
                (boot / "AGENT-LANE.md").read_bytes() == lane)
        checked("local artifacts carry in-directory ignore", (boot / "local/.gitignore").read_text() == "*\n")
        checked("binary unchanged through scenario", sha(exe) == binary_hash)
        report("pass")
    except Exception:
        report("fail")
        print(report_path, flush=True)
        raise
    print(report_path)


if __name__ == "__main__":
    main()
