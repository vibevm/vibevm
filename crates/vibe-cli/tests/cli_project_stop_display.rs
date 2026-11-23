//! A real interrupted hook closes human and quiet output with its rollback verdict.
#![cfg(unix)]

mod common;

use common::UserScratch;
use common::hook_slot::Fixture;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

fn command(user: &UserScratch, home: &Path) -> assert_cmd::Command {
    let mut command = user.vibe();
    command
        .env("HOME", home)
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .env("NO_COLOR", "1")
        .env_remove("VIBE_AGENT_COMMAND")
        .env_remove("VIBE_AGENT_ARGS");
    command
}

fn assert_elapsed(text: &str, wall: Duration) {
    let seconds: u64 = text.strip_suffix('s').unwrap().parse().unwrap();
    assert!(
        seconds <= wall.as_secs() + 1,
        "elapsed {text}, wall {wall:?}"
    );
}

fn stopped_install(quiet: bool) {
    // The common setup calls init with inherited HOME. Reuse its exact
    // registry fixture, but pin HOME for init as well as for the install.
    let outer = tempfile::tempdir().unwrap();
    let registry = common::hook_slot::registry(outer.path(), Fixture::Timing);
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    let home = outer.path().join("home");
    fs::create_dir_all(&home).unwrap();
    command(&user, &home)
        .args(["init", "--path"])
        .arg(project.path())
        .assert()
        .success();
    let before = fs::read(project.path().join("vibe.toml")).unwrap();
    let stop = project.path().join(".vibe/cancel-request");
    fs::create_dir_all(stop.parent().unwrap()).unwrap();
    fs::write(
        registry.join("org.example/hooked/v0.1.0/hooks/pre.sh"),
        "set -eu\nprintf 'STOP-HOOK-RUNNING\\n'\nprintf stop > \"$VIBE_PROJECT_ROOT/.vibe/cancel-request\"\nsleep 30\nprintf 'STOP-HOOK-ESCAPED\\n'\n",
    )
    .unwrap();
    let mut install = command(&user, &home);
    install
        .args([
            "install",
            "org.example/hooked@=0.1.0",
            "--offline",
            "--assume-yes",
            "--stop-file",
        ])
        .arg(&stop)
        .arg("--registry")
        .arg(&registry)
        .arg("--path")
        .arg(project.path());
    if quiet {
        install.arg("--quiet");
    }
    let started = Instant::now();
    let output = install.output().unwrap();
    let wall = started.elapsed();
    let stderr = String::from_utf8(output.stderr).unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(130), "{stdout}\n{stderr}");
    assert!(stop.is_file(), "the real hook must request the stop");
    assert_eq!(fs::read(project.path().join("vibe.toml")).unwrap(), before);
    assert!(
        !project
            .path()
            .join(common::slot_dir("org.example.hooked", "0.1.0"))
            .exists()
    );
    assert!(!stderr.contains("package installation failed"), "{stderr}");
    assert!(!stderr.contains("✗ Installing packages"), "{stderr}");
    assert!(!stdout.contains("SUCCESS") && !stderr.contains("SUCCESS"));
    // This package directory belongs only to this fresh invocation. Require
    // its sole typed record, rather than guessing a timestamp or searching caches.
    let package_records = user
        .settings
        .join("cache/install-steps/org.example/hooked/0.1.0");
    let records: Vec<_> = fs::read_dir(&package_records)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(records.len(), 1);
    let record: vibe_wire::generated::install_steps::InstallSteps =
        serde_json::from_slice(&fs::read(records[0].join("steps.json")).unwrap()).unwrap();
    let stopped = record
        .contributions
        .iter()
        .find(|row| row.id == "pre-install")
        .unwrap();
    assert_eq!(
        stopped.outcome,
        vibe_wire::generated::install_steps::Outcome::Interrupted
    );
    let actual_log = records[0].join(stopped.log.as_deref().unwrap());
    assert!(actual_log.is_file());
    let actual_bytes = fs::read_to_string(&actual_log).unwrap();
    assert!(actual_bytes.contains("STOP-HOOK-RUNNING"));
    assert!(!actual_bytes.contains("STOP-HOOK-ESCAPED"));
    eprintln!(
        "CURRENT STOP PROOF path={} bytes={actual_bytes:?} trailer={:?}",
        actual_log.display(),
        stderr.lines().rfind(|line| !line.trim().is_empty())
    );
    let lines: Vec<_> = stderr
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    let last = *lines.last().unwrap();
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.starts_with("✻ stopped"))
            .count(),
        1,
        "{stderr}"
    );
    let fields: Vec<_> = last.split(" · ").collect();
    if quiet {
        assert_eq!(lines.len(), 1, "quiet has no progress rows: {stderr}");
        assert_eq!(fields.len(), 3, "{last}");
        assert_eq!(fields[0], "✻ stopped");
        assert_eq!(fields[1], "rolled back, nothing changed");
        assert_elapsed(fields[2], wall);
    } else {
        assert_eq!(fields.len(), 5, "{last}");
        assert_eq!(fields[0], "✻ stopped at pre-install");
        assert_eq!(fields[1], "rolled back, nothing changed");
        let rows: Vec<_> = lines
            .iter()
            .filter(|line| line.starts_with(['✓', '○', '✗', '⊘']))
            .collect();
        let count = rows.len();
        assert!(count > 0, "{stderr}");
        let unit = if count == 1 { "step" } else { "steps" };
        assert_eq!(fields[2], format!("{count} {unit}"));
        assert_elapsed(fields[3], wall);
        let run = Path::new(fields[4].strip_prefix("log ").unwrap());
        assert!(run.is_absolute() && run.is_dir(), "{last}");
        assert!(run.starts_with(&user.settings), "{last}");
        let pre = rows
            .iter()
            .find(|row| row.starts_with("○ └ pre-install"))
            .unwrap_or_else(|| panic!("no stopped pre-install row: {stderr}"));
        assert!(pre.contains("stopped after "), "{pre}");
        assert_eq!(run, records[0]);
        assert!(
            pre.contains(actual_log.file_name().unwrap().to_str().unwrap()),
            "{pre}"
        );
        let evidence = fs::read_to_string(&actual_log).unwrap();
        assert!(evidence.contains("STOP-HOOK-RUNNING"), "{evidence}");
        assert!(!evidence.contains("STOP-HOOK-ESCAPED"), "{evidence}");
        assert!(
            rows.iter()
                .any(|row| row.starts_with("○ Installing packages")
                    && row.contains("stopped after ")),
            "{stderr}"
        );
    }
}

#[test]
fn stop_file_run_prints_stop_trailer() {
    stopped_install(false);
}

#[test]
fn quiet_stop_reports_restored_publication() {
    stopped_install(true);
}
