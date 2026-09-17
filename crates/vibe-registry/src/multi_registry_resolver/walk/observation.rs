//! Terminal observation wrappers around the registry walk's decision logic.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::*;

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
    operation: impl FnOnce(&vibe_core::progress::ProgressTask) -> Result<MultiResolution, RegistryError>,
) -> Result<MultiResolution, RegistryError> {
    let task = progress.task(format!("Resolving {}", pkgref.qualified_name()));
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
