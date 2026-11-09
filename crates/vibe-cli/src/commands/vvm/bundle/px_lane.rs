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

/// What `vibe self px` leaves the invocation in: a status that found no
/// px for a route that needs one is not a success
/// (`PROP-019#companion-tools`), exactly as `vibe self doctor` is not one
/// when it counts an unresolved problem.
#[must_use]
pub(in crate::commands::vvm) enum PxVerdict {
    Ok,
    /// px is needed and nowhere to be found; the string is the progress
    /// line's reason.
    Unusable(String),
}

/// `vibe self px install|status` with its progress line
/// (PROP-019 `#companion-tools`).
///
/// A `status` that found no px for a route that needs one reports first
/// and fails after, the way `vibe self doctor` prints its panel and then
/// refuses with an unresolved problem: the gap is the installation's, and
/// the exit code has to say so.
pub(in crate::commands::vvm) fn run_px_cmd(
    ctx: &output::Context,
    env: &VvmEnv,
    args: crate::cli::VvmPxArgs,
) -> Result<()> {
    let task = ctx.progress().task("px");
    let verdict = px_verb(ctx, env, args, &task.progress());
    match &verdict {
        Ok(PxVerdict::Ok) => task.finish(),
        Ok(PxVerdict::Unusable(reason)) => task.fail(reason.clone()),
        Err(_) => task.fail("operation failed"),
    }
    match verdict? {
        PxVerdict::Ok => Ok(()),
        PxVerdict::Unusable(reason) => Err(anyhow::anyhow!(reason)),
    }
}

