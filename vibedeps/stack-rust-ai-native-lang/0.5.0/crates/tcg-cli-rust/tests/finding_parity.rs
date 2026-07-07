//! The finding-parity gate (TCG-PROTOCOL-RUST §3): the relay's
//! per-file enrichment must judge EXACTLY as the gate's own
//! whole-tree scan judges — same crate attribution, same findings,
//! same fingerprints. Drift between the relay-local derivation and
//! the engine's scanner becomes a red test here, not a silent lie.

use conform_core::{ExtractionLog, Store};
use conform_frontend_rust::RustFrontend;
use tcg_cli_rust::{Policy, enrich_validate};
use tcg_oracle_bridge_rust::ValidateOutcome;

const VIOLATING: &str = r#"pub fn shout(input: Option<&str>) -> String {
    input.unwrap().to_uppercase()
}

pub fn env_peek() -> Option<String> {
    std::env::var("HOME").ok()
}
"#;

#[test]
fn relay_enrichment_matches_the_gate_scan_fingerprint_for_fingerprint() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    std::fs::create_dir_all(root.join("crates/app/src")).expect("dirs");
    std::fs::write(root.join("crates/app/Cargo.toml"), "[package]\n").expect("manifest");
    std::fs::write(root.join("crates/app/src/shout.rs"), VIOLATING).expect("source");
    std::fs::write(
        root.join("conform.toml"),
        "roots = [\"crates/*\"]\nmax_file_lines = 600\n\
         gated_crates = [\"app\"]\ngated_pub_doctest = []\n\
         audit_crates = []\nenv_roots = []\n",
    )
    .expect("conform.toml");

    // The gate's path: Store scan over the tree, exactly as run_check.
    let config = conform_cli_rust::load_config(root).expect("config");
    let store = Store::at_repo(root, &config);
    let mut log = ExtractionLog::default();
    let facts = store
        .extract_workspace(root, &RustFrontend, &mut log)
        .expect("extract");
    let owned = conform_cli_rust::build_rules(&config);
    let refs: Vec<&dyn conform_core::Rule> = owned.iter().map(|r| r.as_ref()).collect();
    let gate_findings = conform_core::check(&refs, &facts, None);
    assert!(
        !gate_findings.is_empty(),
        "the fixture must violate something or the parity claim is vacuous"
    );

    // The relay's path: per-file enrichment over the same text.
    let policy = Policy::load(root).expect("policy");
    let enriched = enrich_validate(
        &policy,
        "crates/app/src/shout.rs",
        VIOLATING,
        ValidateOutcome {
            diagnostics: Vec::new(),
            degraded: false,
        },
    );

    let mut gate: Vec<(String, u32, String)> = gate_findings
        .iter()
        .filter(|f| f.file == "crates/app/src/shout.rs")
        .map(|f| (f.rule.to_string(), f.line, f.message.clone()))
        .collect();
    let mut relay: Vec<(String, u32, String)> = enriched
        .conform_findings
        .iter()
        .map(|f| (f.rule.clone(), f.line, f.message.clone()))
        .collect();
    gate.sort();
    relay.sort();
    assert_eq!(
        gate, relay,
        "one engine, one truth — the relay and the gate must agree verbatim"
    );
    assert!(
        gate.iter()
            .any(|(rule, _, _)| rule == "no-unwrap-in-domain"),
        "the unwrap fixture fires the §6 rule in both"
    );
}
