//! Closed, hash-verified bundle inspection and safe source extraction.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use vibe_publish::release_manifest::{
    BundleDistributionManifest, DISTRIBUTION_BOOTSTRAP_MAX_BYTES, DISTRIBUTION_BUNDLE_MAX_BYTES,
    DISTRIBUTION_COMPONENT_MAX_BYTES, DISTRIBUTION_MANIFEST_FILENAME,
    DISTRIBUTION_MANIFEST_MAX_BYTES, DISTRIBUTION_SOURCE_ARCHIVE_FILENAME,
    DISTRIBUTION_SOURCE_ARCHIVE_MAX_BYTES, DistributionAsset, DistributionComponent,
    DistributionComponentName,
};
use zip::ZipArchive;

#[path = "source_verify.rs"]
mod source_verify;
use source_verify::source_matches_archive;
#[path = "source_extract.rs"]
mod source_extract;
use source_extract::extract_source_archive;
pub(super) use source_extract::{collision_key, safe_source_path, special_file_mode};
#[path = "file_verify.rs"]
mod file_verify;
use file_verify::{
    copy_and_hash_bounded, copy_bounded, hash_regular_exact, open_hashed_regular_exact,
    read_regular_bounded,
};

use super::super::model::{InstallRecord, Kind, Origin, Profile, VersionId};
use super::super::placer;
use super::super::store::{BINARY_NAME, INDEX_BINARY_NAME, VersionStore};

const LICENSE_FILENAME: &str = "LICENSE.md";
const README_FILENAME: &str = "README.md";
const MAX_DOCUMENT_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug)]
pub(super) struct InstallOutcome {
    pub record: InstallRecord,
    pub home: PathBuf,
    pub reused: bool,
}

pub(super) fn verify_file_asset(path: &Path, asset: &DistributionAsset) -> Result<()> {
    if asset.size > DISTRIBUTION_BOOTSTRAP_MAX_BYTES {
        bail!("bootstrap asset exceeds distribution policy");
    }
    let digest = hash_regular_exact(path, asset.size, DISTRIBUTION_BOOTSTRAP_MAX_BYTES, true)?;
    require_digest(
        &format!("asset `{}`", asset.name),
        asset.size,
        &digest,
        asset.size,
        &asset.digest,
    )
}

pub(super) fn install_bundle(
    store: &VersionStore,
    bundle_path: &Path,
    expected_manifest: &BundleDistributionManifest,
    expected_asset: &DistributionAsset,
    force: bool,
) -> Result<InstallOutcome> {
    if expected_asset.size > DISTRIBUTION_BUNDLE_MAX_BYTES {
        bail!("distribution bundle exceeds distribution policy");
    }
    let (file, outer_hash) = open_hashed_regular_exact(
        bundle_path,
        expected_asset.size,
        DISTRIBUTION_BUNDLE_MAX_BYTES,
        false,
    )?;
    require_digest(
        "distribution bundle",
        expected_asset.size,
        &outer_hash,
        expected_asset.size,
        &expected_asset.digest,
    )?;

    let mut archive = ZipArchive::new(file).context("opening distribution ZIP")?;
    let embedded_bytes = read_bounded_entry(
        &mut archive,
        DISTRIBUTION_MANIFEST_FILENAME,
        DISTRIBUTION_MANIFEST_MAX_BYTES,
    )?;
    let manifest = BundleDistributionManifest::from_json_slice(&embedded_bytes)?;
    if &manifest != expected_manifest {
        bail!(
            "embedded `{DISTRIBUTION_MANIFEST_FILENAME}` does not match the selected aggregate platform"
        );
    }
    validate_platform_paths(&manifest)?;
    validate_outer_structure(&mut archive, &manifest)?;

    let id = VersionId::new(Kind::Tag, manifest.version.clone());
    let digest = outer_hash.trim_start_matches("sha256:");
    if !force && let Some(record) = reusable_record(store, &id, digest, &manifest)? {
        let home = store.instance_dir(&id, record.instance);
        return Ok(InstallOutcome {
            record,
            home,
            reused: true,
        });
    }

    let instance = store.alloc_instance()?;
    let staging = store
        .version_id_dir(&id)
        .join(format!(".bundle-staging-{instance}"));
    let staging_parent = staging.parent().context("bundle staging has no parent")?;
    store.guard_mutation_path(staging_parent)?;
    fs::create_dir_all(staging_parent)?;
    store.guard_mutation_tree(&staging)?;
    fs::create_dir(&staging)
        .with_context(|| format!("creating bundle staging `{}`", staging.display()))?;
    let mut cleanup = StagingCleanup(Some(staging.clone()));
    extract_verified_bundle(&mut archive, &manifest, &embedded_bytes, &staging)?;
    let home = placer::publish_staged_instance(store, &id, instance, &staging)?;
    cleanup.0 = None;

    let source_path = store.instance_source_dir(&id, instance);
    let source_path = source_path
        .canonicalize()
        .unwrap_or(source_path)
        .display()
        .to_string();
    let record = InstallRecord {
        kind: Kind::Tag,
        id: manifest.version.clone(),
        instance,
        commit: manifest.source_commit.clone(),
        toolchain: format!("prebuilt:{}", manifest.target),
        profile: Profile::Release,
        installed_at: chrono::Utc::now().to_rfc3339(),
        origin: Origin::Binary,
        source_path: Some(source_path),
        payload_sha256: Some(digest.to_string()),
        distribution_manifest_sha256: Some(format!("{:x}", Sha256::digest(&embedded_bytes))),
    };
    store.record_install(record.clone())?;
    Ok(InstallOutcome {
        record,
        home,
        reused: false,
    })
}

