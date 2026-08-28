//! RED arms for the identity, query, address, order and truncation
//! laws, plus the positives that keep them honest. The source-layer
//! and cross-axis coherence arms live beside this file in
//! `tests_coherence.rs`, split along that seam when the suite outgrew
//! the per-file budget.

use super::{
    ADDRESS_CAP_BYTES, AddressDefect, EdgeRef, IMPLEMENTED_LAWS, PathUnsafety, RequirementsError,
    validate, validate_edge,
};
use crate::behaviour::compiler_trace_index::DIAGNOSTIC_CAP_BYTES;
use crate::generated::requirements_report::{
    AdoptionObservation, AdoptionObservationPresence, AuthoringObservation,
    AuthoringObservationPresence, FactStatus, FactStatusStage, FactStatusState, RelationSource,
    RelationSourceProvenance, RelationSourceState, RequirementRelation,
    RequirementRelationProvenance, RequirementRelationVerb, RequirementRow, RequirementSource,
    RequirementSourceKind, RequirementsObservation, RequirementsQuery, RequirementsReport,
    SourceResult, SourceResultState,
};

pub(super) const OBSERVATION_ID: &str =
    "sha256:1111111111111111111111111111111111111111111111111111111111111111";
pub(super) const SOURCE_DIGEST: &str =
    "sha256:2222222222222222222222222222222222222222222222222222222222222222";
pub(super) const HOST_SOURCE_DIGEST: &str =
    "sha256:3333333333333333333333333333333333333333333333333333333333333333";
pub(super) const PACKAGE_SOURCE_DIGEST: &str =
    "sha256:4444444444444444444444444444444444444444444444444444444444444444";
pub(super) const RUN_ID: &str = "00112233445566778899aabbccddeeff";
pub(super) const HOST_ADDRESS: &str = "spec://org.demo/host/vibevm/common/PROP-001#req-one";
pub(super) const PACKAGE_ADDRESS: &str = "spec://org.vendor/tool/vibevm/modules/PROP-002#req-two";

pub(super) fn observation() -> RequirementsObservation {
    RequirementsObservation {
        observation_id: OBSERVATION_ID.to_string(),
        observed_at: "2026-08-28T12:00:00Z".parse().unwrap(),
        selected: ".".to_string(),
        source_digest: SOURCE_DIGEST.to_string(),
        lifecycle_run_id: Some(RUN_ID.to_string()),
    }
}

pub(super) fn query(relations: bool) -> RequirementsQuery {
    RequirementsQuery {
        limit: 100,
        relations,
        address_prefix: None,
    }
}

pub(super) fn status(stage: FactStatusStage, state: FactStatusState) -> FactStatus {
    FactStatus { stage, state }
}

pub(super) fn coordinate(kind: RequirementSourceKind, package: &str) -> RequirementSource {
    RequirementSource {
        kind,
        package: package.to_string(),
    }
}

/// One `available` base source result — the only state that owns rows.
pub(super) fn available(kind: RequirementSourceKind, package: &str, digest: &str) -> SourceResult {
    SourceResult {
        source: coordinate(kind, package),
        state: SourceResultState::Available,
        digest: Some(digest.to_string()),
        reason_code: None,
        adoption_entries: None,
    }
}

pub(super) fn host_row() -> RequirementRow {
    RequirementRow {
        address: HOST_ADDRESS.to_string(),
        source: coordinate(RequirementSourceKind::Host, "org.demo/host"),
        authoring: AuthoringObservation {
            presence: AuthoringObservationPresence::Marked,
            status: Some(status(FactStatusStage::Spec, FactStatusState::Work)),
        },
        adoption: AdoptionObservation {
            presence: AdoptionObservationPresence::NotApplicable,
            status: None,
        },
        relations: Vec::new(),
    }
}

pub(super) fn package_row() -> RequirementRow {
    RequirementRow {
        address: PACKAGE_ADDRESS.to_string(),
        source: coordinate(RequirementSourceKind::Package, "org.vendor/tool"),
        authoring: AuthoringObservation {
            presence: AuthoringObservationPresence::Unmarked,
            status: None,
        },
        adoption: AdoptionObservation {
            presence: AdoptionObservationPresence::Recorded,
            status: Some(status(FactStatusStage::Impl, FactStatusState::Done)),
        },
        relations: Vec::new(),
    }
}

