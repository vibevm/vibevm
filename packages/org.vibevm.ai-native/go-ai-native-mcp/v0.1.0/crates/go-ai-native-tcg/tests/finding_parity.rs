//! The finding-parity test (TCG-PROTOCOL-GO §3): the relay's
//! enrichment must surface the SAME finding set `go-ai-native-conform
//! check` reports for the same file — one engine, one truth, and this
//! test is what keeps the promise falsifiable. Runs the REAL extractor
//! (go required — a stack obligation).

use std::path::PathBuf;

fn fixture_dirty() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/go-extract/test/fixtures/dirty")
}

#[test]
fn relay_enrichment_matches_the_gate_on_the_dirty_cell() {
    let root = fixture_dirty();
    let policy = go_ai_native_tcg::Policy::load(&root).expect("policy");
    let file = "internal/cells/plan/plan.go";
    let text = std::fs::read_to_string(root.join(file)).expect("fixture");

    let enriched = go_ai_native_tcg::enrich_validate(
        &policy,
        file,
        &text,
        go_ai_native_tcg_bridge::ValidateOutcome {
            diagnostics: vec![],
            degraded: false,
        },
    );

    // The gate's own census on this one file: 9 findings (see the
    // conform gate test — the file's 8 census sites + the suppression;
    // t_skip lives in the sibling test file, not here).
    let rules: Vec<&str> = enriched
        .conform_findings
        .iter()
        .map(|f| f.rule.as_str())
        .collect();
    assert_eq!(
        rules.iter().filter(|r| **r == "go-unsafe-in-domain").count(),
        9,
        "{:?}",
        enriched.conform_findings
    );
    // Advice cites the guide, deduplicated per rule/kind.
    assert!(!enriched.advice.is_empty());
    assert!(enriched.advice.iter().all(|a| a.contains("GUIDE-AI-NATIVE-GO")));
    // The markers stream is FILLED (the protocol's named delta): the
    // fixture's scope/implements/deviates directives ride along.
    assert!(enriched.markers.iter().any(|m| m.tag == "scope"));
    assert!(enriched.markers.iter().any(|m| m.tag == "deviates"));
    // Facts carry the file's census with deviation testimony intact.
    assert!(enriched.facts.iter().any(|f| matches!(
        f,
        go_ai_native_extract_bridge::RawFact::GoUnsafe { reason: Some(_), .. }
    )));
}
