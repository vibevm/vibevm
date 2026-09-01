//! The R4.3 lane analyzer's workspace entry: compose and compile ONE
//! selected node's static lane under the analyzer observer, writing
//! nothing (packages-2026-09 architecture §9, the frozen §9.1 ruling).
//!
//! This is the composition `regenerate_boot_from_traced` runs for one
//! node — the same cells, in place, in the same order — minus every
//! write: no unit artifacts are emitted, no INDEX, no transaction. The
//! observer is threaded to the ONE compiler call exactly as T10C
//! threaded the owner plan, so the evidence the CLI lowers into its
//! report is the evidence of the same compile regeneration would run.
//!
//! What comes back is everything the report needs and nothing else: the
//! emitted artifact (byte-equal to what the write path would publish),
//! the typed provider that declared each input (in input order — the
//! attribution side a lane contribution cannot carry), and each input's
//! display `(origin, path)` for the value-level alignment fence between
//! the plan the workspace authored and the evidence the compiler
//! returned.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#install");

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use vibe_core::manifest::SpecFormat;
use vibe_spec::{CompileObserver, DocumentProvider, EmittedArtifact};

use crate::boot;
use crate::boot::hybrid::hoist;
use crate::errors::WorkspaceError;
use crate::extension_world::{
    ExtensionWorldEpoch, OwnerNativeCompileProvider, OwnerRuntimeEpoch, OwnerRuntimeLowering,
    lower_owner_runtimes,
};
use crate::{Workspace, boot_artifacts};

use super::ResolvedDep;
use super::hybrid_emit::{append_hoisted, with_static_set};
use super::owner_plans::world_error;
use super::{
    build_unit_table, desubstitute_covered_units, node_dependency_boot, node_own_boot,
    read_durable_resolution,
};

/// One analyzed lane: the compile's result beside its attribution side.
#[derive(Debug, Clone)]
pub struct AnalyzedLane {
    /// The emitted artifact — byte-equal to what regeneration would
    /// write, held in memory; nothing was written.
    pub artifact: EmittedArtifact,
    /// The typed provider that declared each plan input, in input order.
    /// `None` marks an elided/hoisted seat whose provenance does not type
    /// through the one grammar — an unattributable seat the analyzer
    /// refuses rather than half-reports.
    pub providers: Vec<Option<DocumentProvider>>,
    /// Each input's display provenance `(origin, path)`, in input order,
    /// for the alignment fence between the authored plan and the
    /// compiler's returned evidence.
    pub identities: Vec<(String, String)>,
}

pub struct AnalyzedBoundLane {
    pub artifact: EmittedArtifact,
    pub native: Option<boot_artifacts::OwnerNativeCompileContinuation>,
    pub providers: Vec<Option<DocumentProvider>>,
    pub identities: Vec<(String, String)>,
}

/// Compose and compile ONE selected node's static lane under an analyzer
/// observer, writing nothing.
///
/// `node_rel` is the node's canonical workspace-relative path (`.` for
/// the absolute root). `None` is the honest answer for a node with no
/// static-lane contributions — there is no artifact to analyze. The
/// observer receives exactly the events of this one compile (one
/// emission per accepted artifact, one stage delta per lane/emitted
/// transform); a `None` observer compiles the plain historical schedule.
///
/// The owner-plan rules are the boot seam's own (`owner_plans.rs`): the exact
/// durable lock snapshot is the explicit epoch authority, and every world,
/// closure or owner refusal propagates. Only a missing durable lock is the
/// explicit empty sequence; malformed lock/slot state refuses.
pub fn analyze_node_lane(
    workspace: &Workspace,
    node_rel: &str,
    observer: Option<Arc<dyn CompileObserver>>,
) -> Result<Option<AnalyzedLane>, WorkspaceError> {
    let node_dir = workspace.node_abs_path(node_rel);
    let node_manifest = workspace
        .member_by_rel_path(node_rel)
        .map(|member| member.manifest.clone())
        .unwrap_or_else(|| workspace.root_manifest.clone());
    let root = workspace.root.clone();
    let self_coord = super::root_self_coordinate(&workspace.root_manifest);

    // The per-unit table and the two sets the node lane's composition
    // reads from it: which units carry a compiled STATIC (the
    // substitution set) and which shared packages hoist to the root.
    let resolution: Vec<ResolvedDep> = read_durable_resolution(&root)?;
    let world = ExtensionWorldEpoch::from_resolution(&root, &resolution).map_err(world_error)?;
    let runtimes = lower_owner_runtimes(
        workspace,
        &world,
        OwnerRuntimeLowering::new(node_rel, BTreeMap::new()),
    )?;
    let table = build_unit_table(&root, &resolution);
    let with_static = with_static_set(&table);
    let pulls = hoist::soft_static_pulls(&table);
    let shared: HashSet<_> = pulls
        .iter()
        .filter(|(pkg, pullers)| {
            pullers.len() >= 2 && table.get(pkg).is_some_and(|u| u.has_static_boot())
        })
        .map(|(pkg, _)| pkg.clone())
        .collect();

    // The lane's format follows the committed selected STATIC — what the
    // tree's regeneration actually wrote — falling back to Mixed for a
    // node that has not generated one yet.
    let spec_format = match boot_artifacts::resolve_static_path(&node_dir)? {
        Some(path) if path.extension().and_then(|e| e.to_str()) == Some("xml") => SpecFormat::Xml,
        _ => SpecFormat::Mixed,
    };

    // The node's effective boot: own authored files, the root's
    // foundation inherited by every member, the dependency closure with
    // unit-STATIC substitution, the root's hoisted shared packages, and
    // the B-006 once-each dedup — the regeneration order, all of it.
    let own = node_own_boot(&node_dir, node_rel)?;
    let inherited = if node_rel == "." {
        Vec::new()
    } else {
        node_own_boot(&root, ".")?
            .into_iter()
            .filter(|b| b.category == Some(vibe_core::manifest::BootCategory::Foundation))
            .collect()
    };
    let dependencies = node_dependency_boot(
        &root,
        &node_manifest,
        &resolution,
        &with_static,
        spec_format,
    );
    let mut effective = boot::compute_effective_boot(boot::NodeBootInputs {
        own_boot: &own,
        inherited_foundation: &inherited,
        dependencies: &dependencies,
        default_link: node_manifest.boot.default_link,
    })?;
    if node_rel == "." {
        append_hoisted(&mut effective, &shared, &table, &pulls);
    }
    desubstitute_covered_units(&mut effective, &table);

    let identities: Vec<(String, String)> = effective
        .static_entries()
        .map(|entry| (entry.origin.clone(), entry.path.clone()))
        .collect();

    // The node lane's own owner-scoped plan. Root and members take distinct
    // host seats over this same exact parsed package epoch.
    let transforms = runtimes.node(node_rel)?.transform_plan().clone();

    let compiled = boot_artifacts::compile_static_analyzed(
        &effective,
        &root,
        &self_coord,
        spec_format,
        transforms,
        observer,
    )?;
    Ok(compiled.map(|compiled| AnalyzedLane {
        artifact: compiled.artifact,
        providers: compiled.providers,
        identities,
    }))
}

