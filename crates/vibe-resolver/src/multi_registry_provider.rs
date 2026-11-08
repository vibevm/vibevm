//! `DepProvider` adapter over a `MultiRegistryResolver`.
//!
//! Wraps the multi-registry / mirror / override dispatch from
//! [`vibe_registry::MultiRegistryResolver`] in the trait the solver
//! consumes. Registry resolution → version pick; override resolution →
//! version pulled out of the overridden manifest. Manifest reads go
//! through `git archive` (no clone) for registry-served packages and
//! through the same shallow primitive for overrides.

use specmark::{cell, spec};
use vibe_core::manifest::Manifest;
use vibe_core::{Group, PackageRef};
use vibe_registry::{MultiRegistryResolver, RegistryError};

use crate::{DepProvider, DepProviderError, VersionEnumerator};

/// `DepProvider` impl backed by a [`MultiRegistryResolver`].
#[cell(seam = "DepProvider", variant = "multi-registry", flag = "provider")]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#solver")]
pub struct MultiRegistryDepProvider<'a> {
    resolver: &'a MultiRegistryResolver,
}

impl<'a> MultiRegistryDepProvider<'a> {
    pub fn new(resolver: &'a MultiRegistryResolver) -> Self {
        MultiRegistryDepProvider { resolver }
    }
}

impl<'a> DepProvider for MultiRegistryDepProvider<'a> {
    #[spec(
        implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator"
    )]
    fn resolve_version(&self, pkgref: &PackageRef) -> Result<semver::Version, DepProviderError> {
        // `MultiRegistryResolver::resolve` returns a `MultiResolution`
        // already pinning the version (and tracking provenance). We
        // discard provenance here — the install pipeline still calls
        // `resolver.resolve` separately when it needs to fetch.
        match self.resolver.resolve(pkgref) {
            Ok(r) => Ok(r.resolved.version),
            Err(RegistryError::UnknownPackage { group, name }) => {
                Err(DepProviderError::UnknownPackage { group, name })
            }
            Err(RegistryError::NoMatchingVersion { group, name, req }) => {
                Err(DepProviderError::NoMatchingVersion {
                    group,
                    name,
                    constraint: req,
                })
            }
            // Preserve the structured per-registry attempts so the
            // downstream install-error JSON envelope can ship them
            // verbatim. The summary string carries the same data
            // for prose-only consumers (text mode + `Display`).
            Err(RegistryError::PackageNotFoundEverywhere {
                group,
                name,
                summary,
                attempts,
            }) => Err(DepProviderError::AggregateNotFound {
                group,
                name,
                summary,
                attempts,
            }),
            Err(other @ RegistryError::OfflinePackageUnavailable { .. }) => {
                Err(DepProviderError::offline_unavailable(other))
            }
            Err(other) => Err(DepProviderError::Other(other.to_string())),
        }
    }

    #[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#redirect")]
    fn fetch_manifest(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<Manifest, DepProviderError> {
        // Delegate to the resolver's redirect-aware `fetch_manifest`:
        // walks registries in priority order, follows any
        // `vibe-redirect.toml` stub it lands on (PROP-002 §2.4.2),
        // returns the target's manifest. Overrides are not read here —
        // they short-circuit at `resolve_version` time and the install
        // pipeline handles their content fetch separately.
        match self.resolver.fetch_manifest(group, name, version) {
            Ok(m) => Ok(m),
            Err(RegistryError::UnknownPackage { group, name }) => {
                Err(DepProviderError::UnknownPackage { group, name })
            }
            Err(RegistryError::NoMatchingVersion { group, name, req }) => {
                Err(DepProviderError::NoMatchingVersion {
                    group,
                    name,
                    constraint: req,
                })
            }
            Err(other @ RegistryError::OfflinePackageUnavailable { .. }) => {
                Err(DepProviderError::offline_unavailable(other))
            }
            Err(other) => Err(DepProviderError::Other(other.to_string())),
        }
    }
}

impl<'a> VersionEnumerator for MultiRegistryDepProvider<'a> {
    #[spec(
        implements = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-017#provider-enrichment"
    )]
    fn list_versions(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Vec<semver::Version>, DepProviderError> {
        match self.resolver.list_versions(group, name) {
            Ok(versions) => Ok(versions),
            Err(RegistryError::UnknownPackage { group, name }) => {
                Err(DepProviderError::UnknownPackage { group, name })
            }
            Err(RegistryError::NoMatchingVersion { group, name, req }) => {
                Err(DepProviderError::NoMatchingVersion {
                    group,
                    name,
                    constraint: req,
                })
            }
            Err(RegistryError::PackageNotFoundEverywhere {
                group,
                name,
                summary,
                attempts,
            }) => Err(DepProviderError::AggregateNotFound {
                group,
                name,
                summary,
                attempts,
            }),
            Err(other @ RegistryError::OfflinePackageUnavailable { .. }) => {
                Err(DepProviderError::offline_unavailable(other))
            }
            Err(other) => Err(DepProviderError::Other(other.to_string())),
        }
    }
}

impl crate::SourceCandidateProvider for MultiRegistryDepProvider<'_> {
    fn has_locked_coordinate(&self, group: &Group, name: &str) -> bool {
        self.resolver.has_locked_package(group, name)
    }
    fn source_choice_mode(&self) -> vibe_registry::source_candidates::SourceChoiceMode {
        self.resolver.source_choice_mode()
    }
    fn retain_default_candidate(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<vibe_registry::source_candidates::RetainedCandidateObservation, DepProviderError>
    {
        self.resolver
            .retain_default_candidate(group, name, version)
            .map_err(crate::source_candidates::source_registry_error)
    }
    fn validate_registry_pin(
        &self,
        group: &Group,
        name: &str,
        pin: &vibe_core::manifest::RegistryPin,
    ) -> Result<(), DepProviderError> {
        self.resolver
            .validate_registry_pin(group, name, pin)
            .map_err(crate::source_candidates::source_registry_error)
    }
    fn source_inventory(
        &self,
        group: &Group,
        name: &str,
        scope: vibe_registry::source_candidates::CandidateInventoryScope,
        limits: &vibe_registry::source_candidates::SourceInventoryLimits,
    ) -> Result<vibe_registry::source_candidates::SourceCandidateInventory, DepProviderError> {
        self.resolver
            .candidate_inventory(group, name, limits, scope)
            .map_err(crate::source_candidates::source_registry_error)
    }
    fn respects_locked_selection(
        &self,
        candidate: &vibe_registry::source_candidates::RetainedSourceCandidate,
        locked: &vibe_core::manifest::LockedPackage,
    ) -> Result<bool, DepProviderError> {
        self.resolver
            .respects_locked_selection(candidate, locked)
            .map_err(crate::source_candidates::source_registry_error)
    }
}
