//! Read-only observations from the plan's held acquisitions, without a presentation envelope.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-EVIDENCE-PROJECT-EVENTS");
use super::PlannedInstall;
use vibe_wire::generated::shared::SourceEvidence;

impl PlannedInstall {
    /// Observe the final acquired closure in its held order. Coordinates and
    /// facts come from the same acquisition, never a zip with mutable resolution.
    ///
    /// ```
    /// # fn inspect(plan: &vibe_install::PlannedInstall) {
    /// for (group, name, version, facts) in plan.package_sources() {
    ///     let coordinate = format!("{group}/{name}@{version}");
    ///     assert!(!coordinate.is_empty());
    ///     let _observed_origin = facts.source_url;
    /// }
    /// # }
    /// ```
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-EVIDENCE-PROJECT-EVENTS"
    )]
    pub fn package_sources(
        &self,
    ) -> impl Iterator<Item = (&vibe_core::Group, &str, &semver::Version, SourceEvidence)> {
        self.fetched.iter().map(|fetched| {
            let package = &fetched.cached.resolved;
            let evidence = fetched.in_place.as_ref().map_or_else(
                || fetched.cached.source_evidence(),
                |prepared| prepared.source_evidence(),
            );
            (
                &package.group,
                package.name.as_str(),
                &package.version,
                evidence,
            )
        })
    }
}

impl PlannedInstall {
    /// Retained candidate reasons from the same metadata selection as acquisition.
    ///
    /// ```no_run
    /// # fn read(plan: &vibe_install::PlannedInstall, group: &vibe_core::Group) {
    /// let _reasons = plan.package_candidates(group, "demo");
    /// # }
    /// ```
    pub fn package_candidates(
        &self,
        group: &vibe_core::Group,
        name: &str,
    ) -> Option<&[vibe_registry::source_candidates::SourceCandidateObservation]> {
        self.source_candidates
            .get(&(group.clone(), name.to_owned()))
            .map(Vec::as_slice)
    }
}
