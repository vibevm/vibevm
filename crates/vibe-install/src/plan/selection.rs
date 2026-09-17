//! Planning checks and selected-node helpers kept outside the main pipeline.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::path::Path;

use vibe_core::PackageRef;
use vibe_core::manifest::{Lockfile, Manifest, SpecFormat};
use vibe_workspace::{Workspace, vibedeps};

use crate::error::{Error, Result};
use crate::fetched::Fetched;

pub(super) fn refuse_read_only_kinds(fetched: &[Fetched]) -> Result<()> {
    for node in fetched {
        let meta = node.cached.package_meta();
        if meta.kind.is_read_only() {
            return Err(Error::DocNotInstalled {
                coordinate: format!("{}/{}", meta.group, meta.name),
            });
        }
    }
    Ok(())
}

pub(super) fn migrate_selected_node(
    workspace: &mut Workspace,
    project_root: &Path,
    entries: &[PackageRef],
) {
    let Some(node) = selected_node_manifest_mut(workspace, project_root) else {
        return;
    };
    for entry in entries {
        let already = node
            .requires
            .packages
            .iter()
            .any(|p| p.group == entry.group && p.name == entry.name);
        if !already {
            node.requires.packages.push(entry.clone());
        }
    }
}

fn selected_node_manifest_mut<'a>(
    workspace: &'a mut Workspace,
    project_root: &Path,
) -> Option<&'a mut Manifest> {
    if workspace.root == project_root {
        return Some(&mut workspace.root_manifest);
    }
    let selected = workspace
        .members
        .iter()
        .position(|member| workspace.member_abs_path(member) == project_root)?;
    Some(&mut workspace.members[selected].manifest)
}

pub(super) fn visibility_root_id(manifest: &Manifest) -> String {
    manifest
        .consumer_node()
        .map(|node| node.coordinate())
        .unwrap_or_else(|| "__vibevm__/workspace-root".to_string())
}

pub(super) fn slots_match_spec_format(
    workspace_root: &Path,
    lockfile: &Lockfile,
    spec_format: SpecFormat,
) -> bool {
    lockfile.packages.iter().all(|package| {
        if package.materialization.is_in_place() {
            return spec_format == SpecFormat::Mixed
                && vibedeps::is_in_place_slot(workspace_root, &package.group, &package.name);
        }
        let slot = vibedeps::slot_abs_path(
            workspace_root,
            &package.group,
            &package.name,
            &package.version,
        );
        slot.is_dir() && vibedeps::format_is_current(&slot, spec_format)
    })
}
