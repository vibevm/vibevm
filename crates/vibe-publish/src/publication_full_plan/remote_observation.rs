//! Pure fixed-target GitHub observation normalization and fingerprinting.
//! Input values are untrusted read data, not an authenticated HTTP capture,
//! remote CAS token, approval, or evidence that a prior write was ours.

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};
use vibe_wire::generated::publication_effect_plan::{
    PublicationObservedAsset, PublicationObservedRelease, PublicationObservedReleaseAbsent,
    PublicationObservedReleasePresent, PublicationObservedTag, PublicationObservedTagAbsent,
    PublicationObservedTagPresent,
};

use crate::github_release::{GithubGitRef, GithubRelease, GithubReleaseAsset};
use crate::publication_effect_plan::CapturedPublicationRemote;

const DOMAIN: &[u8] = b"vibevm/github-publication-observation/e1\0";
const MAX_ASSETS: usize = 32 * 100;
const MAX_TEXT: usize = 512;
const MAX_URL: usize = 2048;
/// Bounded release notes: large enough for ordinary mutable release notes,
/// still below the future bounded one-response JSON read budget.
const MAX_BODY: usize = 256 * 1024;
const TAG: &str = "v1.1.0";
const REFERENCE: &str = "refs/tags/v1.1.0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AssetDigestEvidence {
    pub asset_id: u64,
    pub size_bytes: u64,
    pub sha256: String,
}

/// One fully enumerated read epoch. `assets: None` is legal only when the
/// release is absent; a present release needs a complete (possibly empty)
/// listing. Digest evidence is accepted only for metadata with no digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RemoteSnapshot {
    pub tag: Option<GithubGitRef>,
    pub release: Option<GithubRelease>,
    pub assets: Option<Vec<GithubReleaseAsset>>,
    pub streamed_digests: Vec<AssetDigestEvidence>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(crate) enum ObservationError {
    #[error("remote observation refused: {0}")]
    Refused(String),
    #[error("remote observation unavailable: {0}")]
    Unavailable(String),
    #[error("remote observation changed between complete reads")]
    Drift,
}

fn refused(reason: impl Into<String>) -> ObservationError {
    ObservationError::Refused(reason.into())
}

fn bounded(value: &str, cap: usize) -> bool {
    value.len() <= cap && value.trim() == value && !value.chars().any(char::is_control)
}

fn bounded_body(value: &str) -> bool {
    value.len() <= MAX_BODY
        && !value
            .chars()
            .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
}

fn sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
    })
}

fn frame(hash: &mut Sha256, tag: &[u8], bytes: &[u8]) {
    hash.update((tag.len() as u64).to_be_bytes());
    hash.update(tag);
    hash.update((bytes.len() as u64).to_be_bytes());
    hash.update(bytes);
}

fn text(hash: &mut Sha256, tag: &[u8], value: &str) {
    frame(hash, tag, value.as_bytes());
}

fn optional_text(hash: &mut Sha256, tag: &[u8], value: Option<&str>) {
    frame(hash, tag, &[u8::from(value.is_some())]);
    if let Some(value) = value {
        frame(hash, tag, value.as_bytes());
    }
}

fn positive_decimal(value: u64) -> String {
    value.to_string()
}

