//! Existing solver's provider interface over retained manifests only.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#DOCUMENT-ANALYSIS-CAPTURE-LAW");
use semver::Version;
use std::collections::BTreeMap;
use vibe_core::{Group, PackageRef, manifest::Manifest};
use vibe_resolver::{DepProvider, DepProviderError, VersionEnumerator};
#[derive(Debug, Clone, Default)]
pub(super) struct CapturedCatalog {
    manifests: BTreeMap<(Group, String, Version), Manifest>,
}
impl CapturedCatalog {
    pub fn insert(&mut self, manifest: Manifest) -> Result<(), String> {
        let Some(package) = manifest.package.as_ref() else {
            return Ok(());
        };
        let key = (
            package.group.clone(),
            package.name.clone(),
            package.version.clone(),
        );
        if self.manifests.insert(key, manifest).is_some() {
            return Err("ambiguous captured package version".into());
        }
        Ok(())
    }
}
impl DepProvider for CapturedCatalog {
    fn resolve_version(&self, reference: &PackageRef) -> Result<Version, DepProviderError> {
        let group = reference.group.as_ref().ok_or_else(|| {
            DepProviderError::Other("capture requires qualified package roots".into())
        })?;
        self.list_versions(group, reference.name.as_str())?
            .into_iter()
            .filter(|v| reference.version.matches(v))
            .max()
            .ok_or_else(|| DepProviderError::NoMatchingVersion {
                group: group.clone(),
                name: reference.name.to_string(),
                constraint: reference.version.to_string(),
            })
    }
    fn fetch_manifest(
        &self,
        group: &Group,
        name: &str,
        version: &Version,
    ) -> Result<Manifest, DepProviderError> {
        self.manifests
            .get(&(group.clone(), name.into(), version.clone()))
            .cloned()
            .ok_or_else(|| {
                DepProviderError::Other(
                    "manifest unavailable in immutable capture; no fetch is permitted".into(),
                )
            })
    }
}
impl VersionEnumerator for CapturedCatalog {
    fn list_versions(&self, group: &Group, name: &str) -> Result<Vec<Version>, DepProviderError> {
        let versions: Vec<_> = self
            .manifests
            .keys()
            .filter(|(g, n, _)| g == group && n == name)
            .map(|(_, _, v)| v.clone())
            .collect();
        if versions.is_empty() {
            Err(DepProviderError::UnknownPackage {
                group: group.clone(),
                name: name.into(),
            })
        } else {
            Ok(versions)
        }
    }
}
