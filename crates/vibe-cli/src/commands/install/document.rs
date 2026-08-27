//! The one document `vibe install` emits, and the typed outcome the fresh
//! fast path returns.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::path::Path;

use anyhow::Result;

use crate::output;

use super::report::{self, HookReportView};
use super::{InstallDisposition, InstallRun, WorldCallbackOutcome};

/// Render the one `cli-install-report` for an explicit `vibe install`.
///
/// This is the ONLY place the install command prints a machine document, on
/// every path it can take. Callers that are not the outermost command — a
/// phase verb's prerequisite install, update, reinstall — never call it and
/// render their own command-correct report instead.
pub(crate) fn emit_command_document(ctx: &output::Context, run: InstallRun) -> Result<()> {
    let hooks = HookReportView::new(&run.hooks, &[]);
    // Both kinds of row this command ran: the slot-scoped rows from the
    // install barrier, then the `phase:install` ritual rows.
    let mut rows = report::contribution_rows(&run.slot_reports);
    rows.extend(report::phase_rows(&run.contributions));
    report::emit_install_document(
        ctx,
        report::InstallOutcome {
            project_root: &run.project_root,
            progress: &run.progress,
            hooks: &hooks,
            contributions: rows,
            notices: &run.notices,
            delegation: run.parked.as_ref(),
            world_summary: run.world_summary,
        },
    )
}

/// The fresh fast path as a typed outcome: nothing moved, so progress says
/// exactly that and the caller renders it.
pub(super) fn fresh_run(
    project_root: &Path,
    nodes: Vec<String>,
    world: WorldCallbackOutcome,
) -> InstallRun {
    let mut run = InstallRun::new(
        project_root.to_path_buf(),
        if world.parked.is_some() {
            InstallDisposition::Parked
        } else {
            InstallDisposition::Fresh
        },
    );
    run.progress = vibe_install::InstallProgress::fresh(nodes);
    run.contributions = world.contributions;
    run.notices = world.notices;
    run.parked = world.parked;
    run.world_summary = world.summary;
    run
}
