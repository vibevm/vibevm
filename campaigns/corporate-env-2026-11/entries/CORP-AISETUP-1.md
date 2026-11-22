# CORP-AISETUP-1

Planned at commit `9d8428d07`. Line numbers are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** Attended setup offers to install selected agent packages and reports each agent's outcome

**Write.** `crates/vibe-cli/src/commands/ai/setup.rs`, `crates/vibe-cli/src/commands/ai/setup/install.rs`, `crates/vibe-cli/tests/cli_ai_setup_attended.rs`, `crates/vibe-cli/tests/common/ai-setup-pty.py`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --test cli_ai_setup_attended attended_setup_installs_selected_agent_packages_with_independent_outcomes`

**Test.** crates/vibe-cli/tests/cli_ai_setup_attended.rs::attended_setup_installs_selected_agent_packages_with_independent_outcomes. In a PTY, bare `vibe ai setup` with a user localdir registry that holds only org.vibevm.agents/pi. The user answers y, selects pi and qwen, and accepts the child install's Platform and Architecture prompts. The transcript shows 'Agent package pi: installed' and 'Agent package qwen: failed', and a fresh `vibe app list -g --json` lists key org.vibevm.agents/pi@1.0.0.

**Read first.**
- crates/vibe-cli/src/commands/ai/setup.rs
- crates/vibe-cli/src/commands/app/global.rs (settings_root, installed_keys)
- crates/vibe-cli/src/output/prompts/ask.rs (confirm, named, choose_many)
- crates/vibe-cli/tests/common/ordinary-preview-pty.py
- crates/vibe-cli/tests/cli_ai_connections.rs (struct F, for the 0700 settings pattern)

**Steps.**
1. Precondition: tools/cargo-test-ran.sh must exist in the checkout (the lead commits it before this entry). If it is missing, stop and report a packet defect; do not write it yourself.
2. In crates/vibe-cli/src/commands/ai/setup.rs add `mod install;` next to `mod native_settings;`. In `run`, replace the existing `if attended && agents.is_empty() && from_agent_alias.is_none() && mode.is_none() && connection.is_none() && response.is_none() && crate::output::prompts::confirm("Write a connection into selected agents' native settings? Otherwise continue metadata only", false)? { return native_settings::attended(ctx, store, cancel); }` with:
```rust
    let bare = attended
        && agents.is_empty()
        && from_agent_alias.is_none()
        && mode.is_none()
        && connection.is_none()
        && response.is_none();
    if bare {
        install::offer(cancel)?;
        if crate::output::prompts::confirm(
            "Write a connection into selected agents' native settings? Otherwise continue metadata only",
            false,
        )? {
            return native_settings::attended(ctx, store, cancel);
        }
    }
```
3. Create crates/vibe-cli/src/commands/ai/setup/install.rs with exactly:
```rust
//! Optional agent package installation in attended `vibe ai setup`; each
//! selected package goes through the ordinary `vibe install -g` in a child.
use anyhow::Result;
use vibe_core::cancellation::CancellationToken;

/// Agents offered for installation and their `org.vibevm.agents` packages.
const AGENTS: [(&str, &str); 6] = [
    ("pi", "pi"),
    ("qwen", "qwen"),
    ("codex", "codex"),
    ("opencode", "opencode"),
    ("hermes", "hermes"),
    ("claude", "claude-code"),
];

/// Ask (default No), then install each selected package that is not yet
/// installed. One agent's outcome never stops the others.
pub(super) fn offer(cancel: &CancellationToken) -> Result<()> {
    if !crate::output::prompts::confirm("Install AI agent packages now?", false)? {
        return Ok(());
    }
    let names: Vec<&str> = AGENTS.iter().map(|(name, _)| *name).collect();
    let choices = crate::output::prompts::named(&names);
    let Some(selected) = crate::output::prompts::choose_many(
        "Select agent packages to install",
        &choices,
        &[false; 6],
    )?
    else {
        return Ok(());
    };
    let settings = crate::commands::app::global::settings_root()?;
    let exe = std::env::current_exe()?;
    for index in selected {
        let (agent, package) = AGENTS[index];
        let package = format!("org.vibevm.agents/{package}");
        let prefix = format!("{package}@");
        let present = || {
            crate::commands::app::global::installed_keys(&settings)
                .iter()
                .any(|key| key.starts_with(&prefix))
        };
        let outcome = if present() {
            "already-installed"
        } else {
            cancel.checkpoint()?;
            let ran = std::process::Command::new(&exe)
                .args(["install", "-g", package.as_str()])
                .status()
                .is_ok_and(|status| status.success());
            match (ran, present()) {
                (true, true) => "installed",
                (true, false) => "not-installed",
                (false, _) => "failed",
            }
        };
        println!("Agent package {agent}: {outcome}");
    }
    Ok(())
}
```
4. Create crates/vibe-cli/tests/common/ai-setup-pty.py with exactly:
```python
"""Staged PTY driver for attended `vibe ai setup`; synthetic inputs only.

argv: <vibe> <json list of [expected, answer]>. Each answer is typed once
its expected text appears after the previous answer. Prints one JSON object.
"""
import json
import os
import pty
import select
import signal
import subprocess
import sys
import termios
import time

