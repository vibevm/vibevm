//! Bind caller-admitted intent to the incumbent package-source factory.
//! No cells, providers, credentials or wire-supplied authority are created here.
use crate::ports::{PackageSource, PackageSourceBuild, PackageSourceFactory};
use std::{collections::BTreeSet, path::Path};
use vibe_core::{PackageRef, manifest::Lockfile};
use vibe_install::InstallSource;
use vibe_resolver::{SolveError, resolvo_engine::request::NormalizedRequest};

pub struct ConfiguredUpdateSource<'a> {
    pub factory: &'a dyn PackageSourceFactory,
    pub request: NormalizedRequest,
}
impl PackageSourceFactory for ConfiguredUpdateSource<'_> {
    fn build(&self, input: PackageSourceBuild<'_>) -> anyhow::Result<Box<dyn PackageSource>> {
        Ok(Box::new(Bound {
            source: self.factory.build(input)?,
            request: self.request.clone(),
        }))
    }
}
struct Bound {
    source: Box<dyn PackageSource>,
    request: NormalizedRequest,
}
impl PackageSource for Bound {
    fn qualify(&self, p: &PackageRef, lock: &Lockfile) -> anyhow::Result<PackageRef> {
        self.source.qualify(p, lock)
    }
    fn recheck_before_apply(
        &self,
        r: &[vibe_workspace::install::ResolvedDep],
    ) -> anyhow::Result<()> {
        self.source.recheck_before_apply(r)
    }
}
impl InstallSource for Bound {
    fn normalized_request(&self) -> Option<&NormalizedRequest> {
        Some(&self.request)
    }
    fn begin_normalized_transaction(
        &self,
        request: &NormalizedRequest,
    ) -> Result<Option<Box<dyn InstallSource + '_>>, SolveError> {
        if vibe_install::requested_record(request)
            .map_err(|e| SolveError::UnsupportedRoute(e.to_string()))?
            != vibe_install::requested_record(&self.request)
                .map_err(|e| SolveError::UnsupportedRoute(e.to_string()))?
        {
            return Err(SolveError::UnsupportedRoute(
                "update transaction changed admitted intent".into(),
            ));
        }
        let frozen = self.source.begin_normalized_transaction(request)?;
        if frozen.is_none() {
            return Err(unbound());
        }
        Ok(frozen)
    }
    fn solve(&self, _: &[PackageRef]) -> Result<vibe_resolver::ResolvedGraph, SolveError> {
        Err(unbound())
    }
    fn solve_masked(
        &self,
        _: &[PackageRef],
        _: &BTreeSet<(String, String)>,
    ) -> Result<vibe_resolver::ResolvedGraph, SolveError> {
        Err(unbound())
    }
    fn manifest_of(&self, _: &PackageRef) -> Result<vibe_core::manifest::Manifest, SolveError> {
        Err(unbound())
    }
    fn resolve_and_fetch(
        &self,
        _: &PackageRef,
        _: &Path,
        _: Option<&str>,
    ) -> Result<vibe_registry::CachedPackage, vibe_registry::RegistryError> {
        Err(vibe_registry::RegistryError::MalformedMeta {
            path: "<configured-update>".into(),
            reason: unbound().to_string(),
        })
    }
    fn materialise_in_place(
        &self,
        pin: &PackageRef,
        slot: &Path,
    ) -> Result<vibe_registry::InPlaceMaterialised, vibe_registry::RegistryError> {
        // Apply supplies the selected exact pin. This operation repairs or
        // refreshes materialization; it performs no option/default selection.
        self.source.materialise_in_place(pin, slot)
    }
}
fn unbound() -> SolveError {
    SolveError::UnsupportedRoute("configured update requires the incumbent normalized transaction; no legacy/default fallback".into())
}
