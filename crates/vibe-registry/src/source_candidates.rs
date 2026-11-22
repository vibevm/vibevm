//! Immutable metadata and acquisition selections retained by existing source owners.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY");
use crate::{MultiResolution, RegistryError};
use sha2::{Digest, Sha256};
use specmark::spec;
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
    /// Observe content only at an already-retained ordinary local registry route.
    /// This is operator-selected source evidence, not supplier authentication.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY")]
    pub fn observe_local_content_pin(&self) -> Result<vibe_core::ContentHash, RegistryError> {
        let resolution = &self.0.resolution;
        if resolution.locked.is_some()
            || resolution.direct.is_some()
            || resolution.from_store
            || resolution.overridden
            || resolution.is_git_source
            || resolution.is_path_source
            || resolution.via_redirect.is_some()
            || !resolution.source_url.starts_with("file://")
        {
            return Err(RegistryError::MalformedMeta {
                path: resolution.resolved.source_dir.clone(),
                reason: "forwarding requires a fresh retained ordinary local source".into(),
            });
        }
        self.validate_manifest(&Manifest::read(
            resolution.resolved.source_dir.join(Manifest::FILENAME),
        )?)?;
        Ok(vibe_core::ContentHash::from_validated(
            crate::compute_content_hash(&resolution.resolved.source_dir)?,
        ))
    }

    /// Exact local retained route/content agreement for a narrowly admitted
    /// operation transition. This observes no alternate source and grants no write.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
    pub fn matches_locked_local_content(
        &self,
        row: &vibe_core::manifest::LockedPackage,
    ) -> Result<bool, RegistryError> {
        let r = &self.0.resolution;
        if self.group() != &row.group
            || self.name() != row.name.as_str()
            || self.version() != &row.version
            || r.registry_name != row.registry
            || r.source_url != row.source_url.as_str()
            || r.source_ref != row.source_ref
            || r.overridden != row.overridden
            || r.via_redirect != row.via_redirect
            || row.resolved_commit.is_some()
            || row.source_ref.is_some()
            || row.overridden
            || row.via_override.is_some()
            || row.via_redirect.is_some()
            || !row.source_url.as_str().starts_with("file://")
            || row.materialization != vibe_core::manifest::Materialization::Copy
            || row.source_kind != Some(vibe_core::manifest::SourceKind::Registry)
        {
            return Ok(false);
        }
        // Rebinding authored history is stronger than replaying retained bytes:
        // inspect the exact live endpoint with the existing held no-follow proof.
        let live = crate::multi_registry_resolver::local_path_from_url(row.source_url.as_str())?;
        let proof = crate::local_preview::verify_pinned_payload(&live, row.content_hash.as_str())
            .map_err(|_| RegistryError::MalformedMeta {
            path: live.clone(),
            reason: "guarded gate rebind requires unchanged live source bytes".into(),
        })?;
        let observed = Manifest::parse_str(
            std::str::from_utf8(proof.manifest_bytes().map_err(|_| {
                RegistryError::MalformedMeta {
                    path: live.clone(),
                    reason: "guarded live source manifest unavailable".into(),
                }
            })?)
            .map_err(|_| RegistryError::MalformedMeta {
                path: live.clone(),
                reason: "guarded live source manifest encoding invalid".into(),
            })?,
        )?;
        self.validate_manifest(&observed)?;
        proof.recheck().map_err(|_| RegistryError::MalformedMeta {
            path: live,
            reason: "guarded live source changed during rebind".into(),
        })?;
        if let Some(held) = &r.locked {
            // Retention already used the existing locked acquisition owner.
            // Do not relax fresh-source observation for forwarding callers.
            if held.source_row() != row
                || self.binding() != &CanonicalSourceBinding::from_locked(row)
                || !r.source_url.starts_with("file://")
                || r.resolved.source_dir.as_os_str().is_empty()
            {
                return Ok(false);
            }
            self.validate_manifest(&Manifest::read(
                r.resolved.source_dir.join(Manifest::FILENAME),
            )?)?;
            let recipe = if row.content_hash.as_str().starts_with("sha256-tree/1:") {
                crate::RecipeId::Tree1
            } else {
                crate::RecipeId::Legacy0
            };
            return Ok(
                crate::compute_content_hash_with(recipe, &r.resolved.source_dir)?
                    == row.content_hash.as_str(),
            );
        }
        Ok(self.observe_local_content_pin()? == row.content_hash)
    }

    /// Whether both observations were minted by the same original source owner.
    pub fn same_owner(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0.owner, &other.0.owner)
    }

    /// Compare an already policy-admitted observation with the original route.
    /// A held binding includes its content pin whereas discovery has none; this
    /// comparison does not grant eligibility or waive the pin during acquisition.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY")]
    pub fn same_selected_source(&self, other: &Self) -> bool {
        if self.group() != other.group()
            || self.name() != other.name()
            || self.version() != other.version()
            || self.metadata_recipe() != other.metadata_recipe()
            || self.metadata_digest() != other.metadata_digest()
        {
            return false;
        }
        if self.binding() == other.binding() {
            return true;
        }
        if !matches!(self.binding(), CanonicalSourceBinding::Locked { .. })
            && !matches!(other.binding(), CanonicalSourceBinding::Locked { .. })
        {
            return false;
        }
        let a = &self.0.resolution;
        let b = &other.0.resolution;
        a.source_url == b.source_url
            && a.source_ref == b.source_ref
            && a.is_git_source == b.is_git_source
            && a.is_path_source == b.is_path_source
            && a.overridden == b.overridden
            && a.via_redirect == b.via_redirect
            && a.redirect_target_auth == b.redirect_target_auth
            && a.redirect_target_token_env == b.redirect_target_token_env
    }

    /// Retain the personal owner's locked-source verification law after
    /// acquisition through an independently retained original route.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
    pub fn verify_continued_acquisition(
        &self,
        acquired: crate::PackageAcquisition,
    ) -> Result<crate::PackageAcquisition, RegistryError> {
        match acquired {
            crate::PackageAcquisition::Cached(mut cached) => {
                self.validate_manifest(&cached.manifest)?;
                if let Some(held) = &self.0.resolution.locked {
                    held.verify_continued(&mut cached)?;
                }
                Ok(crate::PackageAcquisition::Cached(cached))
            }
            crate::PackageAcquisition::InPlace(prepared) => {
                self.validate_manifest(prepared.manifest())?;
                let prepared = if let Some(held) = &self.0.resolution.locked {
                    held.verify_continued_in_place(prepared)?
                } else {
                    prepared
                };
                Ok(crate::PackageAcquisition::InPlace(prepared))
            }
        }
    }

    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY")]
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
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY-LAW")]
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
