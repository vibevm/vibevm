//! Boot-artifact (re)generation — the boot half of PROP-009's loading model.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#install");

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::Path;

use specmark::spec;
use vibe_core::manifest::{BootCategory, LinkType, Manifest, SpecFormat, TargetSelection};
use vibe_core::{Group, layout};

use crate::boot::hybrid::{UnitId, UnitInput, fingerprint, hoist};
use crate::boot::{self, AuthoredBoot, DependencyBoot, NodeBootInputs};
use crate::extension_world::{
    ExtensionWorldEpoch, LoweredOwnerRuntimes, OwnerRuntimeLowering, lower_owner_runtimes,
};
use crate::{Workspace, WorkspaceError, boot_artifacts, layout_paths, path_to_slash, vibedeps};

use super::{ResolvedDep, io_err};

#[path = "bootgen/generated_inputs.rs"]
mod generated_inputs;
/// Durable-world owner-plan lowering along its own responsibility.
#[path = "bootgen/owner_plans.rs"]
mod owner_plans;
use owner_plans::plan_digest_frames_for;
#[cfg(test)]
#[path = "bootgen/compile_plan_tests.rs"]
mod compile_plan_tests;

#[path = "bootgen/hybrid_emit.rs"]
mod hybrid_emit;
#[cfg(test)]
use hybrid_emit::build_unit_table;
/// The slot whose unit lane a resolved package's emission writes.
pub(crate) use hybrid_emit::unit_slot_rel_path;
use hybrid_emit::{
    append_hoisted, build_unit_table_bound, emit_package_units, verify_fingerprints,
};

#[path = "bootgen/analyze.rs"]
mod analyze;
#[cfg(test)]
pub(crate) use analyze::analyze_effective_bound_native;
pub use analyze::{
    AnalyzedBoundLane, AnalyzedLane, analyze_node_lane, analyze_node_lane_bound_native,
    analyze_node_lane_bound_native_with_source_format, analyze_node_lane_with_source_format,
    compile_node_backend, compile_node_backend_with_source_format,
};

mod materialised_read;
pub(crate) use materialised_read::read_durable_resolution;
mod conditions;
mod node_dependency;
mod snippet_source;
mod transitive;
use node_dependency::node_dependency_boot;
mod publication;
use conditions::{active_snippet, project_targets};
use publication::publish_node_boot;
use transitive::static_transitive_closure;

mod desubstitute;
pub(crate) mod native_managed;
pub(crate) mod replay_prepare;
pub(crate) mod replay_publish;
pub use desubstitute::desubstitute_covered_units;

pub(crate) fn root_self_coordinate(root_manifest: &Manifest) -> vibe_spec::SelfCoordinate {
    match root_manifest.consumer_node() {
        Some(node) => vibe_spec::SelfCoordinate::new(
            node.group.as_ref().map(|g| g.as_str().to_owned()),
            node.name,
        ),
        None => vibe_spec::SelfCoordinate::new(None, String::new()),
    }
}

/// Regenerate each node from `resolution`, returning written node paths.
pub fn regenerate_boot_from(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
) -> Result<Vec<String>, WorkspaceError> {
    regenerate_boot_from_with_spec_format(workspace, resolution, SpecFormat::Mixed)
}

pub fn regenerate_boot_from_with_spec_format(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
    spec_format: SpecFormat,
) -> Result<Vec<String>, WorkspaceError> {
    regenerate_boot_from_traced(workspace, resolution, spec_format, None)
}

/// Traced regeneration; the command owner retains the borrowed run.
pub fn regenerate_boot_from_traced(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
    spec_format: SpecFormat,
    trace: Option<&crate::compile_trace::TraceRun>,
) -> Result<Vec<String>, WorkspaceError> {
    regenerate_boot_from_traced_prepared(
        workspace,
        resolution,
        spec_format,
        trace,
        OwnerRuntimeLowering::compatibility_root_without_presets(),
    )
    .map(|prepared| prepared.nodes)
}

/// One prepared regeneration and the exact neutral runtime set it lowered.
#[derive(Debug)]
pub struct BootRegeneration {
    pub nodes: Vec<String>,
    pub runtimes: LoweredOwnerRuntimes,
}

