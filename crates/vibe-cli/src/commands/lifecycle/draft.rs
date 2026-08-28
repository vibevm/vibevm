//! The owned `cli-lifecycle-report` draft, and the one place it is rendered.
//!
//! The delegation member is already VALIDATED by the time a draft exists.
//! That is the point of the split: validating a hosted handoff used to happen
//! inside the renderer, so a malformed one produced a run that had been
//! finalised as a successful park and then failed while printing it — a trace
//! index saying `running`, a lifecycle state saying `delegated`, and an error
//! saying neither. Now a bad handoff is a failed exit, decided before the
//! funnel, and everything after the funnel is infallible except the write.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE");

use anyhow::Result;
use vibe_orchestrator::values::LifecycleValues;
use vibe_wire::generated::lifecycle_report::{
    LifecycleContributionReport, LifecycleDelegation, LifecycleReport, LifecycleStepReport,
};
use vibe_wire::generated::shared::CompileTraceReport;

use crate::output;

use super::report::render_handoff;

/// Everything the one lifecycle document reports — the shared service's own
/// values, wrapped so THIS surface owns the rendering and the registered
/// family. There is no second copy of the fields: A13 removes the wrapper.
#[derive(Debug)]
pub(crate) struct LifecycleDraft(pub(crate) LifecycleValues);

impl std::ops::Deref for LifecycleDraft {
    type Target = LifecycleValues;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for LifecycleDraft {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl LifecycleDraft {
    /// A completed or parked phase run.
    pub(crate) fn completed(
        requested: &str,
        chain: Vec<String>,
        steps: Vec<LifecycleStepReport>,
        contributions: Vec<LifecycleContributionReport>,
        notices: Vec<String>,
        delegation: Option<LifecycleDelegation>,
    ) -> Self {
        Self(LifecycleValues::completed(
            requested,
            chain,
            steps,
            contributions,
            notices,
            delegation,
        ))
    }

    /// The generated root, with the member attached — pure, total, and the
    /// ONLY place a lifecycle report is built. See
    /// [`crate::commands::install::InstallDraft::into_report`] for why the
    /// attachment is a seam of its own.
    pub(crate) fn into_report(self, trace: Option<CompileTraceReport>) -> LifecycleReport {
        let LifecycleValues {
            ok,
            requested,
            chain,
            steps,
            contributions,
            notices,
            delegation,
        } = self.0;
        LifecycleReport {
            chain,
            command: "lifecycle".to_string(),
            contributions,
            notices,
            ok,
            requested,
            steps,
            delegation,
            trace,
        }
    }

    pub(crate) fn render(
        self,
        ctx: &output::Context,
        trace: Option<CompileTraceReport>,
        quiet_suffix: &str,
    ) -> Result<()> {
        let ok = self.ok;
        let report = self.into_report(trace);
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
}

const fn completion(parked: bool) -> &'static str {
    if parked { "parked" } else { "completed" }
}

#[cfg(test)]
mod refusal_tests {
    use super::*;
    use vibe_wire::behaviour::compile_trace_report::validate;
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
        let draft = LifecycleDraft::completed(
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
        let report = draft.into_report(Some(member.clone()));

        assert!(report.ok, "the COMMAND succeeded");
        let attached = report.trace.expect("and the member was not dropped");
        assert_eq!(attached, member, "carried through byte for byte");
        assert_eq!(attached.status, TraceReportStatus::Running);
        assert!(!attached.finalised);
        validate(&attached).expect("and it is still a valid member");
    }

    #[test]
    fn a_disabled_lifecycle_root_omits_the_member() {
        let draft = LifecycleDraft(LifecycleValues::failed(
            "build",
            Vec::new(),
            "build",
            Vec::new(),
        ));
        let report = draft.into_report(None);
        assert!(report.trace.is_none());
    }
}
