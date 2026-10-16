//! Transaction binding within the incumbent InstallResolver dispatch.
use super::*;
use vibe_core::manifest::Manifest;
use vibe_registry::CandidateMetadata;
use vibe_resolver::resolvo_engine::{CatalogInspection, TransactionCatalog};
use vibe_resolver::{DepProvider, DepProviderError, ResolvedGraph, SolveError, VersionEnumerator};

pub(super) struct CaptureProvider<'a> {
    pub source: &'a InstallResolver,
    pub provider: Box<dyn VersionEnumerator + 'a>,
}
impl DepProvider for CaptureProvider<'_> {
    fn resolve_version(
        &self,
        pkg: &PackageRef,
    ) -> Result<vibe_resolver::resolvo_engine::CandidateVersion, DepProviderError> {
        self.provider.resolve_version(pkg)
    }
    fn fetch_manifest(
        &self,
        group: &Group,
        name: &str,
        version: &vibe_resolver::resolvo_engine::CandidateVersion,
    ) -> Result<Manifest, DepProviderError> {
        Ok(self.candidate_metadata(group, name, version)?.manifest)
    }
    fn candidate_metadata(
        &self,
        group: &Group,
        name: &str,
        version: &vibe_resolver::resolvo_engine::CandidateVersion,
    ) -> Result<CandidateMetadata, DepProviderError> {
        self.source
            .capture_metadata(group, name, version)
            .map_err(provider_error)
    }
}
impl VersionEnumerator for CaptureProvider<'_> {
    fn list_versions(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Vec<vibe_resolver::resolvo_engine::CandidateVersion>, DepProviderError> {
        self.provider.list_versions(group, name)
    }
}

fn provider_error(error: RegistryError) -> DepProviderError {
    match error {
        RegistryError::UnknownPackage { group, name } => {
            DepProviderError::UnknownPackage { group, name }
        }
        RegistryError::NoMatchingVersion { group, name, req } => {
            DepProviderError::NoMatchingVersion {
                group,
                name,
                constraint: req,
            }
        }
        RegistryError::PackageNotFoundEverywhere {
            group,
            name,
            summary,
            attempts,
        } => DepProviderError::AggregateNotFound {
            group,
            name,
            summary,
            attempts,
        },
        error => DepProviderError::Other(error.to_string()),
    }
}

impl InstallResolver {
    fn capture_metadata(
        &self,
        group: &Group,
        name: &str,
        version: &vibe_resolver::resolvo_engine::CandidateVersion,
    ) -> Result<CandidateMetadata, RegistryError> {
        let local = |registry: &LocalRegistry| {
            let dir = registry
                .root()
                .join(group.as_str())
                .join(name)
                .join(format!("v{version}"));
            if !dir.is_dir() {
                return Err(RegistryError::UnknownPackage {
                    group: group.clone(),
                    name: name.into(),
                });
            }
            CandidateMetadata::read_local_registry(&dir)
        };
        match self {
            Self::Local(registry, _) => local(registry),
            Self::Multi(registry, _) => registry.fetch_candidate_metadata(group, name, version),
            Self::Embedded {
                locals,
                declared,
                precedence,
                ..
            } => {
                let family = || {
                    for registry in locals {
                        match local(registry) {
                            Err(e) if is_registry_absent(&e) => {}
                            result => return result,
                        }
                    }
                    Err(RegistryError::UnknownPackage {
                        group: group.clone(),
                        name: name.into(),
                    })
                };
                let remote = || match declared {
                    Some(registry) => registry.fetch_candidate_metadata(group, name, version),
                    None => Err(RegistryError::UnknownPackage {
                        group: group.clone(),
                        name: name.into(),
                    }),
                };
                let (first, second): (
                    &dyn Fn() -> Result<CandidateMetadata, RegistryError>,
                    &dyn Fn() -> Result<CandidateMetadata, RegistryError>,
                ) = match precedence {
                    vibe_resolver::EmbeddedPrecedence::EmbeddedFirst => (&family, &remote),
                    vibe_resolver::EmbeddedPrecedence::EmbeddedLast => (&remote, &family),
                };
                match first() {
                    Err(e) if is_registry_absent(&e) => second(),
                    result => result,
                }
            }
        }
    }
}