/// Lower every owner once, regenerate, and return the retained runtime set.
pub fn regenerate_boot_from_traced_prepared(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
    spec_format: SpecFormat,
    trace: Option<&crate::compile_trace::TraceRun>,
    lowering: OwnerRuntimeLowering,
) -> Result<BootRegeneration, WorkspaceError> {
    super::source_admission::validate_source_admission(workspace, resolution, spec_format)?;
    let mut sources = boot_artifacts::declared_sources::DeclaredBootSources::prepare_with_nodes(
        workspace,
        resolution,
        Some(spec_format),
    )?;
    // B-031 self is always the workspace root coordinate.
    let self_coord = root_self_coordinate(&workspace.root_manifest);

    // Exact supplied world; Ready never consults the still-old ambient lock.
    let world = ExtensionWorldEpoch::from_resolution(&workspace.root, resolution)
        .map_err(owner_plans::world_error)?;

    // Compile package units first so node edges can target complete zones.
    // One `target:` verdict for the whole regeneration, read from the
    // project's own authored declaration (PROP-076 `#SYS-HOST-AND-TARGET`).
    let targets = project_targets(&workspace.root_manifest);
    let table = build_unit_table_bound(
        &workspace.root,
        resolution,
        targets.clone(),
        sources.as_ref(),
    )?;
    // Lower every owner before plan digests feed unit fingerprints.
    let runtimes = lower_owner_runtimes(workspace, &world, lowering)?;
    // Unit version plus its artifact-filtered owner plan drives dirty skips.
    let versions: HashMap<UnitId, String> = resolution
        .iter()
        .map(|d| ((d.group.clone(), d.name.clone()), d.version.to_string()))
        .collect();
    let fps = fingerprint::fingerprints(
        &table,
        &versions,
        &plan_digest_frames_for(&runtimes, spec_format),
    );
    // Hoist packages pulled statically by two or more units.
    let pulls = hoist::soft_static_pulls(&table);
    let shared: HashSet<UnitId> = pulls
        .iter()
        .filter(|(pkg, pullers)| {
            pullers.len() >= 2 && table.get(pkg).is_some_and(|u| u.has_static_boot())
        })
        .map(|(pkg, _)| pkg.clone())
        .collect();
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
        &runtimes,
    )?;

    // The absolute root's foundation boot — inherited by every member
    // (PROP-009 §2.2: inherited foundation flows down).
    let root_foundation: Vec<AuthoredBoot> = sources
        .as_ref()
        .ok_or_else(|| WorkspaceError::Io {
            path: workspace.root.clone(),
            reason: "prepared nodes missing".into(),
        })?
        .node(".")?
        .entries
        .clone()
        .into_iter()
        .filter(|b| b.category == Some(BootCategory::Foundation))
        .collect();

    let mut nodes_regenerated = Vec::new();
    for (rel, manifest) in workspace.iter_nodes() {
        if !publish_node_boot(rel, manifest, resolution) {
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
        let inherited: Vec<AuthoredBoot> = if rel == "." {
            Vec::new()
        } else {
            root_foundation.clone()
        };
        let deps = node_dependency_boot(
            &workspace.root,
            manifest,
            resolution,
            &table,
            &with_static,
            spec_format,
            targets.clone(),
        );
        let mut effective = boot::compute_effective_boot(NodeBootInputs {
            own_boot: &own,
            inherited_foundation: &inherited,
            dependencies: &deps,
            default_link: manifest.boot.default_link,
        })?;
        // The absolute root is the hoist point for shared packages.
        if rel == "." {
            append_hoisted(&mut effective, &shared, &table, &pulls);
        }
        // Collapse covered unit zones after hoisting, before artifact writes.
        desubstitute_covered_units(&mut effective, &table);
        generated_inputs::admit(
            &mut sources,
            &effective,
            resolution,
            &table,
            &runtimes,
            spec_format,
            &self_coord,
        )?;
        // Each node receives its own retained owner plan.
        boot_artifacts::write_boot_artifacts_traced(
            &node_dir,
            rel,
            &workspace.root,
            &self_coord,
            &effective,
            spec_format,
            sources.as_ref(),
            trace,
            runtimes.node(rel)?.compile_plans().clone(),
            Some(boot_artifacts::NodePrompt {
                node_rel: rel,
                manifest,
            }),
        )?;
        nodes_regenerated.push(rel.to_string());
    }
    Ok(BootRegeneration {
        nodes: nodes_regenerated,
        runtimes,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn regenerate_boot_from_traced_native_prepared<F, P>(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
    world: ExtensionWorldEpoch,
    spec_format: SpecFormat,
    trace: Option<&crate::compile_trace::TraceRun>,
    lowering: OwnerRuntimeLowering,
    run: crate::extension_world::OwnerRuntimeRunFacts,
    make_provider: &mut F,
) -> Result<(Vec<String>, super::NativeInstallCarriage), WorkspaceError>
where
    F: FnMut(
        std::collections::BTreeMap<
            crate::extension_world::OwnerRuntimeId,
            vibe_spec::CompilerNativePolicy,
        >,
    ) -> Result<P, WorkspaceError>,
    P: crate::extension_world::OwnerNativeCompileProvider,
{
    let lowered = lower_owner_runtimes(workspace, &world, lowering)?;
    let epoch = lowered.bind_run(run);
    let policies = epoch
        .lowered()
        .nodes()
        .keys()
        .cloned()
        .map(|rel| {
            (
                crate::extension_world::OwnerRuntimeId::Node { rel },
                vibe_spec::CompilerNativePolicy::collect(),
            )
        })
        .chain(epoch.lowered().units().keys().cloned().map(|provider| {
            (
                crate::extension_world::OwnerRuntimeId::Unit { provider },
                vibe_spec::CompilerNativePolicy::collect(),
            )
        }))
        .collect();
    let mut provider = make_provider(policies)?;
    let regenerated = native_managed::regenerate_boot_from_bound_native(
        workspace,
        resolution,
        spec_format,
        trace,
        &epoch,
        Some(&mut provider),
    )?;
    let nodes = regenerated.nodes.clone();
    let replay = regenerated.into_replay_set(&epoch)?;
    Ok((nodes, super::NativeInstallCarriage::new(epoch, replay)?))
}

/// Regenerate from materialised dependency slots, without resolving or copying.
pub fn regenerate_boot(workspace: &Workspace) -> Result<Vec<String>, WorkspaceError> {
    regenerate_boot_with_spec_format(workspace, SpecFormat::Mixed)
}

/// Regenerate the durable lock-named dependency tree in the selected format.
pub fn regenerate_boot_with_spec_format(
    workspace: &Workspace,
    spec_format: SpecFormat,
) -> Result<Vec<String>, WorkspaceError> {
    regenerate_boot_traced(workspace, spec_format, None)
}

/// [`regenerate_boot_with_spec_format`] under one borrowed trace run — the
/// sibling the ready-install / scoped-update / reinstall paths hand their
/// already-open recorder to.
pub fn regenerate_boot_traced(
    workspace: &Workspace,
    spec_format: SpecFormat,
    trace: Option<&crate::compile_trace::TraceRun>,
) -> Result<Vec<String>, WorkspaceError> {
    // PROP-012 §2.4 — reject a malformed instruction-file block before
    // any boot-artifact write.
    let resolution = read_durable_resolution(&workspace.root)?;
    validate_redirect_blocks(workspace, &resolution)?;
    regenerate_boot_from_traced(workspace, &resolution, spec_format, trace)
}

mod verify;
pub use verify::{verify_boot_graph, verify_boot_graph_with_spec_format};

mod own_boot;
pub(crate) use own_boot::PreparedNodeBoot;
pub(crate) fn node_own_boot(
    node_dir: &Path,
    node_rel: &str,
) -> Result<Vec<AuthoredBoot>, WorkspaceError> {
    own_boot::unbound_entries(node_dir, node_rel)
}

/// Validate each published node's agent instruction files before any mutation
/// (PROP-012 §2.4): a malformed `<vibevm>` block aborts the operation
/// here — ahead of materialisation or any boot-artifact write — so an
/// install never half-applies. A missing instruction file is fine; it is
/// created on write.
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-012#plan-time",
    r = 2
)]
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-012#PLAN-TIME-VALIDATION"
)]
pub(super) fn validate_redirect_blocks(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
) -> Result<(), WorkspaceError> {
    for (rel, manifest) in workspace.iter_nodes() {
        if !publish_node_boot(rel, manifest, resolution) {
            continue;
        }
        let node_dir = workspace.node_abs_path(rel);
        for name in boot_artifacts::REDIRECT_FILES {
            let path = node_dir.join(name);
            let content = match fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(io_err(&path, e)),
            };
            if let boot_artifacts::BlockLocation::Malformed(reason) =
                boot_artifacts::locate_block(&content)
            {
                return Err(WorkspaceError::MalformedRedirectBlock { path, reason });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Authored boot discovery accepts both spec serialisations and skips strays.
    #[test]
    fn authored_discovery_finds_the_xml_form() {
        let dir = tempfile::tempdir().expect("tempdir");
        let boot = dir.path().join(layout::current_boot_dir());
        fs::create_dir_all(&boot).expect("mkdir");
        fs::write(boot.join("00-core.md"), "# core\n").expect("write");
        fs::write(
            boot.join("extra.xml"),
            "<spec xmlns=\"https://vibevm.org/spec/1\"/>",
        )
        .expect("write");
        fs::write(boot.join("notes.txt"), "x").expect("write");
        let own = node_own_boot(dir.path(), ".").expect("discover");
        let paths: Vec<&str> = own.iter().map(|b| b.path.as_str()).collect();
        assert_eq!(
            paths,
            vec![
                layout_paths::boot("00-core.md"),
                layout_paths::boot("extra.xml")
            ]
        );
    }

    /// A duplicate Markdown/XML stem stops discovery and names both files.
    #[test]
    fn authored_discovery_rejects_a_document_in_both_forms() {
        let dir = tempfile::tempdir().expect("tempdir");
        let boot = dir.path().join(layout::current_boot_dir());
        fs::create_dir_all(&boot).expect("mkdir");
        fs::write(boot.join("dup.md"), "# d\n").expect("write");
        fs::write(
            boot.join("dup.xml"),
            "<spec xmlns=\"https://vibevm.org/spec/1\"/>",
        )
        .expect("write");
        let err = node_own_boot(dir.path(), ".").expect_err("collision");
        let text = format!("{err:#}");
        assert!(text.contains("dup.md"), "{text}");
        assert!(text.contains("dup.xml"), "{text}");
        assert!(text.contains("one document, one form"), "{text}");
    }
}

#[cfg(test)]
mod declared_tests;

#[cfg(test)]
mod own_boot_system_tests;

#[cfg(test)]
mod own_boot_tests;
