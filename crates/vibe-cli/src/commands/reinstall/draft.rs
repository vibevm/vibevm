//! The owned `cli-reinstall-report` draft, and the one place it is rendered.
//!
//! `vibe reinstall --force` is a MATERIALISATION force — it re-fetches from
//! source so changed-slot callbacks run. It is NOT the lifecycle's repark
//! force: `RunMetadata.force` stays false, precisely so a forced reinstall can
//! adopt and satisfy the run it parked instead of minting a new identity on
//! every resume. `forced` on the document below is the former, and nothing
//! reads it as the latter.
//!
//! Construction and rendering are split so the trace funnel can sit between
//! them, exactly as for install and update. Nothing here disposes of the
//! deferred JSON plan previews: that is the funnel's typed decision, performed
//! once by the adapter.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#cli-surface");

use specmark::spec;
use std::path::PathBuf;

use anyhow::Result;
use vibe_install::{InstallProgress, SlotLifecycleReport};
use vibe_wire::generated::reinstall_report::{
    ReinstallContributionReport, ReinstallDelegation, ReinstallReport,
};
use vibe_wire::generated::shared::CompileTraceReport;

use crate::commands::install::LifecycleHookView;
use crate::output;

/// Which invocation this document belongs to.
///
/// The report's `project` is the SELECTED node — the directory the operator
/// pointed at — even though every operational fact of a reinstall is
/// workspace-rooted: the resolver reads `workspace.root_manifest`, the slot
/// world, lock, boot artifacts, lifecycle state and trace all live at
/// `workspace.root`, and discovery deliberately bubbles there.
///
/// Those are different questions. "Which tree did this rebuild?" is the
/// workspace; "which invocation is this report about?" is the node that was
/// invoked, and an operator running `vibe reinstall` inside a member reads a
/// report that names their member. Collapsing the two — as reporting
/// `workspace.root` does — makes two different invocations produce
/// indistinguishable documents.
///
/// `forced` rides along because it is the same fact: it is the invocation's
/// `--force`, and every branch of one invocation reports the same value.
#[derive(Debug, Clone)]
pub(crate) struct ReinstallIdentity {
    /// The canonical SELECTED project root, not the workspace root.
    pub(crate) selected_project_root: PathBuf,
    /// MATERIALISATION force — see the module note.
    pub(crate) forced: bool,
}

/// Everything the one reinstall document reports, owned.
#[derive(Debug)]
pub(crate) struct ReinstallDraft {
    pub(crate) ok: bool,
    pub(crate) project_root: PathBuf,
    /// MATERIALISATION force — see the module note.
    pub(crate) forced: bool,
    pub(crate) progress: InstallProgress,
    pub(crate) rows: Vec<SlotLifecycleReport>,
    pub(crate) delegation: Option<ReinstallDelegation>,
}

impl ReinstallDraft {
    /// A completed or parked `vibe reinstall`.
    pub(crate) fn completed(
        identity: &ReinstallIdentity,
        progress: InstallProgress,
        rows: Vec<SlotLifecycleReport>,
        delegation: Option<&vibe_lifecycle::Delegation>,
    ) -> Self {
        Self {
            ok: true,
            project_root: identity.selected_project_root.clone(),
            forced: identity.forced,
            progress,
            rows,
            delegation: delegation.map(handoff_member),
        }
    }

    /// The draft a FAILED reinstall carries: `ok: false`, and the rows and
    /// progress the run really measured before it stopped.
    ///
    /// A forced reinstall that fails after materialising several slots has
    /// made those durable; reporting an empty run would describe something
    /// that did not happen. `complete` is forced false by [`Self::into_report`]
    /// whatever the accumulator says, because the COMMAND did not complete.
    pub(crate) fn failed(
        identity: &ReinstallIdentity,
        progress: InstallProgress,
        rows: Vec<SlotLifecycleReport>,
    ) -> Self {
        Self {
            ok: false,
            project_root: identity.selected_project_root.clone(),
            forced: identity.forced,
            progress,
            rows,
            delegation: None,
        }
    }

    /// The generated root, with the member attached — pure, total, and the
    /// ONLY place a reinstall report is built.
    pub(crate) fn into_report(self, trace: Option<CompileTraceReport>) -> ReinstallReport {
        ReinstallReport {
            ok: self.ok,
            command: "reinstall".into(),
            project: vibe_core::machine_json_path(&self.project_root),
            forced: self.forced,
            // Command-level completion: a parked reinstall has a finished
            // materialisation and an unfinished command, and a failed one
            // never completed at all.
            complete: self.ok && self.progress.complete && self.delegation.is_none(),
            unchanged: self.progress.fresh,
            materialised: self.progress.materialised,
            skipped: self.progress.skipped,
            pruned: self.progress.pruned,
            nodes_regenerated: self.progress.nodes_regenerated,
            contributions: self.rows.iter().map(contribution_row).collect(),
            delegation: self.delegation,
            trace,
        }
    }

