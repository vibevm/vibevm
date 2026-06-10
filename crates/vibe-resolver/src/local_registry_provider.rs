//! `DepProvider` adapter over a [`vibe_registry::LocalRegistry`].
//!
//! For the `--registry <path>` install path. Reads manifests directly off
//! disk under `<root>/<group>/<name>/v<ver>/vibe.toml`.

use std::path::PathBuf;

use specmark::{cell, spec};
use vibe_core::manifest::Manifest;
use vibe_core::{Group, PackageRef};
use vibe_registry::{LocalRegistry, RegistryError};

use crate::{DepProvider, DepProviderError};

/// `DepProvider` impl backed by a [`LocalRegistry`].
#[cell(seam = "DepProvider", variant = "local-registry", flag = "provider")]
#[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-002#solver")]
pub struct LocalRegistryProvider<'a> {
    registry: &'a LocalRegistry,
}

impl<'a> LocalRegistryProvider<'a> {
    pub fn new(registry: &'a LocalRegistry) -> Self {
        LocalRegistryProvider { registry }
    }
}

impl<'a> DepProvider for LocalRegistryProvider<'a> {
    fn resolve_version(&self, pkgref: &PackageRef) -> Result<semver::Version, DepProviderError> {
        match self.registry.resolve(pkgref) {
            Ok(r) => Ok(r.version),
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

    fn fetch_manifest(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<Manifest, DepProviderError> {
        let path: PathBuf = self
            .registry
            .root()
            .join(group.as_str())
            .join(name)
            .join(format!("v{version}"))
            .join(Manifest::FILENAME);
        Manifest::read(&path).map_err(|e| {
            DepProviderError::Other(format!(
                "failed to read manifest at `{}`: {e}",
                path.display()
            ))
        })
    }
}