pub(super) fn edge(verb: RequirementRelationVerb, symbol: &str, line: u32) -> RequirementRelation {
    RequirementRelation {
        verb,
        provenance: RequirementRelationProvenance::Authored,
        symbol: symbol.to_string(),
        file: "crates/demo/src/lib.rs".to_string(),
        line,
    }
}

/// One legal base: two available sources, two rows in address order,
/// relations not requested.
pub(super) fn base() -> RequirementsReport {
    RequirementsReport {
        requirements: 1,
        observation: observation(),
        query: query(false),
        sources: vec![
            available(
                RequirementSourceKind::Host,
                "org.demo/host",
                HOST_SOURCE_DIGEST,
            ),
            available(
                RequirementSourceKind::Package,
                "org.vendor/tool",
                PACKAGE_SOURCE_DIGEST,
            ),
        ],
        relation_sources: vec![
            RelationSource {
                package: "org.demo/host".to_string(),
                state: RelationSourceState::NotRequested,
                provenance: RelationSourceProvenance::None,
                reason_code: None,
            },
            RelationSource {
                package: "org.vendor/tool".to_string(),
                state: RelationSourceState::NotRequested,
                provenance: RelationSourceProvenance::None,
                reason_code: None,
            },
        ],
        rows: sorted(vec![host_row(), package_row()]),
        truncated: false,
    }
}

/// One legal enriched base: a current host map, a carried package map,
/// and two edges on each row.
pub(super) fn enriched() -> RequirementsReport {
    let mut report = base();
    report.query = query(true);
    report.relation_sources[0].state = RelationSourceState::Current;
    report.relation_sources[0].provenance = RelationSourceProvenance::FreshProjectMap;
    report.relation_sources[1].state = RelationSourceState::Carried;
    report.relation_sources[1].provenance = RelationSourceProvenance::CarriedPackageMap;
    for row in &mut report.rows {
        row.relations = vec![
            edge(RequirementRelationVerb::Implements, "demo::build", 12),
            edge(RequirementRelationVerb::Verifies, "demo::tests::build", 40),
        ];
    }
    report
}

/// Sort rows the way the wire requires, so a fixture edit cannot make
/// an unrelated arm fail on `row-order`.
pub(super) fn sorted(mut rows: Vec<RequirementRow>) -> Vec<RequirementRow> {
    rows.sort_by(|left, right| left.address.cmp(&right.address));
    rows
}

pub(super) fn law_of(report: &RequirementsReport) -> &'static str {
    validate(report).unwrap_err().law()
}

#[test]
fn the_legal_shapes_all_validate() {
    validate(&base()).unwrap();
    validate(&enriched()).unwrap();

    // An empty answer, a member-selected node and an absent lifecycle
    // run are all legal — a project that never ran a phase still
    // answers.
    let mut empty = base();
    empty.rows.clear();
    empty.sources.clear();
    empty.relation_sources.clear();
    empty.observation.selected = "members/tool".to_string();
    empty.observation.lifecycle_run_id = None;
    validate(&empty).unwrap();

    // Every adoption presence a PACKAGE row may carry with no status.
    for presence in [
        AdoptionObservationPresence::Absent,
        AdoptionObservationPresence::Indeterminate,
    ] {
        let mut report = base();
        report.rows[1].adoption.presence = presence;
        report.rows[1].adoption.status = None;
        validate(&report).unwrap();
    }
}

#[test]
fn the_law_list_is_a_set() {
    let mut sorted = IMPLEMENTED_LAWS.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), IMPLEMENTED_LAWS.len(), "duplicate law label");
    assert!(IMPLEMENTED_LAWS.iter().all(|law| !law.trim().is_empty()));
}

#[test]
fn the_report_identity_is_held_to_its_shape() {
    let mut epoch = base();
    epoch.requirements = 2;
    assert_eq!(law_of(&epoch), "report-identity");

    for field in ["observation_id", "source_digest"] {
        let mut report = base();
        match field {
            "observation_id" => report.observation.observation_id = "nope".to_string(),
            _ => report.observation.source_digest = OBSERVATION_ID.to_uppercase(),
        }
        assert_eq!(law_of(&report), "report-identity", "{field}");
    }

    let mut run = base();
    run.observation.lifecycle_run_id = Some("00112233445566778899AABBCCDDEEFF".to_string());
    assert_eq!(law_of(&run), "report-identity");

    for (selected, reason) in [
        ("", PathUnsafety::Blank),
        ("members\\tool", PathUnsafety::Backslash),
        ("/members/tool", PathUnsafety::Absolute),
        ("C:/work", PathUnsafety::DriveLetter),
        ("../out", PathUnsafety::ParentSegment),
        ("./members", PathUnsafety::DotSegment),
        ("members//tool", PathUnsafety::EmptySegment),
    ] {
        let mut report = base();
        report.observation.selected = selected.to_string();
        let error = validate(&report).unwrap_err();
        assert_eq!(error.law(), "report-identity", "selected {selected:?}");
        assert!(
            matches!(error, RequirementsError::UnsafeSelected { reason: found, .. } if found == reason)
        );
    }
}

