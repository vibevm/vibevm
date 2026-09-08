macro_rules! impl_remote_serde {
    ($actual:ty, $remote:ident) => {
        impl Serialize for $actual {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                $remote::serialize(self, serializer)
            }
        }

        impl<'de> Deserialize<'de> for $actual {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                $remote::deserialize(deserializer)
            }
        }
    };
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "Digest")]
struct DigestDef(String);
impl_remote_serde!(Digest, DigestDef);

#[derive(Serialize, Deserialize)]
#[serde(remote = "ProjectKey")]
struct ProjectKeyDef(String);
impl_remote_serde!(ProjectKey, ProjectKeyDef);

#[derive(Serialize, Deserialize)]
#[serde(remote = "TransactionId")]
struct TransactionIdDef(String);
impl_remote_serde!(TransactionId, TransactionIdDef);

macro_rules! scalar_enum_wire {
    ($actual:ty, $remote:ident, $remote_path:literal, { $($variant:ident),+ $(,)? }) => {
        #[derive(Serialize, Deserialize)]
        #[serde(remote = $remote_path, rename_all = "kebab-case")]
        enum $remote { $($variant),+ }
        impl_remote_serde!($actual, $remote);
    };
}

scalar_enum_wire!(TransactionMode, TransactionModeDef, "TransactionMode", { Export, InPlace });
scalar_enum_wire!(ContractBoundaryAction, ContractBoundaryActionDef, "ContractBoundaryAction", {
    DeleteLastMoved,
    ExternalPreserved,
});

#[derive(Serialize, Deserialize)]
#[serde(remote = "TransactionState", rename_all = "kebab-case")]
enum TransactionStateDef {
    Preparing,
    Prepared,
    BeforePassed,
    Candidate,
    PublishedPendingVerify,
    Mutating,
    ContractBoundary(ContractBoundaryAction),
    Verified,
    CleanupPending,
    Complete,
    RollingBack,
    RolledBack,
    RollbackFailed,
}
impl_remote_serde!(TransactionState, TransactionStateDef);

scalar_enum_wire!(SnapshotKind, SnapshotKindDef, "SnapshotKind", {
    Contract,
    CanonicalContract,
    CanonicalPlan,
    Verifier,
    PreparedAfter,
});

#[derive(Serialize, Deserialize)]
#[serde(remote = "SnapshotRecord", deny_unknown_fields)]
struct SnapshotRecordDef {
    kind: SnapshotKind,
    name: String,
    sha256: Digest,
    bytes: u64,
    mode: Option<u32>,
}
impl_remote_serde!(SnapshotRecord, SnapshotRecordDef);

scalar_enum_wire!(TreeEntryKind, TreeEntryKindDef, "TreeEntryKind", { File, Directory });

#[derive(Serialize, Deserialize)]
#[serde(remote = "TreeEntry", deny_unknown_fields)]
struct TreeEntryDef {
    path: String,
    kind: TreeEntryKind,
    sha256: Option<Digest>,
    bytes: Option<u64>,
    mode: Option<u32>,
}
impl_remote_serde!(TreeEntry, TreeEntryDef);

#[derive(Serialize, Deserialize)]
#[serde(remote = "TreeManifest", deny_unknown_fields)]
struct TreeManifestDef {
    digest: Digest,
    entries: Vec<TreeEntry>,
}
impl_remote_serde!(TreeManifest, TreeManifestDef);

#[derive(Serialize, Deserialize)]
#[serde(remote = "FileState", deny_unknown_fields)]
struct FileStateDef {
    sha256: Digest,
    bytes: u64,
    mode: Option<u32>,
}
impl_remote_serde!(FileState, FileStateDef);

#[derive(Serialize, Deserialize)]
#[serde(
    remote = "ExportPayload",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
enum ExportPayloadDef {
    Source {
        source_path: String,
        before: FileState,
    },
    PreparedAfter {
        snapshot_name: String,
    },
}
impl_remote_serde!(ExportPayload, ExportPayloadDef);

