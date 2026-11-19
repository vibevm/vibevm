//! Pi-only managed route: child-scoped environment, native exit and failure cleanup.
#![cfg(target_os = "linux")]
mod common;
use common::UserScratch;
use std::path::Path;
use std::time::{Duration, Instant};

fn gone(pid: u32) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let done = std::fs::read_to_string(format!("/proc/{pid}/stat")).map_or(true, |s| {
            s.rsplit(')')
                .next()
                .is_some_and(|tail| tail.trim_start().starts_with('Z'))
        });
        if done {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn pi_managed_route_preserves_arguments_status_and_reaps_px_on_spawn_failure() {
    let user = UserScratch::new();
    let root = tempfile::tempdir().unwrap();
    let log = root.path().join("px.log");
    let px = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/common/fake-px.py");
    std::fs::write(user.settings.join("config.toml"), format!(
        "[proxy]\nmode='px'\nupstream='synthetic.invalid:8080'\npx_path='{}'\nno_proxy='.synthetic.test'\n", px.display()
    )).unwrap();
    std::fs::write(user.settings.join("agents.toml"),
        "[launchers.pi]\nkind='pi'\ncommand=['/bin/sh','-c','printf \"%s\\n\" \"$HTTP_PROXY\" \"$https_proxy\" \"$NO_PROXY\" \"$1\"; exit 7','native-prefix']\n"
    ).unwrap();
    let output = user
        .vibe()
        .current_dir(root.path())
        .env("FAKE_PX_LOG", &log)
        .args(["agent", "pi", "two words"])
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(7),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<_> = stdout.lines().collect();
    assert!(lines[0].starts_with("http://127.0.0.1:"));
    assert_eq!(lines[0], lines[1]);
    assert_eq!(lines[2], "localhost,127.0.0.1,::1,.synthetic.test");
    assert_eq!(lines[3], "two words");
    std::fs::write(
        user.settings.join("agents.toml"),
        "[launchers.pi]\nkind='pi'\ncommand=['/no-such-owned-pi']\n",
    )
    .unwrap();
    let refused = user
        .vibe()
        .current_dir(root.path())
        .env("FAKE_PX_LOG", &log)
        .args(["agent", "pi"])
        .output()
        .unwrap();
    assert!(!refused.status.success());
    // A configured corporate route must never silently become unproxied
    // execution for an adapter outside this first qualified identity.
    std::fs::write(
        user.settings.join("agents.toml"),
        "[launchers.other]\nkind='qwen'\ncommand=['/bin/sh','-c','echo SHOULD_NOT_START']\n",
    )
    .unwrap();
    let other = user
        .vibe()
        .current_dir(root.path())
        .env("FAKE_PX_LOG", &log)
        .args(["agent", "other"])
        .output()
        .unwrap();
    assert!(!other.status.success());
    assert!(!String::from_utf8_lossy(&other.stdout).contains("SHOULD_NOT_START"));
    assert!(String::from_utf8_lossy(&other.stderr).contains("not yet qualified"));
    // An existing OpenCode server has a different inference owner and cannot
    // receive the invocation route through a local attachment.
    std::fs::write(
        user.settings.join("agents.toml"),
        "[launchers.remote]\nkind='opencode'\ncommand=['/bin/sh','-c','echo SHOULD_NOT_START']\n",
    )
    .unwrap();
    let attached = user
        .vibe()
        .current_dir(root.path())
        .env("FAKE_PX_LOG", &log)
        .args([
            "agent",
            "remote",
            "--standalone",
            "attach",
            "http://127.0.0.1:9",
        ])
        .output()
        .unwrap();
    assert!(!attached.status.success());
    assert!(!String::from_utf8_lossy(&attached.stdout).contains("SHOULD_NOT_START"));
    assert!(String::from_utf8_lossy(&attached.stderr).contains("existing server owner"));
    let served = user
        .vibe()
        .current_dir(root.path())
        .env("FAKE_PX_LOG", &log)
        .args(["agent", "remote", "--standalone", "serve"])
        .output()
        .unwrap();
    assert!(!served.status.success());
    assert!(!String::from_utf8_lossy(&served.stdout).contains("SHOULD_NOT_START"));
    std::fs::write(
        user.settings.join("agents.toml"),
        "[launchers.retained]\nkind='codex'\ncommand=['/bin/sh','-c','echo SHOULD_NOT_START']\n",
    )
    .unwrap();
    let resumed = user
        .vibe()
        .current_dir(root.path())
        .env("FAKE_PX_LOG", &log)
        .args(["agent", "retained", "exec", "resume", "--last"])
        .output()
        .unwrap();
    assert!(!resumed.status.success());
    assert!(!String::from_utf8_lossy(&resumed.stdout).contains("SHOULD_NOT_START"));
    assert!(String::from_utf8_lossy(&resumed.stderr).contains("fresh noninteractive Codex exec"));
    let text = std::fs::read_to_string(&log).unwrap();
    let pids: Vec<u32> = text
        .lines()
        .filter_map(|l| l.strip_prefix("start "))
        .map(|l| l.split_whitespace().next().unwrap().parse().unwrap())
        .collect();
    assert_eq!(pids.len(), 2, "{text}");
    assert!(pids.into_iter().all(gone));
}
