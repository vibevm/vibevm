use super::{
    boundary_from_wire, boundary_to_wire, d, location_from_wire, location_to_wire,
    mutation_kind_from_wire, mutation_kind_to_wire, snapshot_kind_from_wire, snapshot_kind_to_wire,
    tree_kind_from_wire, tree_kind_to_wire, w,
};

pub(super) fn state_to_wire(value: &d::TransactionState) -> w::TransactionState {
    match value {
        d::TransactionState::Preparing => {
            w::TransactionState::Preparing(Box::new(w::TransactionStatePreparing {}))
        }
        d::TransactionState::Prepared => {
            w::TransactionState::Prepared(Box::new(w::TransactionStatePrepared {}))
        }
        d::TransactionState::BeforePassed => {
            w::TransactionState::BeforePassed(Box::new(w::TransactionStateBeforePassed {}))
        }
        d::TransactionState::Candidate => {
            w::TransactionState::Candidate(Box::new(w::TransactionStateCandidate {}))
        }
        d::TransactionState::PublishedPendingVerify => w::TransactionState::PublishedPendingVerify(
            Box::new(w::TransactionStatePublishedPendingVerify {}),
        ),
        d::TransactionState::Mutating => {
            w::TransactionState::Mutating(Box::new(w::TransactionStateMutating {}))
        }
        d::TransactionState::ContractBoundary(action) => {
            w::TransactionState::ContractBoundary(Box::new(w::TransactionStateContractBoundary {
                action: boundary_to_wire(*action),
            }))
        }
        d::TransactionState::Verified => {
            w::TransactionState::Verified(Box::new(w::TransactionStateVerified {}))
        }
        d::TransactionState::CleanupPending => {
            w::TransactionState::CleanupPending(Box::new(w::TransactionStateCleanupPending {}))
        }
        d::TransactionState::Complete => {
            w::TransactionState::Complete(Box::new(w::TransactionStateComplete {}))
        }
        d::TransactionState::RollingBack => {
            w::TransactionState::RollingBack(Box::new(w::TransactionStateRollingBack {}))
        }
        d::TransactionState::RolledBack => {
            w::TransactionState::RolledBack(Box::new(w::TransactionStateRolledBack {}))
        }
        d::TransactionState::RollbackFailed => {
            w::TransactionState::RollbackFailed(Box::new(w::TransactionStateRollbackFailed {}))
        }
    }
}

pub(super) fn state_from_wire(value: w::TransactionState) -> d::TransactionState {
    match value {
        w::TransactionState::Preparing(_) => d::TransactionState::Preparing,
        w::TransactionState::Prepared(_) => d::TransactionState::Prepared,
        w::TransactionState::BeforePassed(_) => d::TransactionState::BeforePassed,
        w::TransactionState::Candidate(_) => d::TransactionState::Candidate,
        w::TransactionState::PublishedPendingVerify(_) => {
            d::TransactionState::PublishedPendingVerify
        }
        w::TransactionState::Mutating(_) => d::TransactionState::Mutating,
        w::TransactionState::ContractBoundary(value) => {
            d::TransactionState::ContractBoundary(boundary_from_wire(value.action))
        }
        w::TransactionState::Verified(_) => d::TransactionState::Verified,
        w::TransactionState::CleanupPending(_) => d::TransactionState::CleanupPending,
        w::TransactionState::Complete(_) => d::TransactionState::Complete,
        w::TransactionState::RollingBack(_) => d::TransactionState::RollingBack,
        w::TransactionState::RolledBack(_) => d::TransactionState::RolledBack,
        w::TransactionState::RollbackFailed(_) => d::TransactionState::RollbackFailed,
    }
}

