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
use crate::failure::{Measurement, take};
use crate::install::InstallRunContext;
use crate::ports::RunObserver;

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
        started: vibe_core::timestamp::now_utc(),
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

/// The stage narrates nothing here; only the measurement is under test.
struct SilentObserver;

impl RunObserver for SilentObserver {
    fn stream_mode(&self) -> vibe_lifecycle::process::StreamMode {
        vibe_lifecycle::process::StreamMode::Null
    }

    fn binary_quiet(&self) -> bool {
        true
    }

    fn emit_machine_failure(&self) -> bool {
        false
    }

    fn observe_plan(
        &self,
        _plan: &crate::RitualPlan,
        _metadata: &RunMetadata,
        _emit_empty: bool,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn observe_contribution(
        &self,
        _report: &vibe_wire::generated::lifecycle_report::LifecycleContributionReport,
    ) {
    }

    fn observe_untracked_failure(
        &self,
        _metadata: &RunMetadata,
        _phase: &str,
        _contributions: &[vibe_wire::generated::lifecycle_report::LifecycleContributionReport],
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

/// No agent row is planned by these fixtures, so the refusing default proves
/// the injected backend is never reached.
fn agent() -> std::sync::Arc<dyn vibe_lifecycle::AgentBackend> {
    std::sync::Arc::new(vibe_lifecycle::NoAgentBackend)
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
    let path = crate::install::resolve_project_root(project.path()).unwrap();
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

    let error = after_durable_world_stage(&SilentObserver, &path, run, &workspace, &agent())
        .expect_err("the world cannot be collected");

    let carried = take(error)
        .unwrap_or_else(|error| panic!("a post-durability failure must be MEASURED: {error:#}"));
    assert!(
        format!("{:#}", carried.original).contains("absent from effective-world lock"),
        "the original planning error travels unchanged: {:#}",
        carried.original,
    );
    assert!(
        !carried.emit_machine_failure,
        "and with the historical silence of a stage that emitted no document",
    );

    let Measurement::Lifecycle {
        rows,
        stopped_phase,
        ..
    } = carried.measurement
    else {
        panic!("a post-durability stage failure is LIFECYCLE-shaped, not slot-shaped");
    };
    assert_eq!(
        rows.len(),
        1,
        "the slot row measured before the refusal came back with it: {rows:?}",
    );
    assert_eq!(rows[0].point, "slot:post-install");
    assert_eq!(rows[0].status, "ok");
    assert_eq!(
        stopped_phase, "install",
        "and exactly the phase this stage belongs to",
    );
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
    let path = crate::install::resolve_project_root(project.path()).unwrap();
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

    let error = after_durable_world_stage(&SilentObserver, &path, run, &workspace, &agent())
        .expect_err("the world cannot be collected");
    let carried = take(error).unwrap_or_else(|error| panic!("still measured: {error:#}"));
    assert!(matches!(carried.measurement, Measurement::Lifecycle { .. }));
    assert!(
        format!("{:#}", carried.original).contains("absent from effective-world lock"),
        "with the same untouched original",
    );
}

/// A FOREIGN lease refuses the post-durability stage before it plans anything.
///
/// This entry point takes `path`, `workspace`, `run.lease` and
/// `run.metadata.selected` as four independent values, and everything past the
/// gate writes: the plan is collected over the tree, the dispatch begins (or
/// continues) a run whose state store is rooted at the LEASE, and the
/// package-skill pass rebinds under it.
///
/// The mutation this kills is deleting `ensure_root` from the stage. Without it
/// the fixture reaches world collection and fails with the PLANNING error
/// instead — a different message entirely, and one that arrives only after the
/// stage has already read the tree it had no lease for.
#[test]
fn a_foreign_lease_refuses_the_post_durability_stage_before_planning() {
    let project = project_requiring_an_unlocked_package();
    let path = crate::install::resolve_project_root(project.path()).unwrap();
    let workspace = Workspace::discover(&path).expect("the project loads");
    // A lease over a DIFFERENT tree entirely.
    let elsewhere = project_requiring_an_unlocked_package();
    let foreign = crate::install::resolve_project_root(elsewhere.path()).unwrap();
    let run = InstallRunContext {
        lease: std::sync::Arc::new(
            vibe_lifecycle::LifecycleLease::acquire(&foreign).expect("leasable"),
        ),
        metadata: metadata(&path),
        lifecycle_run: None,
        lifecycle_reports: Vec::new(),
    };

    let error = after_durable_world_stage(&SilentObserver, &path, run, &workspace, &agent())
        .expect_err("a foreign lease can never reach the world stage");

    let carried = take(error).unwrap_or_else(|error| panic!("still measured: {error:#}"));
    let rendered = format!("{:#}", carried.original);
    assert!(
        carried
            .original
            .downcast_ref::<vibe_lifecycle::LifecycleLeaseError>()
            .is_some(),
        "the refusal is the lease's own typed error: {rendered}",
    );
    assert!(
        rendered.contains("at the post-durability world stage"),
        "and it names the boundary it fired at: {rendered}",
    );
    assert!(
        !rendered.contains("absent from effective-world lock"),
        "and it fired BEFORE world collection, which would have failed differently: {rendered}",
    );
}

/// The selected-node twin of the gate above.
///
/// The root alone is not enough: two members of one workspace share a root, and
/// a stage whose recorded selected node disagrees with the tree it was handed
/// would plan one node's world and record it under another's identity.
#[test]
fn a_selected_node_mismatch_refuses_the_post_durability_stage() {
    let project = project_requiring_an_unlocked_package();
    let path = crate::install::resolve_project_root(project.path()).unwrap();
    let workspace = Workspace::discover(&path).expect("the project loads");
    let mut wrong = metadata(&path);
    // The tree really maps this root to `"."`; the run claims a member.
    wrong.selected = "members/other".to_string();
    let run = InstallRunContext {
        lease: std::sync::Arc::new(
            vibe_lifecycle::LifecycleLease::acquire(&workspace.root).expect("leasable"),
        ),
        metadata: wrong,
        lifecycle_run: None,
        lifecycle_reports: Vec::new(),
    };

    let error = after_durable_world_stage(&SilentObserver, &path, run, &workspace, &agent())
        .expect_err("a selected-node mismatch can never reach the world stage");
    let carried = take(error).unwrap_or_else(|error| panic!("still measured: {error:#}"));
    assert!(
        carried
            .original
            .downcast_ref::<vibe_lifecycle::LifecycleLeaseError>()
            .is_some(),
        "the refusal is the lease's typed error: {:#}",
        carried.original,
    );
}