#[test]
fn the_query_the_answer_restates_is_one_the_surfaces_would_have_accepted() {
    for limit in [0, 257, u32::MAX] {
        let mut report = base();
        report.query.limit = limit;
        assert_eq!(law_of(&report), "query-bounds", "limit {limit}");
    }
    // Both ends of the closed range stand — the hard maximum is
    // arithmetic, and this is where it is proven.
    for limit in [1, 256] {
        let mut report = base();
        report.query.limit = limit;
        report.rows.truncate(limit as usize);
        validate(&report).unwrap();
    }

    let mut bare_id = base();
    bare_id.query.address_prefix = Some("req-one".to_string());
    assert_eq!(law_of(&bare_id), "query-bounds");

    let mut unsafe_prefix = base();
    unsafe_prefix.query.address_prefix = Some("spec://org.demo/host\n".to_string());
    assert_eq!(law_of(&unsafe_prefix), "query-bounds");

    // The prefix's LENGTH is the bounded-text law's, not this one's —
    // the schema splits them deliberately, and the split is what keeps
    // one mutation from landing in two laws.
    let mut long_prefix = base();
    long_prefix.query.address_prefix = Some(format!("spec://{}", "x".repeat(ADDRESS_CAP_BYTES)));
    assert_eq!(law_of(&long_prefix), "bounded-text");
}

#[test]
fn the_shared_pure_edge_entry_agrees_with_the_edge_gate() {
    // `validate_edge` is the exact per-edge shape law the full edge
    // gate applies, exposed for a relation provider that must validate
    // its edges through the ONE path grammar (A2b follow-up C6). Pin
    // the agreement both ways on a sound row.
    let mut report = base();
    // Relations requested, both sources answered — the shape edges may
    // exist in.
    report.query = query(true);
    report.relation_sources[0].state = RelationSourceState::Current;
    report.relation_sources[0].provenance = RelationSourceProvenance::FreshProjectMap;
    report.relation_sources[1].state = RelationSourceState::Carried;
    report.relation_sources[1].provenance = RelationSourceProvenance::CarriedPackageMap;
    report.rows[0].relations = vec![RequirementRelation {
        verb: RequirementRelationVerb::Verifies,
        provenance: RequirementRelationProvenance::Authored,
        symbol: "demo::t".to_string(),
        file: "crates/demo/src/lib.rs".to_string(),
        line: 1,
    }];
    validate(&report).unwrap();
    for (edge_index, edge) in report.rows[0].relations.iter().enumerate() {
        validate_edge(
            EdgeRef {
                row: 0,
                edge: edge_index,
            },
            edge,
        )
        .unwrap();
    }
    // Every shape failure the entry owns: 0 line, blank symbol, unsafe
    // path, over-cap/control-bearing scalar — each moves both gates.
    for broken in [
        RequirementRelation {
            line: 0,
            ..report.rows[0].relations[0].clone()
        },
        RequirementRelation {
            symbol: "  ".to_string(),
            ..report.rows[0].relations[0].clone()
        },
        RequirementRelation {
            file: "..\\escape.rs".to_string(),
            ..report.rows[0].relations[0].clone()
        },
        RequirementRelation {
            symbol: "x".repeat(DIAGNOSTIC_CAP_BYTES + 1),
            ..report.rows[0].relations[0].clone()
        },
        RequirementRelation {
            symbol: "demo::t\nrewritten".to_string(),
            ..report.rows[0].relations[0].clone()
        },
        RequirementRelation {
            file: "x".repeat(DIAGNOSTIC_CAP_BYTES + 1),
            ..report.rows[0].relations[0].clone()
        },
    ] {
        let mut report = report.clone();
        report.rows[0].relations = vec![broken.clone()];
        assert!(
            validate_edge(EdgeRef { row: 0, edge: 0 }, &broken).is_err(),
            "the pure entry must refuse: {broken:?}"
        );
        assert!(matches!(law_of(&report), "edge-bounds" | "bounded-text"));
    }
}

