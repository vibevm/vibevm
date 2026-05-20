//! `vibe install <kind>:<name>[@version] …` — plan → confirm → apply.
//!
//! Spec: `VIBEVM-SPEC.md` §5.6, §9.1, §11.1.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use dialoguer::Confirm;
use serde::Serialize;
use vibe_core::{PackageRef, VersionSpec};
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_install::{
    InstallError, InstallOptions, InstallPlan, RegisterMetadata, WriteKind, apply_install,
    plan_install_with_options, register_installed_with_metadata,
};
use vibe_registry::{CachedPackage, LocalRegistry, MultiRegistryResolver};
use vibe_resolver::{
    ActivationContext, DepSolver, FeatureExpansion, FeatureRequest, LocalRegistryProvider,
    MultiRegistryProvider, NaiveDepSolver, ResolvedNode, conditional::ConditionalPredicate,
    expand_features,
};

use crate::cli::InstallArgs;
use crate::output;

pub fn run(ctx: &output::Context, args: InstallArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let mut manifest = load_project_manifest(&project_root)?;
    let mut lockfile = load_or_empty_lockfile(&project_root)?;

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
    let cache_root = project_root.join(".vibe/cache");
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
        .map(|raw| {
            PackageRef::parse(raw).with_context(|| format!("parsing `{raw}`"))
        })
        .collect::<Result<_>>()?;

    let roots: Vec<PackageRef> = if cli_roots.is_empty() {
        if manifest.requires.packages.is_empty()
            && manifest.requires.git_packages.is_empty()
            && !lockfile.meta.root_dependencies.is_empty()
        {
            ctx.step(&format!(
                "Migrating [requires] from `vibe.lock` meta.root_dependencies ({} entry{})",
                lockfile.meta.root_dependencies.len(),
                if lockfile.meta.root_dependencies.len() == 1 { "" } else { "ies" },
            ));
            manifest
                .requires
                .packages
                .clone_from(&lockfile.meta.root_dependencies);
            // Persist the migration before solving, so a panic mid-solve
            // does not lose the snapshot we just inferred.
            manifest.write(project_root.join(Manifest::FILENAME))?;
        }
        if manifest.requires.packages.is_empty() && manifest.requires.git_packages.is_empty() {
            bail!(
                "no packages to install. Pass `<kind>:<name>[@<version>] …` on the command \
                 line, or add entries to `[requires].packages` in `{}/vibe.toml`.",
                project_root.display()
            );
        }
        // Combine registry-resolved + git-source declarations into one
        // root set. Resolver dispatches each pkgref through the right
        // path internally (override > git-source > registry-walk).
        let mut all = manifest.requires.packages.clone();
        for g in &manifest.requires.git_packages {
            all.push(PackageRef::new(g.kind, g.name.clone(), VersionSpec::Latest)?);
        }
        all
    } else {
        cli_roots.clone()
    };

    // 2. Run the depsolver.
    ctx.heading(&format!(
        "Resolving {} root package{}…",
        roots.len(),
        if roots.len() == 1 { "" } else { "s" }
    ));
    let graph = resolver
        .solve(&roots)
        .with_context(|| "dependency resolution failed")?;

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
    struct Fetched {
        cached: CachedPackage,
        feature_expansion: FeatureExpansion,
        meta: NodeInstallMeta,
    }
    let mut fetched: Vec<Fetched> = Vec::with_capacity(graph.packages.len());

    for node in graph.iter() {
        let pkgref = exact_pinned_pkgref(node);
        let expected = lockfile
            .find(node.kind, &node.name)
            .map(|p| p.content_hash.clone());
        let cached =
            resolver.resolve_and_fetch(&pkgref, &cache_root, expected.as_deref())?;
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
                    "expanding features for `{}:{}@{}`",
                    cached.resolved.kind, cached.resolved.name, cached.resolved.version
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
        );
        let mut extra: Vec<PackageRef> = Vec::new();
        for f in &fetched {
            for (pred_str, target) in &f.cached.manifest.conditional_deps {
                match ConditionalPredicate::parse(pred_str) {
                    Ok(pred) => {
                        if pred.evaluate(&preliminary_ctx) {
                            for r in &target.dependencies.packages {
                                let already = fetched.iter().any(|g| {
                                    g.cached.resolved.kind == r.kind
                                        && g.cached.resolved.name == r.name
                                }) || extra.iter().any(|x| {
                                    x.kind == r.kind && x.name == r.name
                                });
                                if !already {
                                    extra.push(r.clone());
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            target: "vibe_install",
                            package = %format!("{}:{}", f.cached.resolved.kind, f.cached.resolved.name),
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
                extras = extra
                    .iter()
                    .map(|r| r.qualified_name())
                    .collect::<Vec<_>>()
            );
        }
        ctx.step(&format!(
            "Conditional dependencies (iter {}): {} extra root{}",
            iteration,
            extra.len(),
            if extra.len() == 1 { "" } else { "s" }
        ));
        let mut combined = roots.clone();
        combined.extend(
            fetched
                .iter()
                .filter(|f| f.meta.is_root)
                .map(|f| exact_pinned_pkgref(&ResolvedNode {
                    kind: f.cached.resolved.kind,
                    name: f.cached.resolved.name.clone(),
                    version: f.cached.resolved.version.clone(),
                    dependencies: Vec::new(),
                    is_root: true,
                })),
        );
        combined.extend(extra.iter().cloned());
        // De-duplicate by (kind, name).
        let mut seen: std::collections::HashSet<(_, String)> =
            std::collections::HashSet::new();
        combined.retain(|r| seen.insert((r.kind, r.name.clone())));
        let new_graph = resolver.solve(&combined).with_context(|| {
            "dependency resolution failed during conditional expansion"
        })?;
        for node in new_graph.iter() {
            if fetched.iter().any(|g| {
                g.cached.resolved.kind == node.kind
                    && g.cached.resolved.name == node.name
            }) {
                continue;
            }
            let pkgref = exact_pinned_pkgref(node);
            let expected = lockfile
                .find(node.kind, &node.name)
                .map(|p| p.content_hash.clone());
            let cached = resolver.resolve_and_fetch(
                &pkgref,
                &cache_root,
                expected.as_deref(),
            )?;
            let req = if node.is_root {
                tailor_feature_request(&root_feature_request, &cached.manifest.features)
            } else {
                FeatureRequest::default()
            };
            let feature_expansion =
                expand_features(&cached.manifest.features, &req).with_context(|| {
                    format!(
                        "expanding features for `{}:{}@{}`",
                        cached.resolved.kind,
                        cached.resolved.name,
                        cached.resolved.version
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

    // 5. Build the final activation context once from the (possibly
    //    expanded) graph.
    let activation_context = build_activation_context(
        fetched.iter().map(|f| &f.cached),
        &project_root,
        &language_chain,
    );

    // 5. Phase two — plan per node with feature expansion, activation
    //    context, and language chain plumbed in.
    let mut plans: Vec<InstallPlan> = Vec::new();
    let mut node_meta: Vec<NodeInstallMeta> = Vec::new();
    let mut feature_state: Vec<FeatureExpansion> = Vec::new();
    let mut described_purls: Vec<Option<String>> = Vec::new();
    let resolved_language: Option<String> =
        language_chain.first().cloned().filter(|l| l != "en");

    for f in fetched {
        let describes_string = f
            .cached
            .package_meta()
            .describes
            .as_ref()
            .map(|p| p.to_string());
        let opts = InstallOptions {
            language_chain: language_chain.clone(),
            feature_expansion: f.feature_expansion.clone(),
            activation_context: activation_context.clone(),
            describes: describes_string.clone(),
        };
        let plan = plan_install_with_options(&project_root, &lockfile, f.cached, &opts)?;
        check_cross_plan_conflicts(&plans, &plan)?;
        plans.push(plan);
        node_meta.push(f.meta);
        feature_state.push(f.feature_expansion);
        described_purls.push(describes_string);
    }

    // Show combined plan.
    present_plans(ctx, &project_root, &plans);

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
        let prompt = format!(
            "Apply this install plan ({} file{} across {} package{})?",
            total_writes(&plans),
            if total_writes(&plans) == 1 { "" } else { "s" },
            plans.len(),
            if plans.len() == 1 { "" } else { "s" },
        );
        Confirm::new()
            .with_prompt(prompt)
            .default(false)
            .interact()
            .context("reading user confirmation")?
    };

    if !approved {
        return Err(InstallError::UserDeclined.into());
    }

    // Apply each plan in turn; update lockfile after each success.
    let mut applied: Vec<AppliedReport> = Vec::new();
    for (idx, (plan, meta)) in plans.iter().zip(node_meta.iter()).enumerate() {
        let label = plan.package_label();
        ctx.step(&format!(
            "Installing {label}{}",
            if meta.is_root { "" } else { " (transitive)" }
        ));
        let written = apply_install(plan)?;
        let written_count = written.len();
        let metadata = RegisterMetadata {
            features: feature_state[idx]
                .active_features
                .iter()
                .cloned()
                .collect(),
            describes: described_purls[idx].clone(),
            language: resolved_language.clone(),
        };
        register_installed_with_metadata(
            &mut lockfile,
            plan,
            written.clone(),
            crate::commands::init::current_timestamp_utc(),
            meta.dependencies.clone(),
            metadata,
        );
        applied.push(AppliedReport {
            package: label,
            files_written: written_count,
            paths: written
                .into_iter()
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .collect(),
        });
    }

    // 6. Record top-level lockfile metadata: language chain + active
    //    features. Language chain only when explicitly resolved (not
    //    just default `en`).
    if !language_chain.is_empty() && language_chain != ["en"] {
        lockfile.meta.language_chain = language_chain.clone();
    }
    let mut active_features_global: BTreeSet<String> = BTreeSet::new();
    for (plan, fe) in plans.iter().zip(feature_state.iter()) {
        let pkg_label = format!("{}:{}", plan.cached.resolved.kind, plan.cached.resolved.name);
        for f in &fe.active_features {
            active_features_global.insert(format!("{pkg_label}/{f}"));
        }
    }
    lockfile.meta.active_features = active_features_global.into_iter().collect();

    // 7. Update `vibe.toml` `[requires].packages` with the CLI-supplied
    //    roots. Default constraint shape mirrors Cargo / npm / Poetry:
    //    when the CLI form had no version (`flow:wal`), we resolve to
    //    a concrete version and write the caret form (`flow:wal@^0.1.0`).
    //    When the CLI form had an explicit constraint (`@^0.1`,
    //    `@=0.2.0`, etc.), we preserve it verbatim — the user already
    //    declared their intent. `--exact` overrides both: it always
    //    pins to `=<resolved>` regardless of CLI form.
    //
    //    De-dup by `(kind, name)`; a repeat install with a new
    //    constraint replaces the old entry. No-op when there were no
    //    CLI args (install-from-manifest mode) — the manifest is
    //    already authoritative in that case.
    let finalized_cli_roots: Vec<PackageRef> = cli_roots
        .iter()
        .map(|cli_pkgref| {
            let resolved = plans
                .iter()
                .find(|p| {
                    p.cached.resolved.kind == cli_pkgref.kind
                        && p.cached.resolved.name == cli_pkgref.name
                })
                .map(|p| &p.cached.resolved.version)
                .expect(
                    "every CLI root has a corresponding plan after resolve+plan; \
                     resolver invariant",
                );
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

    // 8. Mirror the manifest's declared roots into `meta.root_dependencies`
    //    so the lockfile remains a self-contained snapshot. PROP-002
    //    §2.7: `root_dependencies` mirrors `vibe.toml`
    //    `[requires].packages`; in CLI mode we feed the finalized
    //    pkgrefs (with caret / `--exact` applied) so the two files
    //    agree byte-for-byte on the constraint shape.
    let lock_roots: &[PackageRef] = if cli_roots.is_empty() {
        &roots
    } else {
        &finalized_cli_roots
    };
    merge_root_dependencies(&mut lockfile, lock_roots);

    // Save lockfile on disk.
    lockfile.write(project_root.join(Lockfile::FILENAME))?;

    emit_report(ctx, &applied, &project_root)?;
    Ok(())
}

#[derive(Debug, Serialize)]
struct AppliedReport {
    package: String,
    files_written: usize,
    paths: Vec<String>,
}

/// Per-node install metadata threaded from the solver into the lockfile
/// register call.
struct NodeInstallMeta {
    dependencies: Vec<PackageRef>,
    is_root: bool,
}

/// Build a `kind:name@=<exact-version>` pkgref for fetching the version
/// the solver chose, regardless of how the user originally constrained
/// the package.
fn exact_pinned_pkgref(node: &ResolvedNode) -> PackageRef {
    let req = semver::VersionReq::parse(&format!("={}", node.version))
        .expect("exact version always parses as VersionReq");
    PackageRef {
        kind: node.kind,
        name: node.name.clone(),
        version: VersionSpec::Req(req),
    }
}

/// Merge new root pkgrefs into `lockfile.meta.root_dependencies`,
/// deduplicating on `(kind, name)` (idempotent re-installs don't grow
/// the list). Existing entries for the same `(kind, name)` are
/// overwritten by the new pkgref so a constraint change in
/// `vibe install` updates the recorded root constraint.
fn merge_root_dependencies(lockfile: &mut Lockfile, roots: &[PackageRef]) {
    for r in roots {
        let pos = lockfile
            .meta
            .root_dependencies
            .iter()
            .position(|existing| existing.kind == r.kind && existing.name == r.name);
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
/// registry-resolved would create a `(kind, name)` duplicate that
/// `try_from = "RequiresWire"` rejects on the next parse.
fn merge_manifest_requires(
    manifest: &mut Manifest,
    roots: &[PackageRef],
) -> bool {
    let mut changed = false;
    for r in roots {
        if manifest
            .requires
            .git_packages
            .iter()
            .any(|g| g.kind == r.kind && g.name == r.name)
        {
            // Already declared as git-source — leave untouched.
            continue;
        }
        let pos = manifest
            .requires
            .packages
            .iter()
            .position(|existing| existing.kind == r.kind && existing.name == r.name);
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
    Multi(MultiRegistryResolver),
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
        match self {
            InstallResolver::Local(r) => {
                let provider = LocalRegistryProvider::new(r);
                NaiveDepSolver::new(provider).solve(roots)
            }
            InstallResolver::Multi(m) => {
                let provider = MultiRegistryProvider::new(m);
                NaiveDepSolver::new(provider).solve(roots)
            }
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
        bail!("--exact has no meaning with --git (constraint shape is registry-resolved); drop one of the two flags");
    }
    if args.registry.is_some() {
        bail!("--git bypasses the registry layer; drop --registry or drop --git");
    }
    if args.packages.len() != 1 {
        bail!("--git requires exactly one positional pkgref `<kind>:<name>`; got {}", args.packages.len());
    }
    // Allow user to type either `flow:internal` or `flow:internal@*` —
    // version is irrelevant for git-source (the ref decides), but we
    // accept both shapes for muscle-memory compatibility.
    let pr = PackageRef::parse(&args.packages[0])
        .with_context(|| format!("parsing `{}`", args.packages[0]))?;
    let url = args.git.clone().expect("caller checked args.git.is_some()");
    let ref_kind = match (args.tag.as_deref(), args.branch.as_deref(), args.rev.as_deref()) {
        (Some(t), None, None) => GitRefKind::Tag(t.to_string()),
        (None, Some(b), None) => GitRefKind::Branch(b.to_string()),
        (None, None, Some(r)) => GitRefKind::Rev(r.to_string()),
        (None, None, None) => bail!(
            "--git requires exactly one of --tag, --branch, or --rev"
        ),
        _ => bail!(
            "--git accepts exactly one of --tag, --branch, --rev — drop the extras"
        ),
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
        name: pr.name.clone(),
        url,
        ref_kind,
        version: None,
        auth,
        token_env: args.git_token_env.clone(),
    };

    // Drop any prior registry-resolved entry for the same pkgref —
    // M1.15 forbids `(kind, name)` collision between
    // `requires.packages` and `requires.git_packages`.
    manifest.requires.packages.retain(|p| !(p.kind == dep.kind && p.name == dep.name));
    // Replace any prior git-source entry for the same pkgref (same
    // shape as updating an existing constraint).
    manifest
        .requires
        .git_packages
        .retain(|g| !(g.kind == dep.kind && g.name == dep.name));
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
        let local = LocalRegistry::new(p.clone())
            .map_err(|e| anyhow!("failed to open registry at `{}`: {e}", p.display()))?;
        return Ok(InstallResolver::Local(local));
    }

    if manifest.registries.is_empty() {
        bail!(
            "no registry configured. Pass `--registry <path>` or add a `[[registry]]` entry to `vibe.toml`."
        );
    }

    let mrr = MultiRegistryResolver::open(
        &manifest.registries,
        &manifest.mirrors,
        &manifest.overrides,
    )
    .context("opening multi-registry resolver")?
    .with_strict_auth(args.auth_required)
    .with_git_packages(manifest.requires.git_packages.clone());
    Ok(InstallResolver::Multi(mrr))
}

fn check_cross_plan_conflicts(prior: &[InstallPlan], new: &InstallPlan) -> Result<()> {
    use std::collections::HashSet;
    let prior_targets: HashSet<&PathBuf> =
        prior.iter().flat_map(|p| p.writes.iter().map(|w| &w.target_rel)).collect();
    for w in &new.writes {
        if prior_targets.contains(&w.target_rel) {
            bail!(
                "two packages in this install would write to the same path `{}`",
                w.target_rel.display()
            );
        }
    }
    let prior_snippets: HashSet<&str> = prior
        .iter()
        .filter_map(|p| p.boot_snippet_filename.as_deref())
        .collect();
    if let Some(snippet) = new.boot_snippet_filename.as_deref()
        && prior_snippets.contains(snippet)
    {
        bail!("two packages in this install share boot snippet filename `{snippet}`");
    }
    Ok(())
}

fn total_writes(plans: &[InstallPlan]) -> usize {
    plans.iter().map(|p| p.writes.len()).sum()
}

fn present_plans(ctx: &output::Context, project_root: &Path, plans: &[InstallPlan]) {
    if ctx.is_json() {
        #[derive(Serialize)]
        struct JsonPlanEntry<'a> {
            package: String,
            version: String,
            source_url: &'a str,
            content_hash: &'a str,
            writes: Vec<String>,
            boot_snippet: Option<&'a str>,
        }
        let payload: Vec<JsonPlanEntry<'_>> = plans
            .iter()
            .map(|p| JsonPlanEntry {
                package: format!(
                    "{}:{}",
                    p.cached.resolved.kind, p.cached.resolved.name
                ),
                version: p.cached.resolved.version.to_string(),
                source_url: p.cached.source_uri.as_str(),
                content_hash: p.cached.content_hash.as_str(),
                writes: p
                    .writes
                    .iter()
                    .map(|w| w.target_rel.to_string_lossy().to_string())
                    .collect(),
                boot_snippet: p.boot_snippet_filename.as_deref(),
            })
            .collect();
        let envelope = serde_json::json!({
            "command": "install:plan",
            "plans": payload,
        });
        let _ = ctx.emit_json(&envelope);
        return;
    }
    if ctx.is_quiet() {
        return;
    }
    for plan in plans {
        ctx.heading(&format!("\nPlan for {}", plan.package_label()));
        for w in &plan.writes {
            let prefix = match &w.kind {
                WriteKind::Regular => "create".to_string(),
                WriteKind::BootSnippet => "boot  ".to_string(),
                WriteKind::SubskillContent { subskill_path } => {
                    format!("sub:{subskill_path}")
                }
                WriteKind::SubskillBootSnippet { subskill_path } => {
                    format!("sub-boot:{subskill_path}")
                }
            };
            let rel = w
                .target_abs
                .strip_prefix(project_root)
                .unwrap_or(&w.target_abs);
            let rel_s = rel.to_string_lossy().replace('\\', "/");
            println!("  {prefix}  {}", rel_s);
        }
        if !plan.active_subskills.is_empty() {
            ctx.step(&format!(
                "  ↳ {} subskill{} active: {}",
                plan.active_subskills.len(),
                if plan.active_subskills.len() == 1 {
                    ""
                } else {
                    "s"
                },
                plan.active_subskills
                    .iter()
                    .map(|s| s.path.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            ));
        }
    }
    println!();
}

fn emit_report(
    ctx: &output::Context,
    applied: &[AppliedReport],
    project_root: &Path,
) -> Result<()> {
    if ctx.is_json() {
        let payload = serde_json::json!({
            "ok": true,
            "command": "install",
            "project": project_root.display().to_string(),
            "installed": applied,
        });
        ctx.emit_json(&payload)?;
        return Ok(());
    }
    let total_files: usize = applied.iter().map(|a| a.files_written).sum();
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe install: {} package{}, {total_files} file{} written",
            applied.len(),
            if applied.len() == 1 { "" } else { "s" },
            if total_files == 1 { "" } else { "s" },
        ));
        return Ok(());
    }
    for a in applied {
        for p in &a.paths {
            ctx.created(p);
        }
    }
    ctx.summary(&format!(
        "\nInstalled {} package{} ({} file{} written).",
        applied.len(),
        if applied.len() == 1 { "" } else { "s" },
        total_files,
        if total_files == 1 { "" } else { "s" },
    ));
    Ok(())
}

/// Build the resolved language chain from a CLI flag override + the
/// project's `[i18n]` declaration. The CLI flag is the head of the
/// chain; project-level preference and fallback come next; canonical /
/// registry-default `en` close the chain.
fn build_language_chain(
    cli_language: Option<&str>,
    manifest: &Manifest,
) -> Vec<String> {
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
) -> ActivationContext
where
    I: IntoIterator<Item = &'a CachedPackage>,
{
    let mut ctx = ActivationContext {
        project_root: Some(project_root.to_path_buf()),
        language_chain: language_chain.to_vec(),
        ..Default::default()
    };
    for c in cached {
        ctx.add_present(format!("{}:{}", c.resolved.kind, c.resolved.name));
        for cap in &c.manifest.provides.capabilities {
            let qualified = cap.qualified();
            ctx.add_present(qualified.clone());
            if qualified.starts_with("interface:") {
                ctx.add_provides(qualified);
            }
        }
        if let Some(purl) = &c.package_meta().describes {
            ctx.describes_types.insert(purl.purl_type.clone());
        }
    }
    ctx
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
        assert!(!changed_again, "second merge of the same pkgref must be a no-op");
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
