use super::*;
use crate::machine_view::{AxisCompleteness, AxisFreshness, AxisParticipation, AxisWitness};
use vibe_wire::generated::machine::e1::entity_reference as entity;

fn r(value: &str) -> OpaqueText {
    OpaqueText::new(value).unwrap()
}
fn snapshot() -> CompositeSnapshot {
    CompositeSnapshot::compose([
        AxisParticipation::Required(Some(AxisWitness {
            owner: r("source-owner"),
            capture: r("capture-a"),
            revision: r("rev-a"),
            observation: r("obs-a"),
            compatibility_claim: Some(r("join-a")),
            expected_revisions: Vec::new(),
            freshness: AxisFreshness::DeclaredCurrent,
            completeness: AxisCompleteness::DeclaredComplete,
        })),
        AxisParticipation::NotParticipating,
        AxisParticipation::NotParticipating,
        AxisParticipation::NotParticipating,
    ])
    .unwrap()
}
fn entity(id: &str, ordinal: usize) -> EntityReference {
    let observed = entity::Observation {
        projection: "projection-a".to_owned(),
        observation: "obs-a".to_owned(),
    };
    EntityReference {
        schema: 1,
        reference_id: id.to_owned(),
        logical: entity::Logical::OwnerMinted(Box::new(entity::LogicalOwnerMinted {
            owner: "spec-owner".to_owned(),
            address: "spec://org.example/a#rule".to_owned(),
        })),
        captured: entity::Captured {
            content_or_source: "capture-a".to_owned(),
            revision: "rev-a".to_owned(),
        },
        observed: observed.clone(),
        occurrence: entity::Occurrence {
            observed,
            occurrence_id: format!("occ-{ordinal}"),
            contribution: format!("include-{ordinal}"),
            ordinal: ordinal.to_string(),
        },
    }
}
fn row(index: usize, parent: Option<&str>) -> wire::Row {
    wire::Row {
        entity_reference_id: format!("row-{index}"),
        occurrence_id: format!("occ-{index}"),
        parent_occurrence_id: parent.map(str::to_owned),
        child_state: wire::ChildState::Complete(Box::new(wire::ChildStateComplete {})),
    }
}
fn rows() -> Vec<wire::Row> {
    vec![
        row(0, None),
        row(1, Some("occ-0")),
        row(2, Some("occ-0")),
        row(3, Some("occ-1")),
        row(4, None),
    ]
}
fn catalog() -> Vec<EntityReference> {
    (0..5).map(|i| entity(&format!("row-{i}"), i)).collect()
}
fn session(
    rows: Vec<wire::Row>,
    extent: ProjectionExtent,
) -> Result<PageSession, PageSessionError> {
    PageSession::new(
        &snapshot(),
        r("projection-a"),
        r("filter-all"),
        r("scope-owned"),
        rows,
        catalog(),
        extent,
    )
}
fn request(token: Option<String>, stamps: Vec<AxisStamp>) -> PageSessionRequest {
    PageSessionRequest {
        projection: r("projection-a"),
        filter: r("filter-all"),
        authorized_scope: r("scope-owned"),
        axis_stamps: stamps,
        cursor_token: token,
        limit: 2,
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-BOUND-PAGING",
    r = 1
)]
fn owner_session_pages_reconstruct_retained_rows_and_parent_order() {
    let original = rows();
    let mut owner = session(original.clone(), ProjectionExtent::DeclaredComplete).unwrap();
    assert_eq!(
        owner.entity("row-3").unwrap().logical,
        owner.entity("row-0").unwrap().logical
    );
    let stamps = snapshot().stamps().to_vec();
    let first = owner.next_page(request(None, stamps.clone())).unwrap();
    assert_eq!(first.value.rows, original[..2]);
    assert_eq!(
        first.cursor_status,
        SessionCursorStatus::OwnerIssuedProcessLocal
    );
    assert!(!first.source_bytes_authenticated);
    let first_token = first.value.next_cursor_id.clone().unwrap();
    let second = owner
        .next_page(request(Some(first_token), stamps.clone()))
        .unwrap();
    assert_eq!(second.value.rows, original[2..4]);
    let second_token = second.value.next_cursor_id.clone().unwrap();
    let last = owner
        .next_page(request(Some(second_token), stamps))
        .unwrap();
    assert_eq!(last.value.rows, original[4..]);
    assert!(last.value.next_cursor_id.is_none());
    assert!(matches!(last.value.end, wire::PageEnd::DeclaredComplete(_)));
    let joined = [first.value.rows, second.value.rows, last.value.rows].concat();
    assert_eq!(joined, original);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-BOUND-PAGING",
    r = 1
)]
fn forged_replayed_cross_session_and_mixed_context_tokens_refuse() {
    let stamps = snapshot().stamps().to_vec();
    let mut a = session(rows(), ProjectionExtent::DeclaredComplete).unwrap();
    let mut b = session(rows(), ProjectionExtent::DeclaredComplete).unwrap();
    assert_eq!(
        a.next_page(request(Some("future-token".to_owned()), stamps.clone())),
        Err(PageSessionError::CursorForgedOrReplayed)
    );
    let first = a.next_page(request(None, stamps.clone())).unwrap();
    let token = first.value.next_cursor_id.unwrap();
    assert_ne!(
        b.next_page(request(None, stamps.clone()))
            .unwrap()
            .value
            .next_cursor_id
            .as_deref(),
        Some(token.as_str())
    );
    assert_eq!(
        b.next_page(request(Some(token.clone()), stamps.clone())),
        Err(PageSessionError::CursorForgedOrReplayed)
    );
    let mut wrong_scope = request(Some(token.clone()), stamps.clone());
    wrong_scope.authorized_scope = r("other-scope");
    assert_eq!(
        a.next_page(wrong_scope),
        Err(PageSessionError::MixedProjection)
    );
    let mut wrong_revision = request(Some(token.clone()), stamps.clone());
    wrong_revision.axis_stamps[0].revision = r("other-revision");
    assert_eq!(
        a.next_page(wrong_revision),
        Err(PageSessionError::MixedProjection)
    );
    a.next_page(request(Some(token.clone()), stamps.clone()))
        .unwrap();
    assert_eq!(
        a.next_page(request(Some(token), stamps)),
        Err(PageSessionError::CursorForgedOrReplayed)
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-ENTITY-REFERENCES",
    r = 1
)]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-BOUND-PAGING",
    r = 1
)]
fn entity_parent_and_partial_child_limits_are_checked_before_publication() {
    let mut foreign_revision = catalog();
    foreign_revision[4].captured.revision = "foreign-revision".to_owned();
    assert!(matches!(
        PageSession::new(
            &snapshot(), r("projection-a"), r("filter-all"), r("scope-owned"),
            rows(), foreign_revision, ProjectionExtent::DeclaredComplete,
        ),
        Err(PageSessionError::EntitySnapshotMismatch(id)) if id == "row-4"
    ));
    let mut foreign_observation = catalog();
    foreign_observation[4].observed.observation = "foreign-observation".to_owned();
    foreign_observation[4].occurrence.observed.observation = "foreign-observation".to_owned();
    assert!(matches!(
        PageSession::new(
            &snapshot(), r("projection-a"), r("filter-all"), r("scope-owned"),
            rows(), foreign_observation, ProjectionExtent::DeclaredComplete,
        ),
        Err(PageSessionError::EntitySnapshotMismatch(id)) if id == "row-4"
    ));
    let multi = CompositeSnapshot::compose([
        snapshot()
            .axis(crate::machine_view::SnapshotAxis::Source)
            .clone(),
        AxisParticipation::Required(Some(AxisWitness {
            owner: r("world-owner"),
            capture: r("world-capture"),
            revision: r("world-rev"),
            observation: r("world-observation"),
            compatibility_claim: Some(r("join-a")),
            expected_revisions: Vec::new(),
            freshness: AxisFreshness::DeclaredCurrent,
            completeness: AxisCompleteness::DeclaredComplete,
        })),
        AxisParticipation::NotParticipating,
        AxisParticipation::NotParticipating,
    ])
    .unwrap();
    assert!(
        PageSession::new(
            &multi,
            r("projection-a"),
            r("filter-all"),
            r("scope-owned"),
            rows(),
            catalog(),
            ProjectionExtent::DeclaredComplete,
        )
        .is_ok(),
        "one matching participating source axis is enough in a multi-owner view"
    );
    let mut unknown = rows();
    unknown[0].entity_reference_id = "unbound".to_owned();
    assert_eq!(
        session(unknown, ProjectionExtent::DeclaredComplete).err(),
        Some(PageSessionError::UnknownEntity("unbound".to_owned()))
    );
    let mut future_parent = rows();
    future_parent[1].parent_occurrence_id = Some("occ-3".to_owned());
    assert_eq!(
        session(future_parent, ProjectionExtent::DeclaredComplete).err(),
        Some(PageSessionError::ParentNotPrior("occ-1".to_owned()))
    );
    let mut duplicate = rows();
    duplicate[2].occurrence_id = "occ-1".to_owned();
    assert_eq!(
        session(duplicate, ProjectionExtent::DeclaredComplete).err(),
        Some(PageSessionError::DuplicateOccurrence("occ-1".to_owned()))
    );
    let mut self_parent = rows();
    self_parent[1].parent_occurrence_id = Some("occ-1".to_owned());
    assert_eq!(
        session(self_parent, ProjectionExtent::DeclaredComplete).err(),
        Some(PageSessionError::ParentNotPrior("occ-1".to_owned()))
    );
    let mut late_blank = rows();
    late_blank[4].child_state =
        wire::ChildState::Unavailable(Box::new(wire::ChildStateUnavailable {
            reason: " ".to_owned(),
        }));
    assert!(matches!(
        session(late_blank, ProjectionExtent::DeclaredComplete),
        Err(PageSessionError::GeneratedLocal(PageWireFailure {
            fault: super::super::PageWireFault::BlankIdentity,
            ..
        }))
    ));

    let mut partial = rows();
    partial[2].child_state = wire::ChildState::Redacted(Box::new(wire::ChildStateRedacted {
        reason: "private".to_owned(),
    }));
    let mut owner = session(partial, ProjectionExtent::DeclaredComplete).unwrap();
    let stamps = snapshot().stamps().to_vec();
    let p1 = owner.next_page(request(None, stamps.clone())).unwrap();
    let p2 = owner
        .next_page(request(p1.value.next_cursor_id, stamps.clone()))
        .unwrap();
    let p3 = owner
        .next_page(request(p2.value.next_cursor_id, stamps))
        .unwrap();
    assert!(matches!(p3.value.end, wire::PageEnd::Incomplete(_)));
}
