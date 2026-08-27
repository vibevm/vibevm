//! Reinstall's inputs: its stream policy, the exact pkgref that re-fetches the
//! locked version, the resolver arguments, the confirmation seam and the
//! lockfile reader.
//!
//! The run METADATA is not built here any more, and neither is the project
//! root: both belong to the command's one prepared epoch ([`super::prepare`]).
//! A helper that selected an identity of its own — as this cell used to, and as
//! the continuation helper did a second time — was a second selector, running
//! later, able to allocate a second run directory, and blind to the effective
//! trace bit the command had already committed to.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use dialoguer::Confirm;
use vibe_core::manifest::Lockfile;
use vibe_core::{Group, PackageKind, PackageRef, VersionSpec};
use vibe_lifecycle::process::StreamMode;

use crate::cli::{InstallArgs, ReinstallArgs};
use crate::output;

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
        // FALSE on purpose, whatever `vibe reinstall --trace-compile` said.
        // These arguments only ever reach the resolver builder, and the command
        // already owns the ONE recorder; a request at this depth could only
        // mean a second owner of the project's cooperative trace lock.
        trace_compile: false,
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
