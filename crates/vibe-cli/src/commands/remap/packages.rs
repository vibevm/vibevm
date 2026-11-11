//! Installing a remap package into a home, and taking one out again
//! (`PROP-066#package`, `PROP-066#uninstall`).
//!
//! A remap package is resolved and fetched exactly as any other package
//! — the registry walk, the machine store, the content hash — and then it
//! goes somewhere no other package goes: `<home>/remap/packages/<key>`,
//! with its offer recorded in the home's index. It is never materialised
//! into a project's dependency root, because its deliverable is policy and
//! policy belongs to the user (`PROP-066#decisions`).
//!
//! Installing records an **offer**. Nothing applies until the acceptance
//! ledger says the user agreed, which is `consent.rs`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#package");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::remap::{InstalledRemap, OfferState, RemapPackage};
use vibe_core::{GlobalRegistryConfig, PackageKind, PackageRef};
use vibe_install::InstallSource;

use crate::cli::InstallArgs;
use crate::commands::install::{InstallResolver, build_install_resolver};
use crate::output;

use super::home::RemapHome;

/// What one resolved remap package install produced.
pub(crate) struct Installed {
    /// The offer now recorded in the home.
    pub(crate) offer: RemapPackage,
    /// The registry that served it.
    pub(crate) registry: Option<String>,
    pub(crate) source: vibe_core::manifest::LockedRemapSource,
}

/// The resolver a remap install uses: `--registry <dir>` when the operator
/// named one, else the project's walk inside a project and the
/// machine-global `~/.vibe/registry.toml` outside one — the same ladder
/// `vibe cache add` uses, because a remap package is fetched like any
/// other package and an operator with no project still has to be able to
/// install one.
fn resolver(args: &InstallArgs, offline: bool) -> Result<InstallResolver> {
    resolver_with_locked(args, offline, &[])
}
fn resolver_with_locked(
    args: &InstallArgs,
    offline: bool,
    locked: &[vibe_core::manifest::LockedPackage],
) -> Result<InstallResolver> {
    let root = args
        .path
        .canonicalize()
        .unwrap_or_else(|_| args.path.clone());
    let manifest = root
        .join(Manifest::FILENAME)
        .exists()
        .then(|| Manifest::read(root.join(Manifest::FILENAME)))
        .transpose()?
        .unwrap_or_default();
    crate::commands::install::build_install_resolver_deferred(
        args, &manifest, None, &root, offline, locked,
    )
}

/// A resolver over one `file://` directory — what `vibe remap import`
/// reads a bundle through, and the same `LocalRegistry` path
/// `--registry <dir>` takes.
pub(crate) fn local_resolver(dir: &Path) -> Result<InstallResolver> {
    let args = InstallArgs {
        registry: Some(dir.to_path_buf()),
        ..inert_args()
    };
    let global = GlobalRegistryConfig::default();
    build_install_resolver(&args, &Manifest::default(), None, dir, &global, true, &[])
}

