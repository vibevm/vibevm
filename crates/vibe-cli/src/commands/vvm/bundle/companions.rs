//! px in the versioned tool store (PROP-019 `#companion-tools`).
//!
//! The px pin (`xtask/dist/px.toml`) is embedded in every vibe build
//! (`vibe_net::px_store::PxPin::embedded`), and a release carries the pinned
//! upstream archive for each target as its own asset
//! `px-<px version>-<target>.<zip|tar.gz>` beside `DISTRIBUTIONS.json` —
//! whose closed shape vibe 1.0.11 and older refuse to extend. The archive is
//! read from the configured release source (GitHub, GitVerse, Nexus, or the
//! release drop) and its files placed in
//! `<install root>/opt/vibevm/tools/px/<px version>/`, which every instance
//! of the installation — release or source build — shares.
//!
//! Release installs do this after the bundle is verified; a source install
//! and `vibe self px install` do it on their own. A source without the
//! asset is reported on the progress line and px mode then falls back to
//! `[proxy] px_path` or px on PATH.
//!
//! **px itself is never checked against a digest** (owner decision
//! 2026-11-09, `##COMPANION-TOOLS-ANY-PX`): px is a third-party program the
//! operator chooses, and a network that needs px needs whichever px
//! authenticates there. The pin says which px to fetch by default; the
//! SHA-256 is measured and written into the store's `PX.toml` as
//! provenance, so a report can name the exact bytes. What still stops an
//! installation is a payload that is not a px at all — an archive that
//! cannot be unpacked, one that escapes its directory, one with no `px`
//! program inside.

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

/// The px the store offers, whichever version directory holds it
/// (`PROP-019#companion-tools`).
pub(crate) fn installed(root: &Path) -> Option<vibe_net::px_store::InstalledPx> {
    vibe_net::px_store::stored(root)
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
    if !force && let Some(found) = installed(&root) {
        return Ok(PxOutcome::AlreadyInstalled(found.program));
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
        let program = install_bytes(&root, pin, asset, &bytes, &shown)?;
        return Ok(PxOutcome::Installed {
            program,
            from: shown,
        });
    }
    Ok(PxOutcome::Unavailable { asset: name, tried })
}

/// The two places a release unpacked by hand carries the px archive: beside
/// the binary, and in `tools/px/` there. Nothing else in the directory is
/// ever read.
pub(crate) fn beside_candidates(pin: &PxPin, asset: &PinnedAsset, dir: &Path) -> [PathBuf; 2] {
    let name = pin.release_asset_name(asset);
    [dir.join(&name), dir.join("tools").join("px").join(&name)]
}

/// The px archive beside a hand-unpacked release, when one of the two
/// places holds it. Reads nothing and writes nothing: the question a
/// `--dry-run` asks.
pub(crate) fn archive_beside(pin: &PxPin, asset: &PinnedAsset, dir: &Path) -> Option<PathBuf> {
    beside_candidates(pin, asset, dir)
        .into_iter()
        .find(|path| fs::symlink_metadata(path).is_ok_and(|meta| meta.is_file()))
}

/// Place px from a release unpacked by hand: the archive under the pinned
/// name, beside the binary or in `tools/px/` there. Nothing else in the
/// directory is read, and the bytes are installed as they are — a px from
/// the operator's own files is their choice, recorded and shown, never
/// refused (`PROP-019#companion-tools`).
pub(super) fn install_beside(store: &VersionStore, pin: &PxPin, dir: &Path) -> Result<PxOutcome> {
    let Some(asset) = pinned_asset(pin) else {
        return Ok(PxOutcome::NoTarget);
    };
    let name = pin.release_asset_name(asset);
    let candidates = beside_candidates(pin, asset, dir);
    let Some(path) = archive_beside(pin, asset, dir) else {
        return Ok(PxOutcome::Unavailable {
            asset: name,
            tried: candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect(),
        });
    };
    let root = store_root(store);
    if let Some(found) = installed(&root) {
        return Ok(PxOutcome::AlreadyInstalled(found.program));
    }
    let bytes = read_bounded(&path)?;
    store.guard_mutation_path(&root)?;
    let program = install_bytes(&root, pin, asset, &bytes, &path.display().to_string())?;
    Ok(PxOutcome::Installed {
        program,
        from: path.display().to_string(),
    })
}