fn reusable_record(
    store: &VersionStore,
    id: &VersionId,
    digest: &str,
    manifest: &BundleDistributionManifest,
) -> Result<Option<InstallRecord>> {
    let record = store
        .instances_of(id)?
        .into_iter()
        .filter(|record| record.payload_sha256.as_deref() == Some(digest))
        .max_by_key(|record| record.instance);
    Ok(record.filter(|record| verified_existing_instance(store, record, manifest)))
}

fn verified_existing_instance(
    store: &VersionStore,
    record: &InstallRecord,
    expected: &BundleDistributionManifest,
) -> bool {
    if record.origin != Origin::Binary
        || record.kind != Kind::Tag
        || record.id.strip_prefix('v').unwrap_or(&record.id) != expected.version
        || record.commit != expected.source_commit
        || record.toolchain != format!("prebuilt:{}", expected.target)
    {
        return false;
    }
    let home = store.instance_dir(&record.version_id(), record.instance);
    if store.guard_mutation_tree(&home).is_err() {
        return false;
    }
    let Ok(manifest_bytes) = read_regular_bounded(
        &home.join(DISTRIBUTION_MANIFEST_FILENAME),
        DISTRIBUTION_MANIFEST_MAX_BYTES,
    ) else {
        return false;
    };
    if !manifest_root_matches(
        record.distribution_manifest_sha256.as_deref(),
        &manifest_bytes,
    ) {
        return false;
    }
    let manifest = BundleDistributionManifest::from_json_slice(&manifest_bytes).ok();
    if manifest.as_ref() != Some(expected) {
        return false;
    }
    for component in &expected.components {
        let path = match component.name {
            DistributionComponentName::Vibe => {
                store.binary_path(&record.version_id(), record.instance)
            }
            DistributionComponentName::VibeIndex => {
                store.index_binary_path(&record.version_id(), record.instance)
            }
        };
        let Ok(digest) = hash_regular_exact(
            &path,
            component.size,
            DISTRIBUTION_COMPONENT_MAX_BYTES,
            true,
        ) else {
            return false;
        };
        if digest != component.digest {
            return false;
        }
    }
    let source_archive = home.join(DISTRIBUTION_SOURCE_ARCHIVE_FILENAME);
    let Ok(digest) = hash_regular_exact(
        &source_archive,
        expected.source_archive.size,
        DISTRIBUTION_SOURCE_ARCHIVE_MAX_BYTES,
        false,
    ) else {
        return false;
    };
    if digest != expected.source_archive.digest {
        return false;
    }
    source_matches_archive(
        &source_archive,
        &store.instance_source_dir(&record.version_id(), record.instance),
    )
    .unwrap_or(false)
}

