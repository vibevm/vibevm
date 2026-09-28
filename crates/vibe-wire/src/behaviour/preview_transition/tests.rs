specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#CONTRIBUTION-HISTORY-SELECTION");

use super::*;

fn captured() -> Transition {
    serde_json::from_str(include_str!(
        "../../../../../formats/corpora/preview-transition/e1/valid/captured.json"
    ))
    .expect("registered preview transition corpus must parse")
}

#[test]
fn contribution_wire_preserves_absent_scope_and_refuses_wrong_shape() {
    assert!(captured().contribution_scope.is_none());
    let transition: Transition = serde_json::from_str(include_str!(
        "../../../../../formats/corpora/preview-transition/e1/valid/with-contribution-scope.json"
    ))
    .unwrap();
    assert_eq!(
        transition,
        serde_json::from_str(&serde_json::to_string(&transition).unwrap()).unwrap()
    );
    assert!(transition.contribution_scope.is_some());
    assert!(serde_json::from_str::<Transition>(include_str!("../../../../../formats/corpora/preview-transition/e1/invalid/wrong-contribution-scope-type.json")).is_err());
}

#[test]
fn supplied_required_claim_selection_is_nonempty_unique_and_sorted() {
    let mut transition: Transition = serde_json::from_str(include_str!(
        "../../../../../formats/corpora/preview-transition/e1/valid/with-contribution-scope.json"
    ))
    .unwrap();
    transition.change_records = transition
        .contribution_scope
        .as_ref()
        .unwrap()
        .record_digests
        .iter()
        .map(|record| record.change_id.clone())
        .collect();
    transition.validate_preview().unwrap();
    for claims in [
        vec![],
        vec![" ".into()],
        vec!["z".into(), "a".into()],
        vec!["a".into(), "a".into()],
    ] {
        transition
            .contribution_scope
            .as_mut()
            .unwrap()
            .record_digests[0]
            .required_claim_ids = Some(claims);
        assert_eq!(
            transition.validate_preview().unwrap_err().field,
            "contribution_scope.required_claim_ids"
        );
    }
    transition
        .contribution_scope
        .as_mut()
        .unwrap()
        .record_digests[0]
        .required_claim_ids = None;
    transition.validate_preview().unwrap();
}

#[test]
fn supplied_scope_requires_safe_exact_sorted_digest_bindings() {
    use crate::generated::preview::e1::transition::{ContributionScope, RecordDigest};
    let mut transition = captured();
    transition.contribution_scope = Some(ContributionScope {
        before_capture_id: "before".into(),
        after_capture_id: "after".into(),
        record_digests: vec![],
    });
    transition.validate_preview().unwrap();
    transition
        .contribution_scope
        .as_mut()
        .unwrap()
        .record_digests
        .push(RecordDigest {
            change_id: "change-one".into(),
            sha256: "a".repeat(64),
            required_claim_ids: None,
        });
    assert_eq!(
        transition.validate_preview().unwrap_err().field,
        "contribution_scope.record_digests"
    );
    transition.change_records.push("change-one".into());
    transition.validate_preview().unwrap();
    transition
        .contribution_scope
        .as_mut()
        .unwrap()
        .record_digests[0]
        .sha256 = "bad".into();
    assert_eq!(
        transition.validate_preview().unwrap_err().field,
        "contribution_scope.record_digests"
    );
    transition
        .contribution_scope
        .as_mut()
        .unwrap()
        .before_capture_id = "../unsafe".into();
    assert_eq!(
        transition.validate_preview().unwrap_err().field,
        "contribution_scope"
    );
}

fn intermediate(identity: &str) -> CaptureRef {
    let mut reference = captured().dp1;
    reference.capture_id = identity.into();
    reference.source = format!("evidence/{identity}.json");
    reference
}

fn set_dp2(transition: &mut Transition, reference: CaptureRef) {
    transition.dp2 = crate::generated::preview::e1::transition::PendingCapture {
        state: PendingCaptureState::Candidate,
        capture_id: Some(reference.capture_id),
        source: Some(reference.source),
        capture_file_sha256: Some(reference.capture_file_sha256),
        commit_sha1: Some(reference.commit_sha1),
        git_tree_sha1: Some(reference.git_tree_sha1),
    };
}

#[test]
fn generated_catalogue_preserves_absent_empty_and_supplied_shapes() {
    let mut transition = captured();
    assert!(transition.captures.is_none());
    assert!(
        !serde_json::to_string(&transition)
            .unwrap()
            .contains("\"captures\"")
    );
    transition.captures = Some(vec![]);
    let roundtrip: Transition =
        serde_json::from_str(&serde_json::to_string(&transition).unwrap()).unwrap();
    assert_eq!(roundtrip.captures, Some(vec![]));
    let supplied: Transition = serde_json::from_str(include_str!(
        "../../../../../formats/corpora/preview-transition/e1/valid/with-catalogue.json"
    ))
    .unwrap();
    assert_eq!(supplied.captures.as_ref().unwrap().len(), 1);
    assert_eq!(supplied.capture_references().unwrap().len(), 2);
    assert!(
        serde_json::from_str::<Transition>(include_str!(
            "../../../../../formats/corpora/preview-transition/e1/invalid/wrong-catalogue-type.json"
        ))
        .is_err()
    );
}

