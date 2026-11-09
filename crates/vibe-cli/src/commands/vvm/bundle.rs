//! Verified binary-bundle bootstrap and mutable-release refresh.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#provenance");

use std::path::Path;

use anyhow::{Context, Result, bail};
use vibe_core::progress::Progress;
use vibe_publish::release_manifest::{
    AggregateDistributionManifest, DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME,
    DISTRIBUTION_AGGREGATE_MANIFEST_MAX_BYTES, DISTRIBUTION_BOOTSTRAP_MAX_BYTES,
    PlatformDistributionFragment,
};

use super::env::{EnvPersister, Shell};
use super::install::InstallLock;
use super::model::{InstallRecord, Kind, Origin};
use super::{VvmEnv, output, provenance, store::VersionStore};
use crate::cli::{ForcedKind, VvmBootstrapArgs, VvmInstallArgs, VvmReinstallArgs, VvmUpdateArgs};

#[path = "bundle/api.rs"]
pub(crate) mod api;
#[path = "bundle/archive.rs"]
pub(super) mod archive; // `self import` installs an unpacked release through it
mod companions;
pub(super) mod px_lane;
pub(super) use px_lane::{ensure_px_after_source_install as px_after_source, run_px_cmd as run_px};
#[path = "bundle/lane.rs"]
mod lane;
use lane::{
    ReleaseManifest, located_distribution, read_release, refuse_remote_offline,
    report_withdrawn_release,
};
#[path = "bundle/download.rs"]
mod download;
#[path = "bundle/release_source.rs"]
pub(crate) mod release_source;
#[path = "bundle/releases.rs"]
mod releases;
#[path = "bundle/report.rs"]
mod report;
#[path = "bundle/selection.rs"]
mod selection;
pub(super) use download::fetch_document;
#[cfg(test)]
use download::{CONNECT_TIMEOUT, TOTAL_TIMEOUT, copy_download, write_download};
use download::{
    DownloadCleanup, Downloader, HttpDownloader, cache_busted, download_path, safe_url,
};
pub(super) use release_source_view::{run_release_source_cmd, run_releases_cmd};
#[path = "bundle/release_source_view.rs"]
mod release_source_view;
use releases::{Newest, ReleaseDir, Releases};
use report::{ActivationReport, emit_outcome};
pub(super) use report::{report_if_current, report_source_if_current};
pub(in crate::commands::vvm) use selection::current_target;
use selection::{read_aggregate, select_platform, validated_release_base};
use vibe_publish::token::Token;

pub(super) fn installed_bundle_intact(store: &VersionStore, record: &InstallRecord) -> bool {
    archive::installed_bundle_intact(store, record)
}

struct RemoteContext<'a> {
    /// The release source this invocation resolved (`PROP-019#release-sources`).
    source: release_source::EffectiveReleaseSource,
    /// The Locator its row lives in.
    locator: &'a vibe_core::remap::Locator,
    ctx: &'a output::Context,
    env: &'a VvmEnv,
    store: &'a VersionStore,
    downloader: &'a dyn Downloader,
    persister: &'a dyn EnvPersister,
    command: &'a str,
    progress: &'a Progress,
}

fn activate_install(
    store: &VersionStore,
    outcome: &archive::InstallOutcome,
    persister: &dyn EnvPersister,
    path_on_current_process: bool,
) -> Result<ActivationReport> {
    let activated = super::env::activate_instance(store, &outcome.home, persister)?;
    Ok(ActivationReport {
        path_on_current_process,
        durable_path_changed: activated.path == super::env::Persisted::Changed,
        advisory_home_warning: activated.advisory_home_warning,
    })
}

