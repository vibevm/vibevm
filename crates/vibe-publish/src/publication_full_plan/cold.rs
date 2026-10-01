//! Read-only cold attachment for production vault and test-only fake lanes.

use super::*;
use crate::publication_payload::vault::{VaultInspection, inspect_payload_vault};

#[derive(Clone, Copy)]
enum ColdLane {
    ProductionVault,
    #[cfg(test)]
    FakeNoVault,
}

/// Reattach from host-retained namespace/scope/key after the original caller
/// and its prepared subject/plan values are gone. This is inspection only:
/// the C.1 digest lookup and immutable input capsule never grant an effect.
/// The capsule is protected local evidence because authored smoke references
/// may contain sensitive text even though no asset bytes or token field live
/// in this record.
pub(crate) fn inspect_full_plan_cold(
    lease: &LifecycleLease,
    host_namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
) -> Result<Inspection> {
    inspect_cold(
        lease,
        host_namespace_token,
        scope,
        key,
        ColdLane::ProductionVault,
    )
}

fn inspect_cold(
    lease: &LifecycleLease,
    host_namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    lane: ColdLane,
) -> Result<Inspection> {
    let namespace = SubmissionNamespace::from_host_token(host_namespace_token.to_owned())?;
    let row = SubmissionReservations::inspect_existing_key(lease, &namespace, scope, key)?
        .ok_or_else(|| Error::Refused("host key has no retained C.1 reservation".into()))?;
    let run_id = row.domain_run_id;
    if row.journal_admission == JournalAdmission::Pending {
        return Ok(Inspection::RecoveryRequired { run_id });
    }
    let capsule = match journal::read_input(lease, &run_id) {
        Ok(Some(capsule)) => capsule,
        Ok(None) | Err(_) => return Ok(Inspection::RecoveryRequired { run_id }),
    };
    let valid_lane = match lane {
        ColdLane::ProductionVault => {
            capsule.format == journal::INPUT_FORMAT && capsule.vault_manifest_sha256.is_some()
        }
        #[cfg(test)]
        ColdLane::FakeNoVault => {
            capsule.format == journal::FAKE_INPUT_FORMAT && capsule.vault_manifest_sha256.is_none()
        }
    };
    if !valid_lane
        || capsule.run_id != run_id
        || capsule.context != scope.context
        || capsule.target != scope.target
        || capsule.principal != scope.principal
        || capsule.key_sha256 != sha(key.as_bytes())
        || capsule.root_correlation_token
            != match lease.publication_journal_project().root_correlation_token() {
                Ok(token) => token,
                Err(_) => return Ok(Inspection::RecoveryRequired { run_id }),
            }
    {
        return Ok(Inspection::RecoveryRequired { run_id });
    }
    let expected = PublicationEffectPlanExpectations {
        subject: crate::publication_subject::PublicationSubjectExpectations {
            version: "1.1.0",
            repository: "vibevm/vibevm",
            smoke_policy_id: "distribution-smoke/e1",
            smoke_policy_epoch: 1,
        },
        host: "github.com",
        owner: "vibevm",
        repository: "vibevm",
        publish: capsule.publish,
        allow_replace_published: capsule.allow_replace_published,
    };
    if capsule.authority_mode.is_some() != capsule.authority_ref.is_some() {
        return Ok(Inspection::RecoveryRequired { run_id });
    }
    let authority = capsule
        .authority_mode
        .as_deref()
        .zip(capsule.authority_ref.as_deref());
    let mut selected = match select(
        lease,
        scope,
        &capsule.subject,
        &capsule.plan,
        &expected,
        authority,
    ) {
        Ok(selected) => selected,
        Err(_) => return Ok(Inspection::RecoveryRequired { run_id }),
    };
    if capsule.request_json.as_bytes() != selected.request
        || capsule.upload_catalogue_sha256 != selected.upload_catalogue_sha256
        || capsule.root_correlation_token != selected.root_correlation_token
        || row.request_sha256 != sha(&selected.request)
        || row.plan_sha256 != sha(&selected.plan_bytes)
    {
        return Ok(Inspection::RecoveryRequired { run_id });
    }
    selected.intent.vault_manifest_sha256 = capsule.vault_manifest_sha256.clone();
    let inspected = inspect(
        lease,
        &namespace,
        scope,
        key,
        &selected,
        &capsule.plan,
        run_id.clone(),
        row.journal_admission,
        row.reservation_ref,
    )?;
    match lane {
        ColdLane::ProductionVault => {
            let vault = inspect_payload_vault(
                lease,
                &namespace,
                scope,
                key,
                &run_id,
                &capsule.subject,
                &expected.subject,
            );
            if matches!(vault, VaultInspection::Complete {
                run_id: ref observed, catalogue_digest: ref catalogue, manifest_sha256: ref manifest,
            } if observed == &run_id && catalogue == &selected.upload_catalogue_sha256
                && Some(manifest) == capsule.vault_manifest_sha256.as_ref())
            {
                Ok(inspected)
            } else {
                Ok(match inspected {
                    Inspection::Completed { partitions, .. } => Inspection::Partial {
                        run_id,
                        landed: partitions,
                        pending: None,
                        uncertain: true,
                    },
                    Inspection::Partial {
                        landed, pending, ..
                    } => Inspection::Partial {
                        run_id,
                        landed,
                        pending,
                        uncertain: true,
                    },
                    _ => Inspection::RecoveryRequired { run_id },
                })
            }
        }
        #[cfg(test)]
        ColdLane::FakeNoVault => Ok(inspected),
    }
}

#[cfg(test)]
pub(super) fn inspect_fake_full_plan_cold(
    lease: &LifecycleLease,
    host_namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
) -> Result<Inspection> {
    inspect_cold(
        lease,
        host_namespace_token,
        scope,
        key,
        ColdLane::FakeNoVault,
    )
}
