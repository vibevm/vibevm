//! Read-only planning for an incremental slot reconciliation.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#DIFF-MATERIALISE");

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::{
    PreparedContent, PreparedSlotFile, directory_contains_only_stale, inspect_incoming_parents,
    inspect_stale_paths, io_err, materialisation_error, refuse_incoming_topology_collisions,
    sha256_file,
};
use crate::{WorkspaceError, path_to_slash};

use super::super::{SlotFile, SlotRecord};

pub(super) struct DiffPlan {
    pub(super) keep: Vec<bool>,
    pub(super) stale: Vec<PathBuf>,
}

impl DiffPlan {
    pub(super) fn build(
        slot: &Path,
        incoming: &[PreparedSlotFile],
        old: &SlotRecord,
    ) -> Result<Self, WorkspaceError> {
        let old_files: BTreeMap<&str, &SlotFile> = old
            .files
            .iter()
            .map(|file| (file.path.as_str(), file))
            .collect();
        let incoming_paths: BTreeSet<String> =
            incoming.iter().map(PreparedSlotFile::path_wire).collect();
        refuse_incoming_topology_collisions(slot, &incoming_paths)?;

        let stale = old
            .files
            .iter()
            .filter(|file| !incoming_paths.contains(&file.path))
            .map(|file| PathBuf::from(&file.path))
            .collect::<Vec<_>>();
        let stale_paths: BTreeSet<String> = stale.iter().map(|path| path_to_slash(path)).collect();
        inspect_stale_paths(slot, &stale)?;
        inspect_incoming_parents(slot, &incoming_paths, &old_files, &stale)?;

        let mut keep = Vec::with_capacity(incoming.len());
        for file in incoming {
            let wire = file.path_wire();
            let destination = slot.join(&file.path);
            let old_file = old_files.get(wire.as_str()).copied();
            let stale_scaffold = old_file.is_none()
                && directory_contains_only_stale(&destination, slot, &stale_paths)?;
            keep.push(inspect_destination(
                &destination,
                old_file,
                file.sha256(),
                match &file.content {
                    PreparedContent::Observed { observation, .. } => {
                        Some(observation.permissions())
                    }
                    _ => None,
                },
                stale_scaffold,
            )?);
        }
        Ok(Self { keep, stale })
    }
}

fn inspect_destination(
    path: &Path,
    old: Option<&SlotFile>,
    incoming_hash: &str,
    incoming_permissions: Option<&std::fs::Permissions>,
    stale_scaffold: bool,
) -> Result<bool, WorkspaceError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if old.is_none() && metadata.file_type().is_file() => {
            let actual = sha256_file(path).map_err(|reason| materialisation_error(path, reason))?;
            if actual == incoming_hash {
                Ok(true)
            } else {
                Err(materialisation_error(
                    path,
                    "incoming payload collides with a different unrecorded on-disk file",
                ))
            }
        }
        Ok(metadata) if old.is_none() && metadata.is_dir() && stale_scaffold => Ok(false),
        Ok(_) if old.is_none() => Err(materialisation_error(
            path,
            "incoming payload collides with an unrecorded on-disk path",
        )),
        Ok(metadata) if metadata.is_dir() => Err(materialisation_error(
            path,
            "recorded payload path became a directory; refusing to remove unrecorded contents",
        )),
        Ok(metadata) if !metadata.file_type().is_file() && !metadata.file_type().is_symlink() => {
            Err(materialisation_error(
                path,
                "recorded payload path is neither a regular file nor a symbolic link",
            ))
        }
        Ok(metadata) if metadata.file_type().is_symlink() => Ok(false),
        Ok(metadata) => {
            let old = old.expect("the unrecorded case returned above");
            if old.sha256 != incoming_hash {
                return Ok(false);
            }
            // Only recorded payloads reconcile the incoming observed mode.
            // The foreign same-byte keep above never grants a chmod operation.
            if incoming_permissions
                .is_some_and(|permissions| metadata.permissions() != *permissions)
            {
                return Ok(false);
            }
            let actual = sha256_file(path).map_err(|reason| materialisation_error(path, reason))?;
            Ok(actual == incoming_hash)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(io_err(path, error)),
    }
}
