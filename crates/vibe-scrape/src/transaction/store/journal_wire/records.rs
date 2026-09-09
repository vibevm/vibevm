use super::plan::{snapshot_from_wire, snapshot_to_wire};
use super::{
    assurance_from_wire, assurance_to_wire, cleanup_from_wire, cleanup_to_wire, d,
    direction_from_wire, direction_to_wire, mode_from_wire, mode_to_wire, mutation_kind_from_wire,
    mutation_kind_to_wire, origin_from_wire, origin_to_wire, outcome_from_wire, outcome_to_wire,
    phase_from_wire, phase_to_wire, status_from_wire, status_to_wire, tree_kind_from_wire,
    tree_kind_to_wire, w,
};

fn owned_entry_to_wire(value: &d::OwnedEntrySeal) -> w::OwnedEntrySeal {
    w::OwnedEntrySeal {
        path: value.path.clone(),
        kind: tree_kind_to_wire(value.kind),
        sha256: value.sha256.as_ref().map(|v| v.0.clone()),
        bytes: value.bytes,
        mode: value.mode,
        identity: value.identity.clone(),
    }
}
fn owned_entry_from_wire(value: w::OwnedEntrySeal) -> d::OwnedEntrySeal {
    d::OwnedEntrySeal {
        path: value.path,
        kind: tree_kind_from_wire(value.kind),
        sha256: value.sha256.map(d::Digest),
        bytes: value.bytes,
        mode: value.mode,
        identity: value.identity,
    }
}
pub(super) fn owned_tree_to_wire(value: &d::OwnedTreeSeal) -> w::OwnedTreeSeal {
    w::OwnedTreeSeal {
        directory_identity: value.directory_identity.clone(),
        manifest_digest: value.manifest_digest.clone(),
        entries: value.entries.iter().map(owned_entry_to_wire).collect(),
    }
}
pub(super) fn owned_tree_from_wire(value: w::OwnedTreeSeal) -> d::OwnedTreeSeal {
    d::OwnedTreeSeal {
        directory_identity: value.directory_identity,
        manifest_digest: value.manifest_digest,
        entries: value
            .entries
            .into_iter()
            .map(owned_entry_from_wire)
            .collect(),
    }
}
fn cleanup_intent_to_wire(value: &d::OwnedTreeCleanupIntent) -> w::OwnedTreeCleanupIntent {
    w::OwnedTreeCleanupIntent {
        intent_token: value.intent_token.clone(),
        progress_key: value.progress_key.clone(),
        path: value.path.clone(),
        expected: owned_entry_to_wire(&value.expected),
        root: value.root,
    }
}
fn cleanup_intent_from_wire(value: w::OwnedTreeCleanupIntent) -> d::OwnedTreeCleanupIntent {
    d::OwnedTreeCleanupIntent {
        intent_token: value.intent_token,
        progress_key: value.progress_key,
        path: value.path,
        expected: owned_entry_from_wire(value.expected),
        root: value.root,
    }
}
pub(super) fn cleanup_wal_to_wire(value: &d::OwnedTreeCleanupWal) -> w::OwnedTreeCleanupWal {
    w::OwnedTreeCleanupWal {
        name: value.name.clone(),
        directory_identity: value.directory_identity.clone(),
        manifest_digest: value.manifest_digest.clone(),
        completed: value.completed.clone(),
        active: value.active.as_ref().map(cleanup_intent_to_wire),
    }
}
pub(super) fn cleanup_wal_from_wire(value: w::OwnedTreeCleanupWal) -> d::OwnedTreeCleanupWal {
    d::OwnedTreeCleanupWal {
        name: value.name,
        directory_identity: value.directory_identity,
        manifest_digest: value.manifest_digest,
        completed: value.completed,
        active: value.active.map(cleanup_intent_from_wire),
    }
}
pub(super) fn workspace_to_wire(
    value: &d::VerificationWorkspaceIntent,
) -> w::VerificationWorkspaceIntent {
    w::VerificationWorkspaceIntent {
        name: value.name.clone(),
        display_root: value.display_root.clone(),
        ownership_token: value.ownership_token.clone(),
    }
}
pub(super) fn workspace_from_wire(
    value: w::VerificationWorkspaceIntent,
) -> d::VerificationWorkspaceIntent {
    d::VerificationWorkspaceIntent {
        name: value.name,
        display_root: value.display_root,
        ownership_token: value.ownership_token,
    }
}

