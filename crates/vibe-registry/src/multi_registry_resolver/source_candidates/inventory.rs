//! Enumerate alternatives without changing the existing default version domain.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY");
use super::*;
use std::collections::{BTreeMap, BTreeSet};
impl MultiRegistryResolver {
    /// A stable allowed inventory. Held coordinates never demand the current catalog.
    pub fn candidate_inventory(
        &self,
        group: &Group,
        name: &str,
        limits: &SourceInventoryLimits,
        scope: CandidateInventoryScope,
    ) -> Result<SourceCandidateInventory, RegistryError> {
        let coordinate = format!("{group}/{name}");
        let fixed = self.has_locked_package(group, name)
            || self.overrides()?.contains_key(&coordinate)
            || self.path_packages.contains_key(&coordinate)
            || self.git_packages.contains_key(&coordinate);
        if fixed || scope == CandidateInventoryScope::DefaultOnly {
            let versions = self.list_versions(group, name)?;
            enforce_limit(versions.len(), limits)?;
            let mut candidates = Vec::with_capacity(versions.len());
            let mut observations = Vec::new();
            for version in versions {
                let retained = self.retain_default_candidate(group, name, &version)?;
                observations.extend(retained.observations);
                candidates.push(SourceCandidateRecord {
                    candidate: retained.candidate,
                    default_eligible: true,
                    in_default_version_domain: true,
                    legacy_preferred_source: true,
                    preference_rank: 0,
                    registry_order: 0,
                });
            }
            return Ok(SourceCandidateInventory {
                candidates,
                observations,
            });
        }
        let policy = self.source_policy_for_new_selection()?;
        let rows = self.declared_registry_rows()?;
        let mut observations = Vec::new();
        for row in rows.iter().filter(|row| !row.enabled) {
            observations.push(SourceCandidateObservation {
                binding: Binding::Registry {
                    registry: RegistryPin::new(row.name.clone())?,
                    source_url: row.url.clone().into(),
                },
                observation: SourceCandidateObservationKind::NotProbed,
                decision: SourceCandidateDecision::Disabled,
            });
        }
        // A registry the effective source policy disables is observed exactly
        // like a row switched off by hand: recorded, never probed (PROP-073
        // `#SOURCE-POLICY`). Deciding it here, from the declared URL, is what
        // keeps the inventory from opening it to be refused later.
        for row in rows
            .iter()
            .filter(|row| row.enabled && policy.excludes_endpoint(&row.url))
        {
            observations.push(SourceCandidateObservation {
                binding: Binding::Registry {
                    registry: RegistryPin::new(row.name.clone())?,
                    source_url: row.url.clone().into(),
                },
                observation: SourceCandidateObservationKind::NotProbed,
                decision: SourceCandidateDecision::ExcludedHost {
                    endpoint: row.url.clone(),
                },
            });
        }
        vibe_core::manifest::validate_registry_aliases(&rows, "effective")?;
        let mut candidates = Vec::new();
        let mut domain = BTreeSet::new();
        let mut preferred = BTreeMap::new();
        let mut default_decided = false;
        let task = self
            .progress
            .task(format!("Discovering source candidates for {coordinate}"));
        for (order, row) in rows
            .iter()
            .filter(|row| row.enabled && !policy.excludes_endpoint(&row.url))
            .enumerate()
        {
            let binding = Binding::Registry {
                registry: RegistryPin::new(row.name.clone())?,
                source_url: row.url.clone().into(),
            };
            let source = match self.selected_registry_source(&row.name) {
                Ok(source) => source,
                Err(error) => {
                    if global_inventory_error(&error) {
                        return Err(error);
                    }
                    if !default_decided {
                        default_decided = true;
                    }
                    observations.push(failed_observation(binding, &error));
                    continue;
                }
            };
            let versions =
                match self.list_versions_in_registry(group, name, &task, Some(source.name())) {
                    Ok(versions) => versions,
                    Err(error) => {
                        if global_inventory_error(&error) {
                            return Err(error);
                        }
                        if !default_decided
                            && !walks_past(source.as_ref(), self.strict_auth, &error)
                        {
                            default_decided = true;
                        }
                        observations.push(failed_observation(binding, &error));
                        continue;
                    }
                };
            if !default_decided && !versions.is_empty() {
                domain.extend(versions.iter().cloned());
                for version in &versions {
                    preferred.insert(version.clone(), binding.clone());
                }
                default_decided = true;
            }
            enforce_limit(candidates.len().saturating_add(versions.len()), limits)?;
            observations.push(SourceCandidateObservation {
                binding: binding.clone(),
                observation: SourceCandidateObservationKind::Available {
                    versions: versions.clone(),
                },
                decision: SourceCandidateDecision::Eligible,
            });
            for version in versions {
                let reference = exact_reference(group, name, &version)?;
                let retained = self
                    .resolve_in_registry(&reference, &task, Some(source.name()))
                    .and_then(|resolution| {
                        self.retain_resolution(resolution, Some(binding.clone()))
                    });
                match retained {
                    Ok(retained) => {
                        let evidence = retained.candidate.source_evidence();
                        let location = evidence
                            .fetched_from
                            .as_deref()
                            .or(evidence.source_url.as_deref())
                            .unwrap_or("");
                        let preference_rank = policy.preference_rank(location);
                        candidates.push(SourceCandidateRecord {
                            candidate: retained.candidate,
                            default_eligible: false,
                            in_default_version_domain: false,
                            legacy_preferred_source: false,
                            preference_rank,
                            registry_order: order,
                        });
                    }
                    Err(error) => {
                        if global_inventory_error(&error) {
                            return Err(error);
                        }
                        observations.push(failed_observation(binding.clone(), &error));
                    }
                }
            }
        }
        task.finish();
        let mut inventory = SourceCandidateInventory {
            candidates,
            observations,
        };
        inventory.qualify_defaults(&domain, &preferred, policy.has_preference());
        Ok(inventory)
    }
}
fn enforce_limit(actual: usize, limits: &SourceInventoryLimits) -> Result<(), RegistryError> {
    if actual > limits.max_candidates {
        return Err(SourcePolicyError::ResourceLimit {
            actual,
            limit: limits.max_candidates,
        }
        .into());
    }
    Ok(())
}
fn global_inventory_error(error: &RegistryError) -> bool {
    matches!(
        error,
        RegistryError::SourcePolicy(_)
            | RegistryError::ContactRefused(crate::ContactRefusal::OperationCancelled)
            | RegistryError::Git(GitError::ContactRefused(
                crate::ContactRefusal::OperationCancelled
            ))
    )
}
/// Whether a registry's answer leaves the default version domain undecided,
/// so a later registry may still decide it (PROP-002 §2.3.1). A registry the
/// filtered walk reports as not holding the package at all — the aggregate
/// `PackageNotFoundEverywhere` a monorepo or a per-package Git registry
/// returns after its version list came back empty — is an absence exactly
/// like `UnknownPackage`, never a decision.
fn walks_past(source: &RegistrySource, strict: bool, error: &RegistryError) -> bool {
    matches!(
        error,
        RegistryError::UnknownPackage { .. } | RegistryError::NoMatchingVersion { .. }
    ) || matches!(
        error,
        RegistryError::PackageNotFoundEverywhere { attempts, .. }
            if attempts.iter().all(|attempt| matches!(
                attempt.status,
                WalkAttemptStatus::NotFound | WalkAttemptStatus::Public401
            ))
    ) || matches!(
        error,
        RegistryError::ContactRefused(crate::ContactRefusal::ExcludedHost { .. })
            | RegistryError::Git(GitError::ContactRefused(
                crate::ContactRefusal::ExcludedHost { .. }
            ))
            | RegistryError::RegistryDisabledByPolicy { .. }
    ) || matches!((source, error), (RegistrySource::Git(registry), RegistryError::Git(GitError::AuthFailed { .. })) if registry.auth_kind() == vibe_core::manifest::AuthKind::None && !strict)
}
fn failed_observation(binding: Binding, error: &RegistryError) -> SourceCandidateObservation {
    let (observation, decision) = match error {
        // The row was named and the policy disables it: recorded as not
        // probed, because it was not (PROP-073 `#SOURCE-POLICY`).
        RegistryError::RegistryDisabledByPolicy { .. } => (
            SourceCandidateObservationKind::NotProbed,
            SourceCandidateDecision::ExcludedHost {
                endpoint: match &binding {
                    Binding::Registry { source_url, .. } => source_url.to_string(),
                    _ => String::new(),
                },
            },
        ),
        RegistryError::ContactRefused(crate::ContactRefusal::ExcludedHost { endpoint })
        | RegistryError::Git(GitError::ContactRefused(crate::ContactRefusal::ExcludedHost {
            endpoint,
        })) => (
            SourceCandidateObservationKind::Unknown,
            SourceCandidateDecision::ExcludedHost {
                endpoint: endpoint.clone(),
            },
        ),
        RegistryError::PackageNotFoundEverywhere { attempts, .. }
            if !attempts.is_empty()
                && attempts
                    .iter()
                    .all(|attempt| matches!(attempt.status, WalkAttemptStatus::NotFound)) =>
        {
            (
                SourceCandidateObservationKind::NotFound,
                SourceCandidateDecision::Eligible,
            )
        }
        RegistryError::PackageNotFoundEverywhere { attempts, .. }
            if attempts
                .iter()
                .any(|attempt| matches!(attempt.status, WalkAttemptStatus::Public401)) =>
        {
            (
                SourceCandidateObservationKind::Unavailable {
                    reason: SourceCandidateFailure::Authentication,
                },
                SourceCandidateDecision::Eligible,
            )
        }
        RegistryError::UnknownPackage { .. } => (
            SourceCandidateObservationKind::NotFound,
            SourceCandidateDecision::Eligible,
        ),
        RegistryError::NoMatchingVersion { req, .. } => (
            SourceCandidateObservationKind::NoMatchingVersion {
                constraint: req.clone(),
            },
            SourceCandidateDecision::Eligible,
        ),
        RegistryError::MalformedMeta { .. }
        | RegistryError::Core(vibe_core::Error::ParseToml { .. })
        | RegistryError::Index(crate::IndexError::Malformed { .. }) => (
            SourceCandidateObservationKind::Unavailable {
                reason: SourceCandidateFailure::InvalidMetadata,
            },
            SourceCandidateDecision::Eligible,
        ),
        RegistryError::Git(GitError::SshHostKeyRejected) => (
            SourceCandidateObservationKind::Unavailable {
                reason: SourceCandidateFailure::Trust,
            },
            SourceCandidateDecision::Eligible,
        ),
        RegistryError::Index(crate::IndexError::AuthIncapable { .. }) => (
            SourceCandidateObservationKind::Unavailable {
                reason: SourceCandidateFailure::Authentication,
            },
            SourceCandidateDecision::Eligible,
        ),
        RegistryError::Index(crate::IndexError::Status {
            status: 401 | 403, ..
        }) => (
            SourceCandidateObservationKind::Unavailable {
                reason: SourceCandidateFailure::Authentication,
            },
            SourceCandidateDecision::Eligible,
        ),
        RegistryError::MissingToken { .. }
        | RegistryError::Git(GitError::AuthFailed { .. } | GitError::SshAuthenticationRejected) => {
            (
                SourceCandidateObservationKind::Unavailable {
                    reason: SourceCandidateFailure::Authentication,
                },
                SourceCandidateDecision::Eligible,
            )
        }
        RegistryError::ContactRefused(_)
        | RegistryError::Git(
            GitError::ContactRefused(_)
            | GitError::NativeContext { .. }
            | GitError::NativeUnavailable { .. },
        ) => (
            SourceCandidateObservationKind::Unavailable {
                reason: SourceCandidateFailure::Unsupported,
            },
            SourceCandidateDecision::Eligible,
        ),
        RegistryError::Git(GitError::NetworkUnreachable { .. })
        | RegistryError::Index(crate::IndexError::Http { .. }) => (
            SourceCandidateObservationKind::Unavailable {
                reason: SourceCandidateFailure::Transport,
            },
            SourceCandidateDecision::Eligible,
        ),
        _ => (
            SourceCandidateObservationKind::Unavailable {
                reason: SourceCandidateFailure::SourceUnavailable,
            },
            SourceCandidateDecision::Eligible,
        ),
    };
    SourceCandidateObservation {
        binding,
        observation,
        decision,
    }
}
