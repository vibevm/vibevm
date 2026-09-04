//! Capability-rooted durable storage for scrape transactions.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-E");

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest as _, Sha256};
use vibe_safefs::{
    CleanupIntent, CleanupPreparation, DirectoryDurability, EntryIdentity, EntryState,
    EntryStateKind, ExternalDirectory, ExternalProjectLock, ExternalStore, OwnedDirectory,
    OwnedDirectoryCreateError, OwnedDirectoryIdentity, OwnedTreeCleanupError,
    OwnedTreeCleanupProgress, Project, TreeEntry as SafefsTreeEntry,
    TreeManifest as SafefsTreeManifest,
};
use vibe_wire::generated::scrape::e1::{plan::Plan as ScrapePlanWire, report as report_wire};

use super::model::*;
use super::report::report_to_wire_plan;
use super::sha256::project_key as derive_project_key;
use super::traits::{ProjectLock, TransactionStore};
use super::validate;

const JOURNAL_FILE: &str = "journal.json";
const OWNER_FILE: &str = "owner.json";
const TRANSACTIONS_DIRECTORY: &str = "t";
const REPORTS_DIRECTORY: &str = "reports";
const SNAPSHOTS_DIRECTORY: &str = "snapshots";
const VERIFICATION_DIRECTORY: &str = "v";
// The embedded canonical plan is itself bounded to 16 MiB. JSON string
// escaping plus the executable recovery projection require a separately
// bounded envelope rather than silently making the plan's legal maximum
// unpersistable.
const MAX_OWNER_BYTES: usize = 4096;
const MAX_RETIREMENT_BYTES: usize = 16 * 1024 * 1024;
const MAX_SNAPSHOT_BYTES: usize = 64 * 1024 * 1024;
const MAX_SNAPSHOT_TOTAL_BYTES: u64 = 256 * 1024 * 1024;
const MAX_DIRECTORY_CHILDREN: usize = 16_384;
const MAX_TREE_ENTRIES: usize = 65_536;

static TRANSACTION_NONCE: AtomicU64 = AtomicU64::new(0);

/// A transaction store rooted at an explicit absolute, caller-injected state
/// directory. The directory is not opened until a project has been pinned, so
/// the first creation can be ordered after the disjointness proof.
#[derive(Debug)]
pub struct SystemTransactionStore {
    state_root: PathBuf,
    external: Option<ExternalStore>,
    proven_project: Option<ProjectKey>,
    proven_display_root: Option<String>,
    external_lock: Option<ExternalProjectLock>,
    locked_project: Option<ProjectKey>,
    live_verification_workspace: Option<OwnedDirectory>,
}

impl SystemTransactionStore {
    pub fn new(state_root: impl Into<PathBuf>) -> Result<Self, TransactionError> {
        let state_root = state_root.into();
        if !state_root.is_absolute() {
            return Err(TransactionError::Store(
                "scrape transaction state root must be absolute".to_owned(),
            ));
        }
        Ok(Self {
            state_root,
            external: None,
            proven_project: None,
            proven_display_root: None,
            external_lock: None,
            locked_project: None,
            live_verification_workspace: None,
        })
    }

    #[must_use]
    pub fn state_root(&self) -> &Path {
        &self.state_root
    }

    /// Read one exact journaled snapshot for a recovery adapter. The name must
    /// occur in the journal's durable prefix; source contract paths are never
    /// consulted.
    pub fn read_snapshot(
        &mut self,
        journal: &Journal,
        name: &str,
    ) -> Result<Vec<u8>, TransactionError> {
        self.require_locked(&journal.project_key)?;
        let index = journal
            .snapshots
            .iter()
            .position(|record| record.name == name)
            .ok_or_else(|| {
                TransactionError::Store(format!("snapshot `{name}` is not journaled"))
            })?;
        if index >= journal.snapshots_persisted {
            return Err(TransactionError::Store(format!(
                "snapshot `{name}` is outside the durable prefix"
            )));
        }
        self.read_snapshot_record(journal, &journal.snapshots[index])?
            .ok_or_else(|| TransactionError::Store(format!("snapshot `{name}` is absent")))
    }

    fn canonical_report_bytes(
        &mut self,
        journal: &Journal,
        report: &TransactionReport,
    ) -> Result<Vec<u8>, TransactionError> {
        let plan: ScrapePlanWire =
            strict_json_parse(&journal.canonical_plan, "embedded canonical scrape plan")?;
        let wire = report_to_wire_plan(report, &plan)?;
        validate_canonical_report_identity(journal, report, &wire)?;
        strict_json_bytes(
            &wire,
            MAX_CANONICAL_REPORT_BYTES,
            "canonical transaction report",
        )
    }

