//! Epoch-2 authored requirements-set acceptance and refusal arms.

use super::{RequirementsError, RequirementsSetDefect, validate};
use crate::generated::requirements_report::RequiredArtifactKind as K;

use super::tests::base;

#[test]
fn absent_requirements_are_unclassified_and_valid() {
    let report = base();
    assert!(
        report
            .rows
            .iter()
            .all(|row| row.authoring.requires.is_none())
    );
    validate(&report).unwrap();
}

#[test]
fn singleton_multi_and_full_canonical_sets_are_valid() {
    for requirements in [
        vec![K::Implementation],
        vec![K::Specification, K::Verification, K::Decision],
        vec![
            K::Specification,
            K::Implementation,
            K::Verification,
            K::Documentation,
            K::Decision,
            K::Research,
            K::Plan,
            K::Disposition,
            K::External,
        ],
    ] {
        let mut report = base();
        report.rows[0].authoring.requires = Some(requirements);
        validate(&report).unwrap();
    }
}

#[test]
fn empty_duplicate_and_unsorted_sets_have_typed_refusals() {
    for (requirements, expected) in [
        (Vec::new(), RequirementsSetDefect::Empty),
        (
            vec![K::Implementation, K::Implementation],
            RequirementsSetDefect::Duplicate,
        ),
        (
            vec![K::Verification, K::Implementation],
            RequirementsSetDefect::OutOfOrder,
        ),
    ] {
        let mut report = base();
        report.rows[0].authoring.requires = Some(requirements);
        assert_eq!(
            validate(&report),
            Err(RequirementsError::RequirementsSet {
                index: 0,
                defect: expected,
            })
        );
    }
}

#[test]
fn requirements_require_marked_non_void_authoring_status() {
    let mut unmarked = base();
    unmarked.rows[0].authoring.presence =
        crate::generated::requirements_report::AuthoringObservationPresence::Unmarked;
    unmarked.rows[0].authoring.status = None;
    unmarked.rows[0].authoring.requires = Some(vec![K::Specification]);
    assert_eq!(
        validate(&unmarked),
        Err(RequirementsError::RequirementsSet {
            index: 0,
            defect: RequirementsSetDefect::UnmarkedAuthoring,
        })
    );

    let mut absent = base();
    absent.rows[0].authoring.status = None;
    absent.rows[0].authoring.requires = Some(vec![K::Specification]);
    assert_eq!(
        validate(&absent),
        Err(RequirementsError::RequirementsSet {
            index: 0,
            defect: RequirementsSetDefect::StatusAbsent,
        })
    );

    let mut void = base();
    void.rows[0].authoring.status.as_mut().unwrap().state =
        crate::generated::requirements_report::FactStatusState::Void;
    void.rows[0].authoring.requires = Some(vec![K::Specification]);
    assert_eq!(
        validate(&void),
        Err(RequirementsError::RequirementsSet {
            index: 0,
            defect: RequirementsSetDefect::VoidStatus,
        })
    );
}
