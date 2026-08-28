//! R3.4 REDs 14/18/23 — hard `slot:post-install` failures under a SCOPED
//! `vibe update` and a `vibe reinstall --force`, plus the secrecy red both
//! share.
//!
//! The failure is the marker-gated sabotage: once armed, the ordered fixture's
//! `later-hard-fail` script breaks `.vibe/lifecycle.toml` so the run's next
//! checkpoint cannot land, prints the exact secret
//! [`HARD_POST_SECRET`] to stderr, and exits 17 — the difference between a
//! plain post-install nonzero (soft, flagged, command stays green) and a hard
//! failure. The sabotage makes each failed project single-use, so every
//! trace-off/trace-on pair seeds SEPARATE projects from one publication.
//!
//! Emission is the historical policy, pinned both ways: these failures were
//! root-SILENT with tracing off and stay silent, and a requested trace makes
//! exactly one root observable — the command's OWN family, never a borrowed
//! install one, and never a transport wrapper on stderr.

mod common;
mod trace_support;

use std::path::Path;

use common::UserScratch;
use common::trace_failure_slot::{
    HARD_POST_SECRET, add_version, arm_hard_post, corrupt_payload, normalise_stderr, project,
    publish_ordered_post_install, seed_untraced,
};
use serde_json::Value;
use trace_support::{
    all_trace_bytes, documents, index_of, run_directories, sole_root, trace_member,
};
use vibe_wire::generated::compiler_trace_index::e1::index::RunStatus;

/// A SCOPED `vibe update org.demo/tools --json`, plus extra flags. The armed
/// sabotage makes it fail.
fn scoped_update(user: &UserScratch, target: &Path, extra: &[&str]) -> std::process::Output {
    let output = user
        .vibe()
        .args(["update", "org.demo/tools", "--json", "--assume-yes"])
        .args(extra)
        .arg("--path")
        .arg(target)
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "the armed hard-post sabotage fails the scoped update",
    );
    output
}

/// A JSON `vibe reinstall --force`, plus extra flags. The armed sabotage over
/// a corrupted payload (so `Verify` re-materialises) makes it fail.
fn forced_reinstall(user: &UserScratch, target: &Path, extra: &[&str]) -> std::process::Output {
    let output = user
        .vibe()
        .args(["reinstall", "--force", "--json", "--assume-yes"])
        .args(extra)
        .arg(target)
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "the armed hard-post sabotage fails the forced reinstall",
    );
    output
}

/// The stream carries NO registered command root at all — the trace-off half
/// of the historical silence. Plan previews are not roots and may be present;
/// a revived per-row `lifecycle` echo is a root and must not be here either.
fn assert_no_registered_roots(docs: &[Value], label: &str) {
    for command in ["update", "install", "reinstall", "lifecycle"] {
        assert!(
            docs.iter().all(|doc| doc["command"] != command),
            "trace off keeps the historical root-silent policy — no `{command}` \
             root, `{label}`: {docs:#?}",
        );
    }
}

/// The ordered pair of slot rows the fixture declares, in their exact order:
/// `earlier-ok` green, then `later-hard-fail` failed with the secret in its
/// captured stderr.
fn assert_ordered_contributions(root: &Value) {
    let rows = root["contributions"]
        .as_array()
        .unwrap_or_else(|| panic!("the slot rows ride the failed root: {root}"));
    assert_eq!(rows.len(), 2, "both declared rows, in order: {root}");
    assert_eq!(rows[0]["point"], "slot:post-install");
    assert_eq!(rows[1]["point"], "slot:post-install");
    assert_eq!(
        rows[0]["reference"], "org.demo/tools#earlier-ok",
        "the earlier builtin declaration ran first: {root}",
    );
    assert_eq!(rows[0]["status"], "ok");
    assert_eq!(
        rows[1]["reference"], "org.demo/tools#later-hard-fail",
        "the sabotaging script declaration ran second: {root}",
    );
    assert_eq!(rows[1]["status"], "fail");
    assert!(
        rows[1].get("flagged").is_none(),
        "the sabotaging row is HARD — a soft post-install nonzero would carry a \
         serialized `flagged` member: {root}",
    );
    assert!(
        rows[1]["stderr"]
            .as_str()
            .is_some_and(|text| text.contains(HARD_POST_SECRET)),
        "the fail row captured the exact sabotage secret — this exact failure: {root}",
    );
}

/// The sabotage oracle of `cli_lifecycle_fatal_outcomes`, applied to a failed
/// twin: `.vibe/lifecycle.toml` is a DIRECTORY after the command, because the
/// armed script replaced the state file so the run's next checkpoint could not
/// land — the state a plain (soft, flagged) post-install nonzero never leaves.
fn assert_checkpoint_sabotaged(project: &Path) {
    assert!(
        project.join(".vibe/lifecycle.toml").is_dir(),
        "the sabotage really fired: the checkpoint state is a directory the run \
         could no longer write, which is what made this failure hard",
    );
}