/// Pure normalization of one typed read epoch. A caller may not replace a
/// missing remote digest with the desired local subject digest.
pub(crate) fn normalize_fixed(
    snapshot: RemoteSnapshot,
    source_commit: &str,
) -> Result<CapturedPublicationRemote, ObservationError> {
    if !matches!(source_commit.len(), 40 | 64)
        || !source_commit
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
    {
        return Err(refused("selected source commit is not a 40-hex Git OID"));
    }
    let tag = match snapshot.tag {
        None => PublicationObservedTag::Absent(Box::new(PublicationObservedTagAbsent {})),
        Some(value) => {
            if value.reference != REFERENCE
                || value.object.kind != "commit"
                || value.object.sha != source_commit
            {
                return Err(refused("tag ref differs from fixed 1.1.0 source commit"));
            }
            PublicationObservedTag::Present(Box::new(PublicationObservedTagPresent {
                reference: value.reference,
                object_kind: value.object.kind,
                object_sha: value.object.sha,
            }))
        }
    };
    let mut hash = Sha256::new();
    hash.update(DOMAIN);
    text(&mut hash, b"target", "github.com/vibevm/vibevm");
    match &tag {
        PublicationObservedTag::Absent(_) => frame(&mut hash, b"tag-state", b"absent"),
        PublicationObservedTag::Present(value) => {
            frame(&mut hash, b"tag-state", b"present");
            text(&mut hash, b"tag-ref", &value.reference);
            text(&mut hash, b"tag-kind", &value.object_kind);
            text(&mut hash, b"tag-oid", &value.object_sha);
        }
    }
    let release = match snapshot.release {
        None => {
            if snapshot.assets.is_some() || !snapshot.streamed_digests.is_empty() {
                return Err(refused("absent release has asset observations"));
            }
            frame(&mut hash, b"release-state", b"absent");
            PublicationObservedRelease::Absent(Box::new(PublicationObservedReleaseAbsent {}))
        }
        Some(value) => {
            if matches!(tag, PublicationObservedTag::Absent(_))
                || value.id == 0
                || value.tag_name != TAG
                || value.target_commitish.is_empty()
                || !bounded(&value.target_commitish, MAX_TEXT)
                || value.name.as_deref().is_some_and(|v| !bounded(v, MAX_TEXT))
                || value.body.as_deref().is_some_and(|v| !bounded_body(v))
                || !bounded(&value.html_url, MAX_URL)
                || !bounded(&value.upload_url, MAX_URL)
            {
                return Err(refused("release identity or bounded metadata differs"));
            }
            let Some(assets) = snapshot.assets else {
                return Err(ObservationError::Unavailable(
                    "present release inventory is incomplete".into(),
                ));
            };
            if assets.len() > MAX_ASSETS {
                return Err(refused("asset inventory exceeds bounded page count"));
            }
            let mut evidence = BTreeMap::new();
            for row in snapshot.streamed_digests {
                if row.asset_id == 0
                    || !sha256(&row.sha256)
                    || evidence.insert(row.asset_id, row).is_some()
                {
                    return Err(refused(
                        "streamed digest evidence is malformed or duplicated",
                    ));
                }
            }
            let mut names = BTreeSet::new();
            let mut ids = BTreeSet::new();
            let mut observed = Vec::with_capacity(assets.len());
            let mut expanded = Vec::with_capacity(assets.len());
            for asset in assets {
                if asset.id == 0
                    || asset.size == 0
                    || !bounded(&asset.name, MAX_TEXT)
                    || asset.name.is_empty()
                    || !bounded(&asset.content_type, MAX_TEXT)
                    || asset.content_type.is_empty()
                    || !bounded(&asset.browser_download_url, MAX_URL)
                    || !names.insert(asset.name.clone())
                    || !ids.insert(asset.id)
                {
                    return Err(refused("asset ID/name/size/media is invalid or duplicated"));
                }
                let digest = match asset.digest {
                    Some(digest) => {
                        if evidence.contains_key(&asset.id) || !sha256(&digest) {
                            return Err(refused("asset metadata digest conflicts with evidence"));
                        }
                        digest
                    }
                    None => {
                        let Some(witness) = evidence.remove(&asset.id) else {
                            return Err(ObservationError::Unavailable(format!(
                                "asset {} has no SHA-256 metadata or streamed witness",
                                asset.id,
                            )));
                        };
                        if witness.size_bytes != asset.size {
                            return Err(refused(
                                "streamed digest size differs from asset metadata",
                            ));
                        }
                        witness.sha256
                    }
                };
                expanded.push((
                    asset.id,
                    asset.name.clone(),
                    asset.size,
                    digest.clone(),
                    asset.content_type,
                    asset.browser_download_url,
                ));
                observed.push(PublicationObservedAsset {
                    asset_id: positive_decimal(asset.id),
                    name: asset.name,
                    sha256: digest,
                    size_bytes: positive_decimal(asset.size),
                });
            }
            if !evidence.is_empty() {
                return Err(refused("streamed digest witness names an unlisted asset"));
            }
            expanded.sort_by_key(|row| row.0);
            observed.sort_by_key(|row| row.asset_id.parse::<u64>().unwrap());
            frame(&mut hash, b"release-state", b"present");
            frame(&mut hash, b"release-id", &value.id.to_be_bytes());
            text(&mut hash, b"release-tag", &value.tag_name);
            frame(&mut hash, b"draft", &[u8::from(value.draft)]);
            frame(&mut hash, b"prerelease", &[u8::from(value.prerelease)]);
            text(&mut hash, b"target-commitish", &value.target_commitish);
            optional_text(&mut hash, b"release-name", value.name.as_deref());
            optional_text(&mut hash, b"release-body", value.body.as_deref());
            text(&mut hash, b"html-url", &value.html_url);
            text(&mut hash, b"upload-url", &value.upload_url);
            frame(
                &mut hash,
                b"asset-count",
                &(expanded.len() as u64).to_be_bytes(),
            );
            for (id, name, size, digest, media, url) in expanded {
                frame(&mut hash, b"asset-id", &id.to_be_bytes());
                text(&mut hash, b"asset-name", &name);
                frame(&mut hash, b"asset-size", &size.to_be_bytes());
                text(&mut hash, b"asset-sha", &digest);
                text(&mut hash, b"asset-media", &media);
                text(&mut hash, b"asset-url", &url);
            }
            // The release's generation is the same fingerprint as the whole
            // tag/release/inventory observation, computed after both states.
            PublicationObservedRelease::Present(Box::new(PublicationObservedReleasePresent {
                release_id: positive_decimal(value.id),
                tag: value.tag_name,
                draft: value.draft,
                prerelease: value.prerelease,
                target_commitish: value.target_commitish,
                name: value.name,
                body: value.body,
                inventory_complete: true,
                assets: observed,
                generation_ref: String::new(),
            }))
        }
    };
    let generation_ref = format!("sha256:{:x}", hash.finalize());
    let release = match release {
        PublicationObservedRelease::Present(mut value) => {
            value.generation_ref = generation_ref.clone();
            PublicationObservedRelease::Present(value)
        }
        absent => absent,
    };
    Ok(CapturedPublicationRemote {
        generation_ref,
        tag,
        release,
    })
}

/// Each invocation must return an independently complete typed read epoch:
/// tag, draft-aware release and all paginated assets. Missing metadata SHA
/// requires a fresh streamed same-ID/size digest witness in THAT epoch.
/// A future HTTP adapter must enforce bounded responses and page exhaustion;
/// implementing this port grants no write or remote CAS authority.
pub(crate) trait RemoteReadPort {
    fn read_complete(&mut self) -> Result<RemoteSnapshot, ObservationError>;
}

/// Compare two independently obtained complete read epochs. This fingerprint
/// is useful for drift detection, never an atomic remote precondition grant.
pub(crate) fn observe_twice(
    port: &mut dyn RemoteReadPort,
    source_commit: &str,
) -> Result<CapturedPublicationRemote, ObservationError> {
    let before = normalize_fixed(port.read_complete()?, source_commit)?;
    let after = normalize_fixed(port.read_complete()?, source_commit)?;
    if before != after {
        return Err(ObservationError::Drift);
    }
    Ok(before)
}

#[cfg(test)]
#[path = "remote_observation/tests.rs"]
mod tests;
