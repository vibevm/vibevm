//! Presentation for `vibe reinstall` outcomes.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#cli-surface");

use anyhow::Result;

use crate::commands::install::HookReportPresentation;
use crate::output;

pub(super) fn emit(
    ctx: &output::Context,
    project_root: &std::path::Path,
    forced: bool,
    rows: &[vibe_install::SlotLifecycleReport],
    nodes_regenerated: &[String],
    pruned: &[String],
    hook_reports: &dyn HookReportPresentation,
) -> Result<()> {
    if ctx.is_json() {
        // Same generated root as the parked document: the command's registered
        // format never changes with runtime status.
        return super::document::emit_reinstall_document(
            ctx,
            project_root,
            &vibe_install::InstallProgress {
                complete: true,
                fresh: false,
                materialised: Vec::new(),
                skipped: Vec::new(),
                pruned: pruned.to_vec(),
                nodes_regenerated: nodes_regenerated.to_vec(),
            },
            forced,
            rows,
            None,
        );
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe reinstall: boot artifacts regenerated for {} node{}{}",
            nodes_regenerated.len(),
            if nodes_regenerated.len() == 1 {
                ""
            } else {
                "s"
            },
            hook_reports.quiet_suffix(),
        ));
        return Ok(());
    }
    ctx.summary(&format!(
        "\nReinstalled — regenerated boot artifacts for {} node{}{}.",
        nodes_regenerated.len(),
        if nodes_regenerated.len() == 1 {
            ""
        } else {
            "s"
        },
        if forced { " from a fresh fetch" } else { "" },
    ));
    if !pruned.is_empty() {
        ctx.step(&format!(
            "pruned {} stale vibedeps/ slot{}",
            pruned.len(),
            if pruned.len() == 1 { "" } else { "s" },
        ));
    }
    Ok(())
}
