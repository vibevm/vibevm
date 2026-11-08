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
                return Err(manifest_drift(&cached.cache_dir));
            }
            require_local_copy(&cached)?;
        }
        Ok(cached)
    }
}
impl<S: InstallSource + ?Sized> InstallSource for PlanningSource<'_, S> {
    fn source_solve_limits(&self) -> Result<vibe_resolver::SourceSolveLimits, SolveError> {
        self.source.source_solve_limits()
    }
    fn solve_metadata(
        &self,
        request: &vibe_resolver::SourceSolveRequest<'_>,
    ) -> Result<vibe_resolver::SourceResolvedGraph, SolveError> {
        let bound = vibe_resolver::SourceSolveRequest {
            roots: request.roots,
            held_roots: request.held_roots,
            authoritative_locked: request.authoritative_locked,
            activated: request.activated,
            limits: request.limits,
            blocked: request.blocked,
            configuration: self.configuration.as_ref().or(request.configuration),
        };
        self.source.solve_metadata(&bound)
    }
    fn acquire_candidate_for_install(
        &self,
        candidate: &vibe_registry::source_candidates::RetainedSourceCandidate,
        store_root: &Path,
        expected_hash: Option<&str>,
        slot: &Path,
        progress: &vibe_core::progress::Progress,
    ) -> Result<vibe_registry::PackageAcquisition, RegistryError> {
        // The same refusal `prepare_in_place` gives, raised before any source
        // acquisition so no checkout, clone or slot is touched first.
        if self.configuration.is_some()
            && candidate
                .manifest()
                .package
                .as_ref()
                .is_some_and(|meta| meta.materialization.is_in_place())
        {
            return Err(in_place_unsupported(&format!(
                "{}/{}",
                candidate.group(),
                candidate.name()
            )));
        }
        self.source.acquire_candidate_for_install(
            candidate,
            store_root,
            expected_hash,
            slot,
            progress,
        )
    }

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
    fn replays_locked_source(&self, pkgref: &PackageRef) -> bool {
        self.source.replays_locked_source(pkgref)
    }
    fn prepare_in_place(
        &self,
        p: &PackageRef,
        slot: &Path,
    ) -> Result<PreparedInPlace, RegistryError> {
        if self.configuration.is_some() {
            return Err(in_place_unsupported(&p.qualified_name()));
        }
        self.source.prepare_in_place(p, slot)
    }
    fn acquire_for_install(
        &self,
        p: &PackageRef,
        store_root: &Path,
        expected_hash: Option<&str>,
        slot: &Path,
        progress: &vibe_core::progress::Progress,
    ) -> Result<vibe_registry::PackageAcquisition, RegistryError> {
        if self.configuration.is_some() {
            self.resolve_and_fetch_with_progress(p, store_root, expected_hash, progress)
                .map(|cached| vibe_registry::PackageAcquisition::Cached(Box::new(cached)))
        } else {
            self.source
                .acquire_for_install(p, store_root, expected_hash, slot, progress)
        }
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

fn in_place_unsupported(package: &str) -> RegistryError {
    PackageConfigurationError::Unsupported {
        package: package.to_owned(),
        reason: "configured local-copy operation does not support incremental in-place preparation"
            .into(),
    }
    .into()
}

pub(super) fn mixed_flags() -> crate::Error {
    PackageConfigurationError::Unsupported {package:"workspace dependency graph".into(),reason:"legacy feature flags cannot override authored common input; edit the actual requires declaration".into()}.into()
}

/// The one refusal for a payload whose actual manifest is not the candidate
/// metadata the plan selected: planning meaning cannot be replaced by bytes
/// observed later, so the caller must replan before any effect.
pub(super) fn manifest_drift(path: &Path) -> RegistryError {
    RegistryError::MalformedMeta {
        path: path.to_owned(),
        reason: "actual fetched manifest differs from the selected candidate metadata; replan before effects".into(),
    }
}

/// The retained metadata path preserves configured acquisition's existing
/// bound. The caller has already refused manifest drift via [`manifest_drift`].
pub(super) fn admit_configured_acquisition(
    cached: &CachedPackage,
    metadata: &vibe_resolver::CandidateMetadata,
) -> Result<(), RegistryError> {
    if metadata.configuration_admission
        == vibe_resolver::source_candidates::ConfigurationAdmission::WorkspaceInputs
    {
        require_local_copy(cached)?;
    }
    Ok(())
}
fn require_local_copy(cached: &CachedPackage) -> Result<(), RegistryError> {
    if !cached.source_uri.starts_with("file://")
        || cached.is_git_source
        || cached.is_path_source
        || cached.overridden
        || cached.is_embedded
        || cached.package_meta().materialization != vibe_core::manifest::Materialization::Copy
    {
        return Err(PackageConfigurationError::Unsupported {
            package: format!("{}/{}", cached.resolved.group, cached.resolved.name),
            reason:
                "configured installation currently requires ordinary local-registry copy sources"
                    .into(),
        }
        .into());
    }
    Ok(())
}