pub(super) fn run_bootstrap_cmd(
    ctx: &output::Context,
    env: &VvmEnv,
    args: VvmBootstrapArgs,
    progress: &Progress,
) -> Result<()> {
    // A site profile is validated before anything is installed
    // (`PROP-019#site-profile`): a closed-network machine is set up in one
    // step, or not touched at all.
    let site = args
        .site
        .as_deref()
        .map(|source| super::site::prepare_profile(ctx, env, source))
        .transpose()?;
    let executable = std::env::current_exe().context("locating the bootstrap executable")?;
    let persister = super::make_persister(env, Shell::detect(env.shell.as_deref()))?;
    let downloader = HttpDownloader::for_invocation()?;
    bootstrap_with_downloader(
        ctx,
        env,
        &args,
        &downloader,
        &executable,
        persister.as_ref(),
        progress,
    )?;
    match site {
        Some(prepared) => super::site::apply_after_bootstrap(ctx, env, &prepared),
        None => Ok(()),
    }
}

fn bootstrap_with_downloader(
    ctx: &output::Context,
    env: &VvmEnv,
    args: &VvmBootstrapArgs,
    downloader: &dyn Downloader,
    bootstrap_executable: &Path,
    persister: &dyn EnvPersister,
    progress: &Progress,
) -> Result<()> {
    let store = env.store()?;
    let aggregate = read_aggregate(&args.manifest)?;
    let platform = select_platform(&aggregate, &args.version, current_target()?)?;
    if platform.bootstrap.size > DISTRIBUTION_BOOTSTRAP_MAX_BYTES {
        bail!("bootstrap declaration exceeds distribution policy");
    }
    archive::verify_file_asset(bootstrap_executable, &platform.bootstrap)?;
    let (declared_base, from_remap) = located_distribution(&args.release_base)?;
    let release_base = validated_release_base(&declared_base, &platform.tag, from_remap)?;
    install_selected(
        &RemoteContext {
            source: release_source::active()?,
            locator: vibe_core::remap::active(),
            ctx,
            env,
            store: &store,
            downloader,
            persister,
            command: "self:bootstrap",
            progress,
        },
        platform,
        &ReleaseDir::Base(release_base),
        None,
        args.force,
    )
    .map(|_| ())
}

/// `self update` (PROP-019 `##CMD-UPDATE`): a binary execution moves to the
/// newest published release; a source execution rebuilds its checkout at
/// `latest`, exactly as before.
pub(super) fn run_update_cmd(
    ctx: &output::Context,
    env: &VvmEnv,
    args: VvmUpdateArgs,
    progress: &Progress,
) -> Result<()> {
    let store = env.store()?;
    if let Some(record) = running_binary_record(&store)? {
        return install_newest_release(
            ctx,
            env,
            &store,
            &record,
            args.force,
            "self:update",
            progress,
        );
    }
    rebuild_latest(
        ctx,
        env,
        args.profile,
        args.release,
        args.force,
        "self:update",
        progress,
    )
}

/// `self reinstall` (PROP-019 `##CMD-REINSTALL`): refetch and reinstall the
/// version that is running — a binary execution redownloads its own release,
/// a source execution rebuilds its checkout. Always a fresh immutable `#N`:
/// that IS the verb, so there is no flag to ask for it.
pub(super) fn run_reinstall_cmd(
    ctx: &output::Context,
    env: &VvmEnv,
    args: VvmReinstallArgs,
    progress: &Progress,
) -> Result<()> {
    let store = env.store()?;
    if let Some(record) = running_binary_record(&store)? {
        binary_release_version(&record)?;
        let persister = super::make_persister(env, Shell::detect(env.shell.as_deref()))?;
        let downloader = HttpDownloader::for_invocation()?;
        return install_release_version(
            &remote_context(
                ctx,
                env,
                &store,
                &downloader,
                persister.as_ref(),
                "self:reinstall",
                progress,
            )?,
            &record.id,
            true,
        )
        .map(|_| ());
    }
    rebuild_latest(
        ctx,
        env,
        args.profile,
        args.release,
        true,
        "self:reinstall",
        progress,
    )
}

/// The running managed BINARY installation, when this execution is one. A
/// source or worktree execution has no release to fetch and returns `None`.
fn running_binary_record(store: &VersionStore) -> Result<Option<InstallRecord>> {
    Ok(provenance::running_record(store)?.filter(|record| record.origin == Origin::Binary))
}

