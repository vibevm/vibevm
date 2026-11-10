use anyhow::Result;
use vibe_lifecycle::RunMetadata;
use vibe_wire::generated::lifecycle_plan::{LifecyclePlan, PlannedContribution};

use crate::output;

pub(crate) fn emit_transition_outcome(
    ctx: &output::Context,
    metadata: &RunMetadata,
    report: &vibe_install::SlotLifecycleReport,
) -> Result<()> {
    if ctx.is_quiet() || ctx.suppresses_output() {
        return Ok(());
    }
    // JSON narrates NOTHING per row: the outermost command emits exactly one
    // document, and every slot row this install ran reaches it as a typed
    // contribution. A per-row echo here was a second (and third) document on
    // the same stdout.
    let _ = metadata;
    if ctx.is_json() {
        return Ok(());
    }
    let target = report.slot_target.as_ref().map_or_else(
        || "unknown target".into(),
        |target| format!("{}/{}@{}", target.group, target.name, target.version),
    );
    ctx.step(&format!(
        "{} `{}` — provider={} target={target}",
        report.status, report.key, report.provider
    ));
    Ok(())
}

pub(crate) fn surface_plan(
    ctx: &output::Context,
    plan: &vibe_install::SlotLifecyclePlan,
    metadata: &RunMetadata,
) -> Result<()> {
    if plan.entries.is_empty() || ctx.suppresses_output() || ctx.is_quiet() {
        return Ok(());
    }
    if ctx.is_json() {
        return ctx.defer_json_plan(&LifecyclePlan {
            chain: metadata.chain.clone(),
            command: "lifecycle:plan".into(),
            contributions: plan
                .entries
                .iter()
                .map(|entry| PlannedContribution {
                    handler: entry.handler.clone(),
                    key: entry.key.clone(),
                    phase: "install".into(),
                    point: entry.point.clone(),
                    provider: entry.provider.clone(),
                    reference: Some(entry.reference.clone()),
                    slot_target: Some(vibe_wire::generated::lifecycle_plan::SlotTarget {
                        group: entry.slot_target.group.clone(),
                        kind: entry.slot_target.kind.clone(),
                        name: entry.slot_target.name.clone(),
                        root: entry.slot_target.root.clone(),
                        version: entry.slot_target.version.clone(),
                    }),
                    tier: entry.tier.clone(),
                    version: entry.version.clone(),
                })
                .collect(),
            notices: Vec::new(),
            requested: metadata.requested.clone(),
        });
    }
    for entry in &plan.entries {
        // PROP-065 `#ordinary-plan`: one row per contribution with its `id`,
        // its handler kind, its `title`, its needs and its `run` / `skip
        // (reason)` verdict. A skipped row names the condition that decided
        // it, so a plan never says `skip` without saying why.
        let verdict = if entry.run {
            "will run".to_string()
        } else {
            format!(
                "skip ({})",
                entry.reason.as_deref().unwrap_or("no reason recorded")
            )
        };
        let title = entry
            .title
            .as_deref()
            .map(|title| format!(" — {title}"))
            .unwrap_or_default();
        ctx.step(&format!(
            "{verdict} `{}`{title} — point={}, handler={}, provider={} target={}/{}@{} tier={}",
            entry.key,
            entry.point,
            entry.handler,
            entry.provider,
            entry.slot_target.group,
            entry.slot_target.name,
            entry.slot_target.version,
            entry.tier,
        ));
        for line in &entry.conditions {
            ctx.step(&format!("    {line}"));
        }
        if let Some(judgment) = entry.needs_line() {
            ctx.step(&format!("    {judgment}"));
        }
    }
    Ok(())
}
