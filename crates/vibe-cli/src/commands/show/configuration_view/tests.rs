//! Synthetic configuration view fixtures. Glyphs asserted here: ○ └.

use super::*;

/// A boolean description and resolution, with no source reads or evaluation.
fn bool_result() -> OrdinaryConfigurationPreviewResult {
    serde_json::from_value(serde_json::json!({
        "protocol": "vibe-ordinary-configuration-preview/1",
        "package": "org.x/pkg", "version": "1.2.0",
        "source": {"content_hash": "sha256:abc", "registry": "corp"},
        "description": {
            "choices": [{"id": "storage", "name": "Storage", "members": [
                {"id": "sqlite", "name": "SQLite"},
                {"id": "postgres", "name": "Postgres"}
            ]}],
            "parameters": [{"id": "tls", "name": "TLS", "required": true, "type": "bool"}]
        },
        "questions": [{"id": "storage", "kind": "choice", "members": ["sqlite", "postgres"]}],
        "resolved": {"features": ["postgres"], "parameters": {"tls": true}}
    })).unwrap()
}

/// An integer description with a synthetic effective value.
fn integer_result() -> OrdinaryIntegerPreviewResult {
    serde_json::from_value(serde_json::json!({
        "protocol": "vibe-ordinary-integer-preview/2",
        "package": "org.x/pkg", "version": "1.2.0",
        "source": {"registry": "corp", "content_hash": "sha256:abc"},
        "project_witness": "w",
        "definition": {
            "parameter": "workers", "name": "Workers", "min": 1, "max": 8,
            "profile": "small", "profile_name": "Small", "preset": 2,
            "group": "size", "group_name": "Size", "members": ["small", "large"]
        },
        "effective": {"workers": 4}
    })).unwrap()
}

/// Retained values and parameters share measured child-key columns.
#[test]
fn configuration_view_data_lists_values_as_children() {
    assert_eq!(data_view(&[], 80, 80), "○ no retained configuration data in this project");
    let rows = [DataRow {
        package: "org.x/pkg".into(),
        version: "1.2.0".into(),
        values: vec![("host".into(), "db".into()), ("port".into(), "8080".into())],
        parameters: vec![("tls".into(), "true".into())],
    }];
    assert_eq!(data_view(&rows, 80, 80), format!(
        "{}\n  org.x/pkg@1.2.0\n  └ host  db\n  └ port  8080\n  └ tls   true  [parameter]",
        rule_at("configuration data", None, 80),
    ));
}

/// Boolean descriptions stay rows both with a resolution and awaiting answers.
#[test]
fn configuration_view_preview_draws_rows_not_json() {
    let mut result = bool_result();
    let head = rule_at("configuration preview of org.x/pkg@1.2.0", Some("registry corp"), 80);
    let description = format!(
        "{head}\n  choice     storage  sqlite, postgres · Storage\n  parameter  tls      bool · TLS  [required]\n  content sha256:abc"
    );
    let resolved = preview_view(&result, 80, 80);
    assert_eq!(resolved, format!(
        "{description}\n\n{}\n  feature    postgres\n  parameter  tls       true",
        rule_at("resolved", None, 80),
    ));
    result.resolved = None;
    let unanswered = preview_view(&result, 80, 80);
    assert_eq!(unanswered, format!("{description}\n{}", cut(
        "  answers needed: 1; pass --input <file>, or --interactive in a terminal", 80,
    )));
    for text in [&resolved, &unanswered] {
        assert!(!text.lines().any(|line| line.starts_with('{')), "{text}");
    }
}

/// Integer descriptions and effective values are measured human rows.
#[test]
fn configuration_view_integer_draws_rows_not_json() {
    let text = integer_view(&integer_result(), 80, 80);
    assert_eq!(text, format!(
        "{}\n  parameter  workers  1..8 · Workers · preset 2\n  profile    small · Small\n  group      size     small, large · Size\n  content sha256:abc\n\n{}\n  workers  4",
        rule_at("configuration preview of org.x/pkg@1.2.0", Some("registry corp"), 80),
        rule_at("effective", None, 80),
    ));
    assert!(!text.lines().any(|line| line.starts_with('{')), "{text}");
}
