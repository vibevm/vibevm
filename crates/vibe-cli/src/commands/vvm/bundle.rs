//! Verified binary-bundle bootstrap and mutable-release refresh.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#provenance");

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use vibe_publish::release_manifest::{
    AggregateDistributionManifest, DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME,
    DISTRIBUTION_AGGREGATE_MANIFEST_MAX_BYTES, DISTRIBUTION_BOOTSTRAP_MAX_BYTES,
    PlatformDistributionFragment,
};

use super::env::{EnvPersister, Shell};
use super::install::InstallLock;
use super::model::{InstallRecord, Kind, Origin};
use super::{
    VvmEnv, output, provenance,
    store::{VersionStore, open_regular_no_follow},
};
use crate::cli::{ForcedKind, VvmBootstrapArgs, VvmInstallArgs, VvmReinstallArgs, VvmUpdateArgs};

#[path = "bundle/archive.rs"]
mod archive;

pub(super) fn installed_bundle_intact(store: &VersionStore, record: &InstallRecord) -> bool {
    archive::installed_bundle_intact(store, record)
}

const GITHUB_RELEASE_ROOT: &str = "https://github.com/vibevm/vibevm/releases/download";
/// GitHub's stable alias for whatever release is newest right now. It is the
/// one address from which a running installation can learn that a version
/// beyond its own exists — every other release URL names a version already
/// known (PROP-019 `##CMD-UPDATE`).
const GITHUB_NEWEST_RELEASE_MANIFEST: &str =
    "https://github.com/vibevm/vibevm/releases/latest/download/DISTRIBUTIONS.json";
static REQUEST_NONCE: AtomicU64 = AtomicU64::new(1);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
const TOTAL_TIMEOUT: Duration = Duration::from_secs(30 * 60);

#[derive(Debug)]
struct ActivationReport {
    path_on_current_process: bool,
    durable_path_changed: bool,
    advisory_home_warning: Option<String>,
}

struct RemoteContext<'a> {
    ctx: &'a output::Context,
    env: &'a VvmEnv,
    store: &'a VersionStore,
    downloader: &'a dyn Downloader,
    persister: &'a dyn EnvPersister,
    command: &'a str,
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
) -> Result<()> {
    let executable = std::env::current_exe().context("locating the bootstrap executable")?;
    let persister = super::make_persister(env, Shell::detect(env.shell.as_deref()))?;
    bootstrap_with_downloader(
        ctx,
        env,
        &args,
        &HttpDownloader,
        &executable,
        persister.as_ref(),
    )
}

