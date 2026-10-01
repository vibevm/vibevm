//! Local-only C.1 reservation and owner journal preparation. No remote write
//! port is reachable from this module or from production callers.

use vibe_lifecycle::process::propose_run_id;
use vibe_lifecycle::{
    ExistingSubmissionInspection, JournalAdmission, LifecycleLease, ReservationDecision,
    SubmissionNamespace, SubmissionReservations, SubmissionScope,
};
use vibe_wire::generated::publication_effect_plan::PublicationEffectPlan;
use vibe_wire::generated::publication_subject::PublicationSubject;

use crate::publication_authority::PublicationAuthorityDecision;
use crate::publication_effect_plan::PublicationEffectPlanExpectations;

use super::journal::{self, InputCapsule, Intent};
use super::{Error, Inspection, RemoteObservation, Result, Selected, initial_matches, select, sha};

pub(super) enum PreparedJournal {
    Existing(Inspection),
    New {
        run_id: String,
        intent: Intent,
        reservation_ref: String,
        before: RemoteObservation,
    },
}

/// A future owner must construct this only from its own remote observer and
/// owner-controlled approval loader. No production constructor exists yet;
/// tests may inject it without introducing a live publication capability.
pub(crate) struct TrustedLocalPublication {
    approved: PublicationAuthorityDecision,
    before: RemoteObservation,
}

/// Production-compiled, crate-private local journal seam. It never calls a
/// remote method and a Ready result does not authorize a later effect.
pub(crate) fn prepare_local_full_plan(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    subject: &PublicationSubject,
    plan: &PublicationEffectPlan,
    expected: &PublicationEffectPlanExpectations<'_>,
    trusted: TrustedLocalPublication,
) -> Result<Inspection> {
    let PublicationAuthorityDecision::Approved {
        mode,
        authority_ref,
        plan_sha256,
    } = trusted.approved
    else {
        return Err(Error::Refused(
            "local preparation has no approved authority".into(),
        ));
    };
    let selected = select(
        lease,
        scope,
        subject,
        plan,
        expected,
        Some((mode, &authority_ref)),
    )?;
    if plan_sha256 != selected.intent.semantic_plan_sha256 {
        return Err(Error::Refused(
            "approved plan digest differs from selected plan".into(),
        ));
    }
    match prepare_journal(
        lease,
        namespace,
        scope,
        key,
        subject,
        plan,
        selected,
        || Ok(trusted.before),
    )? {
        PreparedJournal::Existing(inspection) => Ok(inspection),
        PreparedJournal::New { run_id, .. } => Ok(Inspection::JournalReady { run_id }),
    }
}

