//! Immutable metadata and acquisition selections retained by existing source owners.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY");
use crate::{MultiResolution, RegistryError};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use vibe_core::{
    Group,
    manifest::{CanonicalSourceBinding, Manifest},
};
use vibe_wire::generated::shared::SourceEvidence;

/// Metadata selected from one source. Binding/evidence alone cannot reconstruct
/// this token or retarget its private resolution. The owning resolver must live
/// through acquisition; preview drops the token with its private scratch.
#[derive(Clone)]
pub struct RetainedSourceCandidate(Arc<RetainedCandidate>);
struct RetainedCandidate {
    owner: Arc<()>,
    binding: CanonicalSourceBinding,
    resolution: MultiResolution,
    manifest: Manifest,
    metadata_digest: String,
    evidence: SourceEvidence,
}
impl std::fmt::Debug for RetainedSourceCandidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RetainedSourceCandidate")
            .field("binding", self.binding())
            .field("group", self.group())
            .field("name", &self.name())
            .field("version", self.version())
            .finish_non_exhaustive()
    }
}
impl RetainedSourceCandidate {
    pub fn binding(&self) -> &CanonicalSourceBinding {
        &self.0.binding
    }
    pub fn group(&self) -> &Group {
        &self.0.resolution.resolved.group
    }
    pub fn name(&self) -> &str {
        &self.0.resolution.resolved.name
    }
    pub fn version(&self) -> &semver::Version {
        &self.0.resolution.resolved.version
    }
    pub fn manifest(&self) -> &Manifest {
        &self.0.manifest
    }
    /// Semantic digest of the parsed Manifest under metadata_recipe(), not original fetched bytes.
    pub fn metadata_digest(&self) -> &str {
        &self.0.metadata_digest
    }
    pub fn metadata_recipe(&self) -> &'static str {
        "parsed-manifest-toml-v1"
    }
    pub fn source_evidence(&self) -> &SourceEvidence {
        &self.0.evidence
    }

    pub(crate) fn captured(
        owner: &Arc<()>,
        binding: CanonicalSourceBinding,
        resolution: MultiResolution,
        manifest: Manifest,
        evidence: SourceEvidence,
    ) -> Result<Self, RegistryError> {
        if let CanonicalSourceBinding::Registry { registry, .. } = &binding
            && (resolution.registry_name.as_deref() != Some(registry.as_str())
                || resolution.locked.is_some()
                || resolution.direct.is_some()
                || resolution.overridden
                || resolution.is_git_source
                || resolution.is_path_source)
        {
            return Err(RegistryError::MalformedMeta {
                path: resolution.resolved.source_dir.clone(),
                reason: "registry candidate binding contradicts its resolved source kind".into(),
            });
        }
        let package = manifest.require_package()?;
        if package.group != resolution.resolved.group
            || package.name.as_str() != resolution.resolved.name
            || package.version != resolution.resolved.version
        {
            return Err(RegistryError::MalformedMeta {
                path: resolution.resolved.source_dir.clone(),
                reason: "candidate metadata does not match its selected coordinate".into(),
            });
        }
        let encoded = toml::to_string(&manifest).map_err(|error| RegistryError::MalformedMeta {
            path: resolution.resolved.source_dir.clone(),
            reason: error.to_string(),
        })?;
        Ok(Self(Arc::new(RetainedCandidate {
            owner: Arc::clone(owner),
            binding,
            resolution,
            manifest,
            metadata_digest: format!("sha256:{:x}", Sha256::digest(encoded.as_bytes())),
            evidence,
        })))
    }
    pub(crate) fn resolution_for(
        &self,
        owner: &Arc<()>,
    ) -> Result<&MultiResolution, RegistryError> {
        if !Arc::ptr_eq(owner, &self.0.owner) {
            return Err(RegistryError::MalformedMeta {
                path: self.0.resolution.resolved.source_dir.clone(),
                reason: "candidate belongs to another operation source owner".into(),
            });
        }
        Ok(&self.0.resolution)
    }
    pub(crate) fn validate_manifest(&self, manifest: &Manifest) -> Result<(), RegistryError> {
        if manifest != self.manifest() {
            return Err(RegistryError::MalformedMeta {
                path: self.0.resolution.resolved.source_dir.clone(),
                reason: "selected source metadata changed after dependency resolution".into(),
            });
        }
        Ok(())
    }
}

