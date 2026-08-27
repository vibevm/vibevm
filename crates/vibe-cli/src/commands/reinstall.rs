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
//! ## One command, one owner
//!
//! Every branch — plain, empty `--force`, ordinary `--force`, and the
//! continuation any of them may service — runs under exactly one
//! [`compile_trace::TracePreparation`], opened here before anything compiles
//! and consumed by exactly one typed exit. `--force` is a MATERIALISATION
//! force and never a lifecycle repark force, so a forced run can still adopt
//! and finish the park it created.
//!
//! Nothing between `prepare` and `finalize` returns with `?`: an open recorder
//! holds the project's cooperative lock and leaves its index `running` on disk,
//! so the executed region is a function returning a value and every error
//! inside it is classified into that value.
//!
//! Spec: spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009-loading-model §2.10.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#cli-surface");

mod continuation;
mod draft;
mod force;
mod inputs;
mod prepare;
mod regenerate;

pub(crate) use draft::{ReinstallDraft, ReinstallIdentity};

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_core::manifest::{Manifest, Materialization, SpecFormat};
use vibe_core::user_config::UserConfig;
use vibe_core::{ContentHash, Group};
use vibe_install::{InstallProgress, InstallSource};
use vibe_workspace::Workspace;
use vibe_workspace::compile_trace::TraceRun;
use vibe_workspace::vibedeps;

use crate::cli::ReinstallArgs;
use crate::commands::compile_trace::{self, CommandExit, RegisteredReportDraft, render_finalized};
use crate::commands::install::{PreparedWorkspace, SelectedManifest, build_install_resolver};
use crate::output;

use inputs::{exact_pkgref, load_lockfile, resolver_args};
use prepare::PreparedReinstall;

/// Reload and re-derive exactly one installed transformed slot.
///
/// NOT a `vibe reinstall` outer-command path: it is a point repair W2 calls,
/// with no lifecycle run, no report and therefore no trace owner. It keeps its
/// own discovery and its own config load precisely because it is not this
/// command.
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
    let PreparedReinstall {
        project_root,
        user_config,
        offline,
        metadata,
        manifest,
        workspace,
        trace,
    } = prepare::prepare(ctx, &args, root_offline)?;
    let exit = execute_after_open(
        ctx,
        Execution {
            args,
            embedded_root,
            offline,
            project_root,
            user_config,
            manifest,
            workspace,
            metadata,
        },
        trace.recorder(),
    );
    // Consumes the owner: finishes the index, drops the last handle (and with
    // it the cooperative lock), and returns the member to attach.
    let finalized = compile_trace::finalize(trace, exit, &now);
    render_finalized(ctx, finalized)
}

/// The prepared inputs the executed region owns.
struct Execution {
    args: ReinstallArgs,
    embedded_root: Option<PathBuf>,
    offline: bool,
    project_root: PathBuf,
    user_config: UserConfig,
    manifest: SelectedManifest,
    workspace: PreparedWorkspace,
    metadata: vibe_lifecycle::RunMetadata,
}

/// The one boundary: everything after `prepare` and before `finalize`.
fn execute_after_open(
    ctx: &output::Context,
    execution: Execution,
    trace: Option<&TraceRun>,
) -> CommandExit<RegisteredReportDraft> {
    // The report identity is the SELECTED node, decided before anything can
    // move it — see `ReinstallIdentity`. Operational facts stay workspace-
    // rooted; this answers "which invocation is this document about".
    let identity = ReinstallIdentity {
        selected_project_root: execution.project_root.clone(),
        forced: execution.args.force,
    };
    match run_inner(ctx, execution, trace) {
        Ok(draft) => {
            let parked = draft.delegation.is_some();
            let draft = RegisteredReportDraft::Reinstall(Box::new(draft));
            if parked {
                CommandExit::Parked(draft)
            } else {
                CommandExit::Success(draft)
            }
        }
        Err(error) => compile_trace::classify(error, || {
            RegisteredReportDraft::Reinstall(Box::new(ReinstallDraft::failed(
                &identity,
                InstallProgress::default(),
                Vec::new(),
            )))
        }),
    }
}

fn run_inner(
    ctx: &output::Context,
    execution: Execution,
    trace: Option<&TraceRun>,
) -> Result<ReinstallDraft> {
    let Execution {
        args,
        embedded_root,
        offline,
        project_root,
        user_config,
        manifest,
        workspace,
        metadata,
    } = execution;
    // The stored snapshot is consumed for its ERROR first: a malformed selected
    // `vibe.toml` is this command's failure, in its own words. Its VALUE is
    // deliberately not the operational manifest — see `prepare`.
    let identity = ReinstallIdentity {
        selected_project_root: project_root.clone(),
        forced: args.force,
    };
    let selected = manifest.into_manifest()?;
    // The ONE workspace answer, returned as it was. Retrying could succeed
    // against a tree the identity and the trace were never prepared for.
    let workspace = match workspace {
        PreparedWorkspace::Loaded(workspace) => *workspace,
        PreparedWorkspace::DiscoveryFailed(error) => {
            return Err(anyhow::Error::new(*error)
                .context("discovering the workspace enclosing the project"));
        }
        // Unreachable through the owner above, which consumes the manifest
        // error first; named rather than merged so a future caller is a
        // compile-time question instead of a silent success.
        PreparedWorkspace::SelectedManifestInvalid => {
            bail!(
                "internal: the selected manifest was reported invalid but its error was \
                 already consumed"
            );
        }
        PreparedWorkspace::DiscoverHere => {
            Workspace::discover_with_selected_manifest(&project_root, &selected)
                .context("discovering the workspace enclosing the project")?
        }
    };

    let lockfile = load_lockfile(&workspace.root)?;
    // The workspace ROOT's manifest, from the tree already in hand.
    let spec_format =
        crate::commands::install::resolve_spec_format(&workspace.root_manifest, &user_config);

    if args.force {
        force::run(
            ctx,
            force::Forced {
                args: &args,
                identity: &identity,
                workspace: &workspace,
                lockfile: &lockfile,
                metadata: &metadata,
                spec_format,
                trace,
                embedded_root: embedded_root.as_deref(),
                offline,
            },
        )
    } else {
        regenerate::run(
            ctx,
            regenerate::Plain {
                args: &args,
                identity: &identity,
                workspace: &workspace,
                lockfile: &lockfile,
                metadata: &metadata,
                spec_format,
                trace,
            },
        )
    }
}

/// The injected instant. Both the supersession pass and the finish read time
/// through this one closure, so a test can count its calls and prove the
/// disabled path never asked.
pub(super) fn now() -> vibe_wire::generated::shared::Timestamp {
    chrono::Utc::now()
}
