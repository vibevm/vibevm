//! The acquisition half of planning (see [`super`]): fetching a node,
//! deferring one, the in-place incremental fast path, and conditional-deps
//! expansion — split from `plan.rs` along its responsibility seam.

use specmark::spec;

use super::*;
use vibe_workspace::vibedeps;

/// Resolve-and-fetch one solved node, expanding its features. Roots
/// get the caller's feature request tailored to what the package
/// declares; transitives get the default set.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
pub(super) fn fetch_node<S: InstallSource + ?Sized>(
    source: &S,
    node: &ResolvedNode,
    lockfile: &Lockfile,
    store_root: &Path,
    root_features: &FeatureRequest,
    offline: bool,
    progress: &vibe_core::progress::Progress,
) -> Result<Fetched> {
    let pkgref = exact_pinned_pkgref(node);
    let expected = lockfile
        .find(&node.group, &node.name)
        .map(|p| p.content_hash.clone());
    let cached = source.resolve_and_fetch_with_progress(
        &pkgref,
        store_root,
        expected.as_deref(),
        progress,
    )?;
    let configuration = source.configuration_for_node(node)?;
    crate::configuration_admission::admitted_package(
        pkgref.qualified_name(),
        &cached.manifest,
        configuration.as_ref(),
    )?;
    let req = if node.is_root {
        tailor_feature_request(root_features, &cached.manifest.features)
    } else {
        FeatureRequest::default()
    };
    let feature_expansion = match &configuration {
        Some(record) => vibe_resolver::FeatureExpansion {
            active_features: record.features.iter().cloned().collect(),
            ..Default::default()
        },
        None => expand_features(&cached.manifest.features, &req)?,
    };
    let embedded_sources =
        vibe_registry::lock_embedded_sources_with(&cached.manifest.embedded_sources, offline)?;
    Ok(Fetched {
        cached,
        feature_expansion,
        meta: NodeInstallMeta {
            dependencies: node.dependencies.clone(),
            is_root: node.is_root,
        },
        embedded_sources,
        in_place: None,
        configuration,
    })
}

/// Fetch a solved node, or retain an exact prospective in-place preparation.
/// Preparation can acquire source/cache objects, but changes no project slot.
/// Every selected manifest is admitted before the plan can be confirmed.
// Keep the existing planner inputs explicit; progress adds observation only.
#[allow(clippy::too_many_arguments)]
pub(super) fn fetch_or_defer<S: InstallSource + ?Sized>(
    source: &S,
    node: &ResolvedNode,
    lockfile: &Lockfile,
    store_root: &Path,
    root_features: &FeatureRequest,
    workspace_root: &Path,
    offline: bool,
    progress: &vibe_core::progress::Progress,
) -> Result<Fetched> {
    match try_in_place_incremental(
        source,
        node,
        lockfile,
        workspace_root,
        root_features,
        offline,
    )? {
        Some(fetched) => Ok(fetched),
        None => fetch_node(
            source,
            node,
            lockfile,
            store_root,
            root_features,
            offline,
            progress,
        ),
    }
}

