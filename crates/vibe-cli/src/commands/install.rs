//! `vibe install <kind>:<name>[@version] …` — plan → confirm → apply.
//!
//! Spec: `VIBEVM-SPEC.md` §5.6, §9.1, §11.1.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use dialoguer::Confirm;
use serde::Serialize;
use vibe_core::{PackageRef, VersionSpec};
use vibe_core::manifest::{Lockfile, ProjectManifest};
use vibe_install::{
    InstallError, InstallOptions, InstallPlan, WriteKind, apply_install,
    plan_install_with_options, register_installed,
};
use vibe_registry::{CachedPackage, LocalRegistry, MultiRegistryResolver};
use vibe_resolver::{
    DepSolver, LocalRegistryProvider, MultiRegistryProvider, NaiveDepSolver, ResolvedNode,
};

use crate::cli::InstallArgs;
use crate::output;

pub fn run(ctx: &output::Context, args: InstallArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let manifest = load_project_manifest(&project_root)?;
    let mut lockfile = load_or_empty_lockfile(&project_root)?;
    let resolver = build_install_resolver(&args, &manifest)?;

    // PROP-003 §2.7 — language preference resolution. Order:
    // 1. CLI flag `--language` is the head of the chain.
    // 2. The project manifest's `[i18n]` block contributes its
    //    `preferred` / `available` / `fallback` entries.
    // 3. Canonical (`I18nDecl::canonical`, default `en`) closes the
    //    chain so step 3 of PROP-003 §2.7.2's fallback ladder is
    //    always reachable.
    let install_options = build_install_options(args.language.as_deref(), &manifest);

    // Cache layout matches §8.3: `.vibe/cache/<kind>/<name>/<version>/`.
    let cache_root = project_root.join(".vibe/cache");
    fs::create_dir_all(&cache_root)
        .with_context(|| format!("creating cache dir `{}`", cache_root.display()))?;

    // 1. Parse every CLI pkgref into the typed root list.
    let roots: Vec<PackageRef> = args
        .packages
        .iter()
        .map(|raw| {
            PackageRef::parse(raw).with_context(|| format!("parsing `{raw}`"))
        })
        .collect::<Result<_>>()?;

    // 2. Run the depsolver — this expands transitive deps into the full
    //    graph the install pipeline materialises below. For the
    //    all-empty-deps case (today's three demo flows), the graph
    //    equals the input roots, no behaviour change.
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

    // 3. For each node in graph order (roots first, then transitives in
    //    deterministic order), pin to exact version, fetch, plan.
    let mut plans: Vec<InstallPlan> = Vec::new();
    let mut node_meta: Vec<NodeInstallMeta> = Vec::new();

    for node in graph.iter() {
        let pkgref = exact_pinned_pkgref(node);
        // Pass the lockfile pin (if any) through to the resolver so a
        // mirror-aware fetch can fall through past a source whose
        // content_hash disagrees with the pin. Without a pin (first
        // install for this `(kind, name)`) the registry layer accepts
        // the first reachable source's content; vibe-install's
        // `plan_install` is still the authoritative integrity gate.
        let expected = lockfile
            .find(node.kind, &node.name)
            .map(|p| p.content_hash.clone());
        let cached =
            resolver.resolve_and_fetch(&pkgref, &cache_root, expected.as_deref())?;
        let plan = plan_install_with_options(
            &project_root,
            &lockfile,
            cached,
            &install_options,
        )?;
        check_cross_plan_conflicts(&plans, &plan)?;
        plans.push(plan);
        node_meta.push(NodeInstallMeta {
            dependencies: node.dependencies.clone(),
            is_root: node.is_root,
        });
    }

    // Show combined plan.
    present_plans(ctx, &project_root, &plans);

    // Confirm (unless --assume-yes or --json or not a TTY).
    let approved = if args.assume_yes || ctx.is_json() {
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
    for (plan, meta) in plans.iter().zip(node_meta.iter()) {
        let label = plan.package_label();
        ctx.step(&format!(
            "Installing {label}{}",
            if meta.is_root { "" } else { " (transitive)" }
        ));
        let written = apply_install(plan)?;
        let written_count = written.len();
        register_installed(
            &mut lockfile,
            plan,
            written.clone(),
            crate::commands::init::current_timestamp_utc(),
            meta.dependencies.clone(),
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

    // Update meta.root_dependencies — what the user directly asked for,
    // distinct from transitives the solver pulled in. Lockfile uses this
    // to drive `vibe uninstall` semantics.
    merge_root_dependencies(&mut lockfile, &roots);

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

fn resolve_project_root(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = super::init::strip_unc_public(canonical);
    if !stripped.join(ProjectManifest::FILENAME).exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            stripped.display()
        );
    }
    Ok(stripped)
}

fn load_project_manifest(root: &Path) -> Result<ProjectManifest> {
    let path = root.join(ProjectManifest::FILENAME);
    Ok(ProjectManifest::read(&path)?)
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
    manifest: &ProjectManifest,
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
    .context("opening multi-registry resolver")?;
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
            let prefix = match w.kind {
                WriteKind::Regular => "create",
                WriteKind::BootSnippet => "boot  ",
            };
            let rel = w
                .target_abs
                .strip_prefix(project_root)
                .unwrap_or(&w.target_abs);
            let rel_s = rel.to_string_lossy().replace('\\', "/");
            println!("  {prefix}  {}", rel_s);
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

/// Build [`InstallOptions`] from a CLI flag override + the project's
/// `[i18n]` declaration. The CLI flag is the head of the chain;
/// project-level preference and fallback come next; canonical /
/// registry-default `en` close the chain.
fn build_install_options(
    cli_language: Option<&str>,
    manifest: &ProjectManifest,
) -> InstallOptions {
    let mut effective = manifest.i18n.clone();
    if let Some(lang) = cli_language {
        effective.preferred = Some(lang.to_string());
    }
    let chain = if effective.is_default() && cli_language.is_none() {
        Vec::new()
    } else {
        effective.project_preference_chain()
    };
    InstallOptions {
        language_chain: chain,
    }
}
