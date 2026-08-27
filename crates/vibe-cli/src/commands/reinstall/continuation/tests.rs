//! Reds for the reinstall continuation's OWNERSHIP and its owner mapping.
//!
//! `owned_continuation` is the exact match production runs; the resume and the
//! destructive take are injected so *when* the take happens is provable. A live
//! lifecycle in a unit test could not distinguish "took once, on the arm that
//! owns the value" from "took eagerly and got lucky".

use std::cell::Cell;

use super::*;
use crate::commands::compile_trace::{CommandExit, classify};

#[derive(Debug, thiserror::Error)]
#[error("the resumed row refused")]
struct Sentinel;

fn row(key: &str, status: &str) -> SlotLifecycleReport {
    SlotLifecycleReport {
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

fn keys(rows: &[SlotLifecycleReport]) -> Vec<&str> {
    rows.iter().map(|row| row.key.as_str()).collect()
}

fn identity() -> ReinstallIdentity {
    ReinstallIdentity {
        selected_project_root: std::path::PathBuf::from("/p/member"),
        forced: true,
    }
}

fn metadata() -> RunMetadata {
    RunMetadata {
        requested: "reinstall".into(),
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

/// A NONDEFAULT completed progress — every list distinct and populated, so a
/// replacement with `InstallProgress::default()` or with some other run's
/// record is visible field by field.
fn resumed_progress() -> InstallProgress {
    InstallProgress {
        complete: true,
        fresh: false,
        materialised: vec!["vibedeps/org.demo.tools/0.2.0".into()],
        skipped: vec!["vibedeps/org.demo.quiet/1.0.0".into()],
        pruned: vec!["vibedeps/org.demo.tools/0.1.0".into()],
        nodes_regenerated: vec![".".into(), "member".into()],
    }
}

fn assert_resumed_progress(progress: &InstallProgress) {
    assert!(progress.complete, "the resume's own completion survives");
    assert!(!progress.fresh);
    assert_eq!(progress.materialised, ["vibedeps/org.demo.tools/0.2.0"]);
    assert_eq!(progress.skipped, ["vibedeps/org.demo.quiet/1.0.0"]);
    assert_eq!(progress.pruned, ["vibedeps/org.demo.tools/0.1.0"]);
    assert_eq!(progress.nodes_regenerated, [".", "member"]);
}

fn completed(rows: Vec<SlotLifecycleReport>) -> ResumeOutcome {
    let mut run = crate::commands::install::InstallRun::new(
        std::path::PathBuf::from("/p"),
        InstallDisposition::Fresh,
    );
    run.slot_reports = rows;
    run.progress = resumed_progress();
    ResumeOutcome::Completed(Box::new(crate::commands::install::ResumedInstall {
        run,
        context: crate::commands::install::InstallRunContext {
            metadata: metadata(),
            lifecycle_run: None,
            lifecycle_reports: Vec::new(),
        },
    }))
}

fn failed(rows: Vec<SlotLifecycleReport>) -> ResumeOutcome {
    ResumeOutcome::Failed(ResumeFailure {
        original: anyhow::Error::new(Sentinel).context("finishing the parked slot run"),
        progress: InstallProgress {
            complete: true,
            fresh: false,
            materialised: vec!["vibedeps/org.demo.tools/0.1.0".into()],
            skipped: Vec::new(),
            pruned: Vec::new(),
            nodes_regenerated: vec![".".into()],
        },
        reports: rows,
        packages_resolved: 0,
    })
}

/// FORCED + completed: one take, current pass in front.
#[test]
fn a_forced_completed_resume_takes_once_and_puts_the_apply_first() {
    let takes = Cell::new(0);
    let serviced = owned_continuation(
        &identity(),
        || Ok(completed(vec![row("resumed", "ok")])),
        || {
            takes.set(takes.get() + 1);
            vec![row("apply-a", "ok"), row("apply-b", "ok")]
        },
    )
    .expect("a completed resume is not an error")
    .expect("and it owns the value");
    assert_eq!(takes.get(), 1);
    assert_eq!(keys(&serviced.rows), ["apply-a", "apply-b", "resumed"]);
    assert!(serviced.parked.is_none());
    assert_resumed_progress(&serviced.progress);
}

/// FORCED + reparked: same join, and the handoff survives.
#[test]
fn a_forced_reparked_resume_joins_the_same_way() {
    let takes = Cell::new(0);
    let ResumeOutcome::Completed(mut resumed) = completed(vec![row("resumed", "ok")]) else {
        panic!("completed");
    };
    resumed.run.parked = Some(vibe_lifecycle::Delegation {
        resume: "vibe reinstall".into(),
        run_id: "0".repeat(32),
        tasks: vec![format!(
            "{}/{}/a.md",
            vibe_lifecycle::OUTBOX_RELATIVE,
            "0".repeat(32)
        )],
    });
    let serviced = owned_continuation(
        &identity(),
        || Ok(ResumeOutcome::Completed(resumed)),
        || {
            takes.set(takes.get() + 1);
            vec![row("apply", "ok")]
        },
    )
    .expect("a park is not a failure")
    .expect("and it owns the value");
    assert_eq!(takes.get(), 1);
    assert_eq!(keys(&serviced.rows), ["apply", "resumed"]);
    assert!(serviced.parked.is_some());
    assert_resumed_progress(&serviced.progress);
}

/// FORCED + failed: one take, current pass in front, exact progress and error,
/// and THIS command's registered family with the historical silence.
#[test]
fn a_forced_failed_resume_becomes_a_measured_reinstall_root_with_both_passes() {
    let takes = Cell::new(0);
    let error = owned_continuation(
        &identity(),
        || {
            Ok(failed(vec![
                row("resumed-ok", "ok"),
                row("resumed-fail", "fail"),
            ]))
        },
        || {
            takes.set(takes.get() + 1);
            vec![row("apply", "ok")]
        },
    )
    .expect_err("a failed resume is an error");
    assert_eq!(takes.get(), 1, "exactly one destructive take");

    let CommandExit::Failed {
        report,
        original_error,
        emit_when_trace_disabled,
    } = classify(error, || {
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
    let RegisteredReportDraft::Reinstall(draft) = report else {
        panic!("this command's own family");
    };
    let built = draft.into_report(None);
    assert!(!built.ok);
    assert!(!built.complete);
    assert!(built.forced);
    assert_eq!(
        built.project,
        vibe_core::machine_json_path(std::path::Path::new("/p/member")),
        "and it names the SELECTED node",
    );
    assert_eq!(
        built
            .contributions
            .iter()
            .map(|row| row.key.as_str())
            .collect::<Vec<_>>(),
        ["apply", "resumed-ok", "resumed-fail"],
        "the forced apply's own row precedes the resumed run's, once",
    );
    assert_eq!(built.materialised, ["vibedeps/org.demo.tools/0.1.0"]);
    assert_eq!(built.nodes_regenerated, ["."]);
}

/// FORCED + nothing owed: no take at all.
#[test]
fn nothing_owed_takes_nothing() {
    let takes = Cell::new(0);
    let outcome = owned_continuation(
        &identity(),
        || Ok(ResumeOutcome::Nothing),
        || {
            takes.set(takes.get() + 1);
            vec![row("apply", "ok")]
        },
    )
    .expect("nothing owed is not an error");
    assert!(outcome.is_none());
    assert_eq!(
        takes.get(),
        0,
        "the caller's fallback still owns those rows"
    );
}

/// An ordinary `Err` before a typed outcome: no take, exact error.
#[test]
fn an_error_before_the_typed_outcome_takes_nothing() {
    let takes = Cell::new(0);
    let error = owned_continuation(
        &identity(),
        || Err(anyhow::Error::new(Sentinel).context("reading the lockfile")),
        || {
            takes.set(takes.get() + 1);
            vec![row("apply", "ok")]
        },
    )
    .expect_err("the resume itself failed");
    assert_eq!(takes.get(), 0);
    assert!(error.downcast_ref::<Sentinel>().is_some());
    assert_eq!(
        format!("{error:#}"),
        "reading the lockfile: the resumed row refused",
    );
}

/// PLAIN reinstall has no current pass: its explicit `Vec::new` prefix adds
/// nothing on either arm, and the resumed rows appear exactly once.
#[test]
fn a_plain_reinstall_prefixes_nothing_on_either_arm() {
    let serviced = owned_continuation(
        &identity(),
        || Ok(completed(vec![row("resumed", "ok")])),
        Vec::new,
    )
    .expect("completed")
    .expect("owns the value");
    assert_eq!(keys(&serviced.rows), ["resumed"]);

    let error = owned_continuation(
        &identity(),
        || Ok(failed(vec![row("resumed", "fail")])),
        Vec::new,
    )
    .expect_err("failed");
    let CommandExit::Failed { report, .. } = classify(error, || panic!("carried")) else {
        panic!("a failure is a failure");
    };
    let RegisteredReportDraft::Reinstall(draft) = report else {
        panic!("family");
    };
    assert_eq!(
        draft
            .into_report(None)
            .contributions
            .iter()
            .map(|row| row.key.as_str())
            .collect::<Vec<_>>(),
        ["resumed"],
        "no prefix, and no repeat",
    );
}

/// A current pass that COUNTS what production asks of it.
///
/// The forced path used to select `Some(lifecycle)` at a call site no unit test
/// could reach, so mutating it to `None` deleted forced ownership and stayed
/// green. `forced_continuation` takes this trait instead, and the counters
/// below are the proof that the real lifecycle's gate and take are wired in.
struct FakeCurrent {
    owes: bool,
    rows: Vec<SlotLifecycleReport>,
    gates: Cell<usize>,
    takes: Cell<usize>,
}

impl FakeCurrent {
    fn new(owes: bool, rows: Vec<SlotLifecycleReport>) -> Self {
        Self {
            owes,
            rows,
            gates: Cell::new(0),
            takes: Cell::new(0),
        }
    }
}

impl CurrentRows for FakeCurrent {
    fn owes_slot_work(&self) -> bool {
        self.gates.set(self.gates.get() + 1);
        self.owes
    }

    fn take_rows(&self) -> Vec<SlotLifecycleReport> {
        self.takes.set(self.takes.get() + 1);
        self.rows.clone()
    }
}

/// owes = false: the resume never runs and nothing is taken.
///
/// The gate is what keeps a forced apply that finished cleanly from resuming a
/// continuation nobody owes — and the rows stay with the run so the caller's
/// own fallback can still report them.
#[test]
fn the_forced_gate_refuses_before_resuming_or_taking() {
    let current = FakeCurrent::new(false, vec![row("apply", "ok")]);
    let resumes = Cell::new(0);
    let outcome = forced_continuation(&current, &identity(), || {
        resumes.set(resumes.get() + 1);
        Ok(ResumeOutcome::Nothing)
    })
    .expect("a closed gate is not an error");
    assert!(outcome.is_none());
    assert_eq!(current.gates.get(), 1, "the gate was really consulted");
    assert_eq!(resumes.get(), 0, "and nothing resumed behind it");
    assert_eq!(current.takes.get(), 0, "nor was anything taken");
}

/// owes = true, Completed: one resume, one take, current rows in front.
///
/// This is the wiring `service_if_owed` performs with the REAL lifecycle. A
/// forced path that stopped passing its current pass would leave the resumed
/// rows alone here.
#[test]
fn the_forced_gate_wires_the_current_pass_into_a_completed_resume() {
    let current = FakeCurrent::new(true, vec![row("apply", "ok")]);
    let resumes = Cell::new(0);
    let serviced = forced_continuation(&current, &identity(), || {
        resumes.set(resumes.get() + 1);
        Ok(completed(vec![row("resumed", "ok")]))
    })
    .expect("completed")
    .expect("owns the value");
    assert_eq!(resumes.get(), 1);
    assert_eq!(current.takes.get(), 1, "exactly one destructive take");
    assert_eq!(keys(&serviced.rows), ["apply", "resumed"]);
    assert_resumed_progress(&serviced.progress);
}

/// owes = true, Failed: the same wiring, into the measured Reinstall root.
#[test]
fn the_forced_gate_wires_the_current_pass_into_a_failed_resume() {
    let current = FakeCurrent::new(true, vec![row("apply", "ok")]);
    let error = forced_continuation(&current, &identity(), || {
        Ok(failed(vec![row("resumed-fail", "fail")]))
    })
    .expect_err("a failed resume is an error");
    assert_eq!(current.takes.get(), 1);
    let CommandExit::Failed { report, .. } = classify(error, || panic!("carried")) else {
        panic!("a failure is a failure");
    };
    let RegisteredReportDraft::Reinstall(draft) = report else {
        panic!("family");
    };
    assert_eq!(
        draft
            .into_report(None)
            .contributions
            .iter()
            .map(|row| row.key.as_str())
            .collect::<Vec<_>>(),
        ["apply", "resumed-fail"],
    );
}

/// owes = true but nothing was owed after all, and a pre-outcome `Err`: the
/// gate opens, the resume answers, and NOTHING is taken either way.
#[test]
fn the_forced_gate_takes_nothing_when_the_resume_owns_no_value() {
    for outcome in [
        Ok(ResumeOutcome::Nothing),
        Err(anyhow::Error::new(Sentinel).context("reading the lockfile")),
    ] {
        let current = FakeCurrent::new(true, vec![row("apply", "ok")]);
        let result = forced_continuation(&current, &identity(), || outcome);
        assert_eq!(
            current.takes.get(),
            0,
            "the caller's fallback still owns those rows",
        );
        match result {
            Ok(value) => assert!(value.is_none()),
            Err(error) => assert!(error.downcast_ref::<Sentinel>().is_some()),
        }
    }
}

/// An INVALID handoff on the Completed arm is refused BEFORE the take.
///
/// A malformed hosted task path is a failed command. Validating after the join
/// would lose both halves at once: the joined outcome is dropped on the error
/// path, and the caller's fallback then reads a current pass this seam has
/// already emptied. The counter is the proof that it did not.
#[test]
fn an_invalid_handoff_is_refused_before_the_current_rows_are_taken() {
    let current = FakeCurrent::new(true, vec![row("apply", "ok")]);
    let ResumeOutcome::Completed(mut resumed) = completed(vec![row("resumed", "ok")]) else {
        panic!("completed");
    };
    resumed.run.parked = Some(vibe_lifecycle::Delegation {
        resume: "vibe reinstall".into(),
        run_id: "0".repeat(32),
        // NOT under this run's outbox directory.
        tasks: vec!["docs/somewhere-else.md".into()],
    });
    let error = forced_continuation(&current, &identity(), || {
        Ok(ResumeOutcome::Completed(resumed))
    })
    .expect_err("a malformed handoff is a failed command");

    assert_eq!(
        current.takes.get(),
        0,
        "the current pass still owns its rows, for the caller's fallback",
    );
    assert!(
        format!("{error:#}").contains("does not live directly under run"),
        "and the exact validation error is returned: {error:#}",
    );
    assert!(
        !crate::commands::compile_trace::is_carried(&error),
        "root-neutral: no family was chosen here",
    );
}
