//! Golden tests for [`fractality_backend_claude_code::stream`] against
//! the frozen `hello-glm-stream.jsonl` transcript, plus synthetic
//! tolerance and result-overwrite cases pinning the D14 contract and the
//! authoritativeness of the `result` event.

use std::collections::BTreeMap;

use fractality_backend_claude_code::stream::StreamParser;

/// Feeds every non-blank line of a transcript to a fresh parser.
/// Blank lines are skipped so a trailing newline can never masquerade as
/// a malformed line.
fn parse_all(text: &str) -> StreamParser {
    let mut p = StreamParser::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        p.feed_line(line);
    }
    p
}

/// The frozen real-world transcript parses cleanly: zero malformed lines,
/// every line counted as an event, and the `result` event's token totals
/// win over the assistant events' under-reported zeros.
#[test]
fn golden_fixture_parses_cleanly() {
    let fixture = include_str!("fixtures/hello-glm-stream.jsonl");
    let s = parse_all(fixture).finish();

    assert_eq!(s.malformed_lines, 0);
    assert_eq!(s.totals.events, 60);

    let mut expected = BTreeMap::new();
    expected.insert("assistant".to_owned(), 4u64);
    expected.insert("result".to_owned(), 1);
    expected.insert("system/init".to_owned(), 1);
    expected.insert("system/thinking_tokens".to_owned(), 52);
    expected.insert("user".to_owned(), 2);
    assert_eq!(s.event_counts, expected);

    assert_eq!(s.model.as_deref(), Some("glm-5-turbo"));
    assert_eq!(s.num_turns, Some(3));
    assert!(!s.is_error);

    // The result event's authoritative numbers, not the assistant zeros.
    assert_eq!(s.totals.input_tokens, 17145);
    assert_eq!(s.totals.output_tokens, 236);
    assert_eq!(s.totals.cache_creation_input_tokens, 0);
    assert_eq!(s.totals.cache_read_input_tokens, 23168);
    assert!(
        (s.totals.total_cost_usd - 0.103209).abs() < 1e-6,
        "cost was {}",
        s.totals.total_cost_usd
    );
    assert!(
        s.final_text
            .as_deref()
            .is_some_and(|t| t.starts_with("Both files created:")),
        "final_text was {:?}",
        s.final_text
    );
}

/// D14 tolerance: garbage, an unknown future event kind, and an untyped
/// object are all absorbed without panic — only the garbage is malformed.
#[test]
fn tolerance_counts_malformed_and_unknown_without_failing() {
    let mut p = StreamParser::new();
    p.feed_line("not json at all");
    p.feed_line(r#"{"type":"weird_future_kind"}"#);
    p.feed_line(r#"{"no_type":1}"#);
    let s = p.finish();

    assert_eq!(s.malformed_lines, 1);
    assert_eq!(s.totals.events, 2);
    assert_eq!(s.event_counts.get("weird_future_kind"), Some(&1));
    assert_eq!(s.event_counts.get("untyped"), Some(&1));
    assert_eq!(s.totals.input_tokens, 0);
    assert_eq!(s.totals.output_tokens, 0);
    assert_eq!(s.totals.cache_creation_input_tokens, 0);
    assert_eq!(s.totals.cache_read_input_tokens, 0);
    assert!(!s.is_error);
}

/// The `result` event overwrites the assistant's accumulated usage: its
/// token fields and cost replace the running totals.
#[test]
fn result_event_overwrites_assistant_usage() {
    let mut p = StreamParser::new();
    p.feed_line(
        r#"{"type":"assistant","message":{"model":"m","usage":{"input_tokens":5,"output_tokens":5}}}"#,
    );
    p.feed_line(
        r#"{"type":"result","usage":{"input_tokens":100,"output_tokens":50},"total_cost_usd":1.5}"#,
    );
    let s = p.finish();

    assert_eq!(s.totals.input_tokens, 100);
    assert_eq!(s.totals.output_tokens, 50);
    assert_eq!(s.totals.total_cost_usd, 1.5);
    assert_eq!(s.totals.events, 2);
}
