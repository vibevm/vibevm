//! Producer-level reds for the direct-install callback's failure family.
//!
//! `after_direct_install` runs AFTER the install made its world durable. What
//! it does from there — plan the world, surface the plan, dispatch the phase
//! rows — is lifecycle work, so a failure in any of it is a failed phase run
//! that already has rows to report: the slot rows the install produced, and
//! any an older continuation's resume just serviced.
//!
//! These drive the REAL function. The mutation they kill is deleting the
//! classifying wrapper, or the prefix it prepends: without either, a planning
//! refusal arrives bare at the command boundary, takes the generic INSTALL
//! fallback draft, and reports a run that did nothing — after the slot rows
//! below had already run and been measured.

use vibe_lifecycle::RunMetadata;
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
use vibe_workspace::Workspace;

use super::*;
use crate::cli::AgentModeArg;
use crate::commands::compile_trace::{RegisteredReportDraft, uncarry};
use crate::output;

/// A project that REQUIRES a package its lockfile does not carry.
///
/// After the install barrier that is a malformed durable world, and world
/// collection refuses it by name — a deterministic planning failure that needs
/// no registry, no network and no handler.
fn project_requiring_an_unlocked_package() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname = \"demo\"\ngroup = \"org.demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.demo/tools\" = \"^1.0\"\n",
    )
    .unwrap();
    dir
}

fn metadata(root: &std::path::Path) -> RunMetadata {
    RunMetadata {
        requested: "install".to_string(),
        chain: vec!["validate".to_string(), "install".to_string()],
        offline: true,
        assume_yes: true,
        agent_mode: RunAgentMode::Cli,
        force: false,
        trace_compile: false,
        run_id: vibe_lifecycle::process::allocate_run_id(root).unwrap(),
        started: crate::commands::init::current_timestamp_utc(),
        selected: ".".into(),
    }
}

/// One slot row, of the shape an apply's slot lifecycle really produces.
fn slot_row() -> vibe_install::SlotLifecycleReport {
    vibe_install::SlotLifecycleReport {
        key: "org.demo/tools#slot-log@slot(org.demo/tools@1.0.0)".to_string(),
        reference: "org.demo/tools#slot-log".to_string(),
        slot_target: None,
        point: "slot:post-install".to_string(),
        provider: "org.demo/tools".to_string(),
        handler: "builtin".to_string(),
        tier: "dependency".to_string(),
        version: Some("1.0.0".to_string()),
        status: "ok".to_string(),
        flagged: false,
        message: None,
        stdout: None,
        stderr: None,
        stdout_truncated: false,
        stderr_truncated: false,
    }
}

fn quiet_ctx() -> output::Context {
    output::Context::from_flags(true, false, None, true, AgentModeArg::Cli)
}

/// A planning refusal after measured slot rows arrives CARRIED, in this
/// command's own lifecycle family, with those rows in front of it.
///
/// Nothing about a world-collection failure is install-shaped: the install
/// already finished. Before the wrapper existed this error left the callback
/// bare, and the install boundary's generic fallback reported an Install root
/// with an empty progress and no rows at all.
#[test]
fn a_planning_refusal_after_slot_rows_carries_them_in_the_lifecycle_family() {
    let project = project_requiring_an_unlocked_package();
    let path = crate::commands::install::resolve_project_root(project.path()).unwrap();
    let workspace = Workspace::discover(&path).expect("the project loads");
    let run = InstallRunContext {
        lease: std::sync::Arc::new(
            vibe_lifecycle::LifecycleLease::acquire(&workspace.root)
                .expect("the fixture workspace is leasable"),
        ),
        metadata: metadata(&path),
        lifecycle_run: None,
        lifecycle_reports: vec![slot_row()],
    };

    let error = after_direct_install(
        &quiet_ctx(),
        &path,
        InstallDisposition::Applied,
        run,
        &workspace,
    )
    .expect_err("the world cannot be collected");

    let carried = uncarry(error)
        .unwrap_or_else(|error| panic!("a post-durability failure must be CARRIED: {error:#}"));
    assert!(
        format!("{:#}", carried.original).contains("absent from effective-world lock"),
        "the original planning error travels unchanged: {:#}",
        carried.original,
    );
    assert!(
        !carried.emit_when_trace_disabled,
        "and with the historical silence of a stage that emitted no document",
    );

    let RegisteredReportDraft::Lifecycle(draft) = carried.draft else {
        panic!("a post-durability stage failure is LIFECYCLE-shaped, not install-shaped");
    };
    assert!(!draft.ok, "a failed run");
    assert_eq!(
        draft.contributions.len(),
        1,
        "the slot row measured before the refusal came back with it: {:?}",
        draft.contributions,
    );
    assert_eq!(draft.contributions[0].point, "slot:post-install");
    assert_eq!(draft.contributions[0].status, "ok");
    assert_eq!(
        draft.steps.len(),
        1,
        "and exactly one failed step, for the phase it stopped at",
    );
    assert_eq!(draft.steps[0].phase, "install");
    assert_eq!(draft.steps[0].status, "fail");
}

/// With NO slot rows the same refusal is still lifecycle-shaped — the family
/// is decided by the stage, not by whether it had anything to report.
///
/// This pins the empty-prefix path, where `prepend_lifecycle_rows` returns
/// early and the wrapper's own draft is the only thing that can carry the
/// family.
#[test]
fn a_planning_refusal_with_no_rows_is_still_lifecycle_shaped() {
    let project = project_requiring_an_unlocked_package();
    let path = crate::commands::install::resolve_project_root(project.path()).unwrap();
    let workspace = Workspace::discover(&path).expect("the project loads");
    let run = InstallRunContext {
        lease: std::sync::Arc::new(
            vibe_lifecycle::LifecycleLease::acquire(&workspace.root)
                .expect("the fixture workspace is leasable"),
        ),
        metadata: metadata(&path),
        lifecycle_run: None,
        lifecycle_reports: Vec::new(),
    };

    let error = after_direct_install(
        &quiet_ctx(),
        &path,
        InstallDisposition::Applied,
        run,
        &workspace,
    )
    .expect_err("the world cannot be collected");
    let carried = uncarry(error).unwrap_or_else(|error| panic!("still carried: {error:#}"));
    assert!(matches!(carried.draft, RegisteredReportDraft::Lifecycle(_)));
    assert!(
        format!("{:#}", carried.original).contains("absent from effective-world lock"),
        "with the same untouched original",
    );
}