    fn require_stable_complete_report(
        &mut self,
        journal: &Journal,
    ) -> Result<Vec<u8>, TransactionError> {
        let report = journal
            .report
            .as_ref()
            .ok_or_else(|| store_error("complete journal has no embedded report"))?;
        if report.cleanup != Cleanup::Complete {
            return Err(store_error(
                "transaction retirement requires cleanup-complete report evidence",
            ));
        }
        let expected = self.canonical_report_bytes(journal, report)?;
        let observed = self
            .external()?
            .read_stable_bounded(
                &report_relative(&journal.transaction_id)?,
                MAX_CANONICAL_REPORT_BYTES,
            )
            .map_err(|error| store_error(format!("reading stable report: {error:#}")))?
            .ok_or_else(|| store_error("stable complete report is absent"))?;
        if observed.bytes != expected {
            return Err(store_error(
                "stable report differs from the canonical complete journal report",
            ));
        }
        Ok(expected)
    }
}

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

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnerWire {
    schema: u32,
    project_key: String,
    transaction_id: String,
    journal_intent_sha256: String,
    ownership_token: String,
    directory_identity: String,
    entry_identity: String,
    workspace_directory_identity: String,
    workspace_entry_identity: String,
    workspace_project_identity: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RetirementWire {
    schema: u32,
    project_key: String,
    transaction_id: String,
    stable_report_sha256: String,
    ownership_token: String,
    directory_identity: String,
    manifest: SafefsManifestWire,
    completed: Vec<String>,
    active: Option<CleanupIntentWire>,
    tree_removed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SafefsManifestWire {
    digest: String,
    entries: Vec<SafefsTreeEntryWire>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SafefsTreeEntryWire {
    path: String,
    state: SafefsEntryStateWire,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SafefsEntryStateWire {
    kind: SafefsEntryKindWire,
    sha256: Option<String>,
    bytes: Option<u64>,
    unix_mode: Option<u32>,
    identity: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum SafefsEntryKindWire {
    File,
    Directory,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CleanupIntentWire {
    intent_token: String,
    progress_key: String,
    path: String,
    expected: SafefsEntryStateWire,
    root: bool,
}

impl From<&EntryState> for SafefsEntryStateWire {
    fn from(value: &EntryState) -> Self {
        Self {
            kind: match value.kind {
                EntryStateKind::File => SafefsEntryKindWire::File,
                EntryStateKind::Directory => SafefsEntryKindWire::Directory,
            },
            sha256: value.sha256.clone(),
            bytes: value.bytes,
            unix_mode: value.unix_mode,
            identity: value.identity.as_str().to_owned(),
        }
    }
}

impl TryFrom<SafefsEntryStateWire> for EntryState {
    type Error = TransactionError;

    fn try_from(value: SafefsEntryStateWire) -> Result<Self, Self::Error> {
        Ok(Self {
            kind: match value.kind {
                SafefsEntryKindWire::File => EntryStateKind::File,
                SafefsEntryKindWire::Directory => EntryStateKind::Directory,
            },
            sha256: value.sha256,
            bytes: value.bytes,
            unix_mode: value.unix_mode,
            identity: EntryIdentity::from_token(&value.identity).map_err(|error| {
                store_error(format!("invalid retirement entry identity: {error:#}"))
            })?,
        })
    }
}

impl From<&SafefsTreeManifest> for SafefsManifestWire {
    fn from(value: &SafefsTreeManifest) -> Self {
        Self {
            digest: value.digest.clone(),
            entries: value
                .entries
                .iter()
                .map(|entry| SafefsTreeEntryWire {
                    path: entry.path.clone(),
                    state: SafefsEntryStateWire::from(&entry.state),
                })
                .collect(),
        }
    }
}

impl TryFrom<SafefsManifestWire> for SafefsTreeManifest {
    type Error = TransactionError;

    fn try_from(value: SafefsManifestWire) -> Result<Self, Self::Error> {
        if value.entries.len() > MAX_TREE_ENTRIES {
            return Err(store_error("retirement manifest exceeds entry bound"));
        }
        let manifest = Self {
            digest: value.digest,
            entries: value
                .entries
                .into_iter()
                .map(|entry| {
                    Ok(SafefsTreeEntry {
                        path: entry.path,
                        state: entry.state.try_into()?,
                    })
                })
                .collect::<Result<_, TransactionError>>()?,
        };
        validate_safefs_manifest(&manifest)?;
        Ok(manifest)
    }
}

impl From<&CleanupIntent> for CleanupIntentWire {
    fn from(value: &CleanupIntent) -> Self {
        Self {
            intent_token: value.intent_token.clone(),
            progress_key: value.progress_key.clone(),
            path: value.path.clone(),
            expected: SafefsEntryStateWire::from(&value.expected),
            root: value.root,
        }
    }
}

impl TryFrom<CleanupIntentWire> for CleanupIntent {
    type Error = TransactionError;

    fn try_from(value: CleanupIntentWire) -> Result<Self, Self::Error> {
        Ok(Self {
            intent_token: value.intent_token,
            progress_key: value.progress_key,
            path: value.path,
            expected: value.expected.try_into()?,
            root: value.root,
        })
    }
}

impl TransactionStore for SystemTransactionStore {
    fn prove_outside_project(
        &mut self,
        project_display_root: &str,
    ) -> Result<(), TransactionError> {
        // A previous operation's lock is released only at the explicit start
        // of the next proof/lock sequence.
        self.external_lock = None;
        self.locked_project = None;
        self.external = None;
        self.proven_project = None;
        self.proven_display_root = None;
        let project = Project::open(Path::new(project_display_root)).map_err(|error| {
            TransactionError::Filesystem(format!("pinning scrape project: {error:#}"))
        })?;
        let identity_token = project.identity_token().map_err(|error| {
            TransactionError::Filesystem(format!("identifying pinned scrape project: {error:#}"))
        })?;
        let proven_project = derive_project_key(&identity_token);
        let external = ExternalStore::open_or_create_disjoint(&self.state_root, &project).map_err(
            |error| store_error(format!("opening external transaction root: {error:#}")),
        )?;
        external.require_durable_bootstrap().map_err(|error| {
            store_error(format!(
                "external transaction root is not durable: {error:#}"
            ))
        })?;
        self.external = Some(external);
        self.proven_project = Some(proven_project);
        self.proven_display_root = Some(project_display_root.to_owned());
        Ok(())
    }

    fn lock_project(&mut self, project: &ProjectKey) -> Result<ProjectLock, TransactionError> {
        validate_project_key(project)?;
        if self.external_lock.is_some() {
            return Err(store_error("a scrape project lock is already held"));
        }
        if self.proven_project.as_ref() != Some(project) {
            return Err(store_error(
                "project key does not identify the project pinned for the external-store proof",
            ));
        }
        let lock = self
            .external()?
            .open_and_lock_project(&project.0)
            .map_err(|error| store_error(format!("locking external project state: {error:#}")))?;
        self.external_lock = Some(lock);
        self.locked_project = Some(project.clone());
        Ok(ProjectLock::acquired())
    }

    fn pending(&mut self, project: &ProjectKey) -> Result<Option<Journal>, TransactionError> {
        self.require_locked(project)?;
        let Some(project_home) = self.open_project_home(project)? else {
            return Ok(None);
        };
        let mut transaction_ids = Vec::new();
        let mut retirement_ids = Vec::new();
        for name in project_home
            .child_names_bounded(MAX_DIRECTORY_CHILDREN)
            .map_err(|error| store_error(format!("enumerating pending transactions: {error:#}")))?
        {
            let state = project_home
                .inspect_child_state(&name)
                .map_err(|error| {
                    store_error(format!("inspecting pending entry `{name}`: {error:#}"))
                })?
                .ok_or_else(|| store_error(format!("pending entry `{name}` vanished")))?;
            match state.kind {
                EntryStateKind::Directory if valid_transaction_id_text(&name) => {
                    transaction_ids.push(TransactionId(name));
                }
                EntryStateKind::File => {
                    if let Some(id) = transaction_id_from_retirement_name(&name) {
                        retirement_ids.push(id);
                    } else {
                        return Err(store_error(format!(
                            "unexpected file `{name}` in project transaction home"
                        )));
                    }
                }
                _ => {
                    return Err(store_error(format!(
                        "unexpected entry `{name}` in project transaction home"
                    )));
                }
            }
        }

        // A crash can leave the external retirement checkpoint after the
        // owned root has already disappeared. Complete that exact checkpoint
        // before deciding whether a journal remains pending.
        for id in retirement_ids {
            self.resume_retirement(project, &id)?;
        }
        let mut still_present = Vec::new();
        for id in transaction_ids {
            if project_home
                .open_child(&id.0)
                .map_err(|error| {
                    store_error(format!(
                        "rechecking retired transaction `{}`: {error:#}",
                        id.0
                    ))
                })?
                .is_some()
            {
                still_present.push(id);
            }
        }
        let transaction_ids = still_present;
        if transaction_ids.len() > 1 {
            return Err(store_error(format!(
                "project has {} pending scrape transactions; exactly one is recoverable",
                transaction_ids.len()
            )));
        }

        transaction_ids
            .first()
            .map(|id| self.load_journal(project, id))
            .transpose()
    }

    fn verify_snapshot_progress(
        &mut self,
        journal: &Journal,
    ) -> Result<SnapshotActiveObservation, TransactionError> {
        self.require_locked(&journal.project_key)?;
        validate::journal(journal, &journal.project_key, self.proven_display_root()?)?;
        validate_snapshot_bounds(&journal.snapshots)?;

        let expected_files = journal
            .snapshots
            .iter()
            .take(journal.snapshots_persisted)
            .map(|record| record.name.clone())
            .chain(
                journal
                    .snapshot_active
                    .map(|index| journal.snapshots[index].name.clone()),
            )
            .collect::<BTreeSet<_>>();
        let expected_directories = snapshot_directories(&expected_files);
        let actual = self.snapshot_entries(journal)?;
        for (path, kind) in &actual {
            let expected = match kind {
                EntryStateKind::File => expected_files.contains(path),
                EntryStateKind::Directory => expected_directories.contains(path),
            };
            if !expected {
                return Err(store_error(format!(
                    "unjournaled snapshot entry `{path}` is present"
                )));
            }
        }

        for record in journal.snapshots.iter().take(journal.snapshots_persisted) {
            if self.read_snapshot_record(journal, record)?.is_none() {
                return Err(store_error(format!(
                    "durable-prefix snapshot `{}` is absent",
                    record.name
                )));
            }
        }

        let Some(active) = journal.snapshot_active else {
            return Ok(SnapshotActiveObservation::None);
        };
        match self.read_snapshot_record(journal, &journal.snapshots[active])? {
            Some(_) => Ok(SnapshotActiveObservation::ExactPresent),
            None => Ok(SnapshotActiveObservation::Absent),
        }
    }

    fn read_snapshot(
        &mut self,
        journal: &Journal,
        name: &str,
    ) -> Result<Vec<u8>, TransactionError> {
        SystemTransactionStore::read_snapshot(self, journal, name)
    }

    fn mint_transaction_id(
        &mut self,
        project: &ProjectKey,
    ) -> Result<TransactionId, TransactionError> {
        self.require_locked(project)?;
        let counter = TRANSACTION_NONCE.fetch_add(1, Ordering::Relaxed);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| store_error(format!("system clock precedes Unix epoch: {error}")))?;
        let mut hash = Sha256::new();
        hash.update(b"vibe-scrape-transaction-id-e1\0");
        hash.update(project.0.as_bytes());
        hash.update(std::process::id().to_be_bytes());
        hash.update(now.as_nanos().to_be_bytes());
        hash.update(counter.to_be_bytes());
        let full = format!("{:x}", hash.finalize());
        Ok(TransactionId(full[..32].to_owned()))
    }

    fn verification_workspace_intent(
        &mut self,
        journal: &Journal,
    ) -> Result<VerificationWorkspaceIntent, TransactionError> {
        self.require_locked(&journal.project_key)?;
        let display_root = self
            .external()?
            .path()
            .join(TRANSACTIONS_DIRECTORY)
            .join(project_component(&journal.project_key)?)
            .join(transaction_component(&journal.transaction_id)?)
            .join(VERIFICATION_DIRECTORY)
            .display()
            .to_string();
        Ok(VerificationWorkspaceIntent {
            name: VERIFICATION_DIRECTORY.to_owned(),
            display_root,
            ownership_token: verification_workspace_token(
                &journal.project_key,
                &journal.transaction_id,
            ),
        })
    }

    fn create_transaction(
        &mut self,
        journal: &Journal,
    ) -> Result<VerificationWorkspace, TransactionError> {
        self.require_locked(&journal.project_key)?;
        validate::journal(journal, &journal.project_key, self.proven_display_root()?)?;
        if journal.revision != 0
            || journal.state != TransactionState::Preparing
            || journal.snapshots_persisted != 0
            || journal.snapshot_active.is_some()
        {
            return Err(store_error(
                "new transaction journal is not at the initial preparation boundary",
            ));
        }
        validate_snapshot_bounds(&journal.snapshots)?;
        let expected_workspace = self.verification_workspace_intent(journal)?;
        if journal.verification_workspace.as_ref() != Some(&expected_workspace) {
            return Err(store_error(
                "revision-zero journal has a foreign verification-workspace intent",
            ));
        }
        // This exact byte vector is the revision-zero journal publication.
        // Its strict serialization and 64 MiB cap are proven before the first
        // project-home/transaction/workspace namespace creation.
        let initial_journal_bytes = strict_json_bytes(
            journal,
            MAX_TRANSACTION_JOURNAL_BYTES,
            "initial transaction journal",
        )?;
        if self.pending(&journal.project_key)?.is_some() {
            return Err(store_error(
                "a pending scrape transaction already occupies the project home",
            ));
        }

        let project_home = self.ensure_project_home(&journal.project_key)?;
        let transaction_name = transaction_component(&journal.transaction_id)?;
        let ownership_token =
            transaction_ownership_token(&journal.project_key, &journal.transaction_id);
        let owned = project_home
            .create_owned_child_exclusive(transaction_name, &ownership_token)
            .map_err(map_owned_create_error)?;
        require_namespace_checkpoint(
            owned.parent_durability(),
            "transaction-directory publication",
        )?;
        let root_state = project_home
            .inspect_child_state(transaction_name)
            .map_err(|error| store_error(format!("reobserving transaction root: {error:#}")))?
            .ok_or_else(|| store_error("new transaction root vanished after creation"))?;
        if root_state.kind != EntryStateKind::Directory {
            return Err(store_error("new transaction root is not a directory"));
        }
        let transaction_root = owned
            .directory()
            .map_err(|error| store_error(format!("retaining transaction root: {error:#}")))?;
        let workspace_token = expected_workspace.ownership_token.clone();
        let workspace = transaction_root
            .create_owned_child_exclusive(VERIFICATION_DIRECTORY, &workspace_token)
            .map_err(map_owned_create_error)?;
        require_namespace_checkpoint(
            workspace.parent_durability(),
            "verification-workspace publication",
        )?;
        let workspace_state = transaction_root
            .inspect_child_state(VERIFICATION_DIRECTORY)
            .map_err(|error| store_error(format!("reobserving verification workspace: {error:#}")))?
            .ok_or_else(|| store_error("verification workspace vanished after creation"))?;
        let workspace_project = Project::open(workspace.path()).map_err(|error| {
            store_error(format!(
                "opening verification workspace as a project: {error:#}"
            ))
        })?;
        if workspace.path().display().to_string() != expected_workspace.display_root {
            return Err(store_error(
                "created verification workspace differs from its journaled display root",
            ));
        }
        let workspace_project_identity = workspace_project
            .identity_token()
            .map_err(|error| store_error(format!("sealing verification workspace: {error:#}")))?;
        let owner = OwnerWire {
            schema: 3,
            project_key: journal.project_key.0.clone(),
            transaction_id: journal.transaction_id.0.clone(),
            journal_intent_sha256: journal_intent_sha256(journal)?,
            ownership_token,
            directory_identity: owned.identity().as_str().to_owned(),
            entry_identity: root_state.identity.as_str().to_owned(),
            workspace_directory_identity: workspace.identity().as_str().to_owned(),
            workspace_entry_identity: workspace_state.identity.as_str().to_owned(),
            workspace_project_identity: workspace_project_identity.clone(),
        };
        let owner_bytes = strict_json_bytes(&owner, MAX_OWNER_BYTES, "owner seal")?;
        self.write_durable(
            &format!(
                "{}/{}/{}/{}",
                TRANSACTIONS_DIRECTORY,
                project_component(&journal.project_key)?,
                transaction_name,
                OWNER_FILE
            ),
            &owner_bytes,
            "owner seal",
        )?;
        self.write_journal_bytes(journal, true, &initial_journal_bytes)?;
        self.live_verification_workspace = Some(workspace);
        Ok(VerificationWorkspace {
            intent: expected_workspace,
            directory_identity: owner.workspace_directory_identity,
            entry_identity: owner.workspace_entry_identity,
            project_identity_token: workspace_project_identity,
        })
    }

    fn persist_snapshot(
        &mut self,
        transaction: &TransactionId,
        record: &SnapshotRecord,
        bytes: &[u8],
    ) -> Result<(), TransactionError> {
        let project = self.locked_project()?.clone();
        let current = self.load_journal(&project, transaction)?;
        let expected = current
            .snapshots
            .get(current.snapshots_persisted)
            .filter(|_| current.snapshot_active == Some(current.snapshots_persisted))
            .ok_or_else(|| store_error("journal has no active snapshot write intent"))?;
        if expected != record {
            return Err(store_error(
                "snapshot write differs from active journal intent",
            ));
        }
        verify_snapshot_bytes(record, bytes)?;
        let relative = snapshot_relative(&project, transaction, &record.name)?;
        if let Some(existing) = self.read_snapshot_record(&current, record)? {
            if existing == bytes {
                return Ok(());
            }
            return Err(store_error(format!(
                "snapshot `{}` already exists with different bytes",
                record.name
            )));
        }
        self.write_durable(&relative, bytes, "snapshot")?;
        let reread = self
            .read_snapshot_record(&current, record)?
            .ok_or_else(|| store_error(format!("snapshot `{}` vanished", record.name)))?;
        if reread != bytes {
            return Err(store_error(format!(
                "snapshot `{}` changed after publication",
                record.name
            )));
        }
        Ok(())
    }

    fn persist_journal(&mut self, journal: &Journal) -> Result<(), TransactionError> {
        self.require_locked(&journal.project_key)?;
        validate::journal(journal, &journal.project_key, self.proven_display_root()?)?;
        let existing = self.load_journal(&journal.project_key, &journal.transaction_id)?;
        if existing == *journal {
            self.verify_snapshot_progress(&existing)?;
            return Ok(());
        }
        require_same_transaction(&existing, journal)?;
        let expected_revision = existing
            .revision
            .checked_add(1)
            .ok_or_else(|| store_error("transaction journal revision overflow"))?;
        if journal.revision != expected_revision {
            return Err(store_error(format!(
                "journal revision must advance exactly once from {} to {expected_revision}",
                existing.revision
            )));
        }
        let snapshot_observation = self.verify_snapshot_progress(&existing)?;
        validate_journal_update(&existing, journal, snapshot_observation)?;
        self.write_journal(journal, false)
    }

    fn persist_report(
        &mut self,
        report: &TransactionReport,
        canonical_wire: &[u8],
    ) -> Result<(), TransactionError> {
        self.require_locked(&report.project_key)?;
        if canonical_wire.len() > MAX_CANONICAL_REPORT_BYTES {
            return Err(store_error(format!(
                "canonical transaction report exceeds {MAX_CANONICAL_REPORT_BYTES} byte bound"
            )));
        }
        let durable = self.load_journal(&report.project_key, &report.transaction_id)?;
        if durable.report.as_ref() != Some(report) {
            return Err(store_error(
                "stable report request differs from the authoritative durable journal report",
            ));
        }
        let bytes = self.canonical_report_bytes(&durable, report)?;
        if canonical_wire != bytes {
            return Err(store_error(
                "provided stable report bytes are not the canonical durable report projection",
            ));
        }
        let relative = report_relative(&report.transaction_id)?;
        if let Some(existing) = self
            .external()?
            .read_stable_bounded(&relative, MAX_CANONICAL_REPORT_BYTES)
            .map_err(|error| store_error(format!("reading stable report: {error:#}")))?
        {
            if existing.bytes == bytes {
                return Ok(());
            }
            let old: report_wire::Report =
                strict_json_parse(&existing.bytes, "existing stable transaction report")?;
            let new: report_wire::Report =
                strict_json_parse(&bytes, "replacement stable transaction report")?;
            if !canonical_report_update_is_legal(&old, &new) {
                return Err(store_error(format!(
                    "stable report `{}` replacement is not an append-only pending cleanup transition",
                    report.transaction_id.0
                )));
            }
        }
        self.write_durable(&relative, &bytes, "stable report")?;
        let observed = self
            .external()?
            .read_stable_bounded(&relative, MAX_CANONICAL_REPORT_BYTES)
            .map_err(|error| store_error(format!("re-reading stable report: {error:#}")))?
            .ok_or_else(|| store_error("stable report vanished after publication"))?;
        if observed.bytes != bytes {
            return Err(store_error("stable report changed after publication"));
        }
        Ok(())
    }

    fn retire_transaction(&mut self, journal: &Journal) -> Result<(), TransactionError> {
        self.require_locked(&journal.project_key)?;
        let durable = self.load_journal(&journal.project_key, &journal.transaction_id)?;
        if durable != *journal {
            return Err(store_error(
                "retirement request differs from the authoritative durable journal",
            ));
        }
        if durable.state != TransactionState::Complete {
            return Err(store_error(
                "only a complete transaction journal may be retired; rollback-failed remains pending",
            ));
        }
        self.require_stable_complete_report(&durable)?;
        // The retirement primitive needs DELETE access to the workspace and
        // transaction root; release only our own live namespace seal after
        // the authoritative complete journal and stable report were proved.
        self.live_verification_workspace = None;
        self.resume_retirement(&durable.project_key, &durable.transaction_id)
    }
}

impl SystemTransactionStore {
    fn external(&self) -> Result<&ExternalStore, TransactionError> {
        self.external.as_ref().ok_or_else(|| {
            store_error("external state root has not been proven against the project")
        })
    }

    fn proven_display_root(&self) -> Result<&str, TransactionError> {
        self.proven_display_root
            .as_deref()
            .ok_or_else(|| store_error("pinned project display root is absent"))
    }

    fn locked_project(&self) -> Result<&ProjectKey, TransactionError> {
        if self.external_lock.is_none() {
            return Err(store_error("external project lock is not held"));
        }
        self.locked_project
            .as_ref()
            .ok_or_else(|| store_error("locked project identity is absent"))
    }

    fn require_locked(&self, project: &ProjectKey) -> Result<(), TransactionError> {
        self.external_lock
            .as_ref()
            .ok_or_else(|| store_error("external project lock is not held"))?
            .require_still_named()
            .map_err(|error| store_error(format!("rechecking held project lock: {error:#}")))?;
        let locked = self.locked_project()?;
        if locked != project {
            return Err(store_error(
                "operation names a project other than the locked project",
            ));
        }
        Ok(())
    }

    fn ensure_project_home(
        &self,
        project: &ProjectKey,
    ) -> Result<ExternalDirectory, TransactionError> {
        let root = self
            .external()?
            .root_directory()
            .map_err(|error| store_error(format!("opening external root capability: {error:#}")))?;
        let (transactions, _, durability) = root
            .ensure_child(TRANSACTIONS_DIRECTORY)
            .map_err(|error| store_error(format!("ensuring transaction directory: {error:#}")))?;
        require_optional_sync(durability, "transaction-home parent")?;
        let (project_home, _, durability) = transactions
            .ensure_child(project_component(project)?)
            .map_err(|error| {
                store_error(format!("ensuring project transaction home: {error:#}"))
            })?;
        require_optional_sync(durability, "project transaction-home parent")?;
        Ok(project_home)
    }

    fn open_project_home(
        &self,
        project: &ProjectKey,
    ) -> Result<Option<ExternalDirectory>, TransactionError> {
        self.external()?
            .open_directory(&format!(
                "{}/{}",
                TRANSACTIONS_DIRECTORY,
                project_component(project)?
            ))
            .map_err(|error| store_error(format!("opening project transaction home: {error:#}")))
    }

    fn transaction_directory(
        &self,
        project: &ProjectKey,
        transaction: &TransactionId,
    ) -> Result<Option<ExternalDirectory>, TransactionError> {
        let Some(project_home) = self.open_project_home(project)? else {
            return Ok(None);
        };
        project_home
            .open_child(transaction_component(transaction)?)
            .map_err(|error| store_error(format!("opening transaction directory: {error:#}")))
    }

    fn load_owner(
        &self,
        project: &ProjectKey,
        transaction: &TransactionId,
    ) -> Result<(OwnerWire, ExternalDirectory), TransactionError> {
        let project_home = self
            .open_project_home(project)?
            .ok_or_else(|| store_error("project transaction home is absent"))?;
        let name = transaction_component(transaction)?;
        let root_state = project_home
            .inspect_child_state(name)
            .map_err(|error| store_error(format!("inspecting transaction root: {error:#}")))?
            .ok_or_else(|| store_error("transaction root is absent"))?;
        if root_state.kind != EntryStateKind::Directory {
            return Err(store_error("transaction root is not a directory"));
        }
        let directory = project_home
            .open_child(name)
            .map_err(|error| store_error(format!("opening transaction root: {error:#}")))?
            .ok_or_else(|| store_error("transaction root vanished while opening"))?;
        let owner = directory
            .read_stable_bounded(OWNER_FILE, MAX_OWNER_BYTES)
            .map_err(|error| store_error(format!("reading transaction owner seal: {error:#}")))?
            .ok_or_else(|| store_error("transaction owner seal is absent"))?;
        let owner: OwnerWire = strict_json_parse(&owner.bytes, "owner seal")?;
        let expected_token = transaction_ownership_token(project, transaction);
        if owner.schema != 3
            || owner.project_key != project.0
            || owner.transaction_id != transaction.0
            || !owner
                .journal_intent_sha256
                .strip_prefix("sha256:")
                .is_some_and(valid_lower_hex_digest)
            || owner.ownership_token != expected_token
            || owner.entry_identity != root_state.identity.as_str()
            || !owner
                .workspace_project_identity
                .strip_prefix("sha256:")
                .is_some_and(valid_lower_hex_digest)
        {
            return Err(store_error(
                "transaction owner seal differs from the selected project/root identity",
            ));
        }
        OwnedDirectoryIdentity::from_token(&owner.directory_identity).map_err(|error| {
            store_error(format!("invalid owned-directory identity seal: {error:#}"))
        })?;
        OwnedDirectoryIdentity::from_token(&owner.workspace_directory_identity).map_err(
            |error| store_error(format!("invalid workspace directory identity: {error:#}")),
        )?;
        EntryIdentity::from_token(&owner.workspace_entry_identity)
            .map_err(|error| store_error(format!("invalid workspace entry identity: {error:#}")))?;
        Ok((owner, directory))
    }

    fn load_journal(
        &self,
        project: &ProjectKey,
        transaction: &TransactionId,
    ) -> Result<Journal, TransactionError> {
        let (owner, directory) = self.load_owner(project, transaction)?;
        let bytes = directory
            .read_stable_bounded(JOURNAL_FILE, MAX_TRANSACTION_JOURNAL_BYTES)
            .map_err(|error| store_error(format!("reading transaction journal: {error:#}")))?
            .ok_or_else(|| store_error("transaction journal is absent"))?
            .bytes;
        let journal: Journal = strict_json_parse(&bytes, "transaction journal")?;
        if journal.transaction_id != *transaction {
            return Err(store_error(
                "transaction directory and journal identity differ",
            ));
        }
        if journal_intent_sha256(&journal)? != owner.journal_intent_sha256 {
            return Err(store_error(
                "transaction journal immutable intent differs from its owner seal",
            ));
        }
        validate::journal(&journal, project, self.proven_display_root()?)?;
        validate_workspace_owner(&directory, &owner, &journal)?;
        validate_snapshot_bounds(&journal.snapshots)?;
        Ok(journal)
    }

    fn write_journal(&self, journal: &Journal, initial: bool) -> Result<(), TransactionError> {
        let bytes = strict_json_bytes(
            journal,
            MAX_TRANSACTION_JOURNAL_BYTES,
            "transaction journal",
        )?;
        self.write_journal_bytes(journal, initial, &bytes)
    }

    fn write_journal_bytes(
        &self,
        journal: &Journal,
        initial: bool,
        bytes: &[u8],
    ) -> Result<(), TransactionError> {
        let (_, directory) = self.load_owner(&journal.project_key, &journal.transaction_id)?;
        let existing = directory
            .read_stable_bounded(JOURNAL_FILE, MAX_TRANSACTION_JOURNAL_BYTES)
            .map_err(|error| store_error(format!("checking transaction journal: {error:#}")))?;
        if initial && existing.is_some() {
            return Err(store_error("new transaction journal already exists"));
        }
        if !initial && existing.is_none() {
            return Err(store_error(
                "transaction journal vanished before persistence",
            ));
        }
        self.write_durable(
            &journal_relative(&journal.project_key, &journal.transaction_id)?,
            bytes,
            "transaction journal",
        )?;
        let observed = directory
            .read_stable_bounded(JOURNAL_FILE, MAX_TRANSACTION_JOURNAL_BYTES)
            .map_err(|error| store_error(format!("re-reading transaction journal: {error:#}")))?
            .ok_or_else(|| store_error("transaction journal vanished after persistence"))?;
        if observed.bytes != bytes {
            return Err(store_error("transaction journal changed after persistence"));
        }
        Ok(())
    }

    fn write_durable(
        &self,
        relative: &str,
        bytes: &[u8],
        label: &str,
    ) -> Result<(), TransactionError> {
        let write = self
            .external()?
            .write_durable(relative, bytes)
            .map_err(|error| store_error(format!("publishing {label}: {error}")))?;
        if !write.file_synced {
            return Err(store_error(format!("{label} data was not durably flushed")));
        }
        require_namespace_checkpoint(write.parent, &format!("{label} parent"))?;
        for sync in write.directory_syncs {
            require_namespace_checkpoint(
                sync.durability,
                &format!("{label} directory `{}`", sync.directory.display()),
            )?;
        }
        Ok(())
    }

    fn read_snapshot_record(
        &self,
        journal: &Journal,
        record: &SnapshotRecord,
    ) -> Result<Option<Vec<u8>>, TransactionError> {
        validate_snapshot_record_bound(record)?;
        let relative =
            snapshot_relative(&journal.project_key, &journal.transaction_id, &record.name)?;
        let Some(snapshot) = self
            .external()?
            .read_stable_bounded(
                &relative,
                usize::try_from(record.bytes).map_err(|_| {
                    store_error(format!(
                        "snapshot `{}` size is not addressable",
                        record.name
                    ))
                })?,
            )
            .map_err(|error| {
                store_error(format!("reading snapshot `{}`: {error:#}", record.name))
            })?
        else {
            return Ok(None);
        };
        verify_snapshot_bytes(record, &snapshot.bytes)?;
        let (parent, name) = split_parent(&record.name)?;
        let transaction = self
            .transaction_directory(&journal.project_key, &journal.transaction_id)?
            .ok_or_else(|| store_error("transaction root vanished while reading snapshot"))?;
        let snapshots = transaction
            .open_child(SNAPSHOTS_DIRECTORY)
            .map_err(|error| store_error(format!("opening snapshot root: {error:#}")))?
            .ok_or_else(|| store_error("snapshot root vanished while reading snapshot"))?;
        let state = if parent.is_empty() {
            snapshots.inspect_child_state(name)
        } else {
            let directory = open_descendant(&snapshots, parent)?
                .ok_or_else(|| store_error(format!("snapshot parent `{parent}` vanished")))?;
            directory.inspect_child_state(name)
        }
        .map_err(|error| store_error(format!("inspecting snapshot `{}`: {error:#}", record.name)))?
        .ok_or_else(|| store_error(format!("snapshot `{}` vanished", record.name)))?;
        let expected_sha256 = record
            .sha256
            .0
            .strip_prefix("sha256:")
            .ok_or_else(|| store_error("snapshot journal digest has no sha256 prefix"))?;
        if state.kind != EntryStateKind::File
            || state.sha256.as_deref() != Some(expected_sha256)
            || state.bytes != Some(record.bytes)
            || state.unix_mode != record.mode
        {
            return Err(store_error(format!(
                "snapshot `{}` metadata differs from its journal record",
                record.name
            )));
        }
        Ok(Some(snapshot.bytes))
    }

    fn snapshot_entries(
        &self,
        journal: &Journal,
    ) -> Result<Vec<(String, EntryStateKind)>, TransactionError> {
        let Some(transaction) =
            self.transaction_directory(&journal.project_key, &journal.transaction_id)?
        else {
            return Err(store_error("transaction root is absent"));
        };
        let Some(root) = transaction
            .open_child(SNAPSHOTS_DIRECTORY)
            .map_err(|error| store_error(format!("opening snapshot root: {error:#}")))?
        else {
            return Ok(Vec::new());
        };
        let mut entries = Vec::new();
        collect_external_entries(&root, "", &mut entries, 0)?;
        Ok(entries)
    }

    fn resume_retirement(
        &mut self,
        project: &ProjectKey,
        transaction: &TransactionId,
    ) -> Result<(), TransactionError> {
        let project_home = match self.open_project_home(project)? {
            Some(home) => home,
            None => return Ok(()),
        };
        let sidecar_name = retirement_name(transaction)?;
        let mut wire = match project_home
            .read_stable_bounded(&sidecar_name, MAX_RETIREMENT_BYTES)
            .map_err(|error| store_error(format!("reading retirement checkpoint: {error:#}")))?
        {
            Some(bytes) => {
                let wire: RetirementWire =
                    strict_json_parse(&bytes.bytes, "retirement checkpoint")?;
                validate_retirement_identity(&wire, project, transaction)?;
                self.validate_stable_retirement_report(&wire)?;
                wire
            }
            None => {
                let durable = self.load_journal(project, transaction)?;
                if durable.state != TransactionState::Complete {
                    return Err(store_error(
                        "retirement cannot begin before the durable journal is complete",
                    ));
                }
                self.verify_snapshot_progress(&durable)?;
                let stable_report = self.require_stable_complete_report(&durable)?;
                let (owner, transaction_directory) = match self.load_owner(project, transaction) {
                    Ok(value) => value,
                    Err(error)
                        if project_home
                            .open_child(transaction_component(transaction)?)
                            .map_err(|source| {
                                store_error(format!(
                                    "checking retired transaction root: {source:#}"
                                ))
                            })?
                            .is_none() =>
                    {
                        return Ok(());
                    }
                    Err(error) => return Err(error),
                };
                validate_transaction_home_shape(&transaction_directory, &durable, &owner)?;
                let manifest = observe_safefs_manifest(&transaction_directory)?;
                // Close the pre-sidecar adoption window: the manifest may
                // contain only the exact authority re-proved after observation.
                // Anything inserted during the observation is therefore
                // rejected before its identity can become cleanup authority.
                validate_transaction_home_shape(&transaction_directory, &durable, &owner)?;
                self.verify_snapshot_progress(&durable)?;
                if self.load_journal(project, transaction)? != durable {
                    return Err(store_error(
                        "transaction journal changed while sealing retirement manifest",
                    ));
                }
                let wire = RetirementWire {
                    schema: 2,
                    project_key: project.0.clone(),
                    transaction_id: transaction.0.clone(),
                    stable_report_sha256: sha256_bytes(&stable_report),
                    ownership_token: owner.ownership_token,
                    directory_identity: owner.directory_identity,
                    manifest: SafefsManifestWire::from(&manifest),
                    completed: Vec::new(),
                    active: None,
                    tree_removed: false,
                };
                self.write_retirement(project, transaction, &wire)?;
                wire
            }
        };

        let identity = OwnedDirectoryIdentity::from_token(&wire.directory_identity)
            .map_err(|error| store_error(format!("invalid retirement root identity: {error:#}")))?;
        let manifest: SafefsTreeManifest = wire.manifest.clone().try_into()?;
        let mut progress = OwnedTreeCleanupProgress::from_completed(wire.completed.clone())
            .map_err(|error| store_error(format!("invalid retirement progress: {error:#}")))?;

        loop {
            if wire.tree_removed {
                if project_home
                    .open_child(transaction_component(transaction)?)
                    .map_err(|error| {
                        store_error(format!("checking retired transaction: {error:#}"))
                    })?
                    .is_some()
                {
                    return Err(store_error(
                        "retirement checkpoint says removed but transaction root is present",
                    ));
                }
                return self.remove_retirement_sidecar(&project_home, &sidecar_name);
            }

            if let Some(active) = wire.active.take() {
                let intent: CleanupIntent = active.try_into()?;
                let completion = project_home
                    .execute_owned_child_retirement(
                        transaction_component(transaction)?,
                        &wire.ownership_token,
                        &identity,
                        &manifest,
                        &progress,
                        &intent,
                    )
                    .map_err(map_cleanup_error)?;
                require_namespace_checkpoint(
                    completion.durability(),
                    &format!("retirement step `{}`", completion.progress_key()),
                )?;
                progress.record(&completion).map_err(|error| {
                    store_error(format!("recording retirement progress: {error:#}"))
                })?;
                wire.completed = progress.completed().to_vec();
                wire.tree_removed = intent.root;
                self.write_retirement(project, transaction, &wire)?;
                continue;
            }

            match project_home
                .prepare_owned_child_retirement(
                    transaction_component(transaction)?,
                    &wire.ownership_token,
                    &identity,
                    &manifest,
                    &progress,
                )
                .map_err(map_cleanup_error)?
            {
                CleanupPreparation::Complete => {
                    wire.tree_removed = true;
                    self.write_retirement(project, transaction, &wire)?;
                }
                CleanupPreparation::Intent(intent) => {
                    wire.active = Some(CleanupIntentWire::from(&intent));
                    self.write_retirement(project, transaction, &wire)?;
                }
            }
        }
    }

    fn write_retirement(
        &self,
        project: &ProjectKey,
        transaction: &TransactionId,
        wire: &RetirementWire,
    ) -> Result<(), TransactionError> {
        let bytes = strict_json_bytes(wire, MAX_RETIREMENT_BYTES, "retirement checkpoint")?;
        self.write_durable(
            &retirement_relative(project, transaction)?,
            &bytes,
            "retirement checkpoint",
        )
    }

    fn validate_stable_retirement_report(
        &self,
        wire: &RetirementWire,
    ) -> Result<(), TransactionError> {
        let transaction = TransactionId(wire.transaction_id.clone());
        let observed = self
            .external()?
            .read_stable_bounded(&report_relative(&transaction)?, MAX_CANONICAL_REPORT_BYTES)
            .map_err(|error| store_error(format!("reading retirement stable report: {error:#}")))?
            .ok_or_else(|| store_error("retirement stable report is absent"))?;
        if sha256_bytes(&observed.bytes) != wire.stable_report_sha256 {
            return Err(store_error(
                "retirement stable report differs from the sealed canonical report",
            ));
        }
        let report: report_wire::Report =
            strict_json_parse(&observed.bytes, "retirement stable report")?;
        if report.schema != 1
            || report.transaction_id != wire.transaction_id
            || report.cleanup != report_wire::ReportCleanup::Complete
        {
            return Err(store_error(
                "retirement stable report is not the selected cleanup-complete transaction",
            ));
        }
        Ok(())
    }

    fn remove_retirement_sidecar(
        &self,
        project_home: &ExternalDirectory,
        sidecar_name: &str,
    ) -> Result<(), TransactionError> {
        let Some(state) = project_home
            .inspect_child_state(sidecar_name)
            .map_err(|error| store_error(format!("inspecting retirement checkpoint: {error:#}")))?
        else {
            return Ok(());
        };
        let durability = project_home
            .remove_file_expected(sidecar_name, &state)
            .map_err(map_cleanup_error)?;
        require_namespace_checkpoint(durability, "retirement-checkpoint removal")
    }
}

fn store_error(message: impl Into<String>) -> TransactionError {
    TransactionError::Store(message.into())
}

fn strict_json_bytes<T: Serialize>(
    value: &T,
    maximum: usize,
    label: &str,
) -> Result<Vec<u8>, TransactionError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| store_error(format!("encoding {label}: {error}")))?;
    if bytes.len() > maximum {
        return Err(store_error(format!(
            "encoded {label} exceeds {maximum} byte bound"
        )));
    }
    Ok(bytes)
}

fn serialize_canonical_plan<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let value = std::str::from_utf8(bytes).map_err(serde::ser::Error::custom)?;
    serializer.serialize_str(value)
}

fn deserialize_canonical_plan<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    String::deserialize(deserializer).map(String::into_bytes)
}

fn strict_json_parse<'a, T: Deserialize<'a>>(
    bytes: &'a [u8],
    label: &str,
) -> Result<T, TransactionError> {
    serde_json::from_slice(bytes)
        .map_err(|error| store_error(format!("invalid strict {label} JSON: {error}")))
}

fn validate_canonical_report_identity(
    journal: &Journal,
    report: &TransactionReport,
    wire: &report_wire::Report,
) -> Result<(), TransactionError> {
    let mode_matches = matches!(
        (report.mode, &wire.mode),
        (TransactionMode::Export, report_wire::ReportMode::Export)
            | (TransactionMode::InPlace, report_wire::ReportMode::InPlace)
    );
    let outcome_matches = matches!(
        (report.outcome, &wire.outcome),
        (Outcome::Verified, report_wire::ReportOutcome::Verified)
            | (Outcome::Refused, report_wire::ReportOutcome::Refused)
            | (Outcome::RolledBack, report_wire::ReportOutcome::RolledBack)
            | (
                Outcome::RollbackFailed,
                report_wire::ReportOutcome::RollbackFailed
            )
    );
    let assurance_matches = matches!(
        (report.assurance, &wire.assurance),
        (Assurance::Full, report_wire::ReportAssurance::Full)
            | (Assurance::Reduced, report_wire::ReportAssurance::Reduced)
    );
    let cleanup_matches = matches!(
        (report.cleanup, &wire.cleanup),
        (Cleanup::Complete, report_wire::ReportCleanup::Complete)
            | (Cleanup::Pending, report_wire::ReportCleanup::Pending)
    );
    let before_tree = report
        .before_tree
        .as_ref()
        .map_or("", |digest| digest.0.as_str());
    if wire.schema != 1
        || !matches!(&wire.command, report_wire::ReportCommand::Scrape)
        || wire.transaction_id != report.transaction_id.0
        || wire.plan_id != report.plan_id.0
        || wire.project_display_root != journal.project_display_root
        || wire.before_tree_digest != before_tree
        || wire.after_tree_digest != report.after_tree.as_ref().map(|digest| digest.0.clone())
        || !mode_matches
        || !outcome_matches
        || !assurance_matches
        || !cleanup_matches
    {
        return Err(store_error(
            "canonical scrape report identity/outcome differs from durable journal evidence",
        ));
    }
    Ok(())
}

fn canonical_report_update_is_legal(old: &report_wire::Report, new: &report_wire::Report) -> bool {
    old.schema == new.schema
        && old.command == new.command
        && old.transaction_id == new.transaction_id
        && old.plan_id == new.plan_id
        && old.project_display_root == new.project_display_root
        && old.mode == new.mode
        && old.outcome == new.outcome
        && old.assurance == new.assurance
        && old.before_tree_digest == new.before_tree_digest
        && old.after_tree_digest == new.after_tree_digest
        && old.deleted_artifacts == new.deleted_artifacts
        && old.dependency_graphs == new.dependency_graphs
        && old.health == new.health
        && old.relocations == new.relocations
        && old.residuals == new.residuals
        && old.rewrites == new.rewrites
        && old.unchanged_files == new.unchanged_files
        && is_prefix(&old.recovery, &new.recovery)
        && is_prefix(&old.rollback, &new.rollback)
        && matches!(
            (&old.cleanup, &new.cleanup),
            (
                report_wire::ReportCleanup::Pending,
                report_wire::ReportCleanup::Pending | report_wire::ReportCleanup::Complete
            )
        )
}

fn validate_project_key(project: &ProjectKey) -> Result<(), TransactionError> {
    let Some(hex) = project.0.strip_prefix("sha256:") else {
        return Err(store_error(
            "project key must use sha256:<64-lowercase-hex>",
        ));
    };
    if !valid_lower_hex_digest(hex) {
        return Err(store_error(
            "project key must use sha256:<64-lowercase-hex>",
        ));
    }
    Ok(())
}

fn project_component(project: &ProjectKey) -> Result<&str, TransactionError> {
    validate_project_key(project)?;
    Ok(&project.0["sha256:".len()..])
}

fn valid_lower_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_transaction_id_text(value: &str) -> bool {
    (6..=64).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

fn transaction_component(transaction: &TransactionId) -> Result<&str, TransactionError> {
    if !valid_transaction_id_text(&transaction.0) {
        return Err(store_error(
            "transaction id is outside 6..64 ASCII alphanumerics",
        ));
    }
    Ok(&transaction.0)
}

fn transaction_ownership_token(project: &ProjectKey, transaction: &TransactionId) -> String {
    let mut hash = Sha256::new();
    hash.update(b"vibe-scrape-store-owner-e1\0");
    hash.update(project.0.as_bytes());
    hash.update(b"\0");
    hash.update(transaction.0.as_bytes());
    format!("sha256:{:x}", hash.finalize())
}

fn verification_workspace_token(project: &ProjectKey, transaction: &TransactionId) -> String {
    let mut hash = Sha256::new();
    hash.update(b"vibe-scrape-verification-workspace-e1\0");
    hash.update(project.0.as_bytes());
    hash.update(b"\0");
    hash.update(transaction.0.as_bytes());
    format!("sha256:{:x}", hash.finalize())
}

fn journal_intent_sha256(journal: &Journal) -> Result<String, TransactionError> {
    let mut hash = Sha256::new();
    hash.update(b"vibe-scrape-store-journal-intent-e1\0");
    hash_intent_part(&mut hash, &journal.schema.to_be_bytes());
    hash_intent_part(&mut hash, journal.project_key.0.as_bytes());
    hash_intent_part(&mut hash, journal.transaction_id.0.as_bytes());
    hash_intent_part(
        &mut hash,
        &serde_json::to_vec(&journal.mode)
            .map_err(|error| store_error(format!("encoding journal mode intent: {error}")))?,
    );
    hash_intent_part(&mut hash, journal.plan_id.0.as_bytes());
    hash_intent_part(&mut hash, journal.project_display_root.as_bytes());
    hash_intent_part(&mut hash, &journal.canonical_plan);
    hash_intent_part(
        &mut hash,
        &serde_json::to_vec(&journal.verification_workspace).map_err(|error| {
            store_error(format!("encoding verification workspace intent: {error}"))
        })?,
    );
    hash_intent_part(
        &mut hash,
        &serde_json::to_vec(&journal.execution)
            .map_err(|error| store_error(format!("encoding journal execution intent: {error}")))?,
    );
    hash_intent_part(
        &mut hash,
        &serde_json::to_vec(&journal.snapshots)
            .map_err(|error| store_error(format!("encoding journal snapshot intent: {error}")))?,
    );
    Ok(format!("sha256:{:x}", hash.finalize()))
}

fn hash_intent_part(hash: &mut Sha256, bytes: &[u8]) {
    hash.update((bytes.len() as u64).to_be_bytes());
    hash.update(bytes);
}

fn journal_relative(
    project: &ProjectKey,
    transaction: &TransactionId,
) -> Result<String, TransactionError> {
    Ok(format!(
        "{}/{}/{}/{}",
        TRANSACTIONS_DIRECTORY,
        project_component(project)?,
        transaction_component(transaction)?,
        JOURNAL_FILE
    ))
}

fn snapshot_relative(
    project: &ProjectKey,
    transaction: &TransactionId,
    name: &str,
) -> Result<String, TransactionError> {
    validate_relative(name, "snapshot name")?;
    Ok(format!(
        "{}/{}/{}/{}/{}",
        TRANSACTIONS_DIRECTORY,
        project_component(project)?,
        transaction_component(transaction)?,
        SNAPSHOTS_DIRECTORY,
        name
    ))
}

fn report_relative(transaction: &TransactionId) -> Result<String, TransactionError> {
    Ok(format!(
        "{}/{}.json",
        REPORTS_DIRECTORY,
        transaction_component(transaction)?
    ))
}

fn retirement_name(transaction: &TransactionId) -> Result<String, TransactionError> {
    Ok(format!(
        "{}.retire.json",
        transaction_component(transaction)?
    ))
}

fn retirement_relative(
    project: &ProjectKey,
    transaction: &TransactionId,
) -> Result<String, TransactionError> {
    Ok(format!(
        "{}/{}/{}",
        TRANSACTIONS_DIRECTORY,
        project_component(project)?,
        retirement_name(transaction)?
    ))
}

fn transaction_id_from_retirement_name(name: &str) -> Option<TransactionId> {
    let transaction = name.strip_suffix(".retire.json")?;
    valid_transaction_id_text(transaction).then(|| TransactionId(transaction.to_owned()))
}

fn validate_relative(value: &str, label: &str) -> Result<(), TransactionError> {
    if value.is_empty()
        || value.starts_with('/')
        || value.ends_with('/')
        || value.contains(['\\', ':', '\0'])
        || value
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err(store_error(format!("unsafe {label} `{value}`")));
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/'))
    {
        return Err(store_error(format!("non-portable {label} `{value}`")));
    }
    Ok(())
}

fn split_parent(relative: &str) -> Result<(&str, &str), TransactionError> {
    validate_relative(relative, "relative path")?;
    Ok(relative.rsplit_once('/').unwrap_or(("", relative)))
}

fn open_descendant(
    root: &ExternalDirectory,
    relative: &str,
) -> Result<Option<ExternalDirectory>, TransactionError> {
    if relative.is_empty() {
        // ExternalDirectory is intentionally not Clone. Reopening `.` is not
        // admitted, so callers special-case the root parent instead.
        return Err(store_error(
            "internal attempt to reopen an empty directory path",
        ));
    }
    validate_relative(relative, "directory path")?;
    let mut components = relative.split('/');
    let first = components.next().expect("validated nonempty path");
    let Some(mut current) = root
        .open_child(first)
        .map_err(|error| store_error(format!("opening directory `{first}`: {error:#}")))?
    else {
        return Ok(None);
    };
    for component in components {
        let Some(next) = current
            .open_child(component)
            .map_err(|error| store_error(format!("opening directory `{component}`: {error:#}")))?
        else {
            return Ok(None);
        };
        current = next;
    }
    Ok(Some(current))
}

fn require_optional_sync(
    durability: Option<DirectoryDurability>,
    label: &str,
) -> Result<(), TransactionError> {
    if let Some(durability) = durability {
        require_namespace_checkpoint(durability, label)?;
    }
    Ok(())
}

fn require_namespace_checkpoint(
    durability: DirectoryDurability,
    label: &str,
) -> Result<(), TransactionError> {
    if matches!(
        durability,
        DirectoryDurability::Synced | DirectoryDurability::JournalRecoverable
    ) {
        Ok(())
    } else {
        Err(store_error(format!(
            "{label} did not provide usable namespace durability/recovery evidence: {durability:?}"
        )))
    }
}

fn map_owned_create_error(error: OwnedDirectoryCreateError) -> TransactionError {
    match error {
        OwnedDirectoryCreateError::Unsupported => {
            TransactionError::MissingPrimitive(RequiredPrimitive::ExclusivePinnedDirectory)
        }
        other => store_error(format!("creating owned transaction directory: {other}")),
    }
}

fn map_cleanup_error(error: OwnedTreeCleanupError) -> TransactionError {
    match error {
        OwnedTreeCleanupError::Third { detail } => TransactionError::ThirdState(detail),
        OwnedTreeCleanupError::Unsupported => {
            TransactionError::MissingPrimitive(RequiredPrimitive::ExactManifestTreeRemoval)
        }
        OwnedTreeCleanupError::Io(error) => {
            store_error(format!("retiring owned transaction directory: {error:#}"))
        }
    }
}

fn require_same_transaction(old: &Journal, new: &Journal) -> Result<(), TransactionError> {
    if old.schema != new.schema
        || old.project_key != new.project_key
        || old.transaction_id != new.transaction_id
        || old.mode != new.mode
        || old.plan_id != new.plan_id
        || old.project_display_root != new.project_display_root
        || old.canonical_plan != new.canonical_plan
        || old.verification_workspace != new.verification_workspace
        || old.execution != new.execution
        || old.snapshots != new.snapshots
    {
        return Err(store_error(
            "journal persistence attempted to change immutable transaction intent",
        ));
    }
    Ok(())
}

fn validate_journal_update(
    old: &Journal,
    new: &Journal,
    snapshot_observation: SnapshotActiveObservation,
) -> Result<(), TransactionError> {
    validate_snapshot_update(old, new, snapshot_observation)?;
    validate_state_update(old, new)?;
    validate_step_update(old, new)?;
    validate_cleanup_wal_update(old, new)?;

    if !is_prefix(&old.verification, &new.verification) {
        return Err(store_error(
            "journal verification evidence is not append-only",
        ));
    }
    if !is_prefix(&old.events, &new.events) {
        return Err(store_error("journal events are not append-only"));
    }
    if !is_prefix(&old.actual_mutations, &new.actual_mutations) {
        return Err(store_error("actual mutation evidence is not append-only"));
    }
    for (index, evidence) in new.actual_mutations.iter().enumerate() {
        if new.actual_mutations[..index]
            .iter()
            .any(|prior| prior.id == evidence.id && prior.direction == evidence.direction)
        {
            return Err(store_error(
                "actual mutation evidence repeats an id/direction",
            ));
        }
    }
    if old
        .settlement_intent
        .is_some_and(|intent| new.settlement_intent != Some(intent))
    {
        return Err(store_error(
            "durable settlement intent changed or disappeared",
        ));
    }
    if old
        .delivered_tree
        .as_ref()
        .is_some_and(|tree| new.delivered_tree.as_ref() != Some(tree))
    {
        return Err(store_error("delivered-tree proof changed or disappeared"));
    }
    if old
        .candidate_name
        .as_ref()
        .is_some_and(|name| new.candidate_name.as_ref() != Some(name))
        || old
            .quarantine_name
            .as_ref()
            .is_some_and(|name| new.quarantine_name.as_ref() != Some(name))
        || old
            .owned_tree_token
            .as_ref()
            .is_some_and(|token| new.owned_tree_token.as_ref() != Some(token))
    {
        return Err(store_error(
            "journaled owned-tree name or ownership token changed or disappeared",
        ));
    }
    match (&old.report, &new.report) {
        (None, _) | (Some(_), Some(_)) if report_update_is_legal(&old.report, &new.report) => {}
        _ => {
            return Err(store_error(
                "embedded transaction report regressed or changed evidence",
            ));
        }
    }
    Ok(())
}

fn validate_snapshot_update(
    old: &Journal,
    new: &Journal,
    observation: SnapshotActiveObservation,
) -> Result<(), TransactionError> {
    if old.snapshots_persisted == new.snapshots_persisted
        && old.snapshot_active == new.snapshot_active
    {
        return Ok(());
    }
    match (old.snapshot_active, new.snapshot_active) {
        (None, Some(active))
            if old.snapshots_persisted == new.snapshots_persisted
                && active == old.snapshots_persisted
                && observation == SnapshotActiveObservation::None =>
        {
            Ok(())
        }
        (Some(active), None)
            if new.snapshots_persisted == old.snapshots_persisted + 1
                && active == old.snapshots_persisted
                && observation == SnapshotActiveObservation::ExactPresent =>
        {
            Ok(())
        }
        _ => Err(store_error(
            "snapshot checkpoint regressed, skipped intent/data, or advanced without exact data",
        )),
    }
}

fn validate_state_update(old: &Journal, new: &Journal) -> Result<(), TransactionError> {
    use TransactionState as State;

    if old.state == new.state {
        return Ok(());
    }
    let ordinary = match (&old.state, &new.state, old.mode) {
        (State::Preparing, State::Prepared, _) => true,
        (State::Prepared, State::Candidate, TransactionMode::Export)
        | (State::Candidate, State::PublishedPendingVerify, TransactionMode::Export) => true,
        (State::Prepared, State::BeforePassed, TransactionMode::InPlace)
        | (State::BeforePassed, State::Mutating, TransactionMode::InPlace)
        | (State::Mutating, State::ContractBoundary(_), TransactionMode::InPlace) => true,
        (State::ContractBoundary(old_action), State::ContractBoundary(new_action), _)
            if old_action == new_action =>
        {
            true
        }
        (State::PublishedPendingVerify, State::Verified, TransactionMode::Export)
        | (State::ContractBoundary(_), State::Verified, TransactionMode::InPlace)
        | (State::Verified, State::CleanupPending, _)
        | (State::CleanupPending, State::Complete, _)
        | (State::RollingBack, State::RolledBack | State::RollbackFailed, _)
        | (State::RolledBack, State::Complete, _) => true,
        (
            State::Prepared | State::Candidate | State::PublishedPendingVerify,
            State::RollingBack,
            TransactionMode::Export,
        )
        | (
            State::Prepared | State::BeforePassed | State::Mutating | State::ContractBoundary(_),
            State::RollingBack,
            TransactionMode::InPlace,
        ) => true,
        _ => false,
    };
    let terminal_refusal = new.state == State::Complete
        && new
            .report
            .as_ref()
            .is_some_and(|report| report.outcome == Outcome::Refused)
        && matches!(old.state, State::Preparing | State::Prepared);
    if ordinary || terminal_refusal {
        Ok(())
    } else {
        Err(store_error(format!(
            "illegal durable transaction state transition {:?} -> {:?}",
            old.state, new.state
        )))
    }
}

fn validate_step_update(old: &Journal, new: &Journal) -> Result<(), TransactionError> {
    if new.completed_steps < old.completed_steps
        || new.completed_steps > old.completed_steps.saturating_add(1)
    {
        return Err(store_error(
            "completed mutation prefix regressed or skipped a step",
        ));
    }
    match (old.active_step, new.active_step) {
        (None, Some(active))
            if new.completed_steps == old.completed_steps && active == old.completed_steps => {}
        (Some(old_active), Some(new_active))
            if old_active == new_active && new.completed_steps == old.completed_steps => {}
        (Some(active), None)
            if active == old.completed_steps
                && new.completed_steps == old.completed_steps.saturating_add(1) => {}
        (None, None) if new.completed_steps == old.completed_steps => {}
        _ => {
            return Err(store_error(
                "mutation step checkpoint regressed, skipped intent, or advanced out of order",
            ));
        }
    }
    for (prior, next) in old.mutation_progress.iter().zip(&new.mutation_progress) {
        if prior.id != next.id
            || prior.kind != next.kind
            || !mutation_status_update_is_legal(prior.status, next.status)
            || (next.status == MutationStatus::NoMutation
                && next.kind
                    != PlannedMutationKind::InPlace(MutationKind::ContractExternalPreserve))
        {
            return Err(store_error(format!(
                "mutation progress for `{}` regressed or skipped a durable intent",
                prior.id
            )));
        }
    }
    Ok(())
}

fn mutation_status_update_is_legal(old: MutationStatus, new: MutationStatus) -> bool {
    old == new
        || matches!(
            (old, new),
            (
                MutationStatus::Planned,
                MutationStatus::ApplyIntent | MutationStatus::NoMutation
            ) | (MutationStatus::ApplyIntent, MutationStatus::Applied)
                | (MutationStatus::Applied, MutationStatus::RollbackIntent)
                | (MutationStatus::RollbackIntent, MutationStatus::RolledBack)
        )
}

fn validate_cleanup_wal_update(old: &Journal, new: &Journal) -> Result<(), TransactionError> {
    match (&old.cleanup_wal, &new.cleanup_wal) {
        (None, None) => Ok(()),
        (None, Some(wal)) => {
            if wal.completed.is_empty() && wal.active.is_none() {
                Ok(())
            } else {
                Err(store_error(
                    "cleanup WAL did not start at an empty durable prefix",
                ))
            }
        }
        (Some(old_wal), Some(new_wal)) => {
            if old_wal.name != new_wal.name
                || old_wal.directory_identity != new_wal.directory_identity
                || old_wal.manifest_digest != new_wal.manifest_digest
            {
                return Err(store_error("cleanup WAL changed its sealed tree binding"));
            }
            match (&old_wal.active, &new_wal.active) {
                (None, None) if old_wal.completed == new_wal.completed => Ok(()),
                (None, Some(_)) if old_wal.completed == new_wal.completed => Ok(()),
                (Some(old_active), Some(new_active))
                    if old_active == new_active && old_wal.completed == new_wal.completed =>
                {
                    Ok(())
                }
                (Some(active), None)
                    if new_wal.completed.len() == old_wal.completed.len() + 1
                        && new_wal.completed.starts_with(&old_wal.completed)
                        && new_wal.completed.last() == Some(&active.progress_key) =>
                {
                    Ok(())
                }
                _ => Err(store_error(
                    "cleanup WAL regressed, changed intent, or skipped an exact completion",
                )),
            }
        }
        (Some(_), None)
            if old.state == new.state
                && new.owned_tree_seal.is_none()
                && old.owned_tree_seal.is_some() =>
        {
            Ok(())
        }
        (Some(_), None) => Err(store_error(
            "cleanup WAL cleared before its owned-tree seal at the same durable state",
        )),
    }
}

fn report_update_is_legal(
    old: &Option<TransactionReport>,
    new: &Option<TransactionReport>,
) -> bool {
    match (old, new) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(old), Some(new)) if old == new => true,
        (Some(old), Some(new)) => {
            old.project_key == new.project_key
                && old.transaction_id == new.transaction_id
                && old.plan_id == new.plan_id
                && old.mode == new.mode
                && old.outcome == new.outcome
                && old.assurance == new.assurance
                && old.before_tree == new.before_tree
                && old.after_tree == new.after_tree
                && old.snapshots == new.snapshots
                && old.verification == new.verification
                && old.planned_mutations == new.planned_mutations
                && old.actual_mutations == new.actual_mutations
                && is_prefix(&old.events, &new.events)
                && matches!(
                    (old.cleanup, new.cleanup),
                    (Cleanup::Pending, Cleanup::Pending | Cleanup::Complete)
                )
        }
    }
}

