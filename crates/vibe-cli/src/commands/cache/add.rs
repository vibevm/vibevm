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

use std::collections::BTreeSet;
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
    let (resolver, in_project) = cache_resolver(&args.path, root_offline)?;

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

    let store_root = vibe_registry::store_root().context("resolving the machine store root")?;
    let warmed = warm(&resolver, &store_root, roots)?;

    emit(
        ctx,
        &store_root,
        in_project,
        &warmed.inserted,
        &warmed.already,
        &warmed.unreachable,
    )
}

/// What one warm-up put into the store.
#[derive(Debug, Clone, Default)]
pub(crate) struct Warmed {
    /// Coordinates the store did not hold before.
    pub inserted: Vec<String>,
    /// Coordinates it already held, fetched idempotently.
    pub already: Vec<String>,
    /// Documented subjects the warm-up could not bring along, each with
    /// the reason — a `(coordinate, why)` pair, never a bare list.
    pub unreachable: Vec<(String, String)>,
}

/// Warm `roots` and everything they need into the machine store.
///
/// Separate from [`run`] because it has a second caller: the registry
/// builder warms a published version before it renders it, and it must
/// warm it the SAME way — `##REL-WARMUP-CLOSURE` is a property of the
/// warm-up and not of the command, and two implementations of it would
/// mean a manual whose citations resolve on one path and not on the
/// other.
pub(crate) fn warm(
    resolver: &InstallResolver,
    store_root: &Path,
    roots: Vec<PackageRef>,
) -> Result<Warmed> {
    let mut warmed = Warmed::default();
    let mut fetched: BTreeSet<String> = BTreeSet::new();
    let Warmed {
        inserted,
        already,
        unreachable,
    } = &mut warmed;

    // The walk runs to a fixed point rather than once, because the
    // documentation closure grows a level at a time: warming a manual
    // pulls its subjects, and warming a translation pulls the source
    // documentation, which pulls the source's own subjects. Each round
    // solves and fetches, then asks what the newly warmed documentation
    // itself asks for; a round that adds no coordinate is the last.
    let mut pending = roots;
    let mut derived = false;
    while !pending.is_empty() {
        // The closure walk within one round is the existing solve — it
        // already follows each package's `[requires]`; no bespoke
        // traversal here.
        //
        // A ROOT the user named must resolve, so the first round solves
        // them together and a failure is the command's failure. A
        // DERIVED coordinate is solved on its own and a failure is
        // recorded, not raised: the subject of a documentation may be a
        // project coordinate no registry can hand back — the host
        // itself is one (`##REL-HOST-SUBJECT`) — and refusing there
        // would make a project's own manual unwarmable.
        let nodes = if derived {
            let mut nodes = Vec::new();
            for reference in &pending {
                match resolver.solve(std::slice::from_ref(reference)) {
                    Ok(graph) => nodes.extend(graph.iter().cloned()),
                    Err(e) => unreachable.push((reference.to_string(), e.to_string())),
                }
            }
            nodes
        } else {
            resolver
                .solve(&pending)
                .map_err(|e| anyhow!("resolving the dependency closure: {e}"))?
                .iter()
                .cloned()
                .collect()
        };
        derived = true;
        let mut warmed_now: Vec<(vibe_core::Group, String, semver::Version)> = Vec::new();
        for node in &nodes {
            let name = &node.name;
            let version = &node.version;
            let label = format!("{}/{name}@{version}", node.group.as_str());
            if !fetched.insert(label.clone()) {
                continue;
            }
            // Write-once makes the presence check the honest
            // discriminator: a node already in the store is fetched
            // (idempotently, returning the existing entry) and its
            // bytes stay untouched.
            let was_present =
                vibe_registry::lookup(&node.group, &node.name, &node.version).is_some();
            resolver
                .resolve_and_fetch(&exact_pinned_pkgref(node), store_root, None)
                .with_context(|| format!("fetching {label} into the machine store"))?;
            if was_present {
                already.push(label);
            } else {
                inserted.push(label);
            }
            warmed_now.push((node.group.clone(), node.name.clone(), node.version.clone()));
        }
        pending = documentation_closure(store_root, &warmed_now, unreachable)?;
    }
    Ok(warmed)
}

