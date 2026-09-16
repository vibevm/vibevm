//! Release-index discovery and strict application distribution verification.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#distribution");

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vibe_core::manifest::{ApplicationDistributionDecl, is_windows_unsafe_component};
use vibe_publish::release_manifest::{
    DISTRIBUTION_BUNDLE_MAX_BYTES, DISTRIBUTION_SOURCE_EXPANDED_MAX_BYTES,
    DISTRIBUTION_SOURCE_MAX_DEPTH, DISTRIBUTION_SOURCE_MAX_FILES,
    DISTRIBUTION_SOURCE_MAX_PATH_BYTES,
};
use zip::ZipArchive;

use super::model::{ApplicationIdentity, ApplicationSourceObservation};

const INDEX_PROTOCOL: &str = "vibe-application-distribution-index/1";
const BUNDLE_PROTOCOL: &str = "vibe-application-distribution/1";
const BUNDLE_MANIFEST: &str = "vibe-application-distribution.json";
const MAX_INDEX_BYTES: u64 = 1_048_576;
const MAX_MANIFEST_BYTES: u64 = 4_194_304;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DistributionIndex {
    pub protocol: String,
    pub application: ApplicationIdentity,
    pub release_tag: String,
    pub distributions: Vec<DistributionTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BundleManagement {
    pub runtime: String,
    pub entry: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BundleLauncher {
    pub command: String,
    pub path: String,
    pub destination: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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

pub fn fetch_index(locator: &ApplicationDistributionDecl) -> Result<Option<DistributionIndex>> {
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
    let index: DistributionIndex =
        serde_json::from_slice(&bytes).context("parsing application distribution index")?;
    validate_index(locator, &index)?;
    Ok(Some(index))
}

pub fn matching_target<'a>(
    index: &'a DistributionIndex,
    source_application: &ApplicationIdentity,
) -> Result<Option<&'a DistributionTarget>> {
    if index.protocol != INDEX_PROTOCOL
        || index.application.id != source_application.id
        || index.application.package != source_application.package
    {
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
) -> Result<Option<VerifiedDistribution>> {
    validate_target(target)?;
    let archive_path =
        staging.with_file_name(format!(".application-asset-{}.zip", std::process::id()));
    let mut response = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()?
        .get(&target.url)
        .send()
        .context("fetching application distribution")?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
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
    }
    file.sync_all()?;
    if size != target.size || format!("{:x}", hash.finalize()) != target.sha256 {
        bail!("application distribution digest or size differs from its release index");
    }
    let verified = verify_archive(&archive_path, target, selected, staging)?;
    let _ = fs::remove_file(&archive_path);
    Ok(Some(verified))
}

pub fn verify_archive(
    archive_path: &Path,
    target: &DistributionTarget,
    selected: &ApplicationIdentity,
    staging: &Path,
) -> Result<VerifiedDistribution> {
    if staging.exists() {
        bail!("distribution staging destination already exists");
    }
    fs::create_dir(staging).context("creating distribution staging directory")?;
    let result = (|| {
        let mut archive = ZipArchive::new(File::open(archive_path)?)?;
        if archive.is_empty() || archive.len() as u64 > DISTRIBUTION_SOURCE_MAX_FILES + 1 {
            bail!("distribution ZIP file count is invalid");
        }
        let manifest_bytes = read_entry(&mut archive, BUNDLE_MANIFEST, MAX_MANIFEST_BYTES)?;
        let manifest: BundleManifest = serde_json::from_slice(&manifest_bytes)
            .context("parsing application distribution manifest")?;
        validate_bundle(&manifest, target, selected)?;
        let expected_paths: BTreeSet<_> = manifest.files.iter().map(|f| f.path.clone()).collect();
        let mut actual_paths = BTreeSet::new();
        let mut manifest_entries = 0usize;
        for index in 0..archive.len() {
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
        let _ = fs::remove_dir_all(staging);
    }
    result
}

fn validate_index(locator: &ApplicationDistributionDecl, index: &DistributionIndex) -> Result<()> {
    if index.protocol != INDEX_PROTOCOL || index.release_tag != locator.release_tag {
        bail!("application distribution index protocol or tag is invalid");
    }
    validate_application_identity(&index.application)?;
    let mut targets = BTreeSet::new();
    for target in &index.distributions {
        validate_target(target)?;
        if !targets.insert((target.os.as_str(), target.arch.as_str())) {
            bail!("application distribution index repeats a platform target");
        }
    }
    Ok(())
}

fn validate_target(target: &DistributionTarget) -> Result<()> {
    if target.format != "zip"
        || target.size == 0
        || target.size > DISTRIBUTION_BUNDLE_MAX_BYTES
        || !https_url(&target.url)
        || !hex(&target.sha256, 64)
        || !git_oid(&target.source_commit)
        || !tree_hash(&target.source_tree)
        || !portable_token(&target.os)
        || !portable_token(&target.arch)
    {
        bail!("application distribution target is malformed");
    }
    Ok(())
}

fn validate_bundle(
    manifest: &BundleManifest,
    target: &DistributionTarget,
    expected: &ApplicationIdentity,
) -> Result<()> {
    let selected = &manifest.application;
    if manifest.protocol != BUNDLE_PROTOCOL
        || selected != expected
        || manifest.os != target.os
        || manifest.arch != target.arch
        || manifest.source_commit != target.source_commit
        || manifest.source_tree != target.source_tree
        || manifest.management.runtime != "builtin"
        || portable_path(&manifest.management.entry).is_err()
        || !manifest
            .files
            .iter()
            .any(|file| file.path == manifest.management.entry)
        || manifest.files.is_empty()
        || manifest.launchers.is_empty()
    {
        bail!("application distribution manifest differs from its index or source declaration");
    }
    let mut paths = BTreeSet::new();
    let mut expanded = 0u64;
    for file in &manifest.files {
        portable_path(&file.path)?;
        expanded = expanded
            .checked_add(file.size)
            .ok_or_else(|| anyhow::anyhow!("distribution expanded size overflows"))?;
        if !hex(&file.sha256, 64) || !paths.insert(file.path.to_ascii_lowercase()) {
            bail!("application distribution has an invalid or duplicate file");
        }
    }
    if expanded > DISTRIBUTION_SOURCE_EXPANDED_MAX_BYTES {
        bail!("application distribution expanded payload exceeds its limit");
    }
    let commands: BTreeSet<_> = selected.commands.iter().map(String::as_str).collect();
    let mut covered_commands = BTreeSet::new();
    let mut destinations = BTreeSet::new();
    for launcher in &manifest.launchers {
        portable_path(&launcher.path)?;
        portable_path(&launcher.destination)?;
        if launcher.destination.contains('/')
            || !commands.contains(launcher.command.as_str())
            || !launcher_destination_matches(&launcher.destination, &launcher.command)
            || !manifest.files.iter().any(|f| f.path == launcher.path)
            || !destinations.insert(launcher.destination.to_ascii_lowercase())
        {
            bail!("application distribution launcher is invalid or ambiguous");
        }
        covered_commands.insert(launcher.command.as_str());
    }
    if covered_commands != commands {
        bail!("application distribution does not cover every declared command");
    }
    Ok(())
}

fn launcher_destination_matches(destination: &str, command: &str) -> bool {
    destination == command
        || [".cmd", ".ps1", ".sh"]
            .iter()
            .any(|suffix| destination == format!("{command}{suffix}"))
}

fn validate_application_identity(value: &ApplicationIdentity) -> Result<()> {
    if !portable_token(&value.id) || value.commands.is_empty() {
        bail!("application distribution identity or commands are invalid");
    }
    for package in [&value.package, &value.installer_package] {
        vibe_core::Group::parse(&package.group)?;
        vibe_core::PackageName::parse(&package.name)?;
        semver::Version::parse(&package.version)?;
    }
    let mut commands = BTreeSet::new();
    if value
        .commands
        .iter()
        .any(|command| !portable_token(command) || !commands.insert(command))
    {
        bail!("application distribution commands are invalid or duplicate");
    }
    Ok(())
}

fn read_entry(archive: &mut ZipArchive<File>, name: &str, maximum: u64) -> Result<Vec<u8>> {
    let entry = archive
        .by_name(name)
        .with_context(|| format!("distribution ZIP omits `{name}`"))?;
    if entry.is_dir() || special_mode(entry.unix_mode()) || entry.size() > maximum {
        bail!("distribution manifest entry is invalid");
    }
    let mut bytes = Vec::with_capacity(entry.size() as usize);
    entry.take(maximum + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > maximum {
        bail!("distribution manifest exceeds its size limit");
    }
    Ok(bytes)
}

fn copy_hashed(
    input: &mut impl Read,
    output: &mut impl Write,
    hash: &mut Sha256,
    maximum: u64,
) -> Result<u64> {
    let mut total = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        total += read as u64;
        if total > maximum {
            bail!("distribution file exceeds its declared size");
        }
        hash.update(&buffer[..read]);
        output.write_all(&buffer[..read])?;
    }
    Ok(total)
}

fn portable_path(value: &str) -> Result<PathBuf> {
    if value.is_empty()
        || value.len() > DISTRIBUTION_SOURCE_MAX_PATH_BYTES
        || value.contains('\\')
        || value.starts_with('/')
        || value.contains(':')
    {
        bail!("distribution path is not portable");
    }
    let path = PathBuf::from(value);
    let components: Vec<_> = path.components().collect();
    if components.len() > DISTRIBUTION_SOURCE_MAX_DEPTH
        || components.iter().any(|part| match part {
            Component::Normal(value) => value.to_str().is_none_or(is_windows_unsafe_component),
            _ => true,
        })
    {
        bail!("distribution path contains traversal or an unsafe component");
    }
    Ok(path)
}

fn special_mode(mode: Option<u32>) -> bool {
    mode.is_some_and(|m| m & 0o170000 != 0 && m & 0o170000 != 0o100000)
}
fn portable_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'_'))
}
fn hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn git_oid(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && hex(value, value.len())
}
fn tree_hash(value: &str) -> bool {
    value
        .strip_prefix("sha256-tree/1:")
        .is_some_and(|v| hex(v, 64))
}
fn https_url(value: &str) -> bool {
    value.starts_with("https://")
        && !value.contains(['@', '?', '#', '\\'])
        && !value.bytes().any(|b| b.is_ascii_whitespace())
}

pub fn source_differs(
    target: &DistributionTarget,
    source: Option<&ApplicationSourceObservation>,
) -> bool {
    source.is_some_and(|source| source.source_tree != target.source_tree)
}

#[cfg(test)]
#[path = "distribution/tests.rs"]
mod tests;
