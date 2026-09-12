//! The three forms of a `manifest-field` reference (X-030).

use std::fs;
use std::path::PathBuf;

use super::generate;

const COORDINATE: &str = "org.vibevm.core/vibevm-docs";

const MANIFEST_TEXT: &str = "[package]\n\
name = \"vibevm-docs\"\n\
group = \"org.vibevm.core\"\n\
kind = \"doc\"\n\
title = \"VibeVM Manual\"\n\
abstract = \"\"\"\n\
What it covers: everything.\n\
For whom: readers.\n\
\"\"\"\n\
keywords = [\"documentation\", \"manual\"]\n\
\n\
[[documents]]\n\
package = \"org.vibevm.core/vibevm\"\n";

fn package() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().to_path_buf();
    fs::write(dir.join("vibe.toml"), MANIFEST_TEXT).expect("write manifest");
    (tmp, dir)
}

#[test]
fn a_bare_field_reads_the_documenting_packages_own_manifest() {
    let (_tmp, dir) = package();
    let text = generate("title", &dir, COORDINATE).expect("reads");
    assert_eq!(text, "VibeVM Manual");
}

#[test]
fn a_field_in_the_package_table_is_reachable_with_or_without_the_table_name() {
    let (_tmp, dir) = package();
    assert_eq!(
        generate("package.title", &dir, COORDINATE).expect("dotted"),
        generate("title", &dir, COORDINATE).expect("bare")
    );
}

#[test]
fn a_coordinate_alone_is_the_whole_manifest() {
    let (_tmp, dir) = package();
    let text = generate(COORDINATE, &dir, COORDINATE).expect("reads");
    assert!(text.starts_with("[package]"), "{text}");
    assert!(text.contains("[[documents]]"), "{text}");
    assert_eq!(text, MANIFEST_TEXT.trim_end());
}

#[test]
fn a_coordinate_with_a_field_is_that_one_field() {
    let (_tmp, dir) = package();
    let reference = format!("{COORDINATE}#package.kind");
    assert_eq!(
        generate(&reference, &dir, COORDINATE).expect("reads"),
        "doc"
    );
}

#[test]
fn a_multi_line_scalar_keeps_its_lines_and_loses_its_trailing_blank() {
    let (_tmp, dir) = package();
    let text = generate("abstract", &dir, COORDINATE).expect("reads");
    assert_eq!(text, "What it covers: everything.\nFor whom: readers.");
}

#[test]
fn a_list_keeps_its_toml_form_because_that_is_the_only_honest_rendering() {
    let (_tmp, dir) = package();
    let text = generate("keywords", &dir, COORDINATE).expect("reads");
    assert!(text.contains("documentation"), "{text}");
    assert!(text.starts_with('['), "{text}");
}

#[test]
fn a_foreign_coordinate_is_refused_by_name_rather_than_guessed_at() {
    let (_tmp, dir) = package();
    let e = generate("org.acme/other#package.title", &dir, COORDINATE).expect_err("refused");
    assert!(e.to_string().contains("org.acme/other"), "{e}");
    assert!(e.to_string().contains("arrives through the store"), "{e}");
}

#[test]
fn a_field_the_manifest_does_not_carry_is_a_loud_refusal() {
    let (_tmp, dir) = package();
    let e = generate("package.nonesuch", &dir, COORDINATE).expect_err("refused");
    assert!(
        e.to_string().contains("carries no `package.nonesuch`"),
        "{e}"
    );
}
