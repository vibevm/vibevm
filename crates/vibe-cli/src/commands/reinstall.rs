//! `vibe reinstall [<path>] [--force]` — recompute the materialised state
//! and the boot artifacts of a workspace.
//!
//! `vibe reinstall` regenerates the computed loading model (PROP-009
//! §2.10). It **never re-resolves** — the versions stay exactly as
//! `vibe.lock` pins them; moving a version is `vibe update`'s job.
//!
//! Two modes:
//!
//! - **`vibe reinstall`** (no `--force`) — recompute every node's boot
//!   artifacts from the materialised `vibedeps/` tree already on disk.
//!   No fetch, no network. The fix for a stale or hand-edited boot
//!   artifact — a previous generation pass that produced a wrong
//!   `INDEX.md`. Every locked package must have its `vibedeps/` slot
//!   present; a missing slot is content this mode cannot recover, so it
//!   stops and points the operator at `--force`.
//! - **`vibe reinstall --force`** — re-fetch every locked package's
//!   content from its source repository at the lockfile-pinned version,
//!   bypassing the project cache, then re-materialise `vibedeps/` and
//!   regenerate boot. The escape hatch for a corrupted `vibedeps/`
//!   subtree.
//!
//! Discovery bubbles to the absolute workspace root, so reinstalling
//! regenerates the whole workspace — a node and every ancestor
//! (PROP-009 §2.10): a node's aggregated boot depends on its members'.
//!
//! Spec: spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009-loading-model §2.10.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#cli-surface");

mod report;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use dialoguer::Confirm;
use vibe_core::manifest::{Lockfile, Manifest, Materialization, SpecFormat};
use vibe_core::user_config::{SlotIntegrity, UserConfig};
use vibe_core::{ContentHash, Group, PackageKind, PackageRef, VersionSpec};
use vibe_install::{InstallSlotLifecycle, InstallSource};
use vibe_lifecycle::RunMetadata;
use vibe_lifecycle::process::StreamMode;
use vibe_workspace::Workspace;
use vibe_workspace::install::{
    ResolvedDep, SlotCheck, SlotLifecycleMode, SlotVerifier,
    apply_resolution_with_spec_format_and_slot_lifecycle, regenerate_boot_with_spec_format,
    run_post_install_slot_lifecycle,
};
use vibe_workspace::vibedeps;

use crate::cli::{InstallArgs, ReinstallArgs};
use crate::commands::install::{HookReportView, LifecycleHookView, build_install_resolver};
use crate::exit_code::InstallError;
use crate::output;

struct SourceHashes(HashMap<(Group, String), String>);

impl SlotVerifier for SourceHashes {
    fn source_hash<'a>(&'a self, dep: &ResolvedDep) -> Option<&'a str> {
        self.0
            .get(&(dep.group.clone(), dep.name.clone()))
            .map(String::as_str)
    }

    fn verify_slot(&self, _dep: &ResolvedDep, _slot_abs: &Path) -> SlotCheck {
        SlotCheck::Unverifiable
    }
}

