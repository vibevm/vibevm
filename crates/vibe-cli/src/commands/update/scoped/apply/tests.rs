//! Reds for the scoped update's continuation OWNERSHIP.
//!
//! These drive `owned_continuation` — the exact match production runs — with
//! the resume and the destructive take injected. That is the only way to prove
//! the property that matters: the take is destructive, so *when* it runs is as
//! load-bearing as *what* it produces, and a live lifecycle in a unit test
//! could not show the difference between "took once, on the right arm" and
//! "took eagerly and got lucky".

use std::cell::Cell;

use super::*;
use vibe_wire::generated::update_report::UpdateReportScope;

#[derive(Debug, thiserror::Error)]
#[error("the resumed row refused")]
struct Sentinel;

fn row(key: &str) -> vibe_install::SlotLifecycleReport {
    vibe_install::SlotLifecycleReport {
        key: key.to_string(),
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
    }
}

fn keys(rows: &[vibe_install::SlotLifecycleReport]) -> Vec<&str> {
    rows.iter().map(|row| row.key.as_str()).collect()
}

fn identity() -> UpdateIdentity {
    UpdateIdentity {
        project_root: std::path::PathBuf::from("/p"),
        scope: UpdateReportScope::Scoped,
        packages: vec!["org.demo/tools".into()],
    }
}

/// A `Measured` with a real pre-lifecycle prefix — an in-place slot this run
/// really advanced before any lifecycle existed.
fn measured() -> Measured {
    let mut measured = Measured::default();
    measured.record_in_place("vibedeps/org.demo.tools".into(), true);
    measured.record_bump("org.demo/tools 0.1.0 -> 0.2.0".into());
    measured
}

fn resumed_run(rows: Vec<vibe_install::SlotLifecycleReport>) -> ResumeOutcome {
    let mut run = crate::commands::install::InstallRun::new(
        std::path::PathBuf::from("/p"),
        crate::commands::install::InstallDisposition::Fresh,
    );
    run.slot_reports = rows;
    run.progress = vibe_install::InstallProgress::fresh(vec![".".into()]);
    ResumeOutcome::Completed(Box::new(crate::commands::install::ResumedInstall {
        run,
        context: crate::commands::install::InstallRunContext {
            metadata: metadata(),
            lifecycle_run: None,
            lifecycle_reports: Vec::new(),
        },
    }))
}

fn metadata() -> vibe_lifecycle::RunMetadata {
    vibe_lifecycle::RunMetadata {
        requested: "update".into(),
        chain: vec!["install".into()],
        offline: true,
        assume_yes: true,
        agent_mode: vibe_wire::generated::lifecycle::e1::context::RunAgentMode::Cli,
        force: false,
        trace_compile: false,
        run_id: "0".repeat(32),
        started: "2026-08-27T00:00:00Z".into(),
    }
}

fn failed(rows: Vec<vibe_install::SlotLifecycleReport>) -> ResumeOutcome {
    ResumeOutcome::Failed(crate::commands::install::ResumeFailure {
        original: anyhow::Error::new(Sentinel).context("finishing the parked slot run"),
        progress: vibe_install::InstallProgress::fresh(vec![".".into()]),
        reports: rows,
        packages_resolved: 3,
    })
}

/// COMPLETED: the current pass's rows come first, once, and the resume's own
/// characterised progress survives untouched.
#[test]
fn a_completed_resume_puts_the_current_rows_first_and_takes_once() {
    let takes = Cell::new(0);
    let draft = owned_continuation_values(
        &measured(),
        &identity(),
        3,
        || Ok(resumed_run(vec![row("resumed")])),
        || {
            takes.set(takes.get() + 1);
            vec![row("current-a"), row("current-b")]
        },
    )
    .expect("a completed resume is not an error")
    .expect("and it owns the draft");
    assert_eq!(takes.get(), 1, "exactly one destructive take");
    assert_eq!(keys(&draft.rows), ["current-a", "current-b", "resumed"]);
    assert_eq!(
        draft.progress.nodes_regenerated,
        ["."],
        "success keeps the resume's own characterised progress",
    );
    assert!(draft.ok);
}

/// A resume that PARKED AGAIN is still a completed outcome, and joins the same
/// way.
#[test]
fn a_reparked_resume_joins_the_same_way() {
    let takes = Cell::new(0);
    let ResumeOutcome::Completed(mut resumed) = resumed_run(vec![row("resumed")]) else {
        panic!("completed");
    };
    resumed.run.parked = Some(vibe_lifecycle::Delegation {
        resume: "vibe update".into(),
        run_id: "0".repeat(32),
        tasks: vec![format!(
            "{}/{}/a.md",
            vibe_lifecycle::OUTBOX_RELATIVE,
            "0".repeat(32)
        )],
    });
    let draft = owned_continuation_values(
        &measured(),
        &identity(),
        3,
        || Ok(ResumeOutcome::Completed(resumed)),
        || {
            takes.set(takes.get() + 1);
            vec![row("current")]
        },
    )
    .expect("a park is not a failure")
    .expect("and it owns the draft");
    assert_eq!(takes.get(), 1);
    assert_eq!(keys(&draft.rows), ["current", "resumed"]);
    assert!(draft.delegation.is_some(), "the handoff survives the join");
}

