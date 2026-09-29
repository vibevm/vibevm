use super::*;
use vibe_core::machine_view::OpaqueText;

fn r(value: &str) -> OpaqueText {
    OpaqueText::new(value).unwrap()
}
fn witness(axis: &str, revision: &str, claim: Option<&str>) -> AxisWitness {
    AxisWitness {
        owner: r(&format!("{axis}-owner")),
        capture: r(&format!("{axis}-capture")),
        revision: r(revision),
        observation: r(&format!("{axis}-observation")),
        compatibility_claim: claim.map(r),
        expected_revisions: Vec::new(),
        freshness: AxisFreshness::DeclaredCurrent,
        completeness: AxisCompleteness::DeclaredComplete,
    }
}
fn source_only() -> CompositeSnapshot {
    CompositeSnapshot::compose([
        AxisParticipation::Required(Some(witness("source", "rev-1", Some("join-1")))),
        AxisParticipation::NotParticipating,
        AxisParticipation::NotParticipating,
        AxisParticipation::NotParticipating,
    ])
    .unwrap()
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-COMPOSITE-SNAPSHOT",
    r = 1
)]
fn compose_refuses_missing_stale_and_conflicting_axis_claims() {
    let missing = CompositeSnapshot::compose([
        AxisParticipation::Required(Some(witness("source", "rev-1", Some("join-1")))),
        AxisParticipation::Required(None),
        AxisParticipation::NotParticipating,
        AxisParticipation::NotParticipating,
    ]);
    assert_eq!(missing, Err(CompositeIssue::Missing(SnapshotAxis::World)));
    let mut stale = witness("world", "old-rev", Some("join-1"));
    stale.freshness = AxisFreshness::Stale;
    assert_eq!(
        CompositeSnapshot::compose([
            AxisParticipation::Required(Some(witness("source", "rev-1", Some("join-1")))),
            AxisParticipation::Required(Some(stale)),
            AxisParticipation::NotParticipating,
            AxisParticipation::NotParticipating,
        ]),
        Err(CompositeIssue::Stale(SnapshotAxis::World))
    );
    assert_eq!(
        CompositeSnapshot::compose([
            AxisParticipation::Required(Some(witness("source", "rev-1", Some("join-1")))),
            AxisParticipation::Required(Some(witness("world", "rev-2", Some("join-2")))),
            AxisParticipation::NotParticipating,
            AxisParticipation::NotParticipating,
        ]),
        Err(CompositeIssue::ConflictingClaim(SnapshotAxis::World))
    );
    let mut revision_claim = witness("source", "rev-1", Some("join-1"));
    revision_claim.expected_revisions.push(RevisionExpectation {
        axis: SnapshotAxis::World,
        revision: r("expected-world-rev"),
    });
    assert_eq!(
        CompositeSnapshot::compose([
            AxisParticipation::Required(Some(revision_claim)),
            AxisParticipation::Required(Some(witness("world", "actual-world-rev", Some("join-1")))),
            AxisParticipation::NotParticipating,
            AxisParticipation::NotParticipating,
        ]),
        Err(CompositeIssue::ConflictingRevision {
            claimant: SnapshotAxis::Source,
            target: SnapshotAxis::World,
        })
    );
    assert_eq!(
        CompositeSnapshot::compose(std::array::from_fn(|_| AxisParticipation::NotParticipating)),
        Err(CompositeIssue::NoParticipatingAxis)
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-COMPOSITE-SNAPSHOT",
    r = 1
)]
fn matching_multi_owner_claims_remain_unverified_not_atomic() {
    let snapshot = CompositeSnapshot::compose([
        AxisParticipation::Required(Some(witness("source", "rev-1", Some("join-1")))),
        AxisParticipation::Required(Some(witness("world", "rev-2", Some("join-1")))),
        AxisParticipation::NotParticipating,
        AxisParticipation::NotParticipating,
    ])
    .unwrap();
    assert_eq!(
        snapshot.completeness(),
        CompositeCompleteness::UnverifiedCrossOwner
    );
    assert_eq!(snapshot.stamps().len(), 2);
    assert_eq!(snapshot.stamps()[1].revision.as_str(), "rev-2");

    let without_join = CompositeSnapshot::compose([
        AxisParticipation::Required(Some(witness("source", "rev-1", Some("join-1")))),
        AxisParticipation::Required(Some(witness("world", "rev-2", None))),
        AxisParticipation::NotParticipating,
        AxisParticipation::NotParticipating,
    ])
    .unwrap();
    assert_eq!(
        without_join.completeness(),
        CompositeCompleteness::UnverifiedMissingClaim
    );
    let mut partial = witness("world", "rev-2", Some("join-1"));
    partial.completeness = AxisCompleteness::Partial {
        reason: r("truncated"),
    };
    let incomplete = CompositeSnapshot::compose([
        AxisParticipation::Required(Some(witness("source", "rev-1", Some("join-1")))),
        AxisParticipation::Required(Some(partial)),
        AxisParticipation::NotParticipating,
        AxisParticipation::NotParticipating,
    ])
    .unwrap();
    assert_eq!(incomplete.completeness(), CompositeCompleteness::Partial);
    assert!(matches!(
        incomplete.axis(SnapshotAxis::World),
        AxisParticipation::Required(Some(AxisWitness {
            completeness: AxisCompleteness::Partial { .. },
            ..
        }))
    ));
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-BOUND-PAGING",
    r = 1
)]
fn complete_fixed_pages_reconstruct_every_authorized_row() {
    let rows = vec!["root", "child-a", "child-b", "repeat-a", "tail"];
    let projection = AuthorizedProjection::supplied(
        &source_only(),
        r("projection-1"),
        r("filter-all"),
        r("scope-owned"),
        rows.clone(),
        ProjectionExtent::DeclaredComplete,
    );
    let mut assembled = Vec::new();
    let mut cursor = None;
    loop {
        let page = projection.page(cursor.as_ref(), 2).unwrap();
        assembled.extend(page.rows);
        match page.end {
            PageEnd::More => cursor = page.next,
            PageEnd::DeclaredComplete => {
                assert!(page.next.is_none());
                break;
            }
            PageEnd::Incomplete(reason) => panic!("unexpected incomplete page: {reason:?}"),
        }
    }
    assert_eq!(assembled, rows);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-BOUND-PAGING",
    r = 1
)]
fn mixed_filter_scope_projection_or_revision_cursors_refuse_and_partial_never_finishes_complete() {
    let original = source_only();
    let projection = AuthorizedProjection::supplied(
        &original,
        r("projection-1"),
        r("filter-all"),
        r("scope-owned"),
        vec![1, 2, 3],
        ProjectionExtent::DeclaredComplete,
    );
    let cursor = projection.page(None, 1).unwrap().next.unwrap();
    let same_claims_different_rows = AuthorizedProjection::supplied(
        &original,
        r("projection-1"),
        r("filter-all"),
        r("scope-owned"),
        vec![1, 99, 3],
        ProjectionExtent::DeclaredComplete,
    );
    assert_eq!(
        same_claims_different_rows.page(Some(&cursor), 1),
        Err(PageFault::MixedCursor)
    );
    for (snapshot, id, filter, scope) in [
        (
            original.clone(),
            "projection-2",
            "filter-all",
            "scope-owned",
        ),
        (
            original.clone(),
            "projection-1",
            "filter-subset",
            "scope-owned",
        ),
        (
            original.clone(),
            "projection-1",
            "filter-all",
            "scope-foreign",
        ),
        (
            CompositeSnapshot::compose([
                AxisParticipation::Required(Some(witness("source", "rev-2", Some("join-1")))),
                AxisParticipation::NotParticipating,
                AxisParticipation::NotParticipating,
                AxisParticipation::NotParticipating,
            ])
            .unwrap(),
            "projection-1",
            "filter-all",
            "scope-owned",
        ),
    ] {
        let other = AuthorizedProjection::supplied(
            &snapshot,
            r(id),
            r(filter),
            r(scope),
            vec![1, 2, 3],
            ProjectionExtent::DeclaredComplete,
        );
        assert_eq!(other.page(Some(&cursor), 1), Err(PageFault::MixedCursor));
    }
    assert_eq!(projection.page(None, 0), Err(PageFault::ZeroLimit));

    let partial = AuthorizedProjection::supplied(
        &original,
        r("projection-partial"),
        r("filter-all"),
        r("scope-owned"),
        vec![1],
        ProjectionExtent::Partial {
            reason: r("children-unavailable"),
        },
    );
    assert_eq!(
        partial.page(None, 4).unwrap().end,
        PageEnd::Incomplete(IncompleteReason::DeclaredPartial(r("children-unavailable")))
    );
    let multi = CompositeSnapshot::compose([
        AxisParticipation::Required(Some(witness("source", "rev-1", Some("join-1")))),
        AxisParticipation::Required(Some(witness("world", "rev-2", Some("join-1")))),
        AxisParticipation::NotParticipating,
        AxisParticipation::NotParticipating,
    ])
    .unwrap();
    let unverified = AuthorizedProjection::supplied(
        &multi,
        r("projection-unverified"),
        r("filter-all"),
        r("scope-owned"),
        vec![1],
        ProjectionExtent::DeclaredComplete,
    );
    assert_eq!(
        unverified.page(None, 4).unwrap().end,
        PageEnd::Incomplete(IncompleteReason::CrossOwnerUnverified)
    );
}
