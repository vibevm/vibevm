//! R3.4 — when a compile trace is activated, and what a disabled run leaves
//! behind.
//!
//! Every test drives the real binary: activation is a property of the command
//! as invoked, and a unit test of the truth table would prove nothing about
//! which manifest the command actually consulted.

mod common;
mod trace_support;

use common::UserScratch;
use serde_json::Value;
use trace_support::{install_json, lifecycle_run_dirs, run_directories, trace_dir, trace_member};

// ---------------------------------------------------------------- 1. grammar

/// The flag parses everywhere the packet says it does — and nowhere else.
///
/// Driven through `--help`-free real invocations on an empty project so a clap
/// rejection is unambiguous: exit code 2 with `unexpected argument`, versus a
/// command that ran.
#[test]
fn the_flag_parses_on_install_every_phase_and_a_clean_chain() {
    let user = UserScratch::new();
    for verb in [
        "install", "validate", "generate", "build", "test", "create", "verify", "package", "deploy",
    ] {
        let project = tempfile::tempdir().unwrap();
        user.init_project(project.path());
        let output = user
            .vibe()
            .args([verb, "--trace-compile", "--offline", "--assume-yes"])
            .arg("--path")
            .arg(project.path())
            .output()
            .unwrap();
        assert!(
            !trace_support::clap_rejected(&output),
            "`vibe {verb} --trace-compile` must parse: {}",
            String::from_utf8_lossy(&output.stderr),
        );
    }
    for chained in ["install", "build"] {
        let project = tempfile::tempdir().unwrap();
        user.init_project(project.path());
        let output = user
            .vibe()
            .args(["clean", chained, "--trace-compile", "--offline"])
            .arg("--path")
            .arg(project.path())
            .arg("--assume-yes")
            .output()
            .unwrap();
        assert!(
            !trace_support::clap_rejected(&output),
            "`vibe clean {chained} --trace-compile` must parse: {}",
            String::from_utf8_lossy(&output.stderr),
        );
    }
}

/// Clean-only compiles nothing, so it has no flag to give — and the rejection
/// is clap's, before any run id is minted or any wipe is confirmed.
#[test]
fn bare_clean_rejects_the_flag_and_opens_nothing() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let output = user
        .vibe()
        .args(["clean", "--trace-compile", "--assume-yes"])
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(
        trace_support::clap_rejected(&output),
        "clean-only has no `--trace-compile`"
    );
    assert!(!trace_dir(project.path()).exists());
}

/// A DEPENDENCY that traces its own builds cannot switch tracing on for the
/// project that installed it. The host's own manifest is the only vote.
#[test]
fn a_dependency_manifest_cannot_activate_the_host() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let registry = tempfile::tempdir().unwrap();
    trace_support::publish_tracing_package(registry.path());

    let output = user
        .vibe()
        .args(["install", "org.trace/dep@=0.1.0", "--json", "--registry"])
        .arg(registry.path())
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !trace_dir(project.path()).exists(),
        "a dependency's `[compile] trace` must not open a trace for its host",
    );
    let root = trace_support::sole_root(&output.stdout, "install");
    assert!(root.get("trace").is_none(), "disabled omits the member");
}

/// The manifest half of the truth table, end to end: no flag, and the run is
/// traced anyway because the SELECTED project asked for it.
#[test]
fn the_selected_manifest_alone_activates_without_a_flag() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    trace_support::declare_trace(project.path());

    let report = install_json(&user, project.path(), &[]);
    let trace = trace_member(&report).expect("the manifest's standing request is honoured");
    assert_eq!(trace["status"], "ok");
    assert_eq!(run_directories(project.path()).len(), 1);
}

// ------------------------------------------------------- 2. disabled is inert

/// Trace off writes nothing at all — no directory, no lock — and the old
/// documents are byte-identical.
#[test]
fn a_disabled_install_writes_no_trace_and_keeps_its_old_bytes() {
    let user = UserScratch::new();
    let plain = tempfile::tempdir().unwrap();
    user.init_project(plain.path());
    let off = install_json(&user, plain.path(), &[]);
    assert!(!trace_dir(plain.path()).exists(), "no `.vibe/trace`");
    assert!(
        !trace_support::trace_lock_exists(plain.path()),
        "and no cooperative lock file either — disabled allocates nothing",
    );
    assert!(off.get("trace").is_none());

    // The same invocation with the flag differs ONLY by the added member.
    let traced_project = tempfile::tempdir().unwrap();
    user.init_project(traced_project.path());
    let mut on = install_json(&user, traced_project.path(), &["--trace-compile"]);
    assert!(on.get("trace").is_some());
    let on_object = on.as_object_mut().unwrap();
    on_object.remove("trace");
    let mut off_object = off.clone();
    trace_support::normalise_project(&mut off_object);
    let mut on_normalised = Value::Object(on_object.clone());
    trace_support::normalise_project(&mut on_normalised);
    assert_eq!(
        off_object, on_normalised,
        "the trace member is the ONLY difference a disabled twin has",
    );
}

/// Quiet mode keeps its one line with trace off, and gains a suffix — not a
/// second line — with trace on.
#[test]
fn quiet_keeps_exactly_one_line_with_and_without_the_flag() {
    let user = UserScratch::new();
    let off_project = tempfile::tempdir().unwrap();
    user.init_project(off_project.path());
    let off = trace_support::quiet_install(&user, off_project.path(), &[]);
    assert_eq!(off.lines().count(), 1, "trace off is one line: {off:?}");

    let on_project = tempfile::tempdir().unwrap();
    user.init_project(on_project.path());
    let on = trace_support::quiet_install(&user, on_project.path(), &["--trace-compile"]);
    assert_eq!(on.lines().count(), 1, "trace on is STILL one line: {on:?}");
    assert!(
        on.contains("compile trace ok"),
        "the suffix rides that one line: {on:?}",
    );
    assert!(
        on.starts_with(off.trim_end()),
        "the suffix is APPENDED to the exact old line:\n old: {off:?}\n new: {on:?}",
    );
}

/// The manifest is read ONCE, and what the command does afterwards follows
/// from that one read.
///
/// The mutation is the proof: the file is corrupted before the command runs,
/// so every claim below is about a command that could not have read a sound
/// manifest at any point. It must still select an identity (the run directory
/// is allocated), open NO trace storage (there is no sound workspace to name
/// one), report the request honestly as `unavailable`, and fail with the error
/// its own read produced — consumed inside the funnel, after the identity.
#[test]
fn a_malformed_selected_manifest_is_read_once_and_its_error_reaches_the_funnel() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    trace_support::corrupt_manifest(project.path());

    let output = user
        .vibe()
        .args([
            "install",
            "--trace-compile",
            "--json",
            "--offline",
            "--assume-yes",
        ])
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(!output.status.success(), "a malformed manifest fails");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("vibe.toml"),
        "with the error its own read produced: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        !lifecycle_run_dirs(project.path()).is_empty(),
        "the identity was selected before that error was consumed",
    );
    assert!(
        !trace_dir(project.path()).exists(),
        "no trace storage opens without a sound workspace to name its root",
    );
    // The request was real, so the root still says what became of it.
    let root = trace_support::sole_root(&output.stdout, "install");
    assert_eq!(root["ok"], false);
    let trace = trace_member(&root).expect("a requested trace reports its own fate");
    assert_eq!(
        trace["status"], "unavailable",
        "requested and not opened is `unavailable`, never a silent `disabled`",
    );
    assert!(
        !trace["warnings"].as_array().unwrap().is_empty(),
        "and it says why",
    );
}
