//! Enrichment-layer unit tests — extractor-free where possible (the
//! pure helpers), extractor-backed where the value is the join (the
//! finding-parity test lives in tests/finding_parity.rs and runs the
//! REAL extractor).

use super::*;

#[test]
fn cell_of_prefers_the_cells_dir_and_falls_back_to_the_package_dir() {
    let mut config = Config::default();
    config.go.cells_dir = Some("internal/cells".into());
    assert_eq!(cell_of(&config, "internal/cells/plan/plan.go"), "plan");
    assert_eq!(cell_of(&config, "internal/sim/world.go"), "internal/sim");
    assert_eq!(cell_of(&Config::default(), "cmd/app/main.go"), "cmd/app");
}

#[test]
fn seam_file_is_the_policy_seams_pkg_or_the_own_dir() {
    let mut config = Config::default();
    config.go.seams_pkg = Some("internal/seams".into());
    assert_eq!(
        seam_file_for(&config, "internal/cells/plan/plan.go"),
        "internal/seams"
    );
    assert_eq!(
        seam_file_for(&Config::default(), "internal/cells/plan/plan.go"),
        "internal/cells/plan"
    );
}

#[test]
fn completions_flag_ambient_defaults_only_inside_cells() {
    let mut config = Config::default();
    config.go.cells_dir = Some("internal/cells".into());
    let entries = vec![
        Completion {
            name: "Getenv".into(),
            kind: Some(3),
            type_text: Some("func(key string) string".into()),
        },
        Completion {
            name: "Getwd".into(),
            kind: Some(3),
            type_text: None,
        },
    ];
    let in_cell = finalise_completions(
        &config,
        entries.clone(),
        "internal/cells/plan/plan.go",
        None,
        50,
    );
    assert_eq!(in_cell[0]["unsafe"], true);
    assert_eq!(in_cell[1]["unsafe"], false);
    let outside = finalise_completions(&config, entries, "cmd/app/main.go", None, 50);
    assert_eq!(outside[0]["unsafe"], false);
}

#[test]
fn completions_cut_by_prefix_and_max_before_detail() {
    let entries: Vec<Completion> = ["Alpha", "Alps", "Beta"]
        .iter()
        .map(|n| Completion {
            name: (*n).to_string(),
            kind: None,
            type_text: None,
        })
        .collect();
    let out = finalise_completions(&Config::default(), entries, "a.go", Some("Al"), 1);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0]["name"], "Alpha");
}

#[test]
fn brands_are_exported_defined_types_over_primitives() {
    let record = go_ai_native_extract_bridge::FileRecord {
        protocol: 1,
        file: "internal/seams/ids.go".into(),
        in_test: false,
        degraded: false,
        facts: vec![
            RawFact::Item {
                kind: "type".into(),
                symbol: "AccountID".into(),
                line: 3,
                is_exported: true,
                has_doc_example: false,
                underlying: Some("string".into()),
            },
            RawFact::Item {
                kind: "type".into(),
                symbol: "world".into(),
                line: 9,
                is_exported: false,
                has_doc_example: false,
                underlying: Some("int".into()),
            },
            RawFact::Item {
                kind: "type".into(),
                symbol: "Store".into(),
                line: 12,
                is_exported: true,
                has_doc_example: false,
                underlying: None,
            },
        ],
        markers: vec![],
    };
    let brands = brands_of(&record);
    assert_eq!(brands.len(), 1);
    assert_eq!(brands[0].name, "AccountID");
    assert!(brands[0].heuristic);
}

#[test]
fn positions_parse_and_refuse_garbage() {
    let p = parse_position("12:4").expect("parses");
    assert_eq!((p.line, p.character), (12, 4));
    assert!(parse_position("nope").is_err());
}

#[test]
fn exit_code_fires_on_errors_and_new_findings_only() {
    let clean = EnrichedValidate {
        diagnostics: vec![],
        facts: vec![],
        markers: vec![],
        conform_findings: vec![WireFinding {
            rule: "go-unsafe-in-domain".into(),
            message: "m".into(),
            line: 1,
            baselined: true,
        }],
        advice: vec![],
        degraded: false,
    };
    assert_eq!(validate_exit_code(&clean), 0);
    let dirty = EnrichedValidate {
        conform_findings: vec![WireFinding {
            rule: "go-unsafe-in-domain".into(),
            message: "m".into(),
            line: 1,
            baselined: false,
        }],
        ..clean
    };
    assert_eq!(validate_exit_code(&dirty), 1);
}
