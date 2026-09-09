//! Epoch-2 authored requirements mapping and human projection.

use vibe_wire::generated::requirements_report::RequiredArtifactKind as K;

use crate::tests_query::{ctx, project};
use crate::{RequirementsQuery, query};

#[test]
fn query_maps_absent_singleton_and_full_authored_requirements() {
    let root = project(
        "# Rules\n\n\
         @fact:ABSENT unclassified.\n\n\
         @fact:SINGLE implemented. @requires:implementation @status:impl/done\n\n\
         @fact:FULL all kinds. @requires:specification,implementation,verification,documentation,decision,research,plan,disposition,external <status stage=\"spec\" state=\"done\" ref=\"artifact://fixture\"/>\n",
    );
    let report = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();
    let row = |fact: &str| {
        report
            .rows
            .iter()
            .find(|row| row.address.ends_with(&format!("#{fact}")))
            .unwrap_or_else(|| panic!("missing {fact}: {report:?}"))
    };

    assert!(row("ABSENT").authoring.requires.is_none());
    assert_eq!(
        row("SINGLE").authoring.requires,
        Some(vec![K::Implementation])
    );
    assert_eq!(
        row("FULL").authoring.requires,
        Some(vec![
            K::Specification,
            K::Implementation,
            K::Verification,
            K::Documentation,
            K::Decision,
            K::Research,
            K::Plan,
            K::Disposition,
            K::External,
        ])
    );
    vibe_wire::behaviour::requirements_report::validate(&report).unwrap();
}

#[test]
fn text_names_unclassified_absence_and_canonical_present_sets() {
    let root = project(
        "# Rules\n\n\
         @fact:ABSENT unclassified.\n\n\
         @fact:MULTI selected. @requires:specification,verification,decision @status:test/done\n",
    );
    let report = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();
    let text = crate::text::render(&report);
    assert!(
        text.contains("#ABSENT authoring=unmarked requires=unclassified adoption="),
        "{text}"
    );
    assert!(
        text.contains(
            "#MULTI authoring=marked=test/done requires=[specification,verification,decision] adoption="
        ),
        "{text}"
    );
    for forbidden in ["terminal=", "fulfilled=", "verified=", "verdict="] {
        assert!(
            !text.contains(forbidden),
            "{forbidden} leaked into:\n{text}"
        );
    }
}
