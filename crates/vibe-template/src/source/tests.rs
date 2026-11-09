//! The pure inputs: the version document, the digest documents, and the
//! `[template]` table read out of a template that a typed `Manifest`
//! cannot hold.

use vibe_core::manifest::{Scalar, TemplateSection};

use super::{DigestTable, VersionRecord, template_section, version_records};

fn section(versions: &str, digests: &str) -> TemplateSection {
    toml::from_str(&format!("[versions]\n{versions}\n[digests]\n{digests}\n")).expect("section")
}

fn json_section(extra: &str) -> TemplateSection {
    section(
        &format!(
            "kind = \"json\"\nurl = \"https://n.invalid/index.json\"\nselect = \"\"\n\
             strip_prefix = \"v\"\n{extra}"
        ),
        "kind = \"sha256sums\"\nurl = \"https://n.invalid/{version}/SHASUMS256.txt\"",
    )
}

#[test]
fn a_record_whose_version_is_not_semver_is_skipped_with_a_reason() {
    let (records, skipped) = version_records(
        &json_section(""),
        Some(r#"[{"version":"v3.1.4"},{"version":"nightly"},{"nope":1}]"#),
    )
    .expect("the document parses");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].raw, "v3.1.4");
    assert_eq!(records[0].version.to_string(), "3.1.4");
    assert_eq!(skipped.len(), 2, "{skipped:?}");
    assert!(skipped[0].contains("nightly"), "{skipped:?}");
    assert!(skipped[1].contains("version"), "{skipped:?}");
}

#[test]
fn a_source_that_yields_nothing_is_an_error_naming_what_it_skipped() {
    let error = version_records(&json_section(""), Some(r#"[{"version":"nightly"}]"#))
        .expect_err("refuses");
    assert!(error.message.contains("no usable version"), "{error}");
    assert!(error.message.contains("nightly"), "{error}");
}

#[test]
fn the_bounds_and_the_prerelease_policy_filter_before_anything_else() {
    let document = r#"[{"version":"v4.0.0"},{"version":"v3.0.0"},{"version":"v2.0.0"},
                       {"version":"v3.5.0-rc.1"}]"#;
    let only_three = version_records(
        &json_section("min = \"3.0.0\"\nmax = \"3.9.9\""),
        Some(document),
    )
    .expect("parses")
    .0;
    assert_eq!(
        only_three
            .iter()
            .map(|record| record.version.to_string())
            .collect::<Vec<_>>(),
        vec!["3.0.0".to_string()],
        "the prerelease is excluded by default"
    );
    let with_pre = version_records(
        &json_section("min = \"3.0.0\"\nmax = \"3.9.9\"\nprerelease = \"include\""),
        Some(document),
    )
    .expect("parses")
    .0;
    assert_eq!(with_pre.len(), 2);
}

#[test]
fn a_selector_pointing_at_something_other_than_an_array_says_what_it_found() {
    let pointed = section(
        "kind = \"json\"\nurl = \"https://n.invalid/i.json\"\nselect = \"/releases/stable\"",
        "kind = \"sha256sums\"\nurl = \"https://n.invalid/{version}\"",
    );
    let error =
        version_records(&pointed, Some(r#"{"releases":{"stable":"4.0.0"}}"#)).expect_err("refuses");
    assert!(error.message.contains("a string"), "{error}");
    let error = version_records(&pointed, Some("{}")).expect_err("refuses");
    assert!(error.message.contains("nothing"), "{error}");
}

#[test]
fn a_records_scalars_become_meta_and_its_composites_do_not() {
    let (records, _) = version_records(
        &json_section(""),
        Some(
            r#"[{"version":"v1.2.3","lts":"Jod","security":true,"modules":127,
                 "files":["a"],"nested":{"deep":"yes"},"ratio":1.5,"nothing":null}]"#,
        ),
    )
    .expect("parses");
    let record = &records[0];
    assert_eq!(record.meta.get("lts"), Some(&Scalar::String("Jod".into())));
    assert_eq!(record.meta.get("security"), Some(&Scalar::Bool(true)));
    assert_eq!(record.meta.get("modules"), Some(&Scalar::Integer(127)));
    assert_eq!(
        record.meta.get("nested.deep"),
        Some(&Scalar::String("yes".into()))
    );
    for composite in ["files", "nested", "ratio", "nothing"] {
        assert!(!record.meta.contains_key(composite), "{composite}");
        assert!(record.is_composite(composite), "{composite}");
    }
}

#[test]
fn a_list_source_carries_its_own_meta_and_needs_no_document() {
    let section: TemplateSection = toml::from_str(
        "[versions]\nkind = \"list\"\n\n[[versions.item]]\nversion = \"1.0.0\"\n\
         meta = { channel = \"old\" }\n\n[[versions.item]]\nversion = \"2.0.0\"\n\
         meta = { channel = \"new\" }\n\n[digests]\nkind = \"sha256sums\"\n\
         url = \"https://e.invalid/{version}\"\n",
    )
    .expect("section");
    let (records, skipped) = version_records(&section, None).expect("needs no document");
    assert!(skipped.is_empty());
    assert_eq!(records.len(), 2);
    assert_eq!(
        records[1].meta.get("channel"),
        Some(&Scalar::String("new".into()))
    );
}

#[test]
fn a_json_version_source_without_its_document_refuses() {
    let error = version_records(&json_section(""), None).expect_err("refuses");
    assert!(error.message.contains("fetched document"), "{error}");
}

#[test]
fn the_digest_document_skips_what_it_does_not_need_and_refuses_a_duplicate() {
    let table = DigestTable::parse_sha256sums(&format!(
        "# a header line\n{}  node-v1.tar.gz\n{}  *node-v2.tar.gz\nshort  node-v3.tar.gz\n",
        "a".repeat(64),
        "b".repeat(64)
    ))
    .expect("parses");
    assert_eq!(
        table
            .get("node-v1.tar.gz")
            .map(|entry| entry.sha256.clone()),
        Some("a".repeat(64))
    );
    assert!(
        table.get("node-v2.tar.gz").is_some(),
        "the `*` marker is stripped"
    );
    assert!(
        table.get("node-v3.tar.gz").is_none(),
        "a short digest is not a pair"
    );
    let error = DigestTable::parse_sha256sums(&format!(
        "{}  dup\n{}  dup\n",
        "a".repeat(64),
        "b".repeat(64)
    ))
    .expect_err("refuses");
    assert!(error.message.contains("twice"), "{error}");
}

#[test]
fn a_json_digest_document_reads_digests_and_sizes() {
    let section = section(
        "kind = \"list\"\n\n[[versions.item]]\nversion = \"1.0.0\"",
        "kind = \"json\"\nurl = \"https://e.invalid/{version}.json\"\nselect = \"/files\"\n\
         file_field = \"name\"\ndigest_field = \"sha256\"\nsize_field = \"bytes\"",
    );
    let table = DigestTable::parse(
        &section,
        &format!(
            r#"{{"files":[{{"name":"t.tar.gz","sha256":"{}","bytes":"42"}},
                          {{"name":"u.zip","sha256":"{}","bytes":7}}]}}"#,
            "e".repeat(64),
            "f".repeat(64)
        ),
    )
    .expect("parses");
    assert_eq!(table.get("t.tar.gz").and_then(|entry| entry.size), Some(42));
    assert_eq!(table.get("u.zip").and_then(|entry| entry.size), Some(7));
}

