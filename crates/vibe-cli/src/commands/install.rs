//! `vibe install <kind>:<name>[@version] …` — plan → confirm → apply.
//!
//! Spec: `VIBEVM-SPEC.md` §5.6, §9.1, §11.1.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::exit_code::InstallError;
use anyhow::{Context, Result, anyhow, bail};
use dialoguer::Confirm;
use serde::Serialize;
use vibe_core::manifest::{LockedPackage, Lockfile, Manifest, SourceKind};
use vibe_core::user_config::UserConfig;
use vibe_core::{Group, PackageRef, VersionSpec};
use vibe_registry::{CachedPackage, LocalRegistry, MultiRegistryResolver};
use vibe_resolver::{
    ActivationContext, CapabilityTag, FeatureExpansion, FeatureRequest, ResolvedNode,
    conditional::ConditionalPredicate, expand_features,
};
use vibe_workspace::Workspace;
use vibe_workspace::install::{InstallOutcome, ResolvedDep, apply_resolution};

use crate::cli::InstallArgs;
use crate::commands::short_name;
use crate::output;

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

/// Per-node install metadata threaded from the solver into the lockfile
/// register call.
struct NodeInstallMeta {
    dependencies: Vec<PackageRef>,
    is_root: bool,
}

/// One resolved + fetched package, with the feature expansion and the
/// per-node metadata gathered alongside it during resolution.
struct Fetched {
    cached: CachedPackage,
    feature_expansion: FeatureExpansion,
    meta: NodeInstallMeta,
}

/// Build a `<group>/<name>@=<exact-version>` pkgref for fetching the
/// version the solver chose, regardless of how the user originally
/// constrained the package.
pub(crate) fn exact_pinned_pkgref(node: &ResolvedNode) -> PackageRef {
    let req = semver::VersionReq::parse(&format!("={}", node.version))
        .expect("exact version always parses as VersionReq");
    PackageRef {
        kind: None,
        group: Some(node.group.clone()),
        name: node.name.clone(),
        version: VersionSpec::Req(req),
    }
}

/// Merge new root pkgrefs into `lockfile.meta.root_dependencies`,
/// deduplicating on `(group, name)` (idempotent re-installs don't grow
/// the list). Existing entries for the same `(group, name)` are
/// overwritten by the new pkgref so a constraint change in
/// `vibe install` updates the recorded root constraint.
fn merge_root_dependencies(lockfile: &mut Lockfile, roots: &[PackageRef]) {
    for r in roots {
        let pos = lockfile
            .meta
            .root_dependencies
            .iter()
            .position(|existing| existing.group == r.group && existing.name == r.name);
        match pos {
            Some(i) => lockfile.meta.root_dependencies[i] = r.clone(),
            None => lockfile.meta.root_dependencies.push(r.clone()),
        }
    }
}

/// Convert a CLI-supplied root into the form that lands on disk in
/// `vibe.toml` `[requires].packages`. Three cases:
///
/// 1. `--exact` set → always `=<resolved-version>`, ignoring whatever
///    constraint the user typed (matches npm `--save-exact` —
///    operator wants exact pinning, not the default).
/// 2. CLI had no version (`flow:wal` → `VersionSpec::Latest`) → write
///    caret based on the resolved version (`^0.1.0`). Same default as
///    Cargo `cargo add`, npm `npm install`, Poetry `poetry add`.
/// 3. CLI had an explicit constraint (`@^0.1`, `@=0.2.0`, `@~0.3.1`,
///    `@>=0.2, <1.0`, …) → preserve it verbatim. The user already
///    declared their intent; we don't second-guess.
fn finalize_pkgref_for_manifest(
    cli_pkgref: &PackageRef,
    resolved_version: &semver::Version,
    exact: bool,
) -> PackageRef {
    let version = if exact {
        let req = semver::VersionReq::parse(&format!("={resolved_version}"))
            .expect("`=<version>` always parses as VersionReq");
        VersionSpec::Req(req)
    } else if matches!(cli_pkgref.version, VersionSpec::Latest) {
        let req = semver::VersionReq::parse(&format!("^{resolved_version}"))
            .expect("`^<version>` always parses as VersionReq");
        VersionSpec::Req(req)
    } else {
        cli_pkgref.version.clone()
    };
    PackageRef {
        kind: cli_pkgref.kind,
        group: cli_pkgref.group.clone(),
        name: cli_pkgref.name.clone(),
        version,
    }
}

