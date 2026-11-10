//! The provider-free node generation both read-only consumers share.
//!
//! This is the composition `regenerate_boot_from_traced` runs for one node —
//! the same cells, in place, in the same order — up to but not including the
//! compile, and with no write of any kind. It was the preamble of
//! [`super::analyze_node_lane`]; it is out of line here because a **second**
//! consumer needs exactly the same thing. `vibe prompt build` must publish a
//! node's system-prompt artifacts against the node's own effective boot, its
//! prepared sources and its selected `STATIC` spelling, and re-deriving any
//! of the three would be a second opinion about what one generation contains
//! (PROP-076 `#SYS-OPERATIONS-SURFACE`).
//!
//! **No provider is accepted and none can be reached.** The owner-plan rules
//! are the boot seam's own: the exact durable lock snapshot is the explicit
//! epoch authority, and every world, closure or owner refusal propagates.
//! Only a missing durable lock is the explicit empty sequence; malformed
//! lock or slot state refuses, which is a world-integrity refusal rather
//! than a freshness one (`#SYS-READ-ONLY-CHECK`).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#install");

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

use vibe_core::manifest::{Manifest, SpecFormat};
use vibe_spec::{SelfCoordinate, TransformPlan};

use crate::boot;
use crate::boot::EffectiveBoot;
use crate::boot::hybrid::hoist;
use crate::boot_artifacts::declared_sources::DeclaredBootSources;
use crate::errors::WorkspaceError;
use crate::extension_world::{ExtensionWorldEpoch, OwnerRuntimeLowering, lower_owner_runtimes};
use crate::{Workspace, boot_artifacts};

use super::super::ResolvedDep;
use super::super::hybrid_emit::{append_hoisted, with_static_set};
use super::super::owner_plans::world_error;
use super::super::{desubstitute_covered_units, node_dependency_boot, read_durable_resolution};

/// One node's generation as far as it can be taken without compiling.
pub(crate) struct NodeGeneration {
    /// The node's absolute directory.
    pub(crate) node_dir: PathBuf,
    /// The node's own manifest — the authority for `[system_prompt]`.
    pub(crate) manifest: Manifest,
    /// The workspace root.
    pub(crate) root: PathBuf,
    /// The root's self coordinate.
    pub(crate) self_coord: SelfCoordinate,
    /// The selected `STATIC` spelling this node publishes under.
    pub(crate) spec_format: SpecFormat,
    /// The prepared boot sources, through which every authored input is
    /// observed under one guard.
    pub(crate) sources: Option<DeclaredBootSources>,
    /// The node's computed effective boot.
    pub(crate) effective: EffectiveBoot,
    /// The node's own owner-scoped transform plan.
    pub(crate) transforms: TransformPlan,
    /// Each static input's display provenance `(origin, path)`, in input
    /// order.
    pub(crate) identities: Vec<(String, String)>,
}

/// Take one node's generation up to the compile boundary, writing nothing.
pub(crate) fn node_generation(
    workspace: &Workspace,
    node_rel: &str,
    input_format: Option<SpecFormat>,
) -> Result<NodeGeneration, WorkspaceError> {
    let node_dir = workspace.node_abs_path(node_rel);
    let node_manifest = workspace
        .member_by_rel_path(node_rel)
        .map(|member| member.manifest.clone())
        .unwrap_or_else(|| workspace.root_manifest.clone());
    let root = workspace.root.clone();
    let self_coord = super::super::root_self_coordinate(&workspace.root_manifest);

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
    let mut sources = boot_artifacts::declared_sources::DeclaredBootSources::prepare_with_nodes(
        workspace,
        &resolution,
        input_format,
    )?;
    let targets = super::super::conditions::project_targets(&workspace.root_manifest);
    let table = super::super::build_unit_table_bound(
        &root,
        &resolution,
        targets.clone(),
        sources.as_ref(),
    )?;
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
    let own = sources
        .as_ref()
        .ok_or_else(|| WorkspaceError::Io {
            path: node_dir.clone(),
            reason: "prepared nodes missing".into(),
        })?
        .node(node_rel)?
        .entries
        .clone();
    let inherited = if node_rel == "." {
        Vec::new()
    } else {
        sources
            .as_ref()
            .ok_or_else(|| WorkspaceError::Io {
                path: root.clone(),
                reason: "prepared nodes missing".into(),
            })?
            .node(".")?
            .entries
            .clone()
            .into_iter()
            .filter(|b| b.category == Some(vibe_core::manifest::BootCategory::Foundation))
            .collect()
    };
    let dependencies = node_dependency_boot(
        &root,
        &node_manifest,
        &resolution,
        &table,
        &with_static,
        spec_format,
        targets.clone(),
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
    super::super::generated_inputs::admit(
        &mut sources,
        &effective,
        &resolution,
        &table,
        &runtimes,
        spec_format,
        &self_coord,
    )?;

    let identities: Vec<(String, String)> = effective
        .static_entries()
        .map(|entry| (entry.origin.clone(), entry.path.clone()))
        .collect();

    // The node lane's own owner-scoped plan. Root and members take distinct
    // host seats over this same exact parsed package epoch.
    let transforms = runtimes.node(node_rel)?.transform_plan().clone();
    Ok(NodeGeneration {
        node_dir,
        manifest: node_manifest,
        root,
        self_coord,
        spec_format,
        sources,
        effective,
        transforms,
        identities,
    })
}
