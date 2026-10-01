//! The XML-source loader tests — split from `tests.rs` at the S4 landing
//! (the file crossed the 600-line budget).

use super::*;

/// An `.xml` spec with named sections and facts scans exactly as its
/// canonical MD twin (PROP-045 ##PROJECTION-READ).
#[test]
fn an_xml_spec_counts_the_same_units_as_its_md_projection() {
    let xml_root = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(
        xml_root
            .path()
            .join(vibe_core::layout::current_specs_root()),
    )
    .expect("mkdir");
    std::fs::write(xml_root.path().join(spec_rel("doc.xml")), XML_SPEC).expect("write xml");
    std::fs::write(xml_root.path().join("progress.toml"), xml_fixture_config()).expect("write cfg");

    // The hand-pinned MD twin is also the canonical projection itself.
    let md_root = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(md_root.path().join(vibe_core::layout::current_specs_root()))
        .expect("mkdir");
    let (projection, kind) =
        vibe_specdoc::load_spec_text(&xml_root.path().join(spec_rel("doc.xml"))).expect("project");
    assert_eq!(kind, vibe_specdoc::SourceKind::XmlProjected);
    assert_eq!(projection, MD_SPEC);
    std::fs::write(md_root.path().join(spec_rel("doc.md")), MD_SPEC).expect("write md");
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
    assert!(xml.xml_sources.contains(&spec_rel("doc.xml")));
    assert!(md.xml_sources.is_empty());
}

#[test]
fn named_xml_facts_are_addressable_and_stable_across_two_scans() {
    let root = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(root.path().join(vibe_core::layout::current_specs_root()))
        .expect("mkdir");
    std::fs::write(root.path().join(spec_rel("doc.xml")), XML_SPEC).expect("write xml");
    std::fs::write(root.path().join("progress.toml"), xml_fixture_config()).expect("write cfg");
    let ctx = Context::from_flags(true, false, None, false, crate::cli::AgentModeArg::Auto);

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
        projection_header(&spec_rel("doc.xml")),
        format!(
            "{}: {}",
            spec_rel("doc.xml"),
            vibe_specdoc::PROJECTION_NOTICE
        )
    );
}

/// One logical document in two forms is a split brain — the run stops
/// before any parse, naming both files (PROP-045 ##TARGET-MIXED).
#[test]
fn a_document_in_both_forms_is_a_loud_collision() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    std::fs::create_dir_all(root.join(vibe_core::layout::current_specs_root())).expect("mkdir");
    std::fs::write(root.join(spec_rel("one.md")), "# One {#one}\n").expect("write md");
    std::fs::write(
        root.join(spec_rel("one.xml")),
        "<spec xmlns=\"https://vibevm.org/spec/1\"/>",
    )
    .expect("write xml");
    std::fs::write(root.join("progress.toml"), xml_fixture_config()).expect("write cfg");
    let err = match ground(&args(root, false)) {
        Ok(_) => panic!("collision must stop the run"),
        Err(e) => e,
    };
    let text = format!("{err:#}");
    assert!(text.contains(&spec_rel("one.md")), "{text}");
    assert!(text.contains(&spec_rel("one.xml")), "{text}");
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
/// [`scope::DEFAULT_INCLUDES`] ships). The globs sit under the live
/// specs root, so the R4 layout flip carries them.
fn xml_fixture_config() -> String {
    let prefix = specs_prefix();
    format!(
        "include = [\"{prefix}/**/*.md\", \"{prefix}/**/*.xml\"]\n\n[progress]\ncache_dir = \"{FIXTURE_CACHE_DIR}\"\n"
    )
}

fn write_source(root: &std::path::Path, relative: &std::path::Path, body: impl AsRef<[u8]>) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