/// Merge new root pkgrefs into `manifest.requires.packages`, same
/// dedup discipline as `merge_root_dependencies`. Returns `true` if
/// any entry was added or changed — caller writes the manifest only
/// when the in-memory shape actually diverged from disk.
///
/// Skips pkgrefs that are already declared as a git-source in
/// `manifest.requires.git_packages` — those were recorded earlier via
/// `apply_git_source_flag` (M1.15) and writing them again as
/// registry-resolved would create a `(group, name)` duplicate that
/// `try_from = "RequiresWire"` rejects on the next parse.
fn merge_manifest_requires(manifest: &mut Manifest, roots: &[PackageRef]) -> bool {
    let mut changed = false;
    for r in roots {
        if manifest
            .requires
            .git_packages
            .iter()
            .any(|g| Some(&g.group) == r.group.as_ref() && g.name == r.name)
        {
            // Already declared as git-source — leave untouched.
            continue;
        }
        let pos = manifest
            .requires
            .packages
            .iter()
            .position(|existing| existing.group == r.group && existing.name == r.name);
        match pos {
            Some(i) => {
                if manifest.requires.packages[i] != *r {
                    manifest.requires.packages[i] = r.clone();
                    changed = true;
                }
            }
            None => {
                manifest.requires.packages.push(r.clone());
                changed = true;
            }
        }
    }
    changed
}

fn resolve_project_root(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = super::init::strip_unc_public(canonical);
    if !stripped.join(Manifest::FILENAME).exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            stripped.display()
        );
    }
    Ok(stripped)
}

fn load_project_manifest(root: &Path) -> Result<Manifest> {
    let path = root.join(Manifest::FILENAME);
    Ok(Manifest::read(&path)?)
}

fn load_or_empty_lockfile(root: &Path) -> Result<Lockfile> {
    let path = root.join(Lockfile::FILENAME);
    if path.exists() {
        Ok(Lockfile::read(&path)?)
    } else {
        Ok(Lockfile::empty(
            format!("vibe {}", env!("CARGO_PKG_VERSION")),
            crate::commands::init::current_timestamp_utc(),
        ))
    }
}

/// Either a M0-shape local-directory registry (used by `--registry <path>`
/// and the in-tree fixture path) or a full PROP-002 multi-registry
/// resolver covering the `[[registry]]` / `[[mirror]]` / `[[override]]`
/// sections in `vibe.toml`.
///
/// Both branches expose:
/// - [`Self::resolve_and_fetch`] — returns a [`CachedPackage`] with
///   lockfile-v2 provenance fields populated by the underlying impl
///   (`None` / `false` for the local-dir path; populated for the
///   per-package + override paths).
/// - [`Self::solve`] — runs the depsolver against this resolver via the
///   appropriate `DepProvider` adapter. Returns the full transitive
///   graph the install pipeline materialises.
pub(crate) enum InstallResolver {
    Local(LocalRegistry),
    // Boxed: `MultiRegistryResolver` is by far the larger variant
    // (it carries the registry list plus the override / git-source /
    // path-source maps), so an unboxed enum would bloat every
    // `InstallResolver` value to the size of the multi-registry path.
    Multi(Box<MultiRegistryResolver>),
}

impl InstallResolver {
    /// Resolve `pkgref` and materialise its content into the
    /// per-project cache. `expected_hash` (typically the lockfile pin
    /// for `(pkgref.kind, pkgref.name, version)`) is forwarded to the
    /// multi-registry path's mirror-aware fetch so a source serving
    /// disagreeing bytes can be skipped in favour of a matching one.
    /// The local-directory path ignores the hint — there's only ever
    /// one source on that path, and integrity is checked by
    /// `plan_install` against the lockfile pin.
    pub(crate) fn resolve_and_fetch(
        &self,
        pkgref: &PackageRef,
        cache_root: &Path,
        expected_hash: Option<&str>,
    ) -> Result<CachedPackage> {
        match self {
            InstallResolver::Local(r) => {
                let resolved = r.resolve(pkgref)?;
                Ok(r.fetch(&resolved, cache_root)?)
            }
            InstallResolver::Multi(m) => {
                let resolution = m.resolve(pkgref)?;
                Ok(m.fetch_with_expected_hash(&resolution, cache_root, expected_hash)?)
            }
        }
    }

