//! Surfacing the selected ritual: the machine plan document and its human
//! narration. Selection only — nothing here runs a contribution.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use anyhow::Result;
use vibe_core::manifest::ExtensionHandler;
use vibe_lifecycle::RunMetadata;
use vibe_orchestrator::{planned_contribution, tier_name};
use vibe_wire::generated::lifecycle_plan::LifecyclePlan;

use crate::output;

pub(super) fn surface_cli_plan(
    ctx: &output::Context,
    plan: &vibe_orchestrator::RitualPlan,
    metadata: &RunMetadata,
    emit_empty: bool,
) -> Result<()> {
    if !emit_empty && plan.executions().is_empty() && plan.notices().is_empty() {
        return Ok(());
    }
    if ctx.is_json() {
        return ctx.defer_json_plan(&LifecyclePlan {
            chain: metadata.chain.clone(),
            command: "lifecycle:plan".to_string(),
            contributions: plan.executions().iter().map(planned_contribution).collect(),
            notices: plan.notices().to_vec(),
            requested: metadata.requested.clone(),
        });
    }
    render_ritual(ctx, plan.notices(), plan.executions());
    Ok(())
}

pub(super) fn render_ritual(
    ctx: &output::Context,
    notices: &[String],
    executions: &[vibe_orchestrator::PlannedExecution],
) {
    if ctx.is_json() || ctx.is_quiet() {
        return;
    }
    for notice in notices {
        ctx.step(&format!("lifecycle notice: {notice}"));
    }
    for execution in executions {
        let row = &execution.row;
        let handler = match &row.declaration().handler {
            ExtensionHandler::Builtin { name } => format!("builtin:{name}"),
            other => other.kind().to_string(),
        };
        ctx.step(&format!(
            "will run `{}` — point={}, handler={}, provider={} tier={}",
            row.key(),
            row.declaration().point,
            handler,
            row.provider(),
            tier_name(row.effective_tier()),
        ));
    }
}
