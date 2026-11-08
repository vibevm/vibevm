//! Observer, confirmation and source ports for the offline all-owner fixture.

use super::*;

pub(super) struct Harness;
impl RegistryEnvironment for Harness {
    fn prepare(&self) -> anyhow::Result<RegistryEnvironmentSnapshot> {
        Ok(RegistryEnvironmentSnapshot {
            embedded_root: None,
            global: vibe_core::GlobalRegistryConfig::load()?.into(),
        })
    }
}
impl PackageSourceFactory for Harness {
    fn build(&self, _: PackageSourceBuild<'_>) -> anyhow::Result<Box<dyn PackageSource>> {
        Ok(Box::new(DummySource))
    }
}

struct DummySource;

impl PackageSource for DummySource {
    fn qualify(&self, pkgref: &PackageRef, _locked: &Lockfile) -> anyhow::Result<PackageRef> {
        Ok(pkgref.clone())
    }
}

impl vibe_install::InstallSource for DummySource {
    fn source_solve_limits(
        &self,
    ) -> Result<vibe_resolver::SourceSolveLimits, vibe_resolver::SolveError> {
        unreachable!("fresh fixture never solves")
    }
    fn solve_metadata(
        &self,
        _: &vibe_resolver::SourceSolveRequest<'_>,
    ) -> Result<vibe_resolver::SourceResolvedGraph, vibe_resolver::SolveError> {
        unreachable!("fresh fixture never solves")
    }
    fn acquire_candidate_for_install(
        &self,
        _: &vibe_registry::source_candidates::RetainedSourceCandidate,
        _: &Path,
        _: Option<&str>,
        _: &Path,
        _: &vibe_core::progress::Progress,
    ) -> Result<vibe_registry::PackageAcquisition, vibe_registry::RegistryError> {
        unreachable!("fresh fixture never fetches")
    }
    fn resolve_and_fetch(
        &self,
        _: &PackageRef,
        _: &Path,
        _: Option<&str>,
    ) -> Result<vibe_registry::CachedPackage, vibe_registry::RegistryError> {
        unreachable!("fresh fixture never fetches")
    }
    fn solve(
        &self,
        _: &[PackageRef],
    ) -> Result<vibe_resolver::ResolvedGraph, vibe_resolver::SolveError> {
        unreachable!("fresh fixture never solves")
    }
    fn manifest_of(
        &self,
        _: &PackageRef,
    ) -> Result<vibe_core::manifest::Manifest, vibe_resolver::SolveError> {
        unreachable!("fresh fixture never reads source metadata")
    }
    fn solve_masked(
        &self,
        _: &[PackageRef],
        _: &std::collections::BTreeSet<(String, String)>,
    ) -> Result<vibe_resolver::ResolvedGraph, vibe_resolver::SolveError> {
        unreachable!("fresh fixture never solves masked")
    }
    fn materialise_in_place(
        &self,
        _: &PackageRef,
        _: &Path,
    ) -> Result<vibe_registry::InPlaceMaterialised, vibe_registry::RegistryError> {
        unreachable!("fresh fixture never materialises in place")
    }
}
impl ConfirmGate for Harness {
    fn confirm_install(&self, _: usize) -> anyhow::Result<()> {
        Ok(())
    }
}
impl RunObserver for Harness {
    fn stream_mode(&self) -> StreamMode {
        StreamMode::Null
    }
    fn binary_quiet(&self) -> bool {
        true
    }
    fn emit_machine_failure(&self) -> bool {
        false
    }
    fn observe_plan(&self, _: &crate::RitualPlan, _: &RunMetadata, _: bool) -> anyhow::Result<()> {
        Ok(())
    }
    fn observe_contribution(
        &self,
        _: &vibe_wire::generated::lifecycle_report::LifecycleContributionReport,
    ) {
    }
    fn observe_untracked_failure(
        &self,
        _: &RunMetadata,
        _: &str,
        _: &[vibe_wire::generated::lifecycle_report::LifecycleContributionReport],
    ) -> anyhow::Result<()> {
        Ok(())
    }
}
impl InstallObserver for Harness {
    fn stream_mode(&self) -> StreamMode {
        StreamMode::Null
    }
    fn emit_machine_failure(&self) -> bool {
        false
    }
    fn narrate(&self, _: crate::ports::InstallNarration<'_>) {}
    fn lane_sizes(&self, _: &Path) -> Vec<(String, Option<u64>)> {
        Vec::new()
    }
    fn plan_events(&self) -> &dyn vibe_install::PlanObserver {
        self
    }
    fn slot_observer(&self, _: &RunMetadata) -> Arc<dyn vibe_install::SlotLifecycleObserver> {
        Arc::new(Harness)
    }
}
impl vibe_install::PlanObserver for Harness {
    fn on(&self, _: vibe_install::PlanEvent) {}
}
impl vibe_install::SlotLifecycleObserver for Harness {
    fn observe(&self, _: &vibe_install::SlotLifecyclePlan) -> Result<(), String> {
        Ok(())
    }
    fn outcome(&self, _: &vibe_install::SlotLifecycleReport) -> Result<(), String> {
        Ok(())
    }
}
