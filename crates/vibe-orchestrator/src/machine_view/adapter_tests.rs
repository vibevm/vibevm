use super::*;
use vibe_wire::generated::machine::e1::{bound_page as page, composite_snapshot as composite};

fn single() -> composite::CompositeSnapshot {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/machine-composite-snapshot/e1/valid/single-owner.json"
    )))
    .unwrap()
}
fn multi() -> composite::CompositeSnapshot {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/machine-composite-snapshot/e1/valid/two-owner-unverified.json"
    )))
    .unwrap()
}
fn first() -> page::BoundPage {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/machine-bound-page/e1/valid/first-page.json"
    )))
    .unwrap()
}
fn final_page() -> page::BoundPage {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/machine-bound-page/e1/valid/final-page.json"
    )))
    .unwrap()
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-COMPOSITE-SNAPSHOT",
    r = 1
)]
fn composite_replays_composed_and_refused_values_without_atomicity_claim() {
    let CompositeReplay::Composed(one) = replay_generated_composite(single()).unwrap() else {
        panic!("single owner did not compose")
    };
    assert_eq!(
        one.completeness(),
        CompositeCompleteness::DeclaredCompleteSingleOwner
    );
    let CompositeReplay::Composed(multiple) = replay_generated_composite(multi()).unwrap() else {
        panic!("multi-owner observation did not compose")
    };
    assert_eq!(
        multiple.completeness(),
        CompositeCompleteness::UnverifiedCrossOwner
    );
    let missing: composite::CompositeSnapshot = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/machine-composite-snapshot/e1/valid/missing-required-refusal.json"
    )))
    .unwrap();
    assert_eq!(
        replay_generated_composite(missing).unwrap(),
        CompositeReplay::Refused(CompositeIssue::Missing(SnapshotAxis::World))
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-COMPOSITE-SNAPSHOT",
    r = 1
)]
fn composite_mismatch_epoch_stamps_and_conflicting_claim_retain_original() {
    let mut cases = Vec::new();
    let mut epoch = single();
    epoch.schema = 2;
    cases.push((epoch, CompositeWireFault::UnsupportedEpoch));
    let mut wrong_stamp = single();
    if let composite::Outcome::Composed(outcome) = &mut wrong_stamp.outcome {
        outcome.stamps[0].revision = "foreign".to_owned();
    }
    cases.push((wrong_stamp, CompositeWireFault::OutcomeMismatch));
    let conflicting: composite::CompositeSnapshot = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/machine-composite-snapshot/e1/semantic-invalid/conflicting-claim-outcome.json"
    )))
    .unwrap();
    cases.push((conflicting, CompositeWireFault::OutcomeMismatch));
    for (original, expected) in cases {
        let failure = replay_generated_composite(original.clone()).unwrap_err();
        assert_eq!(failure.original, original);
        assert_eq!(failure.fault, expected);
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-BOUND-PAGING",
    r = 1
)]
fn local_pages_check_stated_adjacency_but_leave_cursor_unverified() {
    let a = review_generated_page(first()).unwrap();
    let b = review_generated_page(final_page()).unwrap();
    assert_eq!(a.cursor_status(), OwnerCursorStatus::Unverified);
    assert_eq!(
        compare_declared_adjacent_pages(&a, &b),
        Ok(OwnerCursorStatus::Unverified)
    );
    let mut altered = final_page();
    altered.axis_stamps[0].revision = "other".to_owned();
    let other = review_generated_page(altered).unwrap();
    assert_eq!(
        compare_declared_adjacent_pages(&a, &other),
        Err(PageWireFault::MixedPageDeclarations)
    );
    let mut altered = final_page();
    altered.page_cursor_id = Some("foreign-token".to_owned());
    let other = review_generated_page(altered).unwrap();
    assert_eq!(
        compare_declared_adjacent_pages(&a, &other),
        Err(PageWireFault::TokenAdjacencyMismatch)
    );
    let partial: page::BoundPage = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/machine-bound-page/e1/valid/partial-children.json"
    )))
    .unwrap();
    assert!(review_generated_page(partial).is_ok());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-BOUND-PAGING",
    r = 1
)]
fn local_page_refuses_false_complete_duplicate_occurrence_and_wrong_epoch() {
    let redacted: page::BoundPage = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/machine-bound-page/e1/semantic-invalid/false-complete-redacted.json"
    )))
    .unwrap();
    let failure = review_generated_page(redacted.clone()).unwrap_err();
    assert_eq!(failure.fault, PageWireFault::FalseCompleteChild);
    assert_eq!(failure.original, redacted);
    let mut duplicate = first();
    duplicate.rows.push(duplicate.rows[0].clone());
    assert_eq!(
        review_generated_page(duplicate).unwrap_err().fault,
        PageWireFault::DuplicateOccurrence
    );
    let mut self_parent = first();
    let occurrence = self_parent.rows[0].occurrence_id.clone();
    self_parent.rows[0].parent_occurrence_id = Some(occurrence);
    assert_eq!(
        review_generated_page(self_parent).unwrap_err().fault,
        PageWireFault::SelfParent
    );
    let mut epoch = first();
    epoch.schema = 2;
    assert_eq!(
        review_generated_page(epoch).unwrap_err().fault,
        PageWireFault::UnsupportedEpoch
    );
}
