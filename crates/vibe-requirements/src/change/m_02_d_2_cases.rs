use super::*;
#[test]
fn adoption_and_markers_remain_metadata_not_terminal_evidence() {
    let registry = vibe_facts::Registry::default();
    let fact = vibe_facts::AuthoredFact {
        address: "spec://org.demo/dep/DOC#ONE".into(),
        requirements: None,
        status: Some(vibe_facts::FactStatus::parse("impl/done").unwrap()),
    };
    let rows = change_fact_rows(&registry, "org.demo/dep", true, &[fact]).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].adoption.presence,
        vibe_wire::generated::requirements_report::AdoptionObservationPresence::Absent
    );
    assert!(rows[0].relations.is_empty());
    vibe_wire::behaviour::requirements_report::validate_fact_metadata(&rows[0]).unwrap();
}
