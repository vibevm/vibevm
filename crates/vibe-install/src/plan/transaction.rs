use super::*;
use vibe_registry::{InPlaceMaterialised, RegistryError};
use vibe_resolver::{ResolvedGraph, SolveError, resolvo_engine::CatalogInspection};

pub(super) struct PlanningSource<'a, S: ?Sized> {
    original: &'a S,
    frozen: Option<Box<dyn InstallSource + 'a>>,
}
impl<'a, S: InstallSource + ?Sized> PlanningSource<'a, S> {
    pub(super) fn capture_authored(
        original: &'a S,
        request: &vibe_resolver::resolvo_engine::request::NormalizedRequest,
        authored: bool,
    ) -> std::result::Result<Self, SolveError> {
        Ok(Self {
            original,
            frozen: if authored {
                original.begin_normalized_transaction(request)?
            } else {
                None
            },
        })
    }
    pub(super) fn capture_request(
        original: &'a S,
        request: &vibe_resolver::resolvo_engine::request::NormalizedRequest,
    ) -> std::result::Result<Self, SolveError> {
        Ok(Self {
            original,
            frozen: original.begin_normalized_transaction(request)?,
        })
    }
}
macro_rules! forward {
    ($self:ident, $method:ident($($arg:expr),*)) => {
        match &$self.frozen { Some(source) => source.$method($($arg),*), None => $self.original.$method($($arg),*) }
    };
}
impl<S: InstallSource + ?Sized> InstallSource for PlanningSource<'_, S> {
    fn begin_normalized_transaction(
        &self,
        request: &vibe_resolver::resolvo_engine::request::NormalizedRequest,
    ) -> std::result::Result<Option<Box<dyn InstallSource + '_>>, SolveError> {
        forward!(self, begin_normalized_transaction(request))
    }
    fn solve_conditional(
        &self,
        roots: &[PackageRef],
        edges: &[vibe_resolver::resolvo_engine::request::ConditionalEdge],
    ) -> std::result::Result<ResolvedGraph, SolveError> {
        forward!(self, solve_conditional(roots, edges))
    }
    fn normalized_request(
        &self,
    ) -> Option<&vibe_resolver::resolvo_engine::request::NormalizedRequest> {
        forward!(self, normalized_request())
    }
    fn route_resolution(&self) -> Option<vibe_resolver::resolvo_engine::request::RouteResolution> {
        forward!(self, route_resolution())
    }
    fn finish_route_projection(
        &self,
        graph: &ResolvedGraph,
    ) -> std::result::Result<(), SolveError> {
        forward!(self, finish_route_projection(graph))
    }
    fn catalog_manifest_of(&self, pkg: &PackageRef) -> std::result::Result<Manifest, SolveError> {
        forward!(self, catalog_manifest_of(pkg))
    }
    fn catalog_inspection(&self) -> Option<&CatalogInspection> {
        forward!(self, catalog_inspection())
    }
    fn validate_catalog_candidate(&self, pkg: &PackageRef) -> std::result::Result<(), SolveError> {
        forward!(self, validate_catalog_candidate(pkg))
    }
    fn solve(&self, roots: &[PackageRef]) -> std::result::Result<ResolvedGraph, SolveError> {
        forward!(self, solve(roots))
    }
    fn manifest_of(&self, pkg: &PackageRef) -> std::result::Result<Manifest, SolveError> {
        forward!(self, manifest_of(pkg))
    }
    fn solve_masked(
        &self,
        roots: &[PackageRef],
        blocked: &BTreeSet<(String, String)>,
    ) -> std::result::Result<ResolvedGraph, SolveError> {
        forward!(self, solve_masked(roots, blocked))
    }
    fn resolve_and_fetch(
        &self,
        pkg: &PackageRef,
        store: &Path,
        expected: Option<&str>,
    ) -> std::result::Result<CachedPackage, RegistryError> {
        forward!(self, resolve_and_fetch(pkg, store, expected))
    }
    fn resolve_and_fetch_with_progress(
        &self,
        pkg: &PackageRef,
        store: &Path,
        expected: Option<&str>,
        progress: &vibe_core::progress::Progress,
    ) -> std::result::Result<CachedPackage, RegistryError> {
        forward!(
            self,
            resolve_and_fetch_with_progress(pkg, store, expected, progress)
        )
    }
    fn materialise_in_place(
        &self,
        pkg: &PackageRef,
        slot: &Path,
    ) -> std::result::Result<InPlaceMaterialised, RegistryError> {
        forward!(self, materialise_in_place(pkg, slot))
    }
}
