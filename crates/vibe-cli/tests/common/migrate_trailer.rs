//! Isolated commands and a bounded coloured PTY for the migration fixture.

use super::common::app_registry::AppRegistry;
use std::process::{Command, Output};

/// Every fixture child has explicit temporary homes and no inherited account state.
fn command(r: &AppRegistry, program: &str) -> Command {
    let mut command = Command::new(program);
    command
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", r.home())
        .env("USERPROFILE", r.home())
        .env("VIBE_SETTINGS", r.settings())
        .env("VIBE_REGISTRY_CACHE", &r.user.cache)
        .env("VIBEVM_SEARCH_CACHE_DIR", &r.user.search_cache)
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .env("TERM", "xterm-256color")
        .env("COLUMNS", "200")
        .current_dir(r.root.path());
    command
}

pub fn vibe(r: &AppRegistry, args: &[&str]) -> Output {
    command(r, env!("CARGO_BIN_EXE_vibe"))
        .args(args)
        .output()
        .expect("isolated fixture child")
}

pub fn app(r: &AppRegistry, verb: &[&str], package: &str, extra: &[&str]) -> Output {
    let registry = r.registry();
    let mut args = verb.to_vec();
    args.extend_from_slice(&[package, "--registry", registry.to_str().unwrap()]);
    args.extend_from_slice(extra);
    vibe(r, &args)
}

pub fn listed(r: &AppRegistry, key: &str) -> serde_json::Value {
    let list: serde_json::Value = serde_json::from_slice(
        &vibe(r, &["app", "list", "-g", "--json"]).stdout,
    ).expect("fixture application list");
    list["installations"].as_array().unwrap().iter()
        .find(|row| row["key"] == key).cloned().unwrap_or(serde_json::Value::Null)
}

/// Resume the real mixed result and inspect the last terminal line, including ANSI roles.
pub fn assert_mixed_trailer(r: &AppRegistry, work: &std::path::Path) {
    let script = r#"
import errno, fcntl, json, os, pty, select, signal, struct, subprocess, sys, termios, time, tty
pairs = [pty.openpty(), pty.openpty()]
for master, slave in pairs:
    tty.setraw(slave)
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 40, 200, 0, 0))
child = None
streams = {master: bytearray() for master, slave in pairs}
active = list(streams)
deadline = time.monotonic() + 30
try:
    child = subprocess.Popen(sys.argv[1:], stdin=subprocess.DEVNULL,
                             stdout=pairs[0][1], stderr=pairs[1][1],
                             env=dict(os.environ), start_new_session=True)
    for master, slave in pairs:
        os.close(slave)
    while active:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise RuntimeError('migration PTY deadline exceeded')
        ready, _, _ = select.select(active, [], [], remaining)
        for master in ready:
            try:
                data = os.read(master, 65536)
            except OSError as error:
                if error.errno != errno.EIO:
                    raise
                data = b''
            if data:
                streams[master].extend(data)
            else:
                active.remove(master)
    status = child.wait(timeout=max(0.01, deadline - time.monotonic()))
    print(json.dumps([status, *(streams[m].decode() for m, _ in pairs)]))
finally:
    if child is not None and child.poll() is None:
        os.killpg(child.pid, signal.SIGKILL)
        child.wait(timeout=2)
    for master, slave in pairs:
        for fd in [master, slave]:
            try:
                os.close(fd)
            except OSError:
                pass
"#;
    let registry = r.registry();
    let output = command(r, "/usr/bin/python3")
        .args(["-I", "-c", script, env!("CARGO_BIN_EXE_vibe")])
        .args([
            "app", "migrate", "stateful", "1.0.0", "2.0.0", "--all", "--resume",
            "--offline", "--registry", registry.to_str().unwrap(),
        ])
        .output()
        .expect("migration PTY runner");
    assert!(output.status.success(), "migration PTY runner failed");
    let (status, stdout, stderr): (i32, String, String) =
        serde_json::from_slice(&output.stdout).expect("migration PTY response");
    assert_eq!(status, 1, "mixed migration must fail");
    let steps = console::strip_ansi_codes(&stderr);
    assert!(steps.contains("First · import `import-first`"), "missing failed import step");
    assert!(steps.contains("exit 5"), "missing actual fixture failure");
    assert!(steps.contains("Second · import `import-second`"), "missing successful import step");
    let last = stdout.lines().last().expect("migration final line");
    let excerpt = last.chars().take(1200).collect::<String>();
    let what = "migrated stateful 1.0.0 → 2.0.0 partly · 1 not carried";
    // The active roles.rs palette is alert red and ok green.
    let alert = console::Style::new().red().force_styling(true);
    let ok = console::Style::new().green().force_styling(true);
    assert!(last.contains(&alert.apply_to("✻").to_string()), "last trailer is not alert {excerpt:?}");
    assert!(last.contains(&alert.apply_to(what).to_string()), "last trailer lacks mixed verdict {excerpt:?}");
    assert!(!last.contains(&ok.apply_to("✻").to_string()), "last trailer is ok {excerpt:?}");
    let plain = console::strip_ansi_codes(last);
    let trailers = console::strip_ansi_codes(&stdout)
        .lines()
        .chain(steps.lines())
        .filter(|line| line.starts_with("✻ "))
        .count();
    assert_eq!(trailers, 1, "expected exactly one final trailer {excerpt:?}");
    let prefix = format!("✻ {what} · ");
    let suffix = format!(" · {}", work.display());
    let metadata = plain
        .strip_prefix(&prefix)
        .and_then(|line| line.strip_suffix(&suffix))
        .unwrap_or_else(|| panic!("wrong final verdict or retained work path {excerpt:?}"));
    let fields = metadata.split(" · ").collect::<Vec<_>>();
    let elapsed = match fields.as_slice() {
        [elapsed] => *elapsed,
        [steps, elapsed] => {
            let (count, unit) = steps.split_once(' ')
                .unwrap_or_else(|| panic!("invalid step metadata {excerpt:?}"));
            let count = count.parse::<usize>()
                .unwrap_or_else(|_| panic!("invalid step count {excerpt:?}"));
            assert!(
                count > 0 && unit == if count == 1 { "step" } else { "steps" },
                "invalid step metadata {excerpt:?}"
            );
            *elapsed
        }
        _ => panic!("invalid trailer metadata order {excerpt:?}"),
    };
    assert!(valid_elapsed(elapsed), "invalid elapsed immediately before work {excerpt:?}");
    assert!(work.is_dir(), "retained work directory missing");
}

/// Accept exactly the canonical forms emitted by the shared elapsed renderer.
fn valid_elapsed(text: &str) -> bool {
    let parts = text.split(' ').collect::<Vec<_>>();
    let number = |part: &str, unit: char| -> Option<u64> {
        let digits = part.strip_suffix(unit)?;
        let value = digits.parse::<u64>().ok()?;
        (digits == value.to_string()).then_some(value)
    };
    match parts.as_slice() {
        [part] => {
            number(part, 's').is_some_and(|n| n < 60)
                || number(part, 'm').is_some_and(|n| (1..60).contains(&n))
                || number(part, 'h').is_some_and(|n| n > 0)
        }
        [first, second] => {
            (number(first, 'm').is_some_and(|n| (1..60).contains(&n))
                && number(second, 's').is_some_and(|n| (1..60).contains(&n)))
                || (number(first, 'h').is_some_and(|n| n > 0)
                    && number(second, 'm').is_some_and(|n| (1..60).contains(&n)))
        }
        _ => false,
    }
}
