//! Building the canonical envelope one slot execution runs under: the
//! provider record, the project/world projections, and the slot target.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENVELOPE-LAW");

use std::path::{Path, PathBuf};

use vibe_core::PackageName;
use vibe_core::manifest::{Manifest, Materialization};
use vibe_lifecycle::{
    DependencyProvider, DependencyProviderId, HostExtensionSource, HostIdentity, HostProvider,
    SlotTarget,
};
use vibe_wire::generated::lifecycle::e1::context::{Project, World, WorldPackage};
use vibe_workspace::Workspace;
use vibe_workspace::install::{ResolvedDep, SlotLifecycleContext};
use vibe_workspace::vibedeps::{in_place_slot_abs_path, slot_abs_path};

use crate::error::{Error, Result};

pub(super) fn dependency_provider(
    workspace_root: &Path,
    dep: &ResolvedDep,
) -> Result<DependencyProvider> {
    let root = if dep
        .manifest
        .package
        .as_ref()
        .is_some_and(|package| package.materialization == Materialization::InPlace)
    {
        in_place_slot_abs_path(workspace_root, &dep.group, &dep.name)
    } else {
        slot_abs_path(workspace_root, &dep.group, &dep.name, &dep.version)
    };
    Ok(DependencyProvider {
        id: DependencyProviderId::new(dep.group.clone(), PackageName::parse(&dep.name)?),
        root,
        version: dep.version.to_string(),
        kind: dep.kind,
        content_hash: dep.source_hash.clone().ok_or_else(|| {
            Error::Lifecycle(format!(
                "planned dependency `{}/{}@{}` has no source hash",
                dep.group, dep.name, dep.version,
            ))
        })?,
    })
}

pub(super) fn project_envelope(root: &Path, manifest: &Manifest) -> Project {
    let (name, version, kind) = if let Some(package) = &manifest.package {
        (
            package.name.clone(),
            package.version.to_string(),
            package.kind.as_str().to_string(),
        )
    } else if let Some(project) = &manifest.project {
        (
            project.name.clone(),
            project.version.clone(),
            "project".into(),
        )
    } else {
        (
            "<virtual-workspace>".into(),
            String::new(),
            "workspace".into(),
        )
    };
    Project {
        kind,
        manifest: vibe_core::machine_json_path(&root.join(Manifest::FILENAME)),
        name,
        root: vibe_core::machine_json_path(root),
        spec_roots: vec![vibe_core::machine_json_path(
            &root.join(vibe_core::layout::current_specs_root()),
        )],
        version,
    }
}

pub(super) fn host_source(root: &Path, manifest: &Manifest) -> Result<HostExtensionSource> {
    let (identity, version, kind) = if let Some(package) = &manifest.package {
        (
            HostIdentity::coordinate(DependencyProviderId::new(
                package.group.clone(),
                PackageName::parse(&package.name)?,
            )),
            package.version.to_string(),
            Some(package.kind),
        )
    } else if let Some(project) = &manifest.project {
        let identity = match &project.group {
            Some(group) => HostIdentity::coordinate(DependencyProviderId::new(
                group.clone(),
                PackageName::parse(&project.name)?,
            )),
            None => HostIdentity::ungrouped_project(project.name.clone()),
        };
        (identity, project.version.clone(), None)
    } else {
        (HostIdentity::virtual_workspace(), String::new(), None)
    };
    Ok(HostExtensionSource {
        provider: HostProvider {
            identity,
            root: root.to_path_buf(),
            version,
            kind,
            content_hash: None,
        },
        declarations: manifest.extensions.clone(),
        controls: manifest.extension_controls.clone(),
        mechanisms: manifest.mechanism_decls.clone(),
    })
}

pub(super) fn world_envelope(workspace: &Workspace, resolution: &[ResolvedDep]) -> World {
    World {
        deps_root: vibe_core::machine_json_path(&workspace.vibedeps_root()),
        lockfile: vibe_core::machine_json_path(&workspace.lockfile_path()),
        packages: resolution
            .iter()
            .map(|dep| WorldPackage {
                group: dep.group.to_string(),
                kind: dep.kind.as_str().to_string(),
                name: dep.name.clone(),
                slot: vibe_core::machine_json_path(&dependency_slot(&workspace.root, dep)),
                version: dep.version.to_string(),
            })
            .collect(),
    }
}

pub(super) fn dependency_slot(workspace_root: &Path, dep: &ResolvedDep) -> PathBuf {
    if dep
        .manifest
        .package
        .as_ref()
        .is_some_and(|package| package.materialization == Materialization::InPlace)
    {
        in_place_slot_abs_path(workspace_root, &dep.group, &dep.name)
    } else {
        slot_abs_path(workspace_root, &dep.group, &dep.name, &dep.version)
    }
}

pub(super) fn nonempty(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}

pub(super) fn slot_target(context: SlotLifecycleContext<'_>) -> SlotTarget {
    SlotTarget {
        group: context.group.to_string(),
        name: context.name.to_string(),
        version: context.version.to_string(),
        kind: context.kind.to_string(),
        root: vibe_core::machine_json_path(context.slot),
    }
}