pub(super) fn snapshot_to_wire(value: &d::SnapshotRecord) -> w::SnapshotRecord {
    w::SnapshotRecord {
        kind: snapshot_kind_to_wire(value.kind),
        name: value.name.clone(),
        sha256: value.sha256.0.clone(),
        bytes: value.bytes,
        mode: value.mode,
    }
}
pub(super) fn snapshot_from_wire(value: w::SnapshotRecord) -> d::SnapshotRecord {
    d::SnapshotRecord {
        kind: snapshot_kind_from_wire(value.kind),
        name: value.name,
        sha256: d::Digest(value.sha256),
        bytes: value.bytes,
        mode: value.mode,
    }
}
fn tree_entry_to_wire(value: &d::TreeEntry) -> w::TreeEntry {
    w::TreeEntry {
        path: value.path.clone(),
        kind: tree_kind_to_wire(value.kind),
        sha256: value.sha256.as_ref().map(|v| v.0.clone()),
        bytes: value.bytes,
        mode: value.mode,
    }
}
fn tree_entry_from_wire(value: w::TreeEntry) -> d::TreeEntry {
    d::TreeEntry {
        path: value.path,
        kind: tree_kind_from_wire(value.kind),
        sha256: value.sha256.map(d::Digest),
        bytes: value.bytes,
        mode: value.mode,
    }
}
pub(super) fn tree_to_wire(value: &d::TreeManifest) -> w::TreeManifest {
    w::TreeManifest {
        digest: value.digest.0.clone(),
        entries: value.entries.iter().map(tree_entry_to_wire).collect(),
    }
}
pub(super) fn tree_from_wire(value: w::TreeManifest) -> d::TreeManifest {
    d::TreeManifest {
        digest: d::Digest(value.digest),
        entries: value
            .entries
            .into_iter()
            .map(tree_entry_from_wire)
            .collect(),
    }
}
fn file_to_wire(value: &d::FileState) -> w::FileState {
    w::FileState {
        sha256: value.sha256.0.clone(),
        bytes: value.bytes,
        mode: value.mode,
    }
}
fn file_from_wire(value: w::FileState) -> d::FileState {
    d::FileState {
        sha256: d::Digest(value.sha256),
        bytes: value.bytes,
        mode: value.mode,
    }
}

fn export_payload_to_wire(value: &d::ExportPayload) -> w::ExportPayload {
    match value {
        d::ExportPayload::Source {
            source_path,
            before,
        } => w::ExportPayload::Source(Box::new(w::ExportPayloadSource {
            before: file_to_wire(before),
            source_path: source_path.clone(),
        })),
        d::ExportPayload::PreparedAfter { snapshot_name } => {
            w::ExportPayload::PreparedAfter(Box::new(w::ExportPayloadPreparedAfter {
                snapshot_name: snapshot_name.clone(),
            }))
        }
    }
}
fn export_payload_from_wire(value: w::ExportPayload) -> d::ExportPayload {
    match value {
        w::ExportPayload::Source(value) => d::ExportPayload::Source {
            source_path: value.source_path,
            before: file_from_wire(value.before),
        },
        w::ExportPayload::PreparedAfter(value) => d::ExportPayload::PreparedAfter {
            snapshot_name: value.snapshot_name,
        },
    }
}
fn export_entry_to_wire(value: &d::ExportEntry) -> w::ExportEntry {
    w::ExportEntry {
        target_path: value.target_path.clone(),
        kind: tree_kind_to_wire(value.kind),
        mode: value.mode,
        payload: value.payload.as_ref().map(export_payload_to_wire),
    }
}
fn export_entry_from_wire(value: w::ExportEntry) -> d::ExportEntry {
    d::ExportEntry {
        target_path: value.target_path,
        kind: tree_kind_from_wire(value.kind),
        mode: value.mode,
        payload: value.payload.map(export_payload_from_wire),
    }
}
fn export_plan_to_wire(value: &d::ExportPlan) -> w::ExportPlan {
    w::ExportPlan {
        output_identity: value.output_identity.clone(),
        output_parent_identity: value.output_parent_identity.clone(),
        output_display_path: value.output_display_path.clone(),
        output_name: value.output_name.clone(),
        before_same_display_path: value.before_same_display_path,
        after_same_display_path: value.after_same_display_path,
        entries: value.entries.iter().map(export_entry_to_wire).collect(),
        source_tree: tree_to_wire(&value.source_tree),
        final_manifest: tree_to_wire(&value.final_manifest),
    }
}
fn export_plan_from_wire(value: w::ExportPlan) -> d::ExportPlan {
    d::ExportPlan {
        output_identity: value.output_identity,
        output_parent_identity: value.output_parent_identity,
        output_display_path: value.output_display_path,
        output_name: value.output_name,
        before_same_display_path: value.before_same_display_path,
        after_same_display_path: value.after_same_display_path,
        entries: value
            .entries
            .into_iter()
            .map(export_entry_from_wire)
            .collect(),
        source_tree: tree_from_wire(value.source_tree),
        final_manifest: tree_from_wire(value.final_manifest),
    }
}

