//! The ONE place a `cli-lifecycle-report` is presented — and nothing else.
//!
//! There is no draft TYPE here any more. The shared
//! [`vibe_orchestrator::values::LifecycleValues`] IS the report's values, and
//! it owns the total conversion into the generated root; what this surface adds
//! is the terminal shape of that root. A newtype whose only job was to host one
//! `render` impl was a second name for one thing, and it had to be kept in step
//! with the values by hand.
//!
//! The delegation member is already VALIDATED by the time these values exist.
//! That is the point of the split: validating a hosted handoff used to happen
//! inside the renderer, so a malformed one produced a run that had been
//! finalised as a successful park and then failed while printing it — a trace
//! index saying `running`, a lifecycle state saying `delegated`, and an error
//! saying neither. Now a bad handoff is a failed exit, decided before the
//! funnel, and everything after the funnel is infallible except the write.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE");

use anyhow::Result;
use vibe_orchestrator::values::LifecycleValues;
use vibe_wire::generated::shared::{CompileTraceReport, EvidenceStatus};

use crate::output;

use super::report::render_handoff;

/// Build the generated root with its `trace` member attached, and emit it.
///
/// `quiet_suffix` is appended to the command's ONE summary line, and only
/// there: a failed root narrates nothing in human mode (it never has), so its
/// suffix travels on the error line instead.
pub(crate) fn render_lifecycle(
    values: LifecycleValues,
    ctx: &output::Context,
    trace: Option<CompileTraceReport>,
    quiet_suffix: &str,
) -> Result<()> {
    let ok = values.ok;
    let report = values.into_report(trace);
    if ctx.is_json() {
        // Exactly ONE generated document, carrying the handoff as a typed
        // member — never a second object appended after it, and never a
        // fence.
        return ctx.emit_json(&report);
    }
    if !ok {
        // A failed lifecycle run's account on the terminal is its error;
        // it has never printed a second, summary-shaped one.
        return Ok(());
    }
    let fresh = report
        .contributions
        .iter()
        .filter(|row| row.status == "fresh")
        .count();
    let executed = report.contributions.len() - fresh;
    let ok = report
        .contributions
        .iter()
        .filter(|row| row.status == "ok")
        .count();
    let contribution_summary = format!(
        "{} contribution(s) selected, {executed} executed, {ok} ok, {fresh} fresh",
        report.contributions.len(),
    );
    let requested = &report.requested;
    // Quiet still prints the required contract: the handoff is the whole
    // point of the invocation, and a caller that suppressed narration did
    // not ask to lose the tasks it must now perform.
    if !ctx.is_quiet() {
        ctx.heading(&format!("lifecycle `{requested}`:"));
        for step in &report.steps {
            ctx.step(&format!("{}: {}", step.phase, step.status));
        }
        // A PROJECTION of the typed member, never a second source: the words
        // are read off the value JSON carries, and the member itself decides
        // whether the line exists at all. A failed run never reaches here —
        // its account on the terminal has always been its error alone.
        if let Some(evidence) = report.verification.as_ref() {
            ctx.step(&format!(
                "verification: {} ({} input(s), {} artifact(s))",
                evidence_status(&evidence.status),
                evidence.inputs.len(),
                evidence.artifacts.len(),
            ));
        }
    }
    render_handoff(ctx, report.delegation.as_ref());
    ctx.summary(&format!(
        "vibe lifecycle: {requested} {} ({} phases, {contribution_summary}, {} notice(s)){quiet_suffix}",
        completion(report.delegation.is_some()),
        report.steps.len(),
        report.notices.len(),
    ));
    Ok(())
}

const fn completion(parked: bool) -> &'static str {
    if parked { "parked" } else { "completed" }
}

/// The evidence status in its exact wire spelling, so the terminal line and
/// the JSON member cannot disagree about one comparison.
const fn evidence_status(status: &EvidenceStatus) -> &'static str {
    match status {
        EvidenceStatus::Matched => "matched",
        EvidenceStatus::Missing => "missing",
        EvidenceStatus::Stale => "stale",
        EvidenceStatus::Unavailable => "unavailable",
        EvidenceStatus::Unstable => "unstable",
    }
}

#[cfg(test)]
mod refusal_tests {
    use super::*;
    use vibe_wire::behaviour::compile_trace_report::validate;
    use vibe_wire::generated::lifecycle_report::LifecycleStepReport;
    use vibe_wire::generated::shared::TraceReportStatus;

    /// The lifecycle root carries a refused member exactly as the install root
    /// does. Both are consumers of the same member, and a divergence between
    /// them would mean one of the four command reports told a different story
    /// about the same run.
    #[test]
    fn a_successful_lifecycle_root_carries_a_refused_member_unchanged() {
        let member = CompileTraceReport {
            budget_exhausted: false,
            events: "2".into(),
            finalised: false,
            run_id: "b".repeat(32),
            snapshot_bytes: "0".into(),
            snapshots: "0".into(),
            status: TraceReportStatus::Running,
            timings: Vec::new(),
            warnings: vec!["the terminal index could not be published".into()],
            run_path: Some(format!("/p/.vibe/trace/{}", "b".repeat(32))),
        };
        let values = LifecycleValues::completed(
            "build",
            vec!["validate".into(), "install".into(), "build".into()],
            vec![LifecycleStepReport {
                phase: "build".into(),
                status: "ok".into(),
            }],
            Vec::new(),
            Vec::new(),
            None,
        );
        let report = values.into_report(Some(member.clone()));

        assert!(report.ok, "the COMMAND succeeded");
        let attached = report.trace.expect("and the member was not dropped");
        assert_eq!(attached, member, "carried through byte for byte");
        assert_eq!(attached.status, TraceReportStatus::Running);
        assert!(!attached.finalised);
        validate(&attached).expect("and it is still a valid member");
    }

    #[test]
    fn a_disabled_lifecycle_root_omits_the_member() {
        let report =
            LifecycleValues::failed("build", Vec::new(), "build", Vec::new()).into_report(None);
        assert!(report.trace.is_none());
    }
}