fn planned_kind_to_wire(value: d::PlannedMutationKind) -> w::PlannedMutationKind {
    match value {
        d::PlannedMutationKind::ExportCandidateCreate => {
            w::PlannedMutationKind::ExportCandidateCreate(Box::new(
                w::PlannedMutationKindExportCandidateCreate {},
            ))
        }
        d::PlannedMutationKind::ExportEntry => {
            w::PlannedMutationKind::ExportEntry(Box::new(w::PlannedMutationKindExportEntry {}))
        }
        d::PlannedMutationKind::ExportPublish => {
            w::PlannedMutationKind::ExportPublish(Box::new(w::PlannedMutationKindExportPublish {}))
        }
        d::PlannedMutationKind::InPlaceQuarantineCreate => {
            w::PlannedMutationKind::InPlaceQuarantineCreate(Box::new(
                w::PlannedMutationKindInPlaceQuarantineCreate {},
            ))
        }
        d::PlannedMutationKind::InPlace(kind) => {
            w::PlannedMutationKind::InPlace(Box::new(w::PlannedMutationKindInPlace {
                mutation: mutation_kind_to_wire(kind),
            }))
        }
    }
}
fn planned_kind_from_wire(value: w::PlannedMutationKind) -> d::PlannedMutationKind {
    match value {
        w::PlannedMutationKind::ExportCandidateCreate(_) => {
            d::PlannedMutationKind::ExportCandidateCreate
        }
        w::PlannedMutationKind::ExportEntry(_) => d::PlannedMutationKind::ExportEntry,
        w::PlannedMutationKind::ExportPublish(_) => d::PlannedMutationKind::ExportPublish,
        w::PlannedMutationKind::InPlaceQuarantineCreate(_) => {
            d::PlannedMutationKind::InPlaceQuarantineCreate
        }
        w::PlannedMutationKind::InPlace(value) => {
            d::PlannedMutationKind::InPlace(mutation_kind_from_wire(value.mutation))
        }
    }
}
pub(super) fn progress_to_wire(value: &d::MutationProgress) -> w::MutationProgress {
    w::MutationProgress {
        id: value.id.clone(),
        kind: planned_kind_to_wire(value.kind),
        status: status_to_wire(value.status),
    }
}
pub(super) fn progress_from_wire(value: w::MutationProgress) -> d::MutationProgress {
    d::MutationProgress {
        id: value.id,
        kind: planned_kind_from_wire(value.kind),
        status: status_from_wire(value.status),
    }
}
pub(super) fn actual_to_wire(value: &d::ActualMutationEvidence) -> w::ActualMutationEvidence {
    w::ActualMutationEvidence {
        id: value.id.clone(),
        kind: planned_kind_to_wire(value.kind),
        direction: direction_to_wire(value.direction),
        origin: origin_to_wire(value.origin),
        status: status_to_wire(value.status),
    }
}
pub(super) fn actual_from_wire(value: w::ActualMutationEvidence) -> d::ActualMutationEvidence {
    d::ActualMutationEvidence {
        id: value.id,
        kind: planned_kind_from_wire(value.kind),
        direction: direction_from_wire(value.direction),
        origin: origin_from_wire(value.origin),
        status: status_from_wire(value.status),
    }
}
fn planned_to_wire(value: &d::PlannedMutationEvidence) -> w::PlannedMutationEvidence {
    w::PlannedMutationEvidence {
        id: value.id.clone(),
        kind: planned_kind_to_wire(value.kind),
    }
}
fn planned_from_wire(value: w::PlannedMutationEvidence) -> d::PlannedMutationEvidence {
    d::PlannedMutationEvidence {
        id: value.id,
        kind: planned_kind_from_wire(value.kind),
    }
}
fn evidence_to_wire(value: &d::VerificationEvidence) -> w::VerificationEvidence {
    w::VerificationEvidence {
        accepted: value.accepted,
        assurance: assurance_to_wire(value.assurance),
        summary: value.summary.clone(),
        canonical_evidence: value.canonical_evidence.clone(),
    }
}
fn evidence_from_wire(value: w::VerificationEvidence) -> d::VerificationEvidence {
    d::VerificationEvidence {
        accepted: value.accepted,
        assurance: assurance_from_wire(value.assurance),
        summary: value.summary,
        canonical_evidence: value.canonical_evidence,
    }
}
pub(super) fn verification_to_wire(value: &d::VerificationRecord) -> w::VerificationRecord {
    w::VerificationRecord {
        phase: phase_to_wire(value.phase),
        evidence_sha256: value.evidence_sha256.0.clone(),
        evidence: evidence_to_wire(&value.evidence),
    }
}
pub(super) fn verification_from_wire(value: w::VerificationRecord) -> d::VerificationRecord {
    d::VerificationRecord {
        phase: phase_from_wire(value.phase),
        evidence_sha256: d::Digest(value.evidence_sha256),
        evidence: evidence_from_wire(value.evidence),
    }
}

pub(super) fn report_to_wire(value: &d::TransactionReport) -> w::InternalReport {
    w::InternalReport {
        project_key: value.project_key.0.clone(),
        transaction_id: value.transaction_id.0.clone(),
        plan_id: value.plan_id.0.clone(),
        mode: mode_to_wire(value.mode),
        outcome: outcome_to_wire(value.outcome),
        assurance: assurance_to_wire(value.assurance),
        cleanup: cleanup_to_wire(value.cleanup),
        before_tree: value.before_tree.as_ref().map(|v| v.0.clone()),
        after_tree: value.after_tree.as_ref().map(|v| v.0.clone()),
        snapshots: value.snapshots.iter().map(snapshot_to_wire).collect(),
        verification: value
            .verification
            .iter()
            .map(verification_to_wire)
            .collect(),
        planned_mutations: value
            .planned_mutations
            .iter()
            .map(planned_to_wire)
            .collect(),
        actual_mutations: value.actual_mutations.iter().map(actual_to_wire).collect(),
        events: value.events.clone(),
    }
}
pub(super) fn report_from_wire(value: w::InternalReport) -> d::TransactionReport {
    d::TransactionReport {
        project_key: d::ProjectKey(value.project_key),
        transaction_id: d::TransactionId(value.transaction_id),
        plan_id: d::Digest(value.plan_id),
        mode: mode_from_wire(value.mode),
        outcome: outcome_from_wire(value.outcome),
        assurance: assurance_from_wire(value.assurance),
        cleanup: cleanup_from_wire(value.cleanup),
        before_tree: value.before_tree.map(d::Digest),
        after_tree: value.after_tree.map(d::Digest),
        snapshots: value
            .snapshots
            .into_iter()
            .map(snapshot_from_wire)
            .collect(),
        verification: value
            .verification
            .into_iter()
            .map(verification_from_wire)
            .collect(),
        planned_mutations: value
            .planned_mutations
            .into_iter()
            .map(planned_from_wire)
            .collect(),
        actual_mutations: value
            .actual_mutations
            .into_iter()
            .map(actual_from_wire)
            .collect(),
        events: value.events,
    }
}