fn is_prefix<T: PartialEq>(old: &[T], new: &[T]) -> bool {
    new.starts_with(old)
}

fn validate_snapshot_bounds(records: &[SnapshotRecord]) -> Result<(), TransactionError> {
    let mut total = 0u64;
    for record in records {
        validate_snapshot_record_bound(record)?;
        total = total
            .checked_add(record.bytes)
            .ok_or_else(|| store_error("snapshot byte total overflow"))?;
    }
    if total > MAX_SNAPSHOT_TOTAL_BYTES {
        return Err(store_error(format!(
            "snapshot set exceeds {} byte bound",
            MAX_SNAPSHOT_TOTAL_BYTES
        )));
    }
    Ok(())
}

fn validate_snapshot_record_bound(record: &SnapshotRecord) -> Result<(), TransactionError> {
    validate_relative(&record.name, "snapshot name")?;
    if record.bytes > MAX_SNAPSHOT_BYTES as u64 {
        return Err(store_error(format!(
            "snapshot `{}` exceeds {} byte bound",
            record.name, MAX_SNAPSHOT_BYTES
        )));
    }
    Ok(())
}

fn verify_snapshot_bytes(record: &SnapshotRecord, bytes: &[u8]) -> Result<(), TransactionError> {
    if bytes.len() as u64 != record.bytes {
        return Err(store_error(format!(
            "snapshot `{}` byte count differs from journal",
            record.name
        )));
    }
    let digest = sha256_bytes(bytes);
    if digest != record.sha256.0 {
        return Err(store_error(format!(
            "snapshot `{}` digest differs from journal",
            record.name
        )));
    }
    Ok(())
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(bytes);
    format!("sha256:{:x}", hash.finalize())
}

