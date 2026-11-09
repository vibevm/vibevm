//! The bounded `px --help` version probe never leaves a server running.

use std::os::unix::fs::PermissionsExt;
use std::time::{Duration, Instant};

use super::{bounded_output, program_version};

fn script(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

#[test]
fn the_px_help_line_is_the_recorded_version() {
    let dir = tempfile::tempdir().unwrap();
    let px = script(
        dir.path(),
        "px",
        "echo 'Px v0.11.0'; echo; echo 'An HTTP proxy server'",
    );
    assert_eq!(program_version(&px).as_deref(), Some("Px v0.11.0"));
}

#[test]
fn a_program_that_never_exits_is_killed_and_records_no_version() {
    let dir = tempfile::tempdir().unwrap();
    // what px 0.11 does with an argument it does not know: serve forever
    let px = script(dir.path(), "px", "echo serving; exec sleep 600");
    let started = Instant::now();
    assert_eq!(
        bounded_output(&px, &["--help"], Duration::from_millis(300)),
        None
    );
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "the probe must not wait for the server"
    );
}