/// FAILED: the current rows still come first, the pre-lifecycle `Measured`
/// prefix is really joined into the progress, and the error is the original.
#[test]
fn a_failed_resume_joins_rows_progress_and_keeps_the_exact_error() {
    let takes = Cell::new(0);
    let error = owned_continuation_values(
        &measured(),
        &identity(),
        3,
        || Ok(failed(vec![row("resumed-ok"), row("resumed-fail")])),
        || {
            takes.set(takes.get() + 1);
            vec![row("current")]
        },
    )
    .expect_err("a failed resume is an error");
    assert_eq!(takes.get(), 1, "exactly one destructive take");

    let crate::commands::compile_trace::CommandExit::Failed {
        report,
        original_error,
        emit_when_trace_disabled,
    } = crate::commands::compile_trace::classify(error, || {
        panic!("the carrier decides the root, not the fallback")
    })
    else {
        panic!("a failure is a failure");
    };
    assert!(!emit_when_trace_disabled, "historically silent");
    assert!(original_error.downcast_ref::<Sentinel>().is_some());
    assert_eq!(
        format!("{original_error:#}"),
        "finishing the parked slot run: the resumed row refused",
    );
    let RegisteredReportDraft::Update(draft) = report else {
        panic!("this command's own family");
    };
    let built = draft.into_report(None);
    assert_eq!(
        built
            .contributions
            .iter()
            .map(|row| row.key.as_str())
            .collect::<Vec<_>>(),
        ["current", "resumed-ok", "resumed-fail"],
        "current pass first, then the resumed run, once",
    );
    assert_eq!(
        built.materialised,
        ["vibedeps/org.demo.tools"],
        "the pre-lifecycle in-place prefix is JOINED into the failure progress —          an empty list here is the defect this seam exists to refuse",
    );
    assert_eq!(
        built.nodes_regenerated,
        ["."],
        "beside the durable progress the resume itself inherited",
    );
    assert_eq!(built.version_bumps, ["org.demo/tools 0.1.0 -> 0.2.0"]);
}

/// NOTHING: no take at all — those rows still belong to the outer fallback.
#[test]
fn nothing_owed_takes_nothing() {
    let takes = Cell::new(0);
    let outcome = owned_continuation_values(
        &measured(),
        &identity(),
        3,
        || Ok(ResumeOutcome::Nothing),
        || {
            takes.set(takes.get() + 1);
            vec![row("current")]
        },
    )
    .expect("nothing owed is not an error");
    assert!(outcome.is_none());
    assert_eq!(takes.get(), 0, "the outer fallback still owns those rows");
}

/// An ordinary `Err` BEFORE a typed outcome: no take, and the error is exact.
///
/// This is the ordering half of the law. An eager take would empty the current
/// pass here, and the outer `carry_measured` fallback — which is what reports
/// this failure — would then describe a run that did nothing.
#[test]
fn an_error_before_the_typed_outcome_takes_nothing_and_keeps_its_error() {
    let takes = Cell::new(0);
    let error = owned_continuation_values(
        &measured(),
        &identity(),
        3,
        || Err(anyhow::Error::new(Sentinel).context("reading the lockfile")),
        || {
            takes.set(takes.get() + 1);
            vec![row("current")]
        },
    )
    .expect_err("the resume itself failed");
    assert_eq!(
        takes.get(),
        0,
        "nothing was taken before the resume answered"
    );
    assert!(error.downcast_ref::<Sentinel>().is_some());
    assert_eq!(
        format!("{error:#}"),
        "reading the lockfile: the resumed row refused",
    );
}

/// An INVALID handoff on the Completed arm is refused BEFORE the take.
///
/// Validating after the join loses both halves at once: the joined draft is
/// dropped on the error path, and the outer `carry_measured` fallback then
/// reads a lifecycle this seam has already emptied — reporting a run that did
/// nothing over one that pruned, materialised and regenerated. The counter is
/// the proof that the rows are still where the fallback will look.
#[test]
fn an_invalid_handoff_is_refused_before_the_current_rows_are_taken() {
    let takes = Cell::new(0);
    let ResumeOutcome::Completed(mut resumed) = resumed_run(vec![row("resumed")]) else {
        panic!("completed");
    };
    resumed.run.parked = Some(vibe_lifecycle::Delegation {
        resume: "vibe update".into(),
        run_id: "0".repeat(32),
        // NOT under this run's outbox directory.
        tasks: vec!["docs/somewhere-else.md".into()],
    });
    let error = owned_continuation_values(
        &measured(),
        &identity(),
        3,
        || Ok(ResumeOutcome::Completed(resumed)),
        || {
            takes.set(takes.get() + 1);
            vec![row("current")]
        },
    )
    .expect_err("a malformed handoff is a failed command");

    assert_eq!(
        takes.get(),
        0,
        "the current pass still owns its rows, for the outer fallback",
    );
    assert!(
        format!("{error:#}").contains("does not live directly under run"),
        "and the exact validation error is returned: {error:#}",
    );
    assert!(
        !crate::commands::compile_trace::is_carried(&error),
        "root-neutral: no draft was fabricated here",
    );
}