cli = sys.argv[1]
steps = json.loads(sys.argv[2])
master, slave = pty.openpty()
original = termios.tcgetattr(slave)


def child_terminal():
    os.setsid()
    import fcntl
    fcntl.ioctl(0, termios.TIOCSCTTY, 0)


process = subprocess.Popen([cli, 'ai', 'setup'], stdin=slave, stdout=slave, stderr=slave, preexec_fn=child_terminal, close_fds=True)
seen = bytearray()
mark = 0
sent = 0
started = time.monotonic()
while process.poll() is None:
    if time.monotonic() - started > 90:
        os.killpg(process.pid, signal.SIGKILL)
        process.wait()
        break
    readable, _, _ = select.select([master], [], [], 0.1)
    if not readable:
        continue
    try:
        data = os.read(master, 16384)
    except OSError:
        break
    seen.extend(data)
    if sent < len(steps) and steps[sent][0].encode() in seen[mark:]:
        time.sleep(0.3)
        os.write(master, steps[sent][1].encode())
        mark = len(seen)
        sent += 1
process.wait(timeout=5)
while select.select([master], [], [], 0)[0]:
    try:
        data = os.read(master, 16384)
    except OSError:
        break
    if not data:
        break
    seen.extend(data)
restored = termios.tcgetattr(slave)
os.close(master)
os.close(slave)
print(json.dumps({'status': process.returncode, 'sent': sent, 'terminal_restored': restored == original, 'transcript': seen.decode('utf-8', 'replace')}))
```
5. Create crates/vibe-cli/tests/cli_ai_setup_attended.rs. It starts with `//! Attended `vibe ai setup` under a pseudo-terminal, synthetic inputs only.` and `#![cfg(target_os = "linux")]`, and contains this helper:
```rust
use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Stdio},
};

struct World {
    root: tempfile::TempDir,
}

impl World {
    fn new() -> Self {
        let root = tempfile::Builder::new()
            .permissions(fs::Permissions::from_mode(0o700))
            .tempdir()
            .unwrap();
        for dir in ["settings", "home"] {
            fs::create_dir(root.path().join(dir)).unwrap();
            fs::set_permissions(root.path().join(dir), fs::Permissions::from_mode(0o700)).unwrap();
        }
        Self { root }
    }
    fn path(&self, tail: &str) -> PathBuf {
        self.root.path().join(tail)
    }
    fn command(&self, program: &str) -> Command {
        let mut command = Command::new(program);
        command
            .env_clear()
            .env("HOME", self.path("home"))
            .env("VIBE_SETTINGS", self.path("settings"))
            .env("VIBE_NO_DEFAULT_REGISTRY", "1")
            .env("PATH", "/usr/bin:/bin")
            .env("TERM", "xterm");
        command
    }
    fn ok(&self, args: &[&str]) {
        let output = self
            .command(env!("CARGO_BIN_EXE_vibe"))
            .args(args)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    }
    fn json(&self, args: &[&str], stdin: Option<&[u8]>) -> serde_json::Value {
        let mut child = self
            .command(env!("CARGO_BIN_EXE_vibe"))
            .args(args)
            .arg("--json")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        if let Some(bytes) = stdin {
            input.write_all(bytes).unwrap();
        }
        drop(input);
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        serde_json::from_slice(&output.stdout).unwrap()
    }
    fn setup(&self, steps: &[(&str, &str)]) -> serde_json::Value {
        let driver = self.path("ai-setup-pty.py");
        fs::write(&driver, include_str!("common/ai-setup-pty.py")).unwrap();
        let output = self
            .command("python3")
            .arg("-I")
            .arg(&driver)
            .arg(env!("CARGO_BIN_EXE_vibe"))
            .arg(serde_json::to_string(steps).unwrap())
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        serde_json::from_slice(&output.stdout).unwrap()
    }
}
```
6. In the same test file, add the fixture and the test:
```rust
fn agent_registry(w: &World) {
    let package = w.path("registry/org.vibevm.agents/pi/v1.0.0");
    fs::create_dir_all(package.join("payload/bin")).unwrap();
    fs::write(
        package.join("vibe.toml"),
        format!(
            "[package]\nname = \"pi\"\ngroup = \"org.vibevm.agents\"\nkind = \"app\"\nversion = \"1.0.0\"\n\n\
             [application]\nid = \"pi\"\n\n\
             [[installer_options]]\nid = \"main\"\nname = \"Main\"\nplatform = \"linux\"\nplatform_name = \"Linux\"\narch = \"{}\"\n\n\
             [[installer_options.steps]]\nid = \"place\"\ntitle = \"Place pi\"\nkind = \"copy\"\nfrom = \"payload\"\n\n\
             [[installer_options.launchers]]\nid = \"main\"\nname = \"pi\"\nexecutable = \"$app_bin/pi\"\nkind = \"command\"\n\n\
             [[installer_options.public_commands]]\nname = \"pi\"\nlauncher = \"main\"\n",
            std::env::consts::ARCH
        ),
    )
    .unwrap();
    let pi = package.join("payload/bin/pi");
    fs::write(&pi, "#!/bin/sh\necho PI_RAN \"$@\"\n").unwrap();
    fs::set_permissions(&pi, fs::Permissions::from_mode(0o755)).unwrap();
    let url = format!("file://{}", w.path("registry").display());
    w.ok(&["registry", "add", "agents", &url, "--type", "localdir", "--scope", "user"]);
}

#[test]
fn attended_setup_installs_selected_agent_packages_with_independent_outcomes() {
    let w = World::new();
    agent_registry(&w);
    let report = w.setup(&[
        ("Install AI agent packages now?", "y"),
        ("Select agent packages to install", " \u{1b}[B \r"),
        ("Platform", "\r"),
        ("Architecture", "\r"),
        ("native settings", "n"),
        ("Configure existing connection", "\u{1b}"),
    ]);
    let transcript = report["transcript"].as_str().unwrap();
    assert_eq!(report["status"], 0, "{transcript}");
    assert_eq!(report["sent"], 6, "{transcript}");
    assert!(transcript.contains("Agent package pi: installed"), "{transcript}");
    assert!(transcript.contains("Agent package qwen: failed"), "{transcript}");
    let list = w.json(&["app", "list", "-g"], None);
    assert!(list["installations"].as_array().unwrap().iter().any(|row| row["key"] == "org.vibevm.agents/pi@1.0.0"));
}
```
7. Run `cargo fmt --all`. Run `cargo clippy -p vibe-cli --all-targets -- -D warnings` and fix every warning without `allow`. Run `cargo xtask conform check`. Check that `wc -l` stays at 600 lines or fewer for each file you touched. Then run the check.

