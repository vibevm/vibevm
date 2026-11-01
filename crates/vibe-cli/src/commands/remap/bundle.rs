//! The transport bundle: what `remap export` writes and `cache import`
//! places (`PROP-066#transport`).
//!
//! Getting the first set into a closed network is a file-copy problem, so
//! the bundle is a directory anyone can carry on a disk: a `file://`
//! registry holding the remap package (the `<group>/<name>/v<version>/`
//! layout `--registry <dir>` already reads), an `artifacts/<sha256>/<filename>`
//! tree laid out exactly as the application download cache is, and a
//! `MANIFEST.json` (`vibe-remap-bundle/1`) listing every entry with its
//! digest and its origin.
//!
//! Importing verifies every digest **before** placing anything, and never
//! overwrites an entry that already verifies: the machine store is
//! write-once, and a bundle is no reason to make it otherwise.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#transport");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use vibe_wire::generated::remap_bundle as wire;

/// The `vibe-remap-bundle/1` protocol.
pub(crate) const BUNDLE_PROTOCOL: &str = "vibe-remap-bundle/1";

/// The manifest's file name inside the bundle.
pub(crate) const MANIFEST: &str = "MANIFEST.json";

/// The directory artifacts are laid out under, by digest.
pub(crate) const ARTIFACTS: &str = "artifacts";

/// What one import placed.
#[derive(Debug, Default)]
pub(crate) struct Imported {
    /// Entries this call wrote.
    pub(crate) placed: Vec<String>,
    /// Entries already present and already verifying — untouched.
    pub(crate) already: Vec<String>,
    /// Entries that could not be placed, each with the reason.
    pub(crate) refused: Vec<(String, String)>,
}

/// Read a bundle's `MANIFEST.json`.
pub(crate) fn read_manifest(dir: &Path) -> Result<wire::RemapBundle> {
    let path = dir.join(MANIFEST);
    let body = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "reading `{}` — `{}` is not a remap bundle (write one with \
             `vibe remap export <package> --out <dir> --with-artifacts`)",
            path.display(),
            dir.display()
        )
    })?;
    let document: wire::RemapBundle =
        serde_json::from_str(&body).with_context(|| format!("parsing `{}`", path.display()))?;
    if document.protocol != BUNDLE_PROTOCOL {
        bail!(
            "`{}` declares protocol `{}`, not `{BUNDLE_PROTOCOL}` \
             (violates spec://org.vibevm.core/vibevm/common/PROP-066#transport; \
              fix: re-export the bundle with this version of vibe)",
            path.display(),
            document.protocol
        );
    }
    Ok(document)
}

/// Write a bundle's `MANIFEST.json`.
pub(crate) fn write_manifest(dir: &Path, document: &wire::RemapBundle) -> Result<()> {
    let path = dir.join(MANIFEST);
    let body = serde_json::to_string_pretty(document).context("rendering the bundle manifest")?;
    std::fs::write(&path, format!("{body}\n"))
        .with_context(|| format!("writing `{}`", path.display()))
}

/// Place every artifact a bundle lists into the machine store or the
/// application download cache, verifying each digest first
/// (`PROP-066#transport`).
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#transport")]
pub(crate) fn place(dir: &Path, document: &wire::RemapBundle) -> Result<Imported> {
    let mut imported = Imported::default();
    let app_cache = vibe_core::settings::settings_dir()
        .map(|settings| vibe_app::SettingsRoot::new(&settings).app_sources_cache());
    for entry in &document.artifacts {
        let source = dir.join(&entry.path);
        let label = format!("{} ({})", entry.filename, short(&entry.sha256));
        let outcome = match entry.place {
            wire::ArtifactPlace::AppCache => match &app_cache {
                Some(cache) => place_in_cache(&source, entry, cache),
                None => Err("no settings home is resolvable for the download cache".to_owned()),
            },
            wire::ArtifactPlace::Store => place_in_store(&source, entry),
        };
        match outcome {
            Ok(true) => imported.placed.push(label),
            Ok(false) => imported.already.push(label),
            Err(reason) => imported.refused.push((label, reason)),
        }
    }
    Ok(imported)
}

