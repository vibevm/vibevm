//! Public trust-question and refusal contracts over a local fixture registry.
mod common;

use std::{fs, path::Path, process::Command};

use common::{UserScratch, hook_slot::Fixture};
use serde_json::Value;

const PACKAGE: &str = "org.example/hooked@=0.1.0";
const RETRY: &str = "vibe install org.example/hooked@=0.1.0 --assume-yes";
const SPEC: &str = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust";

/// Pin every subprocess, including init, to this fixture's home and settings.
fn command(user: &UserScratch, home: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_vibe"));
    cmd.env("HOME", home)
        .env("USERPROFILE", home)
        .env("VIBE_SETTINGS", &user.settings)
        .env("VIBE_REGISTRY_CACHE", &user.cache)
        .env("VIBEVM_SEARCH_CACHE_DIR", &user.search_cache)
        .env("VIBE_NO_DEFAULT_REGISTRY", "1");
    for key in [
        "CI",
        "VIBE_INVOKED_BY",
        "VIBE_UNATTENDED",
        "VIBE_AGENT_COMMAND",
        "VIBE_AGENT_ARGS",
        "VIBEVM_INSTALL_ROOT",
        "NO_COLOR",
        "CLICOLOR",
    ] {
        cmd.env_remove(key);
    }
    cmd
}

/// Reuse JudgmentTrust without setup's implicit init environment.
fn fixture() -> (
    tempfile::TempDir,
    UserScratch,
    std::path::PathBuf,
    std::path::PathBuf,
) {
    let root = tempfile::tempdir().unwrap();
    let user = UserScratch::new();
    let project = root.path().join("project");
    fs::create_dir(&project).unwrap();
    let registry = common::hook_slot::registry(root.path(), Fixture::JudgmentTrust);
    assert!(
        command(&user, root.path())
            .args(["init", "--path"])
            .arg(&project)
            .output()
            .unwrap()
            .status
            .success()
    );
    (root, user, project, registry)
}

