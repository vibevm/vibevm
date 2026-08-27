//! The owned `cli-install-report` draft, and the one place it is rendered.
//!
//! Construction and rendering are split on purpose. Everything fallible about
//! an install report — measuring progress, validating a hosted handoff — has
//! already happened by the time a draft exists; what is left is building a
//! generated struct and writing it, and only the write can fail. That is what
//! lets the trace funnel sit between the two: the command's outcome is fully
//! decided (so the trace can be finalised against it) before one byte is
//! emitted, and the member the funnel returns is simply attached on the way
//! out.
//!
//! A FAILED draft narrates nothing in human mode. That is not an omission:
//! `vibe install`'s failure report has always been a machine document, with
//! the prose half being the error on stderr. Rendering a summary line here
//! would put a second, cheerier account of the same event on the terminal.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::path::{Path, PathBuf};

use anyhow::Result;
use vibe_install::{InstallProgress, SlotLifecycleReport};
use vibe_wire::generated::install_report::{
    InstallContributionReport, InstallDelegation, InstallReport,
};
use vibe_wire::generated::shared::CompileTraceReport;
use vibe_workspace::hooks::HookReport;

use crate::output;

use super::report::{self, HookReportView};
use super::{InstallRun, WorldCallbackSummary};

/// Everything the one install document reports, owned.
#[derive(Debug)]
pub(crate) struct InstallDraft {
    pub(crate) ok: bool,
    pub(crate) project_root: PathBuf,
    pub(crate) progress: InstallProgress,
    pub(crate) hooks: Vec<HookReport>,
    pub(crate) contributions: Vec<InstallContributionReport>,
    pub(crate) notices: Vec<String>,
    pub(crate) delegation: Option<InstallDelegation>,
    pub(crate) world_summary: WorldCallbackSummary,
}

impl InstallDraft {
    /// The draft for a completed, fresh or parked `vibe install`.
    ///
    /// Both kinds of row this command ran land here: the slot-scoped rows from
    /// the install barrier, then the `phase:install` ritual rows. `vibe
    /// install` is the outermost command on this path, so its single report is
    /// the only place either kind can be observed.
    pub(crate) fn from_run(run: InstallRun) -> Self {
        let mut contributions = report::contribution_rows(&run.slot_reports);
        contributions.extend(report::phase_rows(&run.contributions));
        Self {
            ok: true,
            project_root: run.project_root,
            progress: run.progress,
            hooks: run.hooks,
            contributions,
            notices: run.notices,
            delegation: run.parked.as_ref().map(report::handoff_member),
            world_summary: run.world_summary,
        }
    }

    /// The fresh fast path as a typed outcome: nothing moved, so progress says
    /// exactly that and the caller renders it.
    pub(crate) fn fresh_run(
        project_root: &Path,
        nodes: Vec<String>,
        world: super::WorldCallbackOutcome,
    ) -> InstallRun {
        let mut run = InstallRun::new(
            project_root.to_path_buf(),
            if world.parked.is_some() {
                super::InstallDisposition::Parked
            } else {
                super::InstallDisposition::Fresh
            },
        );
        run.progress = InstallProgress::fresh(nodes);
        run.contributions = world.contributions;
        run.notices = world.notices;
        run.parked = world.parked;
        run.world_summary = world.summary;
        run
    }

    /// The draft a FAILED install carries: `ok: false`, the rows and progress
    /// the engine really measured, and nothing invented to fill the rest.
    ///
    /// Hooks and notices are empty because a run that stopped at a slot row
    /// never produced them — a report that listed them would be describing a
    /// run that did not happen.
    pub(crate) fn failed(
        project_root: &Path,
        progress: InstallProgress,
        slot_reports: Vec<SlotLifecycleReport>,
    ) -> Self {
        Self {
            ok: false,
            project_root: project_root.to_path_buf(),
            contributions: report::contribution_rows(&slot_reports),
            progress,
            hooks: Vec::new(),
            notices: Vec::new(),
            delegation: None,
            world_summary: WorldCallbackSummary::default(),
        }
    }

