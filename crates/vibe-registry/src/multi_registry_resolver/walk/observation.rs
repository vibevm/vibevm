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

pub(super) fn versions(
    progress: &vibe_core::progress::Progress,
    group: &Group,
    name: &str,
    operation: impl FnOnce(
        &vibe_core::progress::ProgressTask,
    ) -> Result<Vec<semver::Version>, RegistryError>,
) -> Result<Vec<semver::Version>, RegistryError> {
    let task = progress.task(format!("Discovering versions for {group}/{name}"));
    let result = operation(&task);
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
    let result = operation(&task);
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
    let result = operation(&task);
    match &result {
        Ok(_) => task.finish(),
        Err(_) => task.fail("package metadata read failed"),
    }
    result
}

impl MultiRegistryResolver {
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