/// The logical release version a binary execution can refresh or advance.
/// An instance inventoried under a non-release identity has no release
/// lane at all, and says so instead of guessing a URL.
fn binary_release_version(record: &InstallRecord) -> Result<semver::Version> {
    if record.kind == Kind::Tag
        && let Ok(version) = semver::Version::parse(&record.id)
    {
        return Ok(version);
    }
    bail!(
        "binary instance `{}` has no semantic release version; use `vibe self install VERSION`",
        record.selector()
    )
}

/// The source lane's refresh, shared by `update` and `reinstall`: rebuild the
/// running checkout at `latest`. `command` is the verb that asked, carried
/// through so the report and any refusal name it rather than the install it
/// delegates to.
fn rebuild_latest(
    ctx: &output::Context,
    env: &VvmEnv,
    profile: Option<String>,
    release: bool,
    force: bool,
    command: &str,
    progress: &Progress,
) -> Result<()> {
    super::run_install_cmd(
        ctx,
        env,
        VvmInstallArgs {
            selector: "latest".to_string(),
            kind: ForcedKind {
                tag: false,
                branch: false,
                commit: false,
            },
            profile,
            release,
            mirror: None,
            force,
        },
        command,
        progress,
    )
}

/// Move a binary installation to the NEWEST published release — the single
/// function behind both `self update` and `self install stable` (PROP-019
/// `##CMD-UPDATE`, `##SEL-STABLE`).
pub(super) fn install_newest_release(
    ctx: &output::Context,
    env: &VvmEnv,
    store: &VersionStore,
    record: &InstallRecord,
    force: bool,
    command: &str,
    progress: &Progress,
) -> Result<()> {
    let persister = super::make_persister(env, Shell::detect(env.shell.as_deref()))?;
    let downloader = HttpDownloader::for_invocation()?;
    move_to_newest_release(
        &remote_context(
            ctx,
            env,
            store,
            &downloader,
            persister.as_ref(),
            command,
            progress,
        )?,
        record,
        force,
    )
}

/// Three outcomes, one manifest read. A newer release is installed and
/// activated by the same verified path an explicit version number takes.
/// The SAME version still requires checking the bundle digest: a release can
/// be rebuilt under its own number. An intact reused active instance needs no
/// activation. An OLDER newest release means one was withdrawn on the far
/// side; `update` never walks a machine backwards on its own.
fn move_to_newest_release(
    remote: &RemoteContext<'_>,
    record: &InstallRecord,
    force: bool,
) -> Result<()> {
    let installed = binary_release_version(record)?;
    let api = api::ApiClient::new(remote.downloader.admission())?;
    let releases = Releases::new(&remote.source, &api, remote.locator)?;
    let token = releases.token.clone();
    let (manifest, dir, newest) = match releases.newest()? {
        Newest::Alias(newest_manifest) => {
            let aggregate = fetch_aggregate(remote, &newest_manifest, None)
                .context("learning the newest published release")?;
            let newest = semver::Version::parse(&aggregate.version).with_context(|| {
                format!("reading newest release version `{}`", aggregate.version)
            })?;
            let dir = releases.dir(&newest)?;
            (ReleaseManifest::Aggregate(aggregate), dir, newest)
        }
        Newest::Listed(newest) => {
            let dir = releases.dir(&newest)?;
            let (manifest, dir) = read_release(remote, &dir, &newest, token.as_ref())
                .context("learning the newest published release")?;
            (manifest, dir, newest)
        }
    };
    let selection = remote.progress.task("Selecting release");
    selection.detail(format!("installed {installed}; newest {newest}"));
    selection.finish();
    if newest < installed {
        return report_withdrawn_release(remote, record, &installed, &newest);
    }
    let platform = manifest.platform(&newest.to_string(), current_target()?)?;
    if newest > installed {
        remote.ctx.hint(&format!("updating {installed} → {newest}"));
        install_selected(remote, platform, &dir, token.as_ref(), force)?;
        return Ok(());
    }
    // Said before the install, because its trailer is the last block: it
    // reads `reused` when nothing was rebuilt and `installed` when it was.
    remote.ctx.hint(&format!(
        "newest release is still {newest}; reinstalled only if it was rebuilt since this install"
    ));
    install_selected(remote, platform, &dir, token.as_ref(), force)?;
    Ok(())
}

