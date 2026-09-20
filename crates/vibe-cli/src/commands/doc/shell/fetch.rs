//! Getting the reader's shell from a release, and nothing else (PROP-057
//! `##SHELL-RELEASE-ASSET`, `##SHELL-INSTALL-COMMAND`).
//!
//! The shell is a **separate release asset** with a manifest of its own
//! beside `DISTRIBUTIONS.json`, and the reason is recorded rather than
//! stylistic: the aggregate manifest's schema is closed
//! (`deny_unknown_fields`, one accepted version), so a field added to it
//! would make every installed `vibe 1.0.0` refuse the next release and
//! lose `self update`. A second document old clients never ask for costs
//! them nothing.
//!
//! The transport is `vibe self install`'s, deliberately: the same
//! anonymous client, the same cache-busting, the same read capped at the
//! declared size, the same temporary file removed whatever happens, and
//! the same rule that BOTH the size and the digest must match. It is not
//! shared as code — the VVM's downloader is private to the version store,
//! and taking it would drag the store's whole model into a reader — so
//! the SHAPE is repeated, which is the trade `##PIPE-CRATES` already
//! makes between these crates.
//!
//! Nothing here runs by itself. The one caller asks the operator first,
//! every time.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#SHELL-RELEASE-ASSET");

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};

pub(crate) use vibe_wire::generated::doc_shell_release_manifest::{
    DistributionAsset, DocShellReleaseManifest as DocShellManifest,
};

/// Where releases are read from when nobody names somewhere else.
pub(crate) const RELEASE_ROOT: &str = "https://github.com/vibevm/vibevm/releases/download";

/// The shell's manifest, beside `DISTRIBUTIONS.json` and of the same
/// form.
pub(crate) const MANIFEST_FILENAME: &str = "DOC-SHELL.json";

/// A manifest is small; a reader that accepted an unbounded one would
/// have accepted a denial of service as a document.
const MANIFEST_MAX_BYTES: u64 = 64 * 1024;

/// The ceiling on the shell asset itself. The measured shell is under a
/// megabyte; this is the order of magnitude above it that says «this is
/// not the thing you asked for» without tripping on a legitimate growth.
const ASSET_MAX_BYTES: u64 = 32 * 1024 * 1024;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
const TOTAL_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// The schema this reader knows.
pub(crate) const MANIFEST_SCHEMA_VERSION: u32 = 1;

/// Read a manifest through the registered generated shape and refuse anything
/// it is not.
pub(crate) fn parse_manifest(bytes: &[u8], version: &str) -> Result<DocShellManifest> {
    let manifest: DocShellManifest =
        serde_json::from_slice(bytes).context("reading the shell's release manifest")?;
    if manifest.schema_version != MANIFEST_SCHEMA_VERSION {
        bail!(
            "the shell manifest is written to schema {} and this `vibe` knows \
             {MANIFEST_SCHEMA_VERSION}",
            manifest.schema_version
        );
    }
    if manifest.version != version {
        bail!(
            "the shell manifest is for `vibe {}` and this is `vibe {version}` — a shell \
             is a build of one version's site package and does not travel between them \
             (violates spec://org.vibevm.core/vibevm/common/PROP-057#SHELL-PIN)",
            manifest.version
        );
    }
    Ok(manifest)
}

/// Where bytes come from, so a test can answer without a network.
pub(crate) trait Fetcher {
    /// Read `url` into `destination`, refusing anything past `maximum`.
    fn fetch(&self, url: &str, destination: &Path, maximum: u64) -> Result<()>;
}

/// The real one.
pub(crate) struct HttpFetcher;

impl Fetcher for HttpFetcher {
    fn fetch(&self, url: &str, destination: &Path, maximum: u64) -> Result<()> {
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating `{}`", parent.display()))?;
        }
        if !url.contains("://") {
            // A release copied onto this machine. The size cap is
            // applied the same way, because «too big» is a property of
            // the answer and not of the transport that brought it.
            let bytes = std::fs::read(url).with_context(|| format!("reading `{url}`"))?;
            if bytes.len() as u64 > maximum {
                bail!("`{url}` exceeds its {maximum}-byte limit");
            }
            return std::fs::write(destination, bytes)
                .with_context(|| format!("writing `{}`", destination.display()));
        }
        let client = reqwest::blocking::Client::builder()
            .user_agent(format!("vibevm/{}", env!("CARGO_PKG_VERSION")))
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(TOTAL_TIMEOUT)
            .build()
            .context("building the anonymous release client")?;
        let mut response = client
            .get(url)
            .header(reqwest::header::ACCEPT, "application/octet-stream")
            .header(reqwest::header::CACHE_CONTROL, "no-cache")
            .header(reqwest::header::PRAGMA, "no-cache")
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .with_context(|| format!("downloading `{url}`"))?;
        let mut file = std::fs::File::create(destination)
            .with_context(|| format!("writing `{}`", destination.display()))?;
        let copied = std::io::copy(
            &mut Read::take(&mut response, maximum.saturating_add(1)),
            &mut file,
        )
        .with_context(|| format!("writing `{}`", destination.display()))?;
        if copied > maximum {
            bail!("`{url}` exceeds its {maximum}-byte limit");
        }
        Ok(())
    }
}

