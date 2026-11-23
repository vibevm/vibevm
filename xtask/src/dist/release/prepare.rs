//! `dist prepare`: place the release tag at the current commit and replace any
//! prior release for it with a fresh draft (PROP-019 `#RELEASE-MUTABLE`).
//!
//! A failed release run leaves its tag and draft at the commit it was
//! dispatched from. The next dispatch of the same version moves such a
//! leftover tag, but only while its release is still a draft: a tag of a
//! published release, or a tag with no release at all, is never moved.

use anyhow::Result;
use semver::Version;
use specmark::spec;
use vibe_publish::release_manifest::DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME;
use vibe_publish::{CreateGithubRelease, GithubRelease};

use super::{ReleaseHost, validate_tag_ref};
use crate::dist::snapshot::GitIdentity;

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#RELEASE-MUTABLE")]
pub(super) fn prepare_with(
    host: &dyn ReleaseHost,
    version: &Version,
    identity: &GitIdentity,
) -> Result<GithubRelease> {
    let mut request = CreateGithubRelease::for_version(version, identity.commit.clone());
    request.draft = true;
    request.name = format!("vibevm {version}");
    request.body = format!(
        "Mutable vibevm {version} distribution. Verify downloads through \
         `{DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME}`."
    );
    let tag = request.tag_name.as_str();
    let commit = identity.commit.as_str();
    // The release, once looked up to decide about a foreign tag.
    let mut looked_up = None;
    let reference = match host.find_tag_ref(tag)? {
        Some(reference) => match validate_tag_ref(&reference, tag, commit) {
            Ok(()) => reference,
            Err(mismatch) if reference.object.sha != commit => {
                let release = host.find_release(tag)?;
                if !release.as_ref().is_some_and(|release| release.draft) {
                    return Err(mismatch);
                }
                // GitHub refs have no compare-and-swap: a draft published
                // between this lookup and the move would see its tag moved.
                // The delete below already had the same window.
                looked_up = Some(release);
                host.move_tag_ref(tag, commit)?
            }
            Err(mismatch) => return Err(mismatch),
        },
        None => host.create_tag_ref(tag, commit)?,
    };
    validate_tag_ref(&reference, tag, commit)?;
    let existing = match looked_up {
        Some(release) => release,
        None => host.find_release(tag)?,
    };
    if let Some(existing) = existing {
        host.delete_release(existing.id)?;
    }
    host.create_release(&request)
}
