//! Cross-installation invariants for one application slot.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, bail};
use vibe_core::manifest::InstallerMethod;

use super::model::{
    self, AlternativeRecord, ApplicationIndex, ApplicationRecord, ApplicationStatus, InstallKind,
    PublicCommand,
};
use super::source::ResolvedApplication;
use super::store::installation_key;
use super::{Path, command};

pub(super) fn fresh_revision() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!(
        "{nanos:032x}{:08x}{:016x}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

pub(super) fn register_alternative(
    index: &mut ApplicationIndex,
    slot: &str,
    key: &str,
    commands: &[PublicCommand],
) -> Result<()> {
    for command in commands {
        if index.alternatives.iter().any(|(other_slot, alternative)| {
            other_slot != slot && alternative.reserved_commands.contains(&command.name)
        }) {
            bail!(
                "public command `{}` is already reserved by another application slot",
                command.name
            );
        }
    }
    let entry = index
        .alternatives
        .entry(slot.to_owned())
        .or_insert_with(|| AlternativeRecord {
            global_default: None,
            reserved_commands: Vec::new(),
            runners: Vec::new(),
        });
    if entry.global_default.is_none() {
        entry.global_default = Some(key.to_owned());
    }
    for command in commands {
        if !entry.reserved_commands.contains(&command.name) {
            entry.reserved_commands.push(command.name.clone());
        }
    }
    entry.reserved_commands.sort();
    Ok(())
}

pub(super) fn retire_alternative(index: &mut ApplicationIndex, slot: &str, key: &str) {
    let has_other_ready = index.applications.iter().any(|(other_key, record)| {
        other_key != key
            && record.application.id == slot
            && record.status == ApplicationStatus::Ready
    });
    if let Some(alternative) = index.alternatives.get_mut(slot)
        && alternative.global_default.as_deref() == Some(key)
        && !has_other_ready
    {
        alternative.global_default = None;
    }
}

pub(super) fn ensure_external_uninstall_isolated(
    index: &ApplicationIndex,
    slot: &str,
    key: &str,
) -> Result<()> {
    let Some(record) = index.applications.get(key) else {
        bail!("application installation `{key}` is absent");
    };
    if !matches!(
        record.install_kind,
        Some(InstallKind::Shell | InstallKind::Exe | InstallKind::Msi | InstallKind::Command)
    ) {
        return Ok(());
    }
    let Some(root) = record.app_root.as_deref() else {
        bail!("external installation has no registered root");
    };
    let normalized_root = physically_normalize_declared_root(root)?;
    for (other_key, other) in &index.applications {
        if other_key == key
            || other.application.id != slot
            || other.status != ApplicationStatus::Ready
        {
            continue;
        }
        if let Some(other_root) = other.app_root.as_deref() {
            let normalized_other = physically_normalize_declared_root(other_root)?;
            if path_contains(&normalized_root, &normalized_other)
                || path_contains(&normalized_other, &normalized_root)
            {
                bail!(
                    "non_isolated_installation: external uninstall `{key}` overlaps ready `{other_key}`"
                );
            }
        }
    }
    Ok(())
}

/// External installers must not overwrite another live installation's root.
/// Package-authored commands can still mutate outside that declaration, so
/// the installer author remains responsible for the declared isolation.
pub(super) fn ensure_external_isolation(
    index: &ApplicationIndex,
    resolved: &ResolvedApplication,
    key: &str,
    settings: &Path,
) -> Result<()> {
    let template = match &resolved.installer {
        InstallerMethod::Shell { app_root, .. }
        | InstallerMethod::Exe { app_root, .. }
        | InstallerMethod::Msi { app_root, .. } => app_root,
        InstallerMethod::Zip { .. } | InstallerMethod::TarGz { .. } => return Ok(()),
    };
    let root = command::preview_app_root(template, settings)?;
    let normalized_root = physically_normalize_declared_root(&root)?;
    for (other_key, record) in &index.applications {
        if other_key == key
            || record.status != ApplicationStatus::Ready
            || record.application.id != resolved.application.id
        {
            continue;
        }
        if let Some(other_root) = record.app_root.as_deref() {
            let normalized_other = physically_normalize_declared_root(other_root)?;
            if path_contains(&normalized_root, &normalized_other)
                || path_contains(&normalized_other, &normalized_root)
            {
                bail!(
                    "non_isolated_installation: external root `{}` overlaps ready installation `{other_key}` at `{}`",
                    root.display(),
                    other_root.display()
                );
            }
        }
    }
    Ok(())
}

/// Resolve every existing ancestor while preserving a suffix that an external
/// installer has not created yet. This makes containment checks follow
/// symlinked ancestors without requiring the declared root itself to exist.
fn physically_normalize_declared_root(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() {
        bail!("application root `{}` is not absolute", path.display());
    }

    let mut existing = path;
    let mut missing = Vec::new();
    loop {
        match fs::canonicalize(existing) {
            Ok(canonical) => {
                let mut normalized = crate::commands::init::strip_unc_public(canonical);
                for component in missing.iter().rev() {
                    normalized.push(component);
                }
                return Ok(normalized);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let Some(component) = existing.file_name() else {
                    return Err(error.into());
                };
                missing.push(component.to_os_string());
                let Some(parent) = existing.parent() else {
                    return Err(error.into());
                };
                existing = parent;
            }
            Err(error) => return Err(error.into()),
        }
    }
}

fn path_contains(parent: &Path, child: &Path) -> bool {
    #[cfg(windows)]
    {
        let components = |path: &Path| {
            path.components()
                .map(|part| {
                    part.as_os_str()
                        .to_string_lossy()
                        .replace('/', "\\")
                        .to_ascii_lowercase()
                })
                .collect::<Vec<_>>()
        };
        components(child).starts_with(&components(parent))
    }
    #[cfg(not(windows))]
    {
        child.starts_with(parent)
    }
}

pub(super) fn settings_root() -> Result<PathBuf> {
    vibe_core::settings::settings_dir().ok_or_else(|| {
        anyhow::anyhow!(
            "the Vibe settings root is unavailable; set VIBE_SETTINGS or make the user home resolvable"
        )
    })
}

pub(super) fn reject_identity_collision(
    index: &model::ApplicationIndex,
    application: &model::ApplicationIdentity,
) -> Result<()> {
    let key = installation_key(&application.package);
    if let Some(existing) = index.applications.get(&key)
        && existing.application.id != application.id
    {
        bail!(
            "application installation `{key}` is already bound to slot `{}`",
            existing.application.id
        );
    }
    Ok(())
}

pub(super) fn record_for_package<'a>(
    index: &'a model::ApplicationIndex,
    requested: &vibe_core::PackageRef,
) -> Result<&'a ApplicationRecord> {
    let group = requested
        .group
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("global application package is unqualified"))?
        .to_string();
    let candidates: Vec<_> = index
        .applications
        .values()
        .filter(|record| {
            record.application.package.group == group
                && record.application.package.name == requested.name.as_str()
        })
        .filter(|record| {
            semver::Version::parse(&record.application.package.version)
                .is_ok_and(|version| requested.version.matches(&version))
        })
        .collect();
    if candidates.is_empty() {
        return Err(anyhow::anyhow!(
            "not_installed: global application {group}/{} is not installed",
            requested.name
        ));
    }
    if candidates.len() == 1 {
        return Ok(candidates[0]);
    }
    let active: Vec<_> = candidates
        .iter()
        .copied()
        .filter(|record| {
            index
                .alternatives
                .get(&record.application.id)
                .and_then(|alternative| alternative.global_default.as_deref())
                == Some(installation_key(&record.application.package).as_str())
        })
        .collect();
    if active.len() == 1 {
        return Ok(active[0]);
    }
    Err(anyhow::anyhow!(
        "selection_required: multiple installed versions of {group}/{} match; specify an exact @=VERSION",
        requested.name
    ))
}
