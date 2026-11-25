//! Isolated commands and a bounded coloured PTY for the extras fixture.

use super::common::app_registry::AppRegistry;
use std::process::Command;

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

/// Run the actual failing apply on a bounded coloured PTY, with isolated homes.
pub fn assert_failed_trailer(r: &AppRegistry, parent: &std::path::Path) {
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
            raise RuntimeError('extras PTY deadline exceeded')
        ready, _, _ = select.select(active, [], [], remaining)
        for master in ready:
            try:
                data = os.read(master, 65536)
            except OSError as error:
                if error.errno != errno.EIO:
                    raise
                data = b''
            if data:
                if len(streams[master]) + len(data) > 1048576:
                    raise RuntimeError("extras PTY output exceeded limit")
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
    let before = std::fs::read_dir(parent)
        .expect("fixture work parent")
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    let output = command(r, "/usr/bin/python3")
        .args(["-I", "-c", script, env!("CARGO_BIN_EXE_vibe")])
        .args([
            "app", "extras", "apply", "runtime", "--ecosystem", "npm",
            "--offline", "--registry", registry.to_str().unwrap(),
        ])
        .output()
        .expect("extras PTY runner");
    assert!(output.status.success(), "extras PTY runner failed");
    let (status, stdout, stderr): (i32, String, String) =
        serde_json::from_slice(&output.stdout).expect("extras PTY response");
    assert_eq!(status, 1, "failing extras apply must fail");
    let work = std::fs::read_dir(parent)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| !before.contains(path) && path.join("ecosystem.json").is_file())
        .expect("the actual apply retained its ecosystem record");
    let record: serde_json::Value = serde_json::from_slice(
        &std::fs::read(work.join("ecosystem.json")).unwrap(),
    )
    .expect("retained ecosystem document");
    assert_eq!(record["work"].as_str(), work.to_str());
    assert_eq!(record["key"], "org.example/runtime@1.0.0");
    assert_eq!(record["ok"], false);
    assert_eq!(record["adapters"][0]["outcome"], "failed");
    let steps = console::strip_ansi_codes(&stderr);
    let last = stdout.lines().last().expect("extras final line");
    let excerpt = last.chars().take(1200).collect::<String>();
    let what = "extras of runtime not applied · 1 failed";
    // The active roles.rs palette is alert red and ok green.
    let alert = console::Style::new().red().force_styling(true);
    let ok = console::Style::new().green().force_styling(true);
    assert!(last.contains(&alert.apply_to("✻").to_string()), "last trailer is not alert {excerpt:?}");
    assert!(last.contains(&alert.apply_to(what).to_string()), "last trailer lacks failed verdict {excerpt:?}");
    assert!(!last.contains(&ok.apply_to("✻").to_string()), "last trailer is ok {excerpt:?}");
    assert!(!last.contains(&ok.apply_to(what).to_string()), "last verdict is ok {excerpt:?}");
    let plain = console::strip_ansi_codes(last);
    let trailers = console::strip_ansi_codes(&stdout)
        .lines()
        .chain(steps.lines())
        .filter(|line| line.starts_with("✻ "))
        .count();
    assert_eq!(trailers, 1, "expected exactly one final trailer {excerpt:?}");
    assert!(console::strip_ansi_codes(&stdout).contains(
        "vibe app extras apply runtime --ecosystem npm"
    ), "missing exact retry command");
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
