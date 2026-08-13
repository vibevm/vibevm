//! The acquisition half of planning (see [`super`]): fetching a node,
//! deferring one, the in-place incremental fast path, and conditional-deps
//! expansion — split from `plan.rs` along its responsibility seam.

use specmark::spec;

use super::*;

/// Resolve-and-fetch one solved node, expanding its features. Roots
/// get the caller's feature request tailored to what the package
/// declares; transitives get the default set.
pub(super) fn fetch_node<S: InstallSource + ?Sized>(
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
pub(super) fn fetch_or_defer<S: InstallSource + ?Sized>(
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
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#in-place",
    r = 1
)]
pub(super) fn try_in_place_incremental(
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
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#req-conditional-fixpoint"
)]
pub(super) fn expand_conditional_deps<S: InstallSource + ?Sized>(
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