fn snapshot_directories(files: &BTreeSet<String>) -> BTreeSet<String> {
    let mut directories = BTreeSet::new();
    for file in files {
        let mut prefix = String::new();
        let mut components = file.split('/').peekable();
        while let Some(component) = components.next() {
            if components.peek().is_none() {
                break;
            }
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(component);
            directories.insert(prefix.clone());
        }
    }
    directories
}

fn collect_external_entries(
    directory: &ExternalDirectory,
    prefix: &str,
    output: &mut Vec<(String, EntryStateKind)>,
    depth: usize,
) -> Result<(), TransactionError> {
    if depth > 128 {
        return Err(store_error("external transaction tree exceeds depth bound"));
    }
    for name in directory
        .child_names_bounded(MAX_DIRECTORY_CHILDREN)
        .map_err(|error| store_error(format!("enumerating external transaction tree: {error:#}")))?
    {
        validate_relative(&name, "external transaction component")?;
        if output.len() >= MAX_TREE_ENTRIES {
            return Err(store_error("external transaction tree exceeds entry bound"));
        }
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        let state = directory
            .inspect_child_state(&name)
            .map_err(|error| store_error(format!("inspecting `{path}`: {error:#}")))?
            .ok_or_else(|| store_error(format!("external entry `{path}` vanished")))?;
        output.push((path.clone(), state.kind));
        if state.kind == EntryStateKind::Directory {
            let child = directory
                .open_child(&name)
                .map_err(|error| store_error(format!("opening `{path}`: {error:#}")))?
                .ok_or_else(|| store_error(format!("directory `{path}` vanished")))?;
            collect_external_entries(&child, &path, output, depth + 1)?;
        }
    }
    Ok(())
}