#[test]
fn the_template_table_reads_out_of_a_document_a_typed_manifest_cannot_hold() {
    // `size` holds a string where the typed manifest wants an integer, so
    // the document is not a typed `Manifest` — and the declaration is read
    // all the same (PROP-075 `#TEMPLATE-DOCUMENT-READING`).
    let text = "[package]\nname = \"t\"\ngroup = \"org.e\"\nkind = \"app\"\nversion = \"1.0.0\"\n\
                template = true\n\n[application]\nid = \"t\"\n\n\
                [template.versions]\nkind = \"list\"\n\n\
                [[template.versions.item]]\nversion = \"1.0.0\"\n\n\
                [template.digests]\nkind = \"json\"\nurl = \"https://e.invalid/{version}.json\"\n\
                select = \"\"\nfile_field = \"n\"\ndigest_field = \"d\"\nsize_field = \"b\"\n\n\
                [[installer_options]]\nid = \"l\"\nname = \"t\"\nplatform = \"linux\"\n\
                platform_name = \"Linux\"\narch = \"x86_64\"\n\n\
                [[installer_options.steps]]\nid = \"u\"\ntitle = \"u\"\nkind = \"archive\"\n\
                format = \"tar_gz\"\nsource = \"https://e.invalid/t.tar.gz\"\n\
                sha256 = \"{sha256:t.tar.gz}\"\nsize = \"{size:t.tar.gz}\"\n";
    assert!(
        vibe_core::manifest::Manifest::parse_str(text).is_err(),
        "a numeric coercion site is not typed-parseable"
    );
    let section = template_section(text).expect("the declaration reads all the same");
    assert!(section.has_sizes());
}

#[test]
fn a_document_with_no_template_table_says_so() {
    let error = template_section("[package]\nname = \"t\"\n").expect_err("refuses");
    assert!(error.message.contains("no [template] table"), "{error}");
    let error = template_section("[template]\nversions = 1\n").expect_err("refuses");
    assert!(error.message.contains("unreadable"), "{error}");
}

#[test]
fn a_plain_record_is_its_own_raw_spelling() {
    let record = VersionRecord::plain("1.2.3-rc.1").expect("semver");
    assert_eq!(record.raw, "1.2.3-rc.1");
    assert_eq!(record.version.pre.as_str(), "rc.1");
    assert!(VersionRecord::plain("nope").is_err());
}
