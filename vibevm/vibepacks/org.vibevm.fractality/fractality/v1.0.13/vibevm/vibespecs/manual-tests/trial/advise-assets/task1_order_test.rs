//! Acceptance test for advise task 1 (first-seen-order dedup). The scorer
//! copies this file into a run's `proj-final/tests/`; `cargo test --test
//! task1_order_test` must pass on a correct implementation.
//!
//! The contract pinned here (this is the API the menu-advise.md task 1 asks
//! the worker to add, made exact):
//!
//!     pub fn dedup_keys<'a>(keys: &'a [&'a str]) -> Vec<&'a str>
//!
//! — given a slice of keys with duplicates, return the DISTINCT keys in
//! FIRST-SEEN order (later duplicates dropped). The named lifetime is forced
//! by the borrow checker (`&[&str]` carries two elided lifetimes); the
//! borrowed-in / borrowed-out shape is deliberate, so a `HashSet<&str>`
//! collect also type-checks against it and a reordering HashSet impl COMPILES
//! then FAILS THIS TEST on the order assertion — discrimination is on order,
//! not on a type mismatch. The worker re-exports `dedup_keys` from the crate
//! root, matching how the existing crate surfaces `count_keys`.
//!
//! This test uses no symbols beyond the real public API plus the one new
//! function (`mini_logfmt::dedup_keys`); `Rec`, `parse_line`, and `render`
//! keep their real names and signatures.

use mini_logfmt::dedup_keys;

#[test]
fn distinct_keys_keep_first_seen_order() {
    // b is seen first, then a, then c. A HashSet collect reorders these
    // arbitrarily (RandomState hashing); first-seen order is exactly
    // [b, a, c].
    let out = dedup_keys(&["b", "a", "b", "c", "a"]);
    assert_eq!(out, vec!["b", "a", "c"], "later duplicates drop, order holds");
}

#[test]
fn a_hashset_collect_would_reorder_this() {
    // Enough spread that hashed iteration order almost surely differs from
    // first-seen order — the obvious HashSet impl trips here.
    let out = dedup_keys(&["z", "a", "m", "z", "a", "q", "m", "b"]);
    assert_eq!(out, vec!["z", "a", "m", "q", "b"]);
}

#[test]
fn empty_input_yields_empty_output() {
    let out: Vec<&str> = dedup_keys(&[]);
    assert!(out.is_empty());
}

#[test]
fn no_duplicates_pass_through_untouched() {
    let out = dedup_keys(&["x", "y", "z"]);
    assert_eq!(out, vec!["x", "y", "z"]);
}
