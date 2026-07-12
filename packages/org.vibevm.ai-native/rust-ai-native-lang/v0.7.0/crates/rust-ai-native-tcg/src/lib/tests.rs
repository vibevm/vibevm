//! Enrichment-layer tests: crate/module derivation mirrors the
//! engine, findings flag against the baseline, advice cites the
//! GUIDE, completions finalise per policy. rust-analyzer-free.

specmark::scope!(
    "spec://org.vibevm.ai-native.rust-ai-native-lang/mechanisms/TCG-PROTOCOL-RUST-v0.1#enrichment"
);

use std::path::Path;

use rust_ai_native_tcg_bridge::ValidateOutcome;
use rust_ai_native_tcg_bridge::oracle::Completion;

use super::{
    Policy, derive_crate_module, detect_newtypes, enrich_validate, finalise_completions,
    parse_position, seam_file_for, validate_exit_code,
};

const UNWRAPPY: &str = r#"pub fn shout(input: Option<&str>) -> String {
    input.unwrap().to_uppercase()
}
"#;

/// A scratch project whose conform.toml gates `app` — enough policy
/// for the rules to fire.
fn scratch_policy() -> (tempfile::TempDir, Policy) {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("crates/app/src")).expect("dirs");
    std::fs::write(dir.path().join("crates/app/Cargo.toml"), "[package]\n").expect("manifest");
    std::fs::write(
        dir.path().join("conform.toml"),
        "roots = [\"crates/*\"]\nmax_file_lines = 600\n\
         gated_crates = [\"app\"]\ngated_pub_doctest = []\n\
         audit_crates = []\nenv_roots = []\n",
    )
    .expect("conform.toml");
    let policy = Policy::load(dir.path()).expect("policy");
    (dir, policy)
}

fn clean_outcome() -> ValidateOutcome {
    ValidateOutcome {
        diagnostics: Vec::new(),
        degraded: false,
    }
}

#[test]
fn derivation_mirrors_the_engine_scanner() {
    let roots = vec!["crates/*".to_string(), "xtask".to_string()];
    assert_eq!(
        derive_crate_module(&roots, "crates/rust-demo/src/cells/greeting.rs"),
        (
            "rust-demo".to_string(),
            "rust_demo::cells::greeting".to_string()
        )
    );
    assert_eq!(
        derive_crate_module(&roots, "crates/rust-demo/src/lib.rs"),
        ("rust-demo".to_string(), "rust_demo".to_string())
    );
    assert_eq!(
        derive_crate_module(&roots, "xtask/src/main.rs"),
        ("xtask".to_string(), "xtask".to_string())
    );
    assert_eq!(
        derive_crate_module(&roots, "crates/app/src/a/mod.rs"),
        ("app".to_string(), "app::a".to_string())
    );
}

#[test]
fn a_domain_unwrap_surfaces_unbaselined_with_advice() {
    let (_dir, policy) = scratch_policy();
    let enriched = enrich_validate(
        &policy,
        "crates/app/src/shout.rs",
        UNWRAPPY,
        clean_outcome(),
    );
    let finding = enriched
        .conform_findings
        .iter()
        .find(|f| f.rule == "no-unwrap-in-domain")
        .expect("the gate's own rule fires in-process");
    assert!(!finding.baselined);
    assert!(
        enriched
            .advice
            .iter()
            .any(|a| a.contains("GUIDE-AI-NATIVE-RUST §6")),
        "advice cites the ban: {:?}",
        enriched.advice
    );
    assert_eq!(validate_exit_code(&enriched), 1, "a new finding exits 1");
    assert!(
        enriched.markers.is_empty(),
        "markers reserved-empty in v0.1"
    );
}

