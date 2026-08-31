//! Strict durable-lock projection for Fresh/check/analyze boot work.
//!
//! The root lock is read once, then only the slots it names are read, in exact
//! lock order. The dependency root is never enumerated, so orphan slots remain
//! outside the world. Lock identity/hash/edges are authoritative; the one slot
//! manifest supplies package declarations, controls and boot content.

use super::*;

/// Read the command's durable world exactly once and project its named slots.
/// Missing lock is the explicit empty durable epoch. Every present malformed
/// or non-regular lock and every missing/disagreeing named slot refuses.
pub(in crate::install) fn read_durable_resolution(
    workspace_root: &Path,
) -> Result<Vec<ResolvedDep>, WorkspaceError> {
    let lock_path = workspace_root.join(vibe_core::manifest::Lockfile::FILENAME);
    let metadata = match fs::symlink_metadata(&lock_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(io_err(&lock_path, error)),
    };
    if !metadata.file_type().is_file() {
        return Err(super::owner_plans::world_error(
            crate::extension_world::ExtensionWorldError::NonRegularLock { path: lock_path },
        ));
    }
    let lock = vibe_core::manifest::Lockfile::read(&lock_path).map_err(|error| {
        super::owner_plans::world_error(crate::extension_world::ExtensionWorldError::InvalidLock {
            path: lock_path.clone(),
            reason: error.to_string(),
        })
    })?;

    lock.packages
        .iter()
        .map(|package| durable_row(workspace_root, package))
        .collect()
}

fn durable_row(
    workspace_root: &Path,
    package: &vibe_core::manifest::LockedPackage,
) -> Result<ResolvedDep, WorkspaceError> {
    let slot = match package.materialization {
        vibe_core::manifest::Materialization::InPlace => {
            vibedeps::in_place_slot_abs_path(workspace_root, &package.group, &package.name)
        }
        vibe_core::manifest::Materialization::Copy
        | vibe_core::manifest::Materialization::Hardlink => vibedeps::slot_abs_path(
            workspace_root,
            &package.group,
            &package.name,
            &package.version,
        ),
    };
    let locked = format!(
        "{}:{}/{}@{}",
        package.kind, package.group, package.name, package.version
    );
    if !slot.is_dir() {
        return Err(super::owner_plans::world_error(
            crate::extension_world::ExtensionWorldError::MissingSlot {
                package: locked,
                slot,
            },
        ));
    }
    let manifest_path = slot.join(Manifest::FILENAME);
    let manifest = Manifest::read(&manifest_path).map_err(|error| WorkspaceError::Manifest {
        path: manifest_path,
        source: Box::new(error),
    })?;
    let declared = manifest.package.as_ref().ok_or_else(|| {
        super::owner_plans::world_error(
            crate::extension_world::ExtensionWorldError::SlotWithoutPackage { slot: slot.clone() },
        )
    })?;
    if declared.group != package.group
        || declared.name != package.name
        || declared.version != package.version
        || declared.kind != package.kind
    {
        return Err(super::owner_plans::world_error(
            crate::extension_world::ExtensionWorldError::SlotIdentityMismatch {
                slot,
                declared: format!(
                    "{}:{}/{}@{}",
                    declared.kind, declared.group, declared.name, declared.version
                ),
                locked,
            },
        ));
    }
    if declared.materialization != package.materialization {
        return Err(super::owner_plans::world_error(
            crate::extension_world::ExtensionWorldError::SlotMaterializationMismatch {
                slot,
                declared: crate::extension_world::materialization_name(declared.materialization),
                locked: crate::extension_world::materialization_name(package.materialization),
            },
        ));
    }

    let requires = package
        .dependencies
        .iter()
        .map(|dependency| {
            let group = dependency.group.clone().ok_or_else(|| {
                super::owner_plans::world_error(
                    crate::extension_world::ExtensionWorldError::UngroupedEdge {
                        owner: format!("{}/{}", package.group, package.name),
                        edge: dependency.name.to_string(),
                    },
                )
            })?;
            Ok((group, dependency.name.to_string()))
        })
        .collect::<Result<Vec<_>, WorkspaceError>>()?;

    Ok(ResolvedDep {
        kind: package.kind,
        group: package.group.clone(),
        name: package.name.to_string(),
        version: package.version.clone(),
        content_dir: slot,
        source_hash: Some(package.content_hash.clone()),
        manifest,
        requires,
        admitted_by: package.admitted_by.clone(),
        via_override: package.via_override.clone(),
        source_mutable: false,
        in_place_changed: None,
    })
}