/// Reload and re-derive exactly one installed transformed slot.
///
/// `false` means the package has no versioned slot. Mixed and in-place slots
/// are deliberately left alone by W2 because they carry no derived manifest.
pub(super) fn rederive_package(project_root: &Path, package: &str) -> Result<bool> {
    let Some((group_text, name)) = package.split_once('/') else {
        bail!("package must use the `<group>/<name>` form: {package}");
    };
    if name.is_empty() || name.contains('/') {
        bail!("package must use the `<group>/<name>` form: {package}");
    }
    let group = Group::parse(group_text)?;
    let workspace = Workspace::discover(project_root)
        .context("discovering the workspace enclosing the facts registry")?;
    let lockfile = load_lockfile(&workspace.root)?;
    let Some(locked) = lockfile
        .packages
        .iter()
        .find(|locked| locked.group == group && locked.name == name)
    else {
        return Ok(false);
    };
    if locked.materialization.is_in_place() {
        return Ok(true);
    }
    if !vibedeps::is_materialised(
        &workspace.root,
        &locked.group,
        &locked.name,
        &locked.version,
    ) {
        return Ok(false);
    }

    let manifest = Manifest::read(workspace.root.join(Manifest::FILENAME))?;
    let user_config = UserConfig::load().context("loading the user config")?;
    let spec_format = crate::commands::install::resolve_spec_format(&manifest, &user_config);
    if spec_format == SpecFormat::Mixed {
        return Ok(true);
    }

    let global = vibe_core::GlobalRegistryConfig::load()?;
    // Every installed copy package was inserted into the immutable machine
    // store before its slot was written. Point re-derivation therefore reads
    // that exact locked source locally and never needs a registry walk.
    let offline = true;
    let resolver = build_install_resolver(
        &resolver_args(),
        &workspace.root_manifest,
        None,
        &workspace.root,
        &global,
        offline,
        &lockfile.packages,
    )
    .context("building the point re-derivation resolver")?;
    let pkgref = exact_pkgref(locked.kind, &locked.group, &locked.name, &locked.version)?;
    let store_root =
        vibe_registry::store::store_root().context("resolving the machine package store root")?;
    let cached = resolver
        .resolve_and_fetch(&pkgref, &store_root, Some(&locked.content_hash))
        .with_context(|| format!("re-fetching `{package}` for point re-derivation"))?;
    let mode = match cached
        .manifest
        .package
        .as_ref()
        .map(|package| package.materialization)
    {
        Some(Materialization::Hardlink) => vibedeps::CopyMode::Hardlink,
        _ => vibedeps::CopyMode::Copy,
    };
    vibedeps::materialise_with_spec_format(
        &workspace.root,
        &cached.resolved.group,
        &cached.resolved.name,
        &cached.resolved.version,
        &cached.cache_dir,
        mode,
        spec_format,
        &ContentHash::from_validated(cached.content_hash.clone()),
    )
    .with_context(|| format!("re-deriving the installed `{package}` slot"))?;
    Ok(true)
}

pub fn run(
    ctx: &output::Context,
    args: ReinstallArgs,
    embedded_root: Option<PathBuf>,
    root_offline: bool,
) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let workspace = Workspace::discover(&project_root)
        .context("discovering the workspace enclosing the project")?;
    let lockfile = load_lockfile(&workspace.root)?;
    let user_config = UserConfig::load().context("loading the user config")?;
    let spec_format =
        crate::commands::install::resolve_spec_format(&workspace.root_manifest, &user_config);

    if args.force {
        run_force(
            ctx,
            &workspace,
            &lockfile,
            &args,
            embedded_root.as_deref(),
            root_offline,
            spec_format,
        )
    } else {
        run_regenerate(ctx, &workspace, &lockfile, &args, spec_format)
    }
}

/// `vibe reinstall` — regenerate every node's boot artifacts from the
/// materialised `vibedeps/` tree already on disk. No fetch, no network.
fn run_regenerate(
    ctx: &output::Context,
    workspace: &Workspace,
    lockfile: &Lockfile,
    args: &ReinstallArgs,
    spec_format: SpecFormat,
) -> Result<()> {
    // Without `--force` the materialised `vibedeps/` tree is the only
    // content source. Every locked package must have its slot on disk —
    // a missing slot is content this mode cannot conjure; only a fetch
    // (`--force`) can.
    // An in-place package's slot is the unversioned git working tree
    // (PROP-022 §2.4); every other mode is the versioned slot. Check, and
    // name, the right one per mode.
    let slot_present = |p: &vibe_core::manifest::LockedPackage| {
        if p.materialization.is_in_place() {
            vibedeps::is_in_place_slot(&workspace.root, &p.group, &p.name)
        } else {
            vibedeps::is_materialised(&workspace.root, &p.group, &p.name, &p.version)
        }
    };
    let slot_label = |p: &vibe_core::manifest::LockedPackage| {
        if p.materialization.is_in_place() {
            vibedeps::in_place_slot_rel_path(&p.group, &p.name)
        } else {
            vibedeps::slot_rel_path(&p.group, &p.name, &p.version)
        }
    };
    let missing: Vec<String> = lockfile
        .packages
        .iter()
        .filter(|p| !slot_present(p))
        .map(slot_label)
        .collect();
    if !missing.is_empty() {
        bail!(
            "the materialised `vibedeps/` tree is incomplete — {} slot{} missing:\n  {}\n\
             Run `vibe reinstall --force` to re-fetch the content from source.",
            missing.len(),
            if missing.len() == 1 { "" } else { "s" },
            missing.join("\n  "),
        );
    }

    let node_count = workspace.iter_nodes().count();
    ctx.heading(&format!(
        "\nReinstall — regenerate boot artifacts for {node_count} node{} from vibedeps/.",
        if node_count == 1 { "" } else { "s" },
    ));

    if !confirm(
        ctx,
        args,
        "Regenerate the boot artifacts from the materialised vibedeps/ tree?",
    )? {
        return Err(InstallError::UserDeclined.into());
    }

    let nodes = regenerate_boot_with_spec_format(workspace, spec_format)
        .context("regenerating boot artifacts")?;
    report::emit(ctx, false, &nodes, &[], &HookReportView::empty())?;
    Ok(())
}

