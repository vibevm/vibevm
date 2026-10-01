//! One lease-bound, non-durable owner session after a complete cold proof.
//! It caches no bytes and grants no remote write; each selected stream is
//! proved anew and every future effect still owes a fresh remote observation.

use vibe_lifecycle::{
    JournalAdmission, LifecycleLease, SubmissionNamespace, SubmissionReservations, SubmissionScope,
};
use vibe_safefs::{FileIdentity, Project};
use vibe_wire::generated::publication_effect_plan::PublicationEffectPlan;
use vibe_wire::generated::publication_subject::PublicationSubject;

use crate::publication_approval::{ApprovalHome, ColdRedemptionExpectation};
use crate::publication_effect_plan::PublicationEffectPlanExpectations;
use crate::publication_payload::vault::{VaultAssetStream, cap};
use crate::publication_payload::{ExpectedUploadCatalogue, expected_upload_catalogue};

use super::calls::asset_history::{self, AssetInspection};
use super::calls::noop_completion::{self, NoOpInspection};
use super::calls::{self, CallInspection};
use super::cold::{self, ColdReadyProof};
use super::journal;
use super::{Error, Inspection, Result, sha};

pub(super) enum SessionOpen<'a> {
    Ready(ReadyRun<'a>),
    RecoveryRequired { run_id: String },
    Uncertain { run_id: String },
    Completed { run_id: String },
    Retired { run_id: Option<String> },
}

/// The sole constructor consumes a sealed proof from a full cold validation.
/// The held outer lease and vault directory remain live for every call.
pub(super) struct ReadyRun<'a> {
    pub(super) lease: &'a LifecycleLease,
    pub(super) namespace: &'a SubmissionNamespace,
    pub(super) scope: &'a SubmissionScope,
    pub(super) key: &'a str,
    pub(super) run_id: String,
    pub(super) subject: &'a PublicationSubject,
    pub(super) plan: &'a PublicationEffectPlan,
    pub(super) expected: &'a PublicationEffectPlanExpectations<'a>,
    pub(super) approval_home: &'a ApprovalHome,
    proof: ColdReadyProof,
    catalogue: ExpectedUploadCatalogue,
    vault: Project,
    vault_identity: FileIdentity,
}

