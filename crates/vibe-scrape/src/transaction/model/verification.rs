use super::{Assurance, Digest, MutationKind, TreeManifest};

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