fn manifest_root_matches(expected: Option<&str>, bytes: &[u8]) -> bool {
    expected.is_some_and(|expected| format!("{:x}", Sha256::digest(bytes)) == expected)
}

pub(super) fn installed_bundle_intact(store: &VersionStore, record: &InstallRecord) -> bool {
    let home = store.instance_dir(&record.version_id(), record.instance);
    let Some(manifest) = read_regular_bounded(
        &home.join(DISTRIBUTION_MANIFEST_FILENAME),
        DISTRIBUTION_MANIFEST_MAX_BYTES,
    )
    .ok()
    .and_then(|bytes| BundleDistributionManifest::from_json_slice(&bytes).ok()) else {
        return false;
    };
    verified_existing_instance(store, record, &manifest)
}

fn validate_platform_paths(manifest: &BundleDistributionManifest) -> Result<()> {
    let windows = manifest.target == "x86_64-pc-windows-msvc";
    for component in &manifest.components {
        let expected = match (&component.name, windows) {
            (DistributionComponentName::Vibe, true) => "vibe.exe",
            (DistributionComponentName::Vibe, false) => "vibe",
            (DistributionComponentName::VibeIndex, true) => "vibe-index.exe",
            (DistributionComponentName::VibeIndex, false) => "vibe-index",
        };
        if component.path != expected {
            bail!(
                "component `{}` path `{}` is not canonical for target `{}` (expected `{expected}`)",
                component.name.as_str(),
                component.path,
                manifest.target
            );
        }
    }
    if manifest.source_archive.path != DISTRIBUTION_SOURCE_ARCHIVE_FILENAME {
        bail!(
            "source archive path `{}` is not canonical",
            manifest.source_archive.path
        );
    }
    Ok(())
}

fn validate_outer_structure(
    archive: &mut ZipArchive<fs::File>,
    manifest: &BundleDistributionManifest,
) -> Result<()> {
    let mut expected = BTreeSet::from([
        DISTRIBUTION_MANIFEST_FILENAME.to_string(),
        DISTRIBUTION_SOURCE_ARCHIVE_FILENAME.to_string(),
        LICENSE_FILENAME.to_string(),
        README_FILENAME.to_string(),
    ]);
    expected.extend(
        manifest
            .components
            .iter()
            .map(|component| component.path.clone()),
    );
    let mut actual = BTreeSet::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        let name = entry.name().to_string();
        if entry.is_dir() || !safe_root_file(&name) || special_file_mode(entry.unix_mode(), false) {
            bail!("unsafe outer distribution ZIP entry `{name}`");
        }
        if !actual.insert(name.clone()) {
            bail!("duplicate outer distribution ZIP entry `{name}`");
        }
    }
    if actual != expected {
        bail!("distribution ZIP structure mismatch; expected {expected:?}, got {actual:?}");
    }
    Ok(())
}

fn extract_verified_bundle(
    archive: &mut ZipArchive<fs::File>,
    manifest: &BundleDistributionManifest,
    embedded_manifest: &[u8],
    staging: &Path,
) -> Result<()> {
    let bin_dir = staging.join("bin");
    fs::create_dir_all(&bin_dir)?;
    for component in &manifest.components {
        let destination = match component.name {
            DistributionComponentName::Vibe => bin_dir.join(BINARY_NAME),
            DistributionComponentName::VibeIndex => bin_dir.join(INDEX_BINARY_NAME),
        };
        extract_hashed_entry(archive, component, &destination)?;
        make_executable(&destination)?;
    }

    let source_zip = staging.join(DISTRIBUTION_SOURCE_ARCHIVE_FILENAME);
    extract_hashed_named_entry(
        archive,
        &manifest.source_archive.path,
        manifest.source_archive.size,
        &manifest.source_archive.digest,
        &source_zip,
    )?;
    let source_dir = staging.join("source");
    extract_source_archive(&source_zip, &source_dir)?;
    if !source_dir.join("Cargo.toml").is_file()
        || !source_dir.join("crates").join("vibe-cli").is_dir()
    {
        bail!("source archive does not contain the vibevm repository root");
    }

    fs::write(
        staging.join(DISTRIBUTION_MANIFEST_FILENAME),
        embedded_manifest,
    )?;
    copy_bounded_entry(archive, LICENSE_FILENAME, MAX_DOCUMENT_BYTES, staging)?;
    copy_bounded_entry(archive, README_FILENAME, MAX_DOCUMENT_BYTES, staging)?;
    Ok(())
}

