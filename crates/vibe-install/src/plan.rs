//! The planning phase — derive the effective root set, run the
//! depsolver, fetch and feature-pin every node, expand conditional
//! dependencies to a fixed point, and shape the resolution the caller
//! confirms before [`apply`](crate::apply) mutates anything beyond
//! the recorded migration writes.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::{Group, PackageRef, VersionSpec};
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

    // Cache layout matches §8.3: `.vibe/cache/<kind>/<name>/<version>/`.
    // The cache lives at the absolute workspace root — one shared cache.
    let cache_root = workspace.root.join(".vibe/cache");
    fs::create_dir_all(&cache_root).map_err(|source| Error::CacheDir {
        path: cache_root.display().to_string(),
        source,
    })?;

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
    let mut fetched: Vec<Fetched> = Vec::with_capacity(graph.packages.len());
    for node in graph.iter() {
        fetched.push(fetch_or_defer(
            source,
            node,
            &lockfile,
            &cache_root,
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
        &cache_root,
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

/// Resolve-and-fetch one solved node, expanding its features. Roots
/// get the caller's feature request tailored to what the package
/// declares; transitives get the default set.
fn fetch_node<S: InstallSource + ?Sized>(
    source: &S,
    node: &ResolvedNode,
    lockfile: &Lockfile,
    cache_root: &Path,
    root_features: &FeatureRequest,
) -> Result<Fetched> {
    let pkgref = exact_pinned_pkgref(node);
    let expected = lockfile
        .find(&node.group, &node.name)
        .map(|p| p.content_hash.clone());
    let cached = source.resolve_and_fetch(&pkgref, cache_root, expected.as_deref())?;
    let req = if node.is_root {
        tailor_feature_request(root_features, &cached.manifest.features)
    } else {
        FeatureRequest::default()
    };
    let feature_expansion = expand_features(&cached.manifest.features, &req)?;
    Ok(Fetched {
        cached,
        feature_expansion,
        meta: NodeInstallMeta {
            dependencies: node.dependencies.clone(),
            is_root: node.is_root,
        },
        in_place_incremental: false,
    })
}

/// Fetch one solved node, OR — when it re-resolves an already-present
/// `in-place` package (PROP-022 §2.4) — defer it: build its [`Fetched`] from
/// the existing slot with NO network re-clone, leaving the incremental
/// `git fetch` to [`apply`](crate::apply). This is what keeps a giant in-place
/// repo (Chromium-scale) from being re-cloned on every full-pipeline install;
/// only a *fresh* in-place package (no slot yet) clones, and every
/// snapshot/hardlink package fetches exactly as before.
fn fetch_or_defer<S: InstallSource + ?Sized>(
    source: &S,
    node: &ResolvedNode,
    lockfile: &Lockfile,
    cache_root: &Path,
    root_features: &FeatureRequest,
    workspace_root: &Path,
) -> Result<Fetched> {
    match try_in_place_incremental(node, lockfile, workspace_root, root_features)? {
        Some(fetched) => Ok(fetched),
        None => fetch_node(source, node, lockfile, cache_root, root_features),
    }
}

/// If `node` re-resolves a package the lockfile already records as `in-place`
/// (PROP-022 §2.4) whose project slot is present, build its [`Fetched`] from
/// that slot WITHOUT a network re-clone — the deferred incremental update runs
/// against the live `.git` in [`apply`](crate::apply), post-confirmation, so a
/// declined plan never advances the slot's commit (the plan stays
/// read-mostly). Returns `None` for anything else — a fresh in-place install
/// (no slot yet; restored by a re-clone per §2.7), or any snapshot/hardlink
/// package — so the caller fetches it normally.
///
/// The provisional `cached.cache_dir` IS the slot — the "already placed"
/// signal `materialise_resolution` reads to run the hook and skip the move
/// (§2.4). Reading the slot's manifest is local and network-free; its
/// provenance is carried from the lockfile and overwritten by
/// [`apply`](crate::apply) with the freshly-fetched values once the
/// incremental `git fetch` has run.
#[spec(
    implements = "spec://vibevm/modules/vibe-workspace/PROP-022#in-place",
    r = 1
)]
fn try_in_place_incremental(
    node: &ResolvedNode,
    lockfile: &Lockfile,
    workspace_root: &Path,
    root_features: &FeatureRequest,
) -> Result<Option<Fetched>> {
    let Some(old) = lockfile.find(&node.group, &node.name) else {
        return Ok(None);
    };
    if !old.materialization.is_in_place() {
        return Ok(None);
    }
    let kind = old.kind;
    // A `.gitignore`d in-place slot that was deleted is restored by a re-clone
    // (§2.7), not an incremental fetch — fall back to the normal path.
    if !vibedeps::is_in_place_slot(workspace_root, kind, &node.name) {
        return Ok(None);
    }
    let slot = vibedeps::in_place_slot_abs_path(workspace_root, kind, &node.name);
    // Read the live slot's manifest locally (no network) for the resolution,
    // conditional-dep, and feature passes. A slot with no readable `[package]`
    // table is not a trustworthy incremental base — re-clone it instead.
    let manifest = match Manifest::read(slot.join(Manifest::FILENAME)) {
        Ok(m) if m.package.is_some() => m,
        _ => return Ok(None),
    };
    let req = if node.is_root {
        tailor_feature_request(root_features, &manifest.features)
    } else {
        FeatureRequest::default()
    };
    let feature_expansion = expand_features(&manifest.features, &req)?;
    let cached = CachedPackage {
        resolved: ResolvedPackage {
            group: node.group.clone(),
            name: node.name.clone(),
            version: node.version.clone(),
            source_dir: slot.clone(),
        },
        cache_dir: slot,
        manifest,
        // Carried from the lockfile so the provisional describes the *current*
        // slot; apply overwrites all four once the incremental fetch lands the
        // resolved commit (PROP-022 §2.5).
        content_hash: old.content_hash.as_str().to_string(),
        source_uri: old.source_url.as_str().to_string(),
        registry_name: old.registry.clone(),
        source_ref: old.source_ref.clone(),
        resolved_commit: old.resolved_commit.clone(),
        overridden: false,
        is_git_source: false,
        is_path_source: false,
        is_embedded: false,
        is_local: false,
        via_redirect: None,
    };
    Ok(Some(Fetched {
        cached,
        feature_expansion,
        meta: NodeInstallMeta {
            dependencies: node.dependencies.clone(),
            is_root: node.is_root,
        },
        in_place_incremental: true,
    }))
}

/// The PROP-003 §2.6.1 conditional-dependency loop. Each pass: build
/// the activation context from currently-fetched packages; walk every
/// package's `[target."context(...)".dependencies]`; if any predicate
/// matches and its targets aren't already in the graph, add them as
/// extra roots; re-solve and fetch. Repeat until no new extras emerge,
/// or until the iteration cap is hit.
///
/// Convergence: extras only ADD packages to the fetched set
/// (monotonic), and the predicate evaluation is a pure function of
/// `present` + `provides`, which only grow. So either a pass produces
/// no extras (terminates), or every pass adds at least one package —
/// bounded by the registry's size.
///
/// The cap (5 iterations) catches authoring-bug cases where a chain of
/// conditional deps re-triggers on each iteration without converging.
/// The conservative cap surfaces as a loud error so the operator can
/// either fix the chain or bump the limit explicitly. No realistic
/// graph reaches the cap.
#[expect(
    clippy::too_many_arguments,
    reason = "the fixpoint reads the whole planning context; bundling \
              the borrows into a struct would only rename the arity"
)]
#[spec(implements = "spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-fixpoint")]
fn expand_conditional_deps<S: InstallSource + ?Sized>(
    source: &S,
    roots: &[PackageRef],
    lockfile: &Lockfile,
    cache_root: &Path,
    project_root: &Path,
    workspace_root: &Path,
    language_chain: &[String],
    root_features: &FeatureRequest,
    fetched: &mut Vec<Fetched>,
    observer: &dyn PlanObserver,
) -> Result<()> {
    const COND_DEP_MAX_ITER: usize = 5;
    let mut iteration: usize = 0;
    loop {
        iteration += 1;
        let preliminary_ctx = build_activation_context(
            fetched.iter().map(|f| &f.cached),
            project_root,
            language_chain,
        )?;
        let mut extra: Vec<PackageRef> = Vec::new();
        for f in fetched.iter() {
            for (pred_str, target) in &f.cached.manifest.conditional_deps {
                match ConditionalPredicate::parse(pred_str) {
                    Ok(pred) => {
                        if pred.evaluate(&preliminary_ctx) {
                            for r in &target.dependencies.packages {
                                let already = fetched.iter().any(|g| {
                                    Some(&g.cached.resolved.group) == r.group.as_ref()
                                        && g.cached.resolved.name == r.name
                                }) || extra
                                    .iter()
                                    .any(|x| x.group == r.group && x.name == r.name);
                                if !already {
                                    extra.push(r.clone());
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            target: "vibe_install",
                            package = %format!("{}/{}", f.cached.resolved.group, f.cached.resolved.name),
                            predicate = %pred_str,
                            error = %e,
                            "conditional-dep predicate could not be parsed; skipping"
                        );
                    }
                }
            }
        }
        if extra.is_empty() {
            return Ok(());
        }
        if iteration > COND_DEP_MAX_ITER {
            return Err(Error::ConditionalDepRunaway {
                iterations: COND_DEP_MAX_ITER,
                pending: extra.iter().map(|r| r.qualified_name()).collect(),
            });
        }
        observer.on(events::PlanEvent::ConditionalIteration {
            iteration,
            extras: extra.len(),
        });
        let mut combined = roots.to_vec();
        combined.extend(fetched.iter().filter(|f| f.meta.is_root).map(|f| {
            exact_pinned_pkgref(&ResolvedNode {
                group: f.cached.resolved.group.clone(),
                name: f.cached.resolved.name.clone(),
                version: f.cached.resolved.version.clone(),
                dependencies: Vec::new(),
                is_root: true,
            })
        }));
        combined.extend(extra.iter().cloned());
        // De-duplicate by the `(group, name)` identity (PROP-008 §2.3).
        let mut seen: std::collections::HashSet<(Option<Group>, String)> =
            std::collections::HashSet::new();
        combined.retain(|r| seen.insert((r.group.clone(), r.name.to_string())));
        let new_graph = source.solve(&combined)?;
        for node in new_graph.iter() {
            if fetched.iter().any(|g| {
                g.cached.resolved.group == node.group && g.cached.resolved.name == node.name
            }) {
                continue;
            }
            fetched.push(fetch_or_defer(
                source,
                node,
                lockfile,
                cache_root,
                root_features,
                workspace_root,
            )?);
        }
    }
}
