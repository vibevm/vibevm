//! Acceptance test for task E5 (`src/dedup.rs`). Move this file to
//! `tests/dedup_api_test.rs` once the module exists; `cargo test`
//! must pass with it in place.

use mini_logfmt::dedup::{dedup_batch, DedupOutcome};
use mini_logfmt::parse_line;

#[test]
fn exact_duplicate_lines_collapse_and_order_survives() {
    let lines = [
        "level=info msg=a",
        "level=info msg=b",
        "level=info msg=a",
        "level=warn msg=c",
    ];
    let recs: Vec<_> = lines.iter().map(|l| parse_line(l).unwrap()).collect();
    let out: DedupOutcome = dedup_batch(&recs);
    assert_eq!(out.unique.len(), 3, "one duplicate collapses");
    assert_eq!(out.dropped, 1, "the drop is counted");
    assert_eq!(out.unique[0].get("msg"), Some("a"));
    assert_eq!(out.unique[1].get("msg"), Some("b"));
    assert_eq!(out.unique[2].get("msg"), Some("c"), "first-seen order");
}

#[test]
fn distinct_records_pass_through_untouched() {
    let recs: Vec<_> = ["a=1", "a=2", "a=3"]
        .iter()
        .map(|l| parse_line(l).unwrap())
        .collect();
    let out = dedup_batch(&recs);
    assert_eq!(out.unique.len(), 3);
    assert_eq!(out.dropped, 0);
}

#[test]
fn an_empty_batch_is_an_empty_outcome() {
    let out = dedup_batch(&[]);
    assert!(out.unique.is_empty());
    assert_eq!(out.dropped, 0);
}
