//! `vibe doc shell` — what the reader is wearing, and how to get the real
//! thing (PROP-057 `##SHELL-SERVE-SOURCES`, `##SHELL-INSTALL-COMMAND`,
//! `##SHELL-PIN`).
//!
//! Two verbs and a resolver. `status` prints the shell this binary
//! carries and the three digests that have to agree; `install` downloads
//! the shell this version was built with, after asking; and [`resolve`]
//! is what `vibe doc serve` calls to get one.
//!
//! ## Consent is the whole design
//!
//! `##INV-LOCAL-IS-OFFLINE` says the local reader contacts the network
//! only for a shell download the user explicitly confirmed. So there is
//! no automatic fetch anywhere in this file — not on first run, not on a
//! missing asset, not «just the manifest». A reader with no shell says
//! what it is missing and serves the bare one, which is a working reader.
//! The single network call in the whole documentation surface is behind
//! [`run_install`], and the first thing it does is ask.
//!
//! ## Where a downloaded shell lives
//!
//! `<install root>/vibevm/doc-shell/<sha256>/`, beside `versions/` and
//! `build/` rather than inside a version instance: instances are
//! immutable (PROP-019 §2.4), and one download serves every instance that
//! pins the same digest. The directory is named by the digest of the
//! SHELL TREE — the same number the pin carries — so finding the right
//! shell is a lookup rather than a search, and a shell whose bytes do not
//! hash to its own name cannot be found at all.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#SHELL-INSTALL-COMMAND");

pub mod fetch;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_doc_shell::{Pin, Provenance, Shell, digest};

use super::{DocEnv, observe};
use crate::cli::{DocShellArgs, DocShellCommand, DocShellInstallArgs, DocShellStatusArgs};
use crate::output;
use fetch::{Fetcher, HttpFetcher};

/// Run `vibe doc shell …`.
pub fn run(ctx: &output::Context, args: DocShellArgs, env: DocEnv) -> Result<()> {
    match args.command {
        None => run_status(DocShellStatusArgs { json: false }, env),
        Some(DocShellCommand::Status(status)) => run_status(status, env),
        Some(DocShellCommand::Install(install)) => run_install(ctx, install, env, &HttpFetcher),
    }
}

/// The shell a reader should wear, given what is on this machine.
///
/// Never fails and never reaches the network: the worst case is the bare
/// shell, which reads.
pub fn resolve(env: &DocEnv, bare: bool) -> Shell {
    if bare {
        return Shell::bare();
    }
    Shell::open(store_dir(env).as_deref())
}

/// Where a downloaded shell for THIS build would be, if it is there.
///
/// `None` when the install root is unknown or the pin names no shell —
/// both of which mean «there is nothing to look up», not «look
/// somewhere else».
pub fn store_dir(env: &DocEnv) -> Option<PathBuf> {
    let pin = Pin::compiled_in().ok()?;
    if !pin.is_set() {
        return None;
    }
    Some(
        env.install_root
            .as_ref()?
            .join(vibe_doc_shell::STORE_DIR)
            .join(&pin.sha256),
    )
}

/// `vibe doc shell status` — the three digests and the pin.
fn run_status(args: DocShellStatusArgs, env: DocEnv) -> Result<()> {
    let shell = resolve(&env, false);
    let report = shell.report();
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print!("{}", report.render());
        if report.provenance == Provenance::Fallback
            && let Some(dir) = store_dir(&env)
        {
            println!("  a downloaded shell would be read from {}", dir.display());
        }
    }
    if !report.matches_pin() {
        bail!(
            "the shell this binary carries is not the one it was pinned to (violates \
             spec://org.vibevm.core/vibevm/common/PROP-057#SHELL-PIN; \
             fix: run `cargo xtask embed-doc-shell` and rebuild, or `vibe doc shell install` \
             on a build from source)"
        );
    }
    Ok(())
}

/// `vibe doc shell install` — the one place in this surface that reaches
/// the network, and only after a person said so.
fn run_install(
    ctx: &output::Context,
    args: DocShellInstallArgs,
    env: DocEnv,
    fetcher: &dyn Fetcher,
) -> Result<()> {
    let pin = Pin::compiled_in().context("reading the shell pin compiled into this `vibe`")?;
    if !pin.is_set() {
        bail!(
            "this `vibe` was built from a checkout that has never built a shell, so there is \
             no digest to ask a release for (violates \
             spec://org.vibevm.core/vibevm/common/PROP-057#SHELL-PIN; \
             fix: run `cargo xtask embed-doc-shell` in the checkout and rebuild — a download \
             is for a release binary, and a release binary is pinned)"
        );
    }
    let Some(into) = store_dir(&env) else {
        bail!(
            "there is no install root to place a shell under (violates \
             spec://org.vibevm.core/vibevm/common/PROP-019#ROOT-DEFAULT; \
             fix: set `VIBEVM_INSTALL_ROOT`, or run from a machine with a home directory)"
        );
    };
    if into.join(vibe_doc_shell::index::INDEX_FILE).is_file() {
        println!(
            "the shell this `vibe` is pinned to is already in the store:\n  {}",
            into.display()
        );
        return Ok(());
    }

    let version = env!("CARGO_PKG_VERSION").to_string();
    let base = args
        .from
        .clone()
        .unwrap_or_else(|| fetch::RELEASE_ROOT.to_string());
    consent(ctx, &args, &env, &base, &version, &pin)?;

    let staging = into.with_file_name(format!(".download-{}-{}", std::process::id(), pin.sha256));
    let outcome = install_into(
        &staging,
        &into,
        fetcher,
        &base,
        &version,
        &pin,
        &env.progress,
    );
    let _ = std::fs::remove_dir_all(&staging);
    outcome
}

