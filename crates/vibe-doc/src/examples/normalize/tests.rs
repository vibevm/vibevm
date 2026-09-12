//! The normalisation rules, one law per test.

use super::{Normalizer, Placeholders, replace_path_spellings};
use crate::examples::fixture::{NormalizeDecl, ReplaceDecl};

fn places() -> Placeholders {
    Placeholders {
        sandbox: Some(r"C:\tmp\vdocs\p1".into()),
        repo: Some(r"C:\src\vibevm".into()),
        home: Some(r"C:\Users\someone".into()),
    }
}

fn norm(decl: NormalizeDecl) -> Normalizer {
    Normalizer::compile("t", &decl, places()).expect("the declaration compiles")
}

#[test]
fn the_sandbox_is_replaced_before_the_home_that_contains_it() {
    let out = norm(NormalizeDecl::default()).apply(r"C:\Users\someone\x and C:\tmp\vdocs\p1\work");
    assert_eq!(out, "<HOME>/x and <TMP>/work");
}

#[test]
fn a_path_is_found_in_all_three_spellings_it_reaches_a_stream_in() {
    let raw = "{\"a\":\"C:\\\\tmp\\\\vdocs\\\\p1\\\\w\",\"b\":\"C:/tmp/vdocs/p1/w\"}";
    let mut decl = NormalizeDecl {
        slashes: false,
        ..NormalizeDecl::default()
    };
    decl.product_version = false;
    let out = norm(decl).apply(raw);
    assert_eq!(out, "{\"a\":\"<TMP>\\\\w\",\"b\":\"<TMP>/w\"}");
}

#[test]
fn the_path_replacement_runs_before_slashes_are_unified() {
    // Were the order reversed, the native spelling would already be gone
    // and the replacement would find nothing.
    let out = norm(NormalizeDecl::default()).apply(r"C:\tmp\vdocs\p1\home\settings\cache");
    assert_eq!(out, "<TMP>/home/settings/cache");
}

#[test]
fn the_product_version_is_replaced_and_a_package_version_is_not() {
    let out = norm(NormalizeDecl::default()).apply("vibe 1.0.0\n  org.vibevm.world/wal@1.0.0");
    assert_eq!(out, "vibe <VERSION>\n  org.vibevm.world/wal@1.0.0");
}

#[test]
fn the_executable_name_becomes_what_the_reader_types() {
    let out = norm(NormalizeDecl::default()).apply("Usage: vibe.exe [OPTIONS] <COMMAND>");
    assert_eq!(out, "Usage: vibe [OPTIONS] <COMMAND>");
}

#[test]
fn trailing_space_and_trailing_blank_lines_go() {
    let out = norm(NormalizeDecl::default()).apply("one   \ntwo\t\n\n\n");
    assert_eq!(out, "one\ntwo");
}

#[test]
fn ansi_and_the_osc_icon_sequence_are_stripped() {
    let out = norm(NormalizeDecl::default()).apply("\u{1b}[1mbold\u{1b}[0m\u{1b}]7;name\u{7}tail");
    assert_eq!(out, "boldtail");
}

#[test]
fn a_declared_replacement_closes_exactly_what_it_names() {
    let decl = NormalizeDecl {
        replace: vec![ReplaceDecl {
            pattern: r"\b[0-9A-HJKMNP-TV-Z]{26}\b".into(),
            with: "<RUN-ID>".into(),
        }],
        ..NormalizeDecl::default()
    };
    let out = norm(decl).apply("run 01KXBEHEYJCQ1RNJ5657Q31HVA done");
    assert_eq!(out, "run <RUN-ID> done");
}

#[test]
fn a_sorted_block_orders_only_the_run_of_lines_that_match_its_form() {
    let decl = NormalizeDecl {
        sort_blocks: vec![r"^  org\.".into()],
        ..NormalizeDecl::default()
    };
    let out = norm(decl).apply("head\n  org.b/x\n  org.a/y\ntail\n  org.d/z\n  org.c/w");
    assert_eq!(
        out,
        "head\n  org.a/y\n  org.b/x\ntail\n  org.c/w\n  org.d/z"
    );
}

#[test]
fn an_unusable_pattern_is_refused_with_the_fixture_that_wrote_it() {
    let decl = NormalizeDecl {
        sort_blocks: vec!["([".into()],
        ..NormalizeDecl::default()
    };
    let e = Normalizer::compile("hello-vibe", &decl, places()).expect_err("refused");
    let text = e.to_string();
    assert!(text.contains("hello-vibe"), "{text}");
    assert!(text.contains("PROP-057#PIPE-EXAMPLE-RUNNER"), "{text}");
}

#[test]
fn a_path_is_matched_case_insensitively_because_windows_returns_both() {
    let out = replace_path_spellings(r"c:\TMP\vdocs\P1\w", r"C:\tmp\vdocs\p1", "<TMP>");
    assert_eq!(out, r"<TMP>\w");
}
