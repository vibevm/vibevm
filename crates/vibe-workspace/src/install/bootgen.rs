//! Boot-artifact (re)generation — the boot half of PROP-009's loading model.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#install");

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::Path;

use specmark::spec;
use vibe_core::manifest::{BootCategory, LinkType, Manifest, SpecFormat};
use vibe_core::{Group, layout};

use crate::boot::hybrid::{UnitId, fingerprint, hoist};
use crate::boot::{self, AuthoredBoot, DependencyBoot, NodeBootInputs};
use crate::extension_world::{
    ExtensionWorldEpoch, LoweredOwnerRuntimes, OwnerRuntimeLowering, lower_owner_runtimes,
};
use crate::{Workspace, WorkspaceError, boot_artifacts, layout_paths, path_to_slash, vibedeps};

use super::{ResolvedDep, io_err};

/// Durable-world owner-plan lowering along its own responsibility.
#[path = "bootgen/owner_plans.rs"]
mod owner_plans;
use owner_plans::plan_digest_frames;

#[path = "bootgen/hybrid_emit.rs"]
mod hybrid_emit;
use hybrid_emit::{append_hoisted, build_unit_table, emit_package_units, verify_fingerprints};

/// The R4.3 lane analyzer's write-free entry — one selected node's lane
/// composed and compiled under the analyzer observer.
#[path = "bootgen/analyze.rs"]
mod analyze;
#[cfg(test)]
pub(crate) use analyze::analyze_effective_bound_native;
pub use analyze::{
    AnalyzedBoundLane, AnalyzedLane, analyze_node_lane, analyze_node_lane_bound_native,
};

/// The workspace root's B-031 `<group>/<name>` self coordinate, when declared.
mod materialised_read;
pub(super) use materialised_read::read_durable_resolution;
mod conditions;
mod snippet_source;
mod transitive;
use conditions::{active_snippet, installed_identities};
use transitive::static_transitive_closure;

mod desubstitute;
pub(crate) mod native_managed;
pub(crate) mod replay_prepare;
pub(crate) mod replay_publish;
pub use desubstitute::desubstitute_covered_units;