    pub(crate) fn solve(
        &self,
        roots: &[PackageRef],
    ) -> Result<vibe_resolver::ResolvedGraph, vibe_resolver::SolveError> {
        // Cell selection lives in the registry module (R-001); this
        // match only routes the resource the caller already owns.
        let flags = crate::registry::selection_flags(matches!(self, InstallResolver::Local(_)));
        let solver = match self {
            InstallResolver::Local(r) => {
                crate::registry::dep_solver(&flags, crate::registry::ProviderResource::Local(r))
            }
            InstallResolver::Multi(m) => {
                crate::registry::dep_solver(&flags, crate::registry::ProviderResource::Multi(m))
            }
        };
        solver.solve(roots)
    }

    /// Enumerate every `group` that publishes a package of the bare
    /// `name` — the candidate set short-name resolution (PROP-008
    /// §2.6) walks. The local-directory path scans the registry tree;
    /// the multi-registry path walks each registry's index. The result
    /// is de-duplicated and sorted; `len() > 1` is a collision.
    pub(crate) fn candidate_groups(&self, name: &str) -> Result<Vec<Group>> {
        match self {
            InstallResolver::Local(r) => Ok(r.candidate_groups(name)?),
            InstallResolver::Multi(m) => Ok(m.resolve_name_candidates(name)),
        }
    }
}

/// Process the M1.15 `--git`/`--tag`/`--branch`/`--rev`/`--git-auth`/
/// `--git-token-env` flags. Validates the flag combination, parses
/// the single positional pkgref, builds a `GitPackageDep`, merges it
/// into `manifest.requires.git_packages` (replacing any prior entry
/// for the same `(kind, name)`), and persists the manifest before
/// resolving so a panic mid-resolve cannot leave the on-disk
/// declaration out of sync. Removes any conflicting registry-resolved
/// entry for the same pkgref to keep `manifest.requires` in a valid
/// shape (no duplicate `(kind, name)` between `packages` and
/// `git_packages`).
fn apply_git_source_flag(
    args: &InstallArgs,
    manifest: &mut Manifest,
    project_root: &std::path::Path,
) -> Result<()> {
    use vibe_core::manifest::{AuthKind, GitPackageDep, GitRefKind};

    if args.exact {
        bail!(
            "--exact has no meaning with --git (constraint shape is registry-resolved); drop one of the two flags"
        );
    }
    if args.registry.is_some() {
        bail!("--git bypasses the registry layer; drop --registry or drop --git");
    }
    if args.packages.len() != 1 {
        bail!(
            "--git requires exactly one positional pkgref `<group>/<name>`; got {}",
            args.packages.len()
        );
    }
    // Allow user to type either `org.vibevm/internal` or
    // `org.vibevm/internal@*` — version is irrelevant for git-source (the
    // ref decides), but we accept both shapes for muscle-memory
    // compatibility.
    let pr = PackageRef::parse(&args.packages[0])
        .with_context(|| format!("parsing `{}`", args.packages[0]))?;
    let pr_group = pr.group.clone().ok_or_else(|| {
        anyhow!("package reference `{pr}` is not group-qualified — write `<group>/<name>`")
    })?;
    let url = args.git.clone().expect("caller checked args.git.is_some()");
    let ref_kind = match (
        args.tag.as_deref(),
        args.branch.as_deref(),
        args.rev.as_deref(),
    ) {
        (Some(t), None, None) => GitRefKind::Tag(t.to_string()),
        (None, Some(b), None) => GitRefKind::Branch(b.to_string()),
        (None, None, Some(r)) => GitRefKind::Rev(r.to_string()),
        (None, None, None) => bail!("--git requires exactly one of --tag, --branch, or --rev"),
        _ => bail!("--git accepts exactly one of --tag, --branch, --rev — drop the extras"),
    };
    let auth = match args.git_auth.as_deref() {
        None | Some("none") => AuthKind::None,
        Some("token-env") => AuthKind::TokenEnv,
        Some("credential-helper") => AuthKind::CredentialHelper,
        Some("ssh") => AuthKind::Ssh,
        Some(other) => bail!(
            "unknown --git-auth `{other}` — must be `none`, `token-env`, `credential-helper`, or `ssh`"
        ),
    };
    if args.git_token_env.is_some() && !matches!(auth, AuthKind::TokenEnv) {
        bail!(
            "--git-token-env is only meaningful with --git-auth token-env; got `{}`",
            args.git_auth.as_deref().unwrap_or("none")
        );
    }
    let dep = GitPackageDep {
        kind: pr.kind,
        group: pr_group.clone(),
        name: pr.name.clone(),
        url,
        ref_kind,
        version: None,
        auth,
        token_env: args.git_token_env.clone(),
    };

    // Drop any prior registry-resolved entry for the same pkgref —
    // M1.15 forbids `(group, name)` collision between
    // `requires.packages` and `requires.git_packages`.
    manifest
        .requires
        .packages
        .retain(|p| !(p.group.as_ref() == Some(&dep.group) && p.name == dep.name));
    // Replace any prior git-source entry for the same pkgref (same
    // shape as updating an existing constraint).
    manifest
        .requires
        .git_packages
        .retain(|g| !(g.group == dep.group && g.name == dep.name));
    manifest.requires.git_packages.push(dep);

    manifest.write(project_root.join(Manifest::FILENAME))?;
    Ok(())
}