/// Install one named release version from its own release directory —
/// `self install X.Y.Z` and `self reinstall`. Returns whether an existing
/// immutable instance was reused.
fn install_release_version(remote: &RemoteContext<'_>, version: &str, force: bool) -> Result<bool> {
    let parsed = semver::Version::parse(version)
        .with_context(|| format!("invalid release version `{version}`"))?;
    let api = api::ApiClient::new(remote.downloader.admission())?;
    let releases = Releases::new(&remote.source, &api, remote.locator)?;
    let token = releases.token.clone();
    let dir = releases.dir(&parsed)?;
    let (manifest, dir) = read_release(remote, &dir, &parsed, token.as_ref())?;
    let platform = manifest.platform(version, current_target()?)?;
    install_selected(remote, platform, &dir, token.as_ref(), force)
}

pub(super) fn install_binary_version(
    ctx: &output::Context,
    env: &VvmEnv,
    store: &VersionStore,
    version: &str,
    force: bool,
    progress: &Progress,
) -> Result<()> {
    let persister = super::make_persister(env, Shell::detect(env.shell.as_deref()))?;
    let downloader = HttpDownloader::for_invocation()?;
    install_release_version(
        &remote_context(
            ctx,
            env,
            store,
            &downloader,
            persister.as_ref(),
            "self:install",
            progress,
        )?,
        version,
        force,
    )
    .map(|_| ())
}

/// The context every release-lane verb runs in: the invocation's
/// downloader, the resolved store, and the command label its report carries.
fn remote_context<'a>(
    ctx: &'a output::Context,
    env: &'a VvmEnv,
    store: &'a VersionStore,
    downloader: &'a dyn Downloader,
    persister: &'a dyn EnvPersister,
    command: &'a str,
    progress: &'a Progress,
) -> Result<RemoteContext<'a>> {
    Ok(RemoteContext {
        source: release_source::active()?,
        locator: vibe_core::remap::active(),
        ctx,
        env,
        store,
        downloader,
        persister,
        command,
        progress,
    })
}

/// Fetch and parse one aggregate manifest, cache-busted because every release
/// URL a running installation reads is mutable.
fn fetch_aggregate(
    remote: &RemoteContext<'_>,
    url: &str,
    token: Option<&Token>,
) -> Result<AggregateDistributionManifest> {
    // The release lane's FIRST request, and so the offline posture's first
    // and usually only refusal: nothing has been written, locked or
    // allocated yet. It is also why an already-installed release is no
    // rescue under the posture — the only thing that could recognise one is
    // this manifest, and reading it is the very act being refused.
    refuse_remote_offline(remote, url)?;
    let task = remote.progress.task("Fetching release metadata");
    task.detail(format!("GET {}", safe_url(url)));
    task.set_progress(0, None, "bytes");
    let manifest_path = download_path(remote.store, DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME);
    if let Err(error) = remote.store.guard_mutation_path(&manifest_path) {
        task.fail("release metadata destination was rejected");
        return Err(error.into());
    }
    if let Err(error) = remote.downloader.download_authorized(
        &cache_busted(url),
        &manifest_path,
        DISTRIBUTION_AGGREGATE_MANIFEST_MAX_BYTES,
        None,
        &task,
        token,
    ) {
        task.fail("release metadata download failed");
        return Err(error);
    }
    let _cleanup = DownloadCleanup(manifest_path.clone());
    let result = read_aggregate(&manifest_path);
    if result.is_ok() {
        task.finish();
    } else {
        task.fail("release metadata was invalid");
    }
    result
}

