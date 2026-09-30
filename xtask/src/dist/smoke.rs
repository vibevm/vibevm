//! Stage exact distribution bytes for a later native launch probe.
//!
//! Staging is not execution evidence. The held temporary directory keeps the
//! verified executable available without writing to the user's real home.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use tempfile::TempDir;
use vibe_publish::{
    DISTRIBUTION_BUNDLE_MAX_BYTES, DistributionComponentName, PlatformDistributionFragment,
    SUPPORTED_DISTRIBUTION_TARGETS, sha256_digest,
};
use vibe_wire::generated::publication_subject::PublicationTarget;

use super::build::verified_bundle_entries;

/// A verified, locally staged bundle. No command has been run in this home.
#[allow(dead_code)]
pub(crate) struct StagedBundle {
    hold: TempDir,
    pub(crate) target: String,
    pub(crate) bundle_digest: String,
    pub(crate) home: PathBuf,
    pub(crate) executable: PathBuf,
}

/// Check one held bundle against both its fragment and publication target,
/// then copy only the two declared executables into a fresh temporary home.
/// The caller must still run the exact staged executable on a compatible
/// native host and produce a separate smoke result.
#[allow(dead_code)]
pub(crate) fn stage_verified_bundle(
    bundle_bytes: &[u8],
    expected_target: &str,
    fragment: &PlatformDistributionFragment,
    subject: &PublicationTarget,
) -> Result<StagedBundle> {
    fragment
        .validate()
        .context("validating platform fragment")?;
    ensure!(
        fragment.version == "1.1.0",
        "smoke stage requires mutable 1.1.0, found `{}`",
        fragment.version
    );
    ensure!(
        SUPPORTED_DISTRIBUTION_TARGETS.contains(&expected_target)
            && fragment.target == expected_target
            && subject.target == expected_target,
        "smoke target differs from the supported fragment/subject target"
    );
    let expected_name = format!("vibevm-1.1.0-{expected_target}.zip");
    ensure!(
        fragment.asset.name == expected_name
            && subject.bundle.name == expected_name
            && subject.bundle.media_type == "application/zip",
        "smoke bundle name or media type differs from the exact target"
    );
    ensure!(
        fragment.asset.size <= DISTRIBUTION_BUNDLE_MAX_BYTES
            && u64::try_from(bundle_bytes.len()).ok() == Some(fragment.asset.size)
            && subject.bundle.size_bytes == fragment.asset.size.to_string(),
        "smoke bundle size differs from fragment/subject"
    );
    let digest = sha256_digest(bundle_bytes);
    ensure!(
        digest == fragment.asset.digest && digest == subject.bundle.sha256,
        "smoke bundle SHA-256 differs from fragment/subject"
    );
    let bootstrap_name = if expected_target == "x86_64-pc-windows-msvc" {
        format!("vibe-bootstrap-{expected_target}.exe")
    } else {
        format!("vibe-bootstrap-{expected_target}")
    };
    ensure!(
        fragment.bootstrap.name == bootstrap_name
            && subject.bootstrap.name == bootstrap_name
            && subject.bootstrap.media_type == "application/octet-stream"
            && subject.bootstrap.size_bytes == fragment.bootstrap.size.to_string()
            && subject.bootstrap.sha256 == fragment.bootstrap.digest,
        "smoke bootstrap identity differs from fragment/subject"
    );
    let fragment_bytes = fragment
        .to_json_bytes()
        .context("serializing exact platform fragment")?;
    ensure!(
        subject.fragment.name == format!("vibevm-1.1.0-{expected_target}.fragment.json")
            && subject.fragment.media_type == "application/json"
            && subject.fragment.size_bytes == fragment_bytes.len().to_string()
            && subject.fragment.sha256 == sha256_digest(&fragment_bytes),
        "smoke fragment bytes differ from publication subject"
    );
    ensure!(
        subject.components.len() == fragment.bundle.components.len(),
        "smoke component set differs from publication subject"
    );
    for (actual, expected) in fragment.bundle.components.iter().zip(&subject.components) {
        ensure!(
            actual.name.as_str() == match expected.name {
                vibe_wire::generated::publication_subject::PublicationComponentName::Vibe => "vibe",
                vibe_wire::generated::publication_subject::PublicationComponentName::VibeIndex =>
                    "vibe-index",
            } && actual.path == expected.path
                && actual.size.to_string() == expected.size_bytes
                && actual.digest == expected.sha256,
            "smoke component differs from publication subject"
        );
    }
    let archive = &fragment.bundle.source_archive;
    ensure!(
        archive.path == subject.source_archive.path
            && archive.size.to_string() == subject.source_archive.size_bytes
            && archive.digest == subject.source_archive.sha256
            && archive.tree_oid == subject.source_archive.tree_oid,
        "smoke source archive differs from publication subject"
    );

    // This bounded reader rejects traversal, symlinks, undeclared files and
    // wrong embedded component/source/manifest digests before any file write.
    let entries = verified_bundle_entries(bundle_bytes, &fragment.bundle)
        .context("verifying closed distribution ZIP before staging")?;
    let vibe = fragment
        .bundle
        .components
        .iter()
        .find(|component| component.name == DistributionComponentName::Vibe)
        .context("validated bundle lacks vibe")?;
    let embedded_vibe = entries
        .iter()
        .find(|entry| entry.name == vibe.path)
        .context("verified ZIP lacks vibe")?;
    ensure!(
        embedded_vibe.bytes.len() as u64 == fragment.bootstrap.size
            && sha256_digest(&embedded_vibe.bytes) == fragment.bootstrap.digest,
        "embedded vibe differs from raw bootstrap identity"
    );

    let hold = tempfile::Builder::new()
        .prefix("vibevm-smoke-")
        .tempdir()
        .context("creating fresh isolated smoke home")?;
    let home = hold.path().to_path_buf();
    let bin = home.join("bin");
    fs::create_dir(&bin).context("creating isolated smoke bin")?;
    let mut executable = None;
    for component in &fragment.bundle.components {
        // Fragment validation constrains these paths to fixed basenames. The
        // explicit check keeps the staging boundary safe if that rule changes.
        let path = Path::new(&component.path);
        ensure!(
            path.file_name()
                .is_some_and(|name| name == path.as_os_str())
                && matches!(
                    component.path.as_str(),
                    "vibe" | "vibe.exe" | "vibe-index" | "vibe-index.exe"
                ),
            "smoke executable path is not a fixed basename"
        );
        let entry = entries
            .iter()
            .find(|entry| entry.name == component.path)
            .context("verified ZIP lacks declared executable")?;
        let destination = bin.join(path);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)
            .with_context(|| format!("creating staged executable `{}`", destination.display()))?;
        file.write_all(&entry.bytes)
            .with_context(|| format!("staging executable `{}`", destination.display()))?;
        file.sync_all().context("sealing staged executable bytes")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&destination, fs::Permissions::from_mode(0o700))
                .context("marking staged executable runnable")?;
        }
        if component.name == DistributionComponentName::Vibe {
            executable = Some(destination);
        }
    }
    let executable = executable.ok_or_else(|| anyhow::anyhow!("staged bundle lacks vibe"))?;
    Ok(StagedBundle {
        hold,
        target: expected_target.to_string(),
        bundle_digest: digest,
        home,
        executable,
    })
}

#[cfg(test)]
#[path = "smoke/tests.rs"]
mod tests;
