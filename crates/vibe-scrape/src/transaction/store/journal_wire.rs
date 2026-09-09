//! Domain conversion for the generated epoch-2 transaction journal.

use serde_json::Value;
use vibe_wire::generated::scrape::e2::transaction_journal as w;

use super::super::model as d;

const EPOCH: u32 = 2;

pub(super) fn encode(value: &d::Journal) -> Result<Vec<u8>, String> {
    serde_json::to_vec(&to_wire(value)?).map_err(|error| error.to_string())
}

pub(super) fn decode(bytes: &[u8]) -> Result<d::Journal, String> {
    let wire = serde_json::from_slice::<w::TransactionJournal>(bytes).map_err(|error| {
        if authored_epoch(bytes) == Some(1) {
            legacy_refusal()
        } else {
            format!("invalid strict epoch-2 transaction journal JSON: {error}")
        }
    })?;
    if wire.schema != EPOCH {
        return Err(if wire.schema == 1 {
            legacy_refusal()
        } else {
            format!(
                "transaction journal schema {} is not supported; expected epoch {EPOCH}",
                wire.schema
            )
        });
    }
    Ok(from_wire(wire))
}

fn authored_epoch(bytes: &[u8]) -> Option<u64> {
    serde_json::from_slice::<Value>(bytes)
        .ok()?
        .get("schema")?
        .as_u64()
}

fn legacy_refusal() -> String {
    "pre-public scrape transaction journal epoch 1 cannot be resumed; remove that pending external transaction state and restart scrape from its authored contract (automatic migration is not available)".to_owned()
}

pub(super) struct IntentParts {
    pub mode: Vec<u8>,
    pub workspace: Vec<u8>,
    pub execution: Vec<u8>,
    pub snapshots: Vec<u8>,
}

pub(super) fn intent_parts(value: &d::Journal) -> Result<IntentParts, String> {
    let mode = mode_to_wire(value.mode);
    let workspace = value.verification_workspace.as_ref().map(workspace_to_wire);
    let execution = prepared_to_wire(&value.execution);
    let snapshots = value
        .snapshots
        .iter()
        .map(snapshot_to_wire)
        .collect::<Vec<_>>();
    Ok(IntentParts {
        mode: serde_json::to_vec(&mode).map_err(|error| error.to_string())?,
        workspace: serde_json::to_vec(&workspace).map_err(|error| error.to_string())?,
        execution: serde_json::to_vec(&execution).map_err(|error| error.to_string())?,
        snapshots: serde_json::to_vec(&snapshots).map_err(|error| error.to_string())?,
    })
}

macro_rules! enum_pair {
    ($to:ident, $from:ident, $domain:path, $wire:path, [$($variant:ident),+ $(,)?]) => {
        fn $to(value: $domain) -> $wire {
            match value { $(<$domain>::$variant => <$wire>::$variant),+ }
        }
        fn $from(value: $wire) -> $domain {
            match value { $(<$wire>::$variant => <$domain>::$variant),+ }
        }
    };
}

enum_pair!(
    mode_to_wire,
    mode_from_wire,
    d::TransactionMode,
    w::TransactionMode,
    [Export, InPlace]
);
enum_pair!(
    boundary_to_wire,
    boundary_from_wire,
    d::ContractBoundaryAction,
    w::ContractBoundaryAction,
    [DeleteLastMoved, ExternalPreserved]
);
enum_pair!(
    snapshot_kind_to_wire,
    snapshot_kind_from_wire,
    d::SnapshotKind,
    w::SnapshotKind,
    [
        Contract,
        CanonicalContract,
        CanonicalPlan,
        Verifier,
        PreparedAfter
    ]
);
enum_pair!(
    tree_kind_to_wire,
    tree_kind_from_wire,
    d::TreeEntryKind,
    w::TreeEntryKind,
    [File, Directory]
);
enum_pair!(
    location_to_wire,
    location_from_wire,
    d::Location,
    w::Location,
    [Project, Quarantine]
);
enum_pair!(
    mutation_kind_to_wire,
    mutation_kind_from_wire,
    d::MutationKind,
    w::MutationKind,
    [
        CaptureBeforeImage,
        AtomicRewrite,
        CreateRelocationParent,
        Relocate,
        QuarantineFile,
        PruneEmptyDirectory,
        ContractDeleteLast,
        ContractAncestorTreePark,
        ContractExternalPreserve
    ]
);
enum_pair!(
    assurance_to_wire,
    assurance_from_wire,
    d::Assurance,
    w::Assurance,
    [Full, Reduced]
);
enum_pair!(
    cleanup_to_wire,
    cleanup_from_wire,
    d::Cleanup,
    w::Cleanup,
    [Complete, Pending]
);
enum_pair!(
    outcome_to_wire,
    outcome_from_wire,
    d::Outcome,
    w::Outcome,
    [Verified, Refused, RolledBack, RollbackFailed]
);
enum_pair!(
    direction_to_wire,
    direction_from_wire,
    d::MutationDirection,
    w::MutationDirection,
    [Apply, Rollback]
);
enum_pair!(
    origin_to_wire,
    origin_from_wire,
    d::MutationOrigin,
    w::MutationOrigin,
    [Execution, Recovery]
);
enum_pair!(
    status_to_wire,
    status_from_wire,
    d::MutationStatus,
    w::MutationStatus,
    [
        Planned,
        NoMutation,
        ApplyIntent,
        Applied,
        RollbackIntent,
        RolledBack
    ]
);
enum_pair!(
    phase_to_wire,
    phase_from_wire,
    d::VerificationPhase,
    w::VerificationPhase,
    [
        Before,
        PreContractResidual,
        FinalResidual,
        AfterHealth,
        FinalTree,
        SourceUnchanged
    ]
);