fn px_verb(
    ctx: &output::Context,
    env: &VvmEnv,
    args: crate::cli::VvmPxArgs,
    progress: &Progress,
) -> Result<PxVerdict> {
    use companions::PxOutcome;
    let store = env.store()?;
    let pin = vibe_net::px_store::PxPin::embedded();
    let root = companions::store_root(&store);
    match args.command {
        crate::cli::VvmPxCommand::Status => {
            let asset = companions::pinned_asset(pin);
            let installed = companions::installed(&root);
            let name = asset.map(|asset| pin.release_asset_name(asset));
            let directory = vibe_net::px_store::stored_dir(&root, pin);
            // What the route would actually run, by the same lookup every
            // operation uses: `px_path`, then the store, then `tools/px/`
            // beside the binary, then PATH. An empty store is a problem
            // only when px mode has nothing else to run.
            let route = vibe_core::net::route();
            let needed = route.px().is_some();
            let program = route
                .px()
                .and_then(|settings| vibe_net::locate_px(settings).ok());
            let usable = !needed || program.is_some();
            if ctx.is_json() {
                ctx.emit_json(&serde_json::json!({
                    "version": pin.version,
                    "asset": name,
                    "store": directory,
                    "installed": installed.as_ref().map(|found| &found.program),
                    "in_store": installed.is_some(),
                    "needed": needed,
                    "program": program,
                    "source": installed.as_ref().and_then(|found| found.source.clone()),
                    "sha256": installed.as_ref().and_then(|found| found.sha256.clone()),
                    "installed_at": installed
                        .as_ref()
                        .and_then(|found| found.installed_at.clone()),
                    "px_version": installed.as_ref().and_then(|found| found.version.clone()),
                    "ok": usable,
                }))?;
                return Ok(unusable_verdict(usable, pin, &directory, name.as_deref()));
            }
            let line = match (&installed, &name) {
                (Some(found), _) => {
                    format!("px is installed: `{}`", found.program.display())
                }
                (None, Some(name)) => format!(
                    "no px is installed in `{}`; `vibe self px install` reads `{name}` from the release source or drop, and `--from <archive|directory|executable>` takes a px of your own",
                    root.display()
                ),
                (None, None) => format!(
                    "no px is installed in `{}`, and px has no default build for this platform; `vibe self px install --from <archive|directory|executable>` installs one of your own",
                    root.display()
                ),
            };
            // What vibe knows about those exact bytes: where it read them,
            // the SHA-256 it measured, and what the program says it is.
            // Nothing here is a verdict — px is not checked against any
            // digest (`PROP-019#companion-tools`).
            let mut extra = Vec::new();
            if let Some(found) = &installed {
                if let Some(version) = &found.version {
                    extra.push(format!("  reports: {version}"));
                }
                if let Some(source) = &found.source {
                    extra.push(format!("  read from: {source}"));
                }
                if let Some(sha256) = &found.sha256 {
                    extra.push(format!("  sha256: {sha256}"));
                }
                if let Some(at) = &found.installed_at {
                    extra.push(format!("  installed: {at}"));
                }
            }
            // The px in use when it is not the store's, so a report of an
            // empty store is not read as "no px at all".
            match (&installed, &program) {
                (None, Some(program)) => extra.push(format!(
                    "`[proxy] mode = \"px\"` runs `{}` instead",
                    program.display()
                )),
                (None, None) if needed => extra.push(
                    "`[proxy] mode = \"px\"` has no px to run: the next network command will fail"
                        .to_owned(),
                ),
                _ => {}
            }
            ctx.suspend_progress(|| {
                println!("{line}");
                for row in &extra {
                    println!("{row}");
                }
            });
            Ok(unusable_verdict(usable, pin, &directory, name.as_deref()))
        }
        // A px the operator names is installed from their own files, of
        // any version and on any target, without a digest in the way
        // (`PROP-019#companion-tools`); only a px vibe DOWNLOADS goes
        // through the release source.
        crate::cli::VvmPxCommand::Install(install) if install.from.is_some() => {
            let from = install.from.as_deref().unwrap_or(Path::new("."));
            let _lock = InstallLock::acquire(&store)?;
            let task = progress.task("Installing px");
            let outcome = companions::install_local(&store, pin, from, install.force);
            match &outcome {
                Ok(PxOutcome::AlreadyInstalled(program)) => {
                    task.skip(format!("already in `{}`", program.display()));
                }
                Ok(PxOutcome::Installed { program, from }) => {
                    task.detail(format!("`{}` from `{from}`", program.display()));
                    task.finish();
                }
                Ok(_) => task.skip("nothing to install"),
                Err(_) => task.fail("px could not be installed"),
            }
            let program = match outcome? {
                PxOutcome::AlreadyInstalled(program) | PxOutcome::Installed { program, .. } => {
                    program
                }
                PxOutcome::Unavailable { asset, .. } => bail!(
                    "`{}` carries no `{asset}` and no px program \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools; \
                      fix: name the px archive, the directory holding px, or the px program)",
                    from.display()
                ),
                PxOutcome::NoTarget => bail!(
                    "`{}` holds no px to install \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools; \
                      fix: name the px archive, the directory holding px, or the px program)",
                    from.display()
                ),
            };
            if ctx.is_json() {
                let found = vibe_net::px_store::stored_record(&root, &program);
                ctx.emit_json(&serde_json::json!({
                    "ok": true,
                    "installed": program,
                    "source": found.as_ref().and_then(|found| found.source.clone()),
                    "sha256": found.as_ref().and_then(|found| found.sha256.clone()),
                    "px_version": found.as_ref().and_then(|found| found.version.clone()),
                }))?;
                return Ok(PxVerdict::Ok);
            }
            ctx.suspend_progress(|| println!("{}", program.display()));
            Ok(PxVerdict::Ok)
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
                        ctx.emit_json(&serde_json::json!({"ok": true, "installed": program}))?;
                        return Ok(PxVerdict::Ok);
                    }
                    ctx.suspend_progress(|| println!("{}", program.display()));
                    Ok(PxVerdict::Ok)
                }
                PxOutcome::Unavailable { asset, tried } => bail!(
                    "no release source carries `{asset}` (tried: {}) \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools; \
                      fix: copy `{asset}` into the release drop (`[artifacts] vibe`) or pass \
                      `--from <archive|directory|executable>` with a px of your own, \
                      then re-run; px is installed into `{}`)",
                    if tried.is_empty() {
                        "none".to_owned()
                    } else {
                        tried.join(", ")
                    },
                    vibe_net::px_store::stored_dir(&root, pin).display()
                ),
                PxOutcome::NoTarget => bail!(
                    "px has no default build for this platform \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools; \
                      fix: `vibe self px install --from <archive|directory|executable>` with a px \
                      of your own, or set `[proxy] px_path`)"
                ),
            }
        }
    }
}

