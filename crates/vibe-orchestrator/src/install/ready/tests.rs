//! Freeze the Ready-resume row merge — on ALL THREE arms.
//!
//! A serviced continuation arrives with rows of its own, and the apply that
//! serviced it has rows of its own too. The merge is one line of ordering and,
//! on the completed arm, two carriers: exactly the shape that goes wrong
//! quietly. Reverse it and the document tells the operator the older park
//! happened first; update one carrier and the report and the callback describe
//! different runs; handle only the completed arm and a FAILED resume reports a
//! run with no apply in it.
//!
//! That last one is why every arm is driven here, through the one production
//! [`join_applied_rows`] the Ready match calls. A helper tested only on the
//! completed arm stayed green while the failed arm did its own thing inline.
//!
//! The completed carriers start with DIFFERENT resumed rows on purpose. That is
//! what makes "updated only one" and "copied the wrong vector into both"
//! separately visible — with identical rows on both sides, either mistake would
//! still produce a plausible-looking answer.

use vibe_lifecycle::RunMetadata;
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;

use super::*;
use crate::failure::{MeasuredFailure, Measurement};

/// A slot row identified only by its key — everything else is fixed, so an
/// assertion failure prints the ordering rather than a wall of fields.
fn row(key: &str) -> vibe_install::SlotLifecycleReport {
    vibe_install::SlotLifecycleReport {
        key: key.to_string(),
        reference: format!("org.demo/tools#{key}"),
        slot_target: None,
        point: "slot:post-install".to_string(),
        provider: "org.demo/tools".to_string(),
        handler: "builtin".to_string(),
        tier: "dependency".to_string(),
        version: Some("0.1.0".to_string()),
        status: "ok".to_string(),
        flagged: false,
        message: None,
        stdout: None,
        stderr: None,
        stdout_truncated: false,
        stderr_truncated: false,
    }
}

fn keys(rows: &[vibe_install::SlotLifecycleReport]) -> Vec<&str> {
    rows.iter().map(|row| row.key.as_str()).collect()
}

#[derive(Debug, thiserror::Error)]
#[error("the resumed row refused")]
struct Sentinel;

/// A serviced continuation carrying one row in each carrier, deliberately
/// different from each other.
fn resumed() -> ResumeOutcome {
    let mut run = InstallRun::new(
        std::path::PathBuf::from("/demo"),
        InstallDisposition::Applied,
    );
    run.slot_reports = vec![row("resumed-in-run")];
    ResumeOutcome::Completed(Box::new(resume::ResumedInstall {
        run,
        context: InstallRunContext {
            metadata: RunMetadata {
                requested: "install".to_string(),
                chain: vec!["validate".to_string(), "install".to_string()],
                offline: true,
                assume_yes: true,
                agent_mode: RunAgentMode::Cli,
                force: false,
                trace_compile: false,
                run_id: "fixed-run-id".to_string(),
                started: "2026-08-27T00:00:00Z".to_string(),
                selected: ".".to_string(),
            },
            lease: vibe_test_support::retained_lifecycle_lease(),
            lifecycle_run: None,
            lifecycle_reports: vec![row("resumed-in-context")],
        },
    }))
}

fn failed() -> ResumeOutcome {
    ResumeOutcome::Failed(MeasuredFailure {
        original: anyhow::Error::new(Sentinel).context("finishing the parked slot run"),
        evidence: Measurement::Slot {
            progress: Box::new(vibe_install::InstallProgress::fresh(vec![".".into()])),
            reports: vec![row("resumed-in-failure")],
            packages_resolved: 5,
        },
        emit_machine_failure: false,
    })
}

fn completed_of(outcome: ResumeOutcome) -> resume::ResumedInstall {
    match outcome {
        ResumeOutcome::Completed(resumed) => *resumed,
        _ => panic!("the completed arm"),
    }
}

fn failure_of(outcome: ResumeOutcome) -> MeasuredFailure {
    match outcome {
        ResumeOutcome::Failed(failure) => failure,
        _ => panic!("the failed arm"),
    }
}

/// The slot rows of a measurement, for the assertions below.
fn failure_rows(failure: &MeasuredFailure) -> &[vibe_install::SlotLifecycleReport] {
    match &failure.evidence {
        Measurement::Slot { reports, .. } | Measurement::InstallBarrier { reports, .. } => reports,
        Measurement::Lifecycle { .. } => panic!("an apply measures slot work"),
    }
}

