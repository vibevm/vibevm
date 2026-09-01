//! Bound-epoch boot regeneration through the one managed native compiler.

use std::collections::{BTreeMap, HashMap, HashSet};

use vibe_core::manifest::{BootCategory, SpecFormat};

use crate::boot;
use crate::boot::hybrid::{UnitId, fingerprint, hoist};
use crate::extension_world::{OwnerNativeCompileProvider, OwnerRuntimeEpoch, OwnerRuntimeId};
use crate::{Workspace, WorkspaceError, boot_artifacts};

use super::hybrid_emit::{
    append_hoisted, emit_package_units, emit_package_units_bound, with_static_set,
};
use super::owner_plans::plan_digest_frames;
use super::{
    ResolvedDep, build_unit_table, desubstitute_covered_units, node_dependency_boot, node_own_boot,
    root_self_coordinate,
};

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "remove when R5.4-INSTALL wires regeneration carriage"
    )
)]
pub(crate) struct BoundBootRegeneration {
    pub nodes: Vec<String>,
    pub native: BTreeMap<OwnerRuntimeId, boot_artifacts::OwnerNativeCompileContinuation>,
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "remove when R5.4-INSTALL wires bound regeneration"
    )
)]
pub(crate) fn regenerate_boot_from_bound_native<P: OwnerNativeCompileProvider>(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
    spec_format: SpecFormat,
    trace: Option<&crate::compile_trace::TraceRun>,
    epoch: &OwnerRuntimeEpoch,
    mut provider: Option<&mut P>,
) -> Result<BoundBootRegeneration, WorkspaceError> {
    if epoch.lowered().workspace_root() != workspace.root.as_path() {
        return Err(WorkspaceError::NativeCompileProvider {
            owner: "<epoch>".to_owned(),
            reason: "bound owner-runtime epoch belongs to a different workspace root".to_owned(),
        });
    }
    epoch.assert_resolution(&workspace.root, resolution)?;
    let self_coord = root_self_coordinate(&workspace.root_manifest);
    let table = build_unit_table(&workspace.root, resolution);
    let versions: HashMap<UnitId, String> = resolution
        .iter()
        .map(|dependency| {
            (
                (dependency.group.clone(), dependency.name.clone()),
                dependency.version.to_string(),
            )
        })
        .collect();
    let plan_digests = plan_digest_frames(epoch.lowered());
    let fps = fingerprint::fingerprints(&table, &versions, &plan_digests);
    let pulls = hoist::soft_static_pulls(&table);
    let shared: HashSet<UnitId> = pulls
        .iter()
        .filter(|(package, pullers)| {
            pullers.len() >= 2
                && table
                    .get(package)
                    .is_some_and(|unit| unit.has_static_boot())
        })
        .map(|(package, _)| package.clone())
        .collect();
    let emitted_units = with_static_set(&table);
    let mut has_native_units = false;
    for (owner, runtime) in epoch.lowered().units() {
        if emitted_units.contains(&(owner.group().clone(), owner.name().to_string()))
            && runtime.has_compiler_native_intersection()?
        {
            has_native_units = true;
            break;
        }
    }
    let (with_static, unit_native) = if has_native_units {
        emit_package_units_bound(
            &workspace.root,
            &self_coord,
            resolution,
            &table,
            &shared,
            &versions,
            &plan_digests,
            spec_format,
            trace,
            epoch,
            provider.as_deref_mut(),
        )?
    } else {
        let with_static = emit_package_units(
            &workspace.root,
            &self_coord,
            resolution,
            &table,
            &shared,
            &fps,
            spec_format,
            trace,
            epoch.lowered(),
        )?;
        (with_static, Vec::new())
    };
    let mut native = unit_native.into_iter().collect::<BTreeMap<_, _>>();

    let root_foundation = node_own_boot(&workspace.root, ".")?
        .into_iter()
        .filter(|boot| boot.category == Some(BootCategory::Foundation))
        .collect::<Vec<_>>();
    let mut nodes = Vec::new();
    for (rel, manifest) in workspace.iter_nodes() {
        let node_dir = workspace.node_abs_path(rel);
        let own = node_own_boot(&node_dir, rel)?;
        let inherited = if rel == "." {
            Vec::new()
        } else {
            root_foundation.clone()
        };
        let dependencies = node_dependency_boot(
            &workspace.root,
            manifest,
            resolution,
            &with_static,
            spec_format,
        );
        let mut effective = boot::compute_effective_boot(boot::NodeBootInputs {
            own_boot: &own,
            inherited_foundation: &inherited,
            dependencies: &dependencies,
            default_link: manifest.boot.default_link,
        })?;
        if rel == "." {
            append_hoisted(&mut effective, &shared, &table, &pulls);
        }
        desubstitute_covered_units(&mut effective, &table);
        let owner = OwnerRuntimeId::Node {
            rel: rel.to_owned(),
        };
        let (_, continuation) = boot_artifacts::native_managed::write_boot_artifacts_owner_managed(
            &node_dir,
            rel,
            &workspace.root,
            &self_coord,
            &effective,
            spec_format,
            trace,
            epoch.node(rel)?,
            provider.as_deref_mut(),
        )?;
        if let Some(continuation) = continuation {
            native.insert(owner, continuation);
        }
        nodes.push(rel.to_owned());
    }
    Ok(BoundBootRegeneration { nodes, native })
}

#[cfg(test)]
pub(crate) fn fingerprint_with_pending_for_test(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
    epoch: &OwnerRuntimeEpoch,
    id: &UnitId,
    frame: [u8; 32],
) -> Option<String> {
    let table = build_unit_table(&workspace.root, resolution);
    let versions = resolution
        .iter()
        .map(|dependency| {
            (
                (dependency.group.clone(), dependency.name.clone()),
                dependency.version.to_string(),
            )
        })
        .collect::<HashMap<_, _>>();
    let pending = HashMap::from([(id.clone(), fingerprint::NativePendingFrame::new(frame))]);
    fingerprint::fingerprints_with_pending(
        &table,
        &versions,
        &plan_digest_frames(epoch.lowered()),
        &pending,
    )
    .remove(id)
}