fn root_self_coordinate(root_manifest: &Manifest) -> vibe_spec::SelfCoordinate {
    // Role-blind: a package-rooted checkout compiles its lanes exactly as a
    // project does (PROP-024 ##MANIFEST-ROLES-ARE-EQUIPOTENT).
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

/// The traced sibling of [`regenerate_boot_from_with_spec_format`]: one
/// borrowed run, carried through package-unit emission first, then every
/// root/member node. The run is BORROWED — this layer never opens, finishes
/// or clones it into an outcome; the command owner does all three.
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
    // B-031 self is always the workspace root coordinate.
    let self_coord = root_self_coordinate(&workspace.root_manifest);

    // Exact supplied world; Ready never consults the still-old ambient lock.
    let world = ExtensionWorldEpoch::from_resolution(&workspace.root, resolution)
        .map_err(owner_plans::world_error)?;

    // The per-unit compiler (PROP-038 §2.1): emit each materialised package's
    // own STATIC.md / INDEX.md from its own edges, and learn which packages
    // statically link a child (`with_static`) — a node's dynamic edge to such
    // a package points at its compiled STATIC.md so the whole zone loads, not
    // just the snippet. For a tree with no intermediate static edge this is a
    // no-op, keeping the node artifacts byte-identical (PROP-038 §5).
    let table = build_unit_table(&workspace.root, resolution);
    // Lower every owner before plan digests feed unit fingerprints.
    let runtimes = lower_owner_runtimes(workspace, &world, lowering)?;
    // Boot-graph fingerprints (PROP-038 §2.7) drive the dirty-subgraph skip in
    // per-unit emission (§2.8) — a package whose fingerprint is unchanged is
    // not recompiled. Keyed on each unit's resolved version, plus the owner
    // plan frame of every unit whose owner activated something.
    let versions: HashMap<UnitId, String> = resolution
        .iter()
        .map(|d| ((d.group.clone(), d.name.clone()), d.version.to_string()))
        .collect();
    let fps = fingerprint::fingerprints(&table, &versions, &plan_digest_frames(&runtimes));
    // Soft hoisting (PROP-038 §2.4): a package soft-statically linked by two or
    // more units is `shared` — hoisted to the global root STATIC.md and linked
    // once, its local zones left a #use marker. `pulls` also feeds the
    // shared-by hint. For a tree with no shared package this is all empty.
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
            &with_static,
            spec_format,
        );
        let mut effective = boot::compute_effective_boot(NodeBootInputs {
            own_boot: &own,
            inherited_foundation: &inherited,
            dependencies: &deps,
            default_link: manifest.boot.default_link,
        })?;
        // The absolute root is the hoist point: it carries the single copy of
        // every shared package (PROP-038 §2.4).
        // <!-- REVIEW: DRIFT-030 §4 step 1 — this append is a SECOND write path
        // into the root's static lane. `compute_effective_boot` above has
        // already emitted a static entry for any package in the root's own
        // static closure (`node_dependency_boot` → `boot.rs:246-288`, path from
        // `bootgen.rs:305-310`), and `render_static` concatenates entry by
        // entry with no dedup on `path` (`boot_artifacts.rs:224-261`). So a
        // package that is both hoisted and in the root's closure lands twice.
        // Measured on a fixture of vibevm's shape (root --static-transitive-->
        // content-minimal aggregator --static--> member): counting the
        // entry-point node in `hoist::soft_static_pulls` does clear the
        // aggregator's copy — its zone degrades to the `#use` marker §2.5
        // designs — but the root then holds the member twice, once per path.
        // PROP-038 `##HOIST-LCA` explains why the collision is structural here:
        // the hoist target is the LCA of a *continuous static zone*, and for an
        // unbroken root→aggregator→member static chain that LCA IS the root,
        // i.e. the hoist destination and the root's own compile site coincide.
        // Which mechanism owns the dedup is a design question, so DRIFT-030
        // stopped on its §8 rather than paper over it at this call. -->
        if rel == "." {
            append_hoisted(&mut effective, &shared, &table, &pulls);
        }
        // B-006 (lane dedup): roll a substituted unit-STATIC entry back to the
        // package's own snippet — or elide a contentless umbrella — once every
        // boot-bearing member of its zone is present individually in this
        // lane. A pure pass over the composition and the unit table; coverage
        // is never lost (a member missing from the lane keeps the
        // substitution in place). Sits after `append_hoisted` so the hoisted
        // single-copies count as present, and before the artifact write so the
        // rendered lane is the once-each form.
        desubstitute_covered_units(&mut effective, &table);
        // This node's OWN lane plan (PROP-054 ##COMPILE-ACTIVATION). Root and
        // members take distinct host seats over the same parsed package epoch;
        // no member reparses or reorders the installed world.
        boot_artifacts::write_boot_artifacts_traced(
            &node_dir,
            rel,
            &workspace.root,
            &self_coord,
            &effective,
            spec_format,
            trace,
            runtimes.node(rel)?.transform_plan().clone(),
        )?;
        nodes_regenerated.push(rel.to_string());
    }
    Ok(BootRegeneration {
        nodes: nodes_regenerated,
        runtimes,
    })
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
    validate_redirect_blocks(workspace)?;
    let resolution = read_durable_resolution(&workspace.root)?;
    regenerate_boot_from_traced(workspace, &resolution, spec_format, trace)
}

/// Verify the per-unit boot artifacts are current (PROP-038 §3) — the integrity
/// half of `vibe check`, reconstructing the resolution from the materialised
/// dependency tree. Returns the stale units: a package that statically links a
/// child but whose on-disk fingerprint is missing or mismatched, i.e. one the
/// dirty-subgraph should have regenerated. An empty result means the boot graph
/// is consistent.
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