pub(super) struct FrozenSource<'a> {
    pub source: &'a InstallResolver,
    pub catalog: TransactionCatalog,
}
impl FrozenSource<'_> {
    fn metadata(&self, pkg: &PackageRef) -> Result<&CandidateMetadata, SolveError> {
        let version = self.catalog.resolve_version(pkg)?;
        let group = pkg
            .group
            .as_ref()
            .ok_or_else(|| DepProviderError::Other("unqualified catalog request".into()))?;
        Ok(self.catalog.metadata(group, pkg.name.as_str(), &version)?)
    }
}
impl InstallSource for FrozenSource<'_> {
    fn catalog_inspection(&self) -> Option<&CatalogInspection> {
        Some(self.catalog.inspect())
    }
    fn validate_catalog_candidate(&self, pkg: &PackageRef) -> Result<(), SolveError> {
        let captured = self.metadata(pkg)?;
        let p = captured
            .manifest
            .require_package()
            .map_err(|e| DepProviderError::Other(e.to_string()))?;
        let current = self
            .source
            .capture_metadata(&p.group, &p.name, &p.version)
            .map_err(provider_error)?;
        if current.identity != captured.identity {
            return Err(DepProviderError::Other(format!(
                "candidate catalog changed for {}",
                pkg.qualified_name()
            ))
            .into());
        }
        Ok(())
    }
    fn solve(&self, roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
        let (provider, solver) = self.source.cell_selection();
        crate::cells::catalog_solver(&selection_flags(provider, solver), &self.catalog).solve(roots)
    }
    fn manifest_of(&self, pkg: &PackageRef) -> Result<Manifest, SolveError> {
        Ok(self.metadata(pkg)?.manifest.clone())
    }
    fn solve_masked(
        &self,
        roots: &[PackageRef],
        blocked: &BTreeSet<(String, String)>,
    ) -> Result<ResolvedGraph, SolveError> {
        let (_, solver) = self.source.cell_selection();
        vibe_install::solve_with_visibility_mask(&self.catalog, solver, roots, blocked)
    }
    fn resolve_and_fetch(
        &self,
        pkg: &PackageRef,
        store: &Path,
        expected: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        self.fetch_bound(pkg, store, expected, None)
    }
    fn resolve_and_fetch_with_progress(
        &self,
        pkg: &PackageRef,
        store: &Path,
        expected: Option<&str>,
        progress: &vibe_core::progress::Progress,
    ) -> Result<CachedPackage, RegistryError> {
        self.fetch_bound(pkg, store, expected, Some(progress))
    }
    fn materialise_in_place(
        &self,
        pkg: &PackageRef,
        slot: &Path,
    ) -> Result<vibe_registry::InPlaceMaterialised, RegistryError> {
        self.source.materialise_in_place(pkg, slot)
    }
}

impl FrozenSource<'_> {
    fn fetch_bound(
        &self,
        pkg: &PackageRef,
        store: &Path,
        expected: Option<&str>,
        progress: Option<&vibe_core::progress::Progress>,
    ) -> Result<CachedPackage, RegistryError> {
        self.validate_catalog_candidate(pkg)
            .map_err(|e| RegistryError::MalformedMeta {
                path: "<transaction-catalog>".into(),
                reason: e.to_string(),
            })?;
        let captured = self
            .metadata(pkg)
            .map_err(|e| RegistryError::MalformedMeta {
                path: "<transaction-catalog>".into(),
                reason: e.to_string(),
            })?;
        let cached =
            self.source
                .resolve_and_fetch_inner(pkg, store, expected, progress, Some(captured))?;
        let matches_content = if let Some(commit) = captured.identity.content.strip_prefix("git:") {
            cached.resolved_commit.as_deref() == Some(commit)
        } else {
            cached.content_hash == captured.identity.content
        };
        if !matches_content || cached.manifest != captured.manifest {
            return Err(RegistryError::MalformedMeta {
                path: cached.cache_dir,
                reason: "fetched content differs from immutable transaction catalog".into(),
            });
        }
        Ok(cached)
    }
}
