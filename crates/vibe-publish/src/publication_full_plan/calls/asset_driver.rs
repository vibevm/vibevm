//! One protected asset API call under a sealed ReadyRun. No public/CLI route
//! invokes this driver, and all remote writes still require a separate owner
//! decision after the exact local journal and before-image are checked.

use crate::github_release::{DraftAssetDigestEvidence, GithubReleaseClient, StreamedUploadError};
use crate::publication_full_plan::remote_observation::{RemoteReadPort, observe_twice};
use crate::publication_full_plan::session::ReadyRun;
use crate::publication_payload::vault::VaultAssetStream;

use super::super::journal;
use super::super::{Error, Result};
use super::asset_history::{self, AssetInspection};
use super::assets::{
    self, AssetAction, AssetLanded, AssetPending, LANDED_FORMAT, PENDING_FORMAT, PlannedAssetCall,
};
use super::{CallObservation, sha};

/// A single transport effect; this trait cannot itself advance the journal.
/// Its errors after Pending are always uncertainty, even when transport can
/// distinguish a local refusal from a lost remote response.
pub(super) trait AssetCallTransport {
    fn upload(
        &self,
        release_id: u64,
        call: &PlannedAssetCall,
        held: VaultAssetStream,
    ) -> std::result::Result<u64, String>;
    fn rename(&self, asset_id: u64, call: &PlannedAssetCall) -> std::result::Result<u64, String>;
}

impl AssetCallTransport for GithubReleaseClient {
    fn upload(
        &self,
        release_id: u64,
        call: &PlannedAssetCall,
        held: VaultAssetStream,
    ) -> std::result::Result<u64, String> {
        let result = self.upload_temporary_asset_streamed(
            release_id,
            &call.temporary_name,
            &held.media_type,
            held.size_bytes,
            &held.sha256,
            held.reader,
            held.completion,
        );
        match result {
            Ok(response)
                if response.asset_id > 0
                    && response.temporary_name == call.temporary_name
                    && response.size_bytes == call.size_bytes
                    && response.sha256 == call.sha256 =>
            {
                Ok(response.asset_id)
            }
            Err(StreamedUploadError::UncertainCandidate { candidate, .. })
                if candidate.asset_id > 0
                    && candidate.temporary_name == call.temporary_name
                    && candidate.size_bytes == call.size_bytes =>
            {
                let evidence: DraftAssetDigestEvidence = self
                    .verify_draft_asset_digest_streamed(
                        candidate.asset_id,
                        call.size_bytes,
                        &call.sha256,
                        call.size_bytes,
                    )
                    .map_err(|_| "candidate GET is uncertain".to_owned())?;
                (evidence.asset_id == candidate.asset_id
                    && evidence.size_bytes == call.size_bytes
                    && evidence.sha256 == call.sha256)
                    .then_some(candidate.asset_id)
                    .ok_or_else(|| "candidate GET differs".into())
            }
            _ => Err("temporary POST is uncertain".into()),
        }
    }

    fn rename(&self, asset_id: u64, call: &PlannedAssetCall) -> std::result::Result<u64, String> {
        let response = self
            .rename_temporary_asset_explicit(
                asset_id,
                &call.canonical_name,
                call.size_bytes,
                &call.sha256,
            )
            .map_err(|_| "rename PATCH is uncertain".to_owned())?;
        if response.asset_id != asset_id
            || response.canonical_name != call.canonical_name
            || response.size_bytes != call.size_bytes
        {
            return Err("rename response identity differs".into());
        }
        if response.response_sha256.is_none() {
            let evidence: DraftAssetDigestEvidence = self
                .verify_draft_asset_digest_streamed(
                    asset_id,
                    call.size_bytes,
                    &call.sha256,
                    call.size_bytes,
                )
                .map_err(|_| "renamed asset GET is uncertain".to_owned())?;
            if evidence.asset_id != asset_id
                || evidence.size_bytes != call.size_bytes
                || evidence.sha256 != call.sha256
            {
                return Err("renamed asset GET differs".into());
            }
        }
        Ok(asset_id)
    }
}

/// The production binding is fixed-target and still private. In this
/// campaign nothing routes a live GitHub client to it.
pub(super) fn drive_one_fixed(
    session: &ReadyRun<'_>,
    client: &GithubReleaseClient,
) -> Result<AssetInspection> {
    let mut reader =
        super::super::github_remote::GithubRemoteReadPort::new_fixed(client).map_err(Error::io)?;
    drive_one(session, &mut reader, client)
}

