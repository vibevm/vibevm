//! The XML-source loader tests — split from `tests.rs` at the S4 landing
//! (the file crossed the 600-line budget).

use super::*;

/// An `.xml` spec scans the same units its MD projection would: the
/// projection feeds the parser, so facts, blocks and markers are
/// projection-equal by construction (PROP-045 ##PROJECTION-READ).
#[test]
fn an_xml_spec_counts_the_same_units_as_its_md_projection() {
    let xml_root = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(xml_root.path().join("spec")).expect("mkdir");
    std::fs::write(xml_root.path().join("spec/doc.xml"), XML_SPEC).expect("write xml");
    std::fs::write(xml_root.path().join("progress.toml"), xml_fixture_config()).expect("write cfg");

    // The MD twin: the canonical projection itself, on disk.
    let md_root = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(md_root.path().join("spec")).expect("mkdir");
    let (projection, kind) =
        vibe_specdoc::load_spec_text(&xml_root.path().join("spec/doc.xml")).expect("project");
    assert_eq!(kind, vibe_specdoc::SourceKind::XmlProjected);
    std::fs::write(md_root.path().join("spec/doc.md"), &projection).expect("write md");
    std::fs::write(md_root.path().join("progress.toml"), xml_fixture_config()).expect("write cfg");

    let xml = ground(&args(xml_root.path(), false)).expect("ground xml");
    let md = ground(&args(md_root.path(), false)).expect("ground md");
    assert_eq!(xml.docs.len(), 1);
    assert_eq!(md.docs.len(), 1);
    // Same fact/marker/block population — the doc differs only in `path`.
    assert_eq!(xml.docs[0].fact_count, md.docs[0].fact_count);
    assert_eq!(xml.docs[0].markers.len(), md.docs[0].markers.len());
    assert_eq!(xml.docs[0].blocks.len(), md.docs[0].blocks.len());
    assert!(xml.docs[0].fact_count >= 3, "the fixture carries facts");
    // And the source is marked: its diagnostics are projection-relative.
    assert_eq!(xml.xml_sources.len(), 1);
    assert!(xml.xml_sources.contains("spec/doc.xml"));
    assert!(md.xml_sources.is_empty());
}

/// One logical document in two forms is a split brain — the run stops
/// before any parse, naming both files (PROP-045 ##TARGET-MIXED).
#[test]
fn a_document_in_both_forms_is_a_loud_collision() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    std::fs::create_dir_all(root.join("spec")).expect("mkdir");
    std::fs::write(root.join("spec/one.md"), "# One {#one}\n").expect("write md");
    std::fs::write(
        root.join("spec/one.xml"),
        "<spec xmlns=\"https://vibevm.org/spec/1\"/>",
    )
    .expect("write xml");
    std::fs::write(root.join("progress.toml"), xml_fixture_config()).expect("write cfg");
    let err = match ground(&args(root, false)) {
        Ok(_) => panic!("collision must stop the run"),
        Err(e) => e,
    };
    let text = format!("{err:#}");
    assert!(text.contains("spec/one.md"), "{text}");
    assert!(text.contains("spec/one.xml"), "{text}");
    assert!(text.contains("one document, one form"), "{text}");
}

#[test]
fn check_write_state_defaults_off_and_parses() {
    use crate::cli::{Cli, Command, ProgressArgs, ProgressSubcommand};
    use clap::Parser;

    fn check(argv: &[&str]) -> ProgressCheckArgs {
        let cli = Cli::try_parse_from(argv).expect("parse `vibe progress check`");
        let Command::Progress(ProgressArgs {
            command: ProgressSubcommand::Check(a),
        }) = cli.command
        else {
            panic!("argv did not parse to `progress check`: {argv:?}");
        };
        a
    }

    let off = check(&["vibe", "progress", "check"]);
    assert!(!off.write_state, "off by default — check is read-only");

    let on = check(&["vibe", "progress", "check", "--write-state"]);
    assert!(on.write_state, "the flag turns the write back on");
}

// ---- PROP-045 ##PROJECTION-READ: XML sources through the projection ----

/// A dialect document with a fact, a list and a fence — shape-heavy enough
/// that "the same units" means something.
const XML_SPEC: &str = "<spec xmlns=\"https://vibevm.org/spec/1\">\n  \
     <title id=\"doc\">Doc</title>\n  \
     <p><fact id=\"ONLY\" status=\"impl/done\">one claim</fact></p>\n  \
     <list ordered=\"false\"><item><fact id=\"TWO\" status=\"spec/done\">two</fact></item><item>plain</item></list>\n  \
     <p><fact id=\"CODE\" status=\"spec/done\">typed fact</fact></p>\n  \
     <fence lang=\"rust\" fact=\"CODE\">fn main() {}</fence>\n</spec>";

/// The `progress.toml` observing both serialisations (the glob crate has no
/// brace alternation, so the pair is spelled out — the same pairing
/// [`scope::DEFAULT_INCLUDES`] ships).
fn xml_fixture_config() -> String {
    format!(
        "include = [\"spec/**/*.md\", \"spec/**/*.xml\"]\n\n[progress]\ncache_dir = \"{FIXTURE_CACHE_DIR}\"\n"
    )
}
