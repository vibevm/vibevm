//! Acceptance test for advise task 2 (record count at the boundary). The
//! scorer copies this file into a run's `proj-final/tests/`; `cargo test
//! --test task2_count_test` must pass on a correct implementation.
//!
//! The contract pinned here (the API the menu-advise.md task 2 asks the
//! worker to add, made exact):
//!
//!     pub fn count_records(input: &str) -> usize
//!
//! — count the NON-EMPTY lines (records) in a logfmt input string. A TRAILING
//! NEWLINE is not a record (`"a=1\nb=2\n"` has two records, not three), and a
//! blank line is not a record either. The naive `input.split('\n').count()`
//! returns 3 for the trailing-newline case and fails the first assertion; the
//! bare `input.lines().count()` returns 3 for the blank-line case and fails
//! the second. The worker re-exports `count_records` from the crate root,
//! matching how the existing crate surfaces `count_keys`.
//!
//! This test uses no symbols beyond the real public API plus the one new
//! function (`mini_logfmt::count_records`).

use mini_logfmt::count_records;

#[test]
fn trailing_newline_is_not_a_phantom_record() {
    // The boundary: split('\n').count() counts the empty segment after the
    // final newline and returns 3. Correct is 2.
    assert_eq!(count_records("a=1\nb=2\n"), 2);
}

#[test]
fn no_trailing_newline_still_counts_each_record() {
    assert_eq!(count_records("a=1\nb=2"), 2);
}

#[test]
fn blank_lines_are_not_records() {
    // A record is a NON-EMPTY line; blank lines must not inflate the count.
    assert_eq!(count_records("a=1\n\nb=2\n"), 2);
}

#[test]
fn empty_input_is_zero_records() {
    assert_eq!(count_records(""), 0);
}