/// Discover a node's own authored boot files — every spec source in its
/// live boot lane (`.md` or dialect `.xml`, PROP-045 ##LOADER-LAW), minus
/// the generated `STATIC.md` / `INDEX.md`. The user-owned `00-core.md` /
/// `90-user.md` are `Foundation` / `UserOverride` by name convention; any
/// other authored file is mid-band (`None`).
///
/// One document, one form (PROP-045 ##TARGET-MIXED): `X.md` + `X.xml` in
/// one boot dir is a split brain — an error naming both, never a guess.
///
/// `pub(crate)` so [`crate::publish`] can reuse it to regenerate a
/// staged copy's boot artifacts for the published shape (PROP-009 §2.11).
pub(crate) fn node_own_boot(
    node_dir: &Path,
    node_rel: &str,
) -> Result<Vec<AuthoredBoot>, WorkspaceError> {
    let boot_dir = node_dir.join(layout::current_boot_dir());
    if !boot_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    let mut spec_paths = Vec::new();
    for entry in fs::read_dir(&boot_dir).map_err(|e| io_err(&boot_dir, e))? {
        let entry = entry.map_err(|e| io_err(&boot_dir, e))?;
        let path = entry.path();
        if !entry.file_type().map_err(|e| io_err(&path, e))?.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if !vibe_specdoc::is_spec_source(&path) {
            continue;
        }
        // The generated artifacts are not authored boot.
        if name == boot_artifacts::static_file(SpecFormat::Mixed)
            || name == boot_artifacts::static_file(SpecFormat::Xml)
            || name == boot_artifacts::INDEX_FILE
        {
            continue;
        }
        let category = match name.as_str() {
            "00-core.md" => Some(BootCategory::Foundation),
            "90-user.md" => Some(BootCategory::UserOverride),
            _ => None,
        };
        let rel_path = if node_rel == "." {
            layout_paths::boot(&name)
        } else {
            path_to_slash(
                &Path::new(node_rel)
                    .join(layout::current_boot_dir())
                    .join(&name),
            )
        };
        spec_paths.push(path.clone());
        files.push(AuthoredBoot {
            path: rel_path,
            category,
            origin: node_rel.to_string(),
        });
    }
    if let Some(collision) = vibe_specdoc::pair_collisions_in(&spec_paths).first() {
        let rel = collision
            .markdown
            .strip_prefix(node_dir)
            .unwrap_or(&collision.markdown)
            .to_path_buf();
        return Err(WorkspaceError::Io {
            path: rel,
            reason: collision.message(),
        });
    }
    // Deterministic order — the engine keeps a band's collection order.
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

/// Build the dependency-boot inputs for one node: the transitive closure
/// of its `[requires]` within `resolution`, each turned into a
/// [`DependencyBoot`]. Each snippet's path is resolved LOGICALLY against
/// the materialised slot ([`resolve_snippet_source`], PROP-045
/// ##BOOT-LANE-SCOPE) so an XML-materialised dependency's boot path — and
/// the INDEX entry that carries it — names the file that exists.
fn node_dependency_boot(
    workspace_root: &Path,
    node_manifest: &Manifest,
    resolution: &[ResolvedDep],
    with_static: &HashSet<UnitId>,
    spec_format: SpecFormat,
) -> Vec<DependencyBoot> {
    let installed = installed_identities(resolution);
    let index: HashMap<(&Group, &str), &ResolvedDep> = resolution
        .iter()
        .map(|d| ((&d.group, d.name.as_str()), d))
        .collect();

    // The inline-transitive closure (PROP-035 §12): every package reached
    // through a direct edge the consumer declared `inline-transitive` — the
    // edge's target and its whole `requires` closure — is forced `inline`.
    let forced_inline = static_transitive_closure(node_manifest, &index);

    // Breadth-first transitive closure from the node's direct requires.
    // A `[requires.packages]` key is group-qualified (PROP-008 §2.6), so
    // every `iter_pkgrefs` entry carries a group.
    let mut visited: HashSet<(Group, String)> = HashSet::new();
    let mut queue: VecDeque<(Group, String)> = node_manifest
        .requires
        .iter_pkgrefs()
        .filter_map(|(g, n)| g.map(|g| (g.clone(), n.to_string())))
        .collect();
    let mut closure: Vec<&ResolvedDep> = Vec::new();
    while let Some((group, name)) = queue.pop_front() {
        if !visited.insert((group.clone(), name.clone())) {
            continue;
        }
        if let Some(dep) = index.get(&(&group, name.as_str())) {
            closure.push(dep);
            for (rg, rn) in &dep.requires {
                queue.push_back((rg.clone(), rn.clone()));
            }
        }
    }

    closure
        .iter()
        .map(|dep| {
            // An in-place dependency's boot snippet lives in its unversioned
            // slot (PROP-022 §2.4); a copy/hardlink dep's in the versioned
            // one. Field access auto-derefs the `&&ResolvedDep`.
            let in_place = dep
                .manifest
                .package
                .as_ref()
                .is_some_and(|p| p.materialization.is_in_place());
            let slot = if in_place {
                vibedeps::in_place_slot_rel_path(&dep.group, &dep.name)
            } else {
                vibedeps::slot_rel_path(&dep.group, &dep.name, &dep.version)
            };
            let snippet = dep.manifest.boot_snippet.as_ref();
            let active = active_snippet(workspace_root, &slot, snippet, &installed);
            let main = active.main;
            let all_fragments = active.fragments;
            // PROP-038 §2.1: a dependency that statically links a child is read
            // through its compiled STATIC.md (carrying the whole zone), not its
            // raw snippet. A leaf keeps pointing at the snippet (byte-compat).
            // B-006: remember which entries had their path substituted up to a
            // unit-STATIC — `desubstitute_covered_units` rolls the substitution
            // back (or elides it) once the zone is covered member-by-member.
            let (boot_path, when, fragments, unit_substituted) =
                if with_static.contains(&(dep.group.clone(), dep.name.clone())) {
                    (
                        Some(path_to_slash(
                            &Path::new(&slot)
                                .join(layout::current_boot_dir())
                                .join(boot_artifacts::static_file(spec_format)),
                        )),
                        main.as_ref()
                            .and_then(|contribution| contribution.when.clone()),
                        all_fragments
                            .into_iter()
                            .filter(|fragment| fragment.when.is_some())
                            .collect(),
                        true,
                    )
                } else {
                    (
                        main.as_ref().map(|contribution| contribution.path.clone()),
                        main.and_then(|contribution| contribution.when),
                        all_fragments,
                        false,
                    )
                };
            DependencyBoot {
                kind: dep.kind,
                group: dep.group.clone(),
                name: dep.name.clone(),
                boot_path,
                fragments,
                category: snippet.and_then(|bs| bs.category),
                // An `inline-transitive` edge (or membership in one's closure)
                // forces `inline` (PROP-035 §12); otherwise only a direct
                // requirement carries a consumer-declared `link` and a
                // transitive dependency reads back as `None`.
                declared_link: if forced_inline.contains(&(dep.group.clone(), dep.name.clone())) {
                    Some(LinkType::Static)
                } else {
                    node_manifest.requires.declared_link(&dep.group, &dep.name)
                },
                suggested_link: snippet.and_then(|bs| bs.link),
                // Only an `os:*` predicate can remain after generation-time
                // `installed:*` resolution.
                when,
                requires: dep.requires.clone(),
                // PROP-035 §3 — the package's declared format. A `normal`
                // dependency pulled `static` is compiled to its closure by
                // `render_static` (PROP-035 §8); absent a `[package]` table,
                // it defaults to `simple` (verbatim, fail-safe).
                format: dep
                    .manifest
                    .package
                    .as_ref()
                    .map(|p| p.format)
                    .unwrap_or_default(),
                unit_substituted,
            }
        })
        .collect()
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
pub(super) fn validate_redirect_blocks(workspace: &Workspace) -> Result<(), WorkspaceError> {
    for (rel, _) in workspace.iter_nodes() {
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