/// Fetch, verify, unpack, re-measure, publish.
fn install_into(
    staging: &Path,
    into: &Path,
    fetcher: &dyn Fetcher,
    base: &str,
    version: &str,
    pin: &Pin,
    progress: &vibe_core::progress::Progress,
) -> Result<()> {
    std::fs::create_dir_all(staging)
        .with_context(|| format!("creating `{}`", staging.display()))?;

    let manifest_path = staging.join(fetch::MANIFEST_FILENAME);
    let manifest_url = fetch::asset_url(base, version, fetch::MANIFEST_FILENAME);
    println!("reading {manifest_url}");
    observe::phase(progress, "Downloading reader shell manifest", || {
        fetcher.fetch(&manifest_url, &manifest_path, fetch::manifest_max_bytes())
    })?;
    let manifest = observe::phase(progress, "Verifying reader shell manifest", || {
        let manifest_bytes = std::fs::read(&manifest_path)
            .with_context(|| format!("reading `{}`", manifest_path.display()))?;
        fetch::parse_manifest(&manifest_bytes, version)
    })?;
    // What the release says it is, printed before anything is fetched:
    // an operator who typed `--from` at a mirror should see whose release
    // answered, and a report of a bad download should carry the commit.
    println!(
        "  {} {} from {} ({}), built at {}",
        manifest.product,
        manifest.tag,
        manifest.repository,
        manifest.version,
        manifest.source_commit
    );

    let archive_path = staging.join(&manifest.asset.name);
    let archive_url = fetch::asset_url(base, version, &manifest.asset.name);
    println!("downloading {archive_url}");
    let _cleanup = fetch::Cleanup(archive_path.clone());
    observe::phase(progress, "Downloading reader shell archive", || {
        fetcher.fetch(&archive_url, &archive_path, fetch::asset_max_bytes())
    })?;
    observe::phase(progress, "Verifying reader shell archive", || {
        let archive_bytes = std::fs::read(&archive_path)
            .with_context(|| format!("reading `{}`", archive_path.display()))?;
        fetch::require_digest("the shell asset", &archive_bytes, &manifest.asset)
    })?;

    let unpacked = staging.join("shell");
    let count = observe::phase(progress, "Extracting reader shell", || {
        fetch::unpack(&archive_path, &unpacked)
    })?;

    // The second check, and the one that matters to a reader: the asset
    // was what the release said, and now the SHELL is what this binary
    // was pinned to. A release could be intact and be another version's.
    let measured = observe::phase(progress, "Verifying extracted reader shell", || {
        let mut files = digest::read_tree(&unpacked)
            .with_context(|| format!("reading `{}`", unpacked.display()))?;
        files.remove(vibe_doc_shell::index::INDEX_FILE);
        Ok::<_, anyhow::Error>(digest::of(&files))
    })?;
    if measured != pin.sha256 {
        bail!(
            "the downloaded shell hashes to {measured} and this `vibe` is pinned to {} \
             (violates spec://org.vibevm.core/vibevm/common/PROP-057#SHELL-PIN; \
             fix: nothing local — the release carries a shell built from another tree, and \
             the bare shell is what this reader keeps)",
            pin.sha256
        );
    }

    observe::phase(progress, "Publishing reader shell", || {
        if let Some(parent) = into.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating `{}`", parent.display()))?;
        }
        std::fs::rename(&unpacked, into).with_context(|| {
            format!(
                "placing the shell at `{}` — it is content-addressed, so a directory that is \
                 already there is already the right one",
                into.display()
            )
        })
    })?;
    println!(
        "the shell is in the store: {count} file(s) at {}",
        into.display()
    );
    println!("  `vibe doc serve` will wear it from now on");
    Ok(())
}

/// Ask, unless the operator already answered.
///
/// The posture arrives from the composition root the way every other
/// ambient value in this surface does. `--assume-yes` IS the consent, a
/// `--json` or unattended run has already declared that nobody is at the
/// keyboard, and a run with no terminal and no flag refuses rather than
/// guessing — the pattern `vibe install` set, kept identical because an
/// operator should not have to learn a second one.
fn consent(
    ctx: &output::Context,
    args: &DocShellInstallArgs,
    env: &DocEnv,
    base: &str,
    version: &str,
    pin: &Pin,
) -> Result<()> {
    if args.assume_yes || env.unattended || env.json {
        return Ok(());
    }
    ctx.suspend_progress(|| -> Result<()> {
        println!("The reader's shell is not on this machine.");
        println!("  from    {base}/v{version}/{}", fetch::MANIFEST_FILENAME);
        println!("  pinned  {}", pin.sha256);
        println!(
            "This is the only moment `vibe doc` uses the network. Reading documentation never \
         does, and nothing here is sent anywhere."
        );
        if !console::user_attended() {
            bail!(
                "no terminal to ask at; re-run with `--assume-yes` to take the download as \
             approved (violates \
             spec://org.vibevm.core/vibevm/common/PROP-057#SHELL-INSTALL-COMMAND; \
             fix: `vibe doc shell install --assume-yes`, or keep the bare shell)"
            );
        }
        let approved = dialoguer::Confirm::new()
            .with_prompt("Download the documentation shell?")
            .default(false)
            .interact()
            .context("reading the confirmation")?;
        if !approved {
            bail!(
                "declined; the bare shell is what this reader keeps, and it reads \
             (spec://org.vibevm.core/vibevm/common/PROP-057#LOCAL-OFFLINE-SHELL)"
            );
        }
        Ok(())
    })
}

#[cfg(test)]
#[path = "shell/tests.rs"]
mod tests;
