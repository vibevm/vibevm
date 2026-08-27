//! Reds for the resume seam's typed outcome.
//!
//! The whole point of the sum is that a failure is a VALUE carrying rows, so
//! this pins what that value contains. Every production caller matches the sum
//! exhaustively; there is deliberately no convenience accessor that could turn
//! a failed resume into "nothing was owed" by being the only branch somebody
//! wrote.

use super::*;

fn rows() -> Vec<vibe_install::SlotLifecycleReport> {
    vec![vibe_install::SlotLifecycleReport {
        key: "org.demo/tools#slot:post-install".into(),
        point: "slot:post-install".into(),
        handler: "builtin".into(),
        provider: "org.demo/tools".into(),
        tier: "dependency".into(),
        status: "ok".into(),
        message: None,
        version: None,
        reference: "spec://org.demo/tools".into(),
        flagged: false,
        stdout: None,
        stderr: None,
        stdout_truncated: false,
        stderr_truncated: false,
        slot_target: None,
    }]
}

#[derive(Debug, thiserror::Error)]
#[error("the resumed row refused")]
struct Sentinel;

fn failed() -> ResumeOutcome {
    ResumeOutcome::Failed(ResumeFailure {
        original: anyhow::Error::new(Sentinel).context("finishing the parked slot run"),
        progress: vibe_install::InstallProgress::fresh(vec![".".into()]),
        reports: rows(),
        packages_resolved: 3,
    })
}

/// The failure arm really carries what the region measured: the rows, the
/// durable progress it inherited, and the ORIGINAL error object.
///
/// Deleting the capture in `resume_slot_continuation` — returning `Err` from
/// the region instead — empties both lists here and loses the downcast
/// identity the exit code is read from.
#[test]
fn a_failed_resume_carries_its_rows_progress_and_exact_error() {
    let ResumeOutcome::Failed(failure) = failed() else {
        panic!("the failure arm");
    };
    assert_eq!(failure.reports.len(), 1, "the resumed row survived");
    assert_eq!(failure.reports[0].point, "slot:post-install");
    assert_eq!(
        failure.progress.nodes_regenerated,
        ["."],
        "with the durable progress the resume inherited",
    );
    assert_eq!(
        failure.packages_resolved, 3,
        "and the count the serviced run resolved, so no outer report invents a zero",
    );
    assert!(
        failure.original.downcast_ref::<Sentinel>().is_some(),
        "the typed variant the exit code is read from must survive",
    );
    assert_eq!(
        format!("{:#}", failure.original),
        "finishing the parked slot run: the resumed row refused",
        "context is neither stripped nor re-added",
    );
}

/// The neutral transport is exact: every field comes back, and the error keeps
/// its downcast identity.
///
/// `carry_resume_failure` is not a report decision — it is the seam that lets
/// the measurement reach the ONE outer command that can name a family. If the
/// wrapper lost a field, each of the three consumers below it would silently
/// report a default instead.
#[test]
fn the_neutral_transport_round_trips_every_measured_field() {
    let ResumeOutcome::Failed(failure) = failed() else {
        panic!("the failure arm");
    };
    let carried = carry_resume_failure(failure);
    // It rides `anyhow` as an ordinary error, and its Display is the original's
    // — so even the impossible path where one escaped un-taken would print the
    // right words rather than a type name.
    assert_eq!(
        format!("{carried:#}"),
        "finishing the parked slot run: the resumed row refused",
    );

    let taken = take_resume_failure(carried)
        .unwrap_or_else(|error| panic!("the transport is exact and must come back out: {error:#}"));
    assert_eq!(taken.reports.len(), 1);
    assert_eq!(taken.reports[0].point, "slot:post-install");
    assert_eq!(taken.progress.nodes_regenerated, ["."]);
    assert_eq!(taken.packages_resolved, 3);
    assert!(taken.original.downcast_ref::<Sentinel>().is_some());
}

/// An error that is NOT a transported resume failure passes through untouched,
/// so a consumer can branch without consuming what it may need to pass on.
#[test]
fn an_unrelated_error_is_returned_exactly_as_it_arrived() {
    let error = take_resume_failure(anyhow::Error::new(Sentinel).context("planning"))
        .expect_err("not a resume failure");
    assert_eq!(format!("{error:#}"), "planning: the resumed row refused");
    assert!(error.downcast_ref::<Sentinel>().is_some());
}

