//! The planning phase — derive the effective root set, run the depsolver,
//! fetch and feature-pin every node, expand conditional dependencies to a
//! fixed point, and shape the resolution the caller confirms before
//! [`apply`](crate::apply) mutates anything beyond the recorded migration writes.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use vibe_core::manifest::{Lockfile, Manifest, SpecFormat};
use vibe_core::visibility::{Analysis, ProvenanceRule};
use vibe_core::{ContentHash, Group, PackageRef, VersionSpec};
use vibe_registry::store;
use vibe_registry::{CachedPackage, ResolvedPackage};
use vibe_resolver::{FeatureRequest, ResolvedNode, conditional::ConditionalPredicate};
use vibe_workspace::Workspace;
use vibe_workspace::freshness::is_in_workspace_file_source;
use vibe_workspace::install::ResolvedDep;

use crate::error::Result;
use crate::events::{PlanEvent, PlanObserver};
use crate::fetched::{
    Fetched, NodeInstallMeta, build_activation_context, build_language_chain,
    load_or_empty_lockfile, tailor_feature_request,
};
use crate::record::exact_pinned_pkgref;
use crate::visibility_projection::resolve_effective;
use crate::{InstallSource, events};

/// What the caller asked to install. `roots` is empty for the
/// install-from-manifest shape; when non-empty, every pkgref is
/// already parsed and group-qualified (short-name resolution is the
/// caller's input boundary, PROP-008 §2.6).
#[derive(Debug, Clone)]
pub struct InstallRequest {
    /// Explicitly requested roots; empty = install everything the
    /// workspace's `[requires]` tables declare.
    pub roots: Vec<PackageRef>,
    /// The root-package feature request (PROP-003 §2.4).
    pub features: FeatureRequest,
    /// Language override for the PROP-003 §2.7 chain; `None` defers
    /// to the project's `[i18n]`.
    pub language: Option<String>,
    /// `--exact`: pin `=<resolved>` into the manifest instead of the
    /// caret default.
    pub exact: bool,
    /// Lockfile provenance stamp for a freshly created `vibe.lock`,
    /// e.g. `vibe 1.0.0`.
    pub generated_by: String,
    /// Whole-invocation network posture. Embedded-source hydration obeys the
    /// same hard offline boundary as package resolution.
    pub offline: bool,
}

/// The planning verdict.
#[derive(Debug)]
pub enum Plan {
    /// PROP-011 §2.2 — `vibe.lock` is already a correct resolution of
    /// every node's `[requires]`; nothing to resolve. The caller
    /// regenerates boot artifacts (cheap, self-healing) and reports.
    Fresh,
    /// A real resolution was computed and awaits the caller's
    /// confirmation before [`apply`](crate::apply).
    Ready(Box<PlannedInstall>),
}

/// A confirmed-pending resolution — everything [`apply`](crate::apply)
/// needs, carried by value so the apply phase cannot observe state
/// the plan did not produce.
#[derive(Debug)]
pub struct PlannedInstall {
    pub(crate) strict_conversion: bool,
    pub(crate) project_root: PathBuf,
    pub(crate) request: InstallRequest,
    pub(crate) manifest: Manifest,
    pub(crate) lockfile: Lockfile,
    pub(crate) language_chain: Vec<String>,
    /// The effective root set the solve ran against — the request's
    /// roots verbatim, or the workspace-derived union in
    /// install-from-manifest mode (these become the lockfile's
    /// `meta.root_dependencies` mirror in that mode).
    pub(crate) roots: Vec<PackageRef>,
    pub(crate) fetched: Vec<Fetched>,
    /// The packages to materialise, in resolution order — the shape
    /// the caller presents for confirmation.
    pub resolution: Vec<ResolvedDep>,
    /// Immutable admitted intent and selected declaration/provider evidence.
    pub normalized_request: Option<vibe_resolver::resolvo_engine::request::NormalizedRequest>,
    pub selected_provenance: Vec<vibe_resolver::resolvo_engine::request::EdgeProvenance>,
    /// The workspace this plan was computed over, carried by value.
    ///
    /// Apply and the slot lifecycle need the same tree the plan reasoned
    /// about, including a case-c migration delta that exists only in memory
    /// until apply writes it. Rediscovering there would be a second byte
    /// snapshot of a tree the command is midway through changing.
    pub(crate) workspace: Workspace,
    pub(crate) progress: vibe_core::progress::Progress,
}

