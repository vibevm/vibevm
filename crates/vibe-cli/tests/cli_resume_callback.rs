//! R3.4 — a SATISFIED slot resume still owes the post-durability callback.
//!
//! Both resume sites used to `return Ok(resumed)` the moment the continuation
//! was serviced. That looked harmless — the parked row had finished, so the
//! command could report — but it skipped everything the callback exists to do:
//!
//! * an authored `phase:install` contribution never ran once a park had been
//!   satisfied, on any resume, ever;
//! * a lifecycle prerequisite never learned the resumed rows, so the run it
//!   went on to dispatch reported work it had genuinely done as work that
//!   never happened.
//!
//! Each test here parks a real hosted `slot:post-install` row, satisfies it the
//! way a hosting agent would, and requires the resumed document to carry BOTH
//! halves: the slot row the continuation finished, and the phase work that
//! follows it. Between them they cover all three completion paths — the fresh
//! fast path in `install/mod.rs`, that same path under a lifecycle
//! prerequisite, and the Ready apply in `install/ready.rs` — so deleting
//! `finish_resumed` from either site fails at least one.
//!
//! No test here edits a selected `vibe.toml` after its park. That is a rule,
//! not an accident: the handler fingerprint reads the selected manifest, so
//! any edit to it invalidates the delegated row's record and the resume
//! re-parks instead of resuming. Every trigger below works on inputs the
//! fingerprint does not read.
//!
//! Every step operates on a temporary project.

mod common;

use std::fs;
use std::path::Path;

use common::agent_provider::{MockProvider, configure_provider};
use common::hosted_slot::{
    PAID_RESULT, documents, lifecycle_state, project_at, publish_slot_agent, write_declared_output,
};
use common::{UserScratch, git_available};

/// The authored host-side rows. `phase:install` is the one a satisfied resume
/// used to skip; `phase:build` proves later phases still run in the same run.
const AUTHORED: &str = "\n[[extension]]\nid = \"authored-install\"\npoint = \"phase:install\"\n\
     handler = { kind = \"builtin\", name = \"log\" }\n\
     config = { message = \"AUTHORED-PHASE-INSTALL\" }\n\
     \n[[extension]]\nid = \"authored-build\"\npoint = \"phase:build\"\n\
     handler = { kind = \"builtin\", name = \"log\" }\n\
     config = { message = \"AUTHORED-PHASE-BUILD\" }\n";

/// Declare `org.demo/tools` as a static requirement AND author the host rows.
///
/// The static declaration is what puts the resume on the FRESH fast path: a
/// post-install park writes the lock before it stops, so the next bare install
/// finds the lock already correct.
fn declare_and_author(project: &Path) {
    let manifest = project.join("vibe.toml");
    let mut text = fs::read_to_string(&manifest).unwrap();
    text.push_str(
        "\n[requires]\npackages = { \"flow:org.demo/tools\" = \
         { version = \"^0.1\", link = \"static\" } }\n",
    );
    text.push_str(AUTHORED);
    fs::write(&manifest, text).unwrap();
}

/// `vibe install` from the declared requirement, under a hosting agent.
fn install_declared(user: &UserScratch, project: &Path) -> std::process::Output {
    user.vibe()
        .args(["install", "--json", "--assume-yes"])
        .args(["--agent-mode", "agent"])
        .arg("--path")
        .arg(project)
        .output()
        .unwrap()
}

/// The one root document of `command` on this stream.
fn root(bytes: &[u8], command: &str) -> serde_json::Value {
    let roots: Vec<serde_json::Value> = documents(bytes)
        .into_iter()
        .filter(|doc| doc["command"] == command)
        .collect();
    assert_eq!(
        roots.len(),
        1,
        "exactly one `{command}` root: {}",
        String::from_utf8_lossy(bytes),
    );
    roots.into_iter().next().unwrap()
}

