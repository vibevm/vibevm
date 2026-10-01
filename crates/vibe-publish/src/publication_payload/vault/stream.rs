//! Selected held asset streams after a complete run vault.

use super::*;
use vibe_safefs::{StreamCompletion, VerifiedStreamReader};

/// Exact catalogue metadata and a move-only held reader. These bytes remain
/// local evidence: opening the stream grants no remote call or effect retry.
pub(crate) struct VaultAssetStream {
    pub(crate) name: String,
    pub(crate) media_type: String,
    pub(crate) size_bytes: u64,
    pub(crate) sha256: String,
    pub(crate) reader: VerifiedStreamReader,
    pub(crate) completion: StreamCompletion,
}

/// Select only one canonical catalogue entry from the run-scoped, fully
/// verified vault. Full-plan approval redemption remains a separate gate.
#[allow(clippy::too_many_arguments)]
pub(crate) fn open_vault_asset_stream(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    run_id: &str,
    subject: &PublicationSubject,
    expected: &PublicationSubjectExpectations<'_>,
    asset_index: usize,
    asset_name: &str,
) -> Result<VaultAssetStream, VaultError> {
    let row = SubmissionReservations::inspect_existing_key(lease, namespace, scope, key)
        .map_err(|error| VaultError::Refused(format!("C.1 inspection: {error}")))?
        .ok_or_else(|| VaultError::Refused("C.1 key has no reservation".into()))?;
    if row.retired
        || row.domain_run_id != run_id
        || row.journal_admission != JournalAdmission::Ready
    {
        return Err(VaultError::Refused(
            "C.1 run is foreign, retired or not Ready".into(),
        ));
    }
    let catalogue = prepared_catalogue(subject, expected)?;
    let entry = catalogue
        .entries
        .get(asset_index)
        .ok_or_else(|| VaultError::Refused("asset index is outside exact catalogue".into()))?;
    if entry.name != asset_name {
        return Err(VaultError::Refused(
            "asset name differs from catalogue index".into(),
        ));
    }
    match inspect_payload_vault(lease, namespace, scope, key, run_id, subject, expected) {
        VaultInspection::Complete {
            run_id: observed,
            catalogue_digest,
            ..
        } if observed == run_id && catalogue_digest == catalogue.digest => {}
        other => {
            return Err(VaultError::RecoveryRequired(format!(
                "run vault is not Complete: {other:?}"
            )));
        }
    }
    let home = lease.publication_journal_project();
    let vault = home
        .subproject(&[HOME[0], HOME[1], &vault_name(run_id)?])
        .map_err(|error| {
            VaultError::RecoveryRequired(format!("pinning complete vault: {error:#}"))
        })?;
    let expected_sha256 = entry
        .sha256
        .strip_prefix("sha256:")
        .ok_or_else(|| VaultError::Refused("catalogue digest is malformed".into()))?;
    let (reader, completion) = vault
        .open_verified_stream_bounded(
            &entry.name,
            expected_sha256,
            entry.size_bytes,
            cap(&entry.source),
        )
        .map_err(|error| {
            VaultError::RecoveryRequired(format!("held asset `{}`: {error:#}", entry.name))
        })?;
    Ok(VaultAssetStream {
        name: entry.name.clone(),
        media_type: entry.media_type.clone(),
        size_bytes: entry.size_bytes,
        sha256: entry.sha256.clone(),
        reader,
        completion,
    })
}
