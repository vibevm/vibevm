//! Boot-artifact (re)generation — the boot half of PROP-009's loading model.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#install");

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::Path;

use specmark::spec;
use vibe_core::manifest::{BootCategory, LinkType, Manifest, SpecFormat};
use vibe_core::{Group, layout};

use crate::boot::hybrid::{UnitId, UnitInput, fingerprint, hoist};
use crate::boot::{self, AuthoredBoot, DependencyBoot, NodeBootInputs};
use crate::extension_world::{
    ExtensionWorldEpoch, LoweredOwnerRuntimes, OwnerRuntimeLowering, lower_owner_runtimes,
};
#[cfg(test)]
use crate::layout_paths;
use crate::{Workspace, WorkspaceError, boot_artifacts, path_to_slash, vibedeps};

use super::{ResolvedDep, io_err};

/// Durable-world owner-plan lowering along its own responsibility.
#[path = "bootgen/owner_plans.rs"]
mod owner_plans;
use owner_plans::{plan_digest_frames, plan_digest_frames_for};
#[cfg(test)]
#[path = "bootgen/compile_plan_tests.rs"]
mod compile_plan_tests;
#[cfg(test)]
#[path = "bootgen/selected_node_tests.rs"]
mod selected_node_tests;

#[path = "bootgen/hybrid_emit.rs"]
mod hybrid_emit;
use hybrid_emit::{append_hoisted, build_unit_table, emit_package_units, verify_fingerprints};

#[path = "bootgen/analyze.rs"]
mod analyze;
#[cfg(test)]
pub(crate) use analyze::analyze_effective_bound_native;
pub use analyze::{
    AnalyzedBoundLane, AnalyzedLane, analyze_node_lane, analyze_node_lane_bound_native,
    compile_node_backend,
};

mod materialised_read;
pub(super) use materialised_read::read_durable_resolution;
pub(crate) mod captured;
mod conditions;
pub mod effective_subskills;
mod node_dependency;
mod publication;
mod snippet_source;
mod transitive;
use conditions::{active_snippet, installed_identities};
use node_dependency::node_dependency_boot;
use publication::publish_node_boot;
use transitive::static_transitive_closure;

mod desubstitute;
pub(crate) mod native_managed;
pub(crate) mod replay_prepare;
pub(crate) mod replay_publish;
pub use desubstitute::desubstitute_covered_units;

fn root_self_coordinate(root_manifest: &Manifest) -> vibe_spec::SelfCoordinate {
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
    regenerate_boot_from_traced_prepared_configuration(
        workspace,
        resolution,
        spec_format,
        trace,
        lowering,
        None,
    )
}

pub fn regenerate_boot_from_traced_prepared_configuration(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
    spec_format: SpecFormat,
    trace: Option<&crate::compile_trace::TraceRun>,
    lowering: OwnerRuntimeLowering,
    configuration: Option<&vibe_core::manifest::LockedConfiguration>,
) -> Result<BootRegeneration, WorkspaceError> {
    // B-031 self is always the workspace root coordinate.
    let self_coord = root_self_coordinate(&workspace.root_manifest);

    // Exact supplied world; Ready never consults the still-old ambient lock.
    let world = ExtensionWorldEpoch::from_resolution(&workspace.root, resolution)
        .map_err(owner_plans::world_error)?;

    // Compile package units first so node edges can target complete zones.
    let table = build_unit_table(&workspace.root, resolution);
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
        trace,
        &runtimes,
    )?;

    // The absolute root's foundation boot — inherited by every member
    // (PROP-009 §2.2: inherited foundation flows down).
    let root_foundation: Vec<AuthoredBoot> = node_own_boot(&workspace.root, ".")?
        .into_iter()
        .filter(|b| b.category == Some(BootCategory::Foundation))
        .collect();

    let mut nodes_regenerated = Vec::new();
    for (rel, manifest) in workspace.iter_nodes() {
        if !publish_node_boot(rel, manifest, resolution) {
            continue;
        }
        let node_dir = workspace.node_abs_path(rel);
        let own = node_own_boot(&node_dir, rel)?;
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
        effective_subskills::append_loading(
            &workspace.root,
            resolution,
            &deps,
            configuration,
            &mut effective,
        )?;
        // Each node receives its own retained owner plan.
        boot_artifacts::write_boot_artifacts_traced(
            &node_dir,
            rel,
            &workspace.root,
            &self_coord,
            &effective,
            spec_format,
            trace,
            runtimes.node(rel)?.compile_plans().clone(),
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
    configuration: Option<&vibe_core::manifest::LockedConfiguration>,
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
    let regenerated = native_managed::regenerate_boot_from_bound_native_configuration(
        workspace,
        resolution,
        spec_format,
        trace,
        &epoch,
        Some(&mut provider),
        configuration,
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
    let configuration = effective_subskills::durable_configuration(&workspace.root)?;
    regenerate_boot_from_traced_prepared_configuration(
        workspace,
        &resolution,
        spec_format,
        trace,
        OwnerRuntimeLowering::compatibility_root_without_presets(),
        configuration.as_ref(),
    )
    .map(|prepared| prepared.nodes)
}

/// Return package units whose durable boot fingerprints are stale.
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-038#tests",
    r = 1
)]
pub fn verify_boot_graph(workspace: &Workspace) -> Result<Vec<UnitId>, WorkspaceError> {
    let resolution = read_durable_resolution(&workspace.root)?;
    let world = ExtensionWorldEpoch::from_resolution(&workspace.root, &resolution)
        .map_err(owner_plans::world_error)?;
    let table = build_unit_table(&workspace.root, &resolution);
    let versions: HashMap<UnitId, String> = resolution
        .iter()
        .map(|d| ((d.group.clone(), d.name.clone()), d.version.to_string()))
        .collect();
    // The check half constructs the SAME explicit world the generate half
    // does from the exact materialised resolution, and
    // frames the same owner-plan digests (R4 architecture §7.1). Recomputing
    // without them would call every unit whose owner activates a transform
    // stale on a tree the generator had just left fresh.
    let runtimes = lower_owner_runtimes(
        workspace,
        &world,
        OwnerRuntimeLowering::compatibility_root_without_presets(),
    )?;
    verify_fingerprints(
        &workspace.root,
        &resolution,
        &table,
        &versions,
        &plan_digest_frames(&runtimes),
        &runtimes,
    )
}

