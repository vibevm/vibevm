//! One planning source keeps configured candidate observations through the
//! incumbent solve/projection/fetch pipeline. It never edits source manifests.
use crate::InstallSource;
use std::{collections::BTreeSet, path::Path};
use vibe_core::{
    PackageRef,
    manifest::{Manifest, PackageConfiguration, PackageConfigurationError},
};
use vibe_registry::{CachedPackage, InPlaceMaterialised, PreparedInPlace, RegistryError};
use vibe_resolver::{
    ResolvedGraph, ResolvedNode, SolveError, configuration::ConfigurationSnapshot,
};

pub(super) struct PlanningSource<'a, S: ?Sized> {
    pub source: &'a S,
    pub configuration: Option<ConfigurationSnapshot>,
}
impl<S: InstallSource + ?Sized> PlanningSource<'_, S> {
    fn check_fetch(&self, cached: CachedPackage) -> Result<CachedPackage, RegistryError> {
        if let Some(snapshot) = &self.configuration {
            let original = snapshot
                .original(
                    &cached.resolved.group,
                    &cached.resolved.name,
                    &cached.resolved.version,
                )
                .ok_or_else(|| RegistryError::MalformedMeta {
                    path: cached.cache_dir.clone(),
                    reason: "fetched package was not observed by the configured solver".into(),
                })?;
            if original != cached.manifest {
                return Err(RegistryError::MalformedMeta {path:cached.cache_dir.clone(),reason:"actual fetched manifest differs from configured candidate; replan before effects".into()});
            }
            if !cached.source_uri.starts_with("file://")
                || cached.is_git_source
                || cached.is_path_source
                || cached.overridden
                || cached.is_embedded
                || cached.package_meta().materialization
                    != vibe_core::manifest::Materialization::Copy
            {
                return Err(PackageConfigurationError::Unsupported {package:format!("{}/{}",cached.resolved.group,cached.resolved.name),reason:"configured installation currently requires ordinary local-registry copy sources".into()}.into());
            }
        }
        Ok(cached)
    }
}
impl<S: InstallSource + ?Sized> InstallSource for PlanningSource<'_, S> {
    fn resolve_and_fetch(
        &self,
        p: &PackageRef,
        store: &Path,
        expected: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        self.check_fetch(self.source.resolve_and_fetch(p, store, expected)?)
    }
    fn resolve_and_fetch_with_progress(
        &self,
        p: &PackageRef,
        store: &Path,
        expected: Option<&str>,
        progress: &vibe_core::progress::Progress,
    ) -> Result<CachedPackage, RegistryError> {
        self.check_fetch(
            self.source
                .resolve_and_fetch_with_progress(p, store, expected, progress)?,
        )
    }
    fn solve(&self, roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
        match &self.configuration {
            Some(snapshot) => self
                .source
                .solve_configured(roots, snapshot, &BTreeSet::new()),
            None => self.source.solve(roots),
        }
    }
    fn solve_masked(
        &self,
        roots: &[PackageRef],
        blocked: &BTreeSet<(String, String)>,
    ) -> Result<ResolvedGraph, SolveError> {
        match &self.configuration {
            Some(snapshot) => self.source.solve_configured(roots, snapshot, blocked),
            None => self.source.solve_masked(roots, blocked),
        }
    }
    fn manifest_of(&self, p: &PackageRef) -> Result<Manifest, SolveError> {
        match &self.configuration {
            Some(snapshot) => snapshot.manifest(p),
            None => self.source.manifest_of(p),
        }
    }
    fn configuration_for_node(
        &self,
        node: &ResolvedNode,
    ) -> Result<Option<PackageConfiguration>, SolveError> {
        match &self.configuration {
            Some(snapshot) => snapshot.configuration(&node.group, &node.name, &node.version),
            None => Ok(None),
        }
    }
    fn prepare_in_place(
        &self,
        p: &PackageRef,
        slot: &Path,
    ) -> Result<PreparedInPlace, RegistryError> {
        if self.configuration.is_some() {
            return Err(PackageConfigurationError::Unsupported {package:p.qualified_name(),reason:"configured local-copy operation does not support incremental in-place preparation".into()}.into());
        }
        self.source.prepare_in_place(p, slot)
    }
    fn materialise_in_place(
        &self,
        p: &PackageRef,
        slot: &Path,
    ) -> Result<InPlaceMaterialised, RegistryError> {
        if self.configuration.is_some() {
            return Err(PackageConfigurationError::Unsupported {
                package: p.qualified_name(),
                reason:
                    "configured local-copy operation does not support raw in-place materialization"
                        .into(),
            }
            .into());
        }
        self.source.materialise_in_place(p, slot)
    }
}

pub(super) fn mixed_flags() -> crate::Error {
    PackageConfigurationError::Unsupported {package:"workspace dependency graph".into(),reason:"legacy feature flags cannot override authored common input; edit the actual requires declaration".into()}.into()
}