/// Build the install resolver for this invocation.
///
/// Precedence (matches `VIBEVM-SPEC.md` §9.1):
/// 1. `--registry <path>` — explicit local-directory registry (M0 shape,
///    used by tests and offline workflows).
/// 2. `[[registry]]` array in `vibe.toml` → [`MultiRegistryResolver`]
///    covering priority order, mirrors, and overrides per
///    [PROP-002](../../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md).
pub(crate) fn build_install_resolver(
    args: &InstallArgs,
    manifest: &Manifest,
) -> Result<InstallResolver> {
    if let Some(explicit) = &args.registry {
        let p = explicit
            .canonicalize()
            .with_context(|| format!("registry path `{}`", explicit.display()))?;
        let p = super::init::strip_unc_public(p);
        let local = crate::registry::local_registry(p.clone())
            .map_err(|e| anyhow!("failed to open registry at `{}`: {e}", p.display()))?;
        return Ok(InstallResolver::Local(local));
    }

    if manifest.registries.is_empty() {
        bail!(
            "no registry configured. Pass `--registry <path>` or add a `[[registry]]` entry to `vibe.toml`."
        );
    }

    let mrr =
        MultiRegistryResolver::open(&manifest.registries, &manifest.mirrors, &manifest.overrides)
            .context("opening multi-registry resolver")?
            .with_strict_auth(args.auth_required)
            .with_git_packages(manifest.requires.git_packages.clone());
    Ok(InstallResolver::Multi(Box::new(mrr)))
}

/// Build a [`LockedPackage`] from a fetched node. The lockfile records the
/// resolution provenance; the materialised footprint is the `vibedeps/`
/// slot — deterministic from `(kind, name, version)` — so `files_written`
/// stays empty and the `NN-` `boot_snippet` filename is retired.
fn locked_package_from_fetched(f: &Fetched, language: Option<&str>) -> LockedPackage {
    let c = &f.cached;
    let source_kind = if c.overridden {
        SourceKind::Override
    } else if c.is_path_source {
        SourceKind::Path
    } else if c.is_git_source {
        SourceKind::Git
    } else {
        SourceKind::Registry
    };
    LockedPackage {
        kind: c.package_meta().kind,
        group: c.resolved.group.clone(),
        name: c.resolved.name.clone(),
        version: c.resolved.version.clone(),
        registry: c.registry_name.clone(),
        source_url: c.source_uri.clone(),
        source_ref: c.source_ref.clone(),
        resolved_commit: c.resolved_commit.clone(),
        content_hash: c.content_hash.clone(),
        boot_snippet: None,
        files_written: Vec::new(),
        dependencies: f.meta.dependencies.clone(),
        overridden: c.overridden,
        source_kind: Some(source_kind),
        via_redirect: c.via_redirect.clone(),
        features: f
            .feature_expansion
            .active_features
            .iter()
            .cloned()
            .collect(),
        subskills_active: Vec::new(),
        describes: c.package_meta().describes.as_ref().map(|p| p.to_string()),
        language: language.map(str::to_string),
    }
}

