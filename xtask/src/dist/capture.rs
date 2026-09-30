//! Read-only local publication-subject composition from one selected commit.
//!
//! This owner binds the immutable Git identity and bounded commit-file bytes
//! to an independently pinned distribution output root. It does not create
//! an effect plan, a release, or a file in either tree.

use std::path::Path;

use anyhow::{Context, Result, bail};
use semver::Version;
use vibe_publish::publication_capture::{
    CommittedInputBytes, capture_distribution, subject_from_captured,
};
use vibe_publish::sha256_digest;
use vibe_safefs::{Project, StableFileState};
use vibe_wire::generated::publication_subject::{PublicationSmoke, PublicationSubject};

use super::build;
use super::release;
use super::snapshot::{self, GitIdentity};

const INSTALLER_MAX_BYTES: u64 = 1024 * 1024;

/// Inspectable exact subject, with its pinned source identity and aggregate
/// payload bytes. This is neither smoke readiness nor remote write authority.
pub(crate) struct PreparedSubjectCapture {
    pub(crate) identity: GitIdentity,
    pub(crate) subject: PublicationSubject,
    pub(crate) subject_digest: String,
    pub(crate) aggregate_bytes: Vec<u8>,
}

/// Capture from a fully committed HEAD and one held output directory.
/// Installer/Cargo.lock bytes are read from that selected commit, never from
/// dirty working files; the output Project is opened once and only observed.
pub(crate) fn capture_prepared_subject(
    repo_root: &Path,
    out_dir: &Path,
    smoke: PublicationSmoke,
) -> Result<PreparedSubjectCapture> {
    let identity = snapshot::committed_identity(repo_root, true)?;
    let version = build::workspace_version_at_commit(repo_root, &identity)?;
    if version != Version::new(1, 1, 0) {
        bail!("prepared publication requires the current mutable 1.1.0 source, found `{version}`");
    }
    let committed = |name, cap| {
        snapshot::read_commit_file_bounded(repo_root, &identity.commit, name, cap)
            .with_context(|| format!("capturing `{name}` from pinned commit {}", identity.commit))
    };
    let cargo_lock = committed("Cargo.lock", snapshot::COMMITTED_CARGO_LOCK_MAX_BYTES)?;
    let bash = committed("distribution/install/install.sh", INSTALLER_MAX_BYTES)?;
    let powershell = committed("distribution/install/install.ps1", INSTALLER_MAX_BYTES)?;
    let cmd = committed("distribution/install/install.cmd", INSTALLER_MAX_BYTES)?;

    let output_root = build::absolute_out_dir(repo_root, out_dir);
    let output = Project::open(&output_root)
        .with_context(|| format!("pinning distribution output `{}`", output_root.display()))?;
    let captured = capture_distribution(&output, "1.1.0")
        .context("capturing five exact local distribution outputs")?;
    // The compact stable witnesses alone do not prove what each ZIP embeds.
    // Re-read only through the same pinned root, join each byte string to its
    // exact captured digest/length, then reuse the existing closed ZIP and
    // embedded-manifest verifier before constructing the subject.
    for (fragment, fragment_state, bundle_state, bootstrap_state) in captured.exact_targets() {
        let fragment_name = build::fragment_asset_name(&version, &fragment.target);
        let fragment_bytes = reread_exact(
            &output,
            &fragment_name,
            vibe_publish::DISTRIBUTION_MANIFEST_MAX_BYTES,
            fragment_state,
        )?;
        let rechecked_fragment =
            vibe_publish::PlatformDistributionFragment::from_json_slice(&fragment_bytes)?;
        if &rechecked_fragment != fragment {
            bail!("fragment `{fragment_name}` changed after its stable capture");
        }
        let bundle = reread_exact(
            &output,
            &fragment.asset.name,
            vibe_publish::DISTRIBUTION_BUNDLE_MAX_BYTES,
            bundle_state,
        )?;
        let bootstrap = reread_exact(
            &output,
            &fragment.bootstrap.name,
            vibe_publish::DISTRIBUTION_BOOTSTRAP_MAX_BYTES,
            bootstrap_state,
        )?;
        release::verify_platform_bytes(fragment, &bundle, &bootstrap)
            .with_context(|| format!("verifying embedded bytes for `{}`", fragment.target))?;
    }
    let subject = subject_from_captured(
        &captured,
        CommittedInputBytes {
            commit: &identity.commit,
            tree: &identity.tree,
            cargo_lock: &cargo_lock,
            bash_installer: &bash,
            powershell_installer: &powershell,
            cmd_installer: &cmd,
            smoke,
        },
    )
    .context("constructing inspectable publication subject")?;
    Ok(PreparedSubjectCapture {
        identity,
        subject: subject.subject,
        subject_digest: subject.subject_digest,
        aggregate_bytes: subject.aggregate_bytes,
    })
}

fn reread_exact(
    project: &Project,
    name: &str,
    cap: u64,
    captured: &StableFileState,
) -> Result<Vec<u8>> {
    let cap = usize::try_from(cap).context("distribution byte cap exceeds this host's usize")?;
    let bytes = project
        .read_file_bounded(name, cap)
        .with_context(|| format!("re-reading captured distribution asset `{name}`"))?
        .with_context(|| format!("captured distribution asset `{name}` disappeared"))?;
    if u64::try_from(bytes.len()).ok() != Some(captured.bytes)
        || sha256_digest(&bytes) != format!("sha256:{}", captured.sha256)
    {
        bail!("captured distribution asset `{name}` changed on re-read");
    }
    Ok(bytes)
}

#[cfg(test)]
#[path = "capture/tests.rs"]
mod tests;
