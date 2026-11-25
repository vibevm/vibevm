//! Terminal observation wrappers around the registry walk's decision logic.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::*;

/// Why a caller performs the unchanged registry resolution walk.
///
/// This affects observations only, never selection or source policy.
///
/// ```
/// use vibe_registry::ResolutionPurpose;
/// let purpose = ResolutionPurpose::DownloadSource;
/// assert_ne!(purpose, ResolutionPurpose::VersionSelection);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionPurpose {
    /// Choose a version satisfying the requested constraint.
    VersionSelection,
    /// Locate the source from which package contents will be fetched.
    DownloadSource,
    /// Locate the source for an in-place checkout.
    CheckoutSource,
}

/// Evidence retained while the legacy walk runs, including tolerated refusals.
#[derive(Default)]
pub(super) struct VersionPresence {
    pub present: bool,
    pub indeterminate: bool,
}

impl VersionPresence {
    fn finish(
        self,
        result: Result<Vec<semver::Version>, RegistryError>,
    ) -> Result<Option<Vec<semver::Version>>, RegistryError> {
        match result {
            Ok(versions) => Ok(Some(versions)),
            // A failed selection still confirms a package exists. Do not hide
            // operational errors merely because an earlier source was empty.
            Err(RegistryError::NoMatchingVersion { .. }) => Ok(Some(Vec::new())),
            Err(RegistryError::UnknownPackage { .. }) if self.present && !self.indeterminate => {
                Ok(Some(Vec::new()))
            }
            Err(RegistryError::UnknownPackage { .. }) if !self.indeterminate => Ok(None),
            Err(RegistryError::PackageNotFoundEverywhere { attempts, .. })
                if !self.indeterminate
                    && attempts.iter().all(|attempt| {
                        matches!(attempt.status, WalkAttemptStatus::NotFound)
                    }) =>
            {
                Ok(self.present.then(Vec::new))
            }
            Err(error) => Err(error),
        }
    }
}

pub(super) fn versions(
    progress: &vibe_core::progress::Progress,
    group: &Group,
    name: &str,
    operation: impl FnOnce(
        &vibe_core::progress::ProgressTask,
    ) -> Result<Vec<semver::Version>, RegistryError>,
) -> Result<Vec<semver::Version>, RegistryError> {
    let task = progress.task(format!("Discovering versions for {group}/{name}"));
    let package = format!("{group}/{name}");
    let result = observed(&task, &package, || operation(&task));
    match &result {
        Ok(versions) => {
            task.detail(format!("{} version(s) available", versions.len()));
            task.finish();
        }
        Err(_) => task.fail("version discovery failed"),
    }
    result
}

pub(super) fn resolution(
    progress: &vibe_core::progress::Progress,
    pkgref: &PackageRef,
    purpose: ResolutionPurpose,
    operation: impl FnOnce(&vibe_core::progress::ProgressTask) -> Result<MultiResolution, RegistryError>,
) -> Result<MultiResolution, RegistryError> {
    let operation_label = match purpose {
        ResolutionPurpose::VersionSelection => "Selecting version for",
        ResolutionPurpose::DownloadSource => "Locating package download source for",
        ResolutionPurpose::CheckoutSource => "Locating in-place checkout source for",
    };
    let task = progress.task(format!("{operation_label} {pkgref}"));
    let package = pkgref.qualified_name();
    let result = observed(&task, &package, || operation(&task));
    match &result {
        Ok(resolution) => {
            task.detail(format!(
                "selected {}{}",
                resolution.resolved.version,
                resolution
                    .registry_name
                    .as_deref()
                    .map(|name| format!(" from registry {name}"))
                    .unwrap_or_default()
            ));
            task.finish();
        }
        Err(_) => task.fail("package resolution failed"),
    }
    result
}

pub(super) fn manifest(
    progress: &vibe_core::progress::Progress,
    group: &Group,
    name: &str,
    version: &semver::Version,
    operation: impl FnOnce(&vibe_core::progress::ProgressTask) -> Result<Manifest, RegistryError>,
) -> Result<Manifest, RegistryError> {
    let task = progress.task(format!(
        "Reading package metadata for {group}/{name}@{version}"
    ));
    let package = format!("{group}/{name}");
    let result = observed(&task, &package, || operation(&task));
    match &result {
        Ok(_) => task.finish(),
        Err(_) => task.fail("package metadata read failed"),
    }
    result
}

