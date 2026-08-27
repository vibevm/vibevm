//! R3.4 RED 21 — a FAILED quiet `vibe update` / `vibe reinstall` under a
//! requested compile trace.
//!
//! Quiet's whole contract is one line. A failed quiet command's only line is
//! the terminal error, so the trace suffix travels ON it — appended to the
//! exact old line, never a second line — and the trace-off twin is that old
//! line, byte for byte, having opened no run at all.
//!
//! The fixture and its marker-gated sabotage are the shared ones (see
//! `common::trace_failure_slot`); each test seeds its off/on pair as separate
//! projects because the sabotage makes a failed project single-use.

mod common;
mod trace_support;

use std::path::Path;

use common::UserScratch;
use common::trace_failure_slot::{
    HARD_POST_SECRET, add_version, arm_hard_post, corrupt_payload, normalise_stderr, project,
    publish_ordered_post_install, seed_untraced,
};
use trace_support::{all_trace_bytes, index_of, run_directories, trace_dir};
use vibe_wire::generated::compiler_trace_index::e1::index::RunStatus;

/// The whole of a FAILED quiet run's observable behaviour: the command fails,
/// stdout is empty, and stderr comes back for the one-line contract to judge.
///
/// Deliberately NOT `trace_support::quiet_stdout`, which asserts success —
/// these commands fail, and a helper that demanded their exit 0 would make
/// every red here unrunnable.
fn quiet_failure_stderr(output: &std::process::Output) -> String {
    assert!(
        !output.status.success(),
        "the armed hard-post sabotage fails the command",
    );
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        stdout.is_empty(),
        "a failed quiet command prints nothing on stdout: {stdout:?}",
    );
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The off/on line contract: one line each, the same error, the suffix
/// appended, the exit code unmoved.
fn assert_one_line_with_suffix(
    off_line: &str,
    on_line: &str,
    off_code: Option<i32>,
    on_code: Option<i32>,
    off_project: &Path,
    on_project: &Path,
) {
    assert_eq!(
        off_line.lines().count(),
        1,
        "trace off is exactly one stderr line: {off_line:?}",
    );
    assert_eq!(
        on_line.lines().count(),
        1,
        "and trace on is STILL exactly one: {on_line:?}",
    );
    assert!(
        on_line.contains("compile trace failed"),
        "the failure suffix rides that one line: {on_line:?}",
    );
    let off_folded = normalise_stderr(off_project, off_line.as_bytes());
    let on_folded = normalise_stderr(on_project, on_line.as_bytes());
    assert!(
        on_folded.starts_with(off_folded.trim_end()),
        "the suffix is APPENDED to the exact old line:\n off: {off_folded:?}\n on:  {on_folded:?}",
    );
    assert!(
        on_folded.trim_end().len() > off_folded.trim_end().len(),
        "and something really was appended",
    );
    assert_eq!(off_code, on_code, "the exit code is unchanged");
}

/// The off/on run contract: silent off allocates nothing; traced on owns
/// exactly one terminal failed run with the fixed failure word, and no byte
/// of it quotes the sabotage secret.
fn assert_run_pair(off_project: &Path, on_project: &Path) {
    assert!(
        run_directories(off_project).is_empty() && !trace_dir(off_project).exists(),
        "the trace-off twin creates no run at all",
    );
    let runs = run_directories(on_project);
    assert_eq!(runs.len(), 1, "one command, one run: {runs:?}");
    let index = index_of(on_project, &runs[0]);
    assert!(
        matches!(index.status, RunStatus::Failed),
        "the run is terminal failed: {index:?}",
    );
    assert!(
        index.finished.is_some(),
        "finalised — not abandoned mid-flight: {index:?}",
    );
    assert_eq!(
        index.failure.as_deref(),
        Some("command failed"),
        "the fixed failure word, and nothing else",
    );
    assert!(
        !all_trace_bytes(on_project).contains(HARD_POST_SECRET),
        "and the trace tree never quotes the sabotage secret",
    );
}

/// A scoped update over a freshly published 0.1.1, quiet, armed to fail hard.
#[test]
fn a_failed_quiet_traced_scoped_update_is_one_line_with_a_trace_suffix() {
    if !common::git_available() {
        eprintln!("skipping quiet scoped-update failure e2e: git not on PATH");
        return;
    }
    let outer = tempfile::tempdir().unwrap();
    let published = publish_ordered_post_install(outer.path());
    let user = UserScratch::new();
    let off_project = project(&user, &published.registry);
    let on_project = project(&user, &published.registry);
    seed_untraced(&user, off_project.path());
    seed_untraced(&user, on_project.path());
    add_version(&published, "0.1.1");
    arm_hard_post(off_project.path());
    arm_hard_post(on_project.path());

    let run = |target: &Path, extra: &[&str]| {
        user.vibe()
            .args(["update", "org.demo/tools", "--quiet", "--assume-yes"])
            .args(extra)
            .arg("--path")
            .arg(target)
            .output()
            .unwrap()
    };
    let off = run(off_project.path(), &[]);
    let on = run(on_project.path(), &["--trace-compile"]);
    let off_line = quiet_failure_stderr(&off);
    let on_line = quiet_failure_stderr(&on);
    assert_one_line_with_suffix(
        &off_line,
        &on_line,
        off.status.code(),
        on.status.code(),
        off_project.path(),
        on_project.path(),
    );
    assert_run_pair(off_project.path(), on_project.path());
}

/// A forced reinstall over a corrupted payload, quiet, armed to fail hard.
#[test]
fn a_failed_quiet_traced_forced_reinstall_is_one_line_with_a_trace_suffix() {
    if !common::git_available() {
        eprintln!("skipping quiet forced-reinstall failure e2e: git not on PATH");
        return;
    }
    let outer = tempfile::tempdir().unwrap();
    let published = publish_ordered_post_install(outer.path());
    let user = UserScratch::new();
    let off_project = project(&user, &published.registry);
    let on_project = project(&user, &published.registry);
    seed_untraced(&user, off_project.path());
    seed_untraced(&user, on_project.path());
    arm_hard_post(off_project.path());
    arm_hard_post(on_project.path());
    corrupt_payload(off_project.path(), "0.1.0");
    corrupt_payload(on_project.path(), "0.1.0");

    let run = |target: &Path, extra: &[&str]| {
        user.vibe()
            .args(["reinstall", "--force", "--quiet", "--assume-yes"])
            .args(extra)
            .arg(target)
            .output()
            .unwrap()
    };
    let off = run(off_project.path(), &[]);
    let on = run(on_project.path(), &["--trace-compile"]);
    let off_line = quiet_failure_stderr(&off);
    let on_line = quiet_failure_stderr(&on);
    assert_one_line_with_suffix(
        &off_line,
        &on_line,
        off.status.code(),
        on.status.code(),
        off_project.path(),
        on_project.path(),
    );
    assert_run_pair(off_project.path(), on_project.path());
}
