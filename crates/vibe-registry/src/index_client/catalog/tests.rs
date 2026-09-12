//! The JSON Lines envelope, read without a server.
//!
//! The transport half (a 404 is «absent», a 500 is an error) is exercised
//! by the crate's mock-server integration tests, which already stand up a
//! raw-file root; what cannot be reached that way is the envelope itself,
//! and it is the half a malformed catalog breaks.

use super::*;

const URL: &str = "https://example.invalid/index/primary.jsonl";

fn line(name: &str, version: &str, hash: &str) -> String {
    format!(
        r#"{{"schema_version":1,"kind":"doc","group":"org.example","name":"{name}","version":"{version}","content_hash":"{hash}","source_url":"https://example.invalid/org.example.{name}","source_ref":"v{version}","registry":"example","files_count":3,"indexed_at":"2026-09-12T00:00:00Z","indexed_by":"vibe-index"}}"#
    )
}

#[test]
fn every_record_of_the_catalog_is_read() {
    let body = format!(
        "{}\n{}\n",
        line("a", "1.0.0", "sha256:aa"),
        line("b", "2.0.0", "sha256:bb")
    );
    let entries = parse_primary(URL, body.as_bytes()).expect("a catalog");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].name, "a");
    assert_eq!(entries[0].content_hash, "sha256:aa");
    assert_eq!(entries[1].version.to_string(), "2.0.0");
}

/// A catalog is written with a trailing newline, and an index that
/// separates its batches with a blank line is still a catalog.
#[test]
fn blank_lines_are_not_records() {
    let body = format!("\n{}\n\n", line("a", "1.0.0", "sha256:aa"));
    assert_eq!(parse_primary(URL, body.as_bytes()).unwrap().len(), 1);
}

#[test]
fn an_empty_catalog_is_an_empty_answer_and_not_a_failure() {
    assert!(parse_primary(URL, b"").unwrap().is_empty());
    assert!(parse_primary(URL, b"\n\n").unwrap().is_empty());
}

/// «Somewhere in this file» is not a diagnosis for a catalog of
/// thousands of records, so the refusal carries the line number.
#[test]
fn a_malformed_line_names_its_own_number() {
    let body = format!("{}\n{{\"kind\":\n", line("a", "1.0.0", "sha256:aa"));
    let message = parse_primary(URL, body.as_bytes())
        .expect_err("a refusal")
        .to_string();
    assert!(message.contains("line 2"), "{message}");
    assert!(message.contains(URL), "{message}");
}

#[test]
fn a_catalog_that_is_not_text_is_refused_as_such() {
    let message = parse_primary(URL, &[0xff, 0xfe, 0x00])
        .expect_err("a refusal")
        .to_string();
    assert!(message.contains("not valid UTF-8"), "{message}");
}

/// The shape is the generated record, so the fields the documentation
/// site reads arrive without this module knowing they exist.
#[test]
fn the_relation_fields_arrive_through_the_generated_record() {
    let body = format!(
        r#"{{"schema_version":1,"kind":"doc","group":"org.example","name":"docs","version":"0.1.0","content_hash":"sha256:cc","source_url":"u","source_ref":"r","registry":"example","files_count":1,"indexed_at":"2026-09-12T00:00:00Z","indexed_by":"vibe-index","title":"The manual","documents":[{{"package":"org.example/subject","version":"^1.0"}}]}}{}"#,
        "\n"
    );
    let entries = parse_primary(URL, body.as_bytes()).expect("a catalog");
    assert_eq!(entries[0].title.as_deref(), Some("The manual"));
    assert_eq!(entries[0].documents.len(), 1);
    assert_eq!(entries[0].documents[0].package, "org.example/subject");
}