/// Install one selected platform bundle, returning whether an existing
/// immutable instance was reused instead of a new one allocated.
///
/// The manifest already names the bundle's digest, and an instance already
/// installed from those exact bytes is verified in place — so an unchanged
/// release is recognised BEFORE the bundle is fetched, and costs no download
/// at all. Every actual install still walks the full verified path: fetch,
/// hash, open, cross-check the embedded manifest, extract.
fn install_selected(
    remote: &RemoteContext<'_>,
    platform: &PlatformDistributionFragment,
    dir: &ReleaseDir,
    token: Option<&Token>,
    force: bool,
) -> Result<bool> {
    let existing = remote.progress.task("Checking installed generation");
    if !force
        && let Some(outcome) = archive::installed_from_published_bundle_observed(
            remote.store,
            &platform.bundle,
            &platform.asset,
            &existing.progress(),
        )?
    {
        existing.detail(format!("reusing intact {}", outcome.record.selector()));
        existing.finish();
        if remote.command == "self:update"
            && report_if_current(
                remote.ctx,
                remote.env,
                remote.store,
                &outcome.record,
                remote.command,
            )?
        {
            return Ok(true);
        }
        let _lock = InstallLock::acquire(remote.store)?;
        return finish_install(remote, &outcome);
    }
    if force {
        existing.skip("fresh generation requested");
    } else {
        existing.detail("no intact generation matches the published bundle");
        existing.finish();
    }
    let bundle_url = dir.asset(&platform.asset.name)?;
    // The bundle is the lane's only other request. Getting here means the
    // release is NOT already held from these exact bytes, so there is
    // nothing local left to satisfy the verb with and an offline run says
    // so — before the download path is even allocated.
    refuse_remote_offline(remote, &bundle_url)?;
    let download = remote.progress.task("Downloading release bundle");
    download.detail(format!("GET {}", safe_url(&bundle_url)));
    download.set_progress(0, Some(platform.asset.size), "bytes");
    let bundle_path = download_path(remote.store, &platform.asset.name);
    if let Err(error) = remote.store.guard_mutation_path(&bundle_path) {
        download.fail("release bundle destination was rejected");
        return Err(error.into());
    }
    if let Err(error) = remote.downloader.download_authorized(
        &cache_busted(&bundle_url),
        &bundle_path,
        platform.asset.size,
        Some(platform.asset.size),
        &download,
        token,
    ) {
        download.fail("release bundle download failed");
        return Err(error);
    }
    download.finish();
    let _cleanup = DownloadCleanup(bundle_path.clone());
    let _lock = InstallLock::acquire(remote.store)?;
    let install = remote
        .progress
        .task("Verifying and extracting release bundle");
    let outcome = match archive::install_bundle_observed(
        remote.store,
        &bundle_path,
        &platform.bundle,
        &platform.asset,
        force,
        &install.progress(),
    ) {
        Ok(outcome) => {
            install.finish();
            outcome
        }
        Err(error) => {
            install.fail("release bundle verification or extraction failed");
            return Err(error);
        }
    };
    px_lane::ensure_px(remote, &px_lane::px_dirs(dir), token, false)?; // PROP-019 `#companion-tools`
    finish_install(remote, &outcome)
}

/// Activate the installed-or-reused instance and report it.
fn finish_install(remote: &RemoteContext<'_>, outcome: &archive::InstallOutcome) -> Result<bool> {
    let task = remote.progress.task("Activating installed version");
    let activation = match activate_install(
        remote.store,
        outcome,
        remote.persister,
        super::path_has_dir(remote.env.path_var.as_deref(), &remote.store.shim_dir()),
    ) {
        Ok(activation) => {
            task.finish();
            activation
        }
        Err(error) => {
            task.fail("activation failed");
            return Err(error);
        }
    };
    emit_outcome(remote.ctx, remote.command, outcome, &activation)?;
    Ok(outcome.reused)
}

#[cfg(test)]
#[path = "bundle/tests.rs"]
mod tests;
