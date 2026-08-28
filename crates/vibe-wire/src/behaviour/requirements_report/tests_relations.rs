//! The relation-enrichment and status cross-axis arms — Q5, Q12 and
//! Q14 of the architecture's mutation matrix.
//!
//! Split from `tests_coherence.rs` at the 600-line budget, along the
//! seam between the BASE source layer (there: what could be read, and
//! which rows it may own) and the ENRICHMENT layer plus the adoption
//! axis (here: what the optional provider said, and which observation
//! words a source kind allows).

use super::tests::{
    HOST_SOURCE_DIGEST, PACKAGE_SOURCE_DIGEST, available, base, coordinate, edge, enriched,
    host_row, law_of, package_row, sorted,
};
use super::{RequirementsError, StatusAxis, validate};
use crate::generated::requirements_report::{
    AdoptionObservationPresence, RelationSourceProvenance, RelationSourceState,
    RequirementRelationVerb, RequirementSourceKind, SourceResult, SourceResultState,
};

/// A non-`available` base result, shaped legally for its state — the
/// same fixture the source half uses, restated because a `#[path]`
/// sibling module has no parent to borrow it from.
fn result(kind: RequirementSourceKind, package: &str, state: SourceResultState) -> SourceResult {
    let (digest, entries) = match state {
        SourceResultState::Invalid => (Some(PACKAGE_SOURCE_DIGEST.to_string()), None),
        SourceResultState::Orphaned => (None, Some(3)),
        _ => (None, None),
    };
    SourceResult {
        source: coordinate(kind, package),
        state,
        digest,
        reason_code: Some("the source could not contribute".to_string()),
        adoption_entries: entries,
    }
}

/// Sort the base layer the way the wire requires — by PACKAGE, which
/// is also its identity key.
fn sorted_sources(mut sources: Vec<SourceResult>) -> Vec<SourceResult> {
    sources.sort_by(|left, right| left.source.package.cmp(&right.source.package));
    sources
}

#[test]
fn a_relation_source_provenance_and_reason_follow_from_its_state() {
    use RelationSourceProvenance as P;
    use RelationSourceState as S;

    // The legal matrix, per kind. A HOST source may only speak of a
    // fresh project map; a PACKAGE source only of a carried one.
    for (state, provenance, reason) in [
        (S::Current, P::FreshProjectMap, None),
        (
            S::Stale,
            P::FreshProjectMap,
            Some("map is older than the tree"),
        ),
        (
            S::Invalid,
            P::FreshProjectMap,
            Some("malformed project map"),
        ),
        (S::Unavailable, P::None, Some("no project map")),
    ] {
        let mut report = enriched();
        report.relation_sources[0].state = state.clone();
        report.relation_sources[0].provenance = provenance;
        report.relation_sources[0].reason_code = reason.map(str::to_string);
        validate(&report).unwrap_or_else(|e| panic!("host {state:?} must stand: {e}"));
    }
    for (state, provenance, reason) in [
        (S::Carried, P::CarriedPackageMap, None),
        (S::Stale, P::CarriedPackageMap, Some("witness mismatch")),
        (
            S::Invalid,
            P::CarriedPackageMap,
            Some("malformed carried map"),
        ),
        (S::Unavailable, P::None, Some("no carried map")),
    ] {
        let mut report = enriched();
        report.relation_sources[1].state = state.clone();
        report.relation_sources[1].provenance = provenance;
        report.relation_sources[1].reason_code = reason.map(str::to_string);
        validate(&report).unwrap_or_else(|e| panic!("package {state:?} must stand: {e}"));
    }

    // Provenance that cannot follow from the state at all.
    for (state, provenance) in [
        (S::Current, P::CarriedPackageMap),
        (S::Current, P::None),
        (S::Carried, P::FreshProjectMap),
        (S::Unavailable, P::FreshProjectMap),
        (S::Stale, P::None),
        (S::Invalid, P::None),
    ] {
        let mut report = enriched();
        report.relation_sources[0].state = state.clone();
        report.relation_sources[0].provenance = provenance.clone();
        report.relation_sources[0].reason_code =
            matches!(state, S::Stale | S::Invalid | S::Unavailable).then(|| "why".to_string());
        assert_eq!(
            law_of(&report),
            "relation-state-matrix",
            "{state:?} + {provenance:?}"
        );
    }
}

/// Q14 and the provenance-kind inversion: a host source never carries
/// a package map, a package source never a fresh project map, and a
/// requested query answers for every package that owns a row.
#[test]
fn relation_coverage_and_provenance_follow_the_source_kind() {
    let mut host_carries = enriched();
    host_carries.relation_sources[0].state = RelationSourceState::Carried;
    host_carries.relation_sources[0].provenance = RelationSourceProvenance::CarriedPackageMap;
    let error = validate(&host_carries).unwrap_err();
    assert_eq!(error.law(), "relation-state-matrix");
    assert!(matches!(
        error,
        RequirementsError::RelationProvenanceKind { .. }
    ));

    let mut package_is_current = enriched();
    package_is_current.relation_sources[1].state = RelationSourceState::Current;
    package_is_current.relation_sources[1].provenance = RelationSourceProvenance::FreshProjectMap;
    assert!(matches!(
        validate(&package_is_current).unwrap_err(),
        RequirementsError::RelationProvenanceKind { .. }
    ));

    // Relations requested, but a package that owns a row is unanswered
    // for — even a zero-edge row needs its source state.
    let mut uncovered = enriched();
    uncovered.rows[1].relations.clear();
    uncovered.relation_sources.remove(1);
    let error = validate(&uncovered).unwrap_err();
    assert_eq!(error.law(), "relation-state-matrix");
    assert!(matches!(
        error,
        RequirementsError::RelationSourceMissing { .. }
    ));
}