impl PlannedInstall {
    /// Recheck the confirmation boundary before apply can write anything.
    pub(crate) fn validate_selected_routes(&self) -> Result<()> {
        let Some(request) = &self.normalized_request else {
            return Ok(());
        };
        let inconsistent = || {
            vibe_resolver::SolveError::UnsupportedRoute(
                "planned graph differs from normalized request or selected edge provenance".into(),
            )
        };
        let selected = |p: &PackageRef| {
            self.fetched.iter().any(|f| {
                p.group.as_ref() == Some(&f.cached.resolved.group)
                    && p.name == f.cached.resolved.name
                    && p.version.matches(&f.cached.resolved.version)
            })
        };
        if request.roots.iter().any(|p| !selected(p)) || self.resolution.len() != self.fetched.len()
        {
            return Err(inconsistent().into());
        }
        for f in &self.fetched {
            let node = &f.cached.resolved;
            let Some(row) = self.resolution.iter().find(|r| {
                r.group == node.group && r.name == node.name && r.version == node.version
            }) else {
                return Err(inconsistent().into());
            };
            let dependencies: Vec<_> = f
                .meta
                .dependencies
                .iter()
                .filter_map(|p| p.group.clone().map(|g| (g, p.name.to_string())))
                .collect();
            if row.requires != dependencies
                || f.meta.dependencies.iter().any(|dep| {
                    !selected(dep)
                        || !self.selected_provenance.iter().any(|proof| {
                            let edge = &proof.selected;
                            edge.parent.group.as_ref() == Some(&node.group)
                                && edge.parent.name == node.name
                                && edge.parent.version.matches(&node.version)
                                && edge.provider == *dep
                        })
                })
            {
                return Err(inconsistent().into());
            }
        }
        for proof in &self.selected_provenance {
            let edge = &proof.selected;
            if !selected(&edge.provider)
                || !self.fetched.iter().any(|f| {
                    edge.parent.group.as_ref() == Some(&f.cached.resolved.group)
                        && edge.parent.name == f.cached.resolved.name
                        && edge.parent.version.matches(&f.cached.resolved.version)
                        && f.meta.dependencies.contains(&edge.provider)
                })
            {
                return Err(inconsistent().into());
            }
        }
        Ok(())
    }

    /// The outer owner sets this after planning, before any apply write.
    /// A Fresh plan is checked separately against its retained slot records.
    pub fn set_strict_conversion(&mut self, strict: bool) {
        self.strict_conversion = strict;
    }
}

/// Plan an install transaction over `source` for the project at
/// `project_root` (which must already contain a `vibe.toml`).
///
/// Read-mostly: case-c migration still seeds the entry manifest from
/// `vibe.lock`'s `meta.root_dependencies`. Its disk write follows successful
/// planning so a failed catalog cannot leave install effects (M05C1-FAILURE).
pub fn plan<S: InstallSource + ?Sized>(
    source: &S,
    project_root: &Path,
    request: InstallRequest,
    observer: &dyn PlanObserver,
) -> Result<Plan> {
    plan_with_spec_format(source, project_root, request, SpecFormat::Mixed, observer)
}

/// Plan an install under the effective PROP-045 spec representation.
///
/// The COMPATIBILITY shape: it performs its one legacy read of the selected
/// manifest and its one legacy discovery, then delegates to
/// [`plan_prepared_with_spec_format`], which holds the algorithm. A caller
/// that already owns those two values calls the prepared sibling directly and
/// touches the disk for neither.
pub fn plan_with_spec_format<S: InstallSource + ?Sized>(
    source: &S,
    project_root: &Path,
    request: InstallRequest,
    spec_format: SpecFormat,
    observer: &dyn PlanObserver,
) -> Result<Plan> {
    let mut workspace = Workspace::discover(project_root)?;
    let mut manifest = Manifest::read(project_root.join(Manifest::FILENAME))?;
    plan_prepared_with_spec_format(
        source,
        project_root,
        &mut manifest,
        &mut workspace,
        request,
        spec_format,
        observer,
    )
}

