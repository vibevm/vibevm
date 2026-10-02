use super::*;

#[test]
fn publication_caps_retain_original_owner_evidence_in_owning_error_channel() {
    let f = Fixture::new();
    let route = f.route();
    let request = br#"{"schema":1,"action":{"kind":"submit","key":"copy"}}"#;
    let refusal =
        FixedPackageRoute::dispatch_bounded(Some(&route), request, f.root.path(), 1).unwrap_err();
    assert!(refusal.effects_may_have_begun);
    assert!(matches!(
        refusal.owner_observation,
        Some(StaticFileJournalInspection::Completed { .. })
    ));
    assert_eq!(
        refusal.error.code,
        err::FixedPackageActionErrorCode::PublicationUnavailable
    );
    assert!(refusal.error.known_run_id.is_some());
    let before = std::fs::metadata(f.output()).unwrap().modified().unwrap();
    let refusal = FixedPackageRoute::exchange_bounded(
        Some(&route),
        &frame("submit", "copy"),
        f.root.path(),
        1,
    )
    .unwrap_err();
    assert!(refusal.effects_may_have_begun && refusal.owner_observation.is_some());
    assert_eq!(
        refusal.error.code,
        err::FixedPackageActionErrorCode::PublicationUnavailable
    );
    assert_eq!(
        std::fs::metadata(f.output()).unwrap().modified().unwrap(),
        before
    );
}
#[test]
fn invalid_owner_projection_and_utf8_diagnostic_bounds_are_enforced() {
    let run_id = "a".repeat(32);
    for owner in [
        StaticFileJournalInspection::Completed {
            run_id: run_id.clone(),
            output_sha256: "a".repeat(64),
            artifact_record: "foreign".into(),
        },
        StaticFileJournalInspection::Completed {
            run_id: run_id.clone(),
            output_sha256: "wrong".into(),
            artifact_record: ".vibe/state/artifacts/launcher.bin.json".into(),
        },
        StaticFileJournalInspection::IntentReady { run_id: " ".into() },
        StaticFileJournalInspection::Retired {
            run_id: Some("run".into()),
        },
    ] {
        assert!(contract::project(&owner).is_err());
    }
    let partial = vibe_lifecycle::StaticFilePartialEvidence {
        output_directory: vibe_lifecycle::StaticFileObservedPresence::Present,
        output_file: vibe_lifecycle::StaticFileObservedFile::Present {
            sha256: "a".repeat(64),
            bytes: u64::MAX,
        },
        artifact_record: vibe_lifecycle::StaticFileObservedFile::Absent,
    };
    let owner = StaticFileJournalInspection::Uncertain { run_id, partial };
    assert!(contract::project(&owner).is_err());
    let refusal = dispatch::failure(
        None,
        err::FixedPackageActionErrorCode::PublicationUnavailable,
        &"é".repeat(5000),
        None,
        true,
        Some(owner.clone()),
        None,
    );
    assert!(refusal.error.message.len() <= 4096 && refusal.error.message_truncated);
    assert_eq!(refusal.owner_observation, Some(owner));
    assert!(refusal.error.view.is_none());
    assert_eq!(
        refusal.error.effect_observation,
        err::FixedPackageActionErrorEffectObservation::Unavailable
    );
    let short = dispatch::failure(
        None,
        err::FixedPackageActionErrorCode::InvalidRequest,
        "short",
        None,
        false,
        None,
        None,
    );
    assert!(!short.error.message_truncated);
    contract::roundtrip(&short.error, PUBLICATION_CAP).unwrap();
}

#[test]
fn all_closed_owner_variants_partial_counts_reasons_and_publication_coherence() {
    use vibe_lifecycle::{
        StaticFileObservedFile as F, StaticFileObservedPresence as P, StaticFilePartialEvidence,
    };
    let valid_run = "a".repeat(32);
    for (owner, effect, completeness) in [
        (
            StaticFileJournalInspection::IntentReady {
                run_id: valid_run.clone(),
            },
            "uncertain",
            "partial",
        ),
        (
            StaticFileJournalInspection::RecoveryRequired {
                run_id: valid_run.clone(),
            },
            "uncertain",
            "unavailable",
        ),
        (
            StaticFileJournalInspection::Retired {
                run_id: Some(valid_run.clone()),
            },
            "uncertain",
            "unavailable",
        ),
        (
            StaticFileJournalInspection::Retired { run_id: None },
            "uncertain",
            "unavailable",
        ),
        (
            StaticFileJournalInspection::Uncertain {
                run_id: valid_run.clone(),
                partial: StaticFilePartialEvidence {
                    output_directory: P::Unavailable("exact directory reason".into()),
                    output_file: F::Unavailable("exact file reason".into()),
                    artifact_record: F::Present {
                        sha256: format!("sha256:{}", "a".repeat(64)),
                        bytes: 64 * 1024,
                    },
                },
            },
            "uncertain",
            "partial",
        ),
    ] {
        let projected = contract::project(&owner).unwrap();
        contract::roundtrip(
            &rep::FixedPackageActionReply {
                schema: 1,
                reply: projected.clone(),
            },
            PUBLICATION_CAP,
        )
        .unwrap();
        let refusal = dispatch::failure(
            None,
            err::FixedPackageActionErrorCode::SubmissionRefused,
            "refused",
            Some("submit"),
            true,
            Some(owner),
            None,
        );
        let outer = envelope::error(&refusal).unwrap();
        let v = serde_json::to_value(&outer).unwrap();
        assert_eq!(v["body"]["effect_state"], effect);
        assert_eq!(v["body"]["completeness"], completeness);
        let row = error(&outer);
        let view: rep::Reply =
            serde_json::from_value(serde_json::to_value(row.view.unwrap()).unwrap()).unwrap();
        assert_eq!(view, projected);
        let mut wrong = refusal;
        wrong.error.known_run_id = Some("foreign".into());
        assert!(envelope::error(&wrong).is_err());
    }
    let large = StaticFileJournalInspection::Uncertain {
        run_id: valid_run.clone(),
        partial: StaticFilePartialEvidence {
            output_directory: P::Present,
            output_file: F::Unavailable("é".repeat(PUBLICATION_CAP)),
            artifact_record: F::Absent,
        },
    };
    let refusal = dispatch::failure(
        None,
        err::FixedPackageActionErrorCode::PublicationUnavailable,
        "cap",
        Some("submit"),
        true,
        Some(large.clone()),
        None,
    );
    assert!(envelope::error(&refusal).is_err());
    assert_eq!(refusal.owner_observation, Some(large));
    let mut invalid = StaticFilePartialEvidence {
        output_directory: P::Present,
        output_file: F::Present {
            sha256: "a".repeat(64),
            bytes: 2 * 1024 * 1024,
        },
        artifact_record: F::Present {
            sha256: format!("sha256:{}", "b".repeat(64)),
            bytes: 64 * 1024,
        },
    };
    let o = |p| StaticFileJournalInspection::Uncertain {
        run_id: valid_run.clone(),
        partial: p,
    };
    contract::project(&o(invalid.clone())).unwrap();
    invalid.artifact_record = F::Present {
        sha256: format!("sha256:{}", "b".repeat(64)),
        bytes: 64 * 1024 + 1,
    };
    assert!(contract::project(&o(invalid.clone())).is_err());
    invalid.artifact_record = F::Unavailable(" ".into());
    assert!(contract::project(&o(invalid)).is_err());
}