/// The traced half of every red here: one terminal failed run on disk, the
/// fixed failure word, and real recorded work — nonempty scopes and events.
fn assert_terminal_trace(project: &Path, root: &Value) {
    let trace = trace_member(root).expect("a requested trace rides the root");
    assert_eq!(trace["status"], "failed");
    assert_eq!(trace["finalised"], true);
    let run_id = trace["run_id"].as_str().expect("the member names its run");
    let index = index_of(project, run_id);
    assert!(
        matches!(index.status, RunStatus::Failed),
        "the run's index is terminal failed: {index:?}",
    );
    assert_eq!(
        index.failure.as_deref(),
        Some("command failed"),
        "the index records the FIXED diagnostic and nothing else",
    );
    assert!(
        !index.scopes.is_empty() && !index.events.is_empty(),
        "a failure after real compiles carries its recorded work: {index:?}",
    );
}

/// The off/on failure identity: same exit, same terminal stderr once each
/// twin's own project path is folded away, and no transport wrapper anywhere.
fn assert_same_failure_identity(
    off: &std::process::Output,
    on: &std::process::Output,
    off_project: &Path,
    on_project: &Path,
) {
    assert_eq!(
        off.status.code(),
        on.status.code(),
        "the exit identity is the command's own, not a wrapper's",
    );
    let off_tail = normalise_stderr(off_project, &off.stderr);
    let on_tail = normalise_stderr(on_project, &on.stderr);
    assert_eq!(
        off_tail, on_tail,
        "the terminal error is the ORIGINAL one, unchanged:\n off: {off_tail}\n on:  {on_tail}",
    );
    assert!(
        !on_tail.contains("FailedDraft") && !on_tail.contains("Carried"),
        "the transport carrier never reaches the operator: {on_tail}",
    );
}

