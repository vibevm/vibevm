//! Local/embedded application and installer-provider resolution.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#sources");

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_core::PackageRef;
use vibe_core::manifest::Manifest;
use vibe_registry::LocalRegistry;

use super::model::{ApplicationIdentity, PackageIdentity};

pub struct ResolvedApplication {
    pub application: ApplicationIdentity,
    pub installer_entry: PathBuf,
    pub installer_root: PathBuf,
}

pub fn resolve_application(registry_root: &Path, spelling: &str) -> Result<ResolvedApplication> {
    let registry_root = canonical_directory(registry_root, "application registry")?;
    let requested = qualified_ref(spelling)?;
    let registry = LocalRegistry::new(registry_root.clone())
        .context("opening the local application registry")?;
    let selected = registry
        .resolve(&requested)
        .with_context(|| format!("resolving global application package `{spelling}`"))?;
    let source_root = contained_directory(
        &registry_root,
        &selected.source_dir,
        "application package source",
    )?;
    let manifest = Manifest::read(source_root.join(Manifest::FILENAME))
        .context("reading the application package manifest")?;
    let package = manifest.require_package()?;
    if package.group != selected.group
        || package.name != selected.name
        || package.version != selected.version
    {
        bail!(
            "resolved application package manifest identity differs from its registry coordinate"
        );
    }
    let declaration = manifest
        .application
        .clone()
        .ok_or_else(|| anyhow::anyhow!("package `{spelling}` declares no [application]"))?;
    let provider_ref = &declaration.installer_package;
    let provider = registry
        .resolve(provider_ref)
        .context("resolving the declared application installer package")?;
    let provider_root = contained_directory(
        &registry_root,
        &provider.source_dir,
        "application installer package source",
    )?;
    let provider_manifest = Manifest::read(provider_root.join(Manifest::FILENAME))
        .context("reading the application installer package manifest")?;
    let provider_package = provider_manifest.require_package()?;
    if provider_package.group != provider.group
        || provider_package.name != provider.name
        || provider_package.version != provider.version
    {
        bail!("resolved installer package manifest identity differs from its registry coordinate");
    }
    let entry = provider_root.join(&declaration.entry);
    let entry_metadata = fs::symlink_metadata(&entry)
        .with_context(|| format!("reading installer entry `{}`", entry.display()))?;
    if entry_metadata.file_type().is_symlink() || !entry_metadata.is_file() {
        bail!("application installer entry is not a regular contained source file");
    }
    let entry = crate::commands::init::strip_unc_public(
        fs::canonicalize(&entry)
            .with_context(|| format!("resolving installer entry `{}`", entry.display()))?,
    );
    if !entry.starts_with(&provider_root) || !entry.is_file() {
        bail!("application installer entry escapes its provider source or is not a regular file");
    }
    let application = ApplicationIdentity {
        id: declaration.id.clone(),
        package: PackageIdentity {
            group: package.group.to_string(),
            name: package.name.clone(),
            version: package.version.to_string(),
        },
        installer_package: PackageIdentity {
            group: provider.group.to_string(),
            name: provider.name,
            version: provider.version.to_string(),
        },
        commands: declaration.commands.clone(),
    };
    Ok(ResolvedApplication {
        application,
        installer_entry: entry,
        installer_root: provider_root,
    })
}

pub fn qualified_ref(spelling: &str) -> Result<PackageRef> {
    let requested = PackageRef::parse(spelling)
        .with_context(|| format!("parsing global application package `{spelling}`"))?;
    if requested.group.is_none() {
        bail!("global application package must be fully qualified as <group>/<name>");
    }
    Ok(requested)
}

fn canonical_directory(path: &Path, label: &str) -> Result<PathBuf> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("reading {label} `{}`", path.display()))?;
    if metadata.file_type().is_symlink() {
        bail!("{label} is a symbolic link");
    }
    let canonical = crate::commands::init::strip_unc_public(
        fs::canonicalize(path)
            .with_context(|| format!("resolving {label} `{}`", path.display()))?,
    );
    if !canonical.is_dir() {
        bail!("{label} is not a directory");
    }
    Ok(canonical)
}

fn contained_directory(root: &Path, path: &Path, label: &str) -> Result<PathBuf> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("reading {label} `{}`", path.display()))?;
    if metadata.file_type().is_symlink() {
        bail!("{label} is a symbolic link");
    }
    let canonical = canonical_directory(path, label)?;
    if !canonical.starts_with(root) {
        bail!("{label} escapes the selected registry");
    }
    Ok(canonical)
}