fn observe_safefs_manifest(
    directory: &ExternalDirectory,
) -> Result<SafefsTreeManifest, TransactionError> {
    let mut entries = Vec::new();
    collect_safefs_manifest_entries(directory, "", &mut entries, 0)?;
    entries.sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
    let digest = safefs_manifest_digest(&entries);
    Ok(SafefsTreeManifest { digest, entries })
}

fn validate_workspace_owner(
    directory: &ExternalDirectory,
    owner: &OwnerWire,
    journal: &Journal,
) -> Result<(), TransactionError> {
    let expected = journal
        .verification_workspace
        .as_ref()
        .ok_or_else(|| store_error("transaction journal has no verification-workspace intent"))?;
    let state = directory
        .inspect_child_state(&expected.name)
        .map_err(|error| store_error(format!("inspecting verification workspace: {error:#}")))?
        .ok_or_else(|| store_error("verification workspace is absent"))?;
    if state.kind != EntryStateKind::Directory
        || state.identity.as_str() != owner.workspace_entry_identity
    {
        return Err(store_error(
            "verification workspace entry identity differs from its owner seal",
        ));
    }
    let child = directory
        .open_child(&expected.name)
        .map_err(|error| store_error(format!("opening verification workspace: {error:#}")))?
        .ok_or_else(|| store_error("verification workspace vanished"))?;
    if child.path().display().to_string() != expected.display_root {
        return Err(store_error(
            "verification workspace display root differs from journal intent",
        ));
    }
    let project = Project::open(child.path()).map_err(|error| {
        store_error(format!(
            "opening verification workspace identity: {error:#}"
        ))
    })?;
    if project.identity_token().map_err(|error| {
        store_error(format!(
            "reading verification workspace identity: {error:#}"
        ))
    })? != owner.workspace_project_identity
    {
        return Err(store_error(
            "verification workspace project identity differs from owner seal",
        ));
    }
    Ok(())
}