/// Plan an install from values the caller ALREADY owns.
///
/// This is the real planning algorithm, and it reads neither the selected
/// `vibe.toml` nor the workspace tree: `manifest` is the raw selected manifest
/// (unexpanded, the shape `[i18n]`, visibility and the case-c migration write
/// operate on) and `workspace` is the finalised tree (expanded, the shape the
/// root union and the freshness check read). Every place the compatibility
/// wrapper used to re-`discover` now consults the supplied tree instead.
///
/// Both are taken by `&mut` because case-c migration is a real mutation: it
/// seeds the raw manifest from the lockfile, persists it, and applies the same
/// concrete requirement delta to the exact selected node of the finalised
/// tree. It is a delta and not a wholesale copy on purpose — the finalised
/// node has had `[workspace.versions]` placeholders resolved, and overwriting
/// it with the raw table would undo that.
#[allow(clippy::too_many_arguments)]
pub fn plan_prepared_with_spec_format<S: InstallSource + ?Sized>(
    source: &S,
    project_root: &Path,
    manifest: &mut Manifest,
    workspace: &mut Workspace,
    request: InstallRequest,
    spec_format: SpecFormat,
    observer: &dyn PlanObserver,
) -> Result<Plan> {
    let lockfile = load_or_empty_lockfile(&workspace.root, &request.generated_by)?;
    let configured_lock = crate::record::read_configuration(&lockfile)?;
    if configured_lock
        && source
            .normalized_request()
            .and_then(|r| r.configuration.as_ref())
            .is_none()
    {
        return Err(vibe_resolver::SolveError::UnsupportedRoute(
            "current authored configuration authority unavailable; exact replay refused".into(),
        )
        .into());
    }

    // PROP-003 §2.7 language chain (caller override > project [i18n]).
    let language_chain = build_language_chain(request.language.as_deref(), manifest);

    // 1. Decide the effective root list. Three input shapes:
    //
    //    a. Caller pkgrefs given (`vibe install flow:wal …`) — those
    //       are the roots; they are also merged into `vibe.toml`
    //       `[requires].packages` after a successful apply (Cargo /
    //       npm shape: explicit install records the dep on disk).
    //    b. No caller args, manifest has `[requires].packages` —
    //       install every declared entry. The `cargo build` / `npm
    //       install` shape: a fresh clone reproduces the project's
    //       package set without re-typing.
    //    c. No caller args, manifest is empty, but the lockfile
    //       already carries `meta.root_dependencies` — first-run
    //       migration path for projects that pre-date the
    //       `[requires]` schema (PROP-002 §2.7). Seed the manifest
    //       from the lockfile snapshot, persist it, and proceed as in
    //       case b.
    //
    //    Anything else (no caller roots, no manifest entries, no
    //    lockfile snapshot) is an error — there is nothing to install.
    let mut migration_pending = false;
    if request.roots.is_empty() {
        if manifest.requires.packages.is_empty()
            && manifest.requires.git_packages.is_empty()
            && !lockfile.meta.root_dependencies.is_empty()
        {
            observer.on(PlanEvent::MigratingRequires {
                entries: lockfile.meta.root_dependencies.len(),
            });
            manifest
                .requires
                .packages
                .clone_from(&lockfile.meta.root_dependencies);
            migration_pending = true;
            // The finalised tree below is what the root union reads, so the
            // migration has to reach it too. The wrapper used to get this by
            // re-discovering; here the SAME concrete entries are appended to
            // the exact selected node, which is the only node the write above
            // changed. Nothing else in the tree is touched, and the node's
            // already-expanded entries survive.
            migrate_selected_node(workspace, project_root, &lockfile.meta.root_dependencies);
        }
    }
    let (roots, preferred_roots, root_origins) =
        effective_root_constraints(workspace, project_root, &request.roots, &lockfile)?;

    // ##EMPTY-REQUIRES-IS-A-NO-OP (PROP-011, the 2026-08-24 ruling): zero
    // dependencies is a fresh project, not an error — regenerate the boot
    // artifacts of the empty world and report the fresh shape.
    // Admitted transaction intent is not represented by the legacy lockfile.
    // Neither an empty manifest nor its freshness can discharge that request.
    let admitted_request = source.normalized_request().is_some();
    if roots.is_empty() && !admitted_request {
        return Ok(Plan::Fresh);
    }

    let progress = observer.progress();
    let resolution_task = progress.task("Resolving dependencies");
    resolution_task.detail(format!(
        "{} declared root package{}",
        roots.len(),
        if roots.len() == 1 { "" } else { "s" }
    ));

    // The root set the depsolver actually runs against. For an
    // explicit-pkgref install it is the named refs plus the pin-held
    // remainder above; for a stale bare install PROP-011 §5.3 replaces it
    // with the pin-held set below.
    let mut solve_roots = preferred_roots;

    // PROP-011 §2.2 — the freshness fast path. When no explicit pkgref
    // was given (the install-from-manifest shape) and `vibe.lock` is
    // already a correct resolution of every node's `[requires]`, the
    // depsolver — a registry walk over the network — is skipped
    // entirely: the resolution is the lock, and application is just a
    // whole-tree boot regeneration (cheap, self-healing — PROP-011
    // §2.4). This is also what makes `vibe install`
    // lockfile-respecting: a fresh lock is honoured verbatim, with no
    // version drift inside a constraint. An explicit
    // `vibe install <pkgref>` always runs the full pipeline.
    if request.roots.is_empty() && !admitted_request {
        let ws = &*workspace;
        match vibe_workspace::freshness::check(ws, &lockfile) {
            vibe_workspace::freshness::Freshness::Fresh
                if slots_match_spec_format(&ws.root, &lockfile, spec_format) =>
            {
                if migration_pending {
                    manifest.write(project_root.join(Manifest::FILENAME))?;
                }
                resolution_task.skip("vibe.lock is fresh");
                return Ok(Plan::Fresh);
            }
            vibe_workspace::freshness::Freshness::Fresh => {
                observer.on(PlanEvent::Reresolving {
                    reason: format!(
                        "materialised spec representation is not {}",
                        spec_format.as_str()
                    ),
                });
                solve_roots = vibe_workspace::freshness::hold_pins(&roots, &lockfile);
            }
            vibe_workspace::freshness::Freshness::Stale(reason) => {
                observer.on(PlanEvent::Reresolving {
                    reason: reason.clone(),
                });
                // PROP-011 §5.3 — minimum churn: hold the locked
                // version of every root the change did not touch, so
                // re-resolution moves only the changed dependency and
                // its subtree.
                solve_roots = vibe_workspace::freshness::hold_pins(&roots, &lockfile);
            }
        }
    }

    let mut normalized = vibe_resolver::resolvo_engine::request::NormalizedRequest::legacy(
        &roots,
        &request.features,
    );
    normalized.root_origins = root_origins;
    let transaction = transaction::PlanningSource::capture_request(source, &normalized)?;
    let source = &transaction;
    crate::record::validate_replay(&lockfile, source, &request.roots)?;
    if let Some(catalog) = source.catalog_inspection() {
        observer.catalog(catalog);
    }

    // 2. Run the depsolver.
    observer.on(PlanEvent::ResolvingRoots { roots: roots.len() });
    let mut strict_roots = solve_roots.clone();
    let graph = match source.solve(&solve_roots) {
        Ok(graph) => graph,
        Err(e)
            if solve_roots != roots
                && (!matches!(e, vibe_resolver::SolveError::Provider(_))
                    || matches!(
                        e,
                        vibe_resolver::SolveError::Provider(
                            vibe_resolver::DepProviderError::NoMatchingVersion { .. }
                        )
                    )) =>
        {
            // PROP-011 §5.3 — the pin-held set over-constrained: a
            // changed dependency is incompatible with a held pin. Fall
            // back to a full, free re-resolve.
            observer.on(PlanEvent::HeldPinsConflicted {
                error: e.to_string(),
            });
            strict_roots.clone_from(&roots);
            match source.solve(&roots) {
                Ok(graph) => graph,
                Err(error) => {
                    resolution_task.fail("dependency resolution failed");
                    return Err(error.into());
                }
            }
        }
        Err(e) => {
            resolution_task.fail("dependency resolution failed");
            return Err(e.into());
        }
    };

    let root_id = visibility_root_id(manifest);
    let effective = resolve_effective(source, &strict_roots, manifest, &root_id, graph)?;
    let graph = effective.graph;
    let mut visibility_analysis = effective.analysis;
    let _visibility_iterations = effective.iterations;

    if graph.packages.len() > roots.len() {
        observer.on(PlanEvent::GraphSolved {
            roots: roots.len(),
            total: graph.packages.len(),
        });
    }
    resolution_task.detail(format!(
        "selected {} package{} before conditional expansion",
        graph.packages.len(),
        if graph.packages.len() == 1 { "" } else { "s" }
    ));
    resolution_task.finish();

    // 3. Phase one — fetch every node, pin features per node. We need
    //    the full graph + every fetched manifest before we can build
    //    the activation context, since context probes (`if_present`,
    //    `if_provides`, `if_describes_match`) depend on the union of
    //    capabilities, interfaces, and PURLs across the graph.
    //
    //    Fetched payload lands in the machine-global store
    //    (`~/.vibe/cache/`, PROP-010 §2.7) — resolved here, once per
    //    plan, through the settings chokepoint. The project no longer
    //    has a `.vibe/cache/` copy: the store IS the source
    //    `vibedeps/` materialises from.
    // Validate every selected binding before the first acquisition effect.
    for node in graph.iter() {
        source.validate_catalog_candidate(&exact_pinned_pkgref(node))?;
    }
    let store_root = store::store_root()?;
    let mut fetched: Vec<Fetched> = Vec::with_capacity(graph.packages.len());
    let fetch_task = progress.task("Fetching package content");
    for node in graph.iter() {
        fetch_task.detail(format!("{}/{}@{}", node.group, node.name, node.version));
        fetched.push(fetch_or_defer(
            source,
            node,
            &lockfile,
            &store_root,
            &request.features,
            &workspace.root,
            request.offline,
            &fetch_task.progress(),
        )?);
        fetch_task.set_progress(fetched.len() as u64, None, "packages");
    }

    // The read-only kind stops here, before anything is materialised.
    // A package's kind lives inside its own bytes, so this is the first
    // point in the install where it is known at all — and the last one
    // before `apply` writes a slot (PROP-057
    // `##KIND-DOC-NOT-INSTALLED`).
    refuse_read_only_kinds(&fetched)?;

    // Visibility check: warn if a requested feature was accepted by no
    // root package.
    if !request.features.explicit.is_empty() {
        let accepted: BTreeSet<&str> = fetched
            .iter()
            .filter(|f| f.meta.is_root)
            .flat_map(|f| f.feature_expansion.active_features.iter())
            .map(|s| s.as_str())
            .collect();
        let unmatched: Vec<String> = request
            .features
            .explicit
            .iter()
            .filter(|f| !accepted.contains(f.as_str()))
            .cloned()
            .collect();
        if !unmatched.is_empty() {
            observer.on(PlanEvent::FeaturesUnmatched {
                features: unmatched,
            });
        }
    }

    // 4. Conditional dependency expansion — fixed-point loop.
    expand_conditional_deps(
        source,
        &strict_roots,
        &lockfile,
        &store_root,
        project_root,
        &workspace.root,
        &language_chain,
        &request.features,
        request.offline,
        manifest,
        &root_id,
        &mut fetched,
        &mut visibility_analysis,
        observer,
        &fetch_task,
    )?;
    fetch_task.set_progress(fetched.len() as u64, Some(fetched.len() as u64), "packages");
    fetch_task.finish();

    // 5. Build the resolution — every fetched package as a
    //    `ResolvedDep` the workspace orchestrator materialises. The
    //    loading model materialises a package's tree verbatim, so the
    //    per-file activation context is no longer consulted at install
    //    time.
    let resolution: Vec<ResolvedDep> = fetched
        .iter()
        .map(|f| {
            let coordinate = format!("{}/{}", f.cached.resolved.group, f.cached.resolved.name);
            let provenance = visibility_analysis.effective.get(&coordinate);
            ResolvedDep {
                kind: f.cached.package_meta().kind,
                group: f.cached.resolved.group.clone(),
                name: f.cached.resolved.name.clone(),
                version: f.cached.resolved.version.clone(),
                content_dir: f.cached.cache_dir.clone(),
                source_hash: Some(ContentHash::from_validated(f.cached.content_hash.clone())),
                embedded_sources: f.embedded_sources.clone(),
                manifest: f.cached.manifest.clone(),
                // A `[requires.packages]` dependency pkgref is
                // group-qualified (PROP-008 §2.6).
                requires: f
                    .meta
                    .dependencies
                    .iter()
                    .filter_map(|p| p.group.clone().map(|g| (g, p.name.to_string())))
                    .collect(),
                admitted_by: provenance.map(|witness| match witness.rule {
                    ProvenanceRule::RootEdge => "root-edge".to_string(),
                    ProvenanceRule::PublicChain => "public-chain".to_string(),
                    ProvenanceRule::FriendsChain => "friends-chain".to_string(),
                }),
                via_override: provenance.and_then(|witness| witness.via_override.clone()),
                // Mutable iff an in-workspace `file://` self-hosting source (§2.6).
                source_mutable: is_in_workspace_file_source(&f.cached.source_uri, &workspace.root),
                in_place_changed: None,
            }
        })
        .collect();

    let current_record = crate::record::configuration_record(
        source.normalized_request(),
        &source
            .route_resolution()
            .map(|r| r.provenance)
            .unwrap_or_default(),
        &fetched,
    )?;
    crate::record::validate_replayed_result(&lockfile, current_record.as_ref(), &request.roots)?;
    if migration_pending {
        manifest.write(project_root.join(Manifest::FILENAME))?;
    }
    Ok(Plan::Ready(Box::new(PlannedInstall {
        strict_conversion: false,
        project_root: project_root.to_path_buf(),
        request,
        manifest: manifest.clone(),
        lockfile,
        language_chain,
        roots,
        fetched,
        resolution,
        normalized_request: source.normalized_request().cloned(),
        selected_provenance: source
            .route_resolution()
            .map(|r| r.provenance)
            .unwrap_or_default(),
        // A value, not a rediscovery: apply and the slot lifecycle work on the
        // exact tree that produced this plan.
        workspace: workspace.clone(),
        progress,
    })))
}

