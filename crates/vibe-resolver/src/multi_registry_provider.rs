//! `DepProvider` adapter over a `MultiRegistryResolver`.
//!
//! Wraps the multi-registry / mirror / override dispatch from
//! [`vibe_registry::MultiRegistryResolver`] in the trait the solver
//! consumes. Registry resolution → version pick; override resolution →
//! version pulled out of the overridden manifest. Manifest reads go
//! through `git archive` (no clone) for registry-served packages and
//! through the same shallow primitive for overrides.

use vibe_core::manifest::PackageManifest;
use vibe_core::{PackageKind, PackageRef};
use vibe_registry::{MultiRegistryResolver, RegistryError};

use crate::{DepProvider, DepProviderError};

/// `DepProvider` impl backed by a [`MultiRegistryResolver`].
pub struct MultiRegistryProvider<'a> {
    resolver: &'a MultiRegistryResolver,
}

impl<'a> MultiRegistryProvider<'a> {
    pub fn new(resolver: &'a MultiRegistryResolver) -> Self {
        MultiRegistryProvider { resolver }
    }
}

impl<'a> DepProvider for MultiRegistryProvider<'a> {
    fn resolve_version(
        &self,
        pkgref: &PackageRef,
    ) -> Result<semver::Version, DepProviderError> {
        // `MultiRegistryResolver::resolve` returns a `MultiResolution`
        // already pinning the version (and tracking provenance). We
        // discard provenance here — the install pipeline still calls
        // `resolver.resolve` separately when it needs to fetch.
        match self.resolver.resolve(pkgref) {
            Ok(r) => Ok(r.resolved.version),
            Err(RegistryError::UnknownPackage { kind, name }) => {
                Err(DepProviderError::UnknownPackage { kind, name })
            }
            Err(RegistryError::NoMatchingVersion {
                kind,
                name,
                req,
            }) => Err(DepProviderError::NoMatchingVersion {
                kind,
                name,
                constraint: req,
            }),
            // Preserve the structured per-registry attempts so the
            // downstream install-error JSON envelope can ship them
            // verbatim. The summary string carries the same data
            // for prose-only consumers (text mode + `Display`).
            Err(RegistryError::PackageNotFoundEverywhere {
                kind,
                name,
                summary,
                attempts,
            }) => Err(DepProviderError::AggregateNotFound {
                kind,
                name,
                summary,
                attempts,
            }),
            Err(other) => Err(DepProviderError::Other(other.to_string())),
        }
    }

    fn fetch_manifest(
        &self,
        kind: PackageKind,
        name: &str,
        version: &semver::Version,
    ) -> Result<PackageManifest, DepProviderError> {
        // Delegate to the resolver's redirect-aware `fetch_manifest`:
        // walks registries in priority order, follows any
        // `vibe-redirect.toml` stub it lands on (PROP-002 §2.4.2),
        // returns the target's manifest. Overrides are not read here —
        // they short-circuit at `resolve_version` time and the install
        // pipeline handles their content fetch separately.
        match self.resolver.fetch_manifest(kind, name, version) {
            Ok(m) => Ok(m),
            Err(RegistryError::UnknownPackage { kind, name }) => {
                Err(DepProviderError::UnknownPackage { kind, name })
            }
            Err(RegistryError::NoMatchingVersion { kind, name, req }) => {
                Err(DepProviderError::NoMatchingVersion {
                    kind,
                    name,
                    constraint: req,
                })
            }
            Err(other) => Err(DepProviderError::Other(other.to_string())),
        }
    }
}