impl<'a> ReadyRun<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn open(
        lease: &'a LifecycleLease,
        namespace: &'a SubmissionNamespace,
        scope: &'a SubmissionScope,
        key: &'a str,
        subject: &'a PublicationSubject,
        plan: &'a PublicationEffectPlan,
        expected: &'a PublicationEffectPlanExpectations<'a>,
        approval_home: &'a ApprovalHome,
    ) -> Result<SessionOpen<'a>> {
        if let Some(existing) =
            SubmissionReservations::inspect_existing_key(lease, namespace, scope, key)?
            && existing.retired
        {
            return Ok(SessionOpen::Retired {
                run_id: Some(existing.domain_run_id),
            });
        }
        let (status, proof) =
            cold::inspect_full_plan_cold_proved(lease, namespace.id(), scope, key, approval_home)?;
        let run_id = match &status {
            Inspection::JournalReady { run_id }
            | Inspection::Partial {
                run_id,
                uncertain: false,
                pending: None,
                ..
            } => run_id.clone(),
            Inspection::Completed { run_id, .. } => {
                return Ok(SessionOpen::Completed {
                    run_id: run_id.clone(),
                });
            }
            Inspection::Partial { run_id, .. } => {
                return Ok(SessionOpen::Uncertain {
                    run_id: run_id.clone(),
                });
            }
            Inspection::RecoveryRequired { run_id } => {
                return Ok(SessionOpen::RecoveryRequired {
                    run_id: run_id.clone(),
                });
            }
            Inspection::Retired { run_id } => {
                return Ok(SessionOpen::Retired {
                    run_id: run_id.clone(),
                });
            }
        };
        let proof = proof
            .ok_or_else(|| Error::Refused("cold validation supplied no ready proof".into()))?;
        if proof.capsule.subject != *subject
            || proof.capsule.plan != *plan
            || proof.capsule.run_id != run_id
        {
            return Ok(SessionOpen::RecoveryRequired { run_id });
        }
        let catalogue = expected_upload_catalogue(subject, &expected.subject).map_err(Error::io)?;
        if catalogue.digest != proof.capsule.upload_catalogue_sha256 {
            return Ok(SessionOpen::RecoveryRequired { run_id });
        }
        let vault_name = format!("{run_id}.payloads");
        let vault = lease
            .publication_journal_project()
            .subproject(&[".vibe", "publication-operations", &vault_name])
            .map_err(Error::io)?;
        let vault_identity = vault.root_identity().map_err(Error::io)?;
        let session = Self {
            lease,
            namespace,
            scope,
            key,
            run_id,
            subject,
            plan,
            expected,
            approval_home,
            proof,
            catalogue,
            vault,
            vault_identity,
        };
        session.recheck_small()?;
        match calls::inspect_calls_proved(&session)? {
            CallInspection::Uncertain { .. } => {
                return Ok(SessionOpen::Uncertain {
                    run_id: session.run_id,
                });
            }
            CallInspection::Unavailable { .. } => {
                match asset_history::inspect_assets_proved(&session)? {
                    AssetInspection::Uncertain { .. } => {
                        return Ok(SessionOpen::Uncertain {
                            run_id: session.run_id,
                        });
                    }
                    AssetInspection::Unavailable { landed: 38 }
                        if matches!(
                            noop_completion::inspect_proved(&session)?,
                            NoOpInspection::Uncertain { .. }
                        ) =>
                    {
                        return Ok(SessionOpen::Uncertain {
                            run_id: session.run_id,
                        });
                    }
                    _ => {}
                }
            }
            CallInspection::Next { .. } => {}
        }
        Ok(SessionOpen::Ready(session))
    }

    /// Cheap correlation recheck under the same lease. It never rehashes all
    /// nineteen payloads; the selected file is independently proved later.
    pub(super) fn recheck_small(&self) -> Result<()> {
        let row = SubmissionReservations::inspect_existing_key(
            self.lease,
            self.namespace,
            self.scope,
            self.key,
        )?
        .ok_or_else(|| Error::Refused("session C.1 key vanished".into()))?;
        if row.retired
            || row.journal_admission != JournalAdmission::Ready
            || row.domain_run_id != self.run_id
            || row.request_sha256 != self.proof.request_sha256
            || row.plan_sha256 != self.proof.plan_sha256
            || self
                .lease
                .publication_journal_project()
                .root_correlation_token()
                .map_err(Error::io)?
                != self.proof.capsule.root_correlation_token
            || journal::read_input(self.lease, &self.run_id)
                .ok()
                .flatten()
                .as_ref()
                != Some(&self.proof.capsule)
            || journal::read::<journal::Intent>(self.lease, &self.run_id, "intent")
                .ok()
                .flatten()
                .as_ref()
                != Some(&self.proof.intent)
        {
            return Err(Error::Refused("session C.1/capsule/intent changed".into()));
        }
        let name = format!("{}.payloads", self.run_id);
        let named = self
            .lease
            .publication_journal_project()
            .subproject(&[".vibe", "publication-operations", &name])
            .map_err(Error::io)?;
        if named.root_identity().map_err(Error::io)? != self.vault_identity {
            return Err(Error::Refused("session vault root was replaced".into()));
        }
        let completion = self
            .vault
            .read_file_snapshot_bounded("complete.json", 64 * 1024)
            .map_err(Error::io)?
            .ok_or_else(|| Error::Refused("vault manifest is absent".into()))?;
        if Some(sha(&completion.bytes).as_str())
            != self.proof.capsule.vault_manifest_sha256.as_deref()
        {
            return Err(Error::Refused("vault manifest changed".into()));
        }
        let expectation = ColdRedemptionExpectation {
            authority_id: self
                .proof
                .capsule
                .authority_ref
                .as_deref()
                .ok_or_else(|| Error::Refused("approval reference absent".into()))?,
            run_id: &self.run_id,
            marker_sha256: self
                .proof
                .capsule
                .redemption_marker_sha256
                .as_deref()
                .ok_or_else(|| Error::Refused("redemption SHA absent".into()))?,
            plan_sha256: &self.proof.intent.semantic_plan_sha256,
            c1_plan_sha256: &self.proof.plan_sha256,
            request_sha256: &self.proof.request_sha256,
            approval_record_sha256: self
                .proof
                .capsule
                .approval_record_sha256
                .as_deref()
                .ok_or_else(|| Error::Refused("approval record SHA absent".into()))?,
        };
        self.approval_home
            .inspect_redemption_cold(
                self.lease,
                self.namespace,
                self.scope,
                self.key,
                &expectation,
            )
            .map_err(Error::io)?;
        Ok(())
    }

    /// Read one exact selected asset. This does not decide whether a remote
    /// call may start; call-history and remote-before-image gates remain owed.
    pub(super) fn open_selected_stream(
        &self,
        index: usize,
        name: &str,
    ) -> Result<VaultAssetStream> {
        self.recheck_small()?;
        let entry = self
            .catalogue
            .entries
            .get(index)
            .ok_or_else(|| Error::Refused("session asset index is foreign".into()))?;
        if entry.name != name {
            return Err(Error::Refused(
                "session asset name differs from catalogue".into(),
            ));
        }
        let raw_sha = entry
            .sha256
            .strip_prefix("sha256:")
            .ok_or_else(|| Error::Refused("session asset digest invalid".into()))?;
        let (reader, completion) = self
            .vault
            .open_verified_stream_bounded(
                &entry.name,
                raw_sha,
                entry.size_bytes,
                cap(&entry.source),
            )
            .map_err(Error::io)?;
        Ok(VaultAssetStream {
            name: entry.name.clone(),
            media_type: entry.media_type.clone(),
            size_bytes: entry.size_bytes,
            sha256: entry.sha256.clone(),
            reader,
            completion,
        })
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
