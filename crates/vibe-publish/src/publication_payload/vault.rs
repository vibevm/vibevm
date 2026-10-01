//! Run-scoped local payload staging under a Pending C.1 reservation.
//! This vault performs no remote operation and grants no publication effect.
//! Its held-handle and create-new checks work across Linux, macOS and Windows;
//! hostile aliases that mutate bytes after a witnessed read remain a
//! detection-bound limit, and the root token is same-host correlation only.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vibe_lifecycle::process::is_valid_run_id;
use vibe_lifecycle::{
    JournalAdmission, LifecycleLease, SubmissionNamespace, SubmissionReservations, SubmissionScope,
};
use vibe_safefs::{Project, StreamCompletion, VerifiedStreamReader};
use vibe_wire::generated::publication_subject::{PublicationInstallerKind, PublicationSubject};

use crate::publication_subject::PublicationSubjectExpectations;
use crate::release_manifest::{
    DISTRIBUTION_AGGREGATE_MANIFEST_MAX_BYTES, DISTRIBUTION_BOOTSTRAP_MAX_BYTES,
    DISTRIBUTION_BUNDLE_MAX_BYTES, DISTRIBUTION_MANIFEST_MAX_BYTES,
};

use super::{ExpectedUpload, ExpectedUploadCatalogue, UploadSource, expected_upload_catalogue};

const HOME: [&str; 2] = [".vibe", "publication-operations"];
const COMPLETE: &str = "complete.json";
const COMPLETE_CAP: usize = 64 * 1024;
const INSTALLER_CAP: usize = 1024 * 1024;
const FORMAT: &str = "vibevm.publication.payload-vault/e1";

/// Caller-held final bytes. Installer provenance is not inferred from these
/// slices; the later owner must have obtained them from the selected commit.
pub struct FinalPayloadBytes<'a> {
    pub aggregate: &'a [u8],
    pub bash: &'a [u8],
    pub powershell: &'a [u8],
    pub cmd: &'a [u8],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VaultInspection {
    Complete {
        run_id: String,
        catalogue_digest: String,
        manifest_sha256: String,
    },
    Incomplete {
        run_id: String,
        verified_files: usize,
    },
    Uncertain {
        run_id: String,
        reason: String,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("payload vault refused before staging: {0}")]
    Refused(String),
    #[error("payload vault may contain partial state; inspect before any recovery: {0}")]
    RecoveryRequired(String),
}

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    name: String,
    media_type: String,
    sha256: String,
    size_bytes: u64,
    source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Completion {
    format: String,
    run_id: String,
    catalogue_digest: String,
    root_correlation_token: String,
    entries: Vec<Entry>,
}

fn source_name(source: &UploadSource) -> String {
    match source {
        UploadSource::TargetBundle { target } => format!("bundle:{target}"),
        UploadSource::TargetBootstrap { target } => format!("bootstrap:{target}"),
        UploadSource::TargetFragment { target } => format!("fragment:{target}"),
        UploadSource::Aggregate => "aggregate".into(),
        UploadSource::Installer { kind } => match kind {
            PublicationInstallerKind::Bash => "installer:bash".into(),
            PublicationInstallerKind::Powershell => "installer:powershell".into(),
            PublicationInstallerKind::Cmd => "installer:cmd".into(),
        },
    }
}

fn row(entry: &ExpectedUpload) -> Entry {
    Entry {
        name: entry.name.clone(),
        media_type: entry.media_type.clone(),
        sha256: entry.sha256.clone(),
        size_bytes: entry.size_bytes,
        source: source_name(&entry.source),
    }
}

fn cap(source: &UploadSource) -> u64 {
    match source {
        UploadSource::TargetBundle { .. } => DISTRIBUTION_BUNDLE_MAX_BYTES,
        UploadSource::TargetBootstrap { .. } => DISTRIBUTION_BOOTSTRAP_MAX_BYTES,
        UploadSource::TargetFragment { .. } => DISTRIBUTION_MANIFEST_MAX_BYTES,
        UploadSource::Aggregate => DISTRIBUTION_AGGREGATE_MANIFEST_MAX_BYTES,
        UploadSource::Installer { .. } => INSTALLER_CAP as u64,
    }
}