/// Eligibility and ordering facts are independent from candidate identity.
#[derive(Debug, Clone)]
pub struct SourceCandidateRecord {
    pub candidate: RetainedSourceCandidate,
    pub default_eligible: bool,
    pub in_default_version_domain: bool,
    pub legacy_preferred_source: bool,
    pub preference_rank: usize,
    pub registry_order: usize,
}
#[derive(Debug, Clone)]
pub struct SourceCandidateInventory {
    pub candidates: Vec<SourceCandidateRecord>,
    pub observations: Vec<SourceCandidateObservation>,
}
/// Observation is independent of the final solver decision. A previously probed
/// candidate may later be excluded by a pin; it must not become "not probed".
#[derive(Debug, Clone)]
pub struct SourceCandidateObservation {
    pub binding: CanonicalSourceBinding,
    pub observation: SourceCandidateObservationKind,
    pub decision: SourceCandidateDecision,
}
#[derive(Debug, Clone)]
pub enum SourceCandidateObservationKind {
    /// Historical lookup observations were not retained; not a claim of no contact.
    Unknown,
    NotProbed,
    /// Metadata was observed; no claim of current remote/payload availability or contact.
    Available {
        versions: Vec<semver::Version>,
    },
    NoMatchingVersion {
        constraint: String,
    },
    NotFound,
    Unavailable {
        reason: SourceCandidateFailure,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceCandidateFailure {
    /// Source setup/read failure not known to be transport, auth, trust or metadata.
    SourceUnavailable,
    Transport,
    Authentication,
    Trust,
    InvalidMetadata,
    Unsupported,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceCandidateDecision {
    Eligible,
    Selected,
    ExcludedByPin { registry: String },
    ExcludedHost { endpoint: String },
    Disabled,
    VersionRejected,
    LowerPreference,
    NotSelected,
}
/// Resource exhaustion refuses explicitly; it is not an unsatisfiable graph.
#[derive(Debug, Clone, Copy)]
pub struct SourceInventoryLimits {
    pub max_candidates: usize,
}
impl Default for SourceInventoryLimits {
    fn default() -> Self {
        Self {
            max_candidates: 512,
        }
    }
}

/// The existing local-family precedence owner supplies this tag explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalCandidateKind {
    ExplicitRegistry,
    ProjectLocal,
    Embedded,
}
#[path = "source_candidates/local.rs"]
mod local;

/// Pure observation of operation policy knowledge; Deferred is never an empty policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceChoiceMode {
    LegacyKnown,
    CandidateRequired,
    Deferred,
}
/// Discovery scope, never an accumulated registry-pin constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateInventoryScope {
    DefaultOnly,
    AllAllowed,
}

impl SourceCandidateInventory {
    /// Existing source owners supply the unchanged version domain and exact
    /// precedence winner per version. Preference only admits same-version alternatives.
    pub fn qualify_defaults(
        &mut self,
        domain: &std::collections::BTreeSet<semver::Version>,
        preferred: &std::collections::BTreeMap<semver::Version, CanonicalSourceBinding>,
        preference_active: bool,
    ) {
        for record in &mut self.candidates {
            record.in_default_version_domain = domain.contains(record.candidate.version());
            record.legacy_preferred_source =
                preferred.get(record.candidate.version()) == Some(record.candidate.binding());
            record.default_eligible = record.in_default_version_domain
                && (record.legacy_preferred_source || preference_active);
        }
    }
}

/// Only observations actually retained by the lookup; omitted rows are not absence.
#[derive(Debug, Clone)]
pub struct RetainedCandidateObservation {
    pub candidate: RetainedSourceCandidate,
    pub observations: Vec<SourceCandidateObservation>,
}

#[cfg(test)]
#[path = "source_candidates/tests.rs"]
mod tests;

#[cfg(all(test, unix))]
#[path = "source_candidates/tests_warm.rs"]
mod tests_warm;
