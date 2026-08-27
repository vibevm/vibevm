//! R3.4 — one command, one run: ownership, identity and fidelity.
//!
//! The properties here can only be observed from outside the process. "Exactly
//! one command-owned run directory" and "the higher phase owns its
//! prerequisite install's trace" are statements about which process opened
//! what, and a unit test that called `prepare` twice would not be evidence.
//!
//! Nothing here asserts a measured duration. Timings are compared by NAME,
//! ORDER and INVOCATION COUNT against the run's own on-disk index; the micros
//! are whatever the machine did.

mod common;
mod trace_support;

use std::path::Path;

use common::{UserScratch, fixture_registry};
use serde_json::Value;
use trace_support::{documents, index_of, install_json, run_directories, sole_run, trace_member};

// --------------------------------------------- 3./4. one run, one id, one path

/// Every direct invocation shape opens exactly one command-owned run, and
/// every member of that run agrees on the id. Two non-resume commands mint
/// different ids.
#[test]
fn each_direct_invocation_owns_exactly_one_run_and_two_commands_differ() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());

    // Empty world: nothing declared, boot artifacts regenerated.
    let empty = install_json(&user, project.path(), &["--trace-compile"]);
    let first = sole_run(project.path(), &empty);

    // Fresh lock: the same project again, nothing to re-resolve.
    let fresh = install_json(&user, project.path(), &["--trace-compile"]);
    let second = sole_run(project.path(), &fresh);
    assert_ne!(
        first, second,
        "two non-resume commands mint different run ids",
    );

    // Ready apply: a real dependency moves.
    let ready = user
        .vibe()
        .args([
            "install",
            "flow:org.vibevm/integration-alpha",
            "--trace-compile",
            "--json",
            "--registry",
        ])
        .arg(fixture_registry())
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        ready.status.success(),
        "{}",
        String::from_utf8_lossy(&ready.stderr)
    );
    let ready = trace_support::sole_root(&ready.stdout, "install");
    let third = sole_run(project.path(), &ready);
    assert!(
        third != first && third != second,
        "a ready apply is its own command and its own run",
    );

    // Every run directory that exists is one of the three this test made.
    let directories = run_directories(project.path());
    assert_eq!(directories.len(), 3, "one directory per invocation");
    for id in [&first, &second, &third] {
        assert!(directories.contains(id), "run `{id}` has its own directory");
    }
}

/// A dependency's package UNIT and the project's own NODE land under the ONE
/// directory this command owns, sharing its id and its global sequence.
#[test]
fn package_unit_and_node_events_share_one_command_run() {
    let user = UserScratch::new();
    let project = trace_support::static_project(&user);
    let output = user
        .vibe()
        .args(["install", "--trace-compile", "--json", "--registry"])
        .arg(fixture_registry())
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
    let report = trace_support::sole_root(&output.stdout, "install");
    let run_id = sole_run(project.path(), &report);
    let index = index_of(project.path(), &run_id);

    assert_eq!(index.run_id, run_id, "the index names the command's run");
    let kinds: Vec<String> = index
        .scopes
        .iter()
        .map(|scope| format!("{:?}", scope.kind).to_lowercase())
        .collect();
    assert!(
        kinds.iter().any(|kind| kind == "node"),
        "the project's own node compiled under this run: {kinds:?}",
    );
    assert!(
        !index.events.is_empty(),
        "and the compile really recorded pass events",
    );
    // One run means one dense global sequence across BOTH scope kinds.
    let mut sequences: Vec<u32> = index.events.iter().map(|event| event.sequence).collect();
    sequences.sort_unstable();
    assert_eq!(
        sequences,
        (0..u32::try_from(sequences.len()).unwrap()).collect::<Vec<_>>(),
        "one run, one dense sequence — not two interleaved numberings",
    );
    assert!(
        index
            .scopes
            .iter()
            .all(|scope| !Path::new(&scope.label).is_absolute()),
        "no absolute developer path becomes a scope id",
    );
}

/// A higher lifecycle phase owns its PREREQUISITE install's trace: one run for
/// the whole command, not one for the phase and one for the install beneath it.
#[test]
fn a_higher_phase_owns_its_prerequisite_installs_trace() {
    let user = UserScratch::new();
    let project = trace_support::static_project(&user);
    let output = user
        .vibe()
        .args(["build", "--trace-compile", "--json", "--registry"])
        .arg(fixture_registry())
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
    let report = trace_support::sole_root(&output.stdout, "lifecycle");
    let run_id = sole_run(project.path(), &report);
    assert_eq!(
        run_directories(project.path()),
        vec![run_id.clone()],
        "the phase verb's ONE run is the only one on disk",
    );
    let index = index_of(project.path(), &run_id);
    assert!(
        !index.scopes.is_empty(),
        "the prerequisite install's compiles are inside the phase's run",
    );
}

