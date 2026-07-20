//! `DepProvider` adapter over a [`vibe_registry::LocalRegistry`].
//!
//! For the `--registry <path>` install path. Reads manifests directly off
//! disk under `<root>/<group>/<name>/v<ver>/vibe.toml`.

use std::path::PathBuf;

use specmark::{cell, spec};
use vibe_core::manifest::Manifest;
use vibe_core::{Group, PackageRef};
use vibe_registry::{LocalRegistry, RegistryError};

use crate::{DepProvider, DepProviderError, VersionEnumerator};

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
        let dir: PathBuf = self
            .registry
            .root()
            .join(group.as_str())
            .join(name)
            .join(format!("v{version}"));
        // A missing version directory means this registry does not carry the
        // coordinate — return `UnknownPackage` so a composing provider (the
        // embedded composition) treats it as absent and falls through to the
        // next source, instead of halting on an `Other` read error. A present
        // directory with a missing / unparseable manifest is a real
        // malformation and stays an error.
        if !dir.is_dir() {
            return Err(DepProviderError::UnknownPackage {
                group: group.clone(),
                name: name.to_string(),
            });
        }
        let path = dir.join(Manifest::FILENAME);
        Manifest::read(&path).map_err(|e| {
            DepProviderError::Other(format!(
                "failed to read manifest at `{}`: {e}",
                path.display()
            ))
        })
    }
}

impl<'a> VersionEnumerator for LocalRegistryProvider<'a> {
    #[spec(implements = "spec://vibevm/modules/vibe-resolver/PROP-017#provider-enrichment")]
    fn list_versions(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Vec<semver::Version>, DepProviderError> {
        match self.registry.list_versions(group, name) {
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
            Err(other) => Err(DepProviderError::Other(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A missing version directory is `UnknownPackage` — the "absent"
    /// signal the embedded composition falls through on (not an `Other`
    /// read error that would halt it). This is the fix for an embedded
    /// registry that carries the group but not the coordinate: a declared
    /// local registry that does carry it then serves the package.
    #[test]
    fn fetch_manifest_absent_version_dir_is_unknown_package() {
        let tmp = tempfile::tempdir().unwrap();
        let reg = LocalRegistry::new(tmp.path()).unwrap();
        let provider = LocalRegistryProvider::new(&reg);
        let group = Group::parse("org.vibevm").unwrap();
        let ver = semver::Version::parse("0.1.0").unwrap();
        let err = provider.fetch_manifest(&group, "wal", &ver).unwrap_err();
        assert!(
            matches!(err, DepProviderError::UnknownPackage { .. }),
            "a missing version dir is an absence, not an error; got {err:?}"
        );
    }

    /// A present version directory with a valid `vibe.toml` reads fine.
    #[test]
    fn fetch_manifest_present_dir_reads_manifest() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("org.vibevm").join("wal").join("v0.1.0");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join(Manifest::FILENAME),
            "[package]\ngroup=\"org.vibevm\"\nname=\"wal\"\nkind=\"flow\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        let reg = LocalRegistry::new(tmp.path()).unwrap();
        let provider = LocalRegistryProvider::new(&reg);
        let group = Group::parse("org.vibevm").unwrap();
        let ver = semver::Version::parse("0.1.0").unwrap();
        let manifest = provider.fetch_manifest(&group, "wal", &ver).unwrap();
        assert!(manifest.package.is_some());
    }

    /// A present version directory WITHOUT a `vibe.toml` is a real
    /// malformation (not an absence) — it stays an `Other` error rather than
    /// masquerading as `UnknownPackage`.
    #[test]
    fn fetch_manifest_present_dir_missing_vibe_toml_is_other_error() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("org.vibevm").join("wal").join("v0.1.0");
        fs::create_dir_all(&dir).unwrap();
        // no vibe.toml inside
        let reg = LocalRegistry::new(tmp.path()).unwrap();
        let provider = LocalRegistryProvider::new(&reg);
        let group = Group::parse("org.vibevm").unwrap();
        let ver = semver::Version::parse("0.1.0").unwrap();
        let err = provider.fetch_manifest(&group, "wal", &ver).unwrap_err();
        assert!(
            matches!(err, DepProviderError::Other(_)),
            "a present dir missing its manifest is a malformation, not an absence; got {err:?}"
        );
    }
}
