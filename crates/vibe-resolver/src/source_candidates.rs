//! Retained source identity and original-edge input for the existing engines.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#solver");
use crate::{
    ResolvedGraph,
    source_edges::{OriginalRequirement, RequirementSite},
};
use std::collections::BTreeMap;
use vibe_core::{
    Group,
    manifest::{CanonicalSourceBinding, Manifest, PackageConfiguration, RegistryPin},
};
use vibe_registry::source_candidates::RetainedSourceCandidate;

/// Runtime identity; mirror acquisition URLs never replace canonical binding.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CandidateKey {
    pub group: Group,
    pub name: String,
    pub version: semver::Version,
    pub binding: CanonicalSourceBinding,
}
impl CandidateKey {
    pub fn of(candidate: &RetainedSourceCandidate) -> Self {
        Self {
            group: candidate.group().clone(),
            name: candidate.name().into(),
            version: candidate.version().clone(),
            binding: candidate.binding().clone(),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigurationAdmission {
    Unconfigured,
    WorkspaceInputs,
}

/// Exact acquired metadata and its evaluated dependency view remain separate.
#[derive(Debug, Clone)]
pub struct CandidateMetadata {
    pub configuration_admission: ConfigurationAdmission,
    pub observations: Vec<vibe_registry::source_candidates::SourceCandidateObservation>,
    pub configuration_error: Option<vibe_core::manifest::PackageConfigurationError>,
    pub candidate: RetainedSourceCandidate,
    pub view: Manifest,
    pub original_edges: Vec<OriginalRequirement<RegistryPin>>,
    pub configuration: Option<PackageConfiguration>,
}
impl CandidateMetadata {
    pub fn key(&self) -> CandidateKey {
        CandidateKey::of(&self.candidate)
    }
}
/// No candidate is a root merely because its metadata was discovered.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EdgeOwner {
    Root { consumer: String },
    Candidate(Box<CandidateKey>),
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EdgeId {
    pub owner: EdgeOwner,
    pub site: RequirementSite,
}
#[derive(Debug, Clone)]
pub struct RootRequirement {
    pub id: EdgeId,
    pub requirement: OriginalRequirement<RegistryPin>,
}
/// One original selected alternative, never a reverse synthetic support edge.
#[derive(Debug, Clone)]
pub struct ActiveSourceEdge {
    pub id: EdgeId,
    pub alternative: usize,
    pub target: CandidateKey,
}
/// Operation-local historical conditional activation, retained across passes.
/// Construction belongs to the metadata phase after checking actual selection
/// and the predicate in that phase's activation context; this is not a journal.
#[derive(Debug, Clone)]
pub struct HistoricalActivation {
    pub parent: std::sync::Arc<CandidateMetadata>,
    pub edge: OriginalRequirement<RegistryPin>,
    pub phase: usize,
    pub introduced_targets: std::collections::BTreeSet<(Group, String)>,
}
#[derive(Debug, Clone)]
pub struct SourceResolvedGraph {
    pub graph: ResolvedGraph,
    pub selected: BTreeMap<(Group, String), CandidateMetadata>,
    pub selected_before_projection: BTreeMap<(Group, String), CandidateMetadata>,
    pub active_edges: Vec<ActiveSourceEdge>,
}
/// Explicit bounds apply before allocation and are never reported as UNSAT.
#[derive(Debug, Clone, Copy)]
pub struct SourceSolveLimits {
    pub package_names: usize,
    pub source_candidates: usize,
    pub ranked_candidates: usize,
    pub edge_witnesses: usize,
}
impl SourceSolveLimits {
    /// Named operation bounds; exhaustion is explicit, never unsatisfiability.
    pub const fn standard_operation() -> Self {
        Self {
            package_names: 512,
            source_candidates: 512,
            ranked_candidates: 262_144,
            edge_witnesses: 1_048_576,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceModelResource {
    PackageNames,
    SourceCandidates,
    RankedCandidates,
    EdgeWitnesses,
}

/// Source-aware solves keep ordinary roots distinct from historical extras.
pub struct SourceSolveRequest<'a> {
    pub roots: &'a [RootRequirement],
    pub held_roots: &'a [vibe_core::PackageRef],
    pub authoritative_locked: &'a [vibe_core::manifest::LockedPackage],
    pub activated: &'a [HistoricalActivation],
    pub limits: SourceSolveLimits,
    pub configuration: Option<&'a crate::configuration::ConfigurationSnapshot>,
    pub blocked: &'a std::collections::BTreeSet<(String, String)>,
}

/// Registry/local providers retain discovery observations beside available tokens.
/// Transport selection is still the provider's owner, never the solver's URL logic.
///
/// ```
/// use vibe_core::Group;
/// use vibe_registry::{LocalRegistry, source_candidates::SourceChoiceMode};
/// use vibe_resolver::{LocalRegistryDepProvider, SourceCandidateProvider};
///
/// let dir = tempfile::tempdir().unwrap();
/// let registry = LocalRegistry::new(dir.path()).unwrap();
/// let provider = LocalRegistryDepProvider::new(&registry);
/// // No held lock and no operation policy: the incumbent source choice.
/// let group = Group::parse("org.vibevm").unwrap();
/// assert!(!provider.has_locked_coordinate(&group, "wal"));
/// assert_eq!(provider.source_choice_mode(), SourceChoiceMode::LegacyKnown);
/// ```
pub trait SourceCandidateProvider: crate::VersionEnumerator {
    /// Pure held-authority lookup; composition dispatches it before ambient sources.
    fn has_locked_coordinate(&self, group: &Group, name: &str) -> bool;
    /// Pure operation-policy signal; no catalog or transport demand.
    fn source_choice_mode(&self) -> vibe_registry::source_candidates::SourceChoiceMode;
    /// Retain the same exact-source metadata read the legacy callback would use.
    fn retain_default_candidate(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<
        vibe_registry::source_candidates::RetainedCandidateObservation,
        crate::DepProviderError,
    >;

    /// Admit one authored registry pin before any candidate is captured: a
    /// held lock row is checked against its locked alias, otherwise the pin
    /// must name a declared, enabled registry. Providers without a registry
    /// catalog have nothing to check.
    fn validate_registry_pin(
        &self,
        group: &Group,
        name: &str,
        pin: &RegistryPin,
    ) -> Result<(), crate::DepProviderError> {
        let _ = (group, name, pin);
        Ok(())
    }

    /// Validate under the operation owner's immutable locks and explicit update grant.
    fn respects_locked_selection(
        &self,
        candidate: &RetainedSourceCandidate,
        locked: &vibe_core::manifest::LockedPackage,
    ) -> Result<bool, crate::DepProviderError>;

    fn source_inventory(
        &self,
        group: &Group,
        name: &str,
        scope: vibe_registry::source_candidates::CandidateInventoryScope,
        limits: &vibe_registry::source_candidates::SourceInventoryLimits,
    ) -> Result<vibe_registry::source_candidates::SourceCandidateInventory, crate::DepProviderError>;
}

pub(crate) fn source_registry_error(
    error: vibe_registry::RegistryError,
) -> crate::DepProviderError {
    match error {
        vibe_registry::RegistryError::SourcePolicy(error) => {
            crate::DepProviderError::SourcePolicy(error)
        }
        vibe_registry::RegistryError::UnknownPackage { group, name } => {
            crate::DepProviderError::UnknownPackage { group, name }
        }
        vibe_registry::RegistryError::NoMatchingVersion { group, name, req } => {
            crate::DepProviderError::NoMatchingVersion {
                group,
                name,
                constraint: req,
            }
        }
        vibe_registry::RegistryError::PackageNotFoundEverywhere {
            group,
            name,
            summary,
            attempts,
        } => crate::DepProviderError::AggregateNotFound {
            group,
            name,
            summary,
            attempts,
        },
        error @ vibe_registry::RegistryError::OfflinePackageUnavailable { .. } => {
            crate::DepProviderError::offline_unavailable(error)
        }
        error => crate::DepProviderError::Other(error.to_string()),
    }
}
