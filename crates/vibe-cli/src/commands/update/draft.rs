//! The owned `cli-update-report` draft, and the one place it is rendered.
//!
//! Construction and rendering are split for the same reason they are in
//! [`crate::commands::install::InstallDraft`]: everything fallible about an
//! update report — measuring progress, validating a hosted handoff — has
//! already happened by the time a draft exists, so the trace funnel can sit
//! between the two. The command's outcome is fully decided before one byte is
//! emitted, and the member the funnel returns is attached on the way out.
//!
//! Nothing here disposes of the deferred JSON plan previews. That decision
//! belongs to the funnel's typed [`crate::commands::compile_trace::PlanDisposition`]
//! and is performed once by the adapter; a renderer that also flushed would
//! either double-print a preview or print one beside a handoff that is
//! supposed to stand alone.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::path::{Path, PathBuf};

use anyhow::Result;
use vibe_install::{InstallProgress, SlotLifecycleReport};
use vibe_wire::generated::shared::CompileTraceReport;
use vibe_wire::generated::update_report::{
    UpdateContributionReport, UpdateDelegation, UpdateReport, UpdateReportScope,
};

use crate::cli::UpdateArgs;
use crate::commands::install::{HookReportPresentation, LifecycleHookView};
use crate::output;

/// Everything the one update document reports, owned.
#[derive(Debug)]
pub(crate) struct UpdateDraft {
    pub(crate) ok: bool,
    pub(crate) project_root: PathBuf,
    pub(crate) scope: UpdateReportScope,
    pub(crate) packages: Vec<String>,
    pub(crate) packages_resolved: usize,
    pub(crate) bumps: Vec<String>,
    pub(crate) progress: InstallProgress,
    pub(crate) rows: Vec<SlotLifecycleReport>,
    /// Already validated by [`crate::commands::lifecycle::check_delegation`]
    /// before the draft was built — a malformed handoff is a failed exit, not
    /// a rendering refusal.
    pub(crate) delegation: Option<UpdateDelegation>,
}

/// The invocation facts every update draft repeats: which node, which scope,
/// which packages. Carried as one value so a failure draft cannot describe a
/// different command than the success draft beside it.
#[derive(Debug, Clone)]
pub(crate) struct UpdateIdentity {
    pub(crate) project_root: PathBuf,
    pub(crate) scope: UpdateReportScope,
    pub(crate) packages: Vec<String>,
}

impl UpdateIdentity {
    pub(crate) fn from_args(project_root: &Path, args: &UpdateArgs) -> Self {
        Self {
            project_root: project_root.to_path_buf(),
            scope: if args.all || args.packages.is_empty() {
                UpdateReportScope::All
            } else {
                UpdateReportScope::Scoped
            },
            packages: args.packages.clone(),
        }
    }
}

impl UpdateDraft {
    /// A completed or parked `vibe update`.
    pub(crate) fn completed(
        identity: &UpdateIdentity,
        packages_resolved: usize,
        bumps: Vec<String>,
        progress: InstallProgress,
        rows: Vec<SlotLifecycleReport>,
        delegation: Option<&vibe_lifecycle::Delegation>,
    ) -> Self {
        Self {
            ok: true,
            project_root: identity.project_root.clone(),
            scope: identity.scope.clone(),
            packages: identity.packages.clone(),
            packages_resolved,
            bumps,
            progress,
            rows,
            delegation: delegation.map(handoff_member),
        }
    }

    /// The draft a FAILED update carries: `ok: false`, and the progress, bumps
    /// and rows the run really measured before it stopped.
    ///
    /// Not an empty record. A failure after pruning and materialisation has
    /// already made several things durable, and a report that listed none of
    /// them would describe a run that never happened. `complete` is forced
    /// false by [`Self::into_report`] regardless of what the accumulator said,
    /// because the COMMAND did not complete.
    pub(crate) fn failed(
        identity: &UpdateIdentity,
        packages_resolved: usize,
        bumps: Vec<String>,
        progress: InstallProgress,
        rows: Vec<SlotLifecycleReport>,
    ) -> Self {
        Self {
            ok: false,
            project_root: identity.project_root.clone(),
            scope: identity.scope.clone(),
            packages: identity.packages.clone(),
            packages_resolved,
            bumps,
            progress,
            rows,
            delegation: None,
        }
    }