#[test]
fn references_are_sorted_and_retain_compatible_alternate_sources() {
    let mut transition = captured();
    let mut alternate = transition.dp1.clone();
    alternate.source = "evidence/alternate.json".into();
    transition.captures = Some(vec![alternate.clone(), intermediate("intermediate")]);
    set_dp2(&mut transition, intermediate("dp2-final"));
    let references = transition.capture_references().unwrap();
    assert_eq!(references.len(), 4);
    assert_eq!(references[0], alternate);
    assert_eq!(references[1], transition.dp1);
    assert!(references.windows(2).all(
        |pair| (&pair[0].capture_id, &pair[0].source) <= (&pair[1].capture_id, &pair[1].source)
    ));
    transition.validate_preview().unwrap();
}

#[test]
fn conflicting_catalogue_content_for_endpoint_identity_refuses() {
    for field in ["digest", "commit", "tree"] {
        let mut transition = captured();
        let mut conflict = transition.dp1.clone();
        match field {
            "digest" => conflict.capture_file_sha256 = "a".repeat(64),
            "commit" => conflict.commit_sha1 = "a".repeat(40),
            _ => conflict.git_tree_sha1 = "a".repeat(40),
        }
        transition.captures = Some(vec![conflict]);
        assert_eq!(
            transition.capture_references().unwrap_err().field,
            "captures",
            "{field}"
        );
        assert_eq!(
            transition.validate_preview().unwrap_err().field,
            "captures",
            "{field}"
        );
    }
}

#[test]
fn equal_endpoint_identity_requires_equal_content_but_allows_alternate_source() {
    let mut transition = captured();
    let mut after = transition.dp1.clone();
    after.source = "evidence/alternate.json".into();
    set_dp2(&mut transition, after.clone());
    assert_eq!(transition.capture_references().unwrap().len(), 2);
    after.git_tree_sha1 = "a".repeat(40);
    set_dp2(&mut transition, after);
    assert_eq!(
        transition.capture_references().unwrap_err().field,
        "captures"
    );
}

#[test]
fn duplicate_and_unsorted_additional_catalogue_ids_refuse() {
    for captures in [
        vec![intermediate("a"), intermediate("a")],
        vec![intermediate("b"), intermediate("a")],
    ] {
        let mut transition = captured();
        transition.captures = Some(captures);
        assert_eq!(
            transition.capture_references().unwrap_err().field,
            "captures"
        );
    }
}

#[test]
fn every_reference_uses_the_same_digest_and_path_validation() {
    for field in ["id", "source", "digest", "commit", "tree"] {
        for scope in ["dp1", "dp2", "catalogue"] {
            let mut transition = captured();
            let mut bad = intermediate("intermediate");
            match field {
                "id" => bad.capture_id = " ".into(),
                "source" => bad.source = "../outside.json".into(),
                "digest" => bad.capture_file_sha256 = "bad".into(),
                "commit" => bad.commit_sha1 = "A".repeat(40),
                _ => bad.git_tree_sha1.clear(),
            }
            match scope {
                "dp1" => transition.dp1 = bad,
                "dp2" => set_dp2(&mut transition, bad),
                _ => transition.captures = Some(vec![bad]),
            }
            assert!(transition.capture_references().is_err(), "{scope}/{field}");
            assert!(transition.validate_preview().is_err(), "{scope}/{field}");
        }
    }
}

#[test]
fn pure_reference_seam_refuses_pending_identity_and_incomplete_after_capture() {
    let mut transition = captured();
    transition.dp2.source = Some("evidence/after.json".into());
    assert_eq!(transition.capture_references().unwrap_err().field, "dp2");
    transition.dp2.state = PendingCaptureState::Candidate;
    assert_eq!(transition.capture_references().unwrap_err().field, "dp2");
}

#[test]
fn captured_before_state_is_valid_and_later_proof_is_pending() {
    let transition = captured();
    transition.validate_preview().unwrap();
    assert!(!transition.proof.final_product_panel_passed);
}

#[test]
fn wrong_epoch_type_and_unknown_field_are_refused_by_generated_reader() {
    for bad in [
        include_str!(
            "../../../../../formats/corpora/preview-transition/e1/invalid/wrong-epoch-type.json"
        ),
        include_str!(
            "../../../../../formats/corpora/preview-transition/e1/invalid/unknown-field.json"
        ),
    ] {
        assert!(serde_json::from_str::<Transition>(bad).is_err());
    }
}

#[test]
fn a_pending_dp2_cannot_claim_identity_or_final_proof() {
    let mut transition = captured();
    transition.dp2.capture_id = Some("candidate".into());
    assert_eq!(transition.validate_preview().unwrap_err().field, "dp2");

    let mut transition = captured();
    transition.proof.final_product_panel_passed = true;
    assert_eq!(
        transition.validate_preview().unwrap_err().field,
        "proof.final_product_panel_passed"
    );
}

#[test]
fn unknown_epoch_is_refused_after_shape_parsing() {
    let mut transition = captured();
    transition.schema = 2;
    assert_eq!(transition.validate_preview().unwrap_err().field, "schema");
}

#[test]
fn documentation_and_migration_proof_require_verified_sources() {
    let mut transition = captured();
    transition.proof.documentation_verified = true;
    assert_eq!(
        transition.validate_preview().unwrap_err().field,
        "proof.documentation_verified"
    );

    let mut transition = captured();
    transition.proof.migration_rehearsed = true;
    assert_eq!(
        transition.validate_preview().unwrap_err().field,
        "proof.migration_rehearsed"
    );
}

#[test]
fn duplicate_change_ids_and_parent_paths_are_refused() {
    let mut transition = captured();
    transition.change_records = vec!["C-1".into(), "C-1".into()];
    assert_eq!(
        transition.validate_preview().unwrap_err().field,
        "change_records"
    );

    let mut transition = captured();
    transition.dp1.source = "../outside.json".into();
    assert_eq!(
        transition.validate_preview().unwrap_err().field,
        "dp1.source"
    );
}
