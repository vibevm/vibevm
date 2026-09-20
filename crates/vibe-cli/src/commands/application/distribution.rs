//! Release-index discovery and strict application distribution verification.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#distribution");

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use vibe_core::manifest::{ApplicationDistributionDecl, is_windows_unsafe_component};
use vibe_core::progress::Progress;
use vibe_publish::release_manifest::{
    DISTRIBUTION_BUNDLE_MAX_BYTES, DISTRIBUTION_SOURCE_EXPANDED_MAX_BYTES,
    DISTRIBUTION_SOURCE_MAX_DEPTH, DISTRIBUTION_SOURCE_MAX_FILES,
    DISTRIBUTION_SOURCE_MAX_PATH_BYTES,
};
use vibe_wire::generated::application::e1::{
    bundle_manifest as bundle_wire, distribution_index as index_wire,
};
use zip::ZipArchive;

use super::model::{ApplicationIdentity, ApplicationSourceObservation};

const INDEX_PROTOCOL: &str = "vibe-application-distribution-index/1";
const BUNDLE_PROTOCOL: &str = "vibe-application-distribution/1";
const BUNDLE_MANIFEST: &str = "vibe-application-distribution.json";
const MAX_INDEX_BYTES: u64 = 1_048_576;
const MAX_MANIFEST_BYTES: u64 = 4_194_304;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributionIndex {
    pub protocol: String,
    pub application: ApplicationIdentity,
    pub release_tag: String,
    pub distributions: Vec<DistributionTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributionTarget {
    pub os: String,
    pub arch: String,
    pub format: String,
    pub url: String,
    pub sha256: String,
    pub size: u64,
    pub source_commit: String,
    pub source_tree: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleManifest {
    pub protocol: String,
    pub application: ApplicationIdentity,
    pub os: String,
    pub arch: String,
    pub source_commit: String,
    pub source_tree: String,
    pub management: BundleManagement,
    pub launchers: Vec<BundleLauncher>,
    pub files: Vec<BundleFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleManagement {
    pub runtime: String,
    pub entry: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleLauncher {
    pub command: String,
    pub path: String,
    pub destination: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleFile {
    pub path: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug)]
pub struct VerifiedDistribution {
    pub manifest: BundleManifest,
    pub payload_root: PathBuf,
    pub asset_sha256: String,
}

pub fn release_index_url(locator: &ApplicationDistributionDecl) -> String {
    format!(
        "https://github.com/{}/releases/download/{}/{}",
        locator.repository, locator.release_tag, locator.index_asset
    )
}

pub fn fetch_index(
    locator: &ApplicationDistributionDecl,
    progress: &Progress,
) -> Result<Option<DistributionIndex>> {
    let task = progress.task("Fetching application binary metadata");
    task.detail(format!(
        "release: {} @ {}",
        locator.repository, locator.release_tag
    ));
    task.set_progress(0, None, "bytes");
    let result = (|| {
        let url = release_index_url(locator);
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
        let response = client
            .get(&url)
            .send()
            .context("fetching application distribution index")?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !response.status().is_success() {
            bail!(
                "application distribution index returned HTTP {}",
                response.status()
            );
        }
        if response
            .content_length()
            .is_some_and(|n| n > MAX_INDEX_BYTES)
        {
            bail!("application distribution index exceeds its size limit");
        }
        let bytes = response
            .bytes()
            .context("reading application distribution index")?;
        if bytes.len() as u64 > MAX_INDEX_BYTES {
            bail!("application distribution index exceeds its size limit");
        }
        let wire: index_wire::DistributionIndex =
            serde_json::from_slice(&bytes).context("parsing application distribution index")?;
        let index = distribution_index_from_wire(wire);
        validate_index(locator, &index)?;
        Ok(Some(index))
    })();
    match &result {
        Ok(Some(_)) => task.finish(),
        Ok(None) => task.skip("no binary metadata published"),
        Err(_) => task.fail("application binary metadata failed"),
    }
    result
}

pub fn matching_target<'a>(
    index: &'a DistributionIndex,
    source_application: &ApplicationIdentity,
) -> Result<Option<&'a DistributionTarget>> {
    if index.protocol != INDEX_PROTOCOL || index.application != *source_application {
        bail!("application distribution index identity is invalid");
    }
    if std::env::consts::OS != "windows" {
        return Ok(None);
    }
    let found: Vec<_> = index
        .distributions
        .iter()
        .filter(|row| row.os == std::env::consts::OS && row.arch == std::env::consts::ARCH)
        .collect();
    match found.as_slice() {
        [] => Ok(None),
        [row] => Ok(Some(*row)),
        _ => bail!("application distribution index has duplicate current-platform rows"),
    }
}

pub fn fetch_and_verify(
    target: &DistributionTarget,
    selected: &ApplicationIdentity,
    staging: &Path,
    progress: &Progress,
) -> Result<Option<VerifiedDistribution>> {
    validate_target(target)?;
    let download = progress.task("Downloading application distribution");
    download.detail(format!("platform: {} {}", target.os, target.arch));
    download.set_progress(0, Some(target.size), "bytes");
    let archive_path =
        staging.with_file_name(format!(".application-asset-{}.zip", std::process::id()));
    let mut response = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()?
        .get(&target.url)
        .send()
        .context("fetching application distribution")?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        download.skip("distribution asset not found");
        return Ok(None);
    }
    if !response.status().is_success() {
        bail!(
            "application distribution returned HTTP {}",
            response.status()
        );
    }
    if response.content_length().is_some_and(|n| n != target.size) {
        bail!("application distribution size differs from its release index");
    }
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&archive_path)
        .context("creating no-follow staged distribution archive")?;
    let mut hash = Sha256::new();
    let mut size = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = response.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        size = size
            .checked_add(read as u64)
            .ok_or_else(|| anyhow::anyhow!("distribution size overflow"))?;
        if size > target.size {
            bail!("application distribution exceeds its declared size");
        }
        hash.update(&buffer[..read]);
        file.write_all(&buffer[..read])?;
        download.set_progress(size, Some(target.size), "bytes");
    }
    file.sync_all()?;
    if size != target.size || format!("{:x}", hash.finalize()) != target.sha256 {
        download.fail("distribution digest or size differs");
        bail!("application distribution digest or size differs from its release index");
    }
    download.finish();
    let verified = verify_archive_observed(&archive_path, target, selected, staging, progress)?;
    let _ = fs::remove_file(&archive_path);
    Ok(Some(verified))
}

