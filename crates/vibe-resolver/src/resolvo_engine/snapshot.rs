//! Metadata reads shared by capability discovery, SAT and recommendation trials.
use std::cell::RefCell;
use std::collections::HashMap;

use crate::{DepProvider, DepProviderError, VersionEnumerator};
use vibe_core::{Group, PackageRef, manifest::Manifest};

pub(super) struct MetadataSnapshot<'a, P> {
    provider: &'a P,
    versions: RefCell<HashMap<(Group, String), Result<Vec<semver::Version>, DepProviderError>>>,
    manifests: RefCell<HashMap<(Group, String, semver::Version), Manifest>>,
}

impl<'a, P> MetadataSnapshot<'a, P> {
    pub(super) fn new(provider: &'a P) -> Self {
        Self {
            provider,
            versions: RefCell::new(HashMap::new()),
            manifests: RefCell::new(HashMap::new()),
        }
    }
}

impl<P: VersionEnumerator> DepProvider for MetadataSnapshot<'_, P> {
    fn resolve_version(&self, pkg: &PackageRef) -> Result<semver::Version, DepProviderError> {
        let group = pkg
            .group
            .as_ref()
            .ok_or_else(|| DepProviderError::Other("unqualified transaction root".into()))?;
        self.list_versions(group, pkg.name.as_str())?
            .into_iter()
            .filter(|v| pkg.version.matches(v))
            .max()
            .ok_or_else(|| DepProviderError::NoMatchingVersion {
                group: group.clone(),
                name: pkg.name.to_string(),
                constraint: pkg.version.to_string(),
            })
    }

    fn fetch_manifest(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<Manifest, DepProviderError> {
        let key = (group.clone(), name.to_owned(), version.clone());
        if let Some(manifest) = self.manifests.borrow().get(&key) {
            return Ok(manifest.clone());
        }
        let manifest = self.provider.fetch_manifest(group, name, version)?;
        self.manifests.borrow_mut().insert(key, manifest.clone());
        Ok(manifest)
    }
}

impl<P: VersionEnumerator> VersionEnumerator for MetadataSnapshot<'_, P> {
    fn cached_capabilities(&self) -> Option<&super::CapIndex> {
        self.provider.cached_capabilities()
    }

    fn list_versions(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Vec<semver::Version>, DepProviderError> {
        let key = (group.clone(), name.to_owned());
        if let Some(versions) = self.versions.borrow().get(&key) {
            return copy_versions(versions);
        }
        let versions = self.provider.list_versions(group, name);
        self.versions
            .borrow_mut()
            .insert(key, copy_versions(&versions));
        versions
    }
}

// Preserve the exact public discriminant and aggregate source attempts when
// replaying an absent package, rather than re-opening the mutable provider.
fn copy_versions(
    value: &Result<Vec<semver::Version>, DepProviderError>,
) -> Result<Vec<semver::Version>, DepProviderError> {
    match value {
        Ok(versions) => Ok(versions.clone()),
        Err(DepProviderError::UnknownPackage { group, name }) => {
            Err(DepProviderError::UnknownPackage {
                group: group.clone(),
                name: name.clone(),
            })
        }
        Err(DepProviderError::NoMatchingVersion {
            group,
            name,
            constraint,
        }) => Err(DepProviderError::NoMatchingVersion {
            group: group.clone(),
            name: name.clone(),
            constraint: constraint.clone(),
        }),
        Err(DepProviderError::AggregateNotFound {
            group,
            name,
            summary,
            attempts,
        }) => Err(DepProviderError::AggregateNotFound {
            group: group.clone(),
            name: name.clone(),
            summary: summary.clone(),
            attempts: attempts.clone(),
        }),
        Err(DepProviderError::Other(message)) => Err(DepProviderError::Other(message.clone())),
    }
}