#[test]
fn the_shared_pure_query_entry_agrees_with_the_report_gate() {
    // `validate_query` is the exact grammar the full validator applies
    // to `report.query`, exposed for a query library that must refuse
    // BEFORE filesystem access (R7.5 A2b). Pin the agreement both ways.
    use super::validate_query;

    let mut report = base();
    validate(&report).unwrap();
    assert!(validate_query(&report.query).is_ok());

    for limit in [0, 257, u32::MAX] {
        report.query.limit = limit;
        assert_eq!(law_of(&report), "query-bounds", "limit {limit}");
        assert!(validate_query(&report.query).is_err(), "limit {limit}");
    }
    report.query.limit = 100;
    report.query.address_prefix = Some("req-one".to_string());
    assert_eq!(law_of(&report), "query-bounds");
    assert!(validate_query(&report.query).is_err());
}

#[test]
fn every_row_carries_a_full_fact_address() {
    for (address, defect) in [
        ("req-one", AddressDefect::NoScheme),
        ("spec://org.demo\\host/doc#f", AddressDefect::Backslash),
        ("spec://org.demo/host/doc", AddressDefect::NoAnchor),
        ("spec://org.demo/host/doc#a#b", AddressDefect::ExtraAnchor),
        ("spec://org.demo/host/doc#", AddressDefect::BlankAnchor),
        ("spec://org.demo/host#f", AddressDefect::TooFewSegments),
        ("spec://org.demo/host//doc#f", AddressDefect::BlankSegment),
        ("spec://org.demo/host/../x#f", AddressDefect::ParentSegment),
        ("spec://org.demo/host/./x#f", AddressDefect::DotSegment),
        // Coordinates that only LOOK like coordinates: an underscore
        // is not legal in a group label, and an uppercase package is
        // not kebab-case.
        (
            "spec://org_demo/host/doc#f",
            AddressDefect::UnparseableGroup,
        ),
        (
            "spec://org.demo/Host/doc#f",
            AddressDefect::UnparseablePackage,
        ),
    ] {
        let mut report = base();
        report.rows = vec![host_row()];
        report.rows[0].address = address.to_string();
        let error = validate(&report).unwrap_err();
        assert_eq!(error.law(), "address-grammar", "address {address:?}");
        assert!(
            matches!(error, RequirementsError::AddressGrammar { defect: found, .. } if found == defect),
            "address {address:?} must refuse as {defect:?}, got {error:?}"
        );
    }
}

#[test]
fn a_bounded_query_answers_inside_its_own_scope_and_in_order() {
    let mut scoped = base();
    scoped.query.address_prefix = Some("spec://org.demo/host/".to_string());
    assert_eq!(law_of(&scoped), "prefix-scope");
    scoped
        .rows
        .retain(|row| row.address.starts_with("spec://org.demo/host/"));
    validate(&scoped).unwrap();

    let mut reversed = base();
    reversed.rows.reverse();
    assert_eq!(law_of(&reversed), "row-order");

    let mut duplicated = base();
    duplicated.rows = vec![host_row(), host_row()];
    assert_eq!(law_of(&duplicated), "row-order");

    let mut unsorted_edges = enriched();
    unsorted_edges.rows[0].relations.reverse();
    assert_eq!(law_of(&unsorted_edges), "row-order");

    let mut duplicate_edges = enriched();
    duplicate_edges.rows[0].relations = vec![
        edge(RequirementRelationVerb::Implements, "demo::build", 12),
        edge(RequirementRelationVerb::Implements, "demo::build", 12),
    ];
    assert_eq!(law_of(&duplicate_edges), "row-order");
}

