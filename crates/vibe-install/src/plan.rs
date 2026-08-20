//! The planning phase — derive the effective root set, run the depsolver,
//! fetch and feature-pin every node, expand conditional dependencies to a
//! fixed point, and shape the resolution the caller confirms before
//! [`apply`](crate::apply) mutates anything beyond the recorded migration writes.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::{Group, PackageRef, VersionSpec};
use vibe_registry::store;
use vibe_registry::{CachedPackage, ResolvedPackage};
use vibe_resolver::{
    FeatureRequest, ResolvedNode, conditional::ConditionalPredicate, expand_features,
};
use vibe_workspace::Workspace;
use vibe_workspace::freshness::is_in_workspace_file_source;
use vibe_workspace::install::ResolvedDep;
use vibe_workspace::vibedeps;

use crate::error::{Error, Result};
use crate::events::{PlanEvent, PlanObserver};
use crate::fetched::{
    Fetched, NodeInstallMeta, build_activation_context, build_language_chain,
    load_or_empty_lockfile, tailor_feature_request,
};
use crate::record::exact_pinned_pkgref;
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
    /// e.g. `vibe 0.1.0-dev`.
    pub generated_by: String,
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
}

/// Plan an install transaction over `source` for the project at
/// `project_root` (which must already contain a `vibe.toml`).
///
/// Read-mostly: the single deliberate write is the case-c migration
/// (an empty entry manifest seeded from `vibe.lock`'s
/// `meta.root_dependencies` and persisted before solving, so a panic
/// mid-solve cannot lose it — PROP-002 §2.7).
pub fn plan<S: InstallSource + ?Sized>(
    source: &S,
    project_root: &Path,
    request: InstallRequest,
    observer: &dyn PlanObserver,
) -> Result<Plan> {
    let workspace = Workspace::discover(project_root)?;
    let mut manifest = Manifest::read(project_root.join(Manifest::FILENAME))?;
    let lockfile = load_or_empty_lockfile(&workspace.root, &request.generated_by)?;

    // PROP-003 §2.7 language chain (caller override > project [i18n]).
    let language_chain = build_language_chain(request.language.as_deref(), &manifest);

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
    let roots: Vec<PackageRef> = if request.roots.is_empty() {
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
            manifest.write(project_root.join(Manifest::FILENAME))?;
        }
        // Unified resolution (PROP-009 §2.7): the root set is the union
        // of every workspace node's `[requires]`. Re-discover so the
        // migration above, an earlier `--git` declaration, and any
        // `[workspace.versions]` placeholders are all reflected; a
        // standalone project is a one-node workspace, so this
        // degenerates to "just the entry node". The source dispatches
        // each pkgref through the right path internally
        // (override > git > registry).
        let discovered = Workspace::discover(project_root)?;
        let mut all: Vec<PackageRef> = Vec::new();
        // De-duplicate on the `(group, name)` identity (PROP-008 §2.3).
        // A manifest pkgref is group-qualified, so `group` is present.
        let mut seen: std::collections::HashSet<(Option<Group>, String)> =
            std::collections::HashSet::new();
        for (_, node) in discovered.iter_nodes() {
            for p in &node.requires.packages {
                if seen.insert((p.group.clone(), p.name.to_string())) {
                    all.push(p.clone());
                }
            }
            for g in &node.requires.git_packages {
                if seen.insert((Some(g.group.clone()), g.name.clone())) {
                    all.push(PackageRef::new(
                        g.kind,
                        Some(g.group.clone()),
                        g.name.clone(),
                        VersionSpec::Latest,
                    )?);
                }
            }
        }
        if all.is_empty() {
            return Err(Error::NothingToInstall {
                manifest_dir: project_root.display().to_string(),
            });
        }
        all
    } else {
        request.roots.clone()
    };

    // The root set the depsolver actually runs against. For an
    // explicit-pkgref install it is `roots` verbatim; for a stale bare
    // install PROP-011 §5.3 replaces it with the pin-held set below.
    let mut solve_roots = roots.clone();

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
    if request.roots.is_empty() {
        let ws = Workspace::discover(project_root)?;
        match vibe_workspace::freshness::check(&ws, &lockfile) {
            vibe_workspace::freshness::Freshness::Fresh => {
                return Ok(Plan::Fresh);
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

    // 2. Run the depsolver.
    observer.on(PlanEvent::ResolvingRoots { roots: roots.len() });
    let graph = match source.solve(&solve_roots) {
        Ok(graph) => graph,
        Err(e) if solve_roots != roots => {
            // PROP-011 §5.3 — the pin-held set over-constrained: a
            // changed dependency is incompatible with a held pin. Fall
            // back to a full, free re-resolve.
            observer.on(PlanEvent::HeldPinsConflicted {
                error: e.to_string(),
            });
            source.solve(&roots)?
        }
        Err(e) => return Err(e.into()),
    };

    if graph.packages.len() > roots.len() {
        observer.on(PlanEvent::GraphSolved {
            roots: roots.len(),
            total: graph.packages.len(),
        });
    }

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
    let store_root = store::store_root()?;
    let mut fetched: Vec<Fetched> = Vec::with_capacity(graph.packages.len());
    for node in graph.iter() {
        fetched.push(fetch_or_defer(
            source,
            node,
            &lockfile,
            &store_root,
            &request.features,
            &workspace.root,
        )?);
    }

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
        &roots,
        &lockfile,
        &store_root,
        project_root,
        &workspace.root,
        &language_chain,
        &request.features,
        &mut fetched,
        observer,
    )?;

    // 5. Build the resolution — every fetched package as a
    //    `ResolvedDep` the workspace orchestrator materialises. The
    //    loading model materialises a package's tree verbatim, so the
    //    per-file activation context is no longer consulted at install
    //    time.
    let resolution: Vec<ResolvedDep> = fetched
        .iter()
        .map(|f| ResolvedDep {
            kind: f.cached.package_meta().kind,
            group: f.cached.resolved.group.clone(),
            name: f.cached.resolved.name.clone(),
            version: f.cached.resolved.version.clone(),
            content_dir: f.cached.cache_dir.clone(),
            manifest: f.cached.manifest.clone(),
            // A `[requires.packages]` dependency pkgref is
            // group-qualified (PROP-008 §2.6).
            requires: f
                .meta
                .dependencies
                .iter()
                .filter_map(|p| p.group.clone().map(|g| (g, p.name.to_string())))
                .collect(),
            // Mutable iff an in-workspace `file://` self-hosting source (§2.6).
            source_mutable: is_in_workspace_file_source(&f.cached.source_uri, &workspace.root),
        })
        .collect();

    Ok(Plan::Ready(Box::new(PlannedInstall {
        project_root: project_root.to_path_buf(),
        request,
        manifest,
        lockfile,
        language_chain,
        roots,
        fetched,
        resolution,
    })))
}

/// The acquisition half of planning — fetch/defer helpers and the
/// conditional-dependency expansion, split along the seam when the
/// migration pushed the file past the 600-line budget.
mod fetch;
use fetch::{expand_conditional_deps, fetch_or_defer};