#[derive(Serialize, Deserialize)]
#[serde(remote = "ExportEntry", deny_unknown_fields)]
struct ExportEntryDef {
    target_path: String,
    kind: TreeEntryKind,
    mode: Option<u32>,
    payload: Option<ExportPayload>,
}
impl_remote_serde!(ExportEntry, ExportEntryDef);

#[derive(Serialize, Deserialize)]
#[serde(remote = "ExportPlan", deny_unknown_fields)]
struct ExportPlanDef {
    output_identity: String,
    output_parent_identity: String,
    output_display_path: String,
    output_name: String,
    before_same_display_path: bool,
    after_same_display_path: bool,
    entries: Vec<ExportEntry>,
    source_tree: TreeManifest,
    final_manifest: TreeManifest,
}
impl_remote_serde!(ExportPlan, ExportPlanDef);

#[derive(Serialize, Deserialize)]
#[serde(remote = "SubtreeEntry", deny_unknown_fields)]
struct SubtreeEntryDef {
    relative_path: String,
    kind: TreeEntryKind,
    sha256: Option<Digest>,
    bytes: Option<u64>,
    mode: Option<u32>,
}
impl_remote_serde!(SubtreeEntry, SubtreeEntryDef);

#[derive(Serialize, Deserialize)]
#[serde(remote = "SubtreeState", deny_unknown_fields)]
struct SubtreeStateDef {
    digest: Digest,
    root_mode: Option<u32>,
    descendants: Vec<SubtreeEntry>,
}
impl_remote_serde!(SubtreeState, SubtreeStateDef);

#[derive(Serialize, Deserialize)]
#[serde(remote = "PathState", rename_all = "kebab-case", deny_unknown_fields)]
enum PathStateDef {
    Absent,
    File(FileState),
    EmptyDirectory { mode: Option<u32> },
    Tree(SubtreeState),
}
impl_remote_serde!(PathState, PathStateDef);

scalar_enum_wire!(Location, LocationDef, "Location", { Project, Quarantine });

#[derive(Serialize, Deserialize)]
#[serde(remote = "PathTransition", deny_unknown_fields)]
struct PathTransitionDef {
    location: Location,
    path: String,
    before: PathState,
    after: PathState,
}
impl_remote_serde!(PathTransition, PathTransitionDef);

scalar_enum_wire!(MutationKind, MutationKindDef, "MutationKind", {
    CaptureBeforeImage,
    AtomicRewrite,
    CreateRelocationParent,
    Relocate,
    QuarantineFile,
    PruneEmptyDirectory,
    ContractDeleteLast,
    ContractAncestorTreePark,
    ContractExternalPreserve,
});

#[derive(Serialize, Deserialize)]
#[serde(remote = "MutationStep", deny_unknown_fields)]
struct MutationStepDef {
    id: String,
    pair_id: Option<String>,
    kind: MutationKind,
    transitions: Vec<PathTransition>,
}
impl_remote_serde!(MutationStep, MutationStepDef);

#[derive(Serialize, Deserialize)]
#[serde(
    remote = "ContractCommit",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
enum ContractCommitDef {
    DeleteLast {
        path: String,
        empty_ancestors: Vec<String>,
    },
    ExternalPreserve,
}
impl_remote_serde!(ContractCommit, ContractCommitDef);

#[derive(Serialize, Deserialize)]
#[serde(remote = "InPlacePlan", deny_unknown_fields)]
struct InPlacePlanDef {
    quarantine_parent_identity: String,
    before_same_display_path: bool,
    after_same_display_path: bool,
    steps: Vec<MutationStep>,
    contract: ContractCommit,
    contract_step: MutationStep,
    contract_cleanup_step: Option<MutationStep>,
    before_tree: TreeManifest,
    pre_contract_tree: TreeManifest,
    post_contract_tree: TreeManifest,
    after_tree: TreeManifest,
}
impl_remote_serde!(InPlacePlan, InPlacePlanDef);