/// The edge sort key is the verb's WIRE spelling, and the generated
/// enum's declaration order disagrees with it: `Verifies` is declared
/// before `Documents`, while `documents` sorts before `verifies`. A
/// validator that sorted by the Rust discriminant would accept this
/// document and produce an order no JSON reader could reproduce.
#[test]
fn edge_order_follows_the_wire_spelling_not_the_enum_discriminant() {
    let mut discriminant_order = enriched();
    discriminant_order.rows[0].relations = vec![
        edge(RequirementRelationVerb::Verifies, "demo::a", 1),
        edge(RequirementRelationVerb::Documents, "demo::a", 1),
    ];
    assert_eq!(
        law_of(&discriminant_order),
        "row-order",
        "verifies before documents is the ENUM order, not the wire order"
    );

    let mut wire_order = enriched();
    wire_order.rows[0].relations = vec![
        edge(RequirementRelationVerb::Documents, "demo::a", 1),
        edge(RequirementRelationVerb::Verifies, "demo::a", 1),
    ];
    validate(&wire_order).unwrap();

    // The whole closed set, in wire order, is one legal edge list.
    let mut all_verbs = enriched();
    all_verbs.rows[0].relations = vec![
        edge(RequirementRelationVerb::Deviates, "demo::a", 1),
        edge(RequirementRelationVerb::Documents, "demo::a", 1),
        edge(RequirementRelationVerb::Implements, "demo::a", 1),
        edge(RequirementRelationVerb::Informs, "demo::a", 1),
        edge(RequirementRelationVerb::Verifies, "demo::a", 1),
    ];
    validate(&all_verbs).unwrap();
}

#[test]
fn an_edge_points_at_a_place_a_reader_can_open() {
    let mut zero_line = enriched();
    zero_line.rows[0].relations[0].line = 0;
    assert_eq!(law_of(&zero_line), "edge-bounds");

    let mut blank_symbol = enriched();
    blank_symbol.rows[0].relations[0].symbol = "  ".to_string();
    assert_eq!(law_of(&blank_symbol), "edge-bounds");

    for (file, reason) in [
        ("", PathUnsafety::Blank),
        ("crates\\demo\\src\\lib.rs", PathUnsafety::Backslash),
        ("/etc/passwd", PathUnsafety::Absolute),
        ("C:/work/demo.rs", PathUnsafety::DriveLetter),
        ("../outside/lib.rs", PathUnsafety::ParentSegment),
        ("./src/lib.rs", PathUnsafety::DotSegment),
        ("src//lib.rs", PathUnsafety::EmptySegment),
    ] {
        let mut report = enriched();
        report.rows[0].relations[0].file = file.to_string();
        let error = validate(&report).unwrap_err();
        assert_eq!(error.law(), "edge-bounds", "file {file:?}");
        assert!(
            matches!(error, RequirementsError::EdgeFile { reason: found, .. } if found == reason)
        );
    }
}

#[test]
fn every_scalar_is_bounded_and_printable() {
    let mut long_address = base();
    long_address.rows = vec![host_row()];
    long_address.rows[0].address =
        format!("spec://org.demo/host/{}#f", "x".repeat(ADDRESS_CAP_BYTES));
    let error = validate(&long_address).unwrap_err();
    assert_eq!(error.law(), "bounded-text");
    assert!(matches!(
        error,
        RequirementsError::ScalarOverCap {
            field: "address",
            ..
        }
    ));

    let mut newline_address = base();
    newline_address.rows[0].address = format!("{HOST_ADDRESS}\n");
    assert_eq!(law_of(&newline_address), "bounded-text");

    let mut long_symbol = enriched();
    long_symbol.rows[0].relations[0].symbol = "s".repeat(DIAGNOSTIC_CAP_BYTES + 1);
    assert_eq!(law_of(&long_symbol), "bounded-text");

    let mut long_reason = enriched();
    long_reason.relation_sources[1].state = RelationSourceState::Unavailable;
    long_reason.relation_sources[1].provenance = RelationSourceProvenance::None;
    long_reason.relation_sources[1].reason_code = Some("r".repeat(DIAGNOSTIC_CAP_BYTES + 1));
    assert_eq!(law_of(&long_reason), "bounded-text");
}

#[test]
fn a_truncated_answer_reached_its_own_bound() {
    let mut over = base();
    over.query.limit = 1;
    assert_eq!(law_of(&over), "truncation-honesty");

    let mut claimed = base();
    claimed.truncated = true;
    assert_eq!(law_of(&claimed), "truncation-honesty");

    // …and the honest truncation: exactly `limit` rows, claim made.
    let mut honest = base();
    honest.query.limit = 2;
    honest.truncated = true;
    validate(&honest).unwrap();

    // Exactly `limit` rows WITHOUT a claim is equally legal: the set
    // may simply have ended there.
    honest.truncated = false;
    validate(&honest).unwrap();

    // An empty answer never claims truncation.
    let mut empty_claim = base();
    empty_claim.rows.clear();
    empty_claim.truncated = true;
    assert_eq!(law_of(&empty_claim), "truncation-honesty");
}
