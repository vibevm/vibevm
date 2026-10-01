//! Fixed-target, read-only GitHub transport for one complete observation
//! epoch. The pure normalizer calls this twice; neither read is remote CAS,
//! owner journal evidence, or permission to publish.

use crate::github_release::GithubReleaseClient;
use crate::release_manifest::DISTRIBUTION_BUNDLE_MAX_BYTES;

use super::remote_observation::{
    AssetDigestEvidence, ObservationError, RemoteReadPort, RemoteSnapshot,
};

const TAG: &str = "v1.1.0";

pub(super) struct GithubRemoteReadPort<'a> {
    client: &'a GithubReleaseClient,
}

impl<'a> GithubRemoteReadPort<'a> {
    /// Only a client pinned to the canonical API and upload hosts and the
    /// fixed repository can be normalized as this campaign's remote target.
    pub(super) fn new_fixed(client: &'a GithubReleaseClient) -> Result<Self, ObservationError> {
        if !client.is_fixed_publication_target() {
            return Err(ObservationError::Refused(
                "GitHub client is not fixed to github.com/vibevm/vibevm".into(),
            ));
        }
        Ok(Self { client })
    }

    #[cfg(test)]
    pub(super) fn new_loopback(client: &'a GithubReleaseClient) -> Result<Self, ObservationError> {
        if !client.is_fixed_publication_loopback() {
            return Err(ObservationError::Refused(
                "test GitHub client is not fixed to loopback vibevm/vibevm".into(),
            ));
        }
        Ok(Self { client })
    }
}

impl RemoteReadPort for GithubRemoteReadPort<'_> {
    fn read_complete(&mut self) -> Result<RemoteSnapshot, ObservationError> {
        let latest = self
            .client
            .read_latest_release_bounded()
            .map_err(|error| ObservationError::Unavailable(error.to_string()))?;
        let tag = self
            .client
            .read_tag_ref_bounded(TAG)
            .map_err(|error| ObservationError::Unavailable(error.to_string()))?;
        let release = self
            .client
            .read_release_by_tag_bounded(TAG)
            .map_err(|error| ObservationError::Unavailable(error.to_string()))?;
        let (assets, streamed_digests) = match release.as_ref() {
            None => (None, Vec::new()),
            Some(release) => {
                let assets = self
                    .client
                    .read_all_assets_bounded(release.id)
                    .map_err(|error| ObservationError::Unavailable(error.to_string()))?;
                let mut digests = Vec::new();
                for asset in &assets {
                    if asset.digest.is_none() {
                        // Foreign/prior bytes are observed, never filled from
                        // the local desired subject. The global bundle cap is
                        // an absolute ceiling; normalizer validates rows.
                        let evidence = self
                            .client
                            .capture_draft_asset_digest_streamed(
                                asset.id,
                                asset.size,
                                DISTRIBUTION_BUNDLE_MAX_BYTES,
                            )
                            .map_err(|error| ObservationError::Unavailable(error.to_string()))?;
                        digests.push(AssetDigestEvidence {
                            asset_id: evidence.asset_id,
                            size_bytes: evidence.size_bytes,
                            sha256: evidence.sha256,
                        });
                    }
                }
                (Some(assets), digests)
            }
        };
        Ok(RemoteSnapshot {
            latest,
            tag,
            release,
            assets,
            streamed_digests,
        })
    }
}

#[cfg(test)]
#[path = "github_remote_tests.rs"]
mod tests;