/// The common pre-effect fence for local preparation and the test fake.
/// Existing keys are inspected before `observe` is called. Fresh runs publish
/// the input capsule and intent create-new, read both back, then mark C.1
/// Ready. No effect callback exists here.
pub(super) fn prepare_journal(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    subject: &PublicationSubject,
    plan: &PublicationEffectPlan,
    selected: Selected,
    observe: impl FnOnce() -> Result<RemoteObservation>,
) -> Result<PreparedJournal> {
    match SubmissionReservations::inspect_existing_request(
        lease,
        namespace,
        scope,
        key,
        &selected.request,
    )? {
        ExistingSubmissionInspection::RetiredUnreserved => {
            return Ok(PreparedJournal::Existing(Inspection::Retired {
                run_id: None,
            }));
        }
        ExistingSubmissionInspection::Reserved { .. } => {
            return Ok(PreparedJournal::Existing(super::inspect_full_plan_cold(
                lease,
                namespace.id(),
                scope,
                key,
            )?));
        }
        ExistingSubmissionInspection::Absent => {}
    }
    let before = observe()?;
    if !initial_matches(&before, plan) {
        return Err(Error::Refused(
            "captured remote precondition changed before reservation".into(),
        ));
    }
    let run_id = propose_run_id(lease.root())?;
    if !matches!(SubmissionReservations::reserve(
        lease, namespace, scope, key, &selected.request, &selected.plan_bytes, &run_id,
    )?, ReservationDecision::Created { domain_run_id } if domain_run_id == run_id)
    {
        return Err(Error::Refused(
            "fresh full-plan reservation was not Created".into(),
        ));
    }
    let mut intent = selected.intent;
    intent.run_id = run_id.clone();
    let request_json = String::from_utf8(selected.request).map_err(Error::io)?;
    let capsule = InputCapsule {
        format: journal::INPUT_FORMAT.into(),
        run_id: run_id.clone(),
        context: scope.context.clone(),
        target: scope.target.clone(),
        principal: scope.principal.clone(),
        key_sha256: sha(key.as_bytes()),
        request_json,
        subject: subject.clone(),
        plan: plan.clone(),
        publish: selected.publish,
        allow_replace_published: selected.allow_replace_published,
        upload_catalogue_sha256: selected.upload_catalogue_sha256,
        root_correlation_token: selected.root_correlation_token,
        authority_mode: selected.authority_mode,
        authority_ref: selected.authority_ref,
    };
    if journal::publish_input(lease, &run_id, &capsule).is_err()
        || !matches!(journal::read_input(lease, &run_id), Ok(Some(ref row)) if row == &capsule)
        || journal::publish(lease, &run_id, "intent", &intent).is_err()
        || !matches!(journal::read::<Intent>(lease, &run_id, "intent"), Ok(Some(ref row)) if row == &intent)
        || SubmissionReservations::mark_journal_ready(lease, namespace, scope, key, &run_id)
            .is_err()
        || !matches!(
            SubmissionReservations::journal_admission(lease, namespace, scope, key, &run_id),
            Ok(JournalAdmission::Ready)
        )
    {
        return Ok(PreparedJournal::Existing(Inspection::RecoveryRequired {
            run_id,
        }));
    }
    let reservation_ref =
        match SubmissionReservations::reservation_ref(lease, namespace, scope, key, &run_id) {
            Ok(value) => value,
            Err(_) => {
                return Ok(PreparedJournal::Existing(Inspection::RecoveryRequired {
                    run_id,
                }));
            }
        };
    Ok(PreparedJournal::New {
        run_id,
        intent,
        reservation_ref,
        before,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::publication_full_plan::tests::{expected, prepared, scope};

    fn trusted(
        lease: &LifecycleLease,
        scope: &SubmissionScope,
        subject: &PublicationSubject,
        plan: &PublicationEffectPlan,
    ) -> TrustedLocalPublication {
        let selected = select(
            lease,
            scope,
            subject,
            plan,
            &expected(),
            Some(("A", "fixture-approval")),
        )
        .unwrap();
        TrustedLocalPublication {
            approved: PublicationAuthorityDecision::Approved {
                mode: "A",
                authority_ref: "fixture-approval".into(),
                plan_sha256: selected.intent.semantic_plan_sha256,
            },
            before: RemoteObservation {
                generation_ref: plan.remote_generation_ref.clone(),
                tag: plan.observed_tag.clone(),
                release: plan.observed_release.clone(),
                latest_release_id: None,
                last: None,
            },
        }
    }

    #[test]
    fn local_prepare_retains_ready_capsule_without_any_remote_effect_and_cold_attaches() {
        let root = tempfile::tempdir().unwrap();
        let (subject, plan) = prepared();
        let scope = scope();
        let (token, run_id) = {
            let lease = LifecycleLease::acquire(root.path()).unwrap();
            let namespace =
                SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
            let outcome = prepare_local_full_plan(
                &lease,
                &namespace,
                &scope,
                "local-key",
                &subject,
                &plan,
                &expected(),
                trusted(&lease, &scope, &subject, &plan),
            )
            .unwrap();
            let Inspection::JournalReady { run_id } = outcome else {
                panic!("local journal not Ready")
            };
            assert_eq!(
                SubmissionReservations::journal_admission(
                    &lease,
                    &namespace,
                    &scope,
                    "local-key",
                    &run_id,
                )
                .unwrap(),
                JournalAdmission::Ready,
            );
            let capsule = journal::read_input(&lease, &run_id).unwrap().unwrap();
            assert_eq!(capsule.authority_mode.as_deref(), Some("A"));
            assert_eq!(capsule.authority_ref.as_deref(), Some("fixture-approval"));
            let catalogue = crate::publication_payload::expected_upload_catalogue(
                &subject,
                &expected().subject,
            )
            .unwrap();
            let root_token = lease
                .publication_journal_project()
                .root_correlation_token()
                .unwrap();
            assert_eq!(capsule.upload_catalogue_sha256, catalogue.digest);
            assert_eq!(capsule.root_correlation_token, root_token);
            let request: serde_json::Value = serde_json::from_str(&capsule.request_json).unwrap();
            assert_eq!(request["upload_catalogue_sha256"], catalogue.digest);
            assert_eq!(request["root_correlation_token"], root_token);
            let intent = journal::read::<Intent>(&lease, &run_id, "intent")
                .unwrap()
                .unwrap();
            assert_eq!(intent.upload_catalogue_sha256, catalogue.digest);
            assert_eq!(intent.root_correlation_token, root_token);
            assert!(
                journal::read::<super::super::Step>(&lease, &run_id, "000-pending")
                    .unwrap()
                    .is_none()
            );
            assert!(
                journal::read::<super::super::FinalResult>(&lease, &run_id, "result")
                    .unwrap()
                    .is_none()
            );
            assert_eq!(
                prepare_local_full_plan(
                    &lease,
                    &namespace,
                    &scope,
                    "local-key",
                    &subject,
                    &plan,
                    &expected(),
                    trusted(&lease, &scope, &subject, &plan),
                )
                .unwrap(),
                Inspection::JournalReady {
                    run_id: run_id.clone()
                },
            );
            (namespace.id().to_owned(), run_id)
        };
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        assert_eq!(
            super::super::inspect_full_plan_cold(&lease, &token, &scope, "local-key").unwrap(),
            Inspection::JournalReady {
                run_id: run_id.clone()
            },
        );
        let capsule_path = root.path().join(format!(
            ".vibe/publication-operations/{run_id}.full-plan-input.json"
        ));
        let original_capsule = std::fs::read(&capsule_path).unwrap();
        let mut capsule = journal::read_input(&lease, &run_id).unwrap().unwrap();
        capsule.upload_catalogue_sha256 = sha(b"foreign-catalogue");
        std::fs::write(&capsule_path, serde_json::to_vec(&capsule).unwrap()).unwrap();
        assert_eq!(
            super::super::inspect_full_plan_cold(&lease, &token, &scope, "local-key").unwrap(),
            Inspection::RecoveryRequired {
                run_id: run_id.clone()
            },
        );
        std::fs::write(&capsule_path, &original_capsule).unwrap();
        let mut capsule = journal::read_input(&lease, &run_id).unwrap().unwrap();
        capsule.root_correlation_token = sha(b"foreign-root");
        std::fs::write(&capsule_path, serde_json::to_vec(&capsule).unwrap()).unwrap();
        assert_eq!(
            super::super::inspect_full_plan_cold(&lease, &token, &scope, "local-key").unwrap(),
            Inspection::RecoveryRequired {
                run_id: run_id.clone()
            },
        );
        std::fs::write(&capsule_path, &original_capsule).unwrap();
        let mut capsule = journal::read_input(&lease, &run_id).unwrap().unwrap();
        capsule.authority_ref = Some("changed".into());
        std::fs::write(&capsule_path, serde_json::to_vec(&capsule).unwrap()).unwrap();
        assert_eq!(
            super::super::inspect_full_plan_cold(&lease, &token, &scope, "local-key").unwrap(),
            Inspection::RecoveryRequired {
                run_id: run_id.clone()
            },
        );
        std::fs::remove_file(capsule_path).unwrap();
        assert_eq!(
            super::super::inspect_full_plan_cold(&lease, &token, &scope, "local-key").unwrap(),
            Inspection::RecoveryRequired { run_id },
        );
    }

    #[test]
    fn pre_ready_missing_capsule_and_unapproved_local_selection_never_complete() {
        let root = tempfile::tempdir().unwrap();
        let (subject, plan) = prepared();
        let scope = scope();
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
        let mut unapproved = trusted(&lease, &scope, &subject, &plan);
        unapproved.approved = PublicationAuthorityDecision::NeedsApproval {
            reason: crate::publication_authority::PublicationAuthorityReason::ApprovalMissing,
        };
        assert!(matches!(
            prepare_local_full_plan(
                &lease,
                &namespace,
                &scope,
                "local-key",
                &subject,
                &plan,
                &expected(),
                unapproved
            ),
            Err(Error::Refused(_))
        ));
        let selected = select(
            &lease,
            &scope,
            &subject,
            &plan,
            &expected(),
            Some(("A", "fixture-approval")),
        )
        .unwrap();
        let run_id = propose_run_id(lease.root()).unwrap();
        assert!(matches!(
            SubmissionReservations::reserve(
                &lease,
                &namespace,
                &scope,
                "local-key",
                &selected.request,
                &selected.plan_bytes,
                &run_id,
            )
            .unwrap(),
            ReservationDecision::Created { .. }
        ));
        assert_eq!(
            super::super::inspect_full_plan_cold(&lease, namespace.id(), &scope, "local-key",)
                .unwrap(),
            Inspection::RecoveryRequired { run_id },
        );
    }

    #[test]
    fn copied_tree_at_same_path_cannot_cold_attach_as_ready() {
        use std::fs;
        use std::path::Path;

        fn copy_tree(from: &Path, to: &Path) {
            fs::create_dir_all(to).unwrap();
            for entry in fs::read_dir(from).unwrap() {
                let entry = entry.unwrap();
                let destination = to.join(entry.file_name());
                if entry.file_type().unwrap().is_dir() {
                    copy_tree(&entry.path(), &destination);
                } else {
                    fs::copy(entry.path(), destination).unwrap();
                }
            }
        }

        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("workspace");
        fs::create_dir(&root).unwrap();
        let (subject, plan) = prepared();
        let scope = scope();
        let (token, run_id, original_root_token) = {
            let lease = LifecycleLease::acquire(&root).unwrap();
            let namespace =
                SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
            let inspection = prepare_local_full_plan(
                &lease,
                &namespace,
                &scope,
                "copied-key",
                &subject,
                &plan,
                &expected(),
                trusted(&lease, &scope, &subject, &plan),
            )
            .unwrap();
            let Inspection::JournalReady { run_id } = inspection else {
                panic!("not Ready")
            };
            let root_token = lease
                .publication_journal_project()
                .root_correlation_token()
                .unwrap();
            (namespace.id().to_owned(), run_id, root_token)
        };
        let old = parent.path().join("original");
        fs::rename(&root, &old).unwrap();
        copy_tree(&old, &root);
        let lease = LifecycleLease::acquire(&root).unwrap();
        assert_ne!(
            lease
                .publication_journal_project()
                .root_correlation_token()
                .unwrap(),
            original_root_token,
        );
        let copied = super::super::inspect_full_plan_cold(&lease, &token, &scope, "copied-key");
        assert!(
            matches!(&copied, Ok(Inspection::RecoveryRequired { run_id: same }) if same == &run_id),
            "copied same-path root must recover, got {copied:?}"
        );
    }
}