fn present_resolution(ctx: &output::Context, resolution: &[ResolvedDep]) {
    if ctx.is_json() {
        #[derive(Serialize)]
        struct PlanEntry {
            package: String,
            version: String,
        }
        let payload: Vec<PlanEntry> = resolution
            .iter()
            .map(|d| PlanEntry {
                package: format!("{}/{}", d.group, d.name),
                version: d.version.to_string(),
            })
            .collect();
        let _ = ctx.emit_json(&serde_json::json!({
            "command": "install:plan",
            "packages": payload,
        }));
        return;
    }
    if ctx.is_quiet() {
        return;
    }
    ctx.heading(&format!(
        "\nMaterialising {} package{} into vibedeps/:",
        resolution.len(),
        if resolution.len() == 1 { "" } else { "s" },
    ));
    for d in resolution {
        println!("  {}/{}@{}", d.group, d.name, d.version);
    }
    println!();
}

fn emit_report(ctx: &output::Context, outcome: &InstallOutcome) -> Result<()> {
    if ctx.is_json() {
        ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "install",
            "materialised": outcome.materialised,
            "skipped": outcome.skipped,
            "pruned": outcome.pruned,
            "nodes_regenerated": outcome.nodes_regenerated,
        }))?;
        return Ok(());
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe install: {} package{} materialised",
            outcome.materialised.len(),
            if outcome.materialised.len() == 1 {
                ""
            } else {
                "s"
            },
        ));
        return Ok(());
    }
    ctx.summary(&format!(
        "\nMaterialised {} package{} into vibedeps/; regenerated boot artifacts for {} node{}.",
        outcome.materialised.len(),
        if outcome.materialised.len() == 1 {
            ""
        } else {
            "s"
        },
        outcome.nodes_regenerated.len(),
        if outcome.nodes_regenerated.len() == 1 {
            ""
        } else {
            "s"
        },
    ));
    if !outcome.skipped.is_empty() {
        ctx.step(&format!(
            "{} slot{} already present — re-copy skipped (PROP-011 §2.3)",
            outcome.skipped.len(),
            if outcome.skipped.len() == 1 { "" } else { "s" },
        ));
    }
    if !outcome.pruned.is_empty() {
        ctx.step(&format!(
            "pruned {} stale vibedeps/ slot{}",
            outcome.pruned.len(),
            if outcome.pruned.len() == 1 { "" } else { "s" },
        ));
    }
    Ok(())
}

/// Report the PROP-011 §2.2 fast path — `vibe.lock` was fresh, so no
/// resolution ran. Kept distinct from [`emit_report`] so the operator can
/// tell a no-op `vibe install` from one that materialised packages.
fn emit_fresh_report(ctx: &output::Context, nodes_regenerated: &[String]) -> Result<()> {
    if ctx.is_json() {
        ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "install",
            "unchanged": true,
            "nodes_regenerated": nodes_regenerated,
        }))?;
        return Ok(());
    }
    ctx.summary(&format!(
        "vibe install: vibe.lock unchanged — nothing to re-resolve ({} node{} up to date)",
        nodes_regenerated.len(),
        if nodes_regenerated.len() == 1 {
            ""
        } else {
            "s"
        },
    ));
    Ok(())
}

