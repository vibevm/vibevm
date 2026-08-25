use anyhow::Result;
use vibe_lifecycle::RunMetadata;
use vibe_wire::generated::lifecycle_plan::{LifecyclePlan, PlannedContribution};
use vibe_wire::generated::lifecycle_report::{
    LifecycleContributionReport, LifecycleReport, LifecycleStepReport,
};

use crate::output;

pub(super) fn contribution_report(
    report: vibe_install::SlotLifecycleReport,
) -> LifecycleContributionReport {
    LifecycleContributionReport {
        flagged: report.flagged.then_some(true),
        handler: report.handler,
        key: report.key,
        message: report.message,
        stderr: report.stderr,
        stderr_truncated: report.stderr_truncated.then_some(true),
        stdout: report.stdout,
        stdout_truncated: report.stdout_truncated.then_some(true),
        phase: "install".into(),
        point: report.point,
        provider: report.provider,
        reference: Some(report.reference),
        slot_target: report.slot_target.map(|target| {
            vibe_wire::generated::lifecycle_report::SlotTarget {
                group: target.group,
                kind: target.kind,
                name: target.name,
                root: target.root,
                version: target.version,
            }
        }),
        status: report.status,
        tier: report.tier,
        version: report.version,
    }
}

pub(crate) fn emit_transition_outcome(
    ctx: &output::Context,
    metadata: &RunMetadata,
    report: &vibe_install::SlotLifecycleReport,
) -> Result<()> {
    if ctx.is_quiet() || ctx.suppresses_output() {
        return Ok(());
    }
    if !ctx.is_json() {
        let target = report.slot_target.as_ref().map_or_else(
            || "unknown target".into(),
            |target| format!("{}/{}@{}", target.group, target.name, target.version),
        );
        ctx.step(&format!(
            "{} `{}` — provider={} target={target}",
            report.status, report.key, report.provider
        ));
        return Ok(());
    }
    ctx.emit_json(&LifecycleReport {
        chain: metadata.chain.clone(),
        command: "lifecycle".into(),
        contributions: vec![contribution_report(report.clone())],
        notices: Vec::new(),
        ok: report.status != "fail" || report.flagged,
        requested: metadata.requested.clone(),
        steps: vec![LifecycleStepReport {
            phase: "install".into(),
            status: report.status.clone(),
        }],
    })
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
        return ctx.emit_json(&LifecyclePlan {
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
        ctx.step(&format!(
            "will run `{}` — point={}, handler={}, provider={} target={}/{}@{} tier={}",
            entry.key,
            entry.point,
            entry.handler,
            entry.provider,
            entry.slot_target.group,
            entry.slot_target.name,
            entry.slot_target.version,
            entry.tier,
        ));
    }
    Ok(())
}
