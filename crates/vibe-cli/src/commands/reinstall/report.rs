//! Presentation for `vibe reinstall` outcomes.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#cli-surface");

use anyhow::Result;

use crate::commands::install::HookReportView;
use crate::output;

pub(super) fn emit(
    ctx: &output::Context,
    forced: bool,
    nodes_regenerated: &[String],
    pruned: &[String],
    hook_reports: &HookReportView<'_>,
) -> Result<()> {
    if ctx.is_json() {
        ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "reinstall",
            "forced": forced,
            "nodes_regenerated": nodes_regenerated,
            "pruned": pruned,
            "hooks": hook_reports.json(),
        }))?;
        return Ok(());
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
    hook_reports.render_human(ctx);
    Ok(())
}