fn observed(
    reader: &mut dyn RemoteReadPort,
    source_commit: &str,
) -> std::result::Result<CallObservation, ()> {
    let remote = observe_twice(reader, source_commit).map_err(|_| ())?;
    Ok(CallObservation {
        generation_ref: remote.generation_ref,
        tag: remote.tag,
        release: remote.release,
    })
}

/// Exactly one selected UploadTemporary or RenameTemporary. Pending is
/// create-new/read back before any transport call. A later attach sees Pending
/// and never retries based on an identical remote asset.
fn drive_one(
    session: &ReadyRun<'_>,
    reader: &mut dyn RemoteReadPort,
    transport: &dyn AssetCallTransport,
) -> Result<AssetInspection> {
    let (call, release_id, asset_id, expected_before) =
        match asset_history::inspect_assets_proved(session)? {
            AssetInspection::Next {
                call,
                release_id,
                asset_id,
                before,
            } => (call, release_id, asset_id, before),
            other => return Ok(other),
        };
    let ordinal = call.ordinal;
    let release_number = release_id
        .parse::<u64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| Error::Refused("created draft release ID is invalid".into()))?;
    let rename_number = if matches!(call.action, AssetAction::RenameTemporary) {
        Some(
            asset_id
                .as_deref()
                .and_then(|id| id.parse::<u64>().ok())
                .filter(|id| *id > 0)
                .ok_or_else(|| Error::Refused("prior Upload Landed asset ID is invalid".into()))?,
        )
    } else {
        None
    };
    let before = observed(reader, &session.subject.source.commit)
        .map_err(|_| Error::Refused("remote before-image unavailable before Pending".into()))?;
    if !assets::same_observation(&before, &expected_before) {
        return Err(Error::Refused(
            "asset remote precondition changed before Pending".into(),
        ));
    }
    session.recheck_small()?;
    let pending = AssetPending {
        format: PENDING_FORMAT.into(),
        run_id: session.run_id.clone(),
        call: call.clone(),
        release_id,
        asset_id,
        before_sha256: sha(&serde_json::to_vec(&before)?),
        before: before.clone(),
    };
    if journal::publish(
        session.lease,
        &session.run_id,
        &assets::asset_suffix(ordinal, "pending"),
        &pending,
    )
    .is_err()
    {
        return Ok(AssetInspection::Uncertain { ordinal });
    }
    if session.recheck_small().is_err() {
        return Ok(AssetInspection::Uncertain { ordinal });
    }
    let receipt_id = match call.action {
        AssetAction::UploadTemporary => {
            let held = match session.open_selected_stream(call.asset_index, &call.canonical_name) {
                Ok(held)
                    if held.name == call.canonical_name
                        && held.media_type == call.media_type
                        && held.size_bytes == call.size_bytes
                        && held.sha256 == call.sha256 =>
                {
                    held
                }
                _ => return Ok(AssetInspection::Uncertain { ordinal }),
            };
            transport.upload(release_number, &call, held)
        }
        AssetAction::RenameTemporary => {
            let Some(id) = rename_number else {
                return Ok(AssetInspection::Uncertain { ordinal });
            };
            transport.rename(id, &call)
        }
    };
    let receipt_id = match receipt_id {
        Ok(id) if id > 0 => id.to_string(),
        _ => return Ok(AssetInspection::Uncertain { ordinal }),
    };
    let after = match observed(reader, &session.subject.source.commit) {
        Ok(after) => after,
        Err(_) => return Ok(AssetInspection::Uncertain { ordinal }),
    };
    let landed = AssetLanded {
        format: LANDED_FORMAT.into(),
        run_id: session.run_id.clone(),
        pending_sha256: sha(&serde_json::to_vec(&pending)?),
        receipt_id,
        after_sha256: sha(&serde_json::to_vec(&after)?),
        after,
    };
    if !assets::verify_transition(&pending, &landed) || session.recheck_small().is_err() {
        return Ok(AssetInspection::Uncertain { ordinal });
    }
    if journal::publish(
        session.lease,
        &session.run_id,
        &assets::asset_suffix(ordinal, "landed"),
        &landed,
    )
    .is_err()
    {
        return Ok(AssetInspection::Uncertain { ordinal });
    }
    match asset_history::inspect_assets_proved(session) {
        Ok(status) => Ok(status),
        Err(_) => Ok(AssetInspection::Uncertain { ordinal }),
    }
}

#[cfg(test)]
#[path = "asset_driver_tests.rs"]
mod tests;