/// Build the resolved language chain from a CLI flag override + the
/// project's `[i18n]` declaration. The CLI flag is the head of the
/// chain; project-level preference and fallback come next; canonical /
/// registry-default `en` close the chain.
fn build_language_chain(cli_language: Option<&str>, manifest: &Manifest) -> Vec<String> {
    let mut effective = manifest.i18n.clone();
    if let Some(lang) = cli_language {
        effective.preferred = Some(lang.to_string());
    }
    if effective.is_default() && cli_language.is_none() {
        Vec::new()
    } else {
        effective.project_preference_chain()
    }
}

/// Build the feature request to apply to root packages from the CLI
/// flags. `--all-features` wins over `--features` if both are set.
fn build_feature_request(args: &InstallArgs) -> FeatureRequest {
    FeatureRequest {
        explicit: args.features.clone(),
        no_defaults: args.no_default_features,
        all: args.all_features,
    }
}

/// Per-root-package tailoring: trim `explicit` features down to those
/// the package actually declares. A multi-root `vibe install A B
/// --features X` should not fail just because X belongs to A and not B
/// — silently filter X out of B's request and rely on the post-phase-1
/// visibility check to surface a warning if X matched no root at all.
fn tailor_feature_request(
    request: &FeatureRequest,
    table: &vibe_core::manifest::FeaturesTable,
) -> FeatureRequest {
    FeatureRequest {
        explicit: request
            .explicit
            .iter()
            .filter(|f| table.features.contains_key(f.as_str()))
            .cloned()
            .collect(),
        no_defaults: request.no_defaults,
        all: request.all,
    }
}

