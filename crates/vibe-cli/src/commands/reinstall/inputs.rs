//! Reinstall's inputs: its run metadata and stream policy, the exact pkgref
//! that re-fetches the locked version, and the confirmation seam.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use dialoguer::Confirm;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::{Group, PackageKind, PackageRef, VersionSpec};
use vibe_lifecycle::RunMetadata;
use vibe_lifecycle::process::StreamMode;

use crate::cli::{InstallArgs, ReinstallArgs};
use crate::output;

pub(super) fn reinstall_metadata(
    ctx: &output::Context,
    root: &Path,
    offline: bool,
    args: &ReinstallArgs,
) -> Result<RunMetadata> {
    let chain = vec!["install".to_string()];
    // MATERIALISATION force and HOSTED-REPARK force are different things.
    //
    // `--force` is what re-fetches from source and so reaches changed slot
    // callbacks. The generic `RunMetadata.force` means something else
    // entirely: "fresh run id, no probe, repark". Setting it here made a
    // forced reinstall unable to satisfy its own task — every resume minted a
    // new id and reparked, forever. So the lifecycle force stays FALSE, and
    // `--force` keeps its own, unrelated job.
    //
    // An explicit `--force` on an ordinary lifecycle row still reparks under a
    // fresh id: that law lives on the phase verbs' `--force`, untouched here.
    let identity = crate::commands::lifecycle::run_identity(ctx, root, "reinstall", &chain, false)?;
    Ok(RunMetadata {
        requested: "reinstall".into(),
        chain,
        offline,
        assume_yes: args.assume_yes || ctx.is_unattended() || ctx.is_json(),
        agent_mode: ctx.agent_mode(),
        force: false,
        // The selector's effective sticky bit, not a hard-coded false.
        trace_compile: identity.compile_trace,
        run_id: identity.run_id,
        started: identity.started,
    })
}

pub(super) fn reinstall_stream_mode(ctx: &output::Context) -> StreamMode {
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
pub(super) fn exact_pkgref(
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
pub(super) fn resolver_args() -> InstallArgs {
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
pub(super) fn confirm(ctx: &output::Context, args: &ReinstallArgs, prompt: &str) -> Result<bool> {
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

pub(super) fn resolve_project_root(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = crate::commands::init::strip_unc_public(canonical);
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
pub(super) fn load_lockfile(root: &Path) -> Result<Lockfile> {
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