/// A clone the walk falls back to is a transfer of the observing task, for
/// the package it walks (`PROP-068#git`).
fn observed<T>(
    task: &vibe_core::progress::ProgressTask,
    package: &str,
    operation: impl FnOnce() -> T,
) -> T {
    crate::git_backend::transfers::observed(Some(&task.progress()), Some(package), None, operation)
}

impl MultiRegistryResolver {
    /// Enumerate versions while retaining package presence from the same walk.
    ///
    /// `Some`, including an empty vector, confirms presence; `None` means every
    /// consulted source and the existing resolution fallback confirmed absence.
    /// Tolerated public-auth refusals and offline/cache-only failures are not
    /// confirmed absence. Operational errors remain errors. Source priority,
    /// held pins and redirect selection are unchanged; no extra probe is made.
    ///
    /// ```no_run
    /// use vibe_core::Group;
    /// use vibe_registry::{MultiRegistryResolver, RegistryError};
    /// fn inspect(resolver: &MultiRegistryResolver) -> Result<(), RegistryError> {
    ///     let group = Group::parse("org.example").unwrap();
    ///     let found = resolver.list_versions_with_presence(&group, "demo")?;
    ///     let package_exists = found.is_some();
    ///     let versions = found.unwrap_or_default();
    ///     Ok(())
    /// }
    /// ```
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST"
    )]
    pub fn list_versions_with_presence(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Option<Vec<semver::Version>>, RegistryError> {
        let mut present = false;
        let versions = versions(&self.progress, group, name, |task| {
            let mut evidence = VersionPresence::default();
            let result = self.list_versions_observed(group, name, task, None, &mut evidence);
            evidence.finish(result).map(|found| {
                present = found.is_some();
                found.unwrap_or_default()
            })
        })?;
        Ok(present.then_some(versions))
    }

    /// Run the same resolution walk, identifying its caller's purpose.
    ///
    /// ```no_run
    /// use vibe_core::PackageRef;
    /// use vibe_registry::{MultiRegistryResolver, RegistryError, ResolutionPurpose};
    /// fn locate(resolver: &MultiRegistryResolver) -> Result<(), RegistryError> {
    ///     let pkg = PackageRef::parse("org.example/tool@=1.0.0").unwrap();
    ///     resolver.resolve_for(&pkg, ResolutionPurpose::DownloadSource)?;
    ///     Ok(())
    /// }
    /// ```
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-060#PACKAGE-RESOLUTION-WORK"
    )]
    pub fn resolve_for(
        &self,
        pkgref: &PackageRef,
        purpose: ResolutionPurpose,
    ) -> Result<MultiResolution, RegistryError> {
        self.resolve_for_with_progress(pkgref, purpose, &self.progress)
    }

    /// Observe a purpose-specific walk beneath the supplied caller scope.
    ///
    /// ```no_run
    /// use vibe_core::{PackageRef, progress::Progress};
    /// use vibe_registry::{MultiRegistryResolver, RegistryError, ResolutionPurpose};
    /// fn locate(resolver: &MultiRegistryResolver) -> Result<(), RegistryError> {
    ///     let task = Progress::default().task("fetch packages");
    ///     let pkg = PackageRef::parse("org.example/tool@=1.0.0").unwrap();
    ///     resolver.resolve_for_with_progress(
    ///         &pkg, ResolutionPurpose::DownloadSource, &task.progress(),
    ///     )?;
    ///     task.finish();
    ///     Ok(())
    /// }
    /// ```
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-060#PACKAGE-RESOLUTION-WORK"
    )]
    pub fn resolve_for_with_progress(
        &self,
        pkgref: &PackageRef,
        purpose: ResolutionPurpose,
        progress: &vibe_core::progress::Progress,
    ) -> Result<MultiResolution, RegistryError> {
        resolution(progress, pkgref, purpose, |task| {
            self.resolve_inner(pkgref, task)
        })
    }
}

#[cfg(test)]
#[path = "observation_tests.rs"]
mod tests;
