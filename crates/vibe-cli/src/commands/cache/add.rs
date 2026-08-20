//! `vibe cache add <pkgref>…` — deliberate pre-warming (PROP-010
//! §2.8 CMD-ADD): resolve the named packages and their dependency
//! closure, then fetch every node into the machine store. Nothing is
//! materialised into any project — no `vibe.lock`, no `vibedeps/`,
//! `vibe.toml` untouched — because this command never enters the
//! install transaction: it stops at the fetch, which is exactly the
//! step that already fills the store for `vibe install`.
//!
//! Source selection (§2.4 PROJECTLESS-SOURCE): inside a project the
//! project's registries serve (the existing `build_install_resolver`
//! path, manifest and all); outside one, the user-level
//! `~/.vibe/registry.toml` registries do — the same
//! `MultiRegistryResolver` the install walk uses, opened from the
//! global config's sections alone, which is everything the resolver
//! minimally needs from a manifest that does not exist (no
//! `[requires]`, no git-source declarations outside a project).

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::user_config::UserConfig;
use vibe_core::{EffectiveRegistryConfig, GlobalRegistryConfig, PackageRef};
use vibe_install::InstallSource;
use vibe_registry::MultiRegistryResolver;

use crate::cli::{CacheAddArgs, InstallArgs};
use crate::commands::install::{InstallResolver, build_install_resolver, exact_pinned_pkgref};
use crate::commands::short_name;
use crate::output;

pub(crate) fn run(ctx: &output::Context, args: CacheAddArgs, root_offline: bool) -> Result<()> {
    // The offline ladder resolved once, exactly like `vibe install`
    // (PROP-010 §2.5): root `--offline` > `VIBE_OFFLINE` > user-config
    // `[net].offline`.
    let user_config = UserConfig::load().context("loading the user config")?;
    let offline = output::resolve_offline(root_offline, user_config.net.offline);

    let cwd = args
        .path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", args.path.display()))?;
    let cwd = crate::commands::init::strip_unc_public(cwd);
    let in_project = cwd.join(Manifest::FILENAME).exists();

    let global =
        GlobalRegistryConfig::load().map_err(|e| anyhow!("loading ~/.vibe/registry.toml: {e}"))?;
    let resolver = if in_project {
        build_install_resolver(
            &stub_install_args(cwd.clone()),
            &Manifest::read(cwd.join(Manifest::FILENAME))?,
            None,
            &cwd,
            &global,
            offline,
        )
        .context("building the project's registry resolver")?
    } else {
        projectless_resolver(&global, offline)?
    };

    // Parse the CLI pkgrefs and qualify short names at the input
    // boundary (PROP-008 §2.6) — same seam `vibe install` uses, with
    // an empty lockfile (a pre-warm has no project lock to consult).
    let empty_lock = Lockfile::empty(
        "vibe (cache add)",
        crate::commands::init::current_timestamp_utc(),
    );
    let roots: Vec<PackageRef> = args
        .packages
        .iter()
        .map(|raw| PackageRef::parse(raw).with_context(|| format!("parsing `{raw}`")))
        .collect::<Result<_>>()?;
    let roots: Vec<PackageRef> = roots
        .iter()
        .map(|r| short_name::qualify(&resolver, r, &empty_lock))
        .collect::<Result<_>>()?;

    // The closure walk is the existing solve — it already follows each
    // package's `[requires]`; no bespoke traversal here.
    let graph = resolver
        .solve(&roots)
        .map_err(|e| anyhow!("resolving the dependency closure: {e}"))?;
    let store_root = vibe_registry::store_root().context("resolving the machine store root")?;

    let mut inserted: Vec<String> = Vec::new();
    let mut already: Vec<String> = Vec::new();
    for node in graph.iter() {
        let name = &node.name;
        let version = &node.version;
        let label = format!("{}/{name}@{version}", node.group.as_str());
        // Write-once makes the presence check the honest discriminator:
        // a node already in the store is fetched (idempotently,
        // returning the existing entry) and its bytes stay untouched.
        let was_present = vibe_registry::lookup(&node.group, &node.name, &node.version).is_some();
        resolver
            .resolve_and_fetch(&exact_pinned_pkgref(node), &store_root, None)
            .with_context(|| format!("fetching {label} into the machine store"))?;
        if was_present {
            already.push(label);
        } else {
            inserted.push(label);
        }
    }

    emit(ctx, &store_root, in_project, &inserted, &already)
}

/// The user-level resolver for a projectless pre-warm (§2.4): the
/// global config's sections are everything `MultiRegistryResolver`
/// needs — a project contributes registries/mirrors/overrides through
/// `merge_effective`, and with no project there is nothing to merge,
/// so the global sections stand alone.
fn projectless_resolver(global: &GlobalRegistryConfig, offline: bool) -> Result<InstallResolver> {
    let mut eff = EffectiveRegistryConfig {
        registries: global.registries.clone(),
        mirrors: global.mirrors.clone(),
        overrides: global.overrides.clone(),
    };
    if offline {
        eff = eff.local_only();
    }
    if eff.registries.is_empty() {
        bail!(
            "no registry configured for a projectless `vibe cache add` — add a user-level \
             registry to `~/.vibe/registry.toml` (`[[registry]]` with a `url`, e.g. a \
             `file://` directory registry) or run inside a project whose `vibe.toml` \
             declares its own `[[registry]]`."
        );
    }
    let multi = MultiRegistryResolver::open(&eff.registries, &eff.mirrors, &eff.overrides)
        .context("opening the multi-registry resolver")?;
    Ok(InstallResolver::Multi(Box::new(multi), None))
}

/// The `InstallArgs` a pre-warm hands `build_install_resolver`: every
/// install-only flag at its inert default — a cache add records
/// nothing into the manifest, so `--git` / feature / pinning flags
/// have no meaning here. What survives is what shapes resolution
/// itself: the project path and the embedded/local-registry defaults
/// `vibe install` would use.
fn stub_install_args(path: PathBuf) -> InstallArgs {
    InstallArgs {
        packages: Vec::new(),
        path,
        registry: None,
        assume_yes: true,
        language: None,
        features: Vec::new(),
        no_default_features: false,
        all_features: false,
        exact: false,
        auth_required: false,
        solver: None,
        prefer_embedded: false,
        no_prefer_embedded: false,
        no_default_registry: false,
        offline: false,
        embedded_short_circuit: false,
        prefer_local: false,
        no_prefer_local: false,
        git: None,
        tag: None,
        branch: None,
        rev: None,
        git_auth: None,
        git_token_env: None,
        allow_hooks: false,
    }
}

fn emit(
    ctx: &output::Context,
    store_root: &Path,
    in_project: bool,
    inserted: &[String],
    already: &[String],
) -> Result<()> {
    if ctx.is_json() {
        ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "cache:add",
            "store": store_root.display().to_string(),
            "source": if in_project { "project" } else { "user" },
            "inserted": inserted,
            "already_present": already,
            "count": inserted.len() + already.len(),
        }))?;
        return Ok(());
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe cache add: {} fetched, {} already present",
            inserted.len(),
            already.len()
        ));
        return Ok(());
    }
    ctx.heading(&format!(
        "Pre-warming the machine store ({})",
        store_root.display()
    ));
    for label in inserted {
        ctx.created(label);
    }
    for label in already {
        ctx.skipped(label, "already present — bytes untouched (write-once)");
    }
    ctx.summary(&format!(
        "\n{} fetched, {} already present — nothing materialised into any project.",
        inserted.len(),
        already.len()
    ));
    Ok(())
}
