//! One-contribution lifecycle execution with per-transition checkpoints.

use std::time::Instant;

use anyhow::Result;
use vibe_lifecycle::{
    DispatchBatch, ExecutionSession, LifecycleStateStore, RunMetadata, fingerprint_execution,
    preparation_error_fingerprint,
};
use vibe_wire::generated::lifecycle::e1::reply::{Reply, ReplyStatus};
use vibe_wire::generated::lifecycle_report::LifecycleContributionReport;
use vibe_wire::generated::lifecycle_state::{
    ExecutionRecord, ExecutionRecordStatus, StateArtifact,
};

use crate::output;

use super::world;

pub(super) fn dispatch_plan_untracked(
    ctx: &output::Context,
    plan: &world::RitualPlan,
    metadata: RunMetadata,
) -> Result<Vec<LifecycleContributionReport>> {
    let mut session = ExecutionSession::new(plan.project.clone(), plan.world.clone(), metadata);
    let mut reports = Vec::with_capacity(plan.executions.len());
    let mut cursor = 0;
    while cursor < plan.executions.len() {
        let phase = plan.executions[cursor].phase.clone();
        let end = plan.executions[cursor..]
            .iter()
            .position(|execution| execution.phase != phase)
            .map_or(plan.executions.len(), |offset| cursor + offset);
        let rows = plan.executions[cursor..end]
            .iter()
            .map(|execution| execution.row.clone())
            .collect::<Vec<_>>();
        let DispatchBatch { outcomes, failure } = session.dispatch_phase(&phase, &rows);
        for (execution, outcome) in plan.executions[cursor..end].iter().zip(outcomes) {
            let report = contribution_report(execution, &outcome.reply);
            render_outcome(ctx, &report);
            reports.push(report);
        }
        if let Some(failure) = failure {
            return Err(anyhow::Error::new(failure).context(format!(
                "phase `{phase}` stopped before any later lifecycle contribution"
            )));
        }
        cursor = end;
    }
    Ok(reports)
}

pub(super) fn dispatch_plan(
    ctx: &output::Context,
    plan: &world::RitualPlan,
    metadata: RunMetadata,
    state_chain: Vec<String>,
) -> Result<Vec<LifecycleContributionReport>> {
    let mut store = LifecycleStateStore::begin(
        &plan.workspace_root,
        metadata.requested.clone(),
        state_chain,
        metadata.started.clone(),
    )?;
    let force = metadata.force;
    let mut session = ExecutionSession::new(plan.project.clone(), plan.world.clone(), metadata);
    let mut reports = Vec::with_capacity(plan.executions.len());
    for execution in plan.executions.iter() {
        let key = execution.row.key().to_string();
        let started = Instant::now();
        let envelope = match session.envelope_for(&execution.phase, &execution.row) {
            Ok(envelope) => envelope,
            Err(failure) => {
                return Err(checkpoint_preparation_failure(
                    &mut store,
                    execution,
                    &key,
                    started,
                    anyhow::Error::new(failure),
                ));
            }
        };
        let fingerprint = match fingerprint_execution(&execution.row, &envelope) {
            Ok(fingerprint) => fingerprint,
            Err(failure) => {
                return Err(checkpoint_preparation_failure(
                    &mut store,
                    execution,
                    &key,
                    started,
                    anyhow::Error::new(failure),
                ));
            }
        };
        if !force && let Some(prior) = store.reusable_record(&key, &fingerprint).cloned() {
            session.hydrate_artifacts(&execution.phase, &prior.artifacts);
            store.checkpoint(
                key.clone(),
                ExecutionRecord {
                    artifacts: prior.artifacts,
                    duration_ms: 0,
                    fingerprint,
                    phase: execution.phase.clone(),
                    status: ExecutionRecordStatus::Fresh,
                },
            )?;
            let report = contribution_status_report(execution, "fresh", None);
            render_outcome(ctx, &report);
            reports.push(report);
            continue;
        }

        match session.dispatch_prepared(&execution.row, envelope) {
            Ok(outcome) => {
                let status = match outcome.reply.status {
                    ReplyStatus::Ok => ExecutionRecordStatus::Ok,
                    ReplyStatus::Skip => ExecutionRecordStatus::Skip,
                    ReplyStatus::Fail => ExecutionRecordStatus::Fail,
                };
                let artifacts = outcome
                    .reply
                    .artifacts
                    .iter()
                    .map(|artifact| StateArtifact {
                        id: artifact.id.clone(),
                        kind: artifact.kind.clone(),
                        path: artifact.path.clone(),
                    })
                    .collect();
                store.checkpoint(
                    key,
                    ExecutionRecord {
                        artifacts,
                        duration_ms: elapsed_ms(started),
                        fingerprint,
                        phase: execution.phase.clone(),
                        status,
                    },
                )?;
                let report = contribution_report(execution, &outcome.reply);
                render_outcome(ctx, &report);
                reports.push(report);
            }
            Err(failure) => {
                let primary = anyhow::Error::new(failure).context(format!(
                    "phase `{}` stopped before any later lifecycle contribution",
                    execution.phase,
                ));
                return Err(checkpoint_failure(
                    &mut store,
                    key,
                    ExecutionRecord {
                        artifacts: Vec::new(),
                        duration_ms: elapsed_ms(started),
                        fingerprint,
                        phase: execution.phase.clone(),
                        status: ExecutionRecordStatus::Fail,
                    },
                    primary,
                ));
            }
        }
    }
    Ok(reports)
}