/// Build the [`ActivationContext`] from the set of fetched packages
/// plus project state. Walks every package's manifest once to populate
/// `present`, `provides`, and `describes_types`. Sets `project_root`
/// for `if_files` glob matching and `language_chain` for `if_language`.
fn build_activation_context<'a, I>(
    cached: I,
    project_root: &Path,
    language_chain: &[String],
) -> Result<ActivationContext>
where
    I: IntoIterator<Item = &'a CachedPackage>,
{
    let mut ctx = ActivationContext {
        project_root: Some(project_root.to_path_buf()),
        language_chain: language_chain.to_vec(),
        ..Default::default()
    };
    for c in cached {
        // The conditional-dep `context(<key>)` predicate matches an
        // opaque present-set token; for a package the token is the
        // `<kind>:<name>` tag (PROP-003 §2.6.1), consistent with the
        // `<type>:<name>` shape of capability / interface tags. This is
        // not a package label — identity remains `(group, name)`.
        // Both shapes are `:`-qualified by construction, so the parse
        // can only fail on a malformed manifest that slipped past
        // validation — which deserves the loud exit, not a silent skip.
        let kind_tag =
            CapabilityTag::parse(format!("{}:{}", c.package_meta().kind, c.resolved.name))
                .with_context(|| format!("package tag for `{}`", c.resolved.name))?;
        ctx.add_present(kind_tag);
        for cap in &c.manifest.provides.capabilities {
            let qualified = CapabilityTag::parse(cap.qualified())
                .with_context(|| format!("capability tag of `{}`", c.resolved.name))?;
            let is_interface = qualified.as_str().starts_with("interface:");
            ctx.add_present(qualified.clone());
            if is_interface {
                ctx.add_provides(qualified);
            }
        }
        if let Some(purl) = &c.package_meta().describes {
            ctx.describes_types.insert(purl.purl_type.clone());
        }
    }
    Ok(ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibe_core::manifest::ProjectSection;

    fn empty_manifest() -> Manifest {
        Manifest {
            project: Some(ProjectSection {
                name: "demo".to_string(),
                version: "0.0.1".to_string(),
                authors: vec![],
            }),
            ..Default::default()
        }
    }

    #[test]
    fn merge_manifest_requires_appends_new_pkgref() {
        let mut m = empty_manifest();
        let r = PackageRef::parse("flow:wal@^0.1").unwrap();
        let changed = merge_manifest_requires(&mut m, std::slice::from_ref(&r));
        assert!(changed);
        assert_eq!(m.requires.packages.len(), 1);
        assert_eq!(m.requires.packages[0], r);
    }

    #[test]
    fn merge_manifest_requires_idempotent_on_repeat() {
        let mut m = empty_manifest();
        let r = PackageRef::parse("flow:wal@^0.1").unwrap();
        merge_manifest_requires(&mut m, std::slice::from_ref(&r));
        // Second call with the same pkgref must not duplicate the entry
        // and must not mark the manifest dirty.
        let changed_again = merge_manifest_requires(&mut m, std::slice::from_ref(&r));
        assert!(
            !changed_again,
            "second merge of the same pkgref must be a no-op"
        );
        assert_eq!(m.requires.packages.len(), 1);
    }

    #[test]
    fn merge_manifest_requires_overwrites_constraint_change() {
        let mut m = empty_manifest();
        let r1 = PackageRef::parse("flow:wal@^0.1").unwrap();
        merge_manifest_requires(&mut m, std::slice::from_ref(&r1));
        let r2 = PackageRef::parse("flow:wal@=0.2.0").unwrap();
        let changed = merge_manifest_requires(&mut m, std::slice::from_ref(&r2));
        assert!(changed, "constraint change must mark the manifest dirty");
        assert_eq!(m.requires.packages.len(), 1);
        assert_eq!(m.requires.packages[0], r2);
    }

    fn vsemver(s: &str) -> semver::Version {
        semver::Version::parse(s).unwrap()
    }

    #[test]
    fn finalize_caret_when_cli_had_no_version() {
        // `vibe install flow:wal` → resolves 0.1.0 → manifest gets
        // `flow:wal@^0.1.0`. Same default as Cargo / npm / Poetry.
        let cli = PackageRef::parse("flow:wal").unwrap();
        let out = finalize_pkgref_for_manifest(&cli, &vsemver("0.1.0"), false);
        assert_eq!(out.to_string(), "flow:wal@^0.1.0");
    }

    #[test]
    fn finalize_preserves_explicit_caret() {
        let cli = PackageRef::parse("flow:wal@^0.1").unwrap();
        let out = finalize_pkgref_for_manifest(&cli, &vsemver("0.1.5"), false);
        // CLI form preserved — we don't tighten the operator's
        // explicitly stated constraint.
        assert_eq!(out, cli);
    }

    #[test]
    fn finalize_preserves_explicit_eq() {
        let cli = PackageRef::parse("flow:wal@=0.1.0").unwrap();
        let out = finalize_pkgref_for_manifest(&cli, &vsemver("0.1.0"), false);
        assert_eq!(out, cli);
    }

    #[test]
    fn finalize_preserves_explicit_tilde_and_range() {
        for raw in ["flow:wal@~0.1.0", "flow:wal@>=0.1, <0.3"] {
            let cli = PackageRef::parse(raw).unwrap();
            let out = finalize_pkgref_for_manifest(&cli, &vsemver("0.1.5"), false);
            assert_eq!(out, cli, "explicit constraint `{raw}` must be preserved");
        }
    }

    #[test]
    fn finalize_exact_overrides_cli_form_to_eq_resolved() {
        // `--exact` is always-pin: even `@^0.1` becomes `=0.1.5`.
        let cli = PackageRef::parse("flow:wal@^0.1").unwrap();
        let out = finalize_pkgref_for_manifest(&cli, &vsemver("0.1.5"), true);
        assert_eq!(out.to_string(), "flow:wal@=0.1.5");
    }

    #[test]
    fn finalize_exact_with_no_cli_version() {
        let cli = PackageRef::parse("flow:wal").unwrap();
        let out = finalize_pkgref_for_manifest(&cli, &vsemver("0.1.5"), true);
        assert_eq!(out.to_string(), "flow:wal@=0.1.5");
    }

    #[test]
    fn merge_manifest_requires_keeps_unrelated_entries() {
        let mut m = empty_manifest();
        let other = PackageRef::parse("stack:rust-cli").unwrap();
        m.requires.packages.push(other.clone());
        let r = PackageRef::parse("flow:wal").unwrap();
        merge_manifest_requires(&mut m, std::slice::from_ref(&r));
        assert_eq!(m.requires.packages.len(), 2);
        // Unrelated entry survives.
        assert!(m.requires.packages.contains(&other));
        assert!(m.requires.packages.contains(&r));
    }
}
