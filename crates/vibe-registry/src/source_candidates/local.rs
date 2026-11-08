//! Retained metadata/acquisition for the existing local registry owner.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY");
use super::*;
use crate::{LocalRegistry, PackageAcquisition, ResolvedPackage};
use std::path::Path;
use vibe_core::manifest::{CanonicalSourceBinding, SourcePolicyError};

impl LocalRegistry {
    /// Read one already chosen legacy version without enumerating other sources/versions.
    pub fn retain_default_candidate(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
        kind: LocalCandidateKind,
    ) -> Result<RetainedCandidateObservation, RegistryError> {
        let binding = match kind {
            LocalCandidateKind::ExplicitRegistry => CanonicalSourceBinding::DirectoryRegistry {
                root: self.root().into(),
            },
            LocalCandidateKind::ProjectLocal => CanonicalSourceBinding::Local {
                root: self.root().into(),
            },
            LocalCandidateKind::Embedded => CanonicalSourceBinding::Embedded {
                root: self.root().into(),
            },
        };
        let source_dir = self
            .root()
            .join(group.as_str())
            .join(name)
            .join(format!("v{version}"));
        if !source_dir.is_dir() {
            return Err(RegistryError::UnknownPackage {
                group: group.clone(),
                name: name.into(),
            });
        }
        let manifest = Manifest::read(source_dir.join(Manifest::FILENAME))?;
        let source_url = crate::local_registry::source_uri_for_local(&source_dir);
        let mut evidence = crate::binding_source_evidence(&binding);
        evidence.source_url = Some(vibe_core::progress::transfer::safe_remote(&source_url));
        let resolution = MultiResolution {
            locked: None,
            direct: None,
            selected_route: None,
            resolved: ResolvedPackage {
                group: group.clone(),
                name: name.into(),
                version: version.clone(),
                source_dir,
            },
            registry_name: None,
            source_url,
            source_ref: None,
            overridden: false,
            is_git_source: false,
            is_path_source: false,
            via_redirect: None,
            from_store: false,
            redirect_target_auth: vibe_core::manifest::AuthKind::None,
            redirect_target_token_env: None,
        };
        let candidate = RetainedSourceCandidate::captured(
            &self.candidate_owner,
            binding.clone(),
            resolution,
            manifest,
            evidence,
        )?;
        Ok(RetainedCandidateObservation {
            candidate,
            observations: vec![SourceCandidateObservation {
                binding,
                observation: SourceCandidateObservationKind::Available {
                    versions: vec![version.clone()],
                },
                decision: SourceCandidateDecision::Eligible,
            }],
        })
    }

    pub fn candidate_inventory(
        &self,
        group: &Group,
        name: &str,
        kind: LocalCandidateKind,
        limits: &SourceInventoryLimits,
        _scope: CandidateInventoryScope,
    ) -> Result<SourceCandidateInventory, RegistryError> {
        let binding = match kind {
            LocalCandidateKind::ExplicitRegistry => CanonicalSourceBinding::DirectoryRegistry {
                root: self.root().into(),
            },
            LocalCandidateKind::ProjectLocal => CanonicalSourceBinding::Local {
                root: self.root().into(),
            },
            LocalCandidateKind::Embedded => CanonicalSourceBinding::Embedded {
                root: self.root().into(),
            },
        };
        let versions = match self.list_versions(group, name) {
            Ok(versions) => versions,
            Err(RegistryError::UnknownPackage { .. }) => {
                return Ok(SourceCandidateInventory {
                    candidates: Vec::new(),
                    observations: vec![SourceCandidateObservation {
                        binding,
                        observation: SourceCandidateObservationKind::NotFound,
                        decision: SourceCandidateDecision::Eligible,
                    }],
                });
            }
            Err(other) => return Err(other),
        };
        if versions.len() > limits.max_candidates {
            return Err(SourcePolicyError::ResourceLimit {
                actual: versions.len(),
                limit: limits.max_candidates,
            }
            .into());
        }
        let observations = vec![SourceCandidateObservation {
            binding: binding.clone(),
            observation: SourceCandidateObservationKind::Available {
                versions: versions.clone(),
            },
            decision: SourceCandidateDecision::Eligible,
        }];
        let mut candidates = Vec::with_capacity(versions.len());
        for version in versions {
            let retained = self.retain_default_candidate(group, name, &version, kind)?;
            candidates.push(SourceCandidateRecord {
                candidate: retained.candidate,
                default_eligible: true,
                in_default_version_domain: true,
                legacy_preferred_source: true,
                preference_rank: 0,
                registry_order: 0,
            });
        }
        Ok(SourceCandidateInventory {
            candidates,
            observations,
        })
    }

    /// Acquire the exact retained directory selection, never resolve another version/root.
    pub fn fetch_candidate_for_install(
        &self,
        candidate: &RetainedSourceCandidate,
        store_root: &Path,
        expected_hash: Option<&str>,
        _slot: &Path,
        _progress: &vibe_core::progress::Progress,
    ) -> Result<PackageAcquisition, RegistryError> {
        let resolution = candidate.resolution_for(&self.candidate_owner)?;
        if candidate
            .manifest()
            .require_package()?
            .materialization
            .is_in_place()
        {
            return Err(RegistryError::InPlaceUnsupported {
                group: candidate.group().clone(),
                name: candidate.name().into(),
            });
        }
        let current = Manifest::read(resolution.resolved.source_dir.join(Manifest::FILENAME))?;
        candidate.validate_manifest(&current)?;
        let mut cached = self.fetch(&resolution.resolved, store_root)?;
        candidate.validate_manifest(&cached.manifest)?;
        if let Some(expected) = expected_hash {
            crate::git_package_registry::verify_store_entry_against_pin(
                &cached.cache_dir,
                expected,
                candidate.group(),
                candidate.name(),
                candidate.version(),
            )?;
        }
        cached.is_local = matches!(candidate.binding(), CanonicalSourceBinding::Local { .. });
        cached.is_embedded = matches!(candidate.binding(), CanonicalSourceBinding::Embedded { .. });
        Ok(PackageAcquisition::Cached(Box::new(cached)))
    }
}
