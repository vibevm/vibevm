//! Durable transaction values independent of the generated report wire.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-E");

use std::fmt;

use sha2::{Digest as _, Sha256};

pub(crate) const MAX_TRANSACTION_JOURNAL_BYTES: usize = 64 * 1024 * 1024;
pub(crate) const MAX_CANONICAL_REPORT_BYTES: usize = 64 * 1024 * 1024;

/// A lowercase, domain-separated SHA-256 identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Digest(pub String);

/// Stable key for the pinned project identity. Display paths are deliberately
/// absent, so aliases of the same pinned root cannot acquire different locks.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProjectKey(pub String);

/// A store-issued, create-new transaction identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransactionId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionMode {
    Export,
    InPlace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractBoundaryAction {
    DeleteLastMoved,
    ExternalPreserved,
}

/// The only durable state vocabulary admitted by PROP-056 D/E.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionState {
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

impl TransactionState {
    #[must_use]
    pub const fn is_pre_verified(&self) -> bool {
        matches!(
            self,
            Self::Preparing
                | Self::Prepared
                | Self::BeforePassed
                | Self::Candidate
                | Self::PublishedPendingVerify
                | Self::Mutating
                | Self::ContractBoundary(_)
                | Self::RollingBack
        )
    }

    #[must_use]
    pub const fn rolls_forward(&self) -> bool {
        matches!(self, Self::Verified | Self::CleanupPending)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SnapshotKind {
    Contract,
    CanonicalContract,
    CanonicalPlan,
    Verifier,
    PreparedAfter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub kind: SnapshotKind,
    pub name: String,
    pub bytes: Vec<u8>,
    pub mode: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotRecord {
    pub kind: SnapshotKind,
    pub name: String,
    pub sha256: Digest,
    pub bytes: u64,
    pub mode: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeEntryKind {
    File,
    Directory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeEntry {
    pub path: String,
    pub kind: TreeEntryKind,
    pub sha256: Option<Digest>,
    pub bytes: Option<u64>,
    pub mode: Option<u32>,
}

/// Complete, canonical, no-follow descendant manifest. It never represents a
/// best-effort or truncated walk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeManifest {
    pub digest: Digest,
    pub entries: Vec<TreeEntry>,
}

/// Canonical logical tree projection shared by preparation, safefs
/// observation, recovery prefixes and validation. Identity-bearing safefs
/// manifests deliberately map into this product-state digest rather than
/// inventing an adapter-local hash family.
pub fn logical_tree_manifest(mut entries: Vec<TreeEntry>) -> TreeManifest {
    entries.sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
    let mut hash = Sha256::new();
    for entry in &entries {
        hash.update(match entry.kind {
            TreeEntryKind::File => b"f\0".as_slice(),
            TreeEntryKind::Directory => b"d\0".as_slice(),
        });
        hash.update(entry.path.as_bytes());
        hash.update(b"\0");
        if let Some(digest) = &entry.sha256 {
            hash.update(digest.0.as_bytes());
        }
        hash.update(b"\0");
        if let Some(bytes) = entry.bytes {
            hash.update(bytes.to_be_bytes());
        }
        hash.update(b"\0");
        if let Some(mode) = entry.mode {
            hash.update(mode.to_be_bytes());
        }
        hash.update(b"\n");
    }
    TreeManifest {
        digest: Digest(format!("sha256:{:x}", hash.finalize())),
        entries,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportPayload {
    /// Copy the held source file only while its sealed state still matches.
    Source {
        source_path: String,
        before: FileState,
    },
    /// Publish bytes from a journaled prepared-after snapshot.
    PreparedAfter { snapshot_name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportEntry {
    pub target_path: String,
    pub kind: TreeEntryKind,
    pub mode: Option<u32>,
    pub payload: Option<ExportPayload>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportPlan {
    pub output_identity: String,
    pub output_parent_identity: String,
    pub output_display_path: String,
    pub output_name: String,
    /// Whether the sealed health plan contains a path-sensitive check that
    /// needs an isolated view mounted at the exact source display path.
    pub before_same_display_path: bool,
    /// The output is always checked using its final display path; this flag is
    /// only for a backend that additionally needs a private same-path COW view.
    pub after_same_display_path: bool,
    pub entries: Vec<ExportEntry>,
    pub source_tree: TreeManifest,
    pub final_manifest: TreeManifest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileState {
    pub sha256: Digest,
    pub bytes: u64,
    pub mode: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathState {
    Absent,
    File(FileState),
    EmptyDirectory {
        mode: Option<u32>,
    },
    /// Complete root-relative subtree state for one exact directory
    /// relocation. The root directory itself is implicit.
    Tree(SubtreeState),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtreeState {
    pub digest: Digest,
    pub root_mode: Option<u32>,
    pub descendants: Vec<SubtreeEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtreeEntry {
    pub relative_path: String,
    pub kind: TreeEntryKind,
    pub sha256: Option<Digest>,
    pub bytes: Option<u64>,
    pub mode: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Location {
    Project,
    Quarantine,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathTransition {
    pub location: Location,
    pub path: String,
    pub before: PathState,
    pub after: PathState,
}

/// A fully prepared atomic mutation. Implementations may not expand this into
/// an ambient recursive operation. Every affected name is represented by a
/// before/after transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutationStep {
    pub id: String,
    /// Shared by the adjacent capture/rewrite pair; absent for every other
    /// operation kind.
    pub pair_id: Option<String>,
    pub kind: MutationKind,
    pub transitions: Vec<PathTransition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationKind {
    CaptureBeforeImage,
    AtomicRewrite,
    CreateRelocationParent,
    Relocate,
    QuarantineFile,
    PruneEmptyDirectory,
    ContractDeleteLast,
    ContractAncestorTreePark,
    ContractExternalPreserve,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InPlacePlan {
    pub quarantine_parent_identity: String,
    pub before_same_display_path: bool,
    pub after_same_display_path: bool,
    pub steps: Vec<MutationStep>,
    pub contract: ContractCommit,
    pub contract_step: MutationStep,
    pub contract_cleanup_step: Option<MutationStep>,
    pub before_tree: TreeManifest,
    pub pre_contract_tree: TreeManifest,
    pub post_contract_tree: TreeManifest,
    pub after_tree: TreeManifest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractCommit {
    DeleteLast {
        path: String,
        empty_ancestors: Vec<String>,
    },
    ExternalPreserve,
}

/// Transaction-ready projection of the current planning/rewrite/health types.
/// The later integration adapter must construct it solely from the already
/// prepared `crate::model::PreparedScrape` and prepared health value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTransaction {
    pub project_identity_token: String,
    pub project_display_root: String,
    pub plan_id: Digest,
    /// Bounded canonical generated `scrape_plan/e1` bytes embedded in the
    /// discoverable journal before snapshot zero, so even an early refusal
    /// can produce its canonical report without consulting project state.
    pub canonical_plan: Vec<u8>,
    pub snapshots: Vec<Snapshot>,
    pub mode: PreparedMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreparedMode {
    Export(Box<ExportPlan>),
    InPlace(Box<InPlacePlan>),
}

impl PreparedTransaction {
    #[must_use]
    pub const fn mode(&self) -> TransactionMode {
        match self.mode {
            PreparedMode::Export(_) => TransactionMode::Export,
            PreparedMode::InPlace(_) => TransactionMode::InPlace,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Journal {
    pub schema: u32,
    /// Monotonic compare-and-swap generation for the external journal.  The
    /// create-new preparation record starts at zero; every changed durable
    /// checkpoint advances exactly once.
    pub revision: u64,
    pub project_key: ProjectKey,
    pub transaction_id: TransactionId,
    pub mode: TransactionMode,
    pub plan_id: Digest,
    pub canonical_plan: Vec<u8>,
    /// Store-created, identity-bound verifier workspace below this exact
    /// transaction home. `None` exists only before `create_transaction`
    /// publishes revision zero.
    pub verification_workspace: Option<VerificationWorkspaceIntent>,
    pub project_display_root: String,
    /// Exact executable plan projection. A durable store serializes this
    /// together with the snapshot records; recovery never recreates it from a
    /// source contract or a freshly observed plan.
    pub execution: PreparedMode,
    pub state: TransactionState,
    /// Complete expected set, present before the first snapshot write.
    pub snapshots: Vec<SnapshotRecord>,
    /// Durable prefix of `snapshots`. A crash may only leave this value at or
    /// before the first not-yet-published snapshot.
    pub snapshots_persisted: usize,
    /// Snapshot whose write intent is durable while its data may be either
    /// absent or exact-present. It is always the next prefix index.
    pub snapshot_active: Option<usize>,
    pub candidate_name: Option<String>,
    pub quarantine_name: Option<String>,
    pub owned_tree_token: Option<String>,
    pub owned_tree_seal: Option<OwnedTreeSeal>,
    /// Manifest-bound, entry-at-a-time removal progress.  This remains
    /// durable while a candidate/quarantine is being retired so a crash after
    /// one removal syscall never turns absence into an unproved success.
    pub cleanup_wal: Option<OwnedTreeCleanupWal>,
    /// Number of fully checkpointed mutations.
    pub completed_steps: usize,
    /// A step whose intent was durable before it was attempted. Recovery must
    /// accept either its sealed before or after state.
    pub active_step: Option<usize>,
    pub mutation_progress: Vec<MutationProgress>,
    pub actual_mutations: Vec<ActualMutationEvidence>,
    /// Durable terminal direction selected before refusal/rollback cleanup.
    pub settlement_intent: Option<Outcome>,
    /// Digest obtained from the distinct real-tree reproof, never a planned
    /// projection substituted as observation.
    pub delivered_tree: Option<Digest>,
    pub verification: Vec<VerificationRecord>,
    pub events: Vec<String>,
    pub report: Option<TransactionReport>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedTreeSeal {
    pub directory_identity: String,
    pub manifest_digest: String,
    pub entries: Vec<OwnedEntrySeal>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedEntrySeal {
    pub path: String,
    pub kind: TreeEntryKind,
    pub sha256: Option<Digest>,
    pub bytes: Option<u64>,
    pub mode: Option<u32>,
    pub identity: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedTreeCleanupWal {
    pub name: String,
    pub directory_identity: String,
    pub manifest_digest: String,
    /// Canonical safefs cleanup-order prefix.
    pub completed: Vec<String>,
    /// The sole syscall permitted before the next durable checkpoint.
    pub active: Option<OwnedTreeCleanupIntent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedTreeCleanupIntent {
    pub intent_token: String,
    pub progress_key: String,
    pub path: String,
    pub expected: OwnedEntrySeal,
    pub root: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnedTreeCleanupPreparation {
    Intent(OwnedTreeCleanupIntent),
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedTreeCleanupCompletion {
    pub progress_key: String,
    pub recovered_after_syscall: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assurance {
    Full,
    Reduced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cleanup {
    Complete,
    Pending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Verified,
    Refused,
    RolledBack,
    RollbackFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionReport {
    pub project_key: ProjectKey,
    pub transaction_id: TransactionId,
    pub plan_id: Digest,
    pub mode: TransactionMode,
    pub outcome: Outcome,
    pub assurance: Assurance,
    pub cleanup: Cleanup,
    pub before_tree: Option<Digest>,
    pub after_tree: Option<Digest>,
    pub snapshots: Vec<SnapshotRecord>,
    pub verification: Vec<VerificationRecord>,
    pub planned_mutations: Vec<PlannedMutationEvidence>,
    pub actual_mutations: Vec<ActualMutationEvidence>,
    pub events: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedMutationEvidence {
    pub id: String,
    pub kind: PlannedMutationKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlannedMutationKind {
    ExportCandidateCreate,
    ExportEntry,
    ExportPublish,
    InPlaceQuarantineCreate,
    InPlace(MutationKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationDirection {
    Apply,
    Rollback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationOrigin {
    Execution,
    Recovery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationStatus {
    Planned,
    NoMutation,
    ApplyIntent,
    Applied,
    RollbackIntent,
    RolledBack,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutationProgress {
    pub id: String,
    pub kind: PlannedMutationKind,
    pub status: MutationStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActualMutationEvidence {
    pub id: String,
    pub kind: PlannedMutationKind,
    pub direction: MutationDirection,
    pub origin: MutationOrigin,
    pub status: MutationStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SealedObservation {
    Before,
    After,
    Third { detail: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnedTreeObservation {
    Absent,
    Exact(TreeManifest),
    Third { detail: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotActiveObservation {
    None,
    Absent,
    ExactPresent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnedRootObservation {
    Absent,
    ExactOwned,
    Third { detail: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExclusiveTreeCreation {
    /// The exclusive create did not create anything at the name.
    NotCreated { detail: String },
    /// This invocation created the name but could not re-open/prove it. The
    /// journal remains pending; it is never terminal-refused or adopted.
    CreatedNotReopened { detail: String },
    /// The exact ownership token is durably bound to the created directory.
    Owned,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationEvidence {
    pub accepted: bool,
    pub assurance: Assurance,
    pub summary: String,
    /// Canonical, already bounded/redacted evidence ready for the generated
    /// report mapper. This is never raw child output.
    pub canonical_evidence: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationRecord {
    pub phase: VerificationPhase,
    pub evidence_sha256: Digest,
    pub evidence: VerificationEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VerificationPhase {
    Before,
    PreContractResidual,
    FinalResidual,
    AfterHealth,
    FinalTree,
    SourceUnchanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationRootKind {
    Source,
    ExportFinal,
    InPlaceView,
}

/// Complete input for adapting to `health::run_phase`: the adapter supplies
/// this seal as `PhaseContext::expected_tree` and must refuse when its backend
/// cannot provide the requested private/COW or same-display-path view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationContext<'a> {
    pub phase: VerificationPhase,
    pub root_kind: VerificationRootKind,
    pub root_display: &'a str,
    pub expected_tree: &'a TreeManifest,
    pub same_display_path_required: bool,
    pub contract_exemption: Option<&'a str>,
    pub workspace: &'a VerificationWorkspace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationWorkspace {
    pub intent: VerificationWorkspaceIntent,
    pub directory_identity: String,
    pub entry_identity: String,
    pub project_identity_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationWorkspaceIntent {
    pub name: String,
    pub display_root: String,
    pub ownership_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequiredPrimitive {
    StableProjectIdentityToken,
    ExternalNoFollowStoreAndLock,
    SameVolumeIdentityComparison,
    ExclusivePinnedDirectory,
    CapabilityRelativeRename,
    AtomicNoReplaceDirectoryRename,
    ExactManifestTreeRemoval,
}

impl RequiredPrimitive {
    #[must_use]
    pub const fn required_api(self) -> &'static str {
        match self {
            Self::StableProjectIdentityToken => "vibe_safefs::Project::identity_token()",
            Self::ExternalNoFollowStoreAndLock => {
                "vibe_safefs::ExternalStore::open_and_lock_project(project_key)"
            }
            Self::SameVolumeIdentityComparison => "vibe_safefs::Pinned::same_filesystem(&Pinned)",
            Self::ExclusivePinnedDirectory => {
                "vibe_safefs::Pinned::create_child_exclusive() with durable identity token"
            }
            Self::CapabilityRelativeRename => {
                "vibe_safefs::Pinned::rename_child_to(&Pinned, old, new)"
            }
            Self::AtomicNoReplaceDirectoryRename => {
                "vibe_safefs::Pinned::rename_child_noreplace_to(&Pinned, old, new)"
            }
            Self::ExactManifestTreeRemoval => {
                "vibe_safefs::Pinned::remove_owned_tree_exact(identity, manifest)"
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionError {
    InvalidPrepared(String),
    Store(String),
    Filesystem(String),
    Verification(String),
    MissingPrimitive(RequiredPrimitive),
    AtomicNoReplaceUnsupported,
    OutputRace(String),
    ThirdState(String),
    FaultInjected(DurableBoundary),
    NoPendingTransaction,
}

impl fmt::Display for TransactionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPrepared(message) => write!(f, "invalid prepared scrape: {message}"),
            Self::Store(message) => write!(f, "transaction store failed: {message}"),
            Self::Filesystem(message) => write!(f, "transaction filesystem failed: {message}"),
            Self::Verification(message) => write!(f, "scrape verification failed: {message}"),
            Self::MissingPrimitive(primitive) => write!(
                f,
                "safe transaction primitive is unavailable: {}",
                primitive.clone().required_api()
            ),
            Self::AtomicNoReplaceUnsupported => {
                f.write_str("atomic no-replace directory publication is unsupported")
            }
            Self::OutputRace(message) => write!(f, "export output race: {message}"),
            Self::ThirdState(message) => write!(f, "third state: {message}"),
            Self::FaultInjected(boundary) => write!(f, "fault injected at {boundary:?}"),
            Self::NoPendingTransaction => f.write_str("no pending scrape transaction"),
        }
    }
}

impl std::error::Error for TransactionError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DurableBoundary {
    StoreProvedExternal,
    ProjectLockAcquired,
    TransactionCreated,
    SnapshotPersisted { index: usize },
    SnapshotIntentPersisted { index: usize },
    SnapshotDataPersisted { index: usize },
    JournalPersisted(TransactionState),
    CandidateNamePersisted,
    QuarantineNamePersisted,
    OwnershipPersisted,
    RefusalIntentPersisted,
    StepIntentPersisted { index: usize, id: String },
    StepCompletionPersisted { index: usize, id: String },
    StepRollbackIntentPersisted { index: usize, id: String },
    StepRollbackCompletionPersisted { index: usize, id: String },
    MutationCompleted { label: String },
    OwnedTreeMutationBeforeReseal { label: String },
    PhaseViewProved(VerificationPhase),
    VerificationCompleted(VerificationPhase),
    ReportPersisted(Cleanup),
    CleanupCompleted,
    CleanupStarted,
    CleanupIntentPersisted { progress_key: String },
    CleanupMutationCompleted { progress_key: String },
    CleanupStepCompletionPersisted { progress_key: String },
}