fn bootstrap_with_downloader(
    ctx: &output::Context,
    env: &VvmEnv,
    args: &VvmBootstrapArgs,
    downloader: &dyn Downloader,
    bootstrap_executable: &Path,
    persister: &dyn EnvPersister,
) -> Result<()> {
    let store = env.store()?;
    let aggregate = read_aggregate(&args.manifest)?;
    let platform = select_platform(&aggregate, &args.version, current_target()?)?;
    if platform.bootstrap.size > DISTRIBUTION_BOOTSTRAP_MAX_BYTES {
        bail!("bootstrap declaration exceeds distribution policy");
    }
    archive::verify_file_asset(bootstrap_executable, &platform.bootstrap)?;
    let release_base = validated_release_base(&args.release_base, &platform.tag)?;
    install_selected(
        &RemoteContext {
            ctx,
            env,
            store: &store,
            downloader,
            persister,
            command: "self:bootstrap",
        },
        platform,
        &release_base,
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
) -> Result<()> {
    let store = env.store()?;
    if let Some(record) = running_binary_record(&store)? {
        return install_newest_release(ctx, env, &store, &record, args.force, "self:update");
    }
    rebuild_latest(
        ctx,
        env,
        args.profile,
        args.release,
        args.force,
        "self:update",
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
) -> Result<()> {
    let store = env.store()?;
    if let Some(record) = running_binary_record(&store)? {
        binary_release_version(&record)?;
        let persister = super::make_persister(env, Shell::detect(env.shell.as_deref()))?;
        return install_release_version(
            &remote_context(ctx, env, &store, persister.as_ref(), "self:reinstall"),
            &record.id,
            true,
        )
        .map(|_| ());
    }
    rebuild_latest(ctx, env, args.profile, args.release, true, "self:reinstall")
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
) -> Result<()> {
    let persister = super::make_persister(env, Shell::detect(env.shell.as_deref()))?;
    move_to_newest_release(
        &remote_context(ctx, env, store, persister.as_ref(), command),
        record,
        force,
    )
}

/// Three outcomes, one manifest read. A newer release is installed and
/// activated by the same verified path an explicit version number takes.
/// The SAME version is not a no-op: a release can be rebuilt under its own
/// number, so the manifest's bundle digest decides between a fresh instance
/// and a reuse. An OLDER newest release means one was withdrawn on the far
/// side; `update` never walks a machine backwards on its own.
fn move_to_newest_release(
    remote: &RemoteContext<'_>,
    record: &InstallRecord,
    force: bool,
) -> Result<()> {
    let installed = binary_release_version(record)?;
    let aggregate = fetch_aggregate(remote, GITHUB_NEWEST_RELEASE_MANIFEST)
        .context("learning the newest published release")?;
    let newest = semver::Version::parse(&aggregate.version)
        .with_context(|| format!("reading newest release version `{}`", aggregate.version))?;
    if newest < installed {
        return report_withdrawn_release(remote, record, &installed, &newest);
    }
    let platform = select_platform(&aggregate, &newest.to_string(), current_target()?)?;
    let release_base = format!("{GITHUB_RELEASE_ROOT}/v{newest}");
    if newest > installed {
        remote
            .ctx
            .summary(&format!("updating {installed} → {newest}"));
        install_selected(remote, platform, &release_base, force)?;
        return Ok(());
    }
    let reused = install_selected(remote, platform, &release_base, force)?;
    remote.ctx.summary(&if reused {
        format!("newest release is {newest} — already installed")
    } else {
        format!("newest release is still {newest}, rebuilt since this install — reinstalled")
    });
    Ok(())
}

/// The newest release is older than the installed one: nothing is fetched and
/// nothing moves. Human output is the one line that explains it; a `--json`
/// caller still gets the ordinary envelope, reporting the running instance as
/// the reused, active one — because that is exactly what it now is.
fn report_withdrawn_release(
    remote: &RemoteContext<'_>,
    record: &InstallRecord,
    installed: &semver::Version,
    newest: &semver::Version,
) -> Result<()> {
    remote.ctx.summary(&format!(
        "newest release is {newest}, older than the installed {installed} — staying on \
         {installed}; `vibe self install {newest}` goes back deliberately"
    ));
    if !remote.ctx.is_json() {
        return Ok(());
    }
    let outcome = archive::InstallOutcome {
        record: record.clone(),
        home: remote
            .store
            .instance_dir(&record.version_id(), record.instance),
        reused: true,
    };
    emit_outcome(
        remote.ctx,
        remote.command,
        &outcome,
        &ActivationReport {
            path_on_current_process: super::path_has_dir(
                remote.env.path_var.as_deref(),
                &remote.store.shim_dir(),
            ),
            durable_path_changed: false,
            advisory_home_warning: None,
        },
    )
}

/// Install one named release version from its own release directory —
/// `self install X.Y.Z` and `self reinstall`. Returns whether an existing
/// immutable instance was reused.
fn install_release_version(remote: &RemoteContext<'_>, version: &str, force: bool) -> Result<bool> {
    let release_base = format!("{GITHUB_RELEASE_ROOT}/v{version}");
    let aggregate = fetch_aggregate(
        remote,
        &format!("{release_base}/{DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME}"),
    )?;
    let platform = select_platform(&aggregate, version, current_target()?)?;
    install_selected(remote, platform, &release_base, force)
}

pub(super) fn install_binary_version(
    ctx: &output::Context,
    env: &VvmEnv,
    store: &VersionStore,
    version: &str,
    force: bool,
) -> Result<()> {
    let persister = super::make_persister(env, Shell::detect(env.shell.as_deref()))?;
    install_release_version(
        &remote_context(ctx, env, store, persister.as_ref(), "self:install"),
        version,
        force,
    )
    .map(|_| ())
}

/// The context every release-lane verb runs in: the real HTTP downloader, the
/// resolved store, and the command label its report carries.
fn remote_context<'a>(
    ctx: &'a output::Context,
    env: &'a VvmEnv,
    store: &'a VersionStore,
    persister: &'a dyn EnvPersister,
    command: &'a str,
) -> RemoteContext<'a> {
    RemoteContext {
        ctx,
        env,
        store,
        downloader: &HttpDownloader,
        persister,
        command,
    }
}