/// One px file, read under the companion-asset bound.
pub(super) fn read_bounded(path: &Path) -> Result<Vec<u8>> {
    let (file, _) = crate::commands::vvm::store::open_regular_no_follow(path)
        .with_context(|| format!("reading `{}`", path.display()))?;
    let mut bytes = Vec::new();
    file.take(DISTRIBUTION_TOOL_ASSET_MAX_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .with_context(|| format!("reading `{}`", path.display()))?;
    if bytes.len() as u64 > DISTRIBUTION_TOOL_ASSET_MAX_BYTES {
        bail!(
            "`{}` exceeds the px payload bound of {DISTRIBUTION_TOOL_ASSET_MAX_BYTES} bytes",
            path.display()
        );
    }
    Ok(bytes)
}

/// Unpack `bytes` into the store as `<root>/<version>/`, replacing an
/// older copy only once the new one is complete, and record where they
/// came from. Returns the executable.
///
/// Nothing here refuses px over a digest: px is the operator's choice and
/// vibe does not pin its content (owner decision 2026-11-09,
/// `PROP-019#companion-tools`). The SHA-256 is measured all the same and
/// written into the record, so `vibe self px status` can say exactly which
/// bytes are installed.
pub(crate) fn install_bytes(
    root: &Path,
    pin: &PxPin,
    asset: &PinnedAsset,
    bytes: &[u8],
    source: &str,
) -> Result<PathBuf> {
    place(
        root,
        &vibe_net::px_store::stored_dir(root, pin),
        &Payload::Archive {
            kind: asset.archive.clone(),
            bytes,
            executable: Some(asset.executable.clone()),
        },
        &Provenance {
            source: source.to_owned(),
            sha256: measure(bytes),
            notice: Some(pin.notice.clone()),
        },
    )
}

/// The px an operator supplies from their own files
/// (`vibe self px install --from …`).
mod local;
pub(super) use local::install_local;

/// The SHA-256 vibe records of the bytes it installed — information for
/// the report, never a gate.
pub(crate) fn measure(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

/// What a px installation is made of.
pub(super) enum Payload<'a> {
    /// An archive of the named kind, with the executable's path inside it
    /// when the caller knows it.
    Archive {
        kind: String,
        bytes: &'a [u8],
        executable: Option<String>,
    },
    /// A single px program.
    Executable { bytes: &'a [u8] },
    /// A directory the operator unpacked themselves.
    Directory { path: &'a Path },
}

/// Where the bytes came from, as the record states it.
pub(super) struct Provenance {
    pub(super) source: String,
    pub(super) sha256: String,
    /// The upstream notice, for a px read from the release source.
    pub(super) notice: Option<String>,
}

/// Stage `payload` under `root`, find the px executable in it, record
/// `provenance` beside it and swap the whole directory into `destination`.
pub(super) fn place(
    root: &Path,
    destination: &Path,
    payload: &Payload<'_>,
    provenance: &Provenance,
) -> Result<PathBuf> {
    fs::create_dir_all(root).with_context(|| format!("creating `{}`", root.display()))?;
    let staging = tempfile::Builder::new()
        .prefix(".px-")
        .tempdir_in(root)
        .with_context(|| format!("staging px under `{}`", root.display()))?;
    let named = match payload {
        Payload::Archive {
            kind,
            bytes,
            executable,
        } => {
            match kind.as_str() {
                "zip" => unzip(bytes, staging.path())?,
                "tar.gz" => untar(bytes, staging.path())?,
                other => bail!(
                    "px archive kind `{other}` is not one vibe unpacks \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools; \
                      fix: give a `.zip`, a `.tar.gz`, a directory or the px program itself)"
                ),
            }
            executable.clone()
        }
        Payload::Executable { bytes } => {
            fs::write(staging.path().join(vibe_net::PX_EXE), bytes)
                .with_context(|| format!("writing px under `{}`", staging.path().display()))?;
            Some(vibe_net::PX_EXE.to_owned())
        }
        Payload::Directory { path } => {
            copy_tree(path, staging.path(), 0)?;
            None
        }
    };
    let relative = match named.filter(|name| staging.path().join(name).is_file()) {
        Some(name) => name,
        None => find_program(staging.path())?,
    };
    let program = staging.path().join(&relative);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755))?;
    }
    if let Some(notice) = &provenance.notice {
        fs::write(staging.path().join("NOTICE"), format!("{notice}\n"))?;
    }
    let version = program_version(&program);
    fs::write(
        staging.path().join(vibe_net::px_store::RECORD_FILE),
        vibe_net::px_store::record_text(
            &relative.replace('\\', "/"),
            &provenance.source,
            &provenance.sha256,
            &chrono::Utc::now().to_rfc3339(),
            version.as_deref(),
        ),
    )?;
    let staged = staging.keep();
    if destination.exists() {
        fs::remove_dir_all(destination)
            .with_context(|| format!("replacing `{}`", destination.display()))?;
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    if let Err(error) = fs::rename(&staged, destination) {
        let _ = fs::remove_dir_all(&staged);
        return Err(error).with_context(|| format!("placing `{}`", destination.display()));
    }
    Ok(destination.join(&relative))
}

/// What the installed program says its version is, when it answers at all.
/// A px that cannot be run is still installed: the field is simply absent.
///
/// px has no `--version`: an unknown flag starts it as a proxy server that
/// never exits. `--help` prints `Px vX.Y.Z` on its first line and exits, and
/// the call is bounded anyway, so a px of any shape can never hang the
/// install: past [`VERSION_PROBE_LIMIT`] the child is killed and the version
/// is simply not recorded.
fn program_version(program: &Path) -> Option<String> {
    let text = bounded_output(program, &["--help"], VERSION_PROBE_LIMIT)?;
    let line = text
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("Px v") || line.starts_with("px v"))
        .or_else(|| text.lines().map(str::trim).find(|line| !line.is_empty()))?;
    (!line.is_empty()).then(|| line.chars().take(120).collect())
}