/// The verdict of a `status` that has just reported what it found: a px
/// mode with no px anywhere is the one state the verb refuses, and the
/// refusal names the asset and the repair.
fn unusable_verdict(
    usable: bool,
    pin: &vibe_net::px_store::PxPin,
    directory: &Path,
    asset: Option<&str>,
) -> PxVerdict {
    if usable {
        return PxVerdict::Ok;
    }
    PxVerdict::Unusable(match asset {
        Some(asset) => format!(
            "px {} is not installed in `{}`; `[proxy] mode = \"px\"` needs a px to run \
             (violates spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools; \
              fix: put `{asset}` beside `vibe` (or in `tools/px/` there) and run \
              `vibe self px install`, or `vibe self px install --from \
              <archive|directory|executable>` with a px of your own, or name one with \
              `[proxy] px_path`)",
            pin.version,
            directory.display()
        ),
        None => format!(
            "no px is installed in `{}`, and px has no default build for this platform, which \
             `[proxy] mode = \"px\"` needs \
             (violates spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools; \
              fix: `vibe self px install --from <archive|directory|executable>` with a px of your \
              own, or name one with `[proxy] px_path`)",
            directory.parent().unwrap_or(directory).display()
        ),
    })
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

/// Where the pinned px of `store` lives, named even when it is not there.
pub(in crate::commands::vvm) fn px_store_dir(
    store: &crate::commands::vvm::store::VersionStore,
    pin: &vibe_net::px_store::PxPin,
) -> std::path::PathBuf {
    vibe_net::px_store::stored_dir(&companions::store_root(store), pin)
}

/// The companion asset file name this platform needs, when the pin has a
/// build for it.
pub(in crate::commands::vvm) fn px_asset_name(pin: &vibe_net::px_store::PxPin) -> Option<String> {
    companions::pinned_asset(pin).map(|asset| pin.release_asset_name(asset))
}

/// What `vibe self apply`'s px step found or did
/// (`PROP-019#companion-tools`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::commands::vvm) enum PxPresence {
    /// The effective route does not run the managed px.
    NotNeeded,
    /// The pinned px was already in the store.
    Present(std::path::PathBuf),
    /// It was placed in the store from `from`.
    Placed {
        program: std::path::PathBuf,
        from: String,
    },
    /// A dry run: it would be placed from `from`.
    WouldPlace { from: std::path::PathBuf },
    /// px is needed and is not in the store.
    Absent,
    /// The pin has no px build for this platform.
    NoTarget,
}

/// Make sure the px a just-applied profile selects is in `store` — the
/// store under the install root THAT profile chooses, not the process's
/// (`PROP-019#companion-tools`, `#install-root`).
///
/// An installation whose px is already there is left alone, so a second
/// `vibe self apply` changes nothing. Otherwise px is read from `dir` —
/// the directory holding the running executable, which on a closed network
/// is the unpacked release and the only place the companion archive can
/// be — exactly as an import reads it, on one progress line, with its
/// provenance recorded and nothing refused over a digest. `attempt = false` only observes: an import performed in
/// the same invocation has just run this lane against the same directory
/// and reported on its own line.
pub(in crate::commands::vvm) fn ensure_px_for_apply(
    store: &crate::commands::vvm::store::VersionStore,
    pin: &vibe_net::px_store::PxPin,
    dir: Option<&Path>,
    needed: bool,
    dry_run: bool,
    attempt: bool,
    progress: &Progress,
) -> Result<PxPresence> {
    use companions::PxOutcome;
    if !needed {
        return Ok(PxPresence::NotNeeded);
    }
    let root = companions::store_root(store);
    // A px already installed is a px already installed, whichever version
    // directory holds it and wherever the operator got it from.
    if let Some(found) = companions::installed(&root) {
        return Ok(PxPresence::Present(found.program));
    }
    let Some(asset) = companions::pinned_asset(pin) else {
        return Ok(PxPresence::NoTarget);
    };
    let Some(dir) = dir else {
        return Ok(PxPresence::Absent);
    };
    if dry_run {
        // A dry run reads the directory and writes nothing.
        return Ok(match companions::archive_beside(pin, asset, dir) {
            Some(from) => PxPresence::WouldPlace { from },
            None => PxPresence::Absent,
        });
    }
    if !attempt {
        return Ok(PxPresence::Absent);
    }
    Ok(match px_beside_with(store, pin, dir, progress)? {
        PxOutcome::AlreadyInstalled(program) => PxPresence::Present(program),
        PxOutcome::Installed { program, from } => PxPresence::Placed { program, from },
        PxOutcome::Unavailable { .. } => PxPresence::Absent,
        PxOutcome::NoTarget => PxPresence::NoTarget,
    })
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
