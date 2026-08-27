//! Surfacing the selected ritual: the machine plan document and its human
//! narration. Selection only — nothing here runs a contribution.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use anyhow::Result;
use vibe_core::manifest::ExtensionHandler;
use vibe_lifecycle::{ContributionTier, ExtensionProvider, RunMetadata};
use vibe_wire::generated::lifecycle_plan::{LifecyclePlan, PlannedContribution};

use crate::output;

use super::world;

pub(super) fn surface_plan(
    ctx: &output::Context,
    plan: &world::RitualPlan,
    metadata: &RunMetadata,
    emit_empty: bool,
) -> Result<()> {
    if !emit_empty && plan.executions.is_empty() && plan.notices.is_empty() {
        return Ok(());
    }
    if ctx.is_json() {
        return ctx.defer_json_plan(&LifecyclePlan {
            chain: metadata.chain.clone(),
            command: "lifecycle:plan".to_string(),
            contributions: plan.executions.iter().map(planned_contribution).collect(),
            notices: plan.notices.clone(),
            requested: metadata.requested.clone(),
        });
    }
    render_ritual(ctx, &plan.notices, &plan.executions);
    Ok(())
}

pub(super) fn planned_contribution(execution: &world::PlannedExecution) -> PlannedContribution {
    let row = &execution.row;
    let (provider, version) = provider_and_version(row.provider());
    PlannedContribution {
        handler: row.declaration().handler.kind().to_string(),
        key: row.key().to_string(),
        phase: execution.phase.clone(),
        point: row.declaration().point.to_string(),
        provider,
        reference: None,
        slot_target: None,
        tier: tier_name(row.effective_tier()).to_string(),
        version,
    }
}

pub(super) fn provider_and_version(provider: &ExtensionProvider) -> (String, Option<String>) {
    match provider {
        ExtensionProvider::Dependency(provider) => {
            (provider.id.to_string(), Some(provider.version.clone()))
        }
        ExtensionProvider::Host(provider) => (
            provider.identity.to_string(),
            (!provider.version.is_empty()).then(|| provider.version.clone()),
        ),
    }
}

pub(super) fn render_ritual(
    ctx: &output::Context,
    notices: &[String],
    executions: &[world::PlannedExecution],
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

pub(super) const fn tier_name(tier: ContributionTier) -> &'static str {
    match tier {
        ContributionTier::Preset => "preset",
        ContributionTier::Dependency => "dependency",
        ContributionTier::HostDeclaration => "host-declaration",
        ContributionTier::HostActivation => "host-activation",
    }
}