/// COMPLETED: the applied rows go IN FRONT, in both carriers, and each carrier
/// keeps its own tail.
#[test]
fn the_applied_rows_precede_the_resumed_ones_in_both_carriers() {
    let joined = completed_of(join_applied_rows(resumed(), &[row("applied")]));

    assert_eq!(
        keys(&joined.run.slot_reports),
        ["applied", "resumed-in-run"],
        "the document's carrier: this apply's work, then the park it finished",
    );
    assert_eq!(
        keys(&joined.context.lifecycle_reports),
        ["applied", "resumed-in-context"],
        "and the callback's carrier, merged the same way from its OWN tail",
    );
}

/// Several applied rows keep their relative order, and nothing is duplicated.
#[test]
fn a_multi_row_prefix_keeps_its_order_and_duplicates_nothing() {
    let joined = completed_of(join_applied_rows(
        resumed(),
        &[row("applied-first"), row("applied-second")],
    ));

    assert_eq!(
        keys(&joined.run.slot_reports),
        ["applied-first", "applied-second", "resumed-in-run"],
    );
    assert_eq!(
        keys(&joined.context.lifecycle_reports),
        ["applied-first", "applied-second", "resumed-in-context"],
    );
}

/// An empty prefix leaves both carriers exactly as they were.
#[test]
fn an_empty_prefix_changes_neither_carrier() {
    let before = completed_of(resumed());
    let joined = completed_of(join_applied_rows(resumed(), &[]));

    assert_eq!(joined.run.slot_reports, before.run.slot_reports);
    assert_eq!(
        joined.context.lifecycle_reports,
        before.context.lifecycle_reports,
    );
}

/// The two carriers are never conflated.
#[test]
fn neither_carrier_receives_the_others_rows() {
    let joined = completed_of(join_applied_rows(resumed(), &[row("applied")]));

    assert!(
        !keys(&joined.run.slot_reports).contains(&"resumed-in-context"),
        "the document's carrier never picks up the callback's tail: {:?}",
        keys(&joined.run.slot_reports),
    );
    assert!(
        !keys(&joined.context.lifecycle_reports).contains(&"resumed-in-run"),
        "nor the other way round: {:?}",
        keys(&joined.context.lifecycle_reports),
    );
}

/// FAILED: the same prefix, into the neutral transport's own row list, once —
/// and every other measured field crosses unchanged.
///
/// This is the arm the old split lost. A Ready apply that materialised slots
/// and then hit a failing resumed row would report the resumed rows alone, as
/// though the apply had never run.
#[test]
fn a_failed_resume_receives_the_same_prefix_and_keeps_its_measurement() {
    let failure = failure_of(join_applied_rows(
        failed(),
        &[row("applied-first"), row("applied-second")],
    ));

    let Measurement::Slot {
        progress,
        reports,
        packages_resolved,
    } = &failure.evidence
    else {
        panic!("an apply measures slot work");
    };
    assert_eq!(
        keys(reports),
        ["applied-first", "applied-second", "resumed-in-failure"],
        "this apply's rows, in order, then the resumed run's",
    );
    assert_eq!(
        progress.nodes_regenerated,
        ["."],
        "the durable progress is untouched by the join",
    );
    assert_eq!(*packages_resolved, 5);
    assert!(
        failure.original.downcast_ref::<Sentinel>().is_some(),
        "and the error is still the original object",
    );
}

/// FAILED with no applied rows: the resumed rows are neither dropped nor
/// duplicated.
#[test]
fn a_failed_resume_with_an_empty_prefix_keeps_exactly_its_own_rows() {
    let failure = failure_of(join_applied_rows(failed(), &[]));
    assert_eq!(keys(failure_rows(&failure)), ["resumed-in-failure"]);
}

/// NOTHING stays nothing, and gains no fabricated rows.
///
/// The applied rows still belong to the ordinary completion path below the
/// match; inventing a resumed value here would report a continuation that was
/// never owed.
#[test]
fn nothing_owed_stays_nothing_and_fabricates_no_rows() {
    assert!(matches!(
        join_applied_rows(ResumeOutcome::Nothing, &[row("applied")]),
        ResumeOutcome::Nothing
    ));
}