/// Fetch and parse one aggregate manifest, cache-busted because every release
/// URL a running installation reads is mutable.
fn fetch_aggregate(remote: &RemoteContext<'_>, url: &str) -> Result<AggregateDistributionManifest> {
    // The release lane's FIRST request, and so the offline posture's first
    // and usually only refusal: nothing has been written, locked or
    // allocated yet. It is also why an already-installed release is no
    // rescue under the posture — the only thing that could recognise one is
    // this manifest, and reading it is the very act being refused.
    remote.env.refuse_offline(remote.command, url)?;
    let manifest_path = download_path(remote.store, DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME);
    remote.store.guard_mutation_path(&manifest_path)?;
    remote.downloader.download(
        &cache_busted(url),
        &manifest_path,
        DISTRIBUTION_AGGREGATE_MANIFEST_MAX_BYTES,
    )?;
    let _cleanup = DownloadCleanup(manifest_path.clone());
    read_aggregate(&manifest_path)
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
    release_base: &str,
    force: bool,
) -> Result<bool> {
    if !force
        && let Some(outcome) = archive::installed_from_published_bundle(
            remote.store,
            &platform.bundle,
            &platform.asset,
        )?
    {
        let _lock = InstallLock::acquire(remote.store)?;
        return finish_install(remote, &outcome);
    }
    let bundle_url = format!(
        "{}/{}",
        release_base.trim_end_matches('/'),
        platform.asset.name
    );
    // The bundle is the lane's only other request. Getting here means the
    // release is NOT already held from these exact bytes, so there is
    // nothing local left to satisfy the verb with and an offline run says
    // so — before the download path is even allocated.
    remote.env.refuse_offline(remote.command, &bundle_url)?;
    let bundle_path = download_path(remote.store, &platform.asset.name);
    remote.store.guard_mutation_path(&bundle_path)?;
    remote.downloader.download(
        &cache_busted(&bundle_url),
        &bundle_path,
        platform.asset.size,
    )?;
    let _cleanup = DownloadCleanup(bundle_path.clone());
    let _lock = InstallLock::acquire(remote.store)?;
    let outcome = archive::install_bundle(
        remote.store,
        &bundle_path,
        &platform.bundle,
        &platform.asset,
        force,
    )?;
    finish_install(remote, &outcome)
}

/// Activate the installed-or-reused instance and report it.
fn finish_install(remote: &RemoteContext<'_>, outcome: &archive::InstallOutcome) -> Result<bool> {
    let activation = activate_install(
        remote.store,
        outcome,
        remote.persister,
        super::path_has_dir(remote.env.path_var.as_deref(), &remote.store.shim_dir()),
    )?;
    emit_outcome(remote.ctx, remote.command, outcome, &activation)?;
    Ok(outcome.reused)
}

fn emit_outcome(
    ctx: &output::Context,
    command: &str,
    outcome: &archive::InstallOutcome,
    activation: &ActivationReport,
) -> Result<()> {
    if ctx.is_json() {
        return ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": command,
            "selector": outcome.record.selector().to_string(),
            "instance": outcome.record.instance,
            "home": outcome.home.display().to_string(),
            "source": outcome.record.source_path,
            "payload_sha256": outcome.record.payload_sha256,
            "reused": outcome.reused,
            "vibe_index_restart_required": true,
            "path_on_current_process": activation.path_on_current_process,
            "durable_path_changed": activation.durable_path_changed,
            "reopen_shell": !activation.path_on_current_process,
            "advisory_home_warning": activation.advisory_home_warning,
        }));
    }
    ctx.summary(
        "note: a running `vibe-index serve` keeps its old process; restart it to use this instance.",
    );
    if activation.path_on_current_process {
        ctx.summary("PATH is ready in this process.");
    } else if activation.durable_path_changed {
        ctx.summary("durable PATH updated; reopen the shell to resolve the stable shims.");
    } else {
        ctx.summary("durable PATH was already configured; reopen this shell to pick it up.");
    }
    if let Some(warning) = &activation.advisory_home_warning {
        ctx.summary(&format!(
            "warning: active pointer switched, but advisory VIBEVM_HOME was not updated: {warning}"
        ));
    }
    ctx.summary(&format!(
        "{} {} — active",
        if outcome.reused {
            "reused"
        } else {
            "installed"
        },
        outcome.record.selector()
    ));
    Ok(())
}

