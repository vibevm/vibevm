//! Read-only source-present admission for submitted Stage A inspection.
//! Cold install replay is deliberately not implemented by this observer.
use anyhow::{Result, bail, ensure};
use std::path::{Component, Path};
use vibe_core::manifest::{AuthKind, Lockfile, Manifest, RegistryType};

#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#CANDIDATE-BOUNDARY")]
pub fn verify_submitted_inspection(root: &Path, lock: &Lockfile) -> Result<()> {
    if !lock
        .configurations
        .values()
        .any(|record| !record.submitted_inputs.is_empty())
    {
        return Ok(());
    }
    let workspace = canonical_selected_root(root)?;
    let project = vibe_safefs::Project::open(root)?;
    let captured = project
        .read_file_snapshot_bounded("vibe.toml", 64 * 1024)?
        .ok_or_else(|| anyhow::anyhow!("submitted root manifest missing"))?;
    let manifest = Manifest::parse_str(std::str::from_utf8(&captured.bytes)?)?;
    ensure!(
        manifest.registries.len() == 1,
        "submitted inspection needs one exact local registry"
    );
    let registry = &manifest.registries[0];
    ensure!(
        registry.enabled
            && registry.checked_type()? == RegistryType::Localdir
            && registry.token_env.is_none()
            && registry.auth.is_none_or(|auth| auth == AuthKind::None),
        "submitted inspection registry not admitted"
    );
    let registry_root = vibe_registry::localdir_path(&registry.url)?;
    let relative = registry_root.strip_prefix(&workspace)?;
    let components = relative
        .components()
        .map(|component| match component {
            Component::Normal(name) => name
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("nonUTF8 registry component")),
            _ => Err(anyhow::anyhow!("noncanonical registry")),
        })
        .collect::<Result<Vec<_>>>()?;
    ensure!(!components.is_empty(), "submitted registry must be nested");
    for (key, record) in &lock.configurations {
        for input in record.submitted_inputs.values() {
            ensure!(
                vibe_core::manifest::submitted_project_witness(&workspace, &captured.bytes)
                    .map_err(anyhow::Error::msg)?
                    == input.project_witness,
                "submitted project changed"
            );
            ensure!(
                manifest.requires.packages.len() == 1
                    && manifest.requires.packages[0] == input.requirement
                    && !manifest.requires.configurations.contains_key(key)
                    && !manifest
                        .requires
                        .optional
                        .get(key)
                        .copied()
                        .unwrap_or(false),
                "submitted root declaration changed"
            );
            let row = lock
                .packages
                .iter()
                .find(|row| format!("{}/{}", row.group, row.name) == *key)
                .ok_or_else(|| anyhow::anyhow!("submitted row missing"))?;
            let expected_path = registry_root
                .join(row.group.as_str())
                .join(row.name.as_str())
                .join(format!("v{}", row.version));
            ensure!(
                vibe_registry::localdir_path(row.source_url.as_str())? == expected_path,
                "submitted selected source path differs from exact local owner"
            );
            ensure!(
                row.registry.as_deref() == Some(registry.name.as_str())
                    && input.expected_source.registry == registry.name
                    && row.content_hash == input.expected_source.content_hash,
                "submitted selected source identity changed"
            );
            let observation = vibe_registry::local_preview::observe_local_preview(
                &project,
                &components,
                &input.requirement,
            )?;
            ensure!(
                observation.content_hash == input.expected_source.content_hash.as_str(),
                "submitted source bytes changed"
            );
            row_identity(row, &observation.manifest)?;
            record.validate(&observation.manifest)?;
        }
    }
    ensure!(
        project
            .read_file_snapshot_bounded("vibe.toml", 64 * 1024)?
            .as_ref()
            == Some(&captured),
        "submitted project changed during source observation"
    );
    Ok(())
}
fn canonical_selected_root(root: &Path) -> Result<std::path::PathBuf> {
    let canonical = root.canonicalize()?;
    if canonical != root {
        bail!("submitted root must be canonical");
    }
    Ok(canonical)
}
fn row_identity(row: &vibe_core::manifest::LockedPackage, manifest: &Manifest) -> Result<()> {
    let meta = manifest.require_package()?;
    ensure!(
        meta.group == row.group
            && meta.name == row.name
            && meta.version == row.version
            && meta.kind == row.kind,
        "submitted selected metadata changed"
    );
    Ok(())
}
