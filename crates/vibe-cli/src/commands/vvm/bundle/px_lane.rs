//! px for installations without a bundle to install now — a source build,
//! a repair — and the `vibe self px` verbs (PROP-019 `#companion-tools`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools");

use std::path::Path;

use anyhow::{Context, Result, bail};
use vibe_core::progress::Progress;
use vibe_publish::token::Token;

use super::super::env::Shell;
use super::super::install::InstallLock;
use super::super::{VvmEnv, output};
use super::download::HttpDownloader;
use super::releases::{ReleaseDir, Releases};
use super::{RemoteContext, api, companions, remote_context};

/// The release directories px is read from: `dir`, and the drop's root
/// when `dir` is a version directory of a local drop.
pub(super) fn px_dirs(dir: &ReleaseDir) -> Vec<ReleaseDir> {
    let mut dirs = vec![dir.clone()];
    if let Some(parent) = dir.local().as_deref().and_then(Path::parent)
        && let Ok(url) = reqwest::Url::from_directory_path(parent)
    {
        dirs.push(ReleaseDir::Base(url.to_string()));
    }
    dirs
}

/// Place the pinned px in the tool store from `dirs`, reporting on one
/// progress line. Only bytes that do not match the pin fail.
pub(super) fn ensure_px(
    remote: &RemoteContext<'_>,
    dirs: &[ReleaseDir],
    token: Option<&Token>,
    force: bool,
) -> Result<companions::PxOutcome> {
    use companions::PxOutcome;
    let task = remote.progress.task("Installing px");
    let outcome = companions::install_from_dirs(
        remote,
        vibe_net::px_store::PxPin::embedded(),
        dirs,
        token,
        force,
    );
    match &outcome {
        Ok(PxOutcome::AlreadyInstalled(program)) => {
            task.skip(format!("already in `{}`", program.display()));
        }
        Ok(PxOutcome::Installed { program, from }) => {
            task.detail(format!("`{}` from {from}", program.display()));
            task.finish();
        }
        Ok(PxOutcome::Unavailable { asset, .. }) => task.skip(format!(
            "the release source has no `{asset}`; `[proxy] mode = \"px\"` uses `px_path` or px on PATH"
        )),
        Ok(PxOutcome::NoTarget) => task.skip("px has no build for this platform"),
        Err(_) => task.fail("px could not be installed"),
    }
    outcome
}

/// Release directories to look for px in, for an installation that has no
/// bundle to install now (a source build, a repair): the running version's,
/// then the newest few the source lists, then a drop's root.
fn px_search_dirs(
    remote: &RemoteContext<'_>,
    from: Option<&Path>,
) -> Result<(Vec<ReleaseDir>, Option<Token>)> {
    if let Some(path) = from {
        let path =
            std::path::absolute(path).with_context(|| format!("resolving `{}`", path.display()))?;
        let dir = if path.is_dir() {
            path
        } else {
            path.parent().map(Path::to_path_buf).unwrap_or(path)
        };
        let url = reqwest::Url::from_directory_path(&dir)
            .map_err(|()| anyhow::anyhow!("`{}` is not an absolute directory", dir.display()))?;
        return Ok((vec![ReleaseDir::Base(url.to_string())], None));
    }
    let api = api::ApiClient::new(remote.downloader.admission())?;
    let releases = Releases::new(&remote.source, &api, remote.locator)?;
    let token = releases.token.clone();
    let mut versions = Vec::new();
    if let Ok(running) = semver::Version::parse(env!("CARGO_PKG_VERSION")) {
        versions.push(running);
    }
    if let Ok(listed) = releases.list() {
        let mut listed = listed;
        listed.sort();
        listed.reverse();
        for version in listed.into_iter().take(3) {
            if !versions.contains(&version) {
                versions.push(version);
            }
        }
    }
    let mut dirs = Vec::new();
    for version in versions {
        if let Ok(dir) = releases.dir(&version) {
            for candidate in px_dirs(&dir) {
                if !dirs.contains(&candidate) {
                    dirs.push(candidate);
                }
            }
        }
    }
    Ok((dirs, token))
}