fn subtree_entry_to_wire(value: &d::SubtreeEntry) -> w::SubtreeEntry {
    w::SubtreeEntry {
        relative_path: value.relative_path.clone(),
        kind: tree_kind_to_wire(value.kind),
        sha256: value.sha256.as_ref().map(|v| v.0.clone()),
        bytes: value.bytes,
        mode: value.mode,
    }
}
fn subtree_entry_from_wire(value: w::SubtreeEntry) -> d::SubtreeEntry {
    d::SubtreeEntry {
        relative_path: value.relative_path,
        kind: tree_kind_from_wire(value.kind),
        sha256: value.sha256.map(d::Digest),
        bytes: value.bytes,
        mode: value.mode,
    }
}
fn subtree_to_wire(value: &d::SubtreeState) -> w::SubtreeState {
    w::SubtreeState {
        digest: value.digest.0.clone(),
        root_mode: value.root_mode,
        descendants: value
            .descendants
            .iter()
            .map(subtree_entry_to_wire)
            .collect(),
    }
}
fn subtree_from_wire(value: w::SubtreeState) -> d::SubtreeState {
    d::SubtreeState {
        digest: d::Digest(value.digest),
        root_mode: value.root_mode,
        descendants: value
            .descendants
            .into_iter()
            .map(subtree_entry_from_wire)
            .collect(),
    }
}
fn path_state_to_wire(value: &d::PathState) -> w::PathState {
    match value {
        d::PathState::Absent => w::PathState::Absent(Box::new(w::PathStateAbsent {})),
        d::PathState::File(value) => w::PathState::File(Box::new(w::PathStateFile {
            state: file_to_wire(value),
        })),
        d::PathState::EmptyDirectory { mode } => {
            w::PathState::EmptyDirectory(Box::new(w::PathStateEmptyDirectory { mode: *mode }))
        }
        d::PathState::Tree(value) => w::PathState::Tree(Box::new(w::PathStateTree {
            state: subtree_to_wire(value),
        })),
    }
}
fn path_state_from_wire(value: w::PathState) -> d::PathState {
    match value {
        w::PathState::Absent(_) => d::PathState::Absent,
        w::PathState::File(value) => d::PathState::File(file_from_wire(value.state)),
        w::PathState::EmptyDirectory(value) => d::PathState::EmptyDirectory { mode: value.mode },
        w::PathState::Tree(value) => d::PathState::Tree(subtree_from_wire(value.state)),
    }
}
fn transition_to_wire(value: &d::PathTransition) -> w::PathTransition {
    w::PathTransition {
        location: location_to_wire(value.location),
        path: value.path.clone(),
        before: path_state_to_wire(&value.before),
        after: path_state_to_wire(&value.after),
    }
}
fn transition_from_wire(value: w::PathTransition) -> d::PathTransition {
    d::PathTransition {
        location: location_from_wire(value.location),
        path: value.path,
        before: path_state_from_wire(value.before),
        after: path_state_from_wire(value.after),
    }
}
fn step_to_wire(value: &d::MutationStep) -> w::MutationStep {
    w::MutationStep {
        id: value.id.clone(),
        pair_id: value.pair_id.clone(),
        kind: mutation_kind_to_wire(value.kind),
        transitions: value.transitions.iter().map(transition_to_wire).collect(),
    }
}
fn step_from_wire(value: w::MutationStep) -> d::MutationStep {
    d::MutationStep {
        id: value.id,
        pair_id: value.pair_id,
        kind: mutation_kind_from_wire(value.kind),
        transitions: value
            .transitions
            .into_iter()
            .map(transition_from_wire)
            .collect(),
    }
}

