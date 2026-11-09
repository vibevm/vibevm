//! px in the versioned tool store (PROP-019 `#companion-tools`).
//!
//! The px pin (`xtask/dist/px.toml`) is embedded in every vibe build
//! (`vibe_net::px_store::PxPin::embedded`), and a release carries the pinned
//! upstream archive for each target as its own asset
//! `px-<px version>-<target>.<zip|tar.gz>` beside `DISTRIBUTIONS.json` —
//! whose closed shape vibe 1.0.11 and older refuse to extend. The archive is
//! read from the configured release source (GitHub, GitVerse, Nexus, or the
//! release drop), its SHA-256 checked against the embedded pin, and its
//! files placed in `<install root>/opt/vibevm/tools/px/<px version>/`, which
//! every instance of the installation — release or source build — shares.
//!
//! Release installs do this after the bundle is verified; a source install
//! and `vibe self px install` do it on their own. A source without the
//! asset is reported on the progress line and px mode then falls back to
//! `[proxy] px_path` or px on PATH; bytes that do not match the pin stop
//! the command.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools");

use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_core::progress::transfer::safe_url;
use vibe_net::px_store::{PinnedAsset, PxPin};
use vibe_publish::release_manifest::DISTRIBUTION_TOOL_ASSET_MAX_BYTES;

use super::download::{DownloadCleanup, cache_busted, download_path};
use super::{ReleaseDir, RemoteContext};
use crate::commands::vvm::store::VersionStore;
use vibe_publish::token::Token;

const ENTRY_MAX_BYTES: u64 = DISTRIBUTION_TOOL_ASSET_MAX_BYTES;

/// `<install root>/opt/vibevm/tools/px`.
pub(crate) fn store_root(store: &VersionStore) -> PathBuf {
    store.data_dir().join("tools").join("px")
}

/// The pinned asset of this build's target, if px has one.
pub(crate) fn pinned_asset(pin: &PxPin) -> Option<&PinnedAsset> {
    pin.asset(vibe_net::px_store::running_target()?)
}

/// The installed px executable in `root`, when it is there.
pub(crate) fn installed(root: &Path, pin: &PxPin, asset: &PinnedAsset) -> Option<PathBuf> {
    let program = vibe_net::px_store::stored_dir(root, pin).join(&asset.executable);
    program.is_file().then_some(program)
}

/// What one attempt to place px found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PxOutcome {
    AlreadyInstalled(PathBuf),
    Installed {
        program: PathBuf,
        from: String,
    },
    /// No source had the asset; each location tried is named.
    Unavailable {
        asset: String,
        tried: Vec<String>,
    },
    /// The pin has no asset for this target.
    NoTarget,
}

/// Place the pinned px in the store from the first of `dirs` that carries
/// its asset. `force` reinstalls over an existing copy.
pub(super) fn install_from_dirs(
    remote: &RemoteContext<'_>,
    pin: &PxPin,
    dirs: &[ReleaseDir],
    token: Option<&Token>,
    force: bool,
) -> Result<PxOutcome> {
    let Some(asset) = pinned_asset(pin) else {
        return Ok(PxOutcome::NoTarget);
    };
    let root = store_root(remote.store);
    if !force && let Some(program) = installed(&root, pin, asset) {
        return Ok(PxOutcome::AlreadyInstalled(program));
    }
    let name = pin.release_asset_name(asset);
    let mut tried = Vec::new();
    for dir in dirs {
        let Ok(url) = dir.asset(&name) else {
            continue;
        };
        let shown = safe_url(&url);
        let path = download_path(remote.store, &name);
        remote.store.guard_mutation_path(&path)?;
        let _cleanup = DownloadCleanup(path.clone());
        let task = remote.progress.task(format!("Fetching px {}", pin.version));
        task.detail(shown.clone());
        if remote
            .downloader
            .download_authorized(
                &cache_busted(&url),
                &path,
                DISTRIBUTION_TOOL_ASSET_MAX_BYTES,
                None,
                &task,
                token,
            )
            .is_err()
        {
            task.skip("not there");
            tried.push(shown);
            continue;
        }
        let bytes = fs::read(&path).with_context(|| format!("reading `{}`", path.display()))?;
        task.finish();
        remote.store.guard_mutation_path(&root)?;
        let program = install_bytes(&root, pin, asset, &bytes)?;
        return Ok(PxOutcome::Installed {
            program,
            from: shown,
        });
    }
    Ok(PxOutcome::Unavailable { asset: name, tried })
}