    #[spec(implements = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#cli-surface")]
    pub(crate) fn render(
        self,
        ctx: &output::Context,
        trace: Option<CompileTraceReport>,
        quiet_suffix: &str,
    ) -> Result<()> {
        let ok = self.ok;
        let forced = self.forced;
        let rows = self.rows.clone();
        let report = self.into_report(trace);
        if ctx.is_json() {
            // Same generated root as the parked document: the command's
            // registered format never changes with runtime status.
            return ctx.emit_json(&report);
        }
        if !ok {
            // A failed reinstall's account on the terminal is its error.
            return Ok(());
        }
        if let Some(delegation) = report.delegation.as_ref() {
            crate::commands::lifecycle::render_agent_task_fence(
                ctx,
                &delegation.run_id,
                &delegation.tasks,
                &delegation.resume,
            );
            if ctx.is_quiet() {
                ctx.summary(&format!(
                    "vibe reinstall: parked for the hosting agent — {} task(s) await it; \
                     resume with `{}`{quiet_suffix}",
                    delegation.tasks.len(),
                    delegation.resume,
                ));
            } else {
                let n = delegation.tasks.len();
                ctx.block(&output::language::parked_trailer(
                    &format!(
                        "parked at agent tasks · {n} task{} await{} the hosting agent · resume with",
                        if n == 1 { "" } else { "s" },
                        if n == 1 { "s" } else { "" },
                    ),
                    &delegation.resume,
                ));
            }
            return Ok(());
        }
        let nodes = report.nodes_regenerated.len();
        let plural = if nodes == 1 { "" } else { "s" };
        if ctx.is_quiet() {
            let hooks = LifecycleHookView::new(&rows);
            ctx.summary(&format!(
                "vibe reinstall: boot artifacts regenerated for {nodes} node{plural}{}{quiet_suffix}",
                hooks.quiet_suffix(),
            ));
            return Ok(());
        }
        ctx.trailer(&success_what(forced, nodes, report.pruned.len()));
        Ok(())
    }
}

fn success_what(forced: bool, nodes: usize, pruned: usize) -> String {
    let plural = if nodes == 1 { "" } else { "s" };
    let mut what = if forced {
        format!("reinstalled from a fresh fetch · {nodes} boot node{plural}")
    } else {
        format!("regenerated boot for {nodes} node{plural}")
    };
    if pruned > 0 {
        what.push_str(&format!(
            " · pruned {pruned} stale slot{}",
            if pruned == 1 { "" } else { "s" },
        ));
    }
    what
}

/// The progress EVERY ordinary `vibe reinstall` SUCCESS reports — plain,
/// empty-force and normal-force alike.
///
/// This is a compatibility projection, not a measurement. `vibe reinstall` has
/// always reported the regenerated nodes and the pruned slots and nothing else:
/// `materialised` and `skipped` are empty in every trace-disabled document an
/// existing consumer has ever parsed, even on a normal `--force` run whose
/// apply really did materialise slots.
///
/// The run's own COMPLETED record (`InstallProgress::complete(&outcome)`) is
/// still recorded on the lifecycle, and every park, failure and serviced
/// continuation reports THAT — those outcomes had nothing truthful to say
/// before, so widening them is a repair rather than a change. An ordinary
/// success is the one case with characterised bytes, and this is the shape that
/// keeps them.
pub(crate) fn regenerated(nodes: Vec<String>, pruned: Vec<String>) -> InstallProgress {
    InstallProgress {
        complete: true,
        fresh: false,
        materialised: Vec::new(),
        skipped: Vec::new(),
        pruned,
        nodes_regenerated: nodes,
    }
}

fn handoff_member(delegation: &vibe_lifecycle::Delegation) -> ReinstallDelegation {
    ReinstallDelegation {
        resume: delegation.resume.clone(),
        run_id: delegation.run_id.clone(),
        tasks: delegation.tasks.clone(),
    }
}

fn contribution_row(row: &SlotLifecycleReport) -> ReinstallContributionReport {
    ReinstallContributionReport {
        key: row.key.clone(),
        phase: "install".into(),
        point: row.point.clone(),
        handler: row.handler.clone(),
        provider: row.provider.clone(),
        tier: row.tier.clone(),
        status: row.status.clone(),
        message: row.message.clone(),
        version: row.version.clone(),
        reference: Some(row.reference.clone()),
        flagged: row.flagged.then_some(true),
        stdout: row.stdout.clone(),
        stderr: row.stderr.clone(),
    }
}

#[cfg(test)]
#[path = "draft/tests.rs"]
mod tests;

#[cfg(test)]
mod display_tests {
    use super::*;

    #[test]
    fn reinstall_trailer_names_a_fresh_fetch() {
        assert_eq!(
            success_what(true, 1, 0),
            "reinstalled from a fresh fetch · 1 boot node"
        );
        assert_eq!(success_what(false, 2, 0), "regenerated boot for 2 nodes");
        assert_eq!(
            success_what(false, 1, 1),
            "regenerated boot for 1 node · pruned 1 stale slot"
        );
        assert_eq!(
            success_what(true, 3, 2),
            "reinstalled from a fresh fetch · 3 boot nodes · pruned 2 stale slots"
        );
    }

    #[test]
    fn reinstall_confirmation_refusal_has_a_code() {
        use crate::commands::reinstall::inputs::confirmation_refusal;
        let refusal = crate::output::refusal_of(&anyhow::anyhow!(confirmation_refusal(false)));
        assert_eq!(refusal.code.as_deref(), Some("confirmation_required"));
        assert_eq!(
            refusal.headline,
            "no TTY is available to confirm the reinstall"
        );
        assert_eq!(refusal.fix, ["vibe reinstall --assume-yes"]);
        assert_eq!(
            refusal.see.as_deref(),
            Some("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#cli-surface")
        );
        let forced = crate::output::refusal_of(&anyhow::anyhow!(confirmation_refusal(true)));
        assert_eq!(forced.fix, ["vibe reinstall --force --assume-yes"]);
    }
}