fn extract_hashed_entry(
    archive: &mut ZipArchive<fs::File>,
    component: &DistributionComponent,
    destination: &Path,
) -> Result<()> {
    if component.size > DISTRIBUTION_COMPONENT_MAX_BYTES {
        bail!("component exceeds distribution policy");
    }
    extract_hashed_named_entry(
        archive,
        &component.path,
        component.size,
        &component.digest,
        destination,
    )
}

fn extract_hashed_named_entry(
    archive: &mut ZipArchive<fs::File>,
    name: &str,
    expected_size: u64,
    expected_digest: &str,
    destination: &Path,
) -> Result<()> {
    if name == DISTRIBUTION_SOURCE_ARCHIVE_FILENAME
        && expected_size > DISTRIBUTION_SOURCE_ARCHIVE_MAX_BYTES
    {
        bail!("source archive exceeds distribution policy");
    }
    let mut entry = archive.by_name(name)?;
    if entry.size() != expected_size {
        bail!(
            "ZIP entry `{name}` size {} does not match manifest {expected_size}",
            entry.size()
        );
    }
    let mut destination_file = fs::File::create(destination)?;
    let (actual_size, actual_digest) =
        copy_and_hash_bounded(&mut entry, &mut destination_file, expected_size)?;
    require_digest(
        &format!("ZIP entry `{name}`"),
        actual_size,
        &actual_digest,
        expected_size,
        expected_digest,
    )
}

fn read_bounded_entry(
    archive: &mut ZipArchive<fs::File>,
    name: &str,
    maximum: u64,
) -> Result<Vec<u8>> {
    let mut entry = archive.by_name(name)?;
    if entry.size() > maximum {
        bail!("ZIP entry `{name}` exceeds the {maximum}-byte limit");
    }
    let mut bytes = Vec::with_capacity(entry.size() as usize);
    copy_bounded(&mut entry, &mut bytes, maximum)?;
    Ok(bytes)
}

fn copy_bounded_entry(
    archive: &mut ZipArchive<fs::File>,
    name: &str,
    maximum: u64,
    staging: &Path,
) -> Result<()> {
    let mut entry = archive.by_name(name)?;
    if entry.size() > maximum {
        bail!("ZIP entry `{name}` exceeds the {maximum}-byte limit");
    }
    let mut destination = fs::File::create(staging.join(name))?;
    copy_bounded(&mut entry, &mut destination, maximum)?;
    Ok(())
}

fn safe_root_file(name: &str) -> bool {
    !name.is_empty() && !name.contains(['/', '\\', ':']) && name != "." && name != ".."
}

fn require_digest(
    what: &str,
    actual_size: u64,
    actual_digest: &str,
    expected_size: u64,
    expected_digest: &str,
) -> Result<()> {
    if actual_size != expected_size || actual_digest != expected_digest {
        bail!(
            "{what} integrity mismatch: expected {expected_size} bytes/{expected_digest}, got {actual_size} bytes/{actual_digest}"
        );
    }
    Ok(())
}

fn make_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

struct StagingCleanup(Option<PathBuf>);

impl Drop for StagingCleanup {
    fn drop(&mut self) {
        if let Some(path) = self.0.take() {
            let _ = fs::remove_dir_all(path);
        }
    }
}

#[cfg(test)]
mod trust_tests {
    use super::*;

    #[test]
    fn local_manifest_requires_the_authenticated_byte_root() {
        let bytes = b"authenticated manifest";
        let root = format!("{:x}", Sha256::digest(bytes));
        assert!(manifest_root_matches(Some(&root), bytes));
        assert!(!manifest_root_matches(
            Some(&root),
            b"coordinated replacement"
        ));
        assert!(!manifest_root_matches(None, bytes));
    }
}
