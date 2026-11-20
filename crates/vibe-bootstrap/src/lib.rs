//! Local-only, verified native bootstrap. Configuration and installation stay
//! owned by the verified distribution's native `vibe self apply` implementation.
pub mod file_verify;
pub mod kit;
mod optional_setup;
pub mod regular;
pub mod source_extract;
mod upgrade;

use anyhow::{Context, Result, bail};
use file_verify::{copy_and_hash_bounded, open_hashed_regular_exact};
use std::{
    collections::BTreeSet,
    fs, io,
    path::{Path, PathBuf},
};
use vibe_wire::behaviour::release_manifest::{
    BundleDistributionManifest, DISTRIBUTION_BUNDLE_MAX_BYTES, DISTRIBUTION_COMPONENT_MAX_BYTES,
    DISTRIBUTION_MANIFEST_MAX_BYTES, DISTRIBUTION_SOURCE_ARCHIVE_MAX_BYTES,
};
use zip::ZipArchive;

/// Staging remains alive until all verified native child operations finish.
///
/// ```
/// use vibe_bootstrap::VerifiedBundle;
/// fn version(bundle: &VerifiedBundle) -> &str { &bundle.manifest.version }
/// ```
pub struct VerifiedBundle {
    pub manifest: BundleDistributionManifest,
    pub directory: PathBuf,
    _stage: tempfile::TempDir,
}

/// Select the running native platform without probing another machine.
///
/// ```
/// assert!(!vibe_bootstrap::target().unwrap().is_empty());
/// ```
pub fn target() -> Result<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => Ok("x86_64-pc-windows-msvc"),
        ("linux", "x86_64") if cfg!(target_env = "musl") => Ok("x86_64-unknown-linux-musl"),
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-gnu"),
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        _ => bail!("unsupported native installer platform"),
    }
}

/// Authenticate the complete local ZIP, validate its closed manifest and every
/// component/source member before any bundled executable can be run.
///
/// ```
/// let missing = std::path::Path::new("/missing-vibe-bootstrap-fixture.zip");
/// assert!(vibe_bootstrap::verify_bundle(missing, &"0".repeat(64), "x86_64-unknown-linux-gnu").is_err());
/// ```
pub fn verify_bundle(path: &Path, sha256: &str, expected_target: &str) -> Result<VerifiedBundle> {
    let (_, metadata) = regular::open_regular_no_follow(path)?;
    let (file, measured) =
        open_hashed_regular_exact(path, metadata.len(), DISTRIBUTION_BUNDLE_MAX_BYTES, false)?;
    if measured != format!("sha256:{}", sha256.to_ascii_lowercase()) {
        bail!("bundle SHA256 mismatch; no installation changes were made");
    }
    let mut zip = ZipArchive::new(file).context("opening verified release ZIP")?;
    let bytes = {
        let mut entry = zip
            .by_name("DISTRIBUTION.json")
            .context("release has no DISTRIBUTION.json")?;
        let mut bytes = Vec::new();
        file_verify::copy_bounded(&mut entry, &mut bytes, DISTRIBUTION_MANIFEST_MAX_BYTES)?;
        bytes
    };
    let manifest = BundleDistributionManifest::from_json_slice(&bytes)?;
    if manifest.target != expected_target {
        bail!("bundle target does not match this installer host");
    }
    let expected: BTreeSet<_> = manifest
        .components
        .iter()
        .map(|c| c.path.clone())
        .chain([
            manifest.source_archive.path.clone(),
            "DISTRIBUTION.json".into(),
            "LICENSE.md".into(),
            "README.md".into(),
        ])
        .collect();
    let mut seen = BTreeSet::new();
    for i in 0..zip.len() {
        let entry = zip.by_index(i)?;
        if entry.is_dir()
            || !expected.contains(entry.name())
            || !seen.insert(entry.name().to_owned())
            || source_extract::special_file_mode(entry.unix_mode(), false)
        {
            bail!("release ZIP has an unexpected, duplicate or special entry");
        }
    }
    if seen != expected {
        bail!("release ZIP is missing required members");
    }
    // First pass verifies ALL members. Staging/publication is not a substitute
    // for refusing a damaged source archive before running the native child.
    for (name, size, digest) in manifest
        .components
        .iter()
        .map(|c| (&c.path, c.size, &c.digest))
        .chain(std::iter::once((
            &manifest.source_archive.path,
            manifest.source_archive.size,
            &manifest.source_archive.digest,
        )))
    {
        let mut entry = zip.by_name(name)?;
        if entry.size() != size {
            bail!("release member size mismatch: {name}");
        }
        let limit = if name == &manifest.source_archive.path {
            DISTRIBUTION_SOURCE_ARCHIVE_MAX_BYTES
        } else {
            DISTRIBUTION_COMPONENT_MAX_BYTES
        };
        if size > limit {
            bail!("release member exceeds its bound");
        }
        let (actual, measured) = copy_and_hash_bounded(&mut entry, &mut io::sink(), size)?;
        if actual != size || &measured != digest {
            bail!("release member digest mismatch: {name}");
        }
    }
    let stage = tempfile::Builder::new()
        .prefix("vibe-native-bootstrap-")
        .tempdir()?;
    let directory = stage.path().join("release");
    fs::create_dir(&directory)?;
    for name in &expected {
        let mut entry = zip.by_name(name)?;
        let limit = if name == &manifest.source_archive.path {
            DISTRIBUTION_SOURCE_ARCHIVE_MAX_BYTES
        } else if name == "README.md" || name == "LICENSE.md" || name == "DISTRIBUTION.json" {
            4 * 1024 * 1024
        } else {
            DISTRIBUTION_COMPONENT_MAX_BYTES
        };
        let destination = directory.join(name);
        let mut out = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)?;
        file_verify::copy_bounded(&mut entry, &mut out, limit)?;
        #[cfg(unix)]
        if manifest.components.iter().any(|c| &c.path == name) {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&destination, fs::Permissions::from_mode(0o755))?;
        }
    }
    // The same safe source extractor used by normal bundle import validates
    // collisions, links, Windows names, nesting and expanded-size bounds.
    source_extract::extract_source_archive(
        &directory.join(&manifest.source_archive.path),
        &stage.path().join("source-validation"),
    )?;
    Ok(VerifiedBundle {
        manifest,
        directory,
        _stage: stage,
    })
}