/// Validate compiles nothing — and says so honestly, with a finalised
/// zero-scope terminal rather than an absent or `running` trace.
#[test]
fn validate_only_finalises_a_truthful_zero_scope_run() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let output = user
        .vibe()
        .args(["validate", "--trace-compile", "--json", "--offline"])
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = trace_support::sole_root(&output.stdout, "lifecycle");
    let trace = trace_member(&report).expect("validate-only still reports its trace");
    assert_eq!(trace["status"], "ok");
    assert_eq!(trace["finalised"], true);
    assert_eq!(trace["events"], "0");
    let index = index_of(project.path(), trace["run_id"].as_str().unwrap());
    assert!(index.scopes.is_empty(), "validate declares no scope");
}

// ------------------------------------------- 10./11. presentation and fidelity

/// The JSON stream carries exactly one registered root, that root owns the
/// member, and no standalone trace object is ever emitted.
#[test]
fn json_has_one_command_root_owning_the_member_and_no_standalone_object() {
    let user = UserScratch::new();
    let project = trace_support::static_project(&user);
    let output = user
        .vibe()
        .args(["install", "--trace-compile", "--json", "--registry"])
        .arg(fixture_registry())
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
    let docs = documents(&output.stdout);
    let roots: Vec<&Value> = docs
        .iter()
        .filter(|doc| doc["command"] == "install")
        .collect();
    assert_eq!(roots.len(), 1, "exactly one registered root: {docs:#?}");
    assert!(roots[0].get("trace").is_some(), "and it owns the member");
    for doc in &docs {
        assert!(
            doc["command"].is_string(),
            "every document is a registered root or plan preview: {doc:#?}",
        );
        if doc["command"] != "install" {
            assert!(
                doc.get("trace").is_none(),
                "no other document carries a trace member: {doc:#?}",
            );
        }
    }
    // A completed install keeps its historical plan preview BEFORE the root.
    let preview = docs
        .iter()
        .position(|doc| doc["command"] == "install:plan")
        .expect("the deferred plan preview still flushes");
    let root = docs
        .iter()
        .position(|doc| doc["command"] == "install")
        .unwrap();
    assert!(preview < root, "preview first, then the one root");
}

/// Human mode prints ONE trace block with ONE aligned timing table.
#[test]
fn human_prints_one_trace_heading_and_one_table() {
    let user = UserScratch::new();
    let project = trace_support::static_project(&user);
    let output = user
        .vibe()
        .args(["install", "--trace-compile", "--registry"])
        .arg(fixture_registry())
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
    let text = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        text.matches("compile trace:").count(),
        1,
        "one heading, once:\n{text}",
    );
    let headers: Vec<&str> = text
        .lines()
        .filter(|line| line.contains("pass") && line.contains("invocations"))
        .collect();
    assert_eq!(headers.len(), 1, "one table header:\n{text}");
    assert!(
        !text.contains("//?/"),
        "the Windows verbatim prefix is stripped for display:\n{text}",
    );
}

/// The report's timings, path and counts are the run's OWN index summary —
/// same pass names, same order, same invocation counts. Durations are not
/// compared: they are whatever the machine did.
#[test]
fn the_reported_summary_matches_the_runs_own_on_disk_index() {
    let user = UserScratch::new();
    let project = trace_support::static_project(&user);
    let output = user
        .vibe()
        .args(["install", "--trace-compile", "--json", "--registry"])
        .arg(fixture_registry())
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
    let report = trace_support::sole_root(&output.stdout, "install");
    let trace = trace_member(&report).expect("a traced install reports its trace");
    let run_id = trace["run_id"].as_str().unwrap();
    let index = index_of(project.path(), run_id);

    assert_eq!(
        trace["events"].as_str().unwrap(),
        index.events.len().to_string(),
        "the count is lossless and canonical-decimal",
    );
    let reported: Vec<(String, u32)> = trace["timings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["pass"].as_str().unwrap().to_string(),
                u32::try_from(row["invocations"].as_u64().unwrap()).unwrap(),
            )
        })
        .collect();
    let on_disk: Vec<(String, u32)> = index
        .aggregates
        .iter()
        .map(|row| (row.pass.clone(), row.invocations))
        .collect();
    assert_eq!(
        reported, on_disk,
        "same names, same order, same invocation counts",
    );
    let path = trace["run_path"].as_str().unwrap();
    assert!(
        path.ends_with(&format!(".vibe/trace/{run_id}")),
        "the machine path is the exact forward-slashed run directory: {path}",
    );
}