fn validate_transaction_home_shape(
    directory: &ExternalDirectory,
    journal: &Journal,
    owner_wire: &OwnerWire,
) -> Result<(), TransactionError> {
    let mut owner = false;
    let mut journal_file = false;
    let mut verification = false;
    for name in directory
        .child_names_bounded(MAX_DIRECTORY_CHILDREN)
        .map_err(|error| store_error(format!("enumerating transaction root: {error:#}")))?
    {
        let state = directory
            .inspect_child_state(&name)
            .map_err(|error| {
                store_error(format!(
                    "inspecting transaction-root entry `{name}`: {error:#}"
                ))
            })?
            .ok_or_else(|| store_error(format!("transaction-root entry `{name}` vanished")))?;
        match (name.as_str(), state.kind) {
            (OWNER_FILE, EntryStateKind::File) => owner = true,
            (JOURNAL_FILE, EntryStateKind::File) => journal_file = true,
            (SNAPSHOTS_DIRECTORY, EntryStateKind::Directory) => {}
            (VERIFICATION_DIRECTORY, EntryStateKind::Directory) => {
                verification = true;
            }
            _ => {
                return Err(store_error(format!(
                    "unexpected entry `{name}` in transaction root"
                )));
            }
        }
    }
    if !owner || !journal_file || !verification {
        return Err(store_error(
            "transaction root lacks its exact owner/journal/workspace authority",
        ));
    }
    validate_workspace_owner(directory, owner_wire, journal)?;
    Ok(())
}

fn collect_safefs_manifest_entries(
    directory: &ExternalDirectory,
    prefix: &str,
    output: &mut Vec<SafefsTreeEntry>,
    depth: usize,
) -> Result<(), TransactionError> {
    if depth > 128 {
        return Err(store_error("transaction manifest exceeds depth bound"));
    }
    for name in directory
        .child_names_bounded(MAX_DIRECTORY_CHILDREN)
        .map_err(|error| store_error(format!("enumerating transaction manifest: {error:#}")))?
    {
        validate_relative(&name, "transaction manifest component")?;
        if output.len() >= MAX_TREE_ENTRIES {
            return Err(store_error("transaction manifest exceeds entry bound"));
        }
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        let state = directory
            .inspect_child_state(&name)
            .map_err(|error| store_error(format!("inspecting manifest entry `{path}`: {error:#}")))?
            .ok_or_else(|| store_error(format!("manifest entry `{path}` vanished")))?;
        output.push(SafefsTreeEntry {
            path: path.clone(),
            state: state.clone(),
        });
        if state.kind == EntryStateKind::Directory {
            let child = directory
                .open_child(&name)
                .map_err(|error| {
                    store_error(format!("opening manifest directory `{path}`: {error:#}"))
                })?
                .ok_or_else(|| store_error(format!("manifest directory `{path}` vanished")))?;
            collect_safefs_manifest_entries(&child, &path, output, depth + 1)?;
        }
    }
    Ok(())
}

fn safefs_manifest_digest(entries: &[SafefsTreeEntry]) -> String {
    let mut hash = Sha256::new();
    hash.update(b"vibe-safefs-tree-manifest-e1\0");
    for entry in entries {
        hash.update(entry.path.as_bytes());
        hash.update(b"\0");
        hash.update(match entry.state.kind {
            EntryStateKind::File => b"file\0".as_slice(),
            EntryStateKind::Directory => b"directory\0".as_slice(),
        });
        hash.update(entry.state.identity.as_str().as_bytes());
        hash.update(b"\0");
        if let Some(digest) = &entry.state.sha256 {
            hash.update(digest.as_bytes());
        }
        hash.update(b"\0");
        if let Some(bytes) = entry.state.bytes {
            hash.update(bytes.to_be_bytes());
        }
        hash.update(b"\0");
        if let Some(mode) = entry.state.unix_mode {
            hash.update(mode.to_be_bytes());
        }
        hash.update(b"\n");
    }
    format!("sha256:{:x}", hash.finalize())
}