**Decisions already made.**
- The install step runs only in the bare attended branch: no --agent, --from-agent-alias, --mode, --connection or --apply-preview, and both stdin and stderr are terminals. Every JSON, unattended or flagged run is unchanged.
- The step is the first question of bare setup. It comes before the native-settings question and before any later connection onboarding.
- Each selected package is installed by a child `<current vibe> install -g org.vibevm.agents/<package>` that inherits the terminal, so the install keeps its own prompts and npm/runtime handling. Package names: pi, qwen, codex, opencode, hermes, claude-code (for agent claude).
- Outcome words, printed as `Agent package <agent>: <outcome>`: already-installed (a key `org.vibevm.agents/<package>@...` is in installed_keys before the child), installed (the child succeeded and the key is present), not-installed (the child succeeded but recorded no installation), failed (the child failed to run or exited nonzero). Setup always continues.
- Claude Code is installable like any other agent; selecting it here has nothing to do with the OpenAI warning.
- The test's attended install child asks Platform and Architecture even for a single option. This was measured on the current binary; Enter accepts the detected values.
- Prompt matching skips the styled `?` prefix, because the raw bytes put ANSI codes between `?` and the prompt text.
- The metadata menu is matched by 'Configure existing connection', not 'metadata only'. After the native-settings confirm is answered, dialoguer reprints that prompt collapsed, and the reprint also contains 'metadata only', so that matcher would fire early on a timer race.
- Hidden prerequisite, list-level: tools/cargo-test-ran.sh is untracked today. The lead commits it unchanged before this entry; every entry's check depends on it. python3 and the tempfile/serde_json dev-dependencies are already available.

**Not in this entry.** Not in this entry: connection creation or reuse prompts, changing the existing metadata or native branches, an in-process install API, a separate npm-defer outcome, or committing tools/cargo-test-ran.sh (the lead does that).

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