/// An existing in-place slot may be refreshed only through a source-bound
/// preparation. The old slot establishes a usable base, not the target
/// declaration; features and conditional metadata use the admitted candidate.
/// Missing/unreadable bases retain the ordinary fresh-fetch fallback.
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#in-place",
    r = 1
)]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
pub(super) fn try_in_place_incremental<S: InstallSource + ?Sized>(
    source: &S,
    node: &ResolvedNode,
    lockfile: &Lockfile,
    workspace_root: &Path,
    root_features: &FeatureRequest,
    offline: bool,
) -> Result<Option<Fetched>> {
    let Some(old) = lockfile.find(&node.group, &node.name) else {
        return Ok(None);
    };
    if !old.materialization.is_in_place() {
        return Ok(None);
    }
    let _ = old.kind;
    // A `.gitignore`d in-place slot that was deleted is restored by a re-clone
    // (§2.7), not an incremental fetch — fall back to the normal path.
    if !vibedeps::is_in_place_slot(workspace_root, &old.group, &node.name) {
        return Ok(None);
    }
    let slot = vibedeps::in_place_slot_abs_path(workspace_root, &old.group, &node.name);
    // Read the live slot's manifest locally (no network) for the resolution,
    // conditional-dep, and feature passes. A slot with no readable `[package]`
    // table is not a trustworthy incremental base — re-clone it instead.
    match Manifest::read(slot.join(Manifest::FILENAME)) {
        Ok(m) if m.package.is_some() => {}
        _ => return Ok(None),
    }
    let pkgref = exact_pinned_pkgref(node);
    let prepared = source.prepare_in_place(&pkgref, &slot)?;
    if prepared.slot() != slot {
        return Err(vibe_registry::RegistryError::MalformedMeta {
            path: slot,
            reason: "prepared in-place source belongs to another destination".into(),
        }
        .into());
    }

    crate::configuration_admission::package_manifest(&pkgref, prepared.manifest())?;
    let manifest = prepared.manifest().clone();
    let embedded_sources =
        vibe_registry::lock_embedded_sources_with(&manifest.embedded_sources, offline)?;

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
        // Prospective identity is retained with its preparation; this path is
        // not consumed as materialized content until the bound apply succeeds.
        content_hash: prepared.content_hash().to_owned(),
        source_uri: prepared.source_uri().to_owned(),
        fetched_from: None,
        registry_name: old.registry.clone(),
        source_ref: Some(prepared.source_ref().to_owned()),
        resolved_commit: Some(prepared.resolved_commit().to_owned()),
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
        embedded_sources,
        in_place: Some(prepared),
        configuration: None,
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
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#req-conditional-fixpoint"
)]
pub(super) fn expand_conditional_deps<S: InstallSource + ?Sized>(
    source: &S,
    roots: &[PackageRef],
    held_roots: &[PackageRef],
    lockfile: &Lockfile,
    store_root: &Path,
    project_root: &Path,
    workspace_root: &Path,
    language_chain: &[String],
    root_features: &FeatureRequest,
    offline: bool,
    root_manifest: &Manifest,
    root_id: &str,
    fetched: &mut Vec<Fetched>,
    visibility_analysis: &mut Analysis,
    observer: &dyn PlanObserver,
    fetch_task: &vibe_core::progress::ProgressTask,
) -> Result<()> {
    const COND_DEP_MAX_ITER: usize = 5;
    let mut iteration: usize = 0;
    let mut solve_roots = held_roots.to_vec();
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
        fetch_task.detail(format!(
            "conditional pass {iteration} discovered {} package{}",
            extra.len(),
            if extra.len() == 1 { "" } else { "s" }
        ));
        // Carry previous conditional roots only. Pins for a base coordinate
        // would override a consumer's allowed range and prevent a valid solve.
        let carried: Vec<_> = fetched
            .iter()
            .filter(|f| {
                f.meta.is_root
                    && !roots.iter().any(|r| {
                        r.group.as_ref() == Some(&f.cached.resolved.group)
                            && r.name == f.cached.resolved.name
                    })
            })
            .map(|f| {
                exact_pinned_pkgref(&ResolvedNode {
                    group: f.cached.resolved.group.clone(),
                    name: f.cached.resolved.name.clone(),
                    version: f.cached.resolved.version.clone(),
                    dependencies: Vec::new(),
                    is_root: true,
                })
            })
            .collect();
        let mut combined = solve_roots.clone();
        combined.extend(carried.iter().cloned());
        combined.extend(extra.iter().cloned());
        let new_graph = match source.solve(&combined) {
            Ok(graph) => graph,
            Err(error) if solve_roots != roots => {
                observer.on(events::PlanEvent::HeldPinsConflicted {
                    error: error.to_string(),
                });
                solve_roots = roots.to_vec();
                combined = solve_roots.clone();
                combined.extend(carried);
                combined.extend(extra.iter().cloned());
                source.solve(&combined)?
            }
            Err(error) => return Err(error.into()),
        };
        let effective = crate::visibility_projection::resolve_effective(
            source,
            &combined,
            root_manifest,
            root_id,
            new_graph,
        )?;
        *visibility_analysis = effective.analysis;
        sync_fetched_to_graph(&effective.graph, fetched, |node| {
            fetch_or_defer(
                source,
                node,
                lockfile,
                store_root,
                root_features,
                workspace_root,
                offline,
                &fetch_task.progress(),
            )
        })?;
        fetch_task.set_progress(fetched.len() as u64, None, "packages");
    }
}

/// Re-seat `fetched` onto `graph`: a node an earlier pass already fetched is
/// carried over untouched, a node new to this pass is obtained through
/// `acquire`, and every retained node's edges are refreshed from the solved
/// graph.
///
/// Acquisition is a parameter, not six more borrows of the planning context:
/// the re-seating itself only needs to know WHICH nodes are new, never how
/// one is fetched. That keeps the arity real rather than renamed — the
/// context the caller already holds stays in the caller.
fn sync_fetched_to_graph(
    graph: &vibe_resolver::ResolvedGraph,
    fetched: &mut Vec<Fetched>,
    mut acquire: impl FnMut(&ResolvedNode) -> Result<Fetched>,
) -> Result<()> {
    let mut existing = std::mem::take(fetched);
    let mut next = Vec::with_capacity(graph.packages.len());
    for node in graph.iter() {
        let retained = existing.iter().position(|candidate| {
            candidate.cached.resolved.group == node.group
                && candidate.cached.resolved.name == node.name
                && candidate.cached.resolved.version == node.version
        });
        let mut item = match retained {
            Some(index) => existing.remove(index),
            None => acquire(node)?,
        };
        item.meta.dependencies.clone_from(&node.dependencies);
        item.meta.is_root = node.is_root;
        next.push(item);
    }
    *fetched = next;
    Ok(())
}
