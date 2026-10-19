//! Direct application artifacts and shared verified payload metadata.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#installer-options");

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use vibe_core::manifest::is_windows_unsafe_component;
use vibe_core::progress::Progress;
use vibe_publish::release_manifest::{
    DISTRIBUTION_BUNDLE_MAX_BYTES, DISTRIBUTION_SOURCE_EXPANDED_MAX_BYTES,
    DISTRIBUTION_SOURCE_MAX_DEPTH, DISTRIBUTION_SOURCE_MAX_FILES,
    DISTRIBUTION_SOURCE_MAX_PATH_BYTES,
};
use zip::ZipArchive;

use super::model::ApplicationIdentity;

const BUNDLE_PROTOCOL: &str = "vibe-application-distribution/1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleManifest {
    pub protocol: String,
    pub application: ApplicationIdentity,
    pub os: String,
    pub arch: String,
    pub libc: Option<String>,
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
    pub direct_archive: bool,
}

fn current_libc() -> Option<&'static str> {
    if cfg!(all(target_os = "linux", target_env = "musl")) {
        Some("musl")
    } else if cfg!(all(target_os = "linux", target_env = "gnu")) {
        Some("gnu")
    } else {
        None
    }
}

#[cfg(unix)]
fn apply_zip_permissions(path: &Path, mode: Option<u32>) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    if let Some(mode) = mode {
        fs::set_permissions(path, fs::Permissions::from_mode(mode & 0o777))?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn apply_zip_permissions(_path: &Path, _mode: Option<u32>) -> Result<()> {
    Ok(())
}

mod codec;
mod direct;
use codec::{copy_hashed, portable_path, special_mode};
pub use direct::{DirectArchiveFormat, verify_direct_archive, verify_installer_file};

#[cfg(test)]
#[path = "distribution/tests.rs"]
mod tests;