/// Remove a temporary file whatever happened to the run that made it.
pub(crate) struct Cleanup(pub(crate) PathBuf);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// The address of one release file, at the shape `vibe self` uses:
/// `<base>/v<version>/<name>`.
///
/// A base with no scheme is a directory on this machine — an air-gapped
/// copy of a release, and what a test points at. It is the same address
/// grammar either way, so the caller composes one string and the fetcher
/// decides how to read it.
pub(crate) fn asset_url(base: &str, version: &str, name: &str) -> String {
    format!("{}/v{version}/{name}", base.trim_end_matches('/'))
}

/// Both halves of the integrity check, in one place, because passing one
/// of them is not passing (`##SHELL-RELEASE-ASSET`).
pub(crate) fn require_digest(what: &str, bytes: &[u8], expected: &DistributionAsset) -> Result<()> {
    let size = bytes.len() as u64;
    let digest = format!("sha256:{}", hex(&Sha256::digest(bytes)));
    if size != expected.size || digest != expected.digest {
        bail!(
            "{what} integrity mismatch: the manifest declares {} bytes / {}, the download is \
             {size} bytes / {digest} (violates \
             spec://org.vibevm.core/vibevm/common/PROP-057#SHELL-RELEASE-ASSET; \
             fix: nothing local — the release is not what it says it is, and the shell is not \
             installed)",
            expected.size,
            expected.digest
        );
    }
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Unpack a shell archive into `into`, refusing any entry that is not a
/// plain relative path of ordinary names.
///
/// A zip entry's name is attacker-controlled by construction — it is a
/// string inside the file being verified — so it is checked before it
/// becomes a path, the same rule the reader's routes follow.
pub(crate) fn unpack(archive: &Path, into: &Path) -> Result<usize> {
    let file =
        std::fs::File::open(archive).with_context(|| format!("opening `{}`", archive.display()))?;
    let mut zip = zip::ZipArchive::new(file)
        .with_context(|| format!("`{}` is not a readable archive", archive.display()))?;
    let mut written = 0usize;
    for at in 0..zip.len() {
        let mut entry = zip.by_index(at).context("reading an archive entry")?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        let Some(relative) = plain_relative(&name) else {
            bail!(
                "the shell archive carries an entry named `{name}`, which is not a relative \
                 path of ordinary names (violates \
                 spec://org.vibevm.core/vibevm/common/PROP-057#LOCAL-STATIC; \
                 fix: nothing local — refuse the archive)"
            );
        };
        let mut path = into.to_path_buf();
        for segment in relative.split('/') {
            path.push(segment);
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating `{}`", parent.display()))?;
        }
        let mut out = std::fs::File::create(&path)
            .with_context(|| format!("writing `{}`", path.display()))?;
        std::io::copy(&mut entry, &mut out)
            .with_context(|| format!("writing `{}`", path.display()))?;
        written += 1;
    }
    Ok(written)
}

/// `Some(path)` when a name is a relative path of ordinary names.
pub(crate) fn plain_relative(name: &str) -> Option<&str> {
    let cleaned = name.trim_end_matches('/');
    if cleaned.is_empty() || cleaned.starts_with('/') || cleaned.contains('\\') {
        return None;
    }
    // A Windows drive letter, which is absolute without a leading slash.
    if cleaned.len() > 1 && cleaned.as_bytes().get(1) == Some(&b':') {
        return None;
    }
    for segment in cleaned.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." || segment.contains('\0') {
            return None;
        }
    }
    Some(cleaned)
}

/// The cap a manifest read is allowed.
pub(crate) const fn manifest_max_bytes() -> u64 {
    MANIFEST_MAX_BYTES
}

/// The cap an asset read is allowed.
pub(crate) const fn asset_max_bytes() -> u64 {
    ASSET_MAX_BYTES
}