pub fn analyze_node_lane_bound_native<P: OwnerNativeCompileProvider>(
    workspace: &Workspace,
    node_rel: &str,
    resolution: &[ResolvedDep],
    epoch: &OwnerRuntimeEpoch,
    provider: Option<&mut P>,
    observer: Option<Arc<dyn CompileObserver>>,
) -> Result<Option<AnalyzedBoundLane>, WorkspaceError> {
    if epoch.lowered().workspace_root() != workspace.root.as_path() {
        return Err(WorkspaceError::NativeCompileProvider {
            owner: format!("node:{node_rel}"),
            reason: "bound analyzer epoch belongs to a different workspace root".to_owned(),
        });
    }
    epoch.assert_resolution(&workspace.root, resolution)?;
    let node_dir = workspace.node_abs_path(node_rel);
    let node_manifest = workspace
        .member_by_rel_path(node_rel)
        .map(|member| member.manifest.clone())
        .unwrap_or_else(|| workspace.root_manifest.clone());
    let root = workspace.root.clone();
    let self_coord = super::root_self_coordinate(&workspace.root_manifest);
    let table = build_unit_table(&root, resolution);
    let with_static = with_static_set(&table);
    let pulls = hoist::soft_static_pulls(&table);
    let shared: HashSet<_> = pulls
        .iter()
        .filter(|(package, pullers)| {
            pullers.len() >= 2
                && table
                    .get(package)
                    .is_some_and(|unit| unit.has_static_boot())
        })
        .map(|(package, _)| package.clone())
        .collect();
    let spec_format = match boot_artifacts::resolve_static_path(&node_dir)? {
        Some(path) if path.extension().and_then(|extension| extension.to_str()) == Some("xml") => {
            SpecFormat::Xml
        }
        _ => SpecFormat::Mixed,
    };
    let own = node_own_boot(&node_dir, node_rel)?;
    let inherited = if node_rel == "." {
        Vec::new()
    } else {
        node_own_boot(&root, ".")?
            .into_iter()
            .filter(|boot| boot.category == Some(vibe_core::manifest::BootCategory::Foundation))
            .collect()
    };
    let dependencies =
        node_dependency_boot(&root, &node_manifest, resolution, &with_static, spec_format);
    let mut effective = boot::compute_effective_boot(boot::NodeBootInputs {
        own_boot: &own,
        inherited_foundation: &inherited,
        dependencies: &dependencies,
        default_link: node_manifest.boot.default_link,
    })?;
    if node_rel == "." {
        append_hoisted(&mut effective, &shared, &table, &pulls);
    }
    desubstitute_covered_units(&mut effective, &table);
    analyze_effective_bound_native(
        &effective,
        &root,
        &self_coord,
        spec_format,
        epoch,
        node_rel,
        provider,
        observer,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn analyze_effective_bound_native<P: OwnerNativeCompileProvider>(
    effective: &boot::EffectiveBoot,
    workspace_root: &std::path::Path,
    self_coord: &vibe_spec::SelfCoordinate,
    spec_format: SpecFormat,
    epoch: &OwnerRuntimeEpoch,
    node_rel: &str,
    provider: Option<&mut P>,
    observer: Option<Arc<dyn CompileObserver>>,
) -> Result<Option<AnalyzedBoundLane>, WorkspaceError> {
    let identities = effective
        .static_entries()
        .map(|entry| (entry.origin.clone(), entry.path.clone()))
        .collect();
    let mode = match observer {
        Some(observer) => {
            boot_artifacts::native_managed::OwnerNativeCompileMode::Observed(observer)
        }
        None => boot_artifacts::native_managed::OwnerNativeCompileMode::Plain,
    };
    let compiled = boot_artifacts::native_managed::compile_static_owner_managed(
        effective,
        workspace_root,
        self_coord,
        spec_format,
        epoch.node(node_rel)?,
        mode,
        provider,
    )?;
    Ok(compiled.map(|compiled| {
        let (artifact, native, providers) = compiled.into_parts();
        AnalyzedBoundLane {
            artifact,
            native,
            providers,
            identities,
        }
    }))
}