    /// The generated root, with the member attached — pure, total, and the
    /// ONLY place an install report is built.
    ///
    /// Separate from rendering so the attachment can be proved directly. The
    /// interesting cases are all about a member the command's own outcome
    /// disagrees with: a SUCCESSFUL command whose final index publication was
    /// refused still owes a `running`/`finalised: false` member, because the
    /// on-disk index says exactly that and always will. Rewriting it from the
    /// command's success — or dropping it as uninteresting — is the one lie a
    /// cold reader cannot detect.
    pub(crate) fn into_report(self, trace: Option<CompileTraceReport>) -> InstallReport {
        let hooks = HookReportView::new(&self.hooks, &[]);
        InstallReport {
            ok: self.ok,
            command: "install".into(),
            project: vibe_core::machine_json_path(&self.project_root),
            // `complete` is the COMMAND's completion, not the materialise
            // pass's. A park after the barrier has a finished materialisation
            // and an unfinished command: it is waiting on the hosting agent
            // and will be resumed.
            complete: self.progress.complete && self.delegation.is_none(),
            unchanged: self.progress.fresh,
            materialised: self.progress.materialised.clone(),
            skipped: self.progress.skipped.clone(),
            pruned: self.progress.pruned.clone(),
            nodes_regenerated: self.progress.nodes_regenerated.clone(),
            contributions: self.contributions,
            notices: self.notices,
            hooks: hooks.typed(),
            delegation: self.delegation,
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
        let progress = self.progress.clone();
        let hooks_owned = self.hooks.clone();
        let world_summary = self.world_summary;
        let report = self.into_report(trace);
        let hooks = HookReportView::new(&hooks_owned, &[]);
        if ctx.is_json() {
            return ctx.emit_json(&report);
        }
        if !ok {
            // See the module note: a failed install's account is its error.
            return Ok(());
        }
        if let Some(delegation) = report.delegation.as_ref() {
            crate::commands::lifecycle::render_agent_task_fence(
                ctx,
                &delegation.run_id,
                &delegation.tasks,
                &delegation.resume,
            );
            ctx.summary(&format!(
                "vibe install: parked for the hosting agent — {} task(s) await it; \
                 {} slot(s) materialised so far, resume with `{}`{quiet_suffix}",
                delegation.tasks.len(),
                progress.materialised.len(),
                delegation.resume,
            ));
            return Ok(());
        }
        report::render_human_progress(ctx, &progress, &hooks, world_summary, quiet_suffix);
        Ok(())
    }
}

#[cfg(test)]
mod refusal_tests {
    use super::*;
    use vibe_wire::behaviour::compile_trace_report::validate;
    use vibe_wire::generated::shared::TraceReportStatus;

    /// The member a run whose FINAL index publication was refused produces.
    ///
    /// The writer already decided this from the disk: the terminal bytes never
    /// landed, so the index reads `running` and always will. Both lower layers
    /// already prove that decision
    /// (`vibe-workspace .. a_post_step_index_mutation_is_not_mistaken_for_the_attempted_bytes`
    /// and `compile_trace/tests/bounds.rs ..
    /// an_unpublished_terminal_status_is_reported_as_still_running`); what was
    /// missing is the CONSUMER half — that the root carries it through unchanged.
    fn refused_member() -> CompileTraceReport {
        CompileTraceReport {
            budget_exhausted: false,
            events: "4".into(),
            finalised: false,
            run_id: "a".repeat(32),
            snapshot_bytes: "128".into(),
            snapshots: "1".into(),
            status: TraceReportStatus::Running,
            timings: Vec::new(),
            warnings: vec!["the terminal index could not be published".into()],
            run_path: Some(format!("/p/.vibe/trace/{}", "a".repeat(32))),
        }
    }

    /// A SUCCESSFUL install still reports `running` / `finalised: false` when
    /// that is what the index says.
    ///
    /// The three ways this can go wrong are all silent, and all fatal to a cold
    /// reader: dropping the member, rewriting `status` from the command's own
    /// success, or forcing `finalised` true because the command finished.
    #[test]
    fn a_successful_install_root_carries_a_refused_member_unchanged() {
        let member = refused_member();
        let draft = InstallDraft {
            ok: true,
            project_root: PathBuf::from("/p"),
            progress: InstallProgress::fresh(vec![".".into()]),
            hooks: Vec::new(),
            contributions: Vec::new(),
            notices: Vec::new(),
            delegation: None,
            world_summary: WorldCallbackSummary::default(),
        };
        let report = draft.into_report(Some(member.clone()));

        assert!(report.ok, "the COMMAND succeeded");
        let attached = report.trace.expect("and the member was not dropped");
        assert_eq!(
            attached, member,
            "carried through byte for byte — not renormalised from the command's outcome",
        );
        assert_eq!(attached.status, TraceReportStatus::Running);
        assert!(!attached.finalised);
        validate(&attached).expect("and it is still a valid member");
    }

    /// Disabled omits the member entirely — the byte-for-byte law old corpora
    /// depend on.
    #[test]
    fn a_disabled_install_root_omits_the_member() {
        let draft = InstallDraft::failed(
            std::path::Path::new("/p"),
            InstallProgress::default(),
            Vec::new(),
        );
        let report = draft.into_report(None);
        assert!(report.trace.is_none());
        assert!(
            !serde_json::to_string(&report).unwrap().contains("trace"),
            "and the key is absent from the wire, not merely null",
        );
    }
}
