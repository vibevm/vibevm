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

impl PlannedInstall {
    /// Immutable prospective rows from this plan's acquired manifests and root
    /// witnesses. No discovery, solving, publication or writer permission.
    ///
    /// A plan whose acquired packages and resolution rows no longer name the
    /// same coordinates is refused here, before any row is presented.
    ///
    /// ```no_run
    /// # fn admit(plan: &vibe_install::PlannedInstall) -> Result<(), vibe_install::Error> {
    /// let (packages, roots) = plan.admission_rows()?;
    /// assert!(roots.len() <= packages.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn admission_rows(
        &self,
    ) -> crate::error::Result<(
        Vec<vibe_core::manifest::LockedPackage>,
        Vec<vibe_core::PackageRef>,
    )> {
        let mut lock = self.lockfile.clone();
        let language = self
            .language_chain
            .first()
            .map(String::as_str)
            .filter(|l| *l != "en");
        lock.packages = self
            .fetched
            .iter()
            .map(|f| {
                let resolved = self
                    .resolution
                    .iter()
                    .find(|row| {
                        row.group == f.cached.resolved.group && row.name == f.cached.resolved.name
                    })
                    .ok_or_else(|| {
                        crate::Error::Solve(
                            vibe_resolver::DepProviderError::Other(format!(
                                "acquired package `{}/{}` has no matching resolution row \
                                 (violates spec://org.vibevm.core/vibevm/common/PROP-050#resolver; \
                                  fix: report the mismatched Fetched/ResolvedDep sets in vibe-install)",
                                f.cached.resolved.group, f.cached.resolved.name
                            ))
                            .into(),
                        )
                    })?;
                Ok(crate::record::locked_package_from_fetched(
                    f, resolved, language,
                ))
            })
            .collect::<crate::error::Result<_>>()?;
        crate::record::record_app_requirements(&mut lock, &self.fetched);
        let mut roots = Vec::new();
        for root in self.node_roots.values().flatten() {
            if !roots
                .iter()
                .any(|r: &vibe_core::PackageRef| r.group == root.group && r.name == root.name)
            {
                roots.push(root.clone());
            }
        }
        Ok((lock.packages, roots))
    }
}