/// How long the version probe may run before the child is killed.
const VERSION_PROBE_LIMIT: std::time::Duration = std::time::Duration::from_secs(5);

/// Run `program args…` with no stdin and return stdout (or stderr when stdout
/// is empty), or `None` when it cannot start or does not exit within `limit`
/// — then it is killed and reaped, never left running.
fn bounded_output(program: &Path, args: &[&str], limit: std::time::Duration) -> Option<String> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let mut stderr = child.stderr.take()?;
    let out = std::thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = stdout.read_to_end(&mut buffer);
        buffer
    });
    let err = std::thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = stderr.read_to_end(&mut buffer);
        buffer
    });
    let deadline = std::time::Instant::now() + limit;
    let exited = loop {
        match child.try_wait() {
            Ok(Some(_)) => break true,
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            _ => break false,
        }
    };
    if !exited {
        let _ = child.kill();
        let _ = child.wait();
        return None;
    }
    let stdout = out.join().unwrap_or_default();
    let stderr = err.join().unwrap_or_default();
    let bytes = if stdout.is_empty() { stderr } else { stdout };
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// The px program inside an unpacked tree: the shallowest file named `px`
/// or `px.exe`, searched in a bounded way.
fn find_program(root: &Path) -> Result<String> {
    let mut level = vec![PathBuf::new()];
    for _ in 0..TREE_MAX_DEPTH {
        let mut next = Vec::new();
        let mut directories = Vec::new();
        for relative in &level {
            let entries = match fs::read_dir(root.join(relative)) {
                Ok(entries) => entries,
                Err(_) => continue,
            };
            for entry in entries.flatten().take(TREE_MAX_ENTRIES) {
                let name = entry.file_name().to_string_lossy().into_owned();
                let here = relative.join(&name);
                match entry.file_type() {
                    Ok(kind) if kind.is_dir() => directories.push(here),
                    Ok(kind)
                        if kind.is_file()
                            && (name.eq_ignore_ascii_case("px")
                                || name.eq_ignore_ascii_case("px.exe")) =>
                    {
                        next.push(here);
                    }
                    _ => {}
                }
            }
        }
        next.sort();
        if let Some(found) = next.first() {
            return Ok(found.to_string_lossy().into_owned());
        }
        directories.sort();
        level = directories;
        if level.is_empty() {
            break;
        }
    }
    bail!(
        "no `px` program is in there \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools; \
          fix: give the archive, directory or executable that holds px — the program must be \
          named `px` or `px.exe`)"
    )
}

/// How deep and how wide a px layout is read.
const TREE_MAX_DEPTH: usize = 4;
const TREE_MAX_ENTRIES: usize = 512;

/// Copy a directory the operator unpacked themselves, bounded the way the
/// archive readers are bounded.
fn copy_tree(from: &Path, into: &Path, depth: usize) -> Result<()> {
    if depth >= TREE_MAX_DEPTH {
        return Ok(());
    }
    fs::create_dir_all(into)?;
    for entry in fs::read_dir(from)
        .with_context(|| format!("reading `{}`", from.display()))?
        .flatten()
        .take(TREE_MAX_ENTRIES)
    {
        let target = into.join(entry.file_name());
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => copy_tree(&entry.path(), &target, depth + 1)?,
            Ok(kind) if kind.is_file() => {
                let metadata = entry.metadata()?;
                if metadata.len() > ENTRY_MAX_BYTES {
                    bail!("`{}` exceeds the px payload bound", entry.path().display());
                }
                fs::copy(entry.path(), &target)
                    .with_context(|| format!("copying `{}`", entry.path().display()))?;
            }
            _ => {}
        }
    }
    Ok(())
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

#[cfg(all(test, unix))]
mod version_probe_tests;