/// The acquisition half of planning — fetch/defer helpers and the
/// conditional-dependency expansion, split along the seam when the
/// migration pushed the file past the 600-line budget.
mod fetch;
use fetch::{expand_conditional_deps, fetch_or_defer};

mod selection;
use selection::{
    migrate_selected_node, refuse_read_only_kinds, slots_match_spec_format, visibility_root_id,
};

mod transaction;

#[cfg(test)]
mod m_05_c_1_cases;

#[cfg(test)]
mod m_05_c_3_cases;

/// Preserve each workspace declaration. Explicit roots supersede only the
/// exact selected node's named declaration, never another member's constraint.
fn effective_root_constraints(
    workspace: &Workspace,
    project_root: &Path,
    explicit: &[PackageRef],
    lockfile: &Lockfile,
) -> Result<(
    Vec<PackageRef>,
    Vec<PackageRef>,
    Vec<vibe_resolver::resolvo_engine::request::RootConstraint>,
)> {
    use vibe_resolver::resolvo_engine::request::RootConstraint;
    let mut roots = explicit.to_vec();
    let mut preferred = roots.clone();
    let mut origins: Vec<_> = explicit
        .iter()
        .enumerate()
        .map(|(index, p)| RootConstraint {
            constraint: p.clone(),
            consumer: project_root.display().to_string(),
            declaration: format!("cli.roots[{index}]"),
        })
        .collect();
    for (relative, node) in workspace.iter_nodes() {
        let selected = workspace.node_abs_path(relative) == project_root;
        let consumer = workspace.node_abs_path(relative).display().to_string();
        let mut add = |reference: PackageRef, declaration: String| {
            let named = explicit
                .iter()
                .any(|p| p.group == reference.group && p.name == reference.name);
            if named && selected {
                return;
            }
            if !explicit.is_empty() && !named {
                preferred.extend(vibe_workspace::freshness::hold_pins(
                    std::slice::from_ref(&reference),
                    lockfile,
                ));
            } else {
                preferred.push(reference.clone());
            }
            origins.push(RootConstraint {
                constraint: reference.clone(),
                consumer: consumer.clone(),
                declaration,
            });
            roots.push(reference);
        };
        for (index, p) in node.requires.packages.iter().enumerate() {
            add(p.clone(), format!("requires.packages[{index}]"));
        }
        for (index, g) in node.requires.git_packages.iter().enumerate() {
            add(
                PackageRef::new(
                    g.kind,
                    Some(g.group.clone()),
                    g.name.clone(),
                    VersionSpec::Latest,
                )?,
                format!("requires.git_packages[{index}]"),
            );
        }
    }
    Ok((roots, preferred, origins))
}