/// Every contribution row's `(point, key)` pair, for readable assertions.
fn rows(report: &serde_json::Value) -> Vec<(String, String)> {
    report["contributions"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    (
                        row["point"].as_str().unwrap_or_default().to_string(),
                        row["key"].as_str().unwrap_or_default().to_string(),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The position of the first row at `point`, for chronology assertions.
fn index_of_point(ordered: &[(String, String)], point: &str) -> Option<usize> {
    ordered.iter().position(|(row, _)| row == point)
}

fn has_point(report: &serde_json::Value, point: &str) -> bool {
    rows(report).iter().any(|(row, _)| row == point)
}

fn mentions(report: &serde_json::Value, needle: &str) -> bool {
    rows(report).iter().any(|(_, key)| key.contains(needle))
}

fn assert_ok(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "exit {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// A direct install whose post-install slot row parked, then resumed, reports
/// the resumed slot row AND runs the authored `phase:install` contribution.
///
/// The resume lands on the fresh fast path — the park wrote the lock before it
/// stopped — so this is the `install/mod.rs` site. Delete `finish_resumed`
/// there and the `phase:install` row disappears from the resumed document
/// while the slot row remains, which is exactly the defect: a satisfied park
/// silently cancelled the rest of the install.
#[test]
fn a_satisfied_direct_resume_runs_the_authored_install_phase() {
    if !git_available() {
        eprintln!("skipping hosted resume e2e: git not on PATH");
        return;
    }
    let provider = MockProvider::serving(PAID_RESULT);
    let outer = tempfile::tempdir().unwrap();
    let published = publish_slot_agent(outer.path(), "slot:post-install", "0.1.0");
    let user = UserScratch::new();
    let project = project_at(&user, &published.registry);
    declare_and_author(project.path());
    configure_provider(&user, &provider.endpoint());

    // ---- the parking invocation --------------------------------------
    let parked = install_declared(&user, project.path());
    assert_ok(&parked);
    let parked_report = root(&parked.stdout, "install");
    assert!(
        parked_report["delegation"].is_object(),
        "the post-install row parked: {parked_report}",
    );
    assert!(
        !mentions(&parked_report, "authored-install"),
        "and the authored phase row did NOT run before the park: {:?}",
        rows(&parked_report),
    );

    // ---- the hosting agent does the declared work --------------------
    write_declared_output(project.path());

    // ---- the resume ---------------------------------------------------
    let resumed = install_declared(&user, project.path());
    assert_ok(&resumed);
    let report = root(&resumed.stdout, "install");
    assert!(
        report["delegation"].is_null(),
        "the continuation was satisfied, so nothing is delegated now: {report}",
    );
    assert!(
        has_point(&report, "slot:post-install"),
        "the resumed slot row is in the document: {:?}",
        rows(&report),
    );
    assert!(
        mentions(&report, "authored-install"),
        "and so is the authored `phase:install` contribution the resume owed: {:?}",
        rows(&report),
    );
    assert_eq!(
        provider.hits(),
        0,
        "a hosted row is never paid for, on either invocation",
    );
}

/// A lifecycle prerequisite whose install parked, then resumed, carries the
/// resumed rows into the SAME run and goes on to the later phases.
///
/// This is the second half of the defect. `phase.rs` learns the prerequisite's
/// slot rows from the callback context and nowhere else, so a resume that
/// skipped the callback reported a `vibe build` whose install did nothing —
/// and dispatched the later phases under a fresh run rather than the one the
/// park had opened.
#[test]
fn a_satisfied_prerequisite_resume_keeps_its_rows_and_its_run() {
    if !git_available() {
        eprintln!("skipping hosted resume e2e: git not on PATH");
        return;
    }
    let provider = MockProvider::serving(PAID_RESULT);
    let outer = tempfile::tempdir().unwrap();
    let published = publish_slot_agent(outer.path(), "slot:post-install", "0.1.0");
    let user = UserScratch::new();
    let project = project_at(&user, &published.registry);
    declare_and_author(project.path());
    configure_provider(&user, &provider.endpoint());

    let build = |project: &Path| {
        user.vibe()
            .args(["build", "--json", "--assume-yes"])
            .args(["--agent-mode", "agent"])
            .arg("--path")
            .arg(project)
            .output()
            .unwrap()
    };

    // ---- the parking invocation --------------------------------------
    let parked = build(project.path());
    assert_ok(&parked);
    let parked_report = root(&parked.stdout, "lifecycle");
    let parked_run = parked_report["delegation"]["run_id"]
        .as_str()
        .unwrap_or_else(|| panic!("the prerequisite install parked: {parked_report}"))
        .to_string();
    assert!(
        !mentions(&parked_report, "authored-build"),
        "a parked prerequisite stops the chain before the later phases: {:?}",
        rows(&parked_report),
    );

    // ---- the hosting agent does the declared work --------------------
    write_declared_output(project.path());

    // ---- the resume ---------------------------------------------------
    let resumed = build(project.path());
    assert_ok(&resumed);
    let report = root(&resumed.stdout, "lifecycle");
    assert!(
        has_point(&report, "slot:post-install"),
        "the prerequisite's resumed slot row reached THIS document: {:?}",
        rows(&report),
    );
    assert!(
        mentions(&report, "authored-install"),
        "the authored `phase:install` row ran on the resume: {:?}",
        rows(&report),
    );
    assert!(
        mentions(&report, "authored-build"),
        "and the chain went on to the later phases: {:?}",
        rows(&report),
    );
    // The durable state is where "the same run" is decidable: the report has
    // no run id of its own, and the resume must have ADOPTED the parked run
    // rather than begun a second one beside it.
    assert_eq!(
        lifecycle_state(project.path()).run.run_id.as_deref(),
        Some(parked_run.as_str()),
        "all of it in the run the park opened",
    );
    assert!(
        lifecycle_state(project.path())
            .run
            .slot_continuation
            .is_none(),
        "and nothing is owed any more",
    );
    assert_eq!(provider.hits(), 0, "still nothing paid for");
}

/// The READY apply site owes the callback and the common tail.
///
/// Reaching it needs an apply that completes while a slot park is still live,
/// and `LifecycleStateStore::stage_continuation` admits exactly one such
/// shape: the pass's own payload-event target set must be EMPTY, so the
/// adopted continuation is kept rather than overwritten.
///
/// The trigger is a single edit to `vibe.lock`: `meta.root_dependencies` is
/// CLEARED. The selected manifest is not touched at all, and no payload byte
/// moves. From there:
///
/// * `freshness::check` compares the declared root set against
///   `meta.root_dependencies` by SET EQUALITY, so an empty recorded set is
///   stale and planning is `Ready` rather than fresh;
/// * `hold_pins` keeps the still-locked 0.1.0, so the solve lands on the
///   version already materialised;
/// * that slot is present and identical, default `trust-presence` skips it
///   outright, and a skipped slot emits no payload event — so the pass selects
///   nothing and the adopted continuation survives, which is what lets the
///   Ready site service it;
/// * because the manifest never changes, the delegated row's execution
///   fingerprint still matches and its record stays reusable, so the resume
///   satisfies the park instead of re-parking it.
///
/// Two mutations die here: replacing `finish_resumed` with `Ok(resumed.run)`
/// drops the authored phase row, and skipping the shared tail drops the
/// closure-diff document the ordinary Ready path always emits.
#[test]
fn a_satisfied_ready_resume_runs_the_callback_and_the_common_tail() {
    if !git_available() {
        eprintln!("skipping hosted resume e2e: git not on PATH");
        return;
    }
    let provider = MockProvider::serving(PAID_RESULT);
    let outer = tempfile::tempdir().unwrap();
    let published = publish_slot_agent(outer.path(), "slot:post-install", "0.1.0");
    let user = UserScratch::new();
    let project = project_at(&user, &published.registry);
    declare_and_author(project.path());
    configure_provider(&user, &provider.endpoint());

    let parked = install_declared(&user, project.path());
    assert_ok(&parked);
    let parked_report = root(&parked.stdout, "install");
    let parked_run = parked_report["delegation"]["run_id"]
        .as_str()
        .unwrap_or_else(|| panic!("the post-install row parked: {parked_report}"))
        .to_string();
    write_declared_output(project.path());

    // The ONLY change: clear the lock's recorded ROOT-DEPENDENCY SHAPE. The
    // selected manifest is NOT edited here or anywhere in this test, and no
    // payload byte moves. Both halves matter — `meta.root_dependencies` is
    // what `freshness::check` compares by set equality (so clearing it is
    // enough to make planning Ready), and it is an input the handler
    // fingerprint never reads (so the delegated row's record stays reusable
    // and the resume satisfies the park instead of re-parking it).
    //
    // Editing the declared constraint would do neither: `=0.1.0` still
    // satisfies the locked 0.1.0, so freshness stays Fresh, and touching the
    // manifest re-fingerprints the row. That alternative is rejected, not what
    // runs below.
    let lock_path = project.path().join("vibe.lock");
    let mut lockfile = vibe_core::manifest::Lockfile::read(&lock_path).unwrap();
    assert!(
        !lockfile.meta.root_dependencies.is_empty(),
        "the park wrote the lock before it stopped, roots and all",
    );
    lockfile.meta.root_dependencies.clear();
    lockfile.write(&lock_path).unwrap();

    let resumed = install_declared(&user, project.path());
    assert_ok(&resumed);
    let report = root(&resumed.stdout, "install");

    // ---- this really is the Ready/Applied path -------------------------
    assert_eq!(
        report["unchanged"], false,
        "a Ready apply, not the fresh fast path: {report}",
    );
    assert_eq!(
        report["materialised"].as_array().map(Vec::len),
        Some(0),
        "and it rematerialised nothing: {report}",
    );
    let skipped: Vec<&str> = report["skipped"]
        .as_array()
        .unwrap()
        .iter()
        .map(|slot| slot.as_str().unwrap())
        .collect();
    assert!(
        skipped.iter().any(|slot| slot.contains("org.demo.tools")),
        "the present identical slot was skipped, which is why no payload event \
         fired and the continuation survived: {skipped:?}",
    );

    // ---- the resume did its two jobs -----------------------------------
    assert!(
        report["delegation"].is_null() && report["ok"] == true,
        "the continuation was satisfied and the outcome stayed typed: {report}",
    );
    let ordered = rows(&report);
    let slot_at = index_of_point(&ordered, "slot:post-install")
        .unwrap_or_else(|| panic!("the resumed slot row is in the document: {ordered:?}"));
    let phase_at = index_of_point(&ordered, "phase:install")
        .unwrap_or_else(|| panic!("the authored phase row ran: {ordered:?}"));
    assert!(
        slot_at < phase_at,
        "chronology: resumed slot work precedes the phase callback: {ordered:?}",
    );

    // ---- the shared tail ran -------------------------------------------
    assert!(
        documents(&resumed.stdout)
            .iter()
            .any(|doc| doc["command"] == "install:closure-diff"),
        "the common Ready tail emitted its closure diff: {}",
        String::from_utf8_lossy(&resumed.stdout),
    );

    // ---- same run, nothing owed ----------------------------------------
    let state = lifecycle_state(project.path());
    assert_eq!(
        state.run.run_id.as_deref(),
        Some(parked_run.as_str()),
        "the run the park opened is the run that finished",
    );
    assert!(state.run.slot_continuation.is_none(), "nothing owed");
    assert_eq!(provider.hits(), 0, "still nothing paid for");
}
