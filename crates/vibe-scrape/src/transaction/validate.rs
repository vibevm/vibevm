//! Closed preparation and recovery-journal validation.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-E");

use std::collections::{BTreeMap, BTreeSet};

use super::model::*;
use super::sha256::digest as bytes_digest;

pub fn prepared(value: &PreparedTransaction) -> Result<(), TransactionError> {
    digest(&value.plan_id)?;
    if value.project_identity_token.is_empty() || value.project_display_root.is_empty() {
        return invalid("project identity token/display root is empty");
    }
    canonical_plan(
        &value.canonical_plan,
        &value.plan_id,
        &value.project_display_root,
        value.mode(),
    )?;
    let records = value
        .snapshots
        .iter()
        .map(|snapshot| SnapshotRecord {
            kind: snapshot.kind,
            name: snapshot.name.clone(),
            sha256: bytes_digest(&snapshot.bytes),
            bytes: snapshot.bytes.len() as u64,
            mode: snapshot.mode,
        })
        .collect::<Vec<_>>();
    snapshots(&records)?;
    let embedded_snapshot = value
        .snapshots
        .iter()
        .find(|snapshot| snapshot.kind == SnapshotKind::CanonicalPlan)
        .ok_or_else(|| invalid_error("canonical plan snapshot is absent"))?;
    if embedded_snapshot.bytes != value.canonical_plan {
        return invalid("embedded canonical plan differs from its snapshot");
    }
    execution(value.mode(), &value.mode, &snapshot_names(&records))?;
    prepared_after_records(&value.mode, &records)
}

pub fn journal(
    value: &Journal,
    expected_key: &ProjectKey,
    display_root: &str,
) -> Result<(), TransactionError> {
    if value.schema != 1 || &value.project_key != expected_key {
        return corrupt("journal schema/project key mismatch");
    }
    digest(&Digest(value.project_key.0.clone()))?;
    digest(&value.plan_id)?;
    canonical_plan(
        &value.canonical_plan,
        &value.plan_id,
        &value.project_display_root,
        value.mode,
    )?;
    transaction_id(&value.transaction_id)?;
    if value.project_display_root != display_root {
        return corrupt("journal display root differs from the locked recovery root");
    }
    snapshots(&value.snapshots)?;
    if value.snapshots_persisted > value.snapshots.len() {
        return corrupt("snapshot progress exceeds the expected snapshot set");
    }
    if let Some(active) = value.snapshot_active
        && (active != value.snapshots_persisted || active >= value.snapshots.len())
    {
        return corrupt("active snapshot is not the exact next durable-prefix member");
    }
    execution(
        value.mode,
        &value.execution,
        &snapshot_names(&value.snapshots),
    )?;
    prepared_after_records(&value.execution, &value.snapshots)?;
    state_for_mode(value.mode, &value.state)?;
    validate_names(value)?;
    validate_verification_workspace(value)?;
    validate_counters(value)?;
    validate_progress(value)?;
    validate_cleanup(value)?;
    validate_state_progress(value)?;
    validate_verification(value)?;
    validate_report(value)?;
    if value
        .settlement_intent
        .is_some_and(|outcome| outcome != Outcome::Refused)
        || (value.settlement_intent.is_some() && value.mode != TransactionMode::Export)
    {
        return corrupt("settlement intent is outside typed export refusal direction");
    }
    if let (Some(intent), Some(report)) = (value.settlement_intent, value.report.as_ref())
        && report.outcome != intent
        && !(intent == Outcome::Refused && report.outcome == Outcome::RollbackFailed)
    {
        return corrupt("embedded report contradicts durable settlement intent");
    }
    let incomplete_refusal = value.state == TransactionState::Complete
        && value
            .report
            .as_ref()
            .is_some_and(|report| report.outcome == Outcome::Refused);
    if value.state != TransactionState::Preparing
        && !incomplete_refusal
        && value.snapshots_persisted != value.snapshots.len()
    {
        return corrupt("non-preparing journal has incomplete snapshot progress");
    }
    if value.state != TransactionState::Preparing
        && !incomplete_refusal
        && value.snapshot_active.is_some()
    {
        return corrupt("non-preparing journal carries an active snapshot intent");
    }
    if value.state == TransactionState::Preparing
        && (value.candidate_name.is_some()
            || value.quarantine_name.is_some()
            || value.owned_tree_token.is_some()
            || value.owned_tree_seal.is_some()
            || value.cleanup_wal.is_some()
            || value.completed_steps != 0
            || value.active_step.is_some()
            || !value.actual_mutations.is_empty()
            || !value.verification.is_empty()
            || value.report.is_some())
    {
        return corrupt("preparation journal carries post-preparation state");
    }
    Ok(())
}

fn validate_verification_workspace(value: &Journal) -> Result<(), TransactionError> {
    let Some(workspace) = &value.verification_workspace else {
        if value.state != TransactionState::Preparing {
            return corrupt("non-preparing journal has no verification workspace");
        }
        return Ok(());
    };
    if workspace.name != "v"
        || workspace.display_root.is_empty()
        || !std::path::Path::new(&workspace.display_root).is_absolute()
    {
        return corrupt("verification workspace name/display root is invalid");
    }
    digest(&Digest(workspace.ownership_token.clone()))?;
    let mut material = b"vibe-scrape-verification-workspace-e1\0".to_vec();
    material.extend_from_slice(value.project_key.0.as_bytes());
    material.push(0);
    material.extend_from_slice(value.transaction_id.0.as_bytes());
    if workspace.ownership_token != bytes_digest(&material).0 {
        return corrupt("verification workspace ownership token is not transaction-derived");
    }
    Ok(())
}

const MAX_CANONICAL_PLAN_BYTES: usize = 16 * 1024 * 1024;

include!("validate/prepared.rs");
include!("validate/mutation.rs");
include!("validate/progress.rs");
include!("validate/report.rs");