/// The `InstallArgs` a remap-only path hands the shared resolver builder:
/// every install flag at its inert default, because none of them records
/// anything into a manifest here.
fn inert_args() -> InstallArgs {
    InstallArgs {
        source_policy: Default::default(),
        packages: Vec::new(),
        global: true,
        agent: None,
        server: None,
        platform: None,
        arch: None,
        installer: None,
        library: None,
        steps: Default::default(),
        no_extras: false,
        download_header_file: None,
        version_file: Default::default(),
        plan_only: false,
        hidden: false,
        path: PathBuf::from("."),
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

/// Would `vibe install -g <packages>` be installing remap packages?
///
/// The answer is read from the manifests the registry would serve, not
/// from the spelling of the pkgref, so `vibe install -g org.corp/intranet`
/// works without the operator having to know the kind. A probe that
/// cannot answer — no registry, an unresolvable coordinate, a malformed
/// manifest — answers `None`, so the existing application path produces
/// its own error exactly as before rather than this one guessing.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#package")]
pub(crate) fn claims(args: &InstallArgs, offline: bool) -> Option<Vec<PackageRef>> {
    if args.packages.is_empty() {
        return None;
    }
    let resolver = resolver(args, offline).ok()?;
    let mut refs = Vec::with_capacity(args.packages.len());
    for raw in &args.packages {
        let pkgref = PackageRef::parse(raw).ok()?;
        let manifest = resolver.manifest_of(&pkgref).ok()?;
        if manifest.package.as_ref().map(|meta| meta.kind) != Some(PackageKind::Remap) {
            return None;
        }
        refs.push(pkgref);
    }
    Some(refs)
}

/// Install one or more remap packages into the user home as offers.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#package")]
pub(crate) fn install(
    ctx: &output::Context,
    args: &InstallArgs,
    refs: &[PackageRef],
    offline: bool,
) -> Result<()> {
    let resolver = resolver(args, offline)?;
    let store_root = vibe_registry::store_root().context("resolving the machine store root")?;
    let mut home = RemapHome::open()?;
    let mut installed = Vec::new();
    {
        let _lock = home.lock()?;
        for pkgref in refs {
            installed.push(adopt(&resolver, &store_root, &mut home, pkgref, None)?);
        }
        home.save()?;
    }
    report(ctx, &home, &installed)
}

/// Fetch one remap package and record its offer in `home`.
///
/// Shared by `vibe install -g`, `vibe remap import` and the resolution of
/// a project's `[requires.remaps]`, so all three record the same offer in
/// the same place — the whole point of `PROP-066#layers-packages` being a
/// layer rather than a second mechanism.
pub(crate) fn adopt(
    resolver: &InstallResolver,
    store_root: &Path,
    home: &mut RemapHome,
    pkgref: &PackageRef,
    registry_hint: Option<&str>,
) -> Result<Installed> {
    let (cached, offer) = fetch_offer(resolver, store_root, pkgref)?;
    adopt_cached(home, &cached, offer, registry_hint)
}
fn fetch_offer(
    resolver: &InstallResolver,
    store_root: &Path,
    pkgref: &PackageRef,
) -> Result<(vibe_registry::CachedPackage, RemapPackage)> {
    let cached = resolver
        .resolve_and_fetch(pkgref, store_root, None)
        .with_context(|| format!("fetching the remap package `{}`", pkgref.qualified_name()))?;
    let meta = cached.package_meta();
    if meta.kind != PackageKind::Remap {
        bail!(
            "`{}` is a `{}` package, not a remap package — `vibe remap import` and the \
             remap install path carry rows, never content \
             (violates spec://org.vibevm.core/vibevm/common/PROP-066#package; \
              fix: install it the ordinary way, without the remap path)",
            pkgref.qualified_name(),
            meta.kind
        );
    }
    let offer = vibe_core::remap::offer_from_manifest(&cached.manifest, &cached.content_hash)
        .map_err(|reason| {
            anyhow!(
                "reading the offer of `{}`: {reason}",
                pkgref.qualified_name()
            )
        })?;
    // This is actual canonical acquisition state, not report-only source evidence.
    source_binding(&cached).locked_package()?;
    Ok((cached, offer))
}
fn source_binding(cached: &vibe_registry::CachedPackage) -> vibe_core::manifest::LockedRemap {
    use vibe_core::manifest::{LockedRemap, LockedRemapSource};
    let mut binding = LockedRemap {
        package: format!("{}/{}", cached.resolved.group, cached.resolved.name),
        version: cached.resolved.version.to_string(),
        content_hash: cached.content_hash.clone(),
        registry: cached.registry_name.clone(),
        source: Some(LockedRemapSource {
            source_url: vibe_core::SourceUrl::new(cached.source_uri.clone()),
            source_ref: cached.source_ref.clone(),
            resolved_commit: cached.resolved_commit.clone(),
            source_kind: cached.source_kind(),
            overridden: cached.overridden,
            via_redirect: cached.via_redirect.clone(),
        }),
    };
    if let Some(held) = &cached.locked_selection
        && let Some(row) = held.replayed_row()
    {
        binding.registry = row.registry.clone();
        binding.source = Some(LockedRemapSource {
            source_url: row.source_url.clone(),
            source_ref: row.source_ref.clone(),
            resolved_commit: row.resolved_commit.clone(),
            source_kind: row.source_kind,
            overridden: row.overridden,
            via_redirect: row.via_redirect.clone(),
        });
    }
    binding
}
fn adopt_cached(
    home: &mut RemapHome,
    cached: &vibe_registry::CachedPackage,
    offer: RemapPackage,
    registry_hint: Option<&str>,
) -> Result<Installed> {
    // The tree is copied into the home so `remap export` can hand the
    // package on and `remap list` can show the offer with no registry in
    // reach — a closed network is exactly where the registry is gone.
    let tree = vibe_core::remap::package_dir(home.root(), &offer.key);
    if tree.exists() {
        std::fs::remove_dir_all(&tree)
            .with_context(|| format!("replacing `{}`", tree.display()))?;
    }
    copy_tree(&cached.cache_dir, &tree)
        .with_context(|| format!("copying the remap package into `{}`", tree.display()))?;
    let registry = registry_hint
        .map(str::to_owned)
        .or_else(|| cached.registry_name.clone());
    home.install(InstalledRemap {
        package: offer.clone(),
        registry: registry.clone(),
        installed: crate::commands::init::current_timestamp_utc(),
    });
    let source = source_binding(cached)
        .source
        .ok_or_else(|| anyhow!("acquired remap has no source binding"))?;
    Ok(Installed {
        offer,
        registry,
        source,
    })
}

/// The home's copy of one installed package's tree.
pub(crate) fn tree_of(home: &RemapHome, key: &str) -> PathBuf {
    vibe_core::remap::package_dir(home.root(), key)
}

/// Copy a package tree, skipping the `.git` a clone may carry: the home's
/// copy is the shippable tree, the same thing the machine store holds.
fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == ".git" {
            continue;
        }
        let target = to.join(&name);
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Resolve and record every `[requires.remaps]` entry of `manifest` as a
/// project-layer offer, and pin each one in the lock
/// (`PROP-066#layers-packages`).
///
/// A project's requirement is an offer like any other: every clone of the
/// project sees the same list, and each machine still decides. The pin in
/// the lock is what makes "the same list" true across clones — the
/// acceptance is bound to a content hash, so a clone that resolved a
/// different version is asked again rather than inheriting a yes.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#layers-packages")]
pub(crate) fn adopt_project_requirements(
    args: &InstallArgs,
    manifest: &Manifest,
    lockfile: &mut Lockfile,
    offline: bool,
    workspace_requirements: &[PackageRef],
) -> Result<usize> {
    let retained = workspace_requirements
        .iter()
        .map(PackageRef::qualified_name)
        .collect::<std::collections::BTreeSet<_>>();
    if manifest.requires.remaps.is_empty() {
        lockfile
            .remaps
            .retain(|row| retained.contains(&row.package));
        return Ok(0);
    }
    // Missing legacy authority is diagnosed for the entire demanded remap set
    // before resolver construction, acquisition, or home mutation.
    let locked = manifest
        .requires
        .remaps
        .iter()
        .filter_map(|request| {
            lockfile
                .remaps
                .iter()
                .find(|row| row.package == request.qualified_name())
        })
        .map(vibe_core::manifest::LockedRemap::locked_package)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let resolver = resolver_with_locked(args, offline, &locked)?;
    let store_root = vibe_registry::store_root().context("resolving the machine store root")?;
    let fetched = manifest
        .requires
        .remaps
        .iter()
        .map(|pkgref| fetch_offer(&resolver, &store_root, pkgref))
        .collect::<Result<Vec<_>>>()?;
    for (cached, _) in &fetched {
        let key = format!("{}/{}", cached.resolved.group, cached.resolved.name);
        for request in workspace_requirements
            .iter()
            .filter(|request| request.qualified_name() == key)
        {
            if !request.version.matches(&cached.resolved.version) {
                bail!(
                    "selected remap `{key}` does not satisfy every authored workspace requirement; source binding left unchanged"
                );
            }
        }
    }
    let mut home = RemapHome::open()?;
    let mut pinned = Vec::new();
    {
        let _lock = home.lock()?;
        for (cached, offer) in fetched {
            let installed = adopt_cached(&mut home, &cached, offer, None)?;
            pinned.push(vibe_core::manifest::LockedRemap {
                package: installed.offer.key.clone(),
                version: installed.offer.version.clone(),
                content_hash: installed.offer.content_hash.clone(),
                registry: installed.registry.clone(),
                source: Some(installed.source),
            });
        }
        home.save()?;
    }
    pinned.sort_by(|left, right| left.package.cmp(&right.package));
    let count = pinned.len();
    lockfile
        .remaps
        .retain(|row| retained.contains(&row.package));
    for row in pinned {
        lockfile
            .remaps
            .retain(|existing| existing.package != row.package);
        lockfile.remaps.push(row);
    }
    lockfile
        .remaps
        .sort_by(|left, right| left.package.cmp(&right.package));
    Ok(count)
}

/// One report shape for a remap install: what was offered and what it
/// takes to apply it.
fn report(ctx: &output::Context, home: &RemapHome, installed: &[Installed]) -> Result<()> {
    let rows: usize = installed
        .iter()
        .map(|entry| entry.offer.section.rows.len())
        .sum();
    if ctx.is_json() {
        return ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "remap:install",
            "home": home.root().display().to_string(),
            "packages": installed
                .iter()
                .map(|entry| serde_json::json!({
                    "package": entry.offer.key,
                    "version": entry.offer.version,
                    "content_hash": entry.offer.content_hash,
                    "registry": entry.registry,
                    "rows": entry.offer.section.rows.len(),
                    "state": home
                        .state(&entry.offer.key)
                        .unwrap_or(OfferState::Offered)
                        .label(),
                }))
                .collect::<Vec<_>>(),
            "rows": rows,
        }));
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe install -g: {} remap package(s) offered, {rows} row(s)",
            installed.len()
        ));
        return Ok(());
    }
    for entry in installed {
        ctx.created(&format!(
            "{}@{} ({} row(s)) — {}",
            entry.offer.key,
            entry.offer.version,
            entry.offer.section.rows.len(),
            home.state(&entry.offer.key)
                .unwrap_or(OfferState::Offered)
                .label()
        ));
    }
    let keys: Vec<&str> = installed
        .iter()
        .map(|entry| entry.offer.key.as_str())
        .collect();
    let pending = installed
        .iter()
        .any(|entry| home.state(&entry.offer.key).is_some_and(OfferState::asks));
    if pending {
        ctx.summary(&format!(
            "\nNothing applies yet — a remap is an offer. `vibe remap list` shows every row; \
             `vibe remap accept {}` applies them.",
            keys.join(" ")
        ));
    } else {
        ctx.summary(&format!(
            "\n{rows} row(s) recorded in {}.",
            home.root().display()
        ));
    }
    Ok(())
}