#[derive(Serialize, Deserialize)]
#[serde(remote = "PreparedMode", rename_all = "kebab-case")]
enum PreparedModeDef {
    Export(Box<ExportPlan>),
    InPlace(Box<InPlacePlan>),
}
impl_remote_serde!(PreparedMode, PreparedModeDef);

#[derive(Serialize, Deserialize)]
#[serde(remote = "OwnedEntrySeal", deny_unknown_fields)]
struct OwnedEntrySealDef {
    path: String,
    kind: TreeEntryKind,
    sha256: Option<Digest>,
    bytes: Option<u64>,
    mode: Option<u32>,
    identity: String,
}
impl_remote_serde!(OwnedEntrySeal, OwnedEntrySealDef);

#[derive(Serialize, Deserialize)]
#[serde(remote = "OwnedTreeSeal", deny_unknown_fields)]
struct OwnedTreeSealDef {
    directory_identity: String,
    manifest_digest: String,
    entries: Vec<OwnedEntrySeal>,
}
impl_remote_serde!(OwnedTreeSeal, OwnedTreeSealDef);

#[derive(Serialize, Deserialize)]
#[serde(remote = "OwnedTreeCleanupIntent", deny_unknown_fields)]
struct OwnedTreeCleanupIntentDef {
    intent_token: String,
    progress_key: String,
    path: String,
    expected: OwnedEntrySeal,
    root: bool,
}
impl_remote_serde!(OwnedTreeCleanupIntent, OwnedTreeCleanupIntentDef);

#[derive(Serialize, Deserialize)]
#[serde(remote = "OwnedTreeCleanupWal", deny_unknown_fields)]
struct OwnedTreeCleanupWalDef {
    name: String,
    directory_identity: String,
    manifest_digest: String,
    completed: Vec<String>,
    active: Option<OwnedTreeCleanupIntent>,
}
impl_remote_serde!(OwnedTreeCleanupWal, OwnedTreeCleanupWalDef);

#[derive(Serialize, Deserialize)]
#[serde(remote = "VerificationWorkspaceIntent", deny_unknown_fields)]
struct VerificationWorkspaceIntentDef {
    name: String,
    display_root: String,
    ownership_token: String,
}
impl_remote_serde!(VerificationWorkspaceIntent, VerificationWorkspaceIntentDef);

scalar_enum_wire!(Assurance, AssuranceDef, "Assurance", { Full, Reduced });
scalar_enum_wire!(Cleanup, CleanupDef, "Cleanup", { Complete, Pending });
scalar_enum_wire!(Outcome, OutcomeDef, "Outcome", {
    Verified,
    Refused,
    RolledBack,
    RollbackFailed,
});
#[derive(Serialize, Deserialize)]
#[serde(remote = "PlannedMutationKind", rename_all = "kebab-case")]
enum PlannedMutationKindDef {
    ExportCandidateCreate,
    ExportEntry,
    ExportPublish,
    InPlaceQuarantineCreate,
    InPlace(MutationKind),
}
impl_remote_serde!(PlannedMutationKind, PlannedMutationKindDef);

scalar_enum_wire!(MutationDirection, MutationDirectionDef, "MutationDirection", { Apply, Rollback });
scalar_enum_wire!(MutationOrigin, MutationOriginDef, "MutationOrigin", {
    Execution,
    Recovery,
});
scalar_enum_wire!(MutationStatus, MutationStatusDef, "MutationStatus", {
    Planned,
    NoMutation,
    ApplyIntent,
    Applied,
    RollbackIntent,
    RolledBack,
});

#[derive(Serialize, Deserialize)]
#[serde(remote = "MutationProgress", deny_unknown_fields)]
struct MutationProgressDef {
    id: String,
    kind: PlannedMutationKind,
    status: MutationStatus,
}
impl_remote_serde!(MutationProgress, MutationProgressDef);