#[cfg(test)]
pub fn verify_archive(
    archive_path: &Path,
    target: &DistributionTarget,
    selected: &ApplicationIdentity,
    staging: &Path,
) -> Result<VerifiedDistribution> {
    verify_archive_observed(
        archive_path,
        target,
        selected,
        staging,
        &Progress::default(),
    )
}

fn verify_archive_observed(
    archive_path: &Path,
    target: &DistributionTarget,
    selected: &ApplicationIdentity,
    staging: &Path,
    progress: &Progress,
) -> Result<VerifiedDistribution> {
    let extraction = progress.task("Verifying and extracting application distribution");
    if staging.exists() {
        extraction.fail("distribution staging destination already exists");
        bail!("distribution staging destination already exists");
    }
    fs::create_dir(staging).context("creating distribution staging directory")?;
    let result = (|| {
        let mut archive = ZipArchive::new(File::open(archive_path)?)?;
        if archive.is_empty() || archive.len() as u64 > DISTRIBUTION_SOURCE_MAX_FILES + 1 {
            bail!("distribution ZIP file count is invalid");
        }
        let manifest_bytes = read_entry(&mut archive, BUNDLE_MANIFEST, MAX_MANIFEST_BYTES)?;
        let wire: bundle_wire::BundleManifest = serde_json::from_slice(&manifest_bytes)
            .context("parsing application distribution manifest")?;
        let manifest = bundle_manifest_from_wire(wire);
        validate_bundle(&manifest, target, selected)?;
        let expected_paths: BTreeSet<_> = manifest.files.iter().map(|f| f.path.clone()).collect();
        let mut actual_paths = BTreeSet::new();
        let mut manifest_entries = 0usize;
        let archive_entries = archive.len();
        extraction.set_progress(0, Some(archive_entries as u64), "entries");
        for index in 0..archive_entries {
            let mut entry = archive.by_index(index)?;
            let name = entry.name().replace('\\', "/");
            if name == BUNDLE_MANIFEST {
                manifest_entries += 1;
                if manifest_entries > 1 || entry.is_dir() || special_mode(entry.unix_mode()) {
                    bail!("distribution ZIP has a duplicate or special manifest entry");
                }
                continue;
            }
            if entry.is_dir()
                || special_mode(entry.unix_mode())
                || !actual_paths.insert(name.clone())
            {
                bail!(
                    "distribution ZIP contains duplicate, directory, link, or special entry `{name}`"
                );
            }
            if !expected_paths.contains(name.as_str()) {
                bail!("distribution ZIP contains undeclared entry `{name}`");
            }
            let declared = manifest
                .files
                .iter()
                .find(|file| file.path == name)
                .ok_or_else(|| anyhow::anyhow!("distribution ZIP entry is undeclared"))?;
            let output = staging.join(portable_path(&name)?);
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut out = File::create(&output)?;
            let mut digest = Sha256::new();
            let copied = copy_hashed(&mut entry, &mut out, &mut digest, declared.size)?;
            if copied != declared.size || format!("{:x}", digest.finalize()) != declared.sha256 {
                bail!("distribution file `{name}` differs from its manifest");
            }
            extraction.set_progress((index + 1) as u64, Some(archive_entries as u64), "entries");
        }
        if manifest_entries != 1 || actual_paths != expected_paths {
            bail!("distribution ZIP omits a declared file or its descriptor");
        }
        Ok(VerifiedDistribution {
            manifest,
            payload_root: staging.to_path_buf(),
            asset_sha256: target.sha256.clone(),
        })
    })();
    if result.is_err() {
        extraction.fail("distribution verification or extraction failed");
        let _ = fs::remove_dir_all(staging);
    } else {
        extraction.finish();
    }
    result
}

mod codec;
#[cfg(test)]
use codec::bundle_manifest_to_wire;
pub use codec::source_differs;
use codec::{
    bundle_manifest_from_wire, copy_hashed, distribution_index_from_wire, portable_path,
    read_entry, special_mode, validate_bundle, validate_index, validate_target,
};

#[cfg(test)]
#[path = "distribution/tests.rs"]
mod tests;
