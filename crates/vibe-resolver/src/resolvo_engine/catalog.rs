//! Immutable source-bound view used by one install resolution transaction.
use crate::{DepProvider, DepProviderError, VersionEnumerator};
use std::collections::{BTreeMap, VecDeque};
use vibe_core::{Group, PackageRef, manifest::Manifest};
use vibe_registry::{CandidateIdentity, CandidateMetadata};

type Coordinate = (Group, String, semver::Version);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogInspection {
    pub digest: String,
    pub candidates: Vec<(String, CandidateIdentity)>,
    pub inventory: Vec<(String, String)>,
}

pub struct TransactionCatalog {
    versions: BTreeMap<(Group, String), Result<Vec<semver::Version>, DepProviderError>>,
    definitions: BTreeMap<Coordinate, CandidateMetadata>,
    inspection: CatalogInspection,
    capabilities: super::CapIndex,
}

impl TransactionCatalog {
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#M05C1-TRANSACTION"
    )]
    pub fn capture<P: VersionEnumerator + ?Sized>(
        provider: &P,
        roots: &[PackageRef],
    ) -> Result<Self, DepProviderError> {
        let mut versions = BTreeMap::new();
        let mut definitions = BTreeMap::new();
        let mut queue: VecDeque<_> = roots.iter().cloned().collect();
        while let Some(pkg) = queue.pop_front() {
            let group = pkg
                .group
                .ok_or_else(|| DepProviderError::Other("unqualified catalog coordinate".into()))?;
            let key = (group.clone(), pkg.name.to_string());
            if versions.contains_key(&key) {
                continue;
            }
            let mut candidates = match provider.list_versions(&group, pkg.name.as_str()) {
                Ok(versions) => versions,
                Err(error @ DepProviderError::Other(_)) => return Err(error),
                Err(error) => {
                    versions.insert(key, Err(error));
                    continue;
                }
            };
            candidates.sort();
            candidates.dedup();
            versions.insert(key, Ok(candidates.clone()));
            for version in candidates {
                let metadata = provider.candidate_metadata(&group, pkg.name.as_str(), &version)?;
                let meta = metadata
                    .manifest
                    .require_package()
                    .map_err(|e| DepProviderError::Other(e.to_string()))?;
                if meta.group != group || meta.name != pkg.name.as_str() || meta.version != version
                {
                    return Err(DepProviderError::Other(format!(
                        "corrupt catalog identity for {group}/{}@{version}",
                        pkg.name
                    )));
                }
                queue.extend(metadata.manifest.requires.packages.iter().cloned());
                queue.extend(metadata.manifest.recommends.packages.iter().cloned());
                for alternative in &metadata.manifest.requires_any {
                    queue.extend(alternative.one_of.iter().cloned());
                }
                for target in metadata.manifest.conditional_deps.values() {
                    queue.extend(target.dependencies.packages.iter().cloned());
                }
                definitions.insert((group.clone(), pkg.name.to_string(), version), metadata);
            }
        }
        let candidates: Vec<_> = definitions
            .iter()
            .map(|((g, n, v), m)| (format!("{g}/{n}@{v}"), m.identity.clone()))
            .collect();
        let inventory: Vec<_> = versions
            .iter()
            .map(|((group, name), result)| {
                (
                    format!("{group}/{name}"),
                    match result {
                        Ok(versions) => format!(
                            "available:{}",
                            versions
                                .iter()
                                .map(ToString::to_string)
                                .collect::<Vec<_>>()
                                .join(",")
                        ),
                        Err(error) => format!("absent:{error}"),
                    },
                )
            })
            .collect();
        let inspection = CatalogInspection {
            digest: vibe_registry::catalog_digest(&candidates, &inventory),
            inventory,
            candidates,
        };
        let mut capabilities = super::CapIndex::new();
        for ((group, name, version), metadata) in &definitions {
            for capability in &metadata.manifest.provides.capabilities {
                capabilities
                    .entry(capability.qualified())
                    .or_default()
                    .push(super::CapProvider {
                        group: group.clone(),
                        name: name.clone(),
                        version: version.clone(),
                        provided_version: crate::naive::capability_version_for_provider(
                            capability, version,
                        ),
                    });
            }
        }
        Ok(Self {
            versions,
            definitions,
            inspection,
            capabilities,
        })
    }

    pub fn inspect(&self) -> &CatalogInspection {
        &self.inspection
    }
    pub fn metadata(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<&CandidateMetadata, DepProviderError> {
        self.definitions
            .get(&(group.clone(), name.into(), version.clone()))
            .ok_or_else(|| {
                DepProviderError::Other(format!(
                    "candidate catalog unavailable for {group}/{name}@{version}"
                ))
            })
    }
}
impl DepProvider for TransactionCatalog {
    fn resolve_version(&self, pkg: &PackageRef) -> Result<semver::Version, DepProviderError> {
        let group = pkg
            .group
            .as_ref()
            .ok_or_else(|| DepProviderError::Other("unqualified catalog root".into()))?;
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
        Ok(self.metadata(group, name, version)?.manifest.clone())
    }
    fn candidate_metadata(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<CandidateMetadata, DepProviderError> {
        self.metadata(group, name, version).cloned()
    }
}
impl VersionEnumerator for TransactionCatalog {
    fn cached_capabilities(&self) -> Option<&super::CapIndex> {
        Some(&self.capabilities)
    }
    fn list_versions(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Vec<semver::Version>, DepProviderError> {
        self.versions
            .get(&(group.clone(), name.into()))
            .cloned()
            .unwrap_or_else(|| {
                Err(DepProviderError::Other(format!(
                    "package catalog unavailable for {group}/{name}"
                )))
            })
    }
}
impl DepProvider for &TransactionCatalog {
    fn resolve_version(&self, pkg: &PackageRef) -> Result<semver::Version, DepProviderError> {
        (**self).resolve_version(pkg)
    }
    fn fetch_manifest(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<Manifest, DepProviderError> {
        (**self).fetch_manifest(group, name, version)
    }
}
impl VersionEnumerator for &TransactionCatalog {
    fn cached_capabilities(&self) -> Option<&super::CapIndex> {
        Some(&self.capabilities)
    }
    fn list_versions(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Vec<semver::Version>, DepProviderError> {
        (**self).list_versions(group, name)
    }
}