#[cfg(target_os = "linux")]
#[test]
fn group_untrusted_human_and_json_replay() {
    for json in [false, true] {
        let (root, user, project, registry) = fixture();
        let text = if json {
            let refused = command(&user, root.path())
                .args(["--json", "install", PACKAGE, "--registry"])
                .arg(&registry)
                .arg("--path")
                .arg(&project)
                .output()
                .unwrap();
            assert_eq!(refused.status.code(), Some(1));
            let envelope: Value = serde_json::from_slice(&refused.stderr).unwrap();
            assert_eq!(envelope["ok"], false);
            envelope["error"].as_str().unwrap().to_owned()
        } else {
            let proof = pty(&user, root.path(), &project, &registry, "decline");
            assert_eq!(proof["status"], 1);
            proof["final_screen"].as_str().unwrap().to_owned()
        };
        let words = text.split_whitespace().collect::<Vec<_>>().join(" ");
        for expected in [
            "group_untrusted",
            "org.example/hooked@0.1.0 may not run its installation contributions",
            "group org.example is not trusted and",
            "nothing was run (judged)",
            "trusted_groups",
            "untrusted_groups",
            SPEC,
            RETRY,
        ] {
            assert!(words.contains(expected), "missing {expected}: {words}");
        }
        assert!(words.contains(if json {
            "this run has nobody to ask"
        } else {
            "the contributions were declined"
        }));
        let agents = user.settings.join("agents.toml");
        assert!(
            words.contains(agents.to_str().unwrap()),
            "exact settings remedy: {words}"
        );
        let slot = project.join(common::slot_dir("org.example.hooked", "0.1.0"));
        assert!(
            !slot.exists(),
            "refusal must not populate or run a contribution"
        );
        // Extract only the displayed command. Pass its argv directly: no shell/eval.
        let start = words.find("vibe install ").unwrap();
        let end = start + words[start..].find("--assume-yes").unwrap() + "--assume-yes".len();
        let argv: Vec<_> = words[start..end].split_whitespace().collect();
        assert_eq!(argv, ["vibe", "install", PACKAGE, "--assume-yes"]);
        let replay = command(&user, root.path())
            .args(&argv[1..])
            .arg("--registry")
            .arg(&registry)
            .arg("--path")
            .arg(&project)
            .output()
            .unwrap();
        assert!(
            replay.status.success(),
            "{}",
            String::from_utf8_lossy(&replay.stderr)
        );
        assert_eq!(
            fs::read_to_string(slot.join("hook-order.txt"))
                .unwrap()
                .trim(),
            "post"
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
fn trust_question_suspends_progress() {
    // The real install owns a live task. The child contribution waits long
    // enough to observe its resumed spinner, not just the final transcript.
    for answer in ["allow", "decline", "escape"] {
        let (root, user, project, registry) = fixture();
        fs::write(
            registry.join("org.example/hooked/v0.1.0/hooks/post.sh"),
            "set -eu\nsleep 0.8\nprintf 'post\\n' >> hook-order.txt\n",
        )
        .unwrap();
        let proof = pty(&user, root.path(), &project, &registry, answer);
        assert!(proof["pre_frames"].as_u64().unwrap() > 0, "{proof}");
        assert_eq!(proof["waiting_frames"], 0, "{proof}");
        assert!(proof["wait_seconds"].as_f64().unwrap() >= 0.72, "{proof}");
        let screen = proof["question_screen"].as_str().unwrap();
        for row in [
            "? trust org.example  org.example/hooked@0.1.0 runs 1 installation contribution",
            "  judged  (script)  Judged contribution",
            "  ! group org.example is not trusted",
            "? Run this contribution?",
            "  yes",
            "❯ no",
        ] {
            assert!(
                screen.lines().any(|line| line.starts_with(row)),
                "missing {row}: {screen}"
            );
        }
        let final_screen = proof["final_screen"].as_str().unwrap();
        let slot = project.join(common::slot_dir("org.example.hooked", "0.1.0"));
        if answer == "allow" {
            assert_eq!(proof["status"], 0, "{proof}");
            assert!(proof["post_frames"].as_u64().unwrap() > 0, "{proof}");
            assert!(
                proof["decision_screen"]
                    .as_str()
                    .unwrap()
                    .contains("trust org.example  allowed for this run"),
                "{proof}"
            );
            assert!(
                final_screen.contains("materialised 1 package"),
                "completion positive control: {proof}"
            );
            assert_eq!(
                fs::read_to_string(slot.join("hook-order.txt"))
                    .unwrap()
                    .trim(),
                "post"
            );
        } else {
            assert_eq!(proof["status"], 1, "{proof}");
            assert!(!slot.exists(), "decline/interrupt is fail closed");
            if answer == "decline" {
                assert!(
                    final_screen.contains("trust org.example  declined"),
                    "{proof}"
                );
            } else {
                assert!(final_screen.contains("question failed"), "{proof}");
            }
        }
        eprintln!(
            "PTY {answer}: pre={} quiet={} duration={}s post={} status={}",
            proof["pre_frames"],
            proof["waiting_frames"],
            proof["wait_seconds"],
            proof["post_frames"],
            proof["status"]
        );
    }
}

/// Run the finite PTY driver with exactly the fixture command's environment.
#[cfg(target_os = "linux")]
fn pty(user: &UserScratch, home: &Path, project: &Path, registry: &Path, answer: &str) -> Value {
    let env = command(user, home);
    let mut python = Command::new("python3");
    python
        .args(["-I", "-c", PTY, env!("CARGO_BIN_EXE_vibe")])
        .arg(project)
        .arg(registry)
        .arg(answer);
    for (key, value) in env.get_envs() {
        if let Some(value) = value {
            python.env(key, value);
        } else {
            python.env_remove(key);
        }
    }
    let output = python
        .env("TERM", "xterm")
        .env("LANG", "C.UTF-8")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

/// A small screen model applies cursor movement and erasure before inspecting
/// visible rows. Raw redraws and echoed input are never counted as list rows.
#[cfg(target_os = "linux")]
const PTY: &str = r#"
import errno, fcntl, json, os, pty, re, select, signal, subprocess, sys, termios, time
cli, project, registry, answer = sys.argv[1:]
master, slave = pty.openpty()
fcntl.ioctl(slave, termios.TIOCSWINSZ, __import__('struct').pack('HHHH', 48, 200, 0, 0))
def terminal():
    os.setsid()
    fcntl.ioctl(0, termios.TIOCSCTTY, 0)
p = subprocess.Popen([cli, 'install', 'org.example/hooked@=0.1.0', '--registry', registry, '--path', project], stdin=slave, stdout=slave, stderr=slave, preexec_fn=terminal)
os.close(slave)
seen = bytearray()
started = time.monotonic()
decision = ''
answered = False
def read_for(seconds):
    until = time.monotonic() + seconds
    result = bytearray()
    while time.monotonic() < until:
        if time.monotonic() - started > 20:
            raise RuntimeError('PTY deadline: ' + seen.decode('utf-8', 'replace'))
        if select.select([master], [], [], min(.025, max(0, until-time.monotonic())))[0]:
            try: chunk = os.read(master, 65536)
            except OSError as e:
                if e.errno == errno.EIO: break
                raise
            if not chunk: break
            seen.extend(chunk)
            result.extend(chunk)
            # Apply every screen event inside this chunk before another redraw
            # can erase the collapsed answer; also used by the final drain.
            if answered: screen(seen)
    return bytes(result)
def frames(data):
    text = re.sub(r'\x1b\[[0-?]*[ -/]*[@-~]', '', data.decode('utf-8', 'replace'))
    return len(re.findall(r'[⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏●] [^\r\n]* · ', text))
def screen(data):
    global decision
    text = data.decode('utf-8', 'replace')
    rows = [[]]; y = x = i = 0
    while i < len(text):
        c = text[i]
        if c == '\x1b':
            m = re.match(r'\x1b\[([0-?]*)([ -/]*)([@-~])', text[i:])
            if m:
                arg, _, op = m.groups(); n = int(arg) if arg.isdigit() else 1
                if op == 'A': y = max(0, y-n)
                elif op == 'B': y += n
                elif op == 'C': x += n
                elif op == 'D': x = max(0, x-n)
                elif op == 'G': x = n-1
                elif op == 'K':
                    while len(rows) <= y: rows.append([])
                    if arg == '2': rows[y] = []
                    elif arg == '1': rows[y][:x+1] = [' '] * min(x+1, len(rows[y]))
                    else: del rows[y][x:]
                elif op == 'J' and arg in ('', '0'):
                    while len(rows) <= y: rows.append([])
                    del rows[y][x:]; del rows[y+1:]
                i += len(m.group()); continue
            i += 1; continue
        if c == '\r': x = 0
        elif c == '\n': y += 1
        elif c == '\b': x = max(0, x-1)
        elif c >= ' ':
            while len(rows) <= y: rows.append([])
            while len(rows[y]) <= x: rows[y].append(' ')
            rows[y][x] = c; x += 1
            if answered and c in ('n', 'd'):
                row = ''.join(rows[y]).rstrip()
                if row in ('✓ trust org.example  allowed for this run', '✓ trust org.example  declined'):
                    decision = '\n'.join(''.join(row).rstrip() for row in rows)
        i += 1
    return '\n'.join(''.join(row).rstrip() for row in rows)
try:
    while b'Install 1 package into ' not in seen or '❯' not in seen.decode('utf-8', 'replace'):
        read_for(.025)
        if p.poll() is not None: raise RuntimeError('exited before apply question')
    os.write(master, b'\r')
    while b'Run this contribution?' not in seen or '❯ no' not in screen(seen):
        read_for(.025)
        if p.poll() is not None: raise RuntimeError('exited before question: '+seen.decode('utf-8','replace'))
    # Snapshot before withholding input: no answer bytes have been sent.
    before = bytes(seen)
    question = screen(before)
    wait_start = time.monotonic()
    waiting = read_for(.78)
    elapsed = time.monotonic() - wait_start
    assert p.poll() is None, 'question did not wait for input'
    assert frames(waiting) == 0, 'progress drew while input withheld: '+repr(waiting)
    answered = True
    os.write(master, {'allow': b'\x1b[A\r', 'decline': b'\r', 'escape': b'\x1b'}[answer])
    after = bytearray()
    while p.poll() is None:
        after.extend(read_for(.005))
    after.extend(read_for(.05))
    screen(seen)
    if answer != 'escape': assert decision, 'no visible collapsed answer was captured'
    print(json.dumps(dict(pre_frames=frames(before), waiting_frames=frames(waiting), wait_seconds=elapsed, post_frames=frames(after), question_screen=question, decision_screen=decision, final_screen=screen(seen), status=p.returncode)))
finally:
    if p.poll() is None:
        os.killpg(p.pid, signal.SIGKILL); p.wait()
    os.close(master)
"#;