#[test]
fn opted_in_host_scan_and_mirror_ignore_raw_same_stem_resources() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let specs = vibe_core::layout::current_specs_root();
    let rule = specs.join("common/rule.xml");
    let raw_md = specs.join("common/rule.md");
    let raw_xml = specs.join("common/data.xml");
    let slash = |path: &std::path::Path| path.to_string_lossy().replace('\\', "/");
    std::fs::write(root.join("vibe.toml"), format!(
        "[project]\ngroup='org.host'\nname='demo'\nversion='1.1.0'\n\
         [[project.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n\
         [[project.sources]]\nfile='{}'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='documentation'\nloading='scan'\n\
         [[project.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='explicit'\n",
        slash(&rule), slash(&raw_md), slash(&raw_xml),
    )).unwrap();
    write_source(root, &rule, b"<spec xmlns=\"https://vibevm.org/spec/1\"><p><REAL fact=\"true\" status=\"impl/done\">selected fact</REAL></p></spec>\n");
    let raw_md_bytes = b"# Ordinary\n\n@fact:FALSE literal. @status:impl/done\n";
    let raw_xml_bytes = b"<data>not Vibe XML</data>\n";
    write_source(root, &raw_md, raw_md_bytes);
    write_source(root, &raw_xml, raw_xml_bytes);
    let campaign = root.join("campaigns/selected/run");
    std::fs::create_dir_all(&campaign).unwrap();
    let mut common = args(root, false);
    common.campaign = Some(root.join("campaigns/selected"));
    let observed = ground(&common).unwrap();
    assert_eq!(observed.docs.len(), 1);
    assert_eq!(observed.docs[0].fact_count, 1);
    assert!(observed.selected_sources.contains_key(&slash(&rule)));
    let ctx = Context::from_flags(true, false, None, false, crate::cli::AgentModeArg::Auto);
    scan(&ctx, &common).unwrap();
    mirror(&ctx, &common).unwrap();
    let cache = std::fs::read_to_string(campaign.join("cache.json")).unwrap();
    assert!(cache.contains("common/rule.xml"), "{cache}");
    assert!(!cache.contains("FALSE") && !cache.contains("data.xml"));
    assert_eq!(
        std::fs::read_dir(campaign.join("mirror")).unwrap().count(),
        1
    );
    assert_eq!(std::fs::read(root.join(raw_md)).unwrap(), raw_md_bytes);
    assert_eq!(std::fs::read(root.join(raw_xml)).unwrap(), raw_xml_bytes);
    let (descriptor, digest) = observed.selected_sources[&slash(&rule)].clone();
    write_source(root, &rule, b"changed XML\n");
    let stale = crate::commands::progress::grounding::read_selected(
        root,
        &rule,
        &descriptor,
        Some(&digest),
    )
    .unwrap_err()
    .to_string();
    assert!(stale.contains("changed after grounding"), "{stale}");
}

#[test]
fn in_tree_descriptor_package_excludes_raw_resources_before_pair_collision() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::write(
        root.join("vibe.toml"),
        "[project]\ngroup='org.host'\nname='demo'\nversion='1.1.0'\n",
    )
    .unwrap();
    let package = root.join("vibevm/vibepacks/org.example/flow/v1.1.0");
    let specs = vibe_core::layout::current_specs_root();
    let rule = specs.join("common/rule.xml");
    let raw_md = specs.join("common/rule.md");
    let raw_xml = specs.join("common/data.xml");
    let slash = |path: &std::path::Path| path.to_string_lossy().replace('\\', "/");
    write_source(
        &package,
        std::path::Path::new("vibe.toml"),
        format!(
            "[package]\ngroup='org.example'\nname='flow'\nkind='flow'\nversion='1.1.0'\n\
         [[package.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n\
         [[package.sources]]\nfile='{}'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='documentation'\nloading='scan'\n\
         [[package.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='explicit'\n",
            slash(&rule),
            slash(&raw_md),
            slash(&raw_xml),
        )
        .into_bytes(),
    );
    write_source(&package, &rule, b"<spec xmlns=\"https://vibevm.org/spec/1\"><p><REAL fact=\"true\" status=\"spec/done\">package fact</REAL></p></spec>\n");
    write_source(&package, &raw_md, b"# Raw\n\n@fact:FALSE literal.\n");
    write_source(&package, &raw_xml, b"<data>raw</data>\n");
    let run = root.join("campaigns/in-tree/run");
    std::fs::create_dir_all(&run).unwrap();
    let mut common = args(root, false);
    common.campaign = Some(root.join("campaigns/in-tree"));
    let observed = ground(&common).unwrap();
    assert_eq!(observed.docs.len(), 1);
    assert_eq!(observed.docs[0].fact_count, 1);
    assert!(observed.docs[0].path.ends_with("common/rule.xml"));
    let ctx = Context::from_flags(true, false, None, false, crate::cli::AgentModeArg::Auto);
    scan(&ctx, &common).unwrap();
    let cache = std::fs::read_to_string(run.join("cache.json")).unwrap();
    assert!(cache.contains("common/rule.xml"), "{cache}");
    assert!(!cache.contains("common/rule.md") && !cache.contains("common/data.xml"));
    assert_eq!(
        std::fs::read(package.join(raw_md)).unwrap(),
        b"# Raw\n\n@fact:FALSE literal.\n"
    );
    assert_eq!(
        std::fs::read(package.join(raw_xml)).unwrap(),
        b"<data>raw</data>\n"
    );
}
