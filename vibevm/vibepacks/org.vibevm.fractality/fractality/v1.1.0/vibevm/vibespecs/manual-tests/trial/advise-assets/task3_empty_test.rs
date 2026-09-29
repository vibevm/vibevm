//! Acceptance test for advise task 3 (empty value round-trips). The scorer
//! copies this file into a run's `proj-final/tests/`; `cargo test --test
//! task3_empty_test` must pass on a correct parse->render path.
//!
//! The contract: a key with an EMPTY value must survive a parse->render round
//! trip as exactly `k=` — not dropped, not rendered as a bare `k`. Task 3
//! fixes whichever of parse or render mishandles an empty value; this test
//! holds the line (and catches a "looks-done" refactor that regresses it).
//!
//! This test exercises ONLY the real public API — `mini_logfmt::parse_line`
//! and `mini_logfmt::render` over the real `Rec` — with their actual names
//! and signatures. No new function is involved.

use mini_logfmt::{parse_line, render};

#[test]
fn empty_value_round_trips_as_k_equals() {
    // The boundary: a pair whose value is empty.
    let rec = parse_line("k=").expect("k= parses to one empty-valued pair");
    assert_eq!(rec.get("k"), Some(""), "the empty value is retained");
    // parse -> render must yield exactly the input form back.
    assert_eq!(render(&rec), "k=", "empty value renders back as k=, not dropped");
}

#[test]
fn empty_value_survives_among_other_pairs() {
    // An empty-valued pair must not vanish when it sits next to others.
    let rec = parse_line("a=1 k= b=2").expect("line with an empty-valued pair parses");
    assert_eq!(rec.get("k"), Some(""));
    let back = render(&rec);
    assert!(
        back.contains("k="),
        "the empty-valued pair is still rendered: got {back:?}"
    );
}
