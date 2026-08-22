//! The XML-source loader tests — split from `tests.rs` at the S4 landing
//! (the file crossed the 600-line budget).

use super::*;

/// An `.xml` spec with named sections and facts scans exactly as its
/// canonical MD twin (PROP-045 ##PROJECTION-READ).
#[test]
fn an_xml_spec_counts_the_same_units_as_its_md_projection() {
    let xml_root = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(xml_root.path().join("spec")).expect("mkdir");
    std::fs::write(xml_root.path().join("spec/doc.xml"), XML_SPEC).expect("write xml");
    std::fs::write(xml_root.path().join("progress.toml"), xml_fixture_config()).expect("write cfg");

    // The hand-pinned MD twin is also the canonical projection itself.
    let md_root = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(md_root.path().join("spec")).expect("mkdir");
    let (projection, kind) =
        vibe_specdoc::load_spec_text(&xml_root.path().join("spec/doc.xml")).expect("project");
    assert_eq!(kind, vibe_specdoc::SourceKind::XmlProjected);
    assert_eq!(projection, MD_SPEC);
    std::fs::write(md_root.path().join("spec/doc.md"), MD_SPEC).expect("write md");
    std::fs::write(md_root.path().join("progress.toml"), xml_fixture_config()).expect("write cfg");

    let xml = ground(&args(xml_root.path(), false)).expect("ground xml");
    let md = ground(&args(md_root.path(), false)).expect("ground md");
    assert_eq!(xml.docs.len(), 1);
    assert_eq!(md.docs.len(), 1);
    // Full parser parity: units, facts, statuses, hashes and source spans;
    // only the source path differs.
    let mut xml_doc = xml.docs[0].clone();
    xml_doc.path = md.docs[0].path.clone();
    assert_eq!(xml_doc, md.docs[0]);
    assert_eq!(xml.docs[0].fact_count, 4);
    assert_eq!(xml.docs[0].markers.len(), 3);
    // And the source is marked: its diagnostics are projection-relative.
    assert_eq!(xml.xml_sources.len(), 1);
    assert!(xml.xml_sources.contains("spec/doc.xml"));
    assert!(md.xml_sources.is_empty());
}

#[test]
fn named_xml_facts_are_addressable_and_stable_across_two_scans() {
    let root = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(root.path().join("spec")).expect("mkdir");
    std::fs::write(root.path().join("spec/doc.xml"), XML_SPEC).expect("write xml");
    std::fs::write(root.path().join("progress.toml"), xml_fixture_config()).expect("write cfg");
    let ctx = Context::from_flags(true, false, None, false);

    scan(&ctx, &args(root.path(), true)).expect("first scan");
    let first = ground(&args(root.path(), false)).expect("first ground");
    scan(&ctx, &args(root.path(), true)).expect("second scan");
    let second = ground(&args(root.path(), false)).expect("second ground");

    let addressed = |doc: &ParsedDoc| {
        doc.blocks
            .iter()
            .flat_map(|block| block.facts.iter())
            .filter_map(|fact| {
                fact.id
                    .as_ref()
                    .map(|id| (id.clone(), fact.content_hash.clone()))
            })
            .collect::<BTreeMap<_, _>>()
    };
    let first_facts = addressed(&first.docs[0]);
    let second_facts = addressed(&second.docs[0]);
    assert_eq!(first_facts, second_facts, "content identity is scan-stable");
    assert_eq!(
        first_facts.keys().map(String::as_str).collect::<Vec<_>>(),
        ["CODE", "ONLY", "TWO"]
    );
    assert!(first_facts.values().all(|hash| !hash.is_empty()));

    check(
        &ctx,
        &ProgressCheckArgs {
            common: args(root.path(), false),
            exhaustive: false,
            write_state: false,
        },
    )
    .expect("named XML source passes progress check");
}

#[test]
fn scan_report_marks_xml_source_with_projection_header() {
    assert_eq!(
        projection_header("spec/doc.xml"),
        format!("spec/doc.xml: {}", vibe_specdoc::PROJECTION_NOTICE)
    );
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

/// Named facts plus a named section, list and typed fence.
const XML_SPEC: &str = "<spec xmlns=\"https://vibevm.org/spec/1\">\n  \
     <title id=\"doc\">Doc</title>\n  \
     <p><ONLY fact=\"true\" status=\"impl/done\">one claim</ONLY></p>\n  \
     <laws title=\"Laws\">\n    \
       <list ordered=\"false\"><item><TWO fact=\"true\" status=\"spec/done\">two</TWO></item><item>plain</item></list>\n    \
       <p><CODE fact=\"true\" status=\"spec/done\">typed fact</CODE></p>\n    \
       <fence lang=\"rust\" fact=\"CODE\">fn main() {}</fence>\n  \
     </laws>\n</spec>";

const MD_SPEC: &str = "# Doc {#doc}\n\n\
@fact:ONLY one claim @status:impl/done\n\n\
## Laws {#laws}\n\n\
- @fact:TWO two @status:spec/done\n\
- plain\n\n\
@fact/code:CODE typed fact @status:spec/done\n\n\
```rust\nfn main() {}\n```\n\n";

/// The `progress.toml` observing both serialisations (the glob crate has no
/// brace alternation, so the pair is spelled out — the same pairing
/// [`scope::DEFAULT_INCLUDES`] ships).
fn xml_fixture_config() -> String {
    format!(
        "include = [\"spec/**/*.md\", \"spec/**/*.xml\"]\n\n[progress]\ncache_dir = \"{FIXTURE_CACHE_DIR}\"\n"
    )
}