fn checkpoint_preparation_failure(
    store: &mut LifecycleStateStore,
    execution: &world::PlannedExecution,
    key: &str,
    started: Instant,
    primary: anyhow::Error,
) -> anyhow::Error {
    let record = ExecutionRecord {
        artifacts: Vec::new(),
        duration_ms: elapsed_ms(started),
        fingerprint: preparation_error_fingerprint(execution.row.key(), &execution.phase),
        phase: execution.phase.clone(),
        status: ExecutionRecordStatus::Fail,
    };
    checkpoint_failure(store, key.to_string(), record, primary)
}

fn checkpoint_failure(
    store: &mut LifecycleStateStore,
    key: String,
    record: ExecutionRecord,
    primary: anyhow::Error,
) -> anyhow::Error {
    match store.checkpoint(key.clone(), record) {
        Ok(()) => primary,
        Err(checkpoint) => primary.context(format!(
            "also failed to checkpoint lifecycle failure for `{key}`: {checkpoint}"
        )),
    }
}

fn elapsed_ms(started: Instant) -> u32 {
    u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX)
}

fn contribution_report(
    execution: &world::PlannedExecution,
    reply: &Reply,
) -> LifecycleContributionReport {
    contribution_status_report(
        execution,
        &super::reply_status(&reply.status),
        reply.message.clone(),
    )
}

fn contribution_status_report(
    execution: &world::PlannedExecution,
    status: &str,
    message: Option<String>,
) -> LifecycleContributionReport {
    let row = &execution.row;
    let (provider, version) = super::provider_and_version(row.provider());
    LifecycleContributionReport {
        handler: row.declaration().handler.kind().to_string(),
        key: row.key().to_string(),
        message,
        phase: execution.phase.clone(),
        point: row.declaration().point.to_string(),
        provider,
        status: status.to_string(),
        tier: super::tier_name(row.effective_tier()).to_string(),
        version,
    }
}

fn render_outcome(ctx: &output::Context, report: &LifecycleContributionReport) {
    if ctx.is_json() || ctx.is_quiet() {
        return;
    }
    if report.status == "fresh" {
        ctx.step(&format!(
            "fresh `{}` — provider={}",
            report.key, report.provider
        ));
    } else if let Some(message) = &report.message {
        ctx.step(&format!("log [{}]: {message}", report.provider));
    }
}