/// Diagnose missing demanded authority before project installation or consent writes.
pub(crate) fn preflight_project_sources(selected: &Path) -> Result<()> {
    let manifest_path = selected.join(Manifest::FILENAME);
    if !manifest_path.exists() {
        return Ok(());
    }
    // An unreadable manifest declares no remap authority to admit; the
    // command's own single read reports it inside its funnel, after the run
    // identity is selected (PROP-054 trace activation), so it is not
    // pre-empted here.
    let Ok(manifest) = Manifest::read(&manifest_path) else {
        return Ok(());
    };
    if manifest.requires.remaps.is_empty() {
        return Ok(());
    }
    let workspace = owning_workspace(selected)?;
    let path = workspace.root.join(Lockfile::FILENAME);
    match std::fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
        Ok(_) => {}
    }
    let lock = Lockfile::read(&path)?;
    for request in &manifest.requires.remaps {
        if let Some(row) = lock
            .remaps
            .iter()
            .find(|row| row.package == request.qualified_name())
        {
            row.locked_package()?;
        }
    }
    Ok(())
}

/// Parent workspace owns the lock; a legacy misplaced child lock is not migrated.
pub(crate) fn owning_workspace(selected: &Path) -> Result<vibe_workspace::Workspace> {
    let selected = selected.canonicalize()?;
    let workspace = vibe_workspace::Workspace::discover(&selected)?;
    // Both sides canonical: on Windows `canonicalize` yields the verbatim
    // `\\?\C:\…` form, which a discovered root does not carry.
    let root = workspace
        .root
        .canonicalize()
        .unwrap_or_else(|_| workspace.root.clone());
    if selected != root {
        match std::fs::symlink_metadata(selected.join(Lockfile::FILENAME)) {
            Ok(_) => bail!(
                "misplaced member vibe.lock: preserve it and explicitly reconcile with the owning workspace lock before remap acquisition"
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(workspace)
}

mod preview;
pub(crate) use preview::preview_project_requirements;
