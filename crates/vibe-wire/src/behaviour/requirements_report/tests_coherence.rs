//! The source-layer and cross-axis coherence arms — Q11–Q14 of the
//! architecture's mutation matrix, plus the base-source matrix the
//! central correction added.
//!
//! Split from `tests.rs` along the seam between «is this scalar
//! well-formed» (there) and «do these observations agree» (here).

use super::tests::{
    HOST_SOURCE_DIGEST, PACKAGE_SOURCE_DIGEST, available, base, coordinate, enriched, law_of,
    sorted,
};
use super::{ReasonDefect, RequirementsError, SourceDefect, SourceStateDefect, validate};
use crate::generated::requirements_report::{
    AuthoringObservationPresence, RelationSourceProvenance, RelationSourceState,
    RequirementSourceKind, SourceResult, SourceResultState,
};

/// A non-`available` base result, shaped legally for its state.
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

#[test]
fn every_base_source_state_has_a_legal_shape() {
    for state in [
        SourceResultState::Unavailable,
        SourceResultState::Invalid,
        SourceResultState::Orphaned,
    ] {
        let mut report = base();
        // A non-available source owns no rows, so its row goes too.
        report.sources[1] = result(RequirementSourceKind::Package, "org.vendor/tool", state);
        report
            .rows
            .retain(|row| !matches!(row.source.kind, RequirementSourceKind::Package));
        validate(&report).unwrap_or_else(|e| panic!("{:?}: {e}", report.sources[1].state));
    }
}

#[test]
fn the_base_source_matrix_refuses_every_member_its_state_cannot_mean() {
    let defect_of = |report: &_| match validate(report).unwrap_err() {
        RequirementsError::SourceStateMatrix { defect, .. } => defect,
        other => panic!("expected a source-result-matrix refusal, got {other:?}"),
    };
    let strip_package_rows =
        |report: &mut crate::generated::requirements_report::RequirementsReport| {
            report
                .rows
                .retain(|row| !matches!(row.source.kind, RequirementSourceKind::Package));
        };

    // available without a digest, and available WITH a reason.
    let mut no_digest = base();
    no_digest.sources[1].digest = None;
    assert_eq!(defect_of(&no_digest), SourceStateDefect::AbsentDigest);

    let mut bad_digest = base();
    bad_digest.sources[1].digest = Some("sha256:short".to_string());
    assert_eq!(defect_of(&bad_digest), SourceStateDefect::DigestShape);

    let mut reasoned = base();
    reasoned.sources[1].reason_code = Some("why".to_string());
    assert!(matches!(
        validate(&reasoned).unwrap_err(),
        RequirementsError::SourceReason {
            defect: ReasonDefect::Unexpected,
            ..
        }
    ));

    // unavailable that carries a digest of bytes it never read.
    let mut unavailable = base();
    strip_package_rows(&mut unavailable);
    unavailable.sources[1] = result(
        RequirementSourceKind::Package,
        "org.vendor/tool",
        SourceResultState::Unavailable,
    );
    let mut with_digest = unavailable.clone();
    with_digest.sources[1].digest = Some(PACKAGE_SOURCE_DIGEST.to_string());
    assert_eq!(defect_of(&with_digest), SourceStateDefect::UnexpectedDigest);

    let mut silent = unavailable.clone();
    silent.sources[1].reason_code = None;
    assert!(matches!(
        validate(&silent).unwrap_err(),
        RequirementsError::SourceReason {
            defect: ReasonDefect::Absent,
            ..
        }
    ));

    let mut blank = unavailable.clone();
    blank.sources[1].reason_code = Some("  ".to_string());
    assert!(matches!(
        validate(&blank).unwrap_err(),
        RequirementsError::SourceReason {
            defect: ReasonDefect::Blank,
            ..
        }
    ));

    // invalid READ the bytes, so it owes a digest of them.
    let mut invalid = base();
    strip_package_rows(&mut invalid);
    invalid.sources[1] = result(
        RequirementSourceKind::Package,
        "org.vendor/tool",
        SourceResultState::Invalid,
    );
    let mut invalid_no_digest = invalid.clone();
    invalid_no_digest.sources[1].digest = None;
    assert_eq!(
        defect_of(&invalid_no_digest),
        SourceStateDefect::AbsentDigest
    );

    // orphaned is exactly «entries with no source».
    let mut orphaned = base();
    strip_package_rows(&mut orphaned);
    orphaned.sources[1] = result(
        RequirementSourceKind::Package,
        "org.vendor/tool",
        SourceResultState::Orphaned,
    );
    let mut no_entries = orphaned.clone();
    no_entries.sources[1].adoption_entries = None;
    assert_eq!(
        defect_of(&no_entries),
        SourceStateDefect::AbsentAdoptionEntries
    );

    let mut zero_entries = orphaned.clone();
    zero_entries.sources[1].adoption_entries = Some(0);
    assert_eq!(
        defect_of(&zero_entries),
        SourceStateDefect::ZeroAdoptionEntries
    );

    let mut counted_available = base();
    counted_available.sources[1].adoption_entries = Some(2);
    assert_eq!(
        defect_of(&counted_available),
        SourceStateDefect::UnexpectedAdoptionEntries
    );
}

