//! Bound-epoch boot regeneration through the one managed native compiler.

use std::collections::{BTreeMap, HashMap, HashSet};

use vibe_core::manifest::{BootCategory, SpecFormat};

use crate::boot;
use crate::boot::hybrid::{UnitId, fingerprint, hoist};
use crate::extension_world::{OwnerNativeCompileProvider, OwnerRuntimeEpoch, OwnerRuntimeId};
use crate::{Workspace, WorkspaceError, boot_artifacts};

#[cfg(test)]
use super::build_unit_table;
use super::hybrid_emit::{
    append_hoisted, emit_package_units, emit_package_units_bound, with_static_set,
};
use super::owner_plans::plan_digest_frames_for;
use super::replay_prepare::{
    BootReplayCandidates, BootReplaySet, NodeReplayCandidate, node_generated_dependencies,
    seal_replay_set,
};
use super::{ResolvedDep, desubstitute_covered_units, node_dependency_boot, root_self_coordinate};

pub(crate) struct BoundBootRegeneration {
    pub nodes: Vec<String>,
    pub native: BTreeMap<OwnerRuntimeId, boot_artifacts::OwnerNativeCompileContinuation>,
    replay_candidates: BootReplayCandidates,
}

impl BoundBootRegeneration {
    pub(crate) fn into_replay_set(
        self,
        epoch: &OwnerRuntimeEpoch,
    ) -> Result<BootReplaySet, WorkspaceError> {
        seal_replay_set(self.replay_candidates, self.native, epoch)
    }
}

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
    super::super::source_admission::validate_source_admission(workspace, resolution, spec_format)?;
    let mut sources = boot_artifacts::declared_sources::DeclaredBootSources::prepare_with_nodes(
        workspace,
        resolution,
        Some(spec_format),
    )?;
    let self_coord = root_self_coordinate(&workspace.root_manifest);
    let mut replay_candidates =
        BootReplayCandidates::new(workspace.root.clone(), self_coord.clone())
            .with_sources(sources.clone());
    let targets = super::conditions::project_targets(&workspace.root_manifest);
    let table = super::build_unit_table_bound(
        &workspace.root,
        resolution,
        targets.clone(),
        sources.as_ref(),
    )?;
    let versions: HashMap<UnitId, String> = resolution
        .iter()
        .map(|dependency| {
            (
                (dependency.group.clone(), dependency.name.clone()),
                dependency.version.to_string(),
            )
        })
        .collect();
    let plan_digests = plan_digest_frames_for(epoch.lowered(), spec_format);
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
    let controlled = provider
        .as_ref()
        .and_then(|provider| provider.cancellation())
        .is_some();
    let mut has_native_units = false;
    for (owner, runtime) in epoch.lowered().units() {
        if emitted_units.contains(&(owner.group().clone(), owner.name().to_string()))
            && runtime.has_compiler_native_intersection()?
        {
            has_native_units = true;
            break;
        }
    }
    let (with_static, unit_native, unit_candidates) = if has_native_units || controlled {
        emit_package_units_bound(
            &workspace.root,
            &self_coord,
            resolution,
            &table,
            &shared,
            &versions,
            &plan_digests,
            spec_format,
            sources.as_ref(),
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
            sources.as_ref(),
            trace,
            epoch.lowered(),
        )?;
        (with_static, Vec::new(), Vec::new())
    };
    for candidate in unit_candidates {
        replay_candidates.push_unit(candidate);
    }
    let mut native = unit_native.into_iter().collect::<BTreeMap<_, _>>();

    let root_foundation = sources
        .as_ref()
        .ok_or_else(|| WorkspaceError::Io {
            path: workspace.root.clone(),
            reason: "prepared nodes missing".into(),
        })?
        .node(".")?
        .entries
        .clone()
        .into_iter()
        .filter(|boot| boot.category == Some(BootCategory::Foundation))
        .collect::<Vec<_>>();
    let mut nodes = Vec::new();
    for (rel, manifest) in workspace.iter_nodes() {
        if !super::publish_node_boot(rel, manifest, resolution) {
            continue;
        }
        let node_dir = workspace.node_abs_path(rel);
        let own = sources
            .as_ref()
            .ok_or_else(|| WorkspaceError::Io {
                path: node_dir.clone(),
                reason: "prepared nodes missing".into(),
            })?
            .node(rel)?
            .entries
            .clone();
        let inherited = if rel == "." {
            Vec::new()
        } else {
            root_foundation.clone()
        };
        let dependencies = node_dependency_boot(
            &workspace.root,
            manifest,
            resolution,
            &table,
            &with_static,
            spec_format,
            targets.clone(),
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
        super::generated_inputs::admit(
            &mut sources,
            &effective,
            resolution,
            &table,
            epoch.lowered(),
            spec_format,
            &self_coord,
        )?;
        let owner = OwnerRuntimeId::Node {
            rel: rel.to_owned(),
        };
        let node_runtime = epoch.node(rel)?;
        let native_intersection = node_runtime.runtime().has_compiler_native_intersection()?;
        let replay_dependencies = node_generated_dependencies(&effective, &emitted_units);
        let (_, continuation) =
            boot_artifacts::native_managed::write_boot_artifacts_owner_managed_with_sources(
                &node_dir,
                rel,
                &workspace.root,
                &self_coord,
                &effective,
                spec_format,
                sources.as_ref(),
                trace,
                node_runtime,
                provider.as_deref_mut(),
                Some(boot_artifacts::NodePrompt { manifest }),
            )?;
        if let Some(continuation) = continuation {
            native.insert(owner, continuation);
        }
        replay_candidates.push_node(NodeReplayCandidate::new(
            node_dir,
            rel.to_owned(),
            effective,
            spec_format,
            replay_dependencies,
            native_intersection,
            sources
                .as_ref()
                .map(|sources| sources.node_guards(rel))
                .unwrap_or_default(),
        ));
        nodes.push(rel.to_owned());
    }
    replay_candidates = replay_candidates.with_sources(sources);
    Ok(BoundBootRegeneration {
        nodes,
        native,
        replay_candidates,
    })
}

#[cfg(test)]
pub(crate) fn fingerprint_with_pending_for_test(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
    epoch: &OwnerRuntimeEpoch,
    id: &UnitId,
    frame: [u8; 32],
) -> Option<String> {
    let table = build_unit_table(
        &workspace.root,
        resolution,
        super::conditions::project_targets(&workspace.root_manifest),
    );
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
        &plan_digest_frames_for(epoch.lowered(), SpecFormat::Mixed),
        &pending,
    )
    .remove(id)
}