/// Discover authored node boot sources, rejecting split Markdown/XML forms.
pub(crate) fn node_own_boot(
    node_dir: &Path,
    node_rel: &str,
) -> Result<Vec<AuthoredBoot>, WorkspaceError> {
    let boot_dir = node_dir.join(layout::current_boot_dir());
    if !boot_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(&boot_dir).map_err(|e| io_err(&boot_dir, e))? {
        let entry = entry.map_err(|e| io_err(&boot_dir, e))?;
        let path = entry.path();
        if entry.file_type().map_err(|e| io_err(&path, e))?.is_file() {
            files.push(path.strip_prefix(node_dir).unwrap_or(&path).to_path_buf());
        }
    }
    captured::authored_boot_from_inventory(node_dir, node_rel, &files)
}

/// Validate every node's agent instruction files before any mutation
/// (PROP-012 §2.4): a malformed `<vibevm>` block aborts the operation
/// here — ahead of materialisation or any boot-artifact write — so an
/// install never half-applies. A missing instruction file is fine; it is
/// created on write.
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-012#plan-time",
    r = 1
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

/// Removal already has materialized retained slots. Project exactly those
/// rows through the incumbent durable reader and lower their owner plans before
/// the first manifest, lock or slot mutation.
pub fn preflight_removal(
    workspace: &Workspace,
    desired: &vibe_core::manifest::Lockfile,
    spec_format: SpecFormat,
) -> Result<(), WorkspaceError> {
    let resolution = desired
        .packages
        .iter()
        .map(|package| materialised_read::durable_row(&workspace.root, package))
        .collect::<Result<Vec<_>, _>>()?;
    preflight_resolution_with_configuration(
        workspace,
        &resolution,
        spec_format,
        desired.meta.configuration.as_ref(),
    )?;
    let world = ExtensionWorldEpoch::from_resolution(&workspace.root, &resolution)
        .map_err(owner_plans::world_error)?;
    let _ = lower_owner_runtimes(
        workspace,
        &world,
        OwnerRuntimeLowering::compatibility_root_without_presets(),
    )?;
    Ok(())
}

/// Read-only structural preflight over the complete planned world. Reuses the
/// normal composition; it does not execute native providers
/// or claim that external hook effects have been validated or compensated.
pub fn preflight_resolution(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
    spec_format: SpecFormat,
) -> Result<(), WorkspaceError> {
    preflight_resolution_with_configuration(workspace, resolution, spec_format, None)
}

/// Validate the exact captured selected exports before materialization. This
/// uses source content, never a not-yet-created slot or a replacement epoch.
pub fn preflight_resolution_with_configuration(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
    spec_format: SpecFormat,
    configuration: Option<&vibe_core::manifest::LockedConfiguration>,
) -> Result<(), WorkspaceError> {
    validate_redirect_blocks(workspace, resolution)?;
    for dep in resolution {
        let package = format!("{}/{}@{}", dep.group, dep.name, dep.version);
        let fail = |reason| WorkspaceError::SlotLifecycle {
            phase: "preflight",
            package: package.clone(),
            reason,
        };
        let exports = effective_subskills::selected_exports(
            configuration,
            &package,
            &dep.source_hash
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
        )
        .map_err(&fail)?;
        effective_subskills::capture_selected_subskills(&dep.content_dir, &exports)
            .map_err(fail)?;
    }
    let identities = resolution
        .iter()
        .map(|dep| (dep.group.clone(), dep.name.clone()))
        .collect::<HashSet<_>>();
    if identities.len() != resolution.len()
        || resolution
            .iter()
            .any(|dep| dep.requires.iter().any(|edge| !identities.contains(edge)))
    {
        return Err(WorkspaceError::Io {
            path: workspace.root.clone(),
            reason: "incomplete or duplicate planned dependency graph; refusing before writes"
                .into(),
        });
    }
    let table = build_unit_table(&workspace.root, resolution);
    let with_static = hybrid_emit::with_static_set(&table);
    let inherited = node_own_boot(&workspace.root, ".")?
        .into_iter()
        .filter(|b| b.category == Some(BootCategory::Foundation))
        .collect::<Vec<_>>();
    for (rel, manifest) in workspace.iter_nodes() {
        let own = node_own_boot(&workspace.node_abs_path(rel), rel)?;
        let dependencies = node_dependency_boot(
            &workspace.root,
            manifest,
            resolution,
            &table,
            &with_static,
            spec_format,
        );
        boot::compute_effective_boot(NodeBootInputs {
            own_boot: &own,
            inherited_foundation: if rel == "." { &[] } else { &inherited },
            dependencies: &dependencies,
            default_link: manifest.boot.default_link,
        })?;
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
