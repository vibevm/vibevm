use super::*;
use crate::evidence::{Evidence, EvidenceProvider, NoEvidence};
use crate::model::ArtifactKind;
use crate::parse::parse_document;
use crate::terminal::{ArtifactState, ProviderArtifactEvidence};

struct Exact;

impl EvidenceProvider for Exact {
    fn evidence_for(&self, address: &str) -> Option<Evidence> {
        Some(Evidence {
            implements: usize::from(address.ends_with("#A")),
            verifies: usize::from(address.ends_with("#B")),
            refs: Vec::new(),
        })
    }

    fn artifact_evidence_for(
        &self,
        address: &str,
        artifact: ArtifactKind,
        _reference: Option<&str>,
    ) -> ProviderArtifactEvidence {
        let locator = match (address.rsplit_once('#').map(|(_, id)| id), artifact) {
            (Some("A"), ArtifactKind::Implementation) => Some("impl:A"),
            (Some("B"), ArtifactKind::Verification) => Some("verify:B"),
            _ => None,
        };
        ProviderArtifactEvidence::Known {
            count: usize::from(locator.is_some()),
            locators: Some(locator.into_iter().map(str::to_string).collect()),
        }
    }
}

#[test]
fn mixed_report_exposes_terminal_and_unclassified_from_one_row_model() {
    let doc = parse_document(
        "spec/t.md",
        "# T {#t}\n\n@fact:A closed @requires:specification @spec/done\n\n@fact:B legacy @impl/done\n",
    );
    let report_rows = rows([&doc], None, None, &NoEvidence);
    assert_eq!(report_rows.len(), 2);
    assert_eq!(report_rows[0].terminal, Some(TerminalOutcome::Terminal));
    assert_eq!(report_rows[0].address.as_deref(), Some("spec/t.md#A"));
    assert_eq!(report_rows[1].terminal, Some(TerminalOutcome::Unclassified));
    assert_eq!(report_rows[1].address.as_deref(), Some("spec/t.md#B"));

    let rollups = vec![(doc.path.clone(), crate::rollup::rollup_doc(&doc))];
    let json = serde_json::to_string(&report_rows).unwrap();
    assert!(json.contains("\"terminal\":\"terminal\""), "{json}");
    assert!(json.contains("\"terminal\":\"unclassified\""), "{json}");
    let md = render_md(&report_rows, &rollups);
    assert!(md.contains("| requires | artifacts | terminal |"), "{md}");
    assert!(md.contains("specification=s"), "{md}");
    assert!(md.contains("| unclassified |"), "{md}");
    let xml = render_xml(&report_rows, &rollups);
    assert!(xml.contains("<progress-report schema=\"2\">"), "{xml}");
    assert!(
        xml.contains("address=\"spec/t.md#A\" requires=\"specification\" terminal=\"terminal\""),
        "{xml}"
    );
    assert!(
        xml.contains("address=\"spec/t.md#B\" terminal=\"unclassified\""),
        "{xml}"
    );

    let terminal = rows([&doc], Some(View::Terminal), None, &NoEvidence);
    assert_eq!(terminal.len(), 1);
    assert_eq!(terminal[0].address.as_deref(), Some("spec/t.md#A"));
    assert_eq!(rows([&doc], Some(View::Done), None, &NoEvidence).len(), 2);
}

#[test]
fn same_line_cells_resolve_against_their_exact_fact_addresses() {
    let doc = parse_document(
        "spec/t.md",
        "| A | B |\n|---|---|\n| @fact:A left @requires:implementation @impl/done | @fact:B right @requires:verification @test/done |\n",
    );
    let rows = rows([&doc], None, None, &Exact);
    assert_eq!(rows[0].address.as_deref(), Some("spec/t.md#A"));
    assert_eq!(rows[0].artifacts[0].state, ArtifactState::Satisfied);
    assert_eq!(rows[0].artifacts[0].locators, Some(vec!["impl:A".into()]));
    assert_eq!(rows[1].address.as_deref(), Some("spec/t.md#B"));
    assert_eq!(rows[1].artifacts[0].state, ArtifactState::Satisfied);
    assert_eq!(rows[1].artifacts[0].locators, Some(vec!["verify:B".into()]));
}

#[test]
fn invalid_document_never_receives_a_terminal_observation() {
    let valid = parse_document(
        "valid.md",
        "@fact:A yes @requires:specification @spec/done\n",
    );
    let invalid = parse_document("invalid.md", "@fact:B bad @requires:external @impl/done\n");
    assert!(invalid.error_count() > 0);
    let rows = rows([&valid, &invalid], None, None, &NoEvidence);
    let invalid_row = rows.iter().find(|row| row.path == "invalid.md").unwrap();
    assert!(invalid_row.address.is_none());
    assert!(invalid_row.requires.is_none());
    assert!(invalid_row.terminal.is_none());
    assert!(invalid_row.artifacts.is_empty());
}

#[test]
fn xml_known_zero_needs_no_synthetic_locator_vocabulary() {
    let doc = parse_document(
        "spec/t.md",
        "@fact:C absent @requires:implementation @impl/done\n",
    );
    let report_rows = rows([&doc], None, None, &Exact);
    let rollups = vec![(doc.path.clone(), crate::rollup::rollup_doc(&doc))];
    let xml = render_xml(&report_rows, &rollups);
    assert!(xml.contains("state=\"missing\" count=\"0\"/>"), "{xml}");
    assert!(!xml.contains("locators=\"known\""), "{xml}");
}

#[test]
fn empty_terminal_mode_still_announces_its_schema_and_columns() {
    assert!(render_xml(&[], &[]).contains("schema=\"1\""));
    assert!(render_xml_with_mode(&[], &[], true).contains("schema=\"2\""));
    let md = render_md_with_mode(&[], &[], true);
    assert!(md.contains("| requires | artifacts | terminal |"), "{md}");
}

#[test]
fn unowned_fact_grain_markers_never_inherit_heading_evidence() {
    for source in [
        "# H {#h}\n\n@fact:A <status stage=\"spec\" state=\"done\">fragment</status>\n",
        "# H {#h}\n\nbody @impl/done\n",
    ] {
        let doc = parse_document("spec/t.md", source);
        let rows = rows([&doc], None, None, &Exact);
        assert!(!rows.is_empty());
        assert!(
            rows.iter().all(|row| row.evidence.is_none()),
            "{source}: {rows:#?}"
        );
    }
}

#[test]
fn classified_freeze_uses_its_set_while_unclassified_keeps_legacy_warning() {
    let doc = parse_document(
        "spec/t.md",
        "@fact:A contract @requires:specification @freeze/done\n\n@fact:B old @freeze/done\n\n@fact:C checked @requires:specification @test/done\n",
    );
    let rows = rows([&doc], None, None, &Exact);
    assert!(
        rows[0].mismatch.is_none(),
        "classified freeze has no generic implementation duty"
    );
    assert!(
        rows[1]
            .mismatch
            .as_deref()
            .is_some_and(|value| value.contains("freeze"))
    );
    assert!(
        rows[2]
            .mismatch
            .as_deref()
            .is_some_and(|value| value.contains("test/done"))
    );
}