fn commit_to_wire(value: &d::ContractCommit) -> w::ContractCommit {
    match value {
        d::ContractCommit::DeleteLast {
            path,
            empty_ancestors,
        } => w::ContractCommit::DeleteLast(Box::new(w::ContractCommitDeleteLast {
            empty_ancestors: empty_ancestors.clone(),
            path: path.clone(),
        })),
        d::ContractCommit::ExternalPreserve => {
            w::ContractCommit::ExternalPreserve(Box::new(w::ContractCommitExternalPreserve {}))
        }
    }
}
fn commit_from_wire(value: w::ContractCommit) -> d::ContractCommit {
    match value {
        w::ContractCommit::DeleteLast(value) => d::ContractCommit::DeleteLast {
            path: value.path,
            empty_ancestors: value.empty_ancestors,
        },
        w::ContractCommit::ExternalPreserve(_) => d::ContractCommit::ExternalPreserve,
    }
}
fn inplace_to_wire(value: &d::InPlacePlan) -> w::InPlacePlan {
    w::InPlacePlan {
        quarantine_parent_identity: value.quarantine_parent_identity.clone(),
        before_same_display_path: value.before_same_display_path,
        after_same_display_path: value.after_same_display_path,
        steps: value.steps.iter().map(step_to_wire).collect(),
        contract: commit_to_wire(&value.contract),
        contract_step: step_to_wire(&value.contract_step),
        contract_cleanup_step: value.contract_cleanup_step.as_ref().map(step_to_wire),
        before_tree: tree_to_wire(&value.before_tree),
        pre_contract_tree: tree_to_wire(&value.pre_contract_tree),
        post_contract_tree: tree_to_wire(&value.post_contract_tree),
        after_tree: tree_to_wire(&value.after_tree),
    }
}
fn inplace_from_wire(value: w::InPlacePlan) -> d::InPlacePlan {
    d::InPlacePlan {
        quarantine_parent_identity: value.quarantine_parent_identity,
        before_same_display_path: value.before_same_display_path,
        after_same_display_path: value.after_same_display_path,
        steps: value.steps.into_iter().map(step_from_wire).collect(),
        contract: commit_from_wire(value.contract),
        contract_step: step_from_wire(value.contract_step),
        contract_cleanup_step: value.contract_cleanup_step.map(step_from_wire),
        before_tree: tree_from_wire(value.before_tree),
        pre_contract_tree: tree_from_wire(value.pre_contract_tree),
        post_contract_tree: tree_from_wire(value.post_contract_tree),
        after_tree: tree_from_wire(value.after_tree),
    }
}
pub(super) fn prepared_to_wire(value: &d::PreparedMode) -> w::PreparedMode {
    match value {
        d::PreparedMode::Export(plan) => w::PreparedMode::Export(Box::new(w::PreparedModeExport {
            plan: export_plan_to_wire(plan),
        })),
        d::PreparedMode::InPlace(plan) => {
            w::PreparedMode::InPlace(Box::new(w::PreparedModeInPlace {
                plan: inplace_to_wire(plan),
            }))
        }
    }
}
pub(super) fn prepared_from_wire(value: w::PreparedMode) -> d::PreparedMode {
    match value {
        w::PreparedMode::Export(value) => {
            d::PreparedMode::Export(Box::new(export_plan_from_wire(value.plan)))
        }
        w::PreparedMode::InPlace(value) => {
            d::PreparedMode::InPlace(Box::new(inplace_from_wire(value.plan)))
        }
    }
}