/// The PRODUCTION capture, driven with its destructive dependency injected.
///
/// This is the exact `Result<ResumeOutcome> -> ResumeOutcome` conversion
/// `resume_slot_continuation` performs. Constructing a `ResumeOutcome::Failed`
/// by hand — as the reds above do, to pin the transport — proves nothing about
/// it: deleting the capture and returning empty rows, default progress and a
/// zero count would leave every one of those green.
///
/// The take is a closure with a COUNTER because it is destructive. `take_reports`
/// empties the lifecycle, so an eager call would strip a SUCCESSFUL resume of
/// the rows its completed arm is about to report — a failure mode no assertion
/// on the failure arm could ever see.
#[test]
fn the_production_capture_takes_once_on_failure_and_never_on_success() {
    let takes = std::cell::Cell::new(0);
    let take = || {
        takes.set(takes.get() + 1);
        vec![
            row_named("resumed:ok", "ok"),
            row_named("resumed:fail", "fail"),
        ]
    };
    let progress = vibe_install::InstallProgress {
        complete: true,
        fresh: false,
        materialised: vec!["vibedeps/org.demo.tools/0.2.0".into()],
        skipped: Vec::new(),
        pruned: Vec::new(),
        nodes_regenerated: vec![".".into()],
    };

    let captured = capture(
        Err(anyhow::Error::new(Sentinel).context("finishing the parked slot run")),
        progress,
        7,
        take,
    );
    assert_eq!(takes.get(), 1, "exactly one destructive take on failure");

    let ResumeOutcome::Failed(failure) = captured else {
        panic!("an Err becomes the failure arm");
    };
    assert_eq!(
        failure
            .reports
            .iter()
            .map(|row| row.key.as_str())
            .collect::<Vec<_>>(),
        ["resumed:ok", "resumed:fail"],
        "the take's rows, in order — an empty list is the defect this refuses",
    );
    assert_eq!(
        failure.progress.materialised,
        ["vibedeps/org.demo.tools/0.2.0"],
        "and the caller's real progress, not a default",
    );
    assert_eq!(failure.progress.nodes_regenerated, ["."]);
    assert!(failure.progress.complete);
    assert_eq!(failure.packages_resolved, 7, "and the carried count");
    assert!(
        failure.original.downcast_ref::<Sentinel>().is_some(),
        "the error is MOVED, not rebuilt: the exit code downcasts through it",
    );
    assert_eq!(
        format!("{:#}", failure.original),
        "finishing the parked slot run: the resumed row refused",
        "context is neither stripped nor re-added",
    );
}

/// The other half of the same law: a SUCCESSFUL region takes nothing, and its
/// value crosses unchanged.
///
/// Both success shapes are driven. `Nothing` alone would stay green under a
/// mutation that took only on `Completed` — and `Completed` is precisely the
/// arm where an eager take is fatal, because the rows it would empty are the
/// ones the completed arm is about to report.
#[test]
fn the_production_capture_never_takes_on_a_successful_region() {
    let takes = std::cell::Cell::new(0);
    let count = || {
        takes.set(takes.get() + 1);
        vec![row_named("must-not-be-taken", "ok")]
    };

    let captured = capture(
        Ok(ResumeOutcome::Nothing),
        vibe_install::InstallProgress::default(),
        7,
        count,
    );
    assert!(matches!(captured, ResumeOutcome::Nothing));
    assert_eq!(takes.get(), 0, "nothing owed, nothing taken");

    // And the arm a `Completed`-only take would corrupt.
    let mut run = crate::commands::install::InstallRun::new(
        std::path::PathBuf::from("/p"),
        InstallDisposition::Fresh,
    );
    run.slot_reports = vec![row_named("resumed:a", "ok"), row_named("resumed:b", "ok")];
    run.progress = vibe_install::InstallProgress::fresh(vec![".".into()]);
    let captured = capture(
        Ok(ResumeOutcome::Completed(Box::new(ResumedInstall {
            run,
            context: InstallRunContext {
                metadata: completed_metadata(),
                lifecycle_run: None,
                lifecycle_reports: Vec::new(),
            },
        }))),
        vibe_install::InstallProgress::default(),
        7,
        count,
    );
    assert_eq!(takes.get(), 0, "a COMPLETED region takes nothing either");
    let ResumeOutcome::Completed(resumed) = captured else {
        panic!("the completed value crosses unchanged");
    };
    assert_eq!(
        resumed
            .run
            .slot_reports
            .iter()
            .map(|row| row.key.as_str())
            .collect::<Vec<_>>(),
        ["resumed:a", "resumed:b"],
        "with its own rows, neither replaced by the take nor reordered",
    );
    assert_eq!(resumed.run.progress.nodes_regenerated, ["."]);
}

fn completed_metadata() -> RunMetadata {
    RunMetadata {
        requested: "install".into(),
        chain: vec!["validate".into(), "install".into()],
        offline: true,
        assume_yes: true,
        agent_mode: vibe_wire::generated::lifecycle::e1::context::RunAgentMode::Cli,
        force: false,
        trace_compile: false,
        run_id: "0".repeat(32),
        started: "2026-08-27T00:00:00Z".into(),
    }
}

fn row_named(key: &str, status: &str) -> vibe_install::SlotLifecycleReport {
    vibe_install::SlotLifecycleReport {
        key: key.into(),
        point: "slot:post-install".into(),
        handler: "builtin".into(),
        provider: "org.demo/tools".into(),
        tier: "dependency".into(),
        status: status.into(),
        message: None,
        version: None,
        reference: "spec://org.demo/tools".into(),
        flagged: false,
        stdout: None,
        stderr: None,
        stdout_truncated: false,
        stderr_truncated: false,
        slot_target: None,
    }
}

/// A continuation never crosses an identity boundary. Adoption preserves the
/// run id; a displaced or legacy/missing owner must leave the old work alone.
#[test]
fn only_the_exact_run_identity_owns_a_continuation() {
    assert!(owns_continuation(Some("run-a"), "run-a"));
    assert!(!owns_continuation(Some("run-a"), "run-b"));
    assert!(!owns_continuation(None, "run-b"));
}