#[derive(Serialize, Deserialize)]
#[serde(remote = "ActualMutationEvidence", deny_unknown_fields)]
struct ActualMutationEvidenceDef {
    id: String,
    kind: PlannedMutationKind,
    direction: MutationDirection,
    origin: MutationOrigin,
    status: MutationStatus,
}
impl_remote_serde!(ActualMutationEvidence, ActualMutationEvidenceDef);

#[derive(Serialize, Deserialize)]
#[serde(remote = "PlannedMutationEvidence", deny_unknown_fields)]
struct PlannedMutationEvidenceDef {
    id: String,
    kind: PlannedMutationKind,
}
impl_remote_serde!(PlannedMutationEvidence, PlannedMutationEvidenceDef);

scalar_enum_wire!(VerificationPhase, VerificationPhaseDef, "VerificationPhase", {
    Before,
    PreContractResidual,
    FinalResidual,
    AfterHealth,
    FinalTree,
    SourceUnchanged,
});

#[derive(Serialize, Deserialize)]
#[serde(remote = "VerificationEvidence", deny_unknown_fields)]
struct VerificationEvidenceDef {
    accepted: bool,
    assurance: Assurance,
    summary: String,
    canonical_evidence: Vec<u8>,
}
impl_remote_serde!(VerificationEvidence, VerificationEvidenceDef);

#[derive(Serialize, Deserialize)]
#[serde(remote = "VerificationRecord", deny_unknown_fields)]
struct VerificationRecordDef {
    phase: VerificationPhase,
    evidence_sha256: Digest,
    evidence: VerificationEvidence,
}
impl_remote_serde!(VerificationRecord, VerificationRecordDef);

#[derive(Serialize, Deserialize)]
#[serde(remote = "TransactionReport", deny_unknown_fields)]
struct TransactionReportDef {
    project_key: ProjectKey,
    transaction_id: TransactionId,
    plan_id: Digest,
    mode: TransactionMode,
    outcome: Outcome,
    assurance: Assurance,
    cleanup: Cleanup,
    before_tree: Option<Digest>,
    after_tree: Option<Digest>,
    snapshots: Vec<SnapshotRecord>,
    verification: Vec<VerificationRecord>,
    planned_mutations: Vec<PlannedMutationEvidence>,
    actual_mutations: Vec<ActualMutationEvidence>,
    events: Vec<String>,
}
impl_remote_serde!(TransactionReport, TransactionReportDef);

#[derive(Serialize, Deserialize)]
#[serde(remote = "Journal", deny_unknown_fields)]
struct JournalDef {
    schema: u32,
    revision: u64,
    project_key: ProjectKey,
    transaction_id: TransactionId,
    mode: TransactionMode,
    plan_id: Digest,
    project_display_root: String,
    #[serde(
        serialize_with = "serialize_canonical_plan",
        deserialize_with = "deserialize_canonical_plan"
    )]
    canonical_plan: Vec<u8>,
    verification_workspace: Option<VerificationWorkspaceIntent>,
    execution: PreparedMode,
    state: TransactionState,
    snapshots: Vec<SnapshotRecord>,
    snapshots_persisted: usize,
    snapshot_active: Option<usize>,
    candidate_name: Option<String>,
    quarantine_name: Option<String>,
    owned_tree_token: Option<String>,
    owned_tree_seal: Option<OwnedTreeSeal>,
    cleanup_wal: Option<OwnedTreeCleanupWal>,
    completed_steps: usize,
    active_step: Option<usize>,
    mutation_progress: Vec<MutationProgress>,
    actual_mutations: Vec<ActualMutationEvidence>,
    settlement_intent: Option<Outcome>,
    delivered_tree: Option<Digest>,
    verification: Vec<VerificationRecord>,
    events: Vec<String>,
    report: Option<TransactionReport>,
}
impl_remote_serde!(Journal, JournalDef);
