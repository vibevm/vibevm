//! R3.4 REDs 7/17 — a WHOLE `vibe update` whose Ready apply dies at a hard
//! `slot:pre-install` failure.
//!
//! The root family is a property of the site that MEASURED the failure, not of
//! the command that was typed: the install substrate's Ready apply froze an
//! Install-shaped carrier, so `vibe update --all` reports exactly one INSTALL
//! root — in BOTH trace modes — and never an Update one. This is Ready
//! `SlotFailed`, not a resume failure: the neutral resume path would take the
//! update family and the historical silence, and either of those drifts is
//! exactly what the off/on twins here pin out.
//!
//! The two twins run on INDEPENDENT projects: the sabotage-free fixture fails
//! identically either way, but the trace-on invocation owns a run directory,
//! and folding two owners into one project would make "exactly one run" prove
//! nothing.

mod common;
mod trace_support;

use std::path::Path;

use common::UserScratch;
use common::trace_failure_slot::{
    PRE_INSTALL_SECRET, normalise_json_paths, normalise_stderr, project,
    publish_pre_install_failure,
};
use serde_json::Value;
use trace_support::{documents, index_of, run_directories, sole_run, trace_member};
use vibe_wire::generated::compiler_trace_index::e1::index::RunStatus;

/// `vibe update --all --json --assume-yes`, plus extra flags, against one
/// freshly-wired project. The command FAILS: a hard pre-install slot row.
fn whole_update(
    user: &UserScratch,
    registry: &Path,
    extra: &[&str],
) -> (std::process::Output, tempfile::TempDir) {
    let project = project(user, registry);
    let output = user
        .vibe()
        .args(["update", "--all", "--json", "--assume-yes"])
        .args(extra)
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "a failing pre-install slot row fails the whole update",
    );
    (output, project)
}

/// The one Install root of a stream, proved unique and proved to be the
/// command's FINAL document.
fn sole_install_root(docs: &[Value]) -> Value {
    let mut roots = docs.iter().filter(|doc| doc["command"] == "install");
    let root = roots
        .next()
        .unwrap_or_else(|| panic!("one Install root: {docs:#?}"))
        .clone();
    assert!(
        roots.next().is_none(),
        "exactly one Install root: {docs:#?}",
    );
    assert_eq!(
        docs.last().unwrap()["command"],
        "install",
        "the root is the stream's final document: {docs:#?}",
    );
    root
}

/// RED 7 — the failure keeps the Install root family both ways, differing by
/// the trace member alone.
#[test]
fn whole_update_pre_install_failure_keeps_the_install_root_family_both_ways() {
    if !common::git_available() {
        eprintln!("skipping whole-update slot-failure e2e: git not on PATH");
        return;
    }
    let outer = tempfile::tempdir().unwrap();
    let published = publish_pre_install_failure(outer.path());
    let user = UserScratch::new();

    let (off, off_project) = whole_update(&user, &published.registry, &[]);
    let (on, on_project) = whole_update(&user, &published.registry, &["--trace-compile"]);

    // ---- both streams: one failed Install root, zero Update roots ----------
    let off_docs = documents(&off.stdout);
    let on_docs = documents(&on.stdout);
    let off_root = sole_install_root(&off_docs);
    let on_root = sole_install_root(&on_docs);
    for (label, docs) in [("off", &off_docs), ("on", &on_docs)] {
        assert!(
            docs.iter().all(|doc| doc["command"] != "update"),
            "zero Update roots, `{label}`: {docs:#?}",
        );
        assert!(
            docs.iter().all(|doc| doc["command"] != "lifecycle"),
            "and no Lifecycle root beside it, `{label}`: {docs:#?}",
        );
    }
    assert_eq!(off_root["ok"], false, "a failure root says so");
    assert_eq!(on_root["ok"], false);
    for (label, root) in [("off", &off_root), ("on", &on_root)] {
        let failed = root["contributions"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|row| row["point"] == "slot:pre-install")
            .unwrap_or_else(|| panic!("the failed slot row rides the root, `{label}`: {root}"));
        assert_eq!(
            failed["status"], "fail",
            "the pre-install row is the failure, `{label}`: {root}",
        );
        assert!(
            failed["stderr"]
                .as_str()
                .is_some_and(|text| text.contains(PRE_INSTALL_SECRET)),
            "the row captured the script's sentinel, `{label}`: {root}",
        );
    }

    // ---- the ONLY difference a trace request makes is the member -----------
    assert!(
        off_root.get("trace").is_none(),
        "trace off carries no member: {off_root}",
    );
    assert!(
        !serde_json::to_string(&off_root)
            .unwrap()
            .contains("\"trace\""),
        "the key is absent from the wire, not merely null: {off_root}",
    );
    let trace = trace_member(&on_root).expect("a requested trace rides the root");
    assert_eq!(trace["status"], "failed", "the run finalised failed");
    assert_eq!(trace["finalised"], true);
    let run_id = sole_run(on_project.path(), &on_root);
    let index = index_of(on_project.path(), &run_id);
    assert!(matches!(index.status, RunStatus::Failed));
    assert!(
        index.scopes.is_empty(),
        "a pre-install failure compiles nothing, so zero scopes: {index:?}",
    );
    assert!(
        run_directories(off_project.path()).is_empty(),
        "the trace-off twin opened no run at all",
    );

    // ---- roots equal once the member is removed and the paths folded -------
    let mut on_stripped = on_root;
    on_stripped
        .as_object_mut()
        .expect("an object")
        .remove("trace");
    let mut off_normalised = off_root.clone();
    normalise_json_paths(&mut off_normalised, off_project.path());
    normalise_json_paths(&mut on_stripped, on_project.path());
    assert_eq!(
        off_normalised, on_stripped,
        "the trace member is the ONLY difference between the twins' roots — \
         with every nested `slot_target.root` path folded to `<root>`, the \
         comparison still covers the COMPLETE structured surface",
    );

    // ---- and the failure identity never moved ------------------------------
    assert_eq!(
        off.status.code(),
        on.status.code(),
        "the exit code does not move because an observer was asked for",
    );
    let off_tail = normalise_stderr(off_project.path(), &off.stderr);
    let on_tail = normalise_stderr(on_project.path(), &on.stderr);
    assert_eq!(
        off_tail, on_tail,
        "nor does the terminal error:\n off: {off_tail}\n on:  {on_tail}",
    );
    assert!(
        !on_tail.contains("FailedDraft") && !on_tail.contains("Carried"),
        "the transport carrier never reaches the operator: {on_tail}",
    );
}