/// `vibe reinstall --force` — re-fetch every locked package from source,
/// bypassing the project cache, then re-materialise and regenerate boot.
///
/// `root_offline` carries the invocation's offline posture (PROP-010
/// §2.5): `--force` is the reinstall mode that touches the network, so
/// the root flag / `VIBE_OFFLINE` / `[net].offline` ladder resolves here
/// and narrows the resolver exactly as it does for `vibe install`.
fn run_force(
    ctx: &output::Context,
    workspace: &Workspace,
    lockfile: &Lockfile,
    args: &ReinstallArgs,
    embedded_root: Option<&Path>,
    root_offline: bool,
    spec_format: SpecFormat,
) -> Result<()> {
    // No locked packages — `--force` has nothing to re-fetch. Still
    // regenerate boot so a stale artifact is recomputed.
    if lockfile.packages.is_empty() {
        ctx.heading("\nReinstall --force — no packages locked; regenerate boot only.");
        if !confirm(
            ctx,
            args,
            "No packages are locked — regenerate boot artifacts only?",
        )? {
            return Err(InstallError::UserDeclined.into());
        }
        let outcome = apply_resolution_with_spec_format_and_slot_lifecycle(
            workspace,
            &[],
            SlotIntegrity::Verify,
            spec_format,
            None,
            SlotLifecycleMode::None,
        )
        .context("regenerating the workspace")?;
        let hook_reports = HookReportView::empty();
        report::emit(
            ctx,
            true,
            &outcome.nodes_regenerated,
            &outcome.pruned,
            &hook_reports,
        )?;
        return Ok(());
    }

    ctx.heading(&format!(
        "\nReinstall --force — re-fetch {} package{} from source:",
        lockfile.packages.len(),
        if lockfile.packages.len() == 1 {
            ""
        } else {
            "s"
        },
    ));
    for p in &lockfile.packages {
        ctx.step(&format!("{}:{}@{}", p.kind, p.name, p.version));
    }

    if !confirm(
        ctx,
        args,
        &format!(
            "Re-fetch {} package{} from source and re-materialise vibedeps/?",
            lockfile.packages.len(),
            if lockfile.packages.len() == 1 {
                ""
            } else {
                "s"
            },
        ),
    )? {
        return Err(InstallError::UserDeclined.into());
    }

    // The resolver is built from the workspace root manifest — registries,
    // mirrors, overrides, and git-source declarations are root-level. The
    // offline posture is resolved with the same ladder install uses
    // (PROP-010 §2.5): root `--offline` > `VIBE_OFFLINE` > `[net].offline`.
    // `vibe reinstall` carries no local `--offline` flag of its own.
    let global = vibe_core::GlobalRegistryConfig::load()?;
    let offline = crate::output::resolve_offline(
        root_offline,
        vibe_core::user_config::UserConfig::load()
            .context("loading the user config")?
            .net
            .offline,
    );
    let resolver = build_install_resolver(
        &resolver_args(),
        &workspace.root_manifest,
        embedded_root,
        &workspace.root,
        &global,
        offline,
        &lockfile.packages,
    )
    .context("building the install resolver")?;

    // Re-fetch every locked package at its exact pinned version — no
    // re-resolution, the lockfile decides the version. The recorded
    // `content_hash` is forwarded so a source serving disagreeing bytes
    // is rejected: `vibe reinstall` reproduces the lock, never drifts it.
    //
    // The old "wipe the project cache so every fetch re-downloads"
    // step (PROP-009 §2.10) retired with the project cache itself:
    // payload now lands in the machine-global store (`~/.vibe/cache/`,
    // PROP-010 §2.7), which our code never rewrites — every fetch
    // still walks the sources, and the pin gate plus the read-time
    // entry check make the re-fetched bytes prove themselves against
    // the lockfile regardless of what the store already holds.
    let store_root =
        vibe_registry::store::store_root().context("resolving the machine package store root")?;

    let mut resolution: Vec<ResolvedDep> = Vec::with_capacity(lockfile.packages.len());
    let mut source_hashes = HashMap::new();
    for locked in &lockfile.packages {
        let pkgref = exact_pkgref(locked.kind, &locked.group, &locked.name, &locked.version)?;
        let cached = resolver
            .resolve_and_fetch(&pkgref, &store_root, Some(&locked.content_hash))
            .with_context(|| {
                format!(
                    "re-fetching `{}/{}@{}` from source",
                    locked.group, locked.name, locked.version
                )
            })?;
        source_hashes.insert(
            (cached.resolved.group.clone(), cached.resolved.name.clone()),
            cached.content_hash.clone(),
        );
        resolution.push(ResolvedDep {
            kind: cached.package_meta().kind,
            group: cached.resolved.group.clone(),
            name: cached.resolved.name.clone(),
            version: cached.resolved.version.clone(),
            content_dir: cached.cache_dir.clone(),
            source_hash: Some(ContentHash::from_validated(cached.content_hash.clone())),
            manifest: cached.manifest.clone(),
            // The recorded resolution edges — `apply_resolution` walks
            // them to compose each node's dependency boot. A lockfile
            // dependency pkgref is group-qualified (PROP-008 §2.6).
            requires: locked
                .dependencies
                .iter()
                .filter_map(|p| p.group.clone().map(|g| (g, p.name.to_string())))
                .collect(),
            admitted_by: None,
            via_override: None,
            // `--force` materialises with `Verify` (below), so this flag does
            // not change reinstall's behaviour; set from the source for
            // consistency with `vibe install` (PROP-011 §2.6).
            source_mutable: vibe_workspace::freshness::is_in_workspace_file_source(
                &cached.source_uri,
                &workspace.root,
            ),
            in_place_changed: None,
        });
    }

    let manifest = Manifest::read(workspace.root.join(Manifest::FILENAME))?;
    let lifecycle_metadata = reinstall_metadata(ctx, &workspace.root, root_offline, args)?;
    let lifecycle_observer =
        crate::commands::install::LifecycleSlotObserver::new(ctx, lifecycle_metadata.clone());
    let lifecycle = InstallSlotLifecycle::from_resolution_observed(
        &workspace.root,
        &manifest,
        &resolution,
        lifecycle_metadata.clone(),
        reinstall_stream_mode(ctx),
        std::sync::Arc::new(lifecycle_observer),
    )?;
    let mut outcome = apply_resolution_with_spec_format_and_slot_lifecycle(
        workspace,
        &resolution,
        SlotIntegrity::Verify,
        spec_format,
        Some(&SourceHashes(source_hashes)),
        SlotLifecycleMode::Callback(&lifecycle),
    )
    .context("re-materialising the workspace")?;
    if let Some(plan) = outcome.take_post_install_plan() {
        run_post_install_slot_lifecycle(plan, SlotLifecycleMode::Callback(&lifecycle))
            .context("running post-install lifecycle")?;
    }
    let lifecycle_reports = lifecycle.take_reports()?;
    let hook_reports = LifecycleHookView::new(&lifecycle_reports);
    report::emit(
        ctx,
        true,
        &outcome.nodes_regenerated,
        &outcome.pruned,
        &hook_reports,
    )?;
    Ok(())
}