#[test]
fn the_frozen_baseline_downgrades_to_baselined() {
    let (dir, _stale) = scratch_policy();
    // Freeze exactly the fingerprint the finding will carry: compute
    // it through the SAME engine path the relay uses.
    {
        let policy = Policy::load(dir.path()).expect("policy");
        let enriched = enrich_validate(
            &policy,
            "crates/app/src/shout.rs",
            UNWRAPPY,
            clean_outcome(),
        );
        assert!(enriched.conform_findings.iter().any(|f| !f.baselined));
    }
    // Reproduce the fingerprint via conform-core directly and write it
    // into the baseline file.
    use conform_core::Frontend;
    let frontend = rust_ai_native_conform_frontend::RustFrontend;
    let facts = frontend.extract("crates/app/src/shout.rs", "app", "app::shout", UNWRAPPY);
    let sf = conform_core::SourceFacts {
        file: "crates/app/src/shout.rs".to_string(),
        crate_name: "app".to_string(),
        facts,
    };
    let config = rust_ai_native_conform::load_config(dir.path()).expect("config");
    let owned = rust_ai_native_conform::build_rules(&config);
    let refs: Vec<&dyn conform_core::Rule> = owned.iter().map(|r| r.as_ref()).collect();
    let findings = conform_core::check(&refs, &[sf], None);
    let fp = &findings
        .iter()
        .find(|f| f.rule == "no-unwrap-in-domain")
        .expect("finding")
        .fingerprint;
    std::fs::write(
        dir.path().join(super::DEFAULT_CONFORM_BASELINE),
        serde_json::json!({"schema": 1, "findings": [fp]}).to_string(),
    )
    .expect("baseline");

    let policy = Policy::load(dir.path()).expect("policy reload");
    let enriched = enrich_validate(
        &policy,
        "crates/app/src/shout.rs",
        UNWRAPPY,
        clean_outcome(),
    );
    let finding = enriched
        .conform_findings
        .iter()
        .find(|f| f.rule == "no-unwrap-in-domain")
        .expect("still reported");
    assert!(finding.baselined, "the frozen ratchet downgrades it");
    assert_eq!(
        validate_exit_code(&enriched),
        0,
        "baselined findings do not fail the one-shot"
    );
    assert!(
        enriched.advice.is_empty(),
        "advice speaks only for NEW findings: {:?}",
        enriched.advice
    );
}

#[test]
fn exit_contract_covers_error_diagnostics() {
    let (_dir, policy) = scratch_policy();
    let outcome = ValidateOutcome {
        diagnostics: vec![rust_ai_native_tcg_bridge::Diagnostic {
            code: "E0308".to_string(),
            category: "error".to_string(),
            message: "mismatched types".to_string(),
            line: 2,
            character: 4,
        }],
        degraded: false,
    };
    let enriched = enrich_validate(
        &policy,
        "crates/app/src/clean.rs",
        "pub fn ok() {}\n",
        outcome,
    );
    assert_eq!(validate_exit_code(&enriched), 1);
}

#[test]
fn completions_finalise_prefix_max_and_the_ban() {
    let entries = vec![
        Completion {
            name: "unwrap".to_string(),
            kind: Some(2),
            type_text: Some("fn(self) -> T".to_string()),
        },
        Completion {
            name: "unwrap_or".to_string(),
            kind: Some(2),
            type_text: None,
        },
        Completion {
            name: "map".to_string(),
            kind: Some(2),
            type_text: None,
        },
    ];
    let out = finalise_completions(
        entries.clone(),
        "crates/app/src/domain.rs",
        Some("unwrap"),
        1,
    );
    assert_eq!(out.len(), 1, "prefix then max cut");
    assert_eq!(out[0]["unsafe"], true);
    assert!(
        out[0]["reason"].as_str().unwrap_or_default().contains("§6"),
        "the flag carries its reason"
    );
    // Test files are outside the ban.
    let test_out =
        finalise_completions(entries, "crates/app/src/domain/tests.rs", Some("unwrap"), 5);
    assert_eq!(test_out[0]["unsafe"], false);
}

#[test]
fn seam_resolution_walks_sibling_then_lib() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    std::fs::create_dir_all(root.join("crates/app/src/cells")).expect("dirs");
    std::fs::write(root.join("crates/app/src/cells.rs"), "pub mod x;\n").expect("cells.rs");
    std::fs::write(root.join("crates/app/src/lib.rs"), "pub mod cells;\n").expect("lib.rs");
    assert_eq!(
        seam_file_for(root, "crates/app/src/cells/x.rs"),
        "crates/app/src/cells.rs"
    );
    std::fs::remove_file(root.join("crates/app/src/cells.rs")).expect("rm");
    assert_eq!(
        seam_file_for(root, "crates/app/src/cells/x.rs"),
        "crates/app/src/lib.rs",
        "falls back up to the crate root"
    );
}

#[test]
fn newtype_detection_is_the_private_inner_shape_only() {
    let text = r#"
pub struct GuestName(String);
pub struct Open(pub u32);
struct Hidden(String);
pub struct Pair(String, String);
pub struct Named { inner: String }
"#;
    let hits = detect_newtypes(text, "src/x.rs");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].name, "GuestName");
    assert!(
        detect_newtypes("fn broken(", "src/x.rs").is_empty(),
        "rubble tolerated"
    );
}

#[test]
fn positions_parse_the_outer_convention() {
    let p = parse_position("7:12").expect("parses");
    assert_eq!((p.line, p.character), (7, 12));
    assert!(parse_position("7").is_err());
    assert!(parse_position("a:b").is_err());
}

#[test]
fn derivation_survives_windows_separators() {
    let roots = vec!["crates/*".to_string()];
    let (name, module) = derive_crate_module(&roots, "crates\\app\\src\\cells\\greeting.rs");
    assert_eq!(name, "app");
    assert_eq!(module, "app::cells::greeting");
    let _ = Path::new("."); // keep the Path import honest under cfg(test)
}