fn validate_safefs_manifest(manifest: &SafefsTreeManifest) -> Result<(), TransactionError> {
    let mut previous: Option<&str> = None;
    for entry in &manifest.entries {
        validate_relative(&entry.path, "retirement manifest path")?;
        if previous.is_some_and(|value| value.as_bytes() >= entry.path.as_bytes()) {
            return Err(store_error(
                "retirement manifest paths are not unique and byte-sorted",
            ));
        }
        previous = Some(&entry.path);
        match entry.state.kind {
            EntryStateKind::File if entry.state.sha256.is_none() || entry.state.bytes.is_none() => {
                return Err(store_error("retirement manifest file lacks content state"));
            }
            EntryStateKind::Directory
                if entry.state.sha256.is_some() || entry.state.bytes.is_some() =>
            {
                return Err(store_error(
                    "retirement manifest directory carries file content state",
                ));
            }
            _ => {}
        }
    }
    if safefs_manifest_digest(&manifest.entries) != manifest.digest {
        return Err(store_error("retirement manifest digest mismatch"));
    }
    Ok(())
}

fn validate_retirement_identity(
    wire: &RetirementWire,
    project: &ProjectKey,
    transaction: &TransactionId,
) -> Result<(), TransactionError> {
    if wire.schema != 2
        || wire.project_key != project.0
        || wire.transaction_id != transaction.0
        || !wire
            .stable_report_sha256
            .strip_prefix("sha256:")
            .is_some_and(valid_lower_hex_digest)
        || wire.ownership_token != transaction_ownership_token(project, transaction)
    {
        return Err(store_error(
            "retirement checkpoint identity differs from selected transaction",
        ));
    }
    OwnedDirectoryIdentity::from_token(&wire.directory_identity).map_err(|error| {
        store_error(format!("invalid retirement directory identity: {error:#}"))
    })?;
    let manifest: SafefsTreeManifest = wire.manifest.clone().try_into()?;
    validate_safefs_manifest(&manifest)?;
    let order = safefs_cleanup_order(&manifest);
    if wire.completed.len() > order.len()
        || wire
            .completed
            .iter()
            .zip(&order)
            .any(|(actual, expected)| actual != expected)
    {
        return Err(store_error(
            "retirement completion list is not the canonical manifest prefix",
        ));
    }
    if wire.tree_removed {
        if wire.active.is_some() || wire.completed != order {
            return Err(store_error(
                "removed retirement tree has incomplete or active progress",
            ));
        }
        return Ok(());
    }
    if let Some(active) = &wire.active {
        if wire.completed.len() >= order.len() || active.progress_key != order[wire.completed.len()]
        {
            return Err(store_error(
                "retirement active intent is not the exact next manifest entry",
            ));
        }
        if active.root {
            if active.progress_key != "root"
                || active.path != transaction.0
                || !matches!(active.expected.kind, SafefsEntryKindWire::Directory)
            {
                return Err(store_error("retirement root intent has invalid shape"));
            }
        } else {
            let expected = manifest
                .entries
                .iter()
                .find(|entry| safefs_entry_key(entry) == active.progress_key)
                .ok_or_else(|| store_error("retirement active entry is absent from manifest"))?;
            if active.path != expected.path {
                return Err(store_error(
                    "retirement active path differs from manifest entry",
                ));
            }
        }
    }
    Ok(())
}

fn safefs_entry_key(entry: &SafefsTreeEntry) -> String {
    format!(
        "{}:{}",
        match entry.state.kind {
            EntryStateKind::File => "file",
            EntryStateKind::Directory => "directory",
        },
        entry.path
    )
}