mod plan;
mod records;

use plan::*;
use records::*;

fn to_wire(value: &d::Journal) -> Result<w::TransactionJournal, String> {
    Ok(w::TransactionJournal {
        schema: value.schema,
        revision: value.revision,
        project_key: value.project_key.0.clone(),
        transaction_id: value.transaction_id.0.clone(),
        mode: mode_to_wire(value.mode),
        plan_id: value.plan_id.0.clone(),
        canonical_plan: String::from_utf8(value.canonical_plan.clone())
            .map_err(|error| format!("canonical plan is not UTF-8: {error}"))?,
        verification_workspace: value.verification_workspace.as_ref().map(workspace_to_wire),
        project_display_root: value.project_display_root.clone(),
        execution: prepared_to_wire(&value.execution),
        state: state_to_wire(&value.state),
        snapshots: value.snapshots.iter().map(snapshot_to_wire).collect(),
        snapshots_persisted: value.snapshots_persisted,
        snapshot_active: value.snapshot_active,
        candidate_name: value.candidate_name.clone(),
        quarantine_name: value.quarantine_name.clone(),
        owned_tree_token: value.owned_tree_token.clone(),
        owned_tree_seal: value.owned_tree_seal.as_ref().map(owned_tree_to_wire),
        cleanup_wal: value.cleanup_wal.as_ref().map(cleanup_wal_to_wire),
        completed_steps: value.completed_steps,
        active_step: value.active_step,
        mutation_progress: value
            .mutation_progress
            .iter()
            .map(progress_to_wire)
            .collect(),
        actual_mutations: value.actual_mutations.iter().map(actual_to_wire).collect(),
        settlement_intent: value.settlement_intent.map(outcome_to_wire),
        delivered_tree: value.delivered_tree.as_ref().map(|v| v.0.clone()),
        verification: value
            .verification
            .iter()
            .map(verification_to_wire)
            .collect(),
        events: value.events.clone(),
        report: value.report.as_ref().map(report_to_wire),
    })
}
fn from_wire(value: w::TransactionJournal) -> d::Journal {
    d::Journal {
        schema: value.schema,
        revision: value.revision,
        project_key: d::ProjectKey(value.project_key),
        transaction_id: d::TransactionId(value.transaction_id),
        mode: mode_from_wire(value.mode),
        plan_id: d::Digest(value.plan_id),
        canonical_plan: value.canonical_plan.into_bytes(),
        verification_workspace: value.verification_workspace.map(workspace_from_wire),
        project_display_root: value.project_display_root,
        execution: prepared_from_wire(value.execution),
        state: state_from_wire(value.state),
        snapshots: value
            .snapshots
            .into_iter()
            .map(snapshot_from_wire)
            .collect(),
        snapshots_persisted: value.snapshots_persisted,
        snapshot_active: value.snapshot_active,
        candidate_name: value.candidate_name,
        quarantine_name: value.quarantine_name,
        owned_tree_token: value.owned_tree_token,
        owned_tree_seal: value.owned_tree_seal.map(owned_tree_from_wire),
        cleanup_wal: value.cleanup_wal.map(cleanup_wal_from_wire),
        completed_steps: value.completed_steps,
        active_step: value.active_step,
        mutation_progress: value
            .mutation_progress
            .into_iter()
            .map(progress_from_wire)
            .collect(),
        actual_mutations: value
            .actual_mutations
            .into_iter()
            .map(actual_from_wire)
            .collect(),
        settlement_intent: value.settlement_intent.map(outcome_from_wire),
        delivered_tree: value.delivered_tree.map(d::Digest),
        verification: value
            .verification
            .into_iter()
            .map(verification_from_wire)
            .collect(),
        events: value.events,
        report: value.report.map(report_from_wire),
    }
}