fn committed_bytes<'a>(
    source: &UploadSource,
    bytes: &'a FinalPayloadBytes<'a>,
) -> Option<&'a [u8]> {
    match source {
        UploadSource::Aggregate => Some(bytes.aggregate),
        UploadSource::Installer {
            kind: PublicationInstallerKind::Bash,
        } => Some(bytes.bash),
        UploadSource::Installer {
            kind: PublicationInstallerKind::Powershell,
        } => Some(bytes.powershell),
        UploadSource::Installer {
            kind: PublicationInstallerKind::Cmd,
        } => Some(bytes.cmd),
        _ => None,
    }
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn vault_name(run_id: &str) -> Result<String, VaultError> {
    if !is_valid_run_id(run_id) {
        return Err(VaultError::Refused("invalid reserved run ID".into()));
    }
    Ok(format!("{run_id}.payloads"))
}

fn reserved_run(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    run_id: &str,
    require_pending: bool,
) -> Result<(), VaultError> {
    let row = SubmissionReservations::inspect_existing_key(lease, namespace, scope, key)
        .map_err(|error| VaultError::Refused(format!("C.1 inspection: {error}")))?
        .ok_or_else(|| VaultError::Refused("C.1 key has no reservation".into()))?;
    if row.domain_run_id != run_id
        || row.retired
        || require_pending && row.journal_admission != JournalAdmission::Pending
    {
        return Err(VaultError::Refused(
            "C.1 run is foreign, retired or no longer Pending".into(),
        ));
    }
    Ok(())
}

fn prepared_catalogue(
    subject: &PublicationSubject,
    expected: &PublicationSubjectExpectations<'_>,
) -> Result<ExpectedUploadCatalogue, VaultError> {
    expected_upload_catalogue(subject, expected)
        .map_err(|error| VaultError::Refused(format!("catalogue: {error}")))
}

fn preflight(
    output: &Project,
    catalogue: &ExpectedUploadCatalogue,
    finals: &FinalPayloadBytes<'_>,
) -> Result<(), VaultError> {
    for entry in &catalogue.entries {
        if let Some(bytes) = committed_bytes(&entry.source, finals) {
            if bytes.len() as u64 != entry.size_bytes
                || bytes.len() > cap(&entry.source) as usize
                || digest(bytes) != entry.sha256
            {
                return Err(VaultError::Refused(format!(
                    "final payload `{}` differs from subject",
                    entry.name
                )));
            }
        } else {
            let state = output
                .stable_file_state_bounded(&entry.name, cap(&entry.source))
                .map_err(|error| {
                    VaultError::Refused(format!("output `{}`: {error:#}", entry.name))
                })?
                .ok_or_else(|| VaultError::Refused(format!("output `{}` is absent", entry.name)))?;
            if state.bytes != entry.size_bytes || format!("sha256:{}", state.sha256) != entry.sha256
            {
                return Err(VaultError::Refused(format!(
                    "output `{}` differs from subject",
                    entry.name
                )));
            }
        }
    }
    Ok(())
}

fn publish_error(name: &str, error: vibe_safefs::PublishError) -> VaultError {
    // The run vault already exists. Even a before-publication failure for
    // THIS name can leave earlier exact files staged, so only inspection may
    // classify the whole run; never present this as a clean retry.
    VaultError::RecoveryRequired(format!("`{name}`: {:#}", error.into_report()))
}