/// `vibe self px install|status` (PROP-019 `#companion-tools`).
pub(in crate::commands::vvm) fn run_px_cmd(
    ctx: &output::Context,
    env: &VvmEnv,
    args: crate::cli::VvmPxArgs,
    progress: &Progress,
) -> Result<()> {
    use companions::PxOutcome;
    let store = env.store()?;
    let pin = vibe_net::px_store::PxPin::embedded();
    let root = companions::store_root(&store);
    match args.command {
        crate::cli::VvmPxCommand::Status => {
            let asset = companions::pinned_asset(pin);
            let installed = asset.and_then(|asset| companions::installed(&root, pin, asset));
            let name = asset.map(|asset| pin.release_asset_name(asset));
            let directory = vibe_net::px_store::stored_dir(&root, pin);
            if ctx.is_json() {
                return ctx.emit_json(&serde_json::json!({
                    "version": pin.version,
                    "asset": name,
                    "store": directory,
                    "installed": installed,
                }));
            }
            let line = match (&installed, &name) {
                (Some(program), _) => {
                    format!("px {} is installed: `{}`", pin.version, program.display())
                }
                (None, Some(name)) => format!(
                    "px {} is not installed in `{}`; `vibe self px install` reads `{name}` from the release source or drop",
                    pin.version,
                    directory.display()
                ),
                (None, None) => format!("px {} has no build for this platform", pin.version),
            };
            ctx.suspend_progress(|| println!("{line}"));
            Ok(())
        }
        crate::cli::VvmPxCommand::Install(install) => {
            let persister = super::super::make_persister(env, Shell::detect(env.shell.as_deref()))?;
            let downloader = HttpDownloader::for_invocation()?;
            let remote = remote_context(
                ctx,
                env,
                &store,
                &downloader,
                persister.as_ref(),
                "self:px",
                progress,
            )?;
            let (dirs, token) = px_search_dirs(&remote, install.from.as_deref())?;
            let _lock = InstallLock::acquire(&store)?;
            match ensure_px(&remote, &dirs, token.as_ref(), install.force)? {
                PxOutcome::AlreadyInstalled(program) | PxOutcome::Installed { program, .. } => {
                    if ctx.is_json() {
                        return ctx
                            .emit_json(&serde_json::json!({"ok": true, "installed": program}));
                    }
                    ctx.suspend_progress(|| println!("{}", program.display()));
                    Ok(())
                }
                PxOutcome::Unavailable { asset, tried } => bail!(
                    "no release source carries `{asset}` (tried: {}) \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools; \
                      fix: copy `{asset}` into the release drop (`[artifacts] vibe`) or pass `--from <file>`, \
                      then re-run; px is installed into `{}`)",
                    if tried.is_empty() {
                        "none".to_owned()
                    } else {
                        tried.join(", ")
                    },
                    vibe_net::px_store::stored_dir(&root, pin).display()
                ),
                PxOutcome::NoTarget => bail!(
                    "px {} has no build for this platform \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools; \
                      fix: install px yourself and set `[proxy] px_path`)",
                    pin.version
                ),
            }
        }
    }
}

/// After a source install in px mode: place px from the release source
/// when it is there; a failure is reported and never fails the build that
/// succeeded.
pub(in crate::commands::vvm) fn ensure_px_after_source_install(
    ctx: &output::Context,
    env: &VvmEnv,
    progress: &Progress,
) {
    if env.offline || vibe_core::net::route().px().is_none() {
        return;
    }
    let attempt = || -> Result<()> {
        let store = env.store()?;
        let persister = super::super::make_persister(env, Shell::detect(env.shell.as_deref()))?;
        let downloader = HttpDownloader::for_invocation()?;
        let remote = remote_context(
            ctx,
            env,
            &store,
            &downloader,
            persister.as_ref(),
            "self:install",
            progress,
        )?;
        let (dirs, token) = px_search_dirs(&remote, None)?;
        ensure_px(&remote, &dirs, token.as_ref(), false).map(|_| ())
    };
    if let Err(error) = attempt() {
        ctx.diagnostic(&format!("px was not installed: {error:#}"));
    }
}

/// px from a release unpacked by hand: the pinned archive beside the
/// imported binary (or in `tools/px/` there) is placed in the shared tool
/// store, reported on one progress line. Its absence is not an error.
pub(in crate::commands::vvm) fn px_beside(
    store: &crate::commands::vvm::store::VersionStore,
    dir: &Path,
    progress: &Progress,
) -> Result<companions::PxOutcome> {
    px_beside_with(store, vibe_net::px_store::PxPin::embedded(), dir, progress)
}

pub(super) fn px_beside_with(
    store: &crate::commands::vvm::store::VersionStore,
    pin: &vibe_net::px_store::PxPin,
    dir: &Path,
    progress: &Progress,
) -> Result<companions::PxOutcome> {
    use companions::PxOutcome;
    let task = progress.task("Installing px");
    let outcome = companions::install_beside(store, pin, dir);
    match &outcome {
        Ok(PxOutcome::AlreadyInstalled(program)) => {
            task.skip(format!("already in `{}`", program.display()));
        }
        Ok(PxOutcome::Installed { program, from }) => {
            task.detail(format!("`{}` from `{from}`", program.display()));
            task.finish();
        }
        Ok(PxOutcome::Unavailable { asset, .. }) => task.skip(format!(
            "no `{asset}` beside the binary; `[proxy] mode = \"px\"` uses `px_path` or px on PATH"
        )),
        Ok(PxOutcome::NoTarget) => task.skip("px has no build for this platform"),
        Err(_) => task.fail("px could not be installed"),
    }
    outcome
}