fn read_aggregate(path: &Path) -> Result<AggregateDistributionManifest> {
    let (mut file, metadata) = open_regular_no_follow(path)
        .with_context(|| format!("reading aggregate manifest `{}`", path.display()))?;
    if metadata.len() > DISTRIBUTION_AGGREGATE_MANIFEST_MAX_BYTES {
        bail!(
            "aggregate manifest `{}` is not a bounded regular file",
            path.display()
        );
    }
    let mut bytes = Vec::with_capacity(metadata.len().min(64 * 1024) as usize);
    let copied = copy_download(
        &mut file,
        &mut bytes,
        DISTRIBUTION_AGGREGATE_MANIFEST_MAX_BYTES,
    )?;
    if copied != metadata.len() {
        bail!("aggregate manifest changed size while reading");
    }
    AggregateDistributionManifest::from_json_slice(&bytes).map_err(Into::into)
}

fn select_platform<'a>(
    aggregate: &'a AggregateDistributionManifest,
    expected_version: &str,
    target: &str,
) -> Result<&'a PlatformDistributionFragment> {
    aggregate.validate()?;
    let expected = semver::Version::parse(expected_version)
        .with_context(|| format!("invalid bootstrap version `{expected_version}`"))?
        .to_string();
    if aggregate.version != expected || aggregate.tag != format!("v{expected}") {
        bail!(
            "aggregate manifest version/tag `{}`/`{}` does not match requested `{expected}`",
            aggregate.version,
            aggregate.tag
        );
    }
    aggregate
        .platforms
        .iter()
        .find(|platform| platform.target == target)
        .with_context(|| format!("aggregate manifest has no platform `{target}`"))
}

fn validated_release_base(raw: &str, expected_tag: &str) -> Result<String> {
    let url = reqwest::Url::parse(raw).context("parsing --release-base URL")?;
    if url.scheme() != "https"
        || url.host_str() != Some("github.com")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != format!("/vibevm/vibevm/releases/download/{expected_tag}")
    {
        bail!("release base must be the canonical HTTPS GitHub directory for `{expected_tag}`");
    }
    Ok(raw.trim_end_matches('/').to_string())
}

fn download_path(store: &VersionStore, name: &str) -> PathBuf {
    store.data_dir().join(format!(
        ".download-{}-{}-{name}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ))
}

fn cache_busted(url: &str) -> String {
    let nonce = REQUEST_NONCE.fetch_add(1, Ordering::Relaxed);
    format!(
        "{url}?vvm_nonce={}-{}-{nonce}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    )
}

struct DownloadCleanup(PathBuf);

impl Drop for DownloadCleanup {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

trait Downloader {
    fn download(&self, url: &str, destination: &Path, maximum_bytes: u64) -> Result<()>;
}

struct HttpDownloader;

impl Downloader for HttpDownloader {
    fn download(&self, url: &str, destination: &Path, maximum_bytes: u64) -> Result<()> {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating download directory `{}`", parent.display()))?;
        }
        let client = reqwest::blocking::Client::builder()
            .user_agent(format!("vibevm/{}", env!("CARGO_PKG_VERSION")))
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(TOTAL_TIMEOUT)
            .build()
            .context("building anonymous GitHub release client")?;
        let mut response = client
            .get(url)
            .header(reqwest::header::ACCEPT, "application/octet-stream")
            .header(reqwest::header::CACHE_CONTROL, "no-cache")
            .header(reqwest::header::PRAGMA, "no-cache")
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .with_context(|| format!("downloading `{url}`"))?;
        write_download(destination, maximum_bytes, &mut response)
            .with_context(|| format!("writing download `{}`", destination.display()))?;
        Ok(())
    }
}

fn write_download(destination: &Path, maximum_bytes: u64, reader: &mut impl Read) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .with_context(|| format!("creating `{}`", destination.display()))?;
    let result = copy_download(reader, &mut file, maximum_bytes);
    drop(file);
    if result.is_err() {
        let _ = fs::remove_file(destination);
    }
    result.map(|_| ())
}

fn copy_download(reader: &mut impl Read, writer: &mut impl io::Write, maximum: u64) -> Result<u64> {
    let copied = io::copy(&mut reader.take(maximum.saturating_add(1)), writer)?;
    if copied > maximum {
        bail!("download exceeds its {maximum}-byte limit");
    }
    Ok(copied)
}

fn current_target() -> Result<&'static str> {
    match (std::env::consts::ARCH, std::env::consts::OS) {
        ("x86_64", "windows") => Ok("x86_64-pc-windows-msvc"),
        ("x86_64", "linux") => Ok("x86_64-unknown-linux-musl"),
        ("x86_64", "macos") => Ok("x86_64-apple-darwin"),
        ("aarch64", "macos") => Ok("aarch64-apple-darwin"),
        (arch, os) => bail!("no vibevm binary distribution target for {arch}-{os}"),
    }
}

#[cfg(test)]
#[path = "bundle/tests.rs"]
mod tests;
