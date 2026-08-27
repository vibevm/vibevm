//! The lifecycle owner's mapping of a NEUTRAL resume failure.
//!
//! The prerequisite install transports the measurement without naming a family,
//! because the same `execute_prepared` is also `vibe install`'s body and
//! `vibe update --all`'s delegate. Here the family is the lifecycle one, and
//! this command already has a mechanism for choosing it — the fallback in
//! `execute_after_open` — so the only thing missing is the rows.
//!
//! This drives the production helper and then the SAME fallback/classifier
//! shape the boundary uses, so the two halves cannot agree in isolation and
//! disagree in production.

use super::*;
use crate::commands::compile_trace::{CommandExit, classify};
use crate::commands::install::{ResumeFailure, carry_resume_failure};

#[derive(Debug, thiserror::Error)]
#[error("the resumed row refused")]
struct Sentinel;

fn slot_row(key: &str, status: &str) -> vibe_install::SlotLifecycleReport {
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

fn phase_row(key: &str) -> LifecycleContributionReport {
    LifecycleContributionReport {
        flagged: None,
        handler: "builtin".into(),
        key: key.into(),
        message: None,
        stderr: None,
        stderr_truncated: None,
        stdout: None,
        stdout_truncated: None,
        phase: "clean".into(),
        point: "phase:clean".into(),
        provider: "org.demo/tools".into(),
        reference: None,
        slot_target: None,
        status: "ok".into(),
        tier: "dependency".into(),
        version: None,
    }
}

/// A resume failure from the prerequisite install joins THIS command's
/// accumulator in chronology, and the boundary then reports one lifecycle root.
///
/// Every half of this is a separate way to lose the truth: dropping the helper
/// call empties the resumed rows; putting them in front loses the clean epoch's
/// chronology; carrying a root here would emit an install-shaped document where
/// a phase verb has always emitted a lifecycle-shaped one; and letting the
/// neutral wrapper escape hands `main` an error whose downcast is not the
/// command's.
#[test]
fn a_neutral_resume_failure_joins_the_prefix_and_reports_one_lifecycle_root() {
    // Whatever this command had already measured before the prerequisite ran.
    let mut measured = Measured {
        contributions: vec![phase_row("clean:earlier")],
    };
    let transported = carry_resume_failure(ResumeFailure {
        original: anyhow::Error::new(Sentinel).context("finishing the parked slot run"),
        progress: vibe_install::InstallProgress::fresh(vec![".".into()]),
        reports: vec![
            slot_row("resumed:ok", "ok"),
            slot_row("resumed:fail", "fail"),
        ],
        packages_resolved: 4,
    });

    let error = absorb_resume_failure(transported, &mut measured);

    assert!(
        error.downcast_ref::<Sentinel>().is_some(),
        "the ORIGINAL object reaches the boundary",
    );
    assert!(
        error.downcast_ref::<ResumeFailure>().is_none(),
        "and the neutral wrapper does not escape",
    );
    assert_eq!(
        measured
            .contributions
            .iter()
            .map(|row| row.key.as_str())
            .collect::<Vec<_>>(),
        ["clean:earlier", "resumed:ok", "resumed:fail"],
        "prefix first, then the resumed rows, in order",
    );

    // The SAME fallback shape `execute_after_open` uses.
    let CommandExit::Failed {
        report,
        original_error,
        emit_when_trace_disabled,
    } = classify(error, || {
        RegisteredReportDraft::Lifecycle(Box::new(LifecycleDraft::failed(
            "build",
            vec!["validate".into(), "install".into(), "build".into()],
            "build",
            measured.contributions.clone(),
        )))
    })
    else {
        panic!("a failure is a failure");
    };
    assert!(
        !emit_when_trace_disabled,
        "a generic stage failure has always been silent with tracing off",
    );
    assert!(original_error.downcast_ref::<Sentinel>().is_some());
    assert_eq!(
        format!("{original_error:#}"),
        "finishing the parked slot run: the resumed row refused",
    );
    let RegisteredReportDraft::Lifecycle(draft) = report else {
        panic!("a phase verb reports the LIFECYCLE family");
    };
    let built = draft.into_report(None);
    assert!(!built.ok);
    assert_eq!(
        built
            .contributions
            .iter()
            .map(|row| row.key.as_str())
            .collect::<Vec<_>>(),
        ["clean:earlier", "resumed:ok", "resumed:fail"],
        "exactly one lifecycle draft, carrying both passes in order",
    );
}

/// An error that is not a transported resume failure is returned exactly as it
/// arrived, and the accumulator is untouched.
#[test]
fn an_ordinary_error_passes_through_and_adds_no_rows() {
    let mut measured = Measured {
        contributions: vec![phase_row("clean:earlier")],
    };
    let error = absorb_resume_failure(anyhow::anyhow!("planning blew up"), &mut measured);
    assert_eq!(error.to_string(), "planning blew up");
    assert_eq!(measured.contributions.len(), 1);
}