/// Stage all exact expected uploads while C.1 is Pending. An existing vault
/// is never resumed or overwritten; a later owner must inspect and decide.
pub fn stage_payload_vault(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    run_id: &str,
    subject: &PublicationSubject,
    expected: &PublicationSubjectExpectations<'_>,
    output: &Project,
    finals: &FinalPayloadBytes<'_>,
) -> Result<VaultInspection, VaultError> {
    stage_with_interrupt(
        lease,
        namespace,
        scope,
        key,
        run_id,
        subject,
        expected,
        output,
        finals,
        None,
        |_, _| {},
    )
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn stage_payload_vault_until(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    run_id: &str,
    subject: &PublicationSubject,
    expected: &PublicationSubjectExpectations<'_>,
    output: &Project,
    finals: &FinalPayloadBytes<'_>,
    staged_files: usize,
) -> Result<VaultInspection, VaultError> {
    stage_with_interrupt(
        lease,
        namespace,
        scope,
        key,
        run_id,
        subject,
        expected,
        output,
        finals,
        Some(staged_files),
        |_, _| {},
    )
}

#[allow(clippy::too_many_arguments)]
fn stage_with_interrupt<F: FnMut(usize, &Project)>(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    run_id: &str,
    subject: &PublicationSubject,
    expected: &PublicationSubjectExpectations<'_>,
    output: &Project,
    finals: &FinalPayloadBytes<'_>,
    interrupt_after: Option<usize>,
    mut after_staged: F,
) -> Result<VaultInspection, VaultError> {
    let vault_name = vault_name(run_id)?;
    reserved_run(lease, namespace, scope, key, run_id, true)?;
    let catalogue = prepared_catalogue(subject, expected)?;
    let home = lease.publication_journal_project();
    if home
        .dir_if_present(&[HOME[0], HOME[1], &vault_name])
        .map_err(|error| VaultError::RecoveryRequired(format!("vault occupancy: {error:#}")))?
        .is_some()
    {
        return Err(VaultError::RecoveryRequired(
            "run vault already exists; inspect it, do not retry staging".into(),
        ));
    }
    preflight(output, &catalogue, finals)?;
    let vault = home
        .ensure_subproject(&[HOME[0], HOME[1], &vault_name])
        .map_err(|error| VaultError::RecoveryRequired(format!("creating run vault: {error:#}")))?;
    let dir = vault
        .root_dir()
        .map_err(|error| VaultError::RecoveryRequired(format!("pinning run vault: {error:#}")))?;
    for (index, entry) in catalogue.entries.iter().enumerate() {
        if let Some(bytes) = committed_bytes(&entry.source, finals) {
            vault
                .publish_new_in(&dir, &entry.name, bytes)
                .map_err(|error| publish_error(&entry.name, error))?;
            let state = vault
                .stable_file_state_bounded(&entry.name, cap(&entry.source))
                .map_err(|error| {
                    VaultError::RecoveryRequired(format!(
                        "final readback `{}`: {error:#}",
                        entry.name
                    ))
                })?
                .ok_or_else(|| {
                    VaultError::RecoveryRequired(format!("final `{}` disappeared", entry.name))
                })?;
            if state.bytes != entry.size_bytes || format!("sha256:{}", state.sha256) != entry.sha256
            {
                return Err(VaultError::RecoveryRequired(format!(
                    "final `{}` changed on readback",
                    entry.name
                )));
            }
        } else {
            output
                .copy_stable_file_to_expected_new_bounded(
                    &entry.name,
                    &vault,
                    &entry.name,
                    entry.sha256.strip_prefix("sha256:").unwrap_or(""),
                    entry.size_bytes,
                    cap(&entry.source),
                    None,
                )
                .map_err(|error| publish_error(&entry.name, error))?;
        }
        after_staged(index + 1, &vault);
        if interrupt_after == Some(index + 1) {
            return Err(VaultError::RecoveryRequired(
                "interrupted after an exact staged file".into(),
            ));
        }
    }
    reserved_run(lease, namespace, scope, key, run_id, true).map_err(|error| {
        VaultError::RecoveryRequired(format!("C.1 changed before completion: {error}"))
    })?;
    let completion = Completion {
        format: FORMAT.into(),
        run_id: run_id.into(),
        catalogue_digest: catalogue.digest.clone(),
        root_correlation_token: home.root_correlation_token().map_err(|error| {
            VaultError::RecoveryRequired(format!("held root identity: {error:#}"))
        })?,
        entries: catalogue.entries.iter().map(row).collect(),
    };
    let bytes = serde_json::to_vec(&completion)
        .map_err(|error| VaultError::RecoveryRequired(format!("completion encoding: {error}")))?;
    if bytes.len() > COMPLETE_CAP {
        return Err(VaultError::RecoveryRequired(
            "completion manifest exceeds cap".into(),
        ));
    }
    vault
        .publish_new_in(&dir, COMPLETE, &bytes)
        .map_err(|error| publish_error(COMPLETE, error))?;
    let readback = vault
        .read_file_snapshot_bounded(COMPLETE, COMPLETE_CAP)
        .map_err(|error| VaultError::RecoveryRequired(format!("completion readback: {error:#}")))?
        .ok_or_else(|| VaultError::RecoveryRequired("completion disappeared".into()))?;
    if readback.bytes != bytes {
        return Err(VaultError::RecoveryRequired(
            "completion changed on readback".into(),
        ));
    }
    let manifest_sha256 = digest(&bytes);
    reserved_run(lease, namespace, scope, key, run_id, true).map_err(|error| {
        VaultError::RecoveryRequired(format!("C.1 changed after completion readback: {error}"))
    })?;
    let inspected = inspect_payload_vault(lease, namespace, scope, key, run_id, subject, expected);
    match inspected {
        VaultInspection::Complete {
            run_id: observed,
            catalogue_digest,
            manifest_sha256: observed_manifest,
        } if observed == run_id
            && catalogue_digest == catalogue.digest
            && observed_manifest == manifest_sha256 =>
        {
            Ok(VaultInspection::Complete {
                run_id: observed,
                catalogue_digest,
                manifest_sha256,
            })
        }
        other => Err(VaultError::RecoveryRequired(format!(
            "final vault inspection did not prove Complete: {other:?}"
        ))),
    }
}

/// Inspect without creating a directory or changing C.1. A complete marker
/// is trusted only after all nineteen named files and the current held root
/// match the validated catalogue again.
pub fn inspect_payload_vault(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    run_id: &str,
    subject: &PublicationSubject,
    expected: &PublicationSubjectExpectations<'_>,
) -> VaultInspection {
    let uncertain = |reason: String| VaultInspection::Uncertain {
        run_id: run_id.into(),
        reason,
    };
    let vault_name = match vault_name(run_id) {
        Ok(name) => name,
        Err(error) => return uncertain(error.to_string()),
    };
    if let Err(error) = reserved_run(lease, namespace, scope, key, run_id, false) {
        return uncertain(error.to_string());
    }
    let catalogue = match prepared_catalogue(subject, expected) {
        Ok(catalogue) => catalogue,
        Err(error) => return uncertain(error.to_string()),
    };
    let home = lease.publication_journal_project();
    let vault = match home.subproject(&[HOME[0], HOME[1], &vault_name]) {
        Ok(vault) => vault,
        Err(error) => {
            return match home.dir_if_present(&[HOME[0], HOME[1], &vault_name]) {
                Ok(None) => VaultInspection::Incomplete {
                    run_id: run_id.into(),
                    verified_files: 0,
                },
                Ok(Some(_)) | Err(_) => uncertain(format!("run vault cannot be pinned: {error:#}")),
            };
        }
    };
    let completion = match vault.read_file_snapshot_bounded(COMPLETE, COMPLETE_CAP) {
        Ok(Some(snapshot)) => match serde_json::from_slice::<Completion>(&snapshot.bytes) {
            Ok(value) => {
                let canonical = match serde_json::to_vec(&value) {
                    Ok(bytes) => bytes,
                    Err(error) => return uncertain(format!("completion re-encoding: {error}")),
                };
                if snapshot.bytes != canonical {
                    return uncertain("completion bytes are not canonical".into());
                }
                Some((value, digest(&snapshot.bytes)))
            }
            Err(error) => return uncertain(format!("completion is malformed: {error}")),
        },
        Ok(None) => None,
        Err(error) => return uncertain(format!("completion cannot be read: {error:#}")),
    };
    let mut verified = 0;
    let mut missing = false;
    for entry in &catalogue.entries {
        let state = match vault.stable_file_state_bounded(&entry.name, cap(&entry.source)) {
            Ok(state) => state,
            Err(error) => return uncertain(format!("staged `{}`: {error:#}", entry.name)),
        };
        let Some(state) = state else {
            missing = true;
            continue;
        };
        if state.bytes != entry.size_bytes || format!("sha256:{}", state.sha256) != entry.sha256 {
            return uncertain(format!("staged `{}` differs from catalogue", entry.name));
        }
        verified += 1;
    }
    let Some((completion, manifest_sha256)) = completion else {
        return VaultInspection::Incomplete {
            run_id: run_id.into(),
            verified_files: verified,
        };
    };
    let token = match home.root_correlation_token() {
        Ok(token) => token,
        Err(error) => return uncertain(format!("held root identity: {error:#}")),
    };
    if missing
        || completion.format != FORMAT
        || completion.run_id != run_id
        || completion.catalogue_digest != catalogue.digest
        || completion.root_correlation_token != token
        || completion.entries != catalogue.entries.iter().map(row).collect::<Vec<_>>()
    {
        return uncertain("completion manifest or held root differs from exact catalogue".into());
    }
    VaultInspection::Complete {
        run_id: run_id.into(),
        catalogue_digest: catalogue.digest,
        manifest_sha256,
    }
}

#[cfg(test)]
#[path = "vault/tests.rs"]
mod tests;
