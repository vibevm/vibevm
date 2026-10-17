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
    pub request: vibe_resolver::resolvo_engine::request::NormalizedRequest,
    pub resolution:
        std::cell::RefCell<Option<vibe_resolver::resolvo_engine::request::RouteResolution>>,
    pub commitments: std::cell::RefCell<Vec<vibe_resolver::resolvo_engine::SelectedChoice>>,
    pub conditional:
        std::cell::RefCell<Vec<vibe_resolver::resolvo_engine::request::ConditionalEdge>>,
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
    fn begin_normalized_transaction(
        &self,
        request: &vibe_resolver::resolvo_engine::request::NormalizedRequest,
    ) -> Result<Option<Box<dyn InstallSource + '_>>, SolveError> {
        if request.features.explicit != self.request.features.explicit
            || request.features.no_defaults != self.request.features.no_defaults
            || request.features.all != self.request.features.all
        {
            return Err(SolveError::UnsupportedRoute(
                "planner feature intent differs from the admitted transaction".into(),
            ));
        }
        if request
            .roots
            .iter()
            .any(|r| !self.request.roots.contains(r))
        {
            return Err(SolveError::UnsupportedRoute(
                "planner roots differ from the admitted transaction".into(),
            ));
        }
        Ok(None)
    }
    fn solve_conditional(
        &self,
        roots: &[PackageRef],
        edges: &[vibe_resolver::resolvo_engine::request::ConditionalEdge],
    ) -> Result<ResolvedGraph, SolveError> {
        if self.request.configuration.is_some() && !edges.is_empty() {
            return Err(SolveError::UnsupportedRoute(
                "conditional consumer configuration is not complete".into(),
            ));
        }
        *self.conditional.borrow_mut() = edges.to_vec();
        self.solve(roots)
    }
    fn normalized_request(
        &self,
    ) -> Option<&vibe_resolver::resolvo_engine::request::NormalizedRequest> {
        Some(&self.request)
    }
    fn route_resolution(&self) -> Option<vibe_resolver::resolvo_engine::request::RouteResolution> {
        self.resolution.borrow().clone()
    }
    fn finish_route_projection(&self, graph: &ResolvedGraph) -> Result<(), SolveError> {
        let mut state = self.resolution.borrow_mut();
        let resolution = state
            .as_mut()
            .ok_or_else(|| SolveError::UnsupportedRoute("no selected route to project".into()))?;
        for node in graph.iter() {
            let original = resolution
                .recorded
                .graph
                .find(&node.group, &node.name)
                .ok_or_else(|| {
                    SolveError::UnsupportedRoute("projection introduced a package".into())
                })?;
            if node.version != original.version
                || node.is_root != original.is_root
                || node
                    .dependencies
                    .iter()
                    .any(|d| !original.dependencies.contains(d))
            {
                return Err(SolveError::UnsupportedRoute(
                    "projection introduced an inconsistent selected edge".into(),
                ));
            }
        }
        if self.request.roots.iter().any(|r| {
            r.group
                .as_ref()
                .and_then(|g| graph.find(g, r.name.as_str()))
                .is_none()
        }) {
            return Err(SolveError::UnsupportedRoute(
                "projection removed an original consumer root".into(),
            ));
        }
        let retained = |e: &vibe_resolver::resolvo_engine::SelectedChoice| {
            graph.iter().any(|n| {
                e.parent.group.as_ref() == Some(&n.group)
                    && e.parent.name == n.name
                    && n.dependencies.iter().any(|d| {
                        d.group == e.provider.group
                            && d.name == e.provider.name
                            && d.version == e.provider.version
                    })
            })
        };
        resolution.recorded.choices.retain(retained);
        resolution.provenance.retain(|e| retained(&e.selected));
        resolution.manifests.retain(|key, _| {
            graph
                .iter()
                .any(|n| *key == format!("{}/{}@{}", n.group, n.name, n.version))
        });
        resolution
            .effects
            .retain(|key, _| resolution.manifests.contains_key(key));
        resolution.recorded.graph = graph.clone();
        Ok(())
    }
    fn catalog_manifest_of(&self, pkg: &PackageRef) -> Result<Manifest, SolveError> {
        Ok(self.metadata(pkg)?.manifest.clone())
    }

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
        self.solve_masked(roots, &BTreeSet::new())
    }
    fn manifest_of(&self, pkg: &PackageRef) -> Result<Manifest, SolveError> {
        // Metadata is part of this exact resolution, including configured
        // induced requirements. Never replace it with a legacy reconstruction.
        if self.resolution.borrow().is_none() {
            self.solve(&self.request.roots)?;
        }
        let resolution = self.resolution.borrow();
        let resolution = resolution.as_ref().expect("successful solve recorded");
        let node = pkg
            .group
            .as_ref()
            .and_then(|g| resolution.recorded.graph.find(g, pkg.name.as_str()))
            .filter(|n| pkg.version.matches(&n.version))
            .ok_or_else(|| {
                SolveError::UnsupportedRoute(
                    "metadata request is not in the selected resolution".into(),
                )
            })?;
        Ok(resolution.manifests[&format!("{}/{}@{}", node.group, node.name, node.version)].clone())
    }
    fn solve_masked(
        &self,
        roots: &[PackageRef],
        blocked: &BTreeSet<(String, String)>,
    ) -> Result<ResolvedGraph, SolveError> {
        if self.resolution.borrow().is_none() && !blocked.is_empty() {
            self.solve(roots)?;
        }
        let retained = self.commitments.borrow().clone();
        let (provider, solver) = self.source.cell_selection();
        let mut combined = roots.to_vec();
        let conditional = self.conditional.borrow();
        combined.extend(
            conditional
                .iter()
                .filter(|e| {
                    !blocked.contains(&(e.parent.qualified_name(), e.dependency.qualified_name()))
                })
                .map(|e| e.dependency.clone()),
        );
        let result = crate::cells::solve_catalog_request(&selection_flags(provider,solver), &self.catalog, &self.request, &combined, blocked, &retained)
            .and_then(|mut resolution| {
                for node in &mut resolution.recorded.graph.packages {
                    node.is_root = self.request.roots.iter().chain(roots).any(|r| r.group.as_ref() == Some(&node.group) && r.name == node.name);
                }
                for edge in conditional.iter().filter(|e| !blocked.contains(&(e.parent.qualified_name(), e.dependency.qualified_name()))) {
                    let parent = resolution.recorded.graph.iter().find(|n| edge.parent.group.as_ref() == Some(&n.group) && edge.parent.name == n.name && edge.parent.version.matches(&n.version));
                    if parent.is_none() { return Err(SolveError::UnsupportedRoute("conditional declaration candidate changed; complete conditional candidate search is unsupported".into())); }
                    let target = resolution.recorded.graph.iter().find(|n| edge.dependency.group.as_ref() == Some(&n.group) && edge.dependency.name == n.name && edge.dependency.version.matches(&n.version))
                        .ok_or_else(|| SolveError::UnsupportedRoute("conditional constraint was not preserved".into()))?;
                    let provider = PackageRef::parse(&format!("{}/{}@={}", target.group,target.name,target.version)).expect("selected coordinate");
                    let parent = resolution.recorded.graph.packages.iter_mut().find(|n| edge.parent.group.as_ref() == Some(&n.group) && edge.parent.name == n.name).expect("checked parent");
                    if !parent.dependencies.contains(&provider) { parent.dependencies.push(provider.clone()); }
                    resolution.recorded.choices.push(vibe_resolver::resolvo_engine::SelectedChoice { parent: edge.parent.clone(), declaration: edge.declaration.clone(), alternative: 0, provider });
                }
                let mut rebound = vibe_resolver::resolvo_engine::request::RouteResolution::bind(&self.catalog, &self.request, resolution.recorded, resolution.manifests, resolution.effects)?;
                rebound.rejected = resolution.rejected;
                Ok(rebound)
            });
        match result {
            Ok(resolution) => {
                let graph = resolution.recorded.graph.clone();
                let mut commitments = self.commitments.borrow_mut();
                for edge in &resolution.recorded.choices {
                    if !commitments
                        .iter()
                        .any(|e| e.parent == edge.parent && e.declaration == edge.declaration)
                    {
                        commitments.push(edge.clone());
                    }
                }
                *self.resolution.borrow_mut() = Some(resolution);
                Ok(graph)
            }
            Err(error) => {
                *self.resolution.borrow_mut() = None;
                match error {
                    SolveError::Unsatisfiable { .. }
                    | SolveError::VersionConflict { .. }
                    | SolveError::KindMismatch { .. } => Err(SolveError::RootConstraints {
                        origins: self.request.root_origins.clone(),
                        conflict: Box::new(error),
                    }),
                    error => Err(error),
                }
            }
        }
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