/// One artifact into `<cache>/<sha256>/<filename>`. Returns whether this
/// call wrote it.
fn place_in_cache(
    source: &Path,
    entry: &wire::BundledArtifact,
    cache: &Path,
) -> Result<bool, String> {
    let digest = entry.sha256.to_ascii_lowercase();
    let destination = cache.join(&digest).join(&entry.filename);
    if digest_of(&destination).as_deref() == Some(digest.as_str()) {
        // A verified entry is never overwritten: the bytes are the same
        // bytes by construction, and rewriting them would be a write to a
        // store whose whole contract is that we do not.
        return Ok(false);
    }
    let actual =
        digest_of(source).ok_or_else(|| format!("`{}` is missing from the bundle", entry.path))?;
    if actual != digest {
        return Err(format!(
            "`{}` hashes to {} but the manifest declares {digest}",
            entry.path,
            short(&actual)
        ));
    }
    let parent = destination.parent().unwrap_or(cache);
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    std::fs::copy(source, &destination).map_err(|error| error.to_string())?;
    Ok(true)
}

/// One package tree into the machine store, write-once.
fn place_in_store(source: &Path, entry: &wire::BundledArtifact) -> Result<bool, String> {
    let coordinate = entry
        .package
        .as_deref()
        .ok_or_else(|| "the manifest names no package for a store entry".to_owned())?;
    let (key, version) = coordinate
        .split_once('@')
        .ok_or_else(|| format!("`{coordinate}` is not `<group>/<name>@<version>`"))?;
    let (group, name) = key
        .split_once('/')
        .ok_or_else(|| format!("`{key}` is not `<group>/<name>`"))?;
    let group = vibe_core::Group::parse(group).map_err(|error| error.to_string())?;
    let version = semver::Version::parse(version).map_err(|error| error.to_string())?;
    if !source.is_dir() {
        return Err(format!("`{}` is missing from the bundle", entry.path));
    }
    let actual = vibe_registry::compute_content_hash(source).map_err(|error| error.to_string())?;
    if actual != entry.sha256 {
        return Err(format!(
            "`{}` hashes to {} but the manifest declares {}",
            entry.path,
            short(&actual),
            short(&entry.sha256)
        ));
    }
    match vibe_registry::insert_from(source, &group, name, &version)
        .map_err(|error| error.to_string())?
    {
        vibe_registry::InsertOutcome::Inserted(_) => Ok(true),
        vibe_registry::InsertOutcome::AlreadyPresent(_) => Ok(false),
    }
}

/// The SHA-256 of one file, or `None` when it cannot be read.
pub(crate) fn digest_of(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    Some(format!("{:x}", Sha256::digest(&bytes)))
}

/// Copy one file into `<dir>/artifacts/<sha256>/<filename>` and describe
/// it for the manifest.
pub(crate) fn stage_file(
    dir: &Path,
    file: &Path,
    digest: &str,
    filename: &str,
    origin: Option<String>,
    row: Option<String>,
) -> Result<wire::BundledArtifact> {
    let digest = digest.to_ascii_lowercase();
    let relative = PathBuf::from(ARTIFACTS).join(&digest).join(filename);
    let destination = dir.join(&relative);
    std::fs::create_dir_all(destination.parent().unwrap_or(dir))?;
    std::fs::copy(file, &destination)
        .with_context(|| format!("copying `{}` into the bundle", file.display()))?;
    let bytes = std::fs::metadata(&destination)
        .map(|meta| meta.len())
        .unwrap_or(0);
    Ok(wire::BundledArtifact {
        sha256: digest,
        path: vibe_core::machine_json_path(&relative),
        filename: filename.to_owned(),
        bytes: u32::try_from(bytes).unwrap_or(u32::MAX),
        place: wire::ArtifactPlace::AppCache,
        origin,
        package: None,
        row,
    })
}

/// A digest, shortened for a message.
fn short(digest: &str) -> String {
    let hex = digest.strip_prefix("sha256:").unwrap_or(digest);
    hex.chars().take(12).collect()
}
