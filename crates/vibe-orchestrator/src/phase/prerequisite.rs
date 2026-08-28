//! The named collector a phase chain's prerequisite install reports back
//! through, and the law that decides which tree the LATER phases plan against.
//!
//! This is its own module for one reason: the captured post-install tree is
//! reachable ONLY through [`PrerequisiteInstall::planning_workspace`]. The
//! previous shape kept the field beside its single reader, and the reader was
//! `collector.workspace.unwrap_or(&prelude)` — a silent fall back to the
//! PRE-install world whenever the stage had not run. Making the field private
//! to this module is the construction proof that no call site can reach past
//! the law again; review no longer has to catch it.

use anyhow::{Context, Result};
use vibe_wire::generated::lifecycle_report::LifecycleContributionReport;

use crate::ports::AfterDurableWorld;
use crate::values::contribution_report;

/// What the phase loop needs back out of its prerequisite install.
///
/// A NAMED collector, not a closure: the three things this run must recover —
/// the shared slot run it continues into, the rows that run produced, and the
/// exact post-install tree the later phases plan against — are a stated shape.
/// A `FnOnce` here is what let a surface's whole rendering context cross the
/// lower boundary, and it could not say what it captured.
#[derive(Default)]
pub(super) struct PrerequisiteInstall {
    /// The slot run the install began, so the phase dispatch continues into it
    /// rather than starting a second run beside it.
    lifecycle_run: Option<vibe_lifecycle::LifecycleRunHandle>,
    /// Its slot rows, already projected into contribution shape.
    rows: Vec<LifecycleContributionReport>,
    /// The exact tree the install finished with — captured, never re-read,
    /// because it is the only copy carrying that install's `--git` delta.
    /// Private to this module ON PURPOSE: see the module note.
    workspace: Option<vibe_workspace::Workspace>,
    /// How many times the stage ran. The core consumes it exactly once, on the
    /// one branch that completes, and never fabricates a call on a failure or
    /// a park.
    calls: usize,
}

impl PrerequisiteInstall {
    /// The tree the LATER phases plan against, once the chain's own shape is
    /// known.
    ///
    /// A chain that contains Install and neither parked nor failed MUST have
    /// consumed the stage exactly once, and that call is the only place the
    /// post-install tree exists. The previous `unwrap_or(&prelude)` silently
    /// planned the remaining phases against the PRE-install world whenever the
    /// stage was skipped — a `--git` delta and every freshly materialised slot
    /// would simply be missing, and nothing would say so. Zero calls or two are
    /// therefore internal errors, not fallbacks.
    ///
    /// A chain with no Install phase legitimately never runs the stage: it has
    /// nothing to install, so the prelude load IS its world, and the call count
    /// must be zero.
    pub(super) fn planning_workspace<'a>(
        &'a self,
        prelude: &'a vibe_workspace::Workspace,
        chain_installs: bool,
    ) -> Result<&'a vibe_workspace::Workspace> {
        if !chain_installs {
            anyhow::ensure!(
                self.calls == 0,
                "internal: the post-durability stage ran {} time(s) on a chain with no install phase",
                self.calls,
            );
            return Ok(prelude);
        }
        anyhow::ensure!(
            self.calls == 1,
            "internal: the prerequisite install's post-durability stage ran {} time(s); exactly one call owns the tree the later phases plan against",
            self.calls,
        );
        self.workspace.as_ref().context(
            "internal: the prerequisite install's post-durability stage reported no workspace",
        )
    }

    /// The rows measured so far, taken out for a park's own report.
    pub(super) fn take_rows(&mut self) -> Vec<LifecycleContributionReport> {
        std::mem::take(&mut self.rows)
    }

    /// The rows measured so far, borrowed for this run's frozen prefix.
    pub(super) fn rows(&self) -> &[LifecycleContributionReport] {
        &self.rows
    }

    /// The slot run the install began, if it began one — consuming the
    /// collector, because the run is continued into and never observed twice.
    pub(super) fn into_lifecycle_run(self) -> Option<vibe_lifecycle::LifecycleRunHandle> {
        self.lifecycle_run
    }
}

impl AfterDurableWorld for PrerequisiteInstall {
    fn after(
        &mut self,
        _project_root: &std::path::Path,
        run: crate::install::InstallRunContext,
        workspace: &vibe_workspace::Workspace,
    ) -> Result<crate::install::WorldCallbackOutcome> {
        self.calls += 1;
        self.lifecycle_run = run.lifecycle_run;
        self.rows = run
            .lifecycle_reports
            .into_iter()
            .map(contribution_report)
            .collect();
        // Captured, not re-read: this is the exact tree the install finished
        // with. A phase verb runs its OWN world stage afterwards, so this one
        // contributes nothing of its own.
        self.workspace = Some(workspace.clone());
        Ok(crate::install::WorldCallbackOutcome::default())
    }
}

#[cfg(test)]
mod tests;