/// Q13: a malformed, missing or orphaned source emits no fact row.
/// This is the correction's whole point — a registry-only orphan is a
/// SOURCE observation, never a fabricated `unmarked` authored fact.
#[test]
fn only_an_available_source_owns_fact_rows() {
    for state in [
        SourceResultState::Unavailable,
        SourceResultState::Invalid,
        SourceResultState::Orphaned,
    ] {
        let mut report = base();
        report.sources[1] = result(RequirementSourceKind::Package, "org.vendor/tool", state);
        let error = validate(&report).unwrap_err();
        assert_eq!(error.law(), "row-source-binding");
        assert!(matches!(
            error,
            RequirementsError::RowFromUnavailableSource { .. }
        ));
    }

    // …and a row whose source this answer never named at all. The
    // enrichment entry for that package goes with it: `source-coherence`
    // now refuses an unbased relation source and would answer first,
    // so this arm keeps testing the binding it names.
    let mut unsourced = base();
    unsourced.sources.remove(1);
    unsourced
        .relation_sources
        .retain(|source| source.package != "org.vendor/tool");
    assert!(matches!(
        validate(&unsourced).unwrap_err(),
        RequirementsError::RowWithoutSource { .. }
    ));

    // One coordinate gets ONE base result, so a row disagreeing with
    // it about `host` vs `package` names a source that does not exist
    // — and the refusal says which side said what, rather than
    // pretending the package was never enumerated.
    let mut wrong_kind = base();
    wrong_kind.sources[1] = available(
        RequirementSourceKind::Host,
        "org.vendor/tool",
        PACKAGE_SOURCE_DIGEST,
    );
    wrong_kind.sources = sorted_sources(wrong_kind.sources);
    let error = validate(&wrong_kind).unwrap_err();
    assert_eq!(error.law(), "row-source-binding");
    assert!(
        matches!(
            error,
            RequirementsError::RowSourceKindMismatch {
                declared: RequirementSourceKind::Package,
                base: RequirementSourceKind::Host,
                ..
            }
        ),
        "the mismatch names both kinds, got {error:?}"
    );
}

/// Sort the base layer the way the wire requires — by PACKAGE, which
/// is also its identity key.
fn sorted_sources(mut sources: Vec<SourceResult>) -> Vec<SourceResult> {
    sources.sort_by(|left, right| left.source.package.cmp(&right.source.package));
    sources
}

/// Q11: the coordinate in a row's own address must be the package its
/// source names. Two packages in one row is two claims.
#[test]
fn a_rows_address_coordinate_is_its_source_package() {
    let mut mismatched = base();
    mismatched.rows[1].address =
        "spec://org.other/tool/vibevm/modules/PROP-002#req-two".to_string();
    mismatched.rows = sorted(mismatched.rows);
    let error = validate(&mismatched).unwrap_err();
    assert_eq!(error.law(), "row-source-binding");
    assert!(matches!(
        error,
        RequirementsError::CoordinateMismatch { .. }
    ));

    // …and the inverse mutation: same address, other source package.
    let mut moved_source = base();
    moved_source.rows[1].source.package = "org.demo/host".to_string();
    assert_eq!(law_of(&moved_source), "row-source-binding");
}

/// Coordinates are PARSED, not pattern-matched: a group with an
/// underscore and an uppercase package are both refused, on every
/// layer that carries a coordinate.
#[test]
fn every_coordinate_parses_under_the_projects_own_grammars() {
    for package in [
        "",
        "orgdemo",
        "org.demo/",
        "/host",
        "org.demo/a/b",
        "org_demo/host",
        "org.demo/Host",
    ] {
        let mut report = base();
        report.sources[0].source.package = package.to_string();
        let error = validate(&report).unwrap_err();
        assert_eq!(error.law(), "source-coherence", "sources {package:?}");
        assert!(matches!(
            error,
            RequirementsError::SourceCoherence {
                defect: SourceDefect::NotACoordinate,
                ..
            }
        ));

        let mut relations = enriched();
        relations.relation_sources[0].package = package.to_string();
        assert_eq!(
            law_of(&relations),
            "source-coherence",
            "relation_sources {package:?}"
        );
    }
}

