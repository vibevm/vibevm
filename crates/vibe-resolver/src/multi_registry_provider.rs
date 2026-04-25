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
            Err(other) => Err(DepProviderError::Other(other.to_string())),
        }
    }

    fn fetch_manifest(
        &self,
        kind: PackageKind,
        name: &str,
        version: &semver::Version,
    ) -> Result<PackageManifest, DepProviderError> {
        // Walk registries in priority order; first one that has the
        // package serves the manifest. The `MultiRegistryResolver`
        // doesn't expose a manifest accessor today (its job is
        // resolve+fetch), so we fan out across its `registries()` list
        // ourselves. Overrides are not read here — they short-circuit
        // at `resolve_version` time and the install pipeline handles
        // their content fetch separately.
        let mut last_err: Option<DepProviderError> = None;
        for reg in self.resolver.registries() {
            match reg.fetch_dep_manifest(kind, name, version) {
                Ok(m) => return Ok(m),
                Err(RegistryError::UnknownPackage { .. })
                | Err(RegistryError::Git(_))
                | Err(RegistryError::NoMatchingVersion { .. }) => {
                    // Try the next registry — this one didn't have it.
                    last_err = Some(DepProviderError::UnknownPackage {
                        kind,
                        name: name.to_string(),
                    });
                    continue;
                }
                Err(other) => return Err(DepProviderError::Other(other.to_string())),
            }
        }
        Err(last_err.unwrap_or(DepProviderError::UnknownPackage {
            kind,
            name: name.to_string(),
        }))
    }
}