/// RED 14 — a scoped update over a published 0.1.1, armed to fail hard at the
/// ordered post-install rows.
///
/// The seed happens while ONLY 0.1.0 exists (a `^0.1` seed after 0.1.1 was
/// published would lock 0.1.1 and leave the update nothing to bump), the bump
/// is published only then, and both twins arm and run. Trace off stays
/// root-silent; trace on emits exactly ONE scoped, failed, incomplete Update
/// root carrying the measured bump, the moved slots, the ordered rows and the
/// terminal trace.
#[test]
fn a_hard_post_failure_scoped_update_off_silent_on_one_failed_update_root() {
    if !common::git_available() {
        eprintln!("skipping scoped-update failure e2e: git not on PATH");
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

    let off = scoped_update(&user, off_project.path(), &[]);
    let on = scoped_update(&user, on_project.path(), &["--trace-compile"]);

    // ---- off: the historical root-silent policy ----------------------------
    let off_docs = documents(&off.stdout);
    assert_no_registered_roots(&off_docs, "off");
    assert!(
        run_directories(off_project.path()).is_empty(),
        "and the silent twin opened no trace run at all",
    );

    // ---- on: exactly one scoped, failed, incomplete Update root ------------
    let on_docs = documents(&on.stdout);
    let root = sole_root(&on.stdout, "update");
    assert!(
        on_docs.iter().all(|doc| {
            doc["command"] != "install"
                && doc["command"] != "reinstall"
                && doc["command"] != "lifecycle"
        }),
        "no borrowed Install or Reinstall root, and no per-row Lifecycle echo: {on_docs:#?}",
    );
    assert_eq!(root["scope"], "scoped");
    assert_eq!(root["ok"], false);
    assert_eq!(
        root["complete"], false,
        "a boundary-measured partial record"
    );
    assert_eq!(
        root["version_bumps"],
        serde_json::json!(["org.demo/tools 0.1.0 -> 0.1.1"]),
        "the measured bump: {root}",
    );
    assert_eq!(
        root["materialised"],
        serde_json::json!([common::slot_dir("org.demo.tools", "0.1.1")]),
        "the new slot really materialised before the failure: {root}",
    );
    assert_eq!(
        root["pruned"],
        serde_json::json!([common::slot_dir("org.demo.tools", "0.1.0")]),
        "and the superseded slot was pruned: {root}",
    );
    assert!(
        root["nodes_regenerated"]
            .as_array()
            .is_some_and(|nodes| !nodes.is_empty()),
        "the regeneration it reached is reported, not dropped: {root}",
    );
    assert_ordered_contributions(&root);
    assert_eq!(
        run_directories(on_project.path()).len(),
        1,
        "one command, one run",
    );
    assert_terminal_trace(on_project.path(), &root);

    // The failure is HARD because the checkpoint was sabotaged — proved on
    // BOTH twins, in the state each of them is left in.
    assert_checkpoint_sabotaged(off_project.path());
    assert_checkpoint_sabotaged(on_project.path());

    assert_same_failure_identity(&off, &on, off_project.path(), on_project.path());
}

/// RED 18 — a `reinstall --force` over a corrupted installed payload, armed
/// to fail hard at the ordered post-install rows.
///
/// The corruption is what makes the run REAL: `Verify` integrity
/// re-materialises the slot instead of trusting the bytes, so the failure
/// happens after a measured materialisation and a regeneration, on the
/// command's own borrowed recorder.
#[test]
fn a_hard_post_failure_forced_reinstall_off_silent_on_one_failed_reinstall_root() {
    if !common::git_available() {
        eprintln!("skipping forced-reinstall failure e2e: git not on PATH");
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

    let off = forced_reinstall(&user, off_project.path(), &[]);
    let on = forced_reinstall(&user, on_project.path(), &["--trace-compile"]);

    // ---- off: the historical root-silent policy ----------------------------
    let off_docs = documents(&off.stdout);
    assert_no_registered_roots(&off_docs, "off");
    assert!(
        run_directories(off_project.path()).is_empty(),
        "and the silent twin opened no trace run at all",
    );

    // ---- on: exactly one failed, incomplete, forced Reinstall root ---------
    let on_docs = documents(&on.stdout);
    let root = sole_root(&on.stdout, "reinstall");
    assert!(
        on_docs.iter().all(|doc| {
            doc["command"] != "install"
                && doc["command"] != "update"
                && doc["command"] != "lifecycle"
        }),
        "no borrowed Install or Update root, and no per-row Lifecycle echo: {on_docs:#?}",
    );
    assert_eq!(root["forced"], true, "the materialisation force");
    assert_eq!(root["ok"], false);
    assert_eq!(
        root["complete"], false,
        "a boundary-measured partial record"
    );
    assert_eq!(
        root["materialised"],
        serde_json::json!([common::slot_dir("org.demo.tools", "0.1.0")]),
        "the re-materialised slot is measured, not defaulted away: {root}",
    );
    assert!(
        root["nodes_regenerated"]
            .as_array()
            .is_some_and(|nodes| !nodes.is_empty()),
        "the regeneration it reached is reported, not dropped: {root}",
    );
    assert_ordered_contributions(&root);
    assert_eq!(
        run_directories(on_project.path()).len(),
        1,
        "one command, one run",
    );
    assert_terminal_trace(on_project.path(), &root);

    // The failure is HARD because the checkpoint was sabotaged — proved on
    // BOTH twins, in the state each of them is left in.
    assert_checkpoint_sabotaged(off_project.path());
    assert_checkpoint_sabotaged(on_project.path());

    assert_same_failure_identity(&off, &on, off_project.path(), on_project.path());
}

/// RED 23 — the secrecy contract of both families, on one traced failure.
///
/// The sabotage's secret legitimately reaches the COMMAND's surfaces — the
/// typed report may carry the captured handler stderr, and the terminal error
/// may quote it — so the red deliberately asserts the secret IS in the fail
/// row (making the no-leak half non-vacuous) before asserting it is NOWHERE in
/// the trace: neither in any byte the trace tree wrote, nor in the serialized
/// member. And the Update root, which has no `notices` member, must not
/// invent one.
#[test]
fn the_hard_post_secret_reaches_the_report_but_never_the_trace() {
    if !common::git_available() {
        eprintln!("skipping trace-secrecy e2e: git not on PATH");
        return;
    }
    let outer = tempfile::tempdir().unwrap();
    let published = publish_ordered_post_install(outer.path());
    let user = UserScratch::new();
    let project = project(&user, &published.registry);
    seed_untraced(&user, project.path());
    add_version(&published, "0.1.1");
    arm_hard_post(project.path());

    let on = scoped_update(&user, project.path(), &["--trace-compile"]);
    let root = sole_root(&on.stdout, "update");
    assert_ordered_contributions(&root);

    // The secret really flowed through this run — the assertion the two below
    // are worthless without.
    assert!(
        root["contributions"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|row| row["stderr"]
                .as_str()
                .is_some_and(|text| text.contains(HARD_POST_SECRET))),
        "the report may legitimately carry the captured handler stderr: {root}",
    );

    // And yet no byte the trace wrote quotes it, and neither does the member.
    assert!(
        !all_trace_bytes(project.path()).contains(HARD_POST_SECRET),
        "the whole trace tree is free of the sabotage secret",
    );
    let trace = trace_member(&root).expect("a requested trace rides the root");
    assert!(
        !serde_json::to_string(trace)
            .unwrap()
            .contains(HARD_POST_SECRET),
        "nor may the member: {trace}",
    );

    // The Update root has no `notices` member and must not invent one.
    assert!(
        !serde_json::to_string(&root).unwrap().contains("notices"),
        "no `notices` key appears on a root that never had one: {root}",
    );
}
