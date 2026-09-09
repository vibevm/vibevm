//! Epoch-2 requirements-set corpus and compatibility proofs.
//!
//! The current root owns the optional authored set. Epoch-1 corpus bytes
//! remain readable as historical input, but current semantic validation
//! rejects their epoch instead of silently treating them as epoch 2.

#[path = "wire_support/mod.rs"]
mod support;
use support::read_json;

use vibe_wire::behaviour::requirements_report::{
    RequirementsError, RequirementsSetDefect, validate,
};
use vibe_wire::generated::requirements_report::{RequiredArtifactKind, RequirementsReport};

fn current(name: &str) -> RequirementsReport {
    let authored = read_json(&format!("formats/corpora/requirements/e2/{name}"));
    let report: RequirementsReport =
        serde_json::from_value(authored.clone()).unwrap_or_else(|error| panic!("{name}: {error}"));
    assert_eq!(serde_json::to_value(&report).unwrap(), authored);
    validate(&report).unwrap_or_else(|error| panic!("{name}: {error}"));
    report
}

#[test]
fn positive_corpus_pins_absent_singleton_multi_and_full_sets() {
    assert!(
        current("report_base.json").rows[0]
            .authoring
            .requires
            .is_none()
    );

    assert_eq!(
        current("report_relations.json").rows[0].authoring.requires,
        Some(vec![RequiredArtifactKind::Implementation])
    );
    assert_eq!(
        current("report_partial.json").rows[0].authoring.requires,
        Some(vec![
            RequiredArtifactKind::Specification,
            RequiredArtifactKind::Verification,
            RequiredArtifactKind::Decision,
        ])
    );
    assert_eq!(
        current("report_truncated.json").rows[0].authoring.requires,
        Some(vec![
            RequiredArtifactKind::Specification,
            RequiredArtifactKind::Implementation,
            RequiredArtifactKind::Verification,
            RequiredArtifactKind::Documentation,
            RequiredArtifactKind::Decision,
            RequiredArtifactKind::Research,
            RequiredArtifactKind::Plan,
            RequiredArtifactKind::Disposition,
            RequiredArtifactKind::External,
        ])
    );
}

#[test]
fn relational_refusal_corpus_is_typed() {
    for (name, defect) in [
        (
            "requires_unmarked.json",
            RequirementsSetDefect::UnmarkedAuthoring,
        ),
        (
            "requires_without_status.json",
            RequirementsSetDefect::StatusAbsent,
        ),
        ("requires_void.json", RequirementsSetDefect::VoidStatus),
        ("requires_empty.json", RequirementsSetDefect::Empty),
        ("requires_duplicate.json", RequirementsSetDefect::Duplicate),
        ("requires_unsorted.json", RequirementsSetDefect::OutOfOrder),
    ] {
        let authored = read_json(&format!("formats/corpora/requirements/e2/invalid/{name}"));
        let report: RequirementsReport = serde_json::from_value(authored).unwrap();
        assert_eq!(
            validate(&report),
            Err(RequirementsError::RequirementsSet { index: 0, defect }),
            "{name}"
        );
    }
}

#[test]
fn closed_vocabulary_and_wrong_epoch_shapes_are_refused() {
    let unknown = read_json("formats/corpora/requirements/e2/invalid/requires_unknown.json");
    let error = serde_json::from_value::<RequirementsReport>(unknown).unwrap_err();
    assert!(
        error.to_string().contains("unknown variant `mystery`"),
        "{error}"
    );

    let wrong = read_json("formats/corpora/requirements/e2/invalid/requires_under_epoch_1.json");
    let report: RequirementsReport = serde_json::from_value(wrong).unwrap();
    assert_eq!(
        validate(&report),
        Err(RequirementsError::RequirementsEpoch { found: 1 })
    );
}

#[test]
fn epoch_1_corpus_remains_historical_input_not_current_output() {
    for name in [
        "report_base.json",
        "report_partial.json",
        "report_relations.json",
        "report_truncated.json",
    ] {
        let authored = read_json(&format!("formats/corpora/requirements/e1/{name}"));
        let report: RequirementsReport = serde_json::from_value(authored.clone()).unwrap();
        assert!(
            report
                .rows
                .iter()
                .all(|row| row.authoring.requires.is_none())
        );
        assert_eq!(
            serde_json::to_value(&report).unwrap(),
            authored,
            "{name} does not round-trip as historical input"
        );
        assert_eq!(
            validate(&report),
            Err(RequirementsError::RequirementsEpoch { found: 1 }),
            "{name} must not pass as a current epoch-2 report"
        );
    }
}