fn safefs_cleanup_order(manifest: &SafefsTreeManifest) -> Vec<String> {
    let mut files = manifest
        .entries
        .iter()
        .filter(|entry| entry.state.kind == EntryStateKind::File)
        .map(safefs_entry_key)
        .collect::<Vec<_>>();
    files.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    let mut directories = manifest
        .entries
        .iter()
        .filter(|entry| entry.state.kind == EntryStateKind::Directory)
        .collect::<Vec<_>>();
    directories.sort_by(|left, right| {
        left.path
            .split('/')
            .count()
            .cmp(&right.path.split('/').count())
            .reverse()
            .then_with(|| right.path.as_bytes().cmp(left.path.as_bytes()))
    });
    files.extend(directories.into_iter().map(safefs_entry_key));
    files.push("root".to_owned());
    files
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    #[cfg(windows)]
    struct Fixture {
        _scope: TempDir,
        project_root: PathBuf,
        store: SystemTransactionStore,
        key: ProjectKey,
        journal: Journal,
        snapshot_bytes: Vec<Vec<u8>>,
    }

    #[cfg(windows)]
    fn fixture() -> Fixture {
        let scope = tempfile::tempdir().unwrap();
        let project_root = scope.path().join("project");
        fs::create_dir(&project_root).unwrap();
        let state_root = scope.path().join("external-state");
        let key = derive_project_key(
            &Project::open(&project_root)
                .unwrap()
                .identity_token()
                .unwrap(),
        );
        let plan_id = Digest(format!("sha256:{}", "2".repeat(64)));
        let canonical_plan = canonical_plan_bytes(&project_root, &plan_id);
        let snapshot_bytes = vec![
            b"contract".to_vec(),
            b"canonical-contract".to_vec(),
            canonical_plan.clone(),
        ];
        let snapshots = [
            (SnapshotKind::Contract, "contract"),
            (SnapshotKind::CanonicalContract, "canonical-contract"),
            (SnapshotKind::CanonicalPlan, "canonical-plan"),
        ]
        .into_iter()
        .zip(&snapshot_bytes)
        .map(|((kind, name), bytes)| SnapshotRecord {
            kind,
            name: name.to_owned(),
            sha256: sha256(bytes),
            bytes: bytes.len() as u64,
            mode: None,
        })
        .collect();
        let tree = TreeManifest {
            digest: Digest(format!("sha256:{}", "0".repeat(64))),
            entries: Vec::new(),
        };
        let mut journal = Journal {
            schema: 1,
            revision: 0,
            project_key: key.clone(),
            transaction_id: TransactionId("TX000001".to_owned()),
            mode: TransactionMode::Export,
            plan_id,
            project_display_root: project_root.display().to_string(),
            canonical_plan,
            verification_workspace: None,
            execution: PreparedMode::Export(Box::new(ExportPlan {
                output_identity: "output-identity".to_owned(),
                output_parent_identity: "output-parent-identity".to_owned(),
                output_display_path: project_root.join("output").display().to_string(),
                output_name: "output".to_owned(),
                before_same_display_path: false,
                after_same_display_path: false,
                entries: Vec::new(),
                source_tree: tree.clone(),
                final_manifest: tree,
            })),
            state: TransactionState::Preparing,
            snapshots,
            snapshots_persisted: 0,
            snapshot_active: None,
            candidate_name: None,
            quarantine_name: None,
            owned_tree_token: None,
            owned_tree_seal: None,
            cleanup_wal: None,
            completed_steps: 0,
            active_step: None,
            mutation_progress: vec![
                MutationProgress {
                    id: "export/candidate".to_owned(),
                    kind: PlannedMutationKind::ExportCandidateCreate,
                    status: MutationStatus::Planned,
                },
                MutationProgress {
                    id: "export/publish".to_owned(),
                    kind: PlannedMutationKind::ExportPublish,
                    status: MutationStatus::Planned,
                },
            ],
            actual_mutations: Vec::new(),
            settlement_intent: None,
            delivered_tree: None,
            verification: Vec::new(),
            events: Vec::new(),
            report: None,
        };
        let mut store = SystemTransactionStore::new(state_root).unwrap();
        store
            .prove_outside_project(&journal.project_display_root)
            .unwrap();
        store.lock_project(&key).unwrap();
        journal.verification_workspace =
            Some(store.verification_workspace_intent(&journal).unwrap());
        store.create_transaction(&journal).unwrap();
        Fixture {
            _scope: scope,
            project_root,
            store,
            key,
            journal,
            snapshot_bytes,
        }
    }

    #[cfg(windows)]
    fn canonical_plan_bytes(project_root: &Path, plan_id: &Digest) -> Vec<u8> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../formats/corpora/scrape/e1/valid/plan-minimal.json");
        let mut plan: ScrapePlanWire = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        plan.plan_id = plan_id.0.clone();
        plan.project.display_root = project_root.display().to_string();
        plan.mode = vibe_wire::generated::scrape::e1::plan::Mode::Export;
        serde_json::to_vec(&plan).unwrap()
    }

    #[cfg(windows)]
    fn persist_all_snapshots(fixture: &mut Fixture) {
        for index in 0..fixture.snapshot_bytes.len() {
            fixture.journal.snapshot_active = Some(index);
            fixture.journal.revision += 1;
            fixture.store.persist_journal(&fixture.journal).unwrap();
            fixture
                .store
                .persist_snapshot(
                    &fixture.journal.transaction_id,
                    &fixture.journal.snapshots[index],
                    &fixture.snapshot_bytes[index],
                )
                .unwrap();
            fixture.journal.snapshots_persisted = index + 1;
            fixture.journal.snapshot_active = None;
            fixture.journal.revision += 1;
            fixture.store.persist_journal(&fixture.journal).unwrap();
        }
        fixture.journal.state = TransactionState::Prepared;
        fixture.journal.revision += 1;
        fixture.store.persist_journal(&fixture.journal).unwrap();
    }

    #[cfg(windows)]
    fn refused_report(journal: &Journal) -> TransactionReport {
        TransactionReport {
            project_key: journal.project_key.clone(),
            transaction_id: journal.transaction_id.clone(),
            plan_id: journal.plan_id.clone(),
            mode: journal.mode,
            outcome: Outcome::Refused,
            assurance: Assurance::Full,
            cleanup: Cleanup::Complete,
            before_tree: Some(match &journal.execution {
                PreparedMode::Export(plan) => plan.source_tree.digest.clone(),
                PreparedMode::InPlace(plan) => plan.before_tree.digest.clone(),
            }),
            after_tree: None,
            snapshots: journal.snapshots.clone(),
            verification: journal.verification.clone(),
            planned_mutations: journal
                .mutation_progress
                .iter()
                .map(|progress| PlannedMutationEvidence {
                    id: progress.id.clone(),
                    kind: progress.kind,
                })
                .collect(),
            actual_mutations: journal.actual_mutations.clone(),
            events: Vec::new(),
        }
    }

    #[cfg(windows)]
    fn make_terminal(fixture: &mut Fixture) {
        persist_all_snapshots(fixture);
        let report = refused_report(&fixture.journal);
        fixture.journal.state = TransactionState::Complete;
        fixture.journal.settlement_intent = Some(Outcome::Refused);
        fixture.journal.report = Some(report);
        fixture.journal.revision += 1;
        fixture.store.persist_journal(&fixture.journal).unwrap();
        let report = fixture.journal.report.clone().unwrap();
        let canonical = fixture
            .store
            .canonical_report_bytes(&fixture.journal, &report)
            .unwrap();
        fixture.store.persist_report(&report, &canonical).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn durable_snapshots_and_journal_survive_store_restart() {
        let mut fixture = fixture();
        persist_all_snapshots(&mut fixture);
        let root = fixture.store.state_root().to_path_buf();
        drop(fixture.store);

        let mut restarted = SystemTransactionStore::new(root).unwrap();
        restarted
            .prove_outside_project(&fixture.project_root.display().to_string())
            .unwrap();
        restarted.lock_project(&fixture.key).unwrap();
        let recovered = restarted.pending(&fixture.key).unwrap().unwrap();
        assert_eq!(recovered, fixture.journal);
        assert_eq!(
            restarted
                .read_snapshot(&recovered, "canonical-plan")
                .unwrap(),
            fixture.snapshot_bytes[2]
        );
        assert_eq!(
            restarted.verify_snapshot_progress(&recovered).unwrap(),
            SnapshotActiveObservation::None
        );
    }

    #[cfg(windows)]
    #[test]
    fn strict_bounded_journal_corruption_fails_closed() {
        let fixture = fixture();
        let relative = journal_relative(&fixture.key, &fixture.journal.transaction_id).unwrap();
        fixture
            .store
            .write_durable(
                &relative,
                br#"{"schema":1,"unexpected":true}"#,
                "corrupt test journal",
            )
            .unwrap();
        let error = fixture
            .store
            .load_journal(&fixture.key, &fixture.journal.transaction_id);
        assert!(matches!(error, Err(TransactionError::Store(_))));
    }

    #[cfg(windows)]
    #[test]
    fn owner_seal_rejects_semantically_valid_immutable_journal_drift() {
        let fixture = fixture();
        let mut corrupted = fixture.journal.clone();
        let PreparedMode::Export(plan) = &mut corrupted.execution else {
            unreachable!()
        };
        plan.output_identity = "different-but-valid-output-identity".to_owned();
        let bytes = strict_json_bytes(&corrupted, MAX_TRANSACTION_JOURNAL_BYTES, "drifted journal")
            .unwrap();
        let relative = journal_relative(&fixture.key, &fixture.journal.transaction_id).unwrap();
        fixture
            .store
            .write_durable(&relative, &bytes, "drifted test journal")
            .unwrap();

        assert!(matches!(
            fixture
                .store
                .load_journal(&fixture.key, &fixture.journal.transaction_id),
            Err(TransactionError::Store(_))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn embedded_canonical_plan_is_strict_utf8_not_an_unbounded_byte_array() {
        let fixture = fixture();
        let bytes = strict_json_bytes(
            &fixture.journal,
            MAX_TRANSACTION_JOURNAL_BYTES,
            "test journal",
        )
        .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            value["canonical_plan"].as_str().unwrap().as_bytes(),
            fixture.journal.canonical_plan
        );
        assert_eq!(
            strict_json_parse::<Journal>(&bytes, "test journal").unwrap(),
            fixture.journal
        );
    }

    #[cfg(windows)]
    #[test]
    fn stable_report_is_a_retryable_one_shot() {
        let mut fixture = fixture();
        make_terminal(&mut fixture);
        let report = fixture.journal.report.clone().unwrap();
        let canonical = fixture
            .store
            .canonical_report_bytes(&fixture.journal, &report)
            .unwrap();
        fixture.store.persist_report(&report, &canonical).unwrap();
        let mut different = report;
        different.events.push("different".to_owned());
        assert!(matches!(
            fixture.store.persist_report(&different, &canonical),
            Err(TransactionError::Store(_))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn caller_supplied_terminal_state_cannot_retire_a_live_transaction() {
        let mut fixture = fixture();
        let mut forged = fixture.journal.clone();
        let report = refused_report(&forged);
        forged.state = TransactionState::Complete;
        forged.settlement_intent = Some(Outcome::Refused);
        forged.report = Some(report);

        assert!(matches!(
            fixture.store.retire_transaction(&forged),
            Err(TransactionError::Store(_))
        ));
        assert_eq!(
            fixture.store.pending(&fixture.key).unwrap(),
            Some(fixture.journal.clone())
        );
    }

    #[cfg(windows)]
    #[test]
    fn journal_revision_rejects_stale_same_revision_and_skip_ahead_writes() {
        let mut fixture = fixture();
        let initial = fixture.journal.clone();

        let mut same_revision_change = initial.clone();
        same_revision_change.events.push("stale change".to_owned());
        assert!(matches!(
            fixture.store.persist_journal(&same_revision_change),
            Err(TransactionError::Store(_))
        ));

        let mut skip_ahead = initial.clone();
        skip_ahead.revision = 2;
        skip_ahead.events.push("skipped revision".to_owned());
        assert!(matches!(
            fixture.store.persist_journal(&skip_ahead),
            Err(TransactionError::Store(_))
        ));

        fixture.journal.snapshot_active = Some(0);
        fixture.journal.revision = 1;
        fixture.store.persist_journal(&fixture.journal).unwrap();
        assert!(matches!(
            fixture.store.persist_journal(&initial),
            Err(TransactionError::Store(_))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn snapshot_prefix_cannot_advance_without_intent_and_exact_data() {
        let mut fixture = fixture();

        let mut skipped_intent = fixture.journal.clone();
        skipped_intent.revision = 1;
        skipped_intent.snapshots_persisted = 1;
        assert!(matches!(
            fixture.store.persist_journal(&skipped_intent),
            Err(TransactionError::Store(_))
        ));

        fixture.journal.revision = 1;
        fixture.journal.snapshot_active = Some(0);
        fixture.store.persist_journal(&fixture.journal).unwrap();
        fixture.journal.revision = 2;
        fixture.journal.snapshot_active = None;
        fixture.journal.snapshots_persisted = 1;
        assert!(matches!(
            fixture.store.persist_journal(&fixture.journal),
            Err(TransactionError::Store(_))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn retirement_refuses_to_adopt_an_unjournaled_transaction_root_entry() {
        let mut fixture = fixture();
        make_terminal(&mut fixture);
        let foreign_relative = format!(
            "{}/{}/{}/foreign.bin",
            TRANSACTIONS_DIRECTORY,
            project_component(&fixture.key).unwrap(),
            fixture.journal.transaction_id.0
        );
        fixture
            .store
            .write_durable(&foreign_relative, b"foreign", "foreign test entry")
            .unwrap();

        assert!(matches!(
            fixture.store.retire_transaction(&fixture.journal),
            Err(TransactionError::Store(_))
        ));
        assert_eq!(
            fs::read(fixture.store.state_root().join(foreign_relative)).unwrap(),
            b"foreign"
        );
        assert!(
            fixture
                .store
                .transaction_directory(&fixture.key, &fixture.journal.transaction_id)
                .unwrap()
                .is_some()
        );
    }

    #[cfg(windows)]
    #[test]
    fn incomplete_transaction_bootstrap_is_left_for_explicit_manual_recovery() {
        let mut fixture = fixture();
        let journal = fixture
            .store
            .state_root()
            .join(journal_relative(&fixture.key, &fixture.journal.transaction_id).unwrap());
        fs::remove_file(journal).unwrap();

        assert!(matches!(
            fixture.store.pending(&fixture.key),
            Err(TransactionError::Store(_))
        ));
        assert!(
            fixture
                .store
                .transaction_directory(&fixture.key, &fixture.journal.transaction_id)
                .unwrap()
                .is_some()
        );
    }

    #[cfg(windows)]
    #[test]
    fn retirement_recovers_after_one_uncheckpointed_removal() {
        let mut fixture = fixture();
        make_terminal(&mut fixture);
        let (owner, directory) = fixture
            .store
            .load_owner(&fixture.key, &fixture.journal.transaction_id)
            .unwrap();
        let manifest = observe_safefs_manifest(&directory).unwrap();
        drop(directory);
        let mut wire = RetirementWire {
            schema: 2,
            project_key: fixture.key.0.clone(),
            transaction_id: fixture.journal.transaction_id.0.clone(),
            stable_report_sha256: sha256_bytes(
                &fixture
                    .store
                    .require_stable_complete_report(&fixture.journal)
                    .unwrap(),
            ),
            ownership_token: owner.ownership_token,
            directory_identity: owner.directory_identity,
            manifest: SafefsManifestWire::from(&manifest),
            completed: Vec::new(),
            active: None,
            tree_removed: false,
        };
        let project_home = fixture
            .store
            .open_project_home(&fixture.key)
            .unwrap()
            .unwrap();
        let identity = OwnedDirectoryIdentity::from_token(&wire.directory_identity).unwrap();
        let progress = OwnedTreeCleanupProgress::new();
        let CleanupPreparation::Intent(intent) = project_home
            .prepare_owned_child_retirement(
                &fixture.journal.transaction_id.0,
                &wire.ownership_token,
                &identity,
                &manifest,
                &progress,
            )
            .unwrap()
        else {
            panic!("nonempty transaction must have a retirement intent")
        };
        wire.active = Some(CleanupIntentWire::from(&intent));
        fixture
            .store
            .write_retirement(&fixture.key, &fixture.journal.transaction_id, &wire)
            .unwrap();
        project_home
            .execute_owned_child_retirement(
                &fixture.journal.transaction_id.0,
                &wire.ownership_token,
                &identity,
                &manifest,
                &progress,
                &intent,
            )
            .unwrap();
        let root = fixture.store.state_root().to_path_buf();
        drop(fixture.store);

        let mut restarted = SystemTransactionStore::new(root).unwrap();
        restarted
            .prove_outside_project(&fixture.project_root.display().to_string())
            .unwrap();
        restarted.lock_project(&fixture.key).unwrap();
        assert!(restarted.pending(&fixture.key).unwrap().is_none());
    }

    #[cfg(windows)]
    fn sha256(bytes: &[u8]) -> Digest {
        let mut hash = Sha256::new();
        hash.update(bytes);
        Digest(format!("sha256:{:x}", hash.finalize()))
    }

    #[test]
    fn state_root_must_be_absolute() {
        assert!(matches!(
            SystemTransactionStore::new("relative"),
            Err(TransactionError::Store(_))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn oversized_initial_canonical_or_execution_bytes_create_no_transaction_home() {
        let template = fixture();
        let scope = tempfile::tempdir().unwrap();
        let state_root = scope.path().join("state");
        let mut store = SystemTransactionStore::new(&state_root).unwrap();
        store
            .prove_outside_project(&template.project_root.display().to_string())
            .unwrap();
        store.lock_project(&template.key).unwrap();
        let project_home = state_root
            .join(TRANSACTIONS_DIRECTORY)
            .join(project_component(&template.key).unwrap());

        let mut canonical = template.journal.clone();
        canonical.transaction_id = TransactionId("TXOVERSIZECANONICAL".into());
        canonical.canonical_plan = vec![b'x'; 16 * 1024 * 1024 + 1];
        canonical.verification_workspace =
            Some(store.verification_workspace_intent(&canonical).unwrap());
        assert!(store.create_transaction(&canonical).is_err());
        assert!(!project_home.exists());

        let mut execution = template.journal.clone();
        execution.transaction_id = TransactionId("TXOVERSIZEEXECUTION".into());
        let PreparedMode::Export(plan) = &mut execution.execution else {
            unreachable!()
        };
        // JSON escapes every quote, pushing the exact revision-zero envelope
        // over 64 MiB without allocating a second 64 MiB source string.
        plan.output_display_path = "\"".repeat(MAX_TRANSACTION_JOURNAL_BYTES / 2 + 1);
        execution.verification_workspace =
            Some(store.verification_workspace_intent(&execution).unwrap());
        assert!(store.create_transaction(&execution).is_err());
        assert!(!project_home.exists());
    }

    #[test]
    fn lock_key_must_identify_the_project_used_for_disjointness_proof() {
        let scope = tempfile::tempdir().unwrap();
        let project_root = scope.path().join("project");
        fs::create_dir(&project_root).unwrap();
        let state_root = scope.path().join("state");
        fs::create_dir(&state_root).unwrap();
        let mut store = SystemTransactionStore::new(state_root).unwrap();
        store
            .prove_outside_project(&project_root.display().to_string())
            .unwrap();
        let wrong = ProjectKey(format!("sha256:{}", "f".repeat(64)));
        assert!(matches!(
            store.lock_project(&wrong),
            Err(TransactionError::Store(_))
        ));
        let exact = derive_project_key(
            &Project::open(&project_root)
                .unwrap()
                .identity_token()
                .unwrap(),
        );
        store.lock_project(&exact).unwrap();
    }

    #[test]
    fn strict_enum_payloads_reject_unknown_and_duplicate_members() {
        let unknown = serde_json::from_slice::<ExportPayload>(
            br#"{"prepared-after":{"snapshot_name":"after","unexpected":true}}"#,
        );
        assert!(unknown.is_err());
        let duplicate = serde_json::from_slice::<ExportPayload>(
            br#"{"prepared-after":{"snapshot_name":"first","snapshot_name":"second"}}"#,
        );
        assert!(duplicate.is_err());
    }

    #[test]
    fn retirement_progress_requires_a_durable_parent_sync() {
        assert!(matches!(
            require_namespace_checkpoint(
                DirectoryDurability::Unsupported(std::io::ErrorKind::PermissionDenied),
                "retirement step",
            ),
            Err(TransactionError::Store(_))
        ));
    }

    #[test]
    fn stable_report_updates_are_only_append_only_pending_cleanup_transitions() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../formats/corpora/scrape/e1/valid/report-minimal.json");
        let mut pending: report_wire::Report =
            serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        pending.cleanup = report_wire::ReportCleanup::Pending;

        let mut appended = pending.clone();
        appended.recovery.push(report_wire::RecoveryStep {
            action: "cleanup".to_owned(),
            operation_id: "in-place/quarantine".to_owned(),
            result: report_wire::RecoveryStepResult::Complete,
            sequence: 0,
        });
        assert!(canonical_report_update_is_legal(&pending, &appended));

        let mut complete = appended.clone();
        complete.cleanup = report_wire::ReportCleanup::Complete;
        assert!(canonical_report_update_is_legal(&appended, &complete));

        let mut rewritten = appended.clone();
        rewritten.plan_id = format!("sha256:{}", "f".repeat(64));
        assert!(!canonical_report_update_is_legal(&pending, &rewritten));
        assert!(!canonical_report_update_is_legal(&complete, &pending));
    }
}