/// Verify `bytes` against the pin and unpack them into the store as
/// `<root>/<version>/`, replacing an older copy only once the new one is
/// complete. Returns the executable.
pub(crate) fn install_bytes(
    root: &Path,
    pin: &PxPin,
    asset: &PinnedAsset,
    bytes: &[u8],
) -> Result<PathBuf> {
    use sha2::{Digest, Sha256};
    let digest = format!("{:x}", Sha256::digest(bytes));
    let name = pin.release_asset_name(asset);
    if digest != asset.sha256 {
        bail!(
            "px archive `{name}` integrity mismatch: the pin is sha256:{}, the bytes are \
             sha256:{digest} (violates spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools; \
             fix: copy the release's `{name}` again — it must be the pinned upstream archive)",
            asset.sha256
        );
    }
    fs::create_dir_all(root).with_context(|| format!("creating `{}`", root.display()))?;
    let staging = tempfile::Builder::new()
        .prefix(".px-")
        .tempdir_in(root)
        .with_context(|| format!("staging px under `{}`", root.display()))?;
    match asset.archive.as_str() {
        "zip" => unzip(bytes, staging.path())?,
        "tar.gz" => untar(bytes, staging.path())?,
        other => bail!("px archive kind `{other}` is not one vibe unpacks"),
    }
    let program = staging.path().join(&asset.executable);
    if !program.is_file() {
        bail!(
            "the px archive `{name}` has no `{}` \
             (violates spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools; \
             fix: copy the release's `{name}` again)",
            asset.executable
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755))?;
    }
    fs::write(staging.path().join("NOTICE"), format!("{}\n", pin.notice))?;
    let destination = vibe_net::px_store::stored_dir(root, pin);
    let staged = staging.keep();
    if destination.exists() {
        fs::remove_dir_all(&destination)
            .with_context(|| format!("replacing `{}`", destination.display()))?;
    }
    if let Err(error) = fs::rename(&staged, &destination) {
        let _ = fs::remove_dir_all(&staged);
        return Err(error).with_context(|| format!("placing `{}`", destination.display()));
    }
    Ok(destination.join(&asset.executable))
}

fn unzip(bytes: &[u8], into: &Path) -> Result<()> {
    let mut zip =
        zip::ZipArchive::new(std::io::Cursor::new(bytes)).context("opening the px archive")?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).context("reading the px archive")?;
        let Some(relative) = contained(entry.name())? else {
            continue;
        };
        let target = into.join(&relative);
        if entry.is_dir() {
            fs::create_dir_all(&target)?;
            continue;
        }
        if !entry.is_file() {
            bail!("px archive entry `{}` is not a regular file", entry.name());
        }
        write_entry(&mut entry, &target)?;
    }
    Ok(())
}

fn untar(bytes: &[u8], into: &Path) -> Result<()> {
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(bytes));
    for entry in archive.entries().context("reading the px archive")? {
        let mut entry = entry.context("reading the px archive")?;
        let name = entry.path()?.to_string_lossy().into_owned();
        let Some(relative) = contained(&name)? else {
            continue;
        };
        let target = into.join(&relative);
        let kind = entry.header().entry_type();
        if kind.is_dir() {
            fs::create_dir_all(&target)?;
        } else if kind.is_file() {
            write_entry(&mut entry, &target)?;
        } else {
            bail!("px archive entry `{name}` is not a regular file");
        }
    }
    Ok(())
}

fn write_entry(entry: &mut impl Read, target: &Path) -> Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut out =
        fs::File::create(target).with_context(|| format!("writing `{}`", target.display()))?;
    let copied = std::io::copy(&mut entry.take(ENTRY_MAX_BYTES.saturating_add(1)), &mut out)?;
    if copied > ENTRY_MAX_BYTES {
        bail!("px archive entry `{}` exceeds its bound", target.display());
    }
    Ok(())
}

/// A relative, normal archive path (`None` for the archive's root `./`);
/// anything that could escape is refused.
fn contained(name: &str) -> Result<Option<PathBuf>> {
    let mut out = PathBuf::new();
    for component in Path::new(name).components() {
        match component {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            _ => bail!("px archive entry `{name}` escapes its directory"),
        }
    }
    Ok((!out.as_os_str().is_empty()).then_some(out))
}