/// RED 17 — the failure FLUSHES both plan previews before that root, in both
/// trace modes, and the two streams carry the identical command vector.
///
/// These are the checks that go red if the funnel's plan disposition is
/// deleted (no preview flushes at all) or demoted to the park one (`Discard`
/// drops them): both preview documents are asserted PRESENT, CONTENTFUL and
/// BEFORE the final root, on both sides of the trace flag.
#[test]
fn a_failing_whole_update_flushes_both_plan_previews_before_the_root_in_both_modes() {
    if !common::git_available() {
        eprintln!("skipping whole-update plan-flush e2e: git not on PATH");
        return;
    }
    let outer = tempfile::tempdir().unwrap();
    let published = publish_pre_install_failure(outer.path());
    let user = UserScratch::new();

    let (off, _off_project) = whole_update(&user, &published.registry, &[]);
    let (on, _on_project) = whole_update(&user, &published.registry, &["--trace-compile"]);
    let off_docs = documents(&off.stdout);
    let on_docs = documents(&on.stdout);

    // The streams agree on WHAT they are: the same commands, in the same
    // order, on both sides of the trace flag.
    let commands = |docs: &[Value]| -> Vec<String> {
        docs.iter()
            .map(|doc| doc["command"].as_str().unwrap_or("").to_string())
            .collect()
    };
    assert_eq!(
        commands(&off_docs),
        commands(&on_docs),
        "the command vector is identical off and on",
    );

    for (label, docs) in [("off", &off_docs), ("on", &on_docs)] {
        let root_position = docs
            .iter()
            .position(|doc| doc["command"] == "install")
            .expect("the final root positions the previews against");

        // The resolution preview, naming the package it resolved.
        let install_plan = docs
            .iter()
            .position(|doc| doc["command"] == "install:plan")
            .unwrap_or_else(|| panic!("the install plan preview flushes, `{label}`: {docs:#?}"));
        assert!(
            docs[install_plan]["packages"]
                .as_array()
                .is_some_and(|rows| rows.iter().any(|row| {
                    row["version"] == "0.1.0"
                        && row["package"]
                            .as_str()
                            .is_some_and(|package| package.ends_with("tools"))
                })),
            "the preview names the resolved slot package, `{label}`: {}",
            docs[install_plan],
        );

        // The lifecycle preview, carrying the slot row that is about to fail.
        let lifecycle_plan = docs
            .iter()
            .position(|doc| doc["command"] == "lifecycle:plan")
            .unwrap_or_else(|| panic!("the lifecycle plan preview flushes, `{label}`: {docs:#?}"));
        assert!(
            docs[lifecycle_plan]["contributions"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|row| row["point"] == "slot:pre-install"),
            "the planned ritual includes the slot row, `{label}`: {}",
            docs[lifecycle_plan],
        );

        // EVERY plan preview lands before the final root — a flush displaced
        // after the root, or discarded, is the mutation this red exists to
        // catch.
        for (position, doc) in docs.iter().enumerate() {
            if doc["command"]
                .as_str()
                .is_some_and(|command| command.ends_with(":plan"))
            {
                assert!(
                    position < root_position,
                    "every plan preview precedes the root, `{label}`: {docs:#?}",
                );
            }
        }
    }
}
