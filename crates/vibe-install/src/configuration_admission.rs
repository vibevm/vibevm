//! Unsupported package configuration is refused at its actual consumer boundary.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION");

use crate::error::{Error, Result};
use std::path::Path;
use vibe_core::{PackageRef, manifest::Manifest};
use vibe_workspace::Workspace;

/// Admit every real workspace consumer and the raw selected snapshot before
/// source edits, migration, freshness, solving or boot publication. A workspace
/// package's own interface declaration is not its dependency configuration.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
pub fn workspace_inputs(
    workspace: &Workspace,
    selected_root: &Path,
    raw_selected: &Manifest,
) -> Result<()> {
    let selected =
        workspace
            .node_rel_of(selected_root)
            .ok_or_else(|| Error::SelectedConsumerMissing {
                selected: selected_root.display().to_string(),
                workspace: workspace.root.display().to_string(),
            })?;
    for (consumer, manifest) in workspace
        .iter_nodes()
        .chain(std::iter::once((selected.as_str(), raw_selected)))
    {
        if let Some(dependency) = manifest.requires.configurations.keys().next() {
            return Err(Error::ConfigurationInputUnsupported {
                consumer: format!("{consumer}/vibe.toml"),
                dependency: dependency.clone(),
            });
        }
    }
    Ok(())
}

/// Admit the actual manifest consumed as a package, before legacy projection or
/// project effects. Selected installer configuration belongs to the app owner.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
pub fn package_manifest(package: &PackageRef, manifest: &Manifest) -> Result<()> {
    manifest_named(package.qualified_name(), manifest)
}
fn manifest_named(package: String, manifest: &Manifest) -> Result<()> {
    if manifest.configuration.is_some() {
        return Err(Error::ConfigurationUnsupported { package });
    }
    if let Some(dependency) = manifest.requires.configurations.keys().next() {
        return Err(Error::ConfigurationInputUnsupported {
            consumer: format!("package {package}"),
            dependency: dependency.clone(),
        });
    }
    Ok(())
}

/// Admit a complete consumed package projection before its first effect.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
pub fn resolved_packages(packages: &[vibe_workspace::install::ResolvedDep]) -> Result<()> {
    for package in packages {
        manifest_named(
            format!("{}/{}", package.group, package.name),
            &package.manifest,
        )?;
    }
    Ok(())
}

pub(crate) fn planned(planned: &crate::PlannedInstall) -> Result<()> {
    workspace_inputs(&planned.workspace, &planned.project_root, &planned.manifest)?;
    resolved_packages(&planned.resolution)?;
    for fetched in &planned.fetched {
        let package = format!(
            "{}/{}",
            fetched.cached.resolved.group, fetched.cached.resolved.name
        );
        manifest_named(package.clone(), &fetched.cached.manifest)?;
        if let Some(prepared) = &fetched.in_place {
            manifest_named(package, prepared.manifest())?;
        }
    }
    Ok(())
}

/// Read only the lock-selected materialized providers when a Fresh migration
/// would publish authored inputs without going through normal fetching.
pub(crate) fn locked_packages(
    workspace: &Workspace,
    lock: &vibe_core::manifest::Lockfile,
) -> Result<()> {
    for package in &lock.packages {
        let slot = if package.materialization.is_in_place() {
            vibe_workspace::vibedeps::in_place_slot_abs_path(
                &workspace.root,
                &package.group,
                &package.name,
            )
        } else {
            vibe_workspace::vibedeps::slot_abs_path(
                &workspace.root,
                &package.group,
                &package.name,
                &package.version,
            )
        };
        let manifest = Manifest::read(slot.join(Manifest::FILENAME))?;
        manifest_named(format!("{}/{}", package.group, package.name), &manifest)?;
    }
    Ok(())
}