/// What the documentation just warmed asks for in turn: the subjects of
/// every `doc` package's `[[documents]]` and the source of its
/// `[translates]`, each at its declared constraint
/// (PROP-057 `##REL-WARMUP-CLOSURE`).
///
/// This is the whole reason a documentation warm-up is not just a
/// fetch: a page cites its subject by `spec://`, and a citation that
/// cannot be opened offline is a dead link in the one mode where there
/// is nowhere else to look.
///
/// A coordinate whose form no registry could serve is recorded in
/// `unreachable` rather than raised: the subject of a documentation may
/// be a PROJECT coordinate — the host itself is one
/// (`##REL-HOST-SUBJECT`) — and refusing the warm-up over it would make
/// the manual of a project unwarmable.
fn documentation_closure(
    store_root: &Path,
    warmed: &[(vibe_core::Group, String, semver::Version)],
    unreachable: &mut Vec<(String, String)>,
) -> Result<Vec<PackageRef>> {
    let mut next: Vec<PackageRef> = Vec::new();
    for (group, name, version) in warmed {
        let entry = vibe_registry::store::entry_dir(store_root, group, name, version);
        let Ok(manifest) = Manifest::read(entry.join(Manifest::FILENAME)) else {
            continue;
        };
        if manifest
            .package
            .as_ref()
            .is_none_or(|p| p.kind != vibe_core::PackageKind::Doc)
        {
            continue;
        }
        let edges = manifest
            .documents
            .iter()
            .map(|d| (d.package.as_str(), d.version.as_str()))
            .chain(
                manifest
                    .translates
                    .iter()
                    .map(|t| (t.package.as_str(), t.version.as_str())),
            );
        for (coordinate, constraint) in edges {
            match warm_ref(coordinate, constraint) {
                Some(reference) => next.push(reference),
                None => unreachable.push((
                    format!("{coordinate}@{constraint}"),
                    "not a `<group>/<name>` coordinate a registry could serve".to_string(),
                )),
            }
        }
    }
    Ok(next)
}

/// A `<group>/<name>` coordinate plus its constraint as a pkgref, or
/// `None` when the coordinate is not one a registry could serve.
fn warm_ref(coordinate: &str, constraint: &str) -> Option<PackageRef> {
    let (group, name) = coordinate.split_once('/')?;
    let group = vibe_core::Group::parse(group).ok()?;
    let version = vibe_core::VersionSpec::parse(constraint).ok()?;
    PackageRef::new(None, Some(group), name, version).ok()
}

/// The registry resolver every cache-family command that needs to
/// FETCH resolves through — shared by `add` (the pre-warm) and
/// `check --repair` (the re-fetch rung), one construction site:
/// inside a project, the project's own `[[registry]]` walk (the
/// existing `build_install_resolver` path, manifest and all); outside
/// one, the user-level `~/.vibe/registry.toml` registries (§2.4
/// PROJECTLESS-SOURCE). Returns the resolver and whether a project
/// was found (callers report the source they used).
///
/// The offline ladder is resolved here once, exactly like `vibe
/// install` (PROP-010 §2.5): root `--offline` > `VIBE_OFFLINE` >
/// user-config `[net].offline`.
pub(crate) fn cache_resolver(path: &Path, root_offline: bool) -> Result<(InstallResolver, bool)> {
    let user_config = UserConfig::load().context("loading the user config")?;
    let offline = output::resolve_offline(root_offline, user_config.net.offline);

    let cwd = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let cwd = crate::commands::init::strip_unc_public(cwd);
    let in_project = cwd.join(Manifest::FILENAME).exists();

    let global =
        GlobalRegistryConfig::load().map_err(|e| anyhow!("loading ~/.vibe/registry.toml: {e}"))?;
    let resolver = if in_project {
        // No lock entries ride along (§2.8: a pre-warm fetches from the
        // real sources — the availability fallback must NOT quietly
        // serve an already-locked version instead of verifying the
        // registry still provides it).
        build_install_resolver(
            &stub_install_args(cwd.clone()),
            &Manifest::read(cwd.join(Manifest::FILENAME))?,
            None,
            &cwd,
            &global,
            offline,
            &[],
        )
        .context("building the project's registry resolver")?
    } else {
        projectless_resolver(&global, offline)?
    };
    Ok((resolver, in_project))
}

/// The user-level resolver for a projectless fetch (§2.4): the global
/// config's sections are everything `MultiRegistryResolver` needs — a
/// project contributes registries/mirrors/overrides through
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
            "no registry configured for a projectless registry fetch (`vibe cache add` / \
             `vibe cache check --repair`) — add a user-level registry to \
             `~/.vibe/registry.toml` (`[[registry]]` with a `url`, e.g. a `file://` \
             directory registry) or run inside a project whose `vibe.toml` declares \
             its own `[[registry]]`."
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
        global: false,
        from_source: false,
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
        force: false,
        trace_compile: false,
    }
}

fn emit(
    ctx: &output::Context,
    store_root: &Path,
    in_project: bool,
    inserted: &[String],
    already: &[String],
    unreachable: &[(String, String)],
) -> Result<()> {
    if ctx.is_json() {
        ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "cache:add",
            "store": store_root.display().to_string(),
            "source": if in_project { "project" } else { "user" },
            "inserted": inserted,
            "already_present": already,
            "unwarmed_subjects": unreachable
                .iter()
                .map(|(coordinate, reason)| serde_json::json!({
                    "package": coordinate,
                    "reason": reason,
                }))
                .collect::<Vec<_>>(),
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
    // A subject no registry could serve is said out loud: the warm-up
    // succeeded, but one `spec://` citation in that documentation will
    // not open offline, and silence would let the reader discover that
    // on a plane.
    for (coordinate, reason) in unreachable {
        ctx.step(&format!(
            "documented subject `{coordinate}` not warmed — {reason}"
        ));
    }
    ctx.summary(&format!(
        "\n{} fetched, {} already present — nothing materialised into any project.",
        inserted.len(),
        already.len()
    ));
    Ok(())
}
