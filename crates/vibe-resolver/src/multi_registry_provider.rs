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

use crate::{DepProvider, DepProviderError};

/// `DepProvider` impl backed by a [`MultiRegistryResolver`].
#[cell(seam = "DepProvider", variant = "multi-registry", flag = "provider")]
#[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-002#solver")]
pub struct MultiRegistryProvider<'a> {
    resolver: &'a MultiRegistryResolver,
}

impl<'a> MultiRegistryProvider<'a> {
    pub fn new(resolver: &'a MultiRegistryResolver) -> Self {
        MultiRegistryProvider { resolver }
    }
}

impl<'a> DepProvider for MultiRegistryProvider<'a> {
    #[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-002#failure-discriminator")]
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
            Err(other) => Err(DepProviderError::Other(other.to_string())),
        }
    }

    #[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-002#redirect")]
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
            Err(other) => Err(DepProviderError::Other(other.to_string())),
        }
    }
}
