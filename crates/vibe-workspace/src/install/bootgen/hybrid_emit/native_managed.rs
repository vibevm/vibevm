//! Managed compiler-native package-unit emission over one bound owner epoch.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use vibe_core::manifest::SpecFormat;
use vibe_core::{PackageName, layout};
use vibe_extension_registry::DependencyProviderId;

use crate::boot::hybrid::fingerprint::{NativePendingFrame, fingerprints_with_pending};
use crate::boot::hybrid::{self, UnitId, UnitInput};
use crate::compile_trace::TraceRun;
use crate::extension_world::{OwnerNativeCompileProvider, OwnerRuntimeEpoch, OwnerRuntimeId};
use crate::{WorkspaceError, boot_artifacts};

use super::super::ResolvedDep;
use super::super::replay_prepare::{UnitReplayCandidate, static_dependencies};
use super::{UnitTrace, slot_rel_path, with_static_set, zone_projection::zone_to_effective};

type UnitNativeContinuations = Vec<(
    OwnerRuntimeId,
    boot_artifacts::OwnerNativeCompileContinuation,
)>;
type BoundUnitEmission = (
    HashSet<UnitId>,
    UnitNativeContinuations,
    Vec<UnitReplayCandidate>,
);

#[allow(clippy::too_many_arguments)]
pub(in crate::install::bootgen) fn emit_package_units_bound<P: OwnerNativeCompileProvider>(
    workspace_root: &Path,
    self_coord: &vibe_spec::SelfCoordinate,
    resolution: &[ResolvedDep],
    table: &HashMap<UnitId, UnitInput>,
    shared: &HashSet<UnitId>,
    versions: &HashMap<UnitId, String>,
    plan_digests: &HashMap<UnitId, String>,
    spec_format: SpecFormat,
    trace: Option<&TraceRun>,
    epoch: &OwnerRuntimeEpoch,
    mut provider: Option<&mut P>,
) -> Result<BoundUnitEmission, WorkspaceError> {
    let slots: HashMap<UnitId, String> = resolution
        .iter()
        .map(|dependency| {
            (
                (dependency.group.clone(), dependency.name.clone()),
                slot_rel_path(dependency),
            )
        })
        .collect();
    let trace_versions: Option<HashMap<UnitId, String>> = trace.map(|_| {
        resolution
            .iter()
            .map(|dependency| {
                (
                    (dependency.group.clone(), dependency.name.clone()),
                    dependency.version.to_string(),
                )
            })
            .collect()
    });
    let with_static = with_static_set(table);
    let base_fingerprints =
        crate::boot::hybrid::fingerprint::fingerprints(table, versions, plan_digests);
    let mut continuations = Vec::new();
    let mut replay_candidates = Vec::new();
    let ordered = hybrid::topo_zone(&with_static, table);
    let mut native_ids = HashSet::new();
    for (owner, runtime) in epoch.lowered().units() {
        let id = (owner.group().clone(), owner.name().to_string());
        if with_static.contains(&id) && runtime.has_compiler_native_intersection()? {
            native_ids.insert(id);
        }
    }
    let mut pending = HashMap::new();

    for id in ordered {
        let Some(slot) = slots.get(&id) else {
            continue;
        };
        let effective = zone_to_effective(
            &id,
            &hybrid::resolve_zone(&id, table),
            table,
            &with_static,
            &slots,
            shared,
            spec_format,
        );
        let boot_dir = workspace_root.join(slot).join(layout::current_boot_dir());
        let fingerprints = fingerprints_with_pending(table, versions, plan_digests, &pending);
        let fingerprint = fingerprints.get(&id).map(String::as_str).unwrap_or("");
        let unit_trace = trace
            .filter(|_| effective.static_entries().next().is_some())
            .map(|run| {
                UnitTrace::new(
                    run,
                    &id,
                    trace_versions
                        .as_ref()
                        .and_then(|versions| versions.get(&id))
                        .map_or("", String::as_str),
                    spec_format,
                    slot,
                )
            });
        let index = boot_dir.join(boot_artifacts::INDEX_FILE);
        let static_path = boot_dir.join(boot_artifacts::static_file(spec_format));
        let stale_path = boot_dir.join(if matches!(spec_format, SpecFormat::Xml) {
            boot_artifacts::STATIC_FILE
        } else {
            boot_artifacts::STATIC_XML_FILE
        });
        let has_native = native_ids.contains(&id);
        let owner = DependencyProviderId::new(
            id.0.clone(),
            PackageName::parse(&id.1).map_err(|error| WorkspaceError::UntypedBootProvenance {
                origin: format!("{}/{}", id.0, id.1),
                component: "unit package name",
                spelling: id.1.clone(),
                reason: error.to_string(),
            })?,
        );
        let owner_id = OwnerRuntimeId::Unit {
            provider: owner.clone(),
        };
        let base_fingerprint = base_fingerprints.get(&id).cloned().unwrap_or_default();
        let replay_dependencies = static_dependencies(&id, table, &with_static);
        let unchanged = !has_native
            && static_path.is_file()
            && !stale_path.exists()
            && fs::read_to_string(&index)
                .ok()
                .and_then(|existing| {
                    boot_artifacts::publication::read_unit_index_freshness(&existing).ok()
                })
                .is_some_and(|recorded| {
                    recorded.pending.is_none() && recorded.fingerprint == fingerprint
                });
        if unchanged {
            if let Some(unit_trace) = &unit_trace {
                unit_trace.record_fresh_skip(workspace_root);
            }
            replay_candidates.push(UnitReplayCandidate::new(
                workspace_root,
                slot,
                owner_id,
                id,
                effective,
                spec_format,
                base_fingerprint,
                replay_dependencies,
                has_native,
            ));
            continue;
        }
        let mode = match unit_trace.as_ref() {
            Some(unit_trace) => boot_artifacts::native_managed::OwnerNativeCompileMode::Traced(
                unit_trace.acquisition(),
            ),
            None => boot_artifacts::native_managed::OwnerNativeCompileMode::Plain,
        };
        let prepared_index = boot_artifacts::publication::prepare_index(&effective, spec_format)?;
        let compiled = boot_artifacts::native_managed::compile_static_owner_managed(
            &effective,
            workspace_root,
            self_coord,
            spec_format,
            epoch.unit(&owner)?,
            mode,
            provider.as_deref_mut(),
        )?;
        let pending_frame = compiled
            .as_ref()
            .and_then(|compiled| compiled.native())
            .and_then(|outcome| outcome.pending())
            .map(|(evidence, _)| NativePendingFrame::new(*evidence.fingerprint().as_bytes()));
        if let Some(frame) = pending_frame {
            pending.insert(id.clone(), frame);
        } else {
            pending.remove(&id);
        }
        let final_fingerprints = fingerprints_with_pending(table, versions, plan_digests, &pending);
        let final_fingerprint = final_fingerprints
            .get(&id)
            .map(String::as_str)
            .unwrap_or("");
        let index_text = boot_artifacts::publication::finish_index(
            prepared_index,
            Some(final_fingerprint),
            pending_frame,
        );
        let static_bytes = compiled
            .as_ref()
            .map(|compiled| compiled.artifact().bytes());
        boot_artifacts::publish_unit_artifacts(&boot_dir, &index_text, static_bytes, spec_format)?;
        if let Some(compiled) = compiled {
            let (_, native, _) = compiled.into_parts();
            if let Some(native) = native {
                continuations.push((owner_id, native));
            }
        }
        replay_candidates.push(UnitReplayCandidate::new(
            workspace_root,
            slot,
            OwnerRuntimeId::Unit { provider: owner },
            id,
            effective,
            spec_format,
            base_fingerprint,
            replay_dependencies,
            has_native,
        ));
    }
    Ok((with_static, continuations, replay_candidates))
}