/// One package coordinate gets ONE base source result, and the kind is
/// NOT part of that identity. `relation_sources` names a package and
/// nothing else, while its provenance law has to read exactly one kind
/// back off this layer — so a coordinate enumerated under both kinds
/// would make the very same document pass or fail depending on which
/// result a lookup reached first.
#[test]
fn one_package_coordinate_gets_one_base_source_whatever_its_kind() {
    let mut both_kinds = base();
    both_kinds.sources = sorted_sources(vec![
        available(
            RequirementSourceKind::Host,
            "org.demo/host",
            HOST_SOURCE_DIGEST,
        ),
        available(
            RequirementSourceKind::Package,
            "org.demo/host",
            PACKAGE_SOURCE_DIGEST,
        ),
        available(
            RequirementSourceKind::Package,
            "org.vendor/tool",
            PACKAGE_SOURCE_DIGEST,
        ),
    ]);
    let error = validate(&both_kinds).unwrap_err();
    assert_eq!(error.law(), "source-coherence");
    assert!(
        matches!(
            error,
            RequirementsError::SourceCoherence {
                defect: SourceDefect::DuplicatePackage,
                ..
            }
        ),
        "one coordinate under two kinds is a duplicate, got {error:?}"
    );
    // …and it stays refused with the kinds the other way round, so the
    // arm is about the coordinate rather than about `host` first.
    both_kinds.sources[0].source.kind = RequirementSourceKind::Package;
    both_kinds.sources[1].source.kind = RequirementSourceKind::Host;
    assert_eq!(law_of(&both_kinds), "source-coherence");
}

/// A relation source the base layer never enumerated has no kind for
/// the fresh-vs-carried law to apply. Skipping the law there is how a
/// document acquires a provenance verdict nobody checked, so the
/// entry is refused instead.
#[test]
fn a_relation_source_names_exactly_one_base_source() {
    let mut orphan = enriched();
    orphan
        .relation_sources
        .push(crate::generated::requirements_report::RelationSource {
            package: "org.zzz/unknown".to_string(),
            state: RelationSourceState::Carried,
            provenance: RelationSourceProvenance::CarriedPackageMap,
            reason_code: None,
        });
    let error = validate(&orphan).unwrap_err();
    assert_eq!(error.law(), "source-coherence");
    assert!(
        matches!(
            error,
            RequirementsError::SourceCoherence {
                defect: SourceDefect::NoBaseSource,
                ..
            }
        ),
        "an unbased relation source must be named, got {error:?}"
    );

    // The same entry with a base source result is legal — proving the
    // refusal is about the missing BASE, not about the package.
    let mut based = orphan;
    based.sources = sorted_sources({
        let mut sources = based.sources.clone();
        sources.push(available(
            RequirementSourceKind::Package,
            "org.zzz/unknown",
            PACKAGE_SOURCE_DIGEST,
        ));
        sources
    });
    validate(&based).unwrap();
}

#[test]
fn each_source_layer_is_sorted_and_named_once() {
    let mut duplicate = base();
    duplicate.sources[1] = available(
        RequirementSourceKind::Host,
        "org.demo/host",
        HOST_SOURCE_DIGEST,
    );
    assert!(matches!(
        validate(&duplicate).unwrap_err(),
        RequirementsError::SourceCoherence {
            defect: SourceDefect::DuplicatePackage,
            ..
        }
    ));

    let mut unsorted = base();
    unsorted.sources.reverse();
    assert!(matches!(
        validate(&unsorted).unwrap_err(),
        RequirementsError::SourceCoherence {
            defect: SourceDefect::OutOfOrder,
            ..
        }
    ));

    let mut duplicate_relation = base();
    duplicate_relation.relation_sources[1].package = "org.demo/host".to_string();
    assert!(matches!(
        validate(&duplicate_relation).unwrap_err(),
        RequirementsError::SourceCoherence {
            defect: SourceDefect::DuplicatePackage,
            ..
        }
    ));

    let mut unsorted_relation = base();
    unsorted_relation.relation_sources.reverse();
    assert!(matches!(
        validate(&unsorted_relation).unwrap_err(),
        RequirementsError::SourceCoherence {
            defect: SourceDefect::OutOfOrder,
            ..
        }
    ));
}

#[test]
fn an_unmarked_authoring_row_is_still_a_real_row() {
    // The orphan correction does not remove the legitimate `unmarked`
    // case: an addressed fact in an AVAILABLE source with no marker
    // still appears, and that is different from a registry orphan.
    let mut report = base();
    report.rows[1].authoring.presence = AuthoringObservationPresence::Unmarked;
    report.rows[1].authoring.status = None;
    validate(&report).unwrap();
}