fn reinstall_metadata(
    ctx: &output::Context,
    root: &Path,
    offline: bool,
    args: &ReinstallArgs,
) -> Result<RunMetadata> {
    Ok(RunMetadata {
        requested: "reinstall".into(),
        chain: vec!["install".into()],
        offline,
        assume_yes: args.assume_yes || ctx.is_unattended() || ctx.is_json(),
        agent_mode: vibe_wire::generated::lifecycle::e1::context::RunAgentMode::Cli,
        force: true,
        run_id: vibe_lifecycle::process::allocate_run_id(root)?,
        started: crate::commands::init::current_timestamp_utc(),
    })
}

fn reinstall_stream_mode(ctx: &output::Context) -> StreamMode {
    if ctx.is_json() {
        StreamMode::Capture
    } else if ctx.suppresses_output() {
        StreamMode::Null
    } else {
        StreamMode::Inherit
    }
}

/// Build the `=<version>` pkgref that re-fetches exactly the locked
/// version — `vibe reinstall` never re-resolves.
fn exact_pkgref(
    kind: PackageKind,
    group: &Group,
    name: &str,
    version: &semver::Version,
) -> Result<PackageRef> {
    // Build `=<version>` structurally rather than parsing a string —
    // `VersionReq::parse` panics on a version carrying build metadata
    // (`1.0.0+build`), the latent panic SHRINK-v0.1 killed at the other
    // `={v}` sites.
    let req = semver::VersionReq {
        comparators: vec![semver::Comparator {
            op: semver::Op::Exact,
            major: version.major,
            minor: Some(version.minor),
            patch: Some(version.patch),
            pre: version.pre.clone(),
        }],
    };
    Ok(PackageRef::new(
        Some(kind),
        Some(group.clone()),
        name.to_string(),
        VersionSpec::Req(req),
    )?)
}

