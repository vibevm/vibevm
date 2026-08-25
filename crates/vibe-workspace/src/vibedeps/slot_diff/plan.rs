//! Read-only planning for an incremental slot reconciliation.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#DIFF-MATERIALISE");

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::{
    PreparedSlotFile, directory_contains_only_stale, inspect_destination, inspect_incoming_parents,
    inspect_stale_paths, refuse_incoming_topology_collisions,
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
                stale_scaffold,
            )?);
        }
        Ok(Self { keep, stale })
    }
}