/// Q5: a query that did not ask loaded no map, in both directions.
#[test]
fn a_query_that_did_not_ask_for_relations_loaded_no_map() {
    let mut requested = enriched();
    requested.relation_sources[0].state = RelationSourceState::NotRequested;
    requested.relation_sources[0].provenance = RelationSourceProvenance::None;
    assert_eq!(law_of(&requested), "relation-state-matrix");

    let mut unrequested = base();
    unrequested.relation_sources[0].state = RelationSourceState::Current;
    unrequested.relation_sources[0].provenance = RelationSourceProvenance::FreshProjectMap;
    assert_eq!(law_of(&unrequested), "relation-state-matrix");

    let mut edges = base();
    edges.rows[0].relations = vec![edge(RequirementRelationVerb::Implements, "demo::build", 12)];
    let error = validate(&edges).unwrap_err();
    assert_eq!(error.law(), "relation-state-matrix");
    assert!(matches!(
        error,
        RequirementsError::EdgesWithoutRequest { .. }
    ));
}

/// Q12: the source kind decides which adoption words are legal, and
/// the presence word decides whether a status is owed.
#[test]
fn a_presence_word_its_status_and_its_source_kind_are_one_statement() {
    let mut unmarked_with_status = base();
    unmarked_with_status.rows[1].authoring.status = Some(super::tests::status(
        crate::generated::requirements_report::FactStatusStage::Doc,
        crate::generated::requirements_report::FactStatusState::Done,
    ));
    let error = validate(&unmarked_with_status).unwrap_err();
    assert_eq!(error.law(), "status-presence");
    assert!(matches!(
        error,
        RequirementsError::StatusPresence {
            axis: StatusAxis::Authoring,
            expected: false,
            ..
        }
    ));

    let mut marked_without_status = base();
    marked_without_status.rows[0].authoring.status = None;
    assert!(matches!(
        validate(&marked_without_status).unwrap_err(),
        RequirementsError::StatusPresence {
            axis: StatusAxis::Authoring,
            expected: true,
            ..
        }
    ));

    let mut recorded_without_status = base();
    recorded_without_status.rows[1].adoption.status = None;
    assert!(matches!(
        validate(&recorded_without_status).unwrap_err(),
        RequirementsError::StatusPresence {
            axis: StatusAxis::Adoption,
            expected: true,
            ..
        }
    ));

    // A HOST row that carries a package adoption word…
    for presence in [
        AdoptionObservationPresence::Absent,
        AdoptionObservationPresence::Indeterminate,
    ] {
        let mut report = base();
        report.rows[0].adoption.presence = presence;
        let error = validate(&report).unwrap_err();
        assert_eq!(error.law(), "status-presence");
        assert!(matches!(
            error,
            RequirementsError::AdoptionKind { host: true, .. }
        ));
    }

    // …and the inverse: a PACKAGE row claiming there is no overlay to
    // consult, when its whole point is that there is one.
    let mut package_not_applicable = base();
    package_not_applicable.rows[1].adoption.presence = AdoptionObservationPresence::NotApplicable;
    package_not_applicable.rows[1].adoption.status = None;
    assert!(matches!(
        validate(&package_not_applicable).unwrap_err(),
        RequirementsError::AdoptionKind { host: false, .. }
    ));
}

/// One document exercising every corrected layer at once: a host row
/// under a stale fresh map, a package source that is invalid and
/// therefore owns nothing, and an orphaned third source.
#[test]
fn a_fully_exercised_document_still_validates() {
    let mut report = enriched();
    report.sources = sorted_sources(vec![
        available(
            RequirementSourceKind::Host,
            "org.demo/host",
            HOST_SOURCE_DIGEST,
        ),
        result(
            RequirementSourceKind::Package,
            "org.other/absent",
            SourceResultState::Orphaned,
        ),
        result(
            RequirementSourceKind::Package,
            "org.vendor/tool",
            SourceResultState::Invalid,
        ),
    ]);
    report.rows = sorted(vec![host_row()]);
    report.rows[0].relations = vec![
        edge(RequirementRelationVerb::Documents, "demo::doc", 3),
        edge(RequirementRelationVerb::Implements, "demo::build", 12),
    ];
    report
        .relation_sources
        .retain(|source| source.package == "org.demo/host");
    report.relation_sources[0].state = RelationSourceState::Stale;
    report.relation_sources[0].provenance = RelationSourceProvenance::FreshProjectMap;
    report.relation_sources[0].reason_code = Some("project-map-witness-mismatch".to_string());
    validate(&report).unwrap();

    // The package row cannot come back while its source is invalid:
    // the binding law refuses it before anything downstream is even
    // asked, which is the ordering the correction wanted.
    report.rows = sorted(vec![host_row(), package_row()]);
    assert_eq!(law_of(&report), "row-source-binding");
}