/// The `InstallArgs` `build_install_resolver` reads. `vibe reinstall`
/// carries no `--registry` / `--git` / feature flags, so they default
/// off — the resolver is built purely from the manifest's `[[registry]]`
/// / `[[mirror]]` / `[[override]]` / git-source declarations.
fn resolver_args() -> InstallArgs {
    InstallArgs {
        packages: Vec::new(),
        path: PathBuf::from("."),
        registry: None,
        assume_yes: false,
        language: None,
        features: Vec::new(),
        no_default_features: false,
        all_features: false,
        exact: false,
        auth_required: false,
        solver: None,
        git: None,
        tag: None,
        branch: None,
        rev: None,
        git_auth: None,
        git_token_env: None,
        force: false,
        prefer_embedded: false,
        no_prefer_embedded: false,
        no_default_registry: false,
        offline: false,
        embedded_short_circuit: false,
        prefer_local: false,
        no_prefer_local: false,
    }
}

/// Interactive confirmation, matching the install / update / uninstall
/// contract: `--assume-yes`, `--unattended`, and `--json` all imply yes;
/// a non-TTY with none of those set is a hard error.
fn confirm(ctx: &output::Context, args: &ReinstallArgs, prompt: &str) -> Result<bool> {
    if args.assume_yes || ctx.is_unattended() || ctx.is_json() {
        return Ok(true);
    }
    if !console::user_attended() {
        bail!(
            "no TTY available for confirmation; re-run with `--assume-yes` to reinstall \
             non-interactively"
        );
    }
    Confirm::new()
        .with_prompt(prompt)
        .default(false)
        .interact()
        .context("reading user confirmation")
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

/// Load the workspace lockfile, or an empty one when none exists yet.
/// `vibe reinstall` does not require a lockfile — without one it simply
/// regenerates the boot artifacts from the authored boot-lane tree.
fn load_lockfile(root: &Path) -> Result<Lockfile> {
    let path = root.join(Lockfile::FILENAME);
    if path.exists() {
        Ok(Lockfile::read(&path)?)
    } else {
        Ok(Lockfile::empty(
            format!("vibe {}", env!("CARGO_PKG_VERSION")),
            super::init::current_timestamp_utc(),
        ))
    }
}