    /// The generated root, with the member attached — pure, total, and the
    /// ONLY place an update report is built.
    pub(crate) fn into_report(self, trace: Option<CompileTraceReport>) -> UpdateReport {
        UpdateReport {
            ok: self.ok,
            command: "update".into(),
            project: vibe_core::machine_json_path(&self.project_root),
            scope: self.scope,
            packages: self.packages,
            packages_resolved: u32::try_from(self.packages_resolved).unwrap_or(u32::MAX),
            version_bumps: self.bumps,
            // Command-level completion, decided once here: a parked update has
            // a finished materialisation and an unfinished command, and a
            // failed one never completed at all.
            complete: self.ok && self.progress.complete && self.delegation.is_none(),
            unchanged: self.progress.fresh,
            materialised: self.progress.materialised,
            skipped: self.progress.skipped,
            pruned: self.progress.pruned,
            nodes_regenerated: self.progress.nodes_regenerated,
            // Emitted even when empty: the pre-R7.3 document carried these
            // members unconditionally, and the per-row echo that used to expose
            // slot rows is gone, so this is now their only home.
            hooks: Vec::new(),
            contributions: self.rows.iter().map(contribution_row).collect(),
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
        let count = self.packages_resolved;
        let scoped = self.scope == UpdateReportScope::Scoped;
        let bumps = self.bumps.clone();
        let rows = self.rows.clone();
        let report = self.into_report(trace);
        if ctx.is_json() {
            // The ordinary path uses the SAME generated root as the parked one
            // — an ad-hoc object here would mean the command's format depended
            // on its runtime status, which is what the registered format exists
            // to prevent. Both scopes have always emitted it.
            return ctx.emit_json(&report);
        }
        if !ok {
            // A failed update's account on the terminal is its error; it has
            // never printed a second, summary-shaped one.
            return Ok(());
        }
        if let Some(delegation) = report.delegation.as_ref() {
            // A park narrates in BOTH scopes, and always has: the handoff is
            // the whole point of the invocation.
            crate::commands::lifecycle::render_agent_task_fence(
                ctx,
                &delegation.run_id,
                &delegation.tasks,
                &delegation.resume,
            );
            ctx.summary(&format!(
                "vibe update: parked for the hosting agent — {} task(s) await it; \
                 resume with `{}`{quiet_suffix}",
                delegation.tasks.len(),
                delegation.resume,
            ));
            return Ok(());
        }
        let quiet = ctx.is_quiet();
        // The bump list is prose, and only the verbose scoped shape has ever
        // printed it.
        if scoped && !quiet {
            for bump in &bumps {
                ctx.created(bump);
            }
        }
        let hooks = LifecycleHookView::new(&rows);
        if let Some(line) = success_line(Presentation {
            scoped,
            quiet,
            count,
            bumps: bumps.len(),
            hooks: &HookReportPresentation::quiet_suffix(&hooks),
            quiet_suffix,
        }) {
            ctx.summary(&line);
        }
        Ok(())
    }
}

/// The inputs the success-line policy decides from.
struct Presentation<'a> {
    scoped: bool,
    quiet: bool,
    count: usize,
    bumps: usize,
    hooks: &'a str,
    quiet_suffix: &'a str,
}

/// What a SUCCESSFUL update prints on a terminal — `None` where it has always
/// printed nothing.
///
/// The policy is a table rather than a fall-through because the two scopes
/// really do differ, and defaulting one to the other changes bytes an operator
/// or a script already depends on:
///
/// ```text
/// scoped, human   →  "\nUpdated N package(s) (M version bump(s))."
/// scoped, quiet   →  one line, with the hook and trace suffixes
/// all,    human   →  nothing — the substrate already narrated, and this
///                    command has never added a completion summary
/// all,    quiet   →  nothing, UNLESS a trace suffix exists
/// ```
///
/// That last exception is the narrowest one that makes the new feature
/// observable at all. A whole update's compile-trace summary has nowhere else
/// to go in quiet mode: human mode prints the adapter's table and JSON carries
/// the member, while quiet's entire contract is a single line. With tracing off
/// the suffix is empty, no line is printed, and the trace-disabled bytes are
/// exactly what they were.
fn success_line(inputs: Presentation<'_>) -> Option<String> {
    let Presentation {
        scoped,
        quiet,
        count,
        bumps,
        hooks,
        quiet_suffix,
    } = inputs;
    let packages = if count == 1 { "" } else { "s" };
    if !scoped {
        if !quiet || quiet_suffix.is_empty() {
            return None;
        }
        return Some(format!(
            "vibe update: {count} package{packages} re-resolved{quiet_suffix}"
        ));
    }
    let moved = if bumps == 1 { "" } else { "s" };
    if quiet {
        return Some(format!(
            "vibe update: {count} package{packages} re-resolved, {bumps} \
             bump{moved}{hooks}{quiet_suffix}"
        ));
    }
    Some(format!(
        "\nUpdated {count} package{packages} ({bumps} version bump{moved})."
    ))
}

fn handoff_member(delegation: &vibe_lifecycle::Delegation) -> UpdateDelegation {
    UpdateDelegation {
        resume: delegation.resume.clone(),
        run_id: delegation.run_id.clone(),
        tasks: delegation.tasks.clone(),
    }
}

fn contribution_row(row: &SlotLifecycleReport) -> UpdateContributionReport {
    UpdateContributionReport {
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
