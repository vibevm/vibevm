//! The `vibe install` pipeline — plan → confirm → apply, end to end
//! (`VIBEVM-SPEC.md` §5.6, §9.1, §11.1). Resolver construction lives in
//! [`super::resolver`], plan-side helpers in [`super::planning`], and
//! manifest / lockfile recording plus reporting in [`super::recording`].

specmark::scope!("spec://vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::collections::BTreeSet;
use std::fs;

use crate::exit_code::InstallError;
use anyhow::{Context, Result, bail};
use dialoguer::Confirm;
use vibe_core::manifest::Manifest;
use vibe_core::user_config::UserConfig;
use vibe_core::{Group, PackageRef, VersionSpec};
use vibe_resolver::{
    FeatureRequest, ResolvedNode, conditional::ConditionalPredicate, expand_features,
};
use vibe_workspace::Workspace;
use vibe_workspace::install::{ResolvedDep, apply_resolution};

use crate::cli::InstallArgs;
use crate::commands::short_name;
use crate::output;

use super::planning::{
    Fetched, NodeInstallMeta, build_activation_context, build_feature_request,
    build_language_chain, exact_pinned_pkgref, load_or_empty_lockfile, load_project_manifest,
    resolve_project_root, tailor_feature_request,
};
use super::recording::{
    emit_fresh_report, emit_report, finalize_pkgref_for_manifest, locked_package_from_fetched,
    merge_manifest_requires, merge_root_dependencies, present_resolution,
};
use super::resolver::{apply_git_source_flag, build_install_resolver};

pub fn run(ctx: &output::Context, args: InstallArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let workspace = Workspace::discover(&project_root)
        .context("discovering the workspace enclosing the project")?;
    let mut manifest = load_project_manifest(&project_root)?;
    let mut lockfile = load_or_empty_lockfile(&workspace.root)?;
    // PROP-011 §2.3 — the materialise-diff strategy, read once from the
    // user config so a malformed config fails before any resolution.
    let slot_integrity = UserConfig::load()
        .context("loading the user config")?
        .install
        .slot_integrity;

    // M1.15: `vibe install <pkgref> --git <url> --tag/branch/rev <ref>`
    // adds a git-source declaration to `[requires.packages]` before
    // resolving. The added declaration is picked up by the resolver
    // built immediately below; subsequent installs of the same project
    // reproduce the install via the now-recorded git-source entry.
    if args.git.is_some() {
        apply_git_source_flag(&args, &mut manifest, &project_root)
            .context("recording --git declaration to vibe.toml")?;
    }

    let resolver = build_install_resolver(&args, &manifest)?;

    // PROP-003 §2.7 language chain (CLI flag > project [i18n]).
    let language_chain = build_language_chain(args.language.as_deref(), &manifest);
    // PROP-003 §2.4 root-package feature request.
    let root_feature_request = build_feature_request(&args);

    // Cache layout matches §8.3: `.vibe/cache/<kind>/<name>/<version>/`.
    // The cache lives at the absolute workspace root — one shared cache.
    let cache_root = workspace.root.join(".vibe/cache");
    fs::create_dir_all(&cache_root)
        .with_context(|| format!("creating cache dir `{}`", cache_root.display()))?;

    // 1. Decide the effective root list. Three input shapes:
    //
    //    a. CLI pkgrefs given (`vibe install flow:wal …`) — those are
    //       the roots; they are also merged into `vibe.toml`
    //       `[requires].packages` after a successful apply (Cargo /
    //       npm shape: explicit install records the dep on disk).
    //    b. No CLI args, manifest has `[requires].packages` — install
    //       every declared entry. The cargo `cargo build` / `npm
    //       install` shape: a fresh clone reproduces the project's
    //       package set without re-typing.
    //    c. No CLI args, manifest is empty, but the lockfile already
    //       carries `meta.root_dependencies` — first-run migration
    //       path for projects that pre-date the `[requires]` schema
    //       (PROP-002 §2.7). Seed the manifest from the lockfile
    //       snapshot, persist the manifest, and proceed as in case b.
    //
    //    Anything else (no CLI, no manifest entries, no lockfile
    //    snapshot) is an error — there is nothing to install.
    let cli_roots: Vec<PackageRef> = args
        .packages
        .iter()
        .map(|raw| PackageRef::parse(raw).with_context(|| format!("parsing `{raw}`")))
        .collect::<Result<_>>()?;

    // PROP-008 §2.6 — short-name resolution at the CLI input boundary.
    // A bare `vibe install wal` is qualified to `org.vibevm/wal` here,
    // once, before the depsolver runs and before the pkgref is merged
    // into `[requires].packages` — manifests only ever store the
    // qualified form. A pkgref that already carries a group passes
    // through untouched; an unresolvable or ambiguous short name fails
    // the command (the resolver never guesses — PROP-008 §2.7).
    let cli_roots: Vec<PackageRef> = cli_roots
        .iter()
        .map(|r| short_name::qualify(&resolver, r, &lockfile))
        .collect::<Result<_>>()?;

    let roots: Vec<PackageRef> = if cli_roots.is_empty() {
        // Legacy migration (case c): a project predating `[requires]`
        // whose entry manifest is empty but whose lockfile snapshot is
        // not — seed the entry manifest from `meta.root_dependencies`,
        // persisted before solving so a panic mid-solve cannot lose it.
        if manifest.requires.packages.is_empty()
            && manifest.requires.git_packages.is_empty()
            && !lockfile.meta.root_dependencies.is_empty()
        {
            ctx.step(&format!(
                "Migrating [requires] from `vibe.lock` meta.root_dependencies ({} entry{})",
                lockfile.meta.root_dependencies.len(),
                if lockfile.meta.root_dependencies.len() == 1 {
                    ""
                } else {
                    "ies"
                },
            ));
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
        // standalone project is a one-node workspace, so this degenerates
        // to "just the entry node". The resolver dispatches each pkgref
        // through the right path internally (override > git > registry).
        let discovered = Workspace::discover(&project_root)
            .context("re-discovering the workspace to collect every member's [requires]")?;
        let mut all: Vec<PackageRef> = Vec::new();
        // De-duplicate on the `(group, name)` identity (PROP-008 §2.3). A
        // manifest pkgref is group-qualified, so `group` is always present.
        let mut seen: std::collections::HashSet<(Option<Group>, String)> =
            std::collections::HashSet::new();
        for (_, node) in discovered.iter_nodes() {
            for p in &node.requires.packages {
                if seen.insert((p.group.clone(), p.name.clone())) {
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
            bail!(
                "no packages to install. Pass `<group>/<name>[@<version>] …` on the command \
                 line, or add entries to `[requires].packages` in `{}/vibe.toml`.",
                project_root.display()
            );
        }
        all
    } else {
        cli_roots.clone()
    };

    // The root set the depsolver actually runs against. For a CLI-pkgref
    // install it is `roots` verbatim; for a stale bare install PROP-011
    // §5.3 replaces it with the pin-held set below.
    let mut solve_roots = roots.clone();

    // PROP-011 §2.2 — the freshness fast path. When no CLI pkgref was
    // given (the install-from-manifest shape) and `vibe.lock` is already
    // a correct resolution of every node's `[requires]`, the depsolver —
    // a registry walk over the network — is skipped entirely: the
    // resolution is the lock, and application is just a whole-tree boot
    // regeneration (cheap, self-healing — PROP-011 §2.4). This is also
    // what makes `vibe install` lockfile-respecting: a fresh lock is
    // honoured verbatim, with no version drift inside a constraint. An
    // explicit `vibe install <pkgref>` always runs the full pipeline.
    if cli_roots.is_empty() {
        let ws = Workspace::discover(&project_root)
            .context("re-discovering the workspace for the freshness check")?;
        match vibe_workspace::freshness::check(&ws, &lockfile) {
            vibe_workspace::freshness::Freshness::Fresh => {
                ctx.heading("vibe.lock is fresh — skipping resolution");
                let nodes = vibe_workspace::install::regenerate_boot(&ws)
                    .context("regenerating boot artifacts from the materialised state")?;
                return emit_fresh_report(ctx, &nodes);
            }
            vibe_workspace::freshness::Freshness::Stale(reason) => {
                ctx.step(&format!("re-resolving — {reason}"));
                // PROP-011 §5.3 — minimum churn: hold the locked version
                // of every root the change did not touch, so re-resolution
                // moves only the changed dependency and its subtree.
                solve_roots = vibe_workspace::freshness::hold_pins(&roots, &lockfile);
            }
        }
    }

    // 2. Run the depsolver.
    ctx.heading(&format!(
        "Resolving {} root package{}…",
        roots.len(),
        if roots.len() == 1 { "" } else { "s" }
    ));
    let graph = match resolver.solve(&solve_roots) {
        Ok(graph) => graph,
        Err(e) if solve_roots != roots => {
            // PROP-011 §5.3 — the pin-held set over-constrained: a changed
            // dependency is incompatible with a held pin. Fall back to a
            // full, free re-resolve.
            ctx.step(&format!(
                "held pins conflicted with the change ({e}); re-resolving freely"
            ));
            resolver
                .solve(&roots)
                .with_context(|| "dependency resolution failed")?
        }
        Err(e) => {
            return Err(anyhow::Error::new(e).context("dependency resolution failed"));
        }
    };

    if graph.packages.len() > roots.len() {
        ctx.step(&format!(
            "{} root, {} transitive — {} package{} total",
            roots.len(),
            graph.packages.len() - roots.len(),
            graph.packages.len(),
            if graph.packages.len() == 1 { "" } else { "s" },
        ));
    }

    // 3. Phase one — fetch every node, pin features per node. We need
    //    full graph + every fetched manifest before we can build the
    //    activation context, since context probes (`if_present`,
    //    `if_provides`, `if_describes_match`) depend on the union of
    //    capabilities, interfaces, and PURLs across the graph.
    let mut fetched: Vec<Fetched> = Vec::with_capacity(graph.packages.len());

    for node in graph.iter() {
        let pkgref = exact_pinned_pkgref(node);
        let expected = lockfile
            .find(&node.group, &node.name)
            .map(|p| p.content_hash.clone());
        let cached = resolver.resolve_and_fetch(&pkgref, &cache_root, expected.as_deref())?;
        // Roots get the user-requested feature set, but features the
        // root doesn't declare are silently filtered out — multi-root
        // `vibe install A B --features X` should not fail just because
        // X belongs to A and not B. Cargo's `--features` behaviour for
        // multi-root installs is the same. The cross-root visibility
        // check at the end of phase 1 surfaces a warning if some
        // requested feature ended up matching no root at all.
        let req = if node.is_root {
            tailor_feature_request(&root_feature_request, &cached.manifest.features)
        } else {
            FeatureRequest::default()
        };
        let feature_expansion =
            expand_features(&cached.manifest.features, &req).with_context(|| {
                format!(
                    "expanding features for `{}/{}@{}`",
                    cached.resolved.group, cached.resolved.name, cached.resolved.version
                )
            })?;
        fetched.push(Fetched {
            cached,
            feature_expansion,
            meta: NodeInstallMeta {
                dependencies: node.dependencies.clone(),
                is_root: node.is_root,
            },
        });
    }

    // Visibility check: warn if `--features X` was requested but no
    // root package accepted X.
    if !root_feature_request.explicit.is_empty() {
        let accepted: BTreeSet<&str> = fetched
            .iter()
            .filter(|f| f.meta.is_root)
            .flat_map(|f| f.feature_expansion.active_features.iter())
            .map(|s| s.as_str())
            .collect();
        let unmatched: Vec<&str> = root_feature_request
            .explicit
            .iter()
            .filter(|f| !accepted.contains(f.as_str()))
            .map(|s| s.as_str())
            .collect();
        if !unmatched.is_empty() {
            ctx.step(&format!(
                "warning: requested feature{} {} not declared on any root package — silently ignored",
                if unmatched.len() == 1 { "" } else { "s" },
                unmatched
                    .iter()
                    .map(|s| format!("`{s}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }

    // 4. Conditional dependency expansion (PROP-003 §2.6.1) —
    //    fixed-point loop. Each pass: build activation context from
    //    currently-fetched packages; walk every package's
    //    `[target."context(...)".dependencies]`; if any predicate
    //    matches and its targets aren't already in the graph, add
    //    them as extra roots; re-solve and fetch. Repeat until no new
    //    extras emerge, or until the iteration cap is hit.
    //
    //    Convergence: extras only ADD packages to the fetched set
    //    (monotonic), and the predicate evaluation is a pure function
    //    of `present` + `provides` which only grow. So either a pass
    //    produces no extras (terminates), or every pass adds at
    //    least one package — bounded by the registry's size.
    //
    //    The cap (5 iterations) catches authoring-bug cases where a
    //    chain of conditional deps re-triggers on each iteration
    //    without converging. The conservative cap surfaces as a
    //    loud `bail!` so the operator can either fix the chain or
    //    bump the limit explicitly. No realistic graph reaches the
    //    cap.
    const COND_DEP_MAX_ITER: usize = 5;
    let mut iteration: usize = 0;
    loop {
        iteration += 1;
        let preliminary_ctx = build_activation_context(
            fetched.iter().map(|f| &f.cached),
            &project_root,
            &language_chain,
        )?;
        let mut extra: Vec<PackageRef> = Vec::new();
        for f in &fetched {
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
            break;
        }
        if iteration > COND_DEP_MAX_ITER {
            bail!(
                "conditional-dep expansion exceeded {COND_DEP_MAX_ITER} iterations — cascading predicates may form a cycle or runaway chain. Pending extras at break: {extras:?}",
                extras = extra.iter().map(|r| r.qualified_name()).collect::<Vec<_>>()
            );
        }
        ctx.step(&format!(
            "Conditional dependencies (iter {}): {} extra root{}",
            iteration,
            extra.len(),
            if extra.len() == 1 { "" } else { "s" }
        ));
        let mut combined = roots.clone();
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
        combined.retain(|r| seen.insert((r.group.clone(), r.name.clone())));
        let new_graph = resolver
            .solve(&combined)
            .with_context(|| "dependency resolution failed during conditional expansion")?;
        for node in new_graph.iter() {
            if fetched.iter().any(|g| {
                g.cached.resolved.group == node.group && g.cached.resolved.name == node.name
            }) {
                continue;
            }
            let pkgref = exact_pinned_pkgref(node);
            let expected = lockfile
                .find(&node.group, &node.name)
                .map(|p| p.content_hash.clone());
            let cached = resolver.resolve_and_fetch(&pkgref, &cache_root, expected.as_deref())?;
            let req = if node.is_root {
                tailor_feature_request(&root_feature_request, &cached.manifest.features)
            } else {
                FeatureRequest::default()
            };
            let feature_expansion =
                expand_features(&cached.manifest.features, &req).with_context(|| {
                    format!(
                        "expanding features for `{}/{}@{}`",
                        cached.resolved.group, cached.resolved.name, cached.resolved.version
                    )
                })?;
            fetched.push(Fetched {
                cached,
                feature_expansion,
                meta: NodeInstallMeta {
                    dependencies: node.dependencies.clone(),
                    is_root: node.is_root,
                },
            });
        }
    }

    // 5. Build the resolution — every fetched package as a `ResolvedDep`
    //    the workspace orchestrator materialises. The loading model
    //    materialises a package's tree verbatim, so the per-file
    //    activation context is no longer consulted at install time.
    let resolved_language: Option<String> = language_chain.first().cloned().filter(|l| l != "en");
    let resolution: Vec<ResolvedDep> = fetched
        .iter()
        .map(|f| ResolvedDep {
            kind: f.cached.package_meta().kind,
            group: f.cached.resolved.group.clone(),
            name: f.cached.resolved.name.clone(),
            version: f.cached.resolved.version.clone(),
            content_dir: f.cached.cache_dir.clone(),
            manifest: f.cached.manifest.clone(),
            // A `[requires.packages]` dependency pkgref is group-qualified
            // (PROP-008 §2.6).
            requires: f
                .meta
                .dependencies
                .iter()
                .filter_map(|p| p.group.clone().map(|g| (g, p.name.clone())))
                .collect(),
        })
        .collect();

    // Show the plan: the packages to materialise.
    present_resolution(ctx, &resolution);

    // Confirm (unless --assume-yes or --json or not a TTY).
    let approved = if args.assume_yes || ctx.is_unattended() || ctx.is_json() {
        true
    } else if !console::user_attended() {
        // No TTY → refuse to apply without explicit --assume-yes. This matches
        // the book's "ask a human" discipline for any destructive action.
        bail!(
            "no TTY available for confirmation; re-run with `--assume-yes` to apply this plan non-interactively"
        );
    } else {
        Confirm::new()
            .with_prompt(format!(
                "Materialise {} package{} into vibedeps/ and regenerate boot artifacts?",
                resolution.len(),
                if resolution.len() == 1 { "" } else { "s" },
            ))
            .default(false)
            .interact()
            .context("reading user confirmation")?
    };

    if !approved {
        return Err(InstallError::UserDeclined.into());
    }

    // 6. Update `vibe.toml` `[requires].packages` with the CLI-supplied
    //    roots — caret by default, `--exact` pins `=<resolved>`, an
    //    explicit constraint is preserved verbatim. De-dup by
    //    `(group, name)`; a no-op in install-from-manifest mode.
    //
    //    This MUST run before the boot regeneration below: `apply_resolution`
    //    composes each node's boot from its `[requires]`, so a package
    //    installed by pkgref has to be declared first or its boot snippet
    //    is dropped from the generated `INDEX.md`.
    let finalized_cli_roots: Vec<PackageRef> = cli_roots
        .iter()
        .map(|cli_pkgref| {
            let resolved = fetched
                .iter()
                .find(|f| {
                    Some(&f.cached.resolved.group) == cli_pkgref.group.as_ref()
                        && f.cached.resolved.name == cli_pkgref.name
                })
                .map(|f| &f.cached.resolved.version)
                .expect("every CLI root has a fetched package — resolver invariant");
            finalize_pkgref_for_manifest(cli_pkgref, resolved, args.exact)
        })
        .collect();
    let manifest_changed = if !finalized_cli_roots.is_empty() {
        merge_manifest_requires(&mut manifest, &finalized_cli_roots)
    } else {
        false
    };
    if manifest_changed {
        manifest.write(project_root.join(Manifest::FILENAME))?;
    }

    // 7. Re-discover the workspace so the boot computation reads the
    //    just-updated `[requires]` from disk.
    let workspace = Workspace::discover(&project_root)
        .context("re-discovering the workspace after the manifest update")?;

    // 8. Apply: materialise each package into vibedeps/ and regenerate
    //    every node's boot artifacts.
    let outcome = apply_resolution(&workspace, &resolution, slot_integrity)
        .context("materialising the resolution into the workspace")?;

    // 9. Rebuild the lockfile from the fresh resolution — `vibe install`
    //    re-resolves the whole graph, so the recorded package set is
    //    replaced wholesale.
    lockfile.packages.clear();
    for f in &fetched {
        lockfile
            .packages
            .push(locked_package_from_fetched(f, resolved_language.as_deref()));
    }
    lockfile.meta.generated_at = crate::commands::init::current_timestamp_utc();
    if !language_chain.is_empty() && language_chain != ["en"] {
        lockfile.meta.language_chain = language_chain.clone();
    }
    let mut active_features_global: BTreeSet<String> = BTreeSet::new();
    for f in &fetched {
        let pkg_label = format!("{}/{}", f.cached.resolved.group, f.cached.resolved.name);
        for feat in &f.feature_expansion.active_features {
            active_features_global.insert(format!("{pkg_label}/{feat}"));
        }
    }
    lockfile.meta.active_features = active_features_global.into_iter().collect();

    // 10. Mirror the declared roots into `meta.root_dependencies` so the
    //     lockfile stays a self-contained snapshot (PROP-002 §2.7).
    let lock_roots: &[PackageRef] = if cli_roots.is_empty() {
        &roots
    } else {
        &finalized_cli_roots
    };
    merge_root_dependencies(&mut lockfile, lock_roots);

    lockfile.write(workspace.lockfile_path())?;

    emit_report(ctx, &outcome)?;
    Ok(())
}
