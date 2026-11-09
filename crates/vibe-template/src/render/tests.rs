//! The render: typing, scoping, determinism and the refusals that keep a
//! wrong manifest from ever being produced.

use std::collections::BTreeMap;

use vibe_core::manifest::Scalar;

use crate::error::RenderErrorKind;
use crate::source::{DigestEntry, DigestTable, VersionRecord};

use super::{render, skeleton};

/// A template whose `[[installer_options]]` row carries the fields a test
/// wants to watch. `extra` is spliced into the step table.
fn template(package_extra: &str, step_extra: &str) -> String {
    format!(
        "[package]\nname = \"tool\"\ngroup = \"org.example\"\nkind = \"app\"\n\
         version = \"1.0.0\"\ntemplate = true\n{package_extra}\n\
         [application]\nid = \"tool\"\n\n\
         [template.versions]\nkind = \"list\"\n\n\
         [[template.versions.item]]\nversion = \"2.3.4\"\n\n\
         [template.digests]\nkind = \"json\"\n\
         url = \"https://e.invalid/{{version}}.json\"\nselect = \"/files\"\n\
         file_field = \"name\"\ndigest_field = \"sha256\"\nsize_field = \"bytes\"\n\n\
         [[installer_options]]\nid = \"linux-x64\"\nname = \"tool\"\n\
         platform = \"linux\"\nplatform_name = \"Linux\"\narch = \"x86_64\"\n\
         path_dirs = [\"bin\"]\n\n\
         [[installer_options.steps]]\nid = \"unpack\"\ntitle = \"Unpack\"\n\
         kind = \"archive\"\nformat = \"tar_gz\"\n\
         source = \"https://e.invalid/tool-{{version}}-{{platform}}-{{arch}}.tar.gz\"\n\
         sha256 = \"{{sha256:tool-{{version}}-{{platform}}-{{arch}}.tar.gz}}\"\n{step_extra}"
    )
}

fn record() -> VersionRecord {
    VersionRecord::plain("2.3.4").expect("semver")
}

fn record_with_meta(pairs: &[(&str, Scalar)], composite: &[&str]) -> VersionRecord {
    let mut record = record();
    record.meta = pairs
        .iter()
        .map(|(key, value)| ((*key).to_string(), value.clone()))
        .collect();
    let mut full = template_record_composite(composite);
    full.meta = record.meta.clone();
    full
}

fn template_record_composite(composite: &[&str]) -> VersionRecord {
    // `composite` is private, so a record carrying composite paths is
    // built through the JSON path the renderer itself uses.
    let document = format!(
        "[{{\"version\":\"2.3.4\"{}}}]",
        composite
            .iter()
            .map(|path| format!(",\"{path}\":[1,2]"))
            .collect::<String>()
    );
    let section: vibe_core::manifest::TemplateSection = toml::from_str(
        "[versions]\nkind = \"json\"\nurl = \"https://e.invalid/i.json\"\nselect = \"\"\n\
         [digests]\nkind = \"sha256sums\"\nurl = \"https://e.invalid/{version}\"\n",
    )
    .expect("section");
    crate::source::version_records(&section, Some(&document))
        .expect("records")
        .0
        .remove(0)
}

fn digests(size: Option<u64>) -> DigestTable {
    let mut entries = BTreeMap::new();
    entries.insert(
        "tool-2.3.4-linux-x86_64.tar.gz".to_string(),
        DigestEntry {
            sha256: "c".repeat(64),
            size,
        },
    );
    DigestTable::from_entries(entries)
}

#[test]
fn platform_and_arch_bind_to_the_rows_own_ids() {
    let rendered = render(&template("", ""), &record(), &digests(None)).expect("renders");
    assert!(
        rendered
            .toml
            .contains("source = \"https://e.invalid/tool-2.3.4-linux-x86_64.tar.gz\""),
        "{}",
        rendered.toml
    );
}

#[test]
fn platform_outside_an_installer_option_refuses_naming_the_field() {
    let error = render(
        &template("description = \"for {platform}\"", ""),
        &record(),
        &digests(None),
    )
    .expect_err("refuses");
    assert_eq!(error.kind, RenderErrorKind::Grammar);
    assert_eq!(error.at, "package.description");
    assert!(error.message.contains("nothing to bind it to"), "{error}");
}

#[test]
fn a_digest_variable_outside_a_digest_bearing_field_refuses() {
    let error = render(
        &template("", "payload_root = \"{sha256:tool-{version}.tar.gz}\"\n"),
        &record(),
        &digests(None),
    )
    .expect_err("refuses");
    assert_eq!(error.kind, RenderErrorKind::Grammar);
    assert_eq!(error.at, "installer_options[0].steps[0].payload_root");
    assert!(error.message.contains("digest-bearing fields"), "{error}");
    // Outside an installer option altogether, likewise.
    let error = render(
        &template("description = \"{sha256:x}\"", ""),
        &record(),
        &digests(None),
    )
    .expect_err("refuses");
    assert_eq!(error.at, "package.description");
}

#[test]
fn a_lone_variable_in_a_numeric_field_renders_a_number() {
    let rendered = render(
        &template(
            "",
            "size = \"{size:tool-{version}-{platform}-{arch}.tar.gz}\"\n",
        ),
        &record(),
        &digests(Some(1234)),
    )
    .expect("renders");
    assert!(rendered.toml.contains("size = 1234"), "{}", rendered.toml);
    assert!(
        !rendered.toml.contains("size = \"1234\""),
        "the whole TOML node is replaced, not the text inside the quotes"
    );
    // A variable mixed with other text always renders as a string.
    let rendered = render(
        &template("", "payload_root = \"root-{version}\"\n"),
        &record(),
        &digests(None),
    )
    .expect("renders");
    assert!(
        rendered.toml.contains("payload_root = \"root-2.3.4\""),
        "{}",
        rendered.toml
    );
}

#[test]
fn a_lone_meta_variable_adopts_the_records_own_type() {
    let record = record_with_meta(
        &[
            ("security", Scalar::Bool(true)),
            ("modules", Scalar::Integer(137)),
            ("codename", Scalar::String("Jod".into())),
        ],
        &[],
    );
    let rendered = render(
        &template(
            "description = \"{meta.codename}\"",
            "optional = \"{meta.security}\"\n",
        ),
        &record,
        &digests(None),
    )
    .expect("renders");
    assert!(
        rendered.toml.contains("description = \"Jod\""),
        "{}",
        rendered.toml
    );
    assert!(
        rendered.toml.contains("optional = true"),
        "{}",
        rendered.toml
    );
}

#[test]
fn a_meta_variable_resolving_to_an_array_refuses_naming_the_path() {
    let record = template_record_composite(&["files"]);
    let error = render(
        &template("description = \"{meta.files}\"", ""),
        &record,
        &digests(None),
    )
    .expect_err("refuses");
    assert_eq!(error.kind, RenderErrorKind::Grammar);
    assert_eq!(error.at, "package.description");
    assert!(
        error.message.contains("resolves to an array or a table"),
        "{error}"
    );
    // An absent path says so instead of guessing.
    let error = render(
        &template("description = \"{meta.absent}\"", ""),
        &record,
        &digests(None),
    )
    .expect_err("refuses");
    assert!(error.message.contains("names no field"), "{error}");
}

#[test]
fn a_size_variable_needs_a_digest_source_that_carries_sizes() {
    let sizeless = template("", "size = \"{size:tool-{version}.tar.gz}\"\n").replace(
        "kind = \"json\"\nurl = \"https://e.invalid/{version}.json\"\nselect = \"/files\"\n\
         file_field = \"name\"\ndigest_field = \"sha256\"\nsize_field = \"bytes\"",
        "kind = \"sha256sums\"\nurl = \"https://e.invalid/{version}/SHASUMS256.txt\"",
    );
    let error = render(&sizeless, &record(), &digests(None)).expect_err("refuses");
    assert_eq!(error.kind, RenderErrorKind::Grammar);
    assert!(error.message.contains("carries sizes"), "{error}");
}

#[test]
fn the_render_strips_every_trace_of_the_template_vocabulary() {
    let rendered = render(&template("", ""), &record(), &digests(None)).expect("renders");
    assert!(!rendered.toml.contains("template"), "{}", rendered.toml);
    assert!(rendered.toml.contains("version = \"2.3.4\""));
    assert!(!rendered.toml.contains("version = \"1.0.0\""));
}

#[test]
fn an_inline_array_of_installer_options_refuses_rather_than_rendering_half() {
    // `installer_options` is a root key, so the inline spelling has to sit
    // above every table header — which is exactly how an author would
    // reach for it.
    let inline = concat!(
        "installer_options = [{ id = \"linux-x64\", name = \"tool {version}\" }]\n\n",
        "[package]\nname = \"tool\"\ngroup = \"org.example\"\nkind = \"app\"\n",
        "version = \"1.0.0\"\ntemplate = true\n\n[application]\nid = \"tool\"\n\n",
        "[template.versions]\nkind = \"list\"\n\n",
        "[[template.versions.item]]\nversion = \"2.3.4\"\n\n",
        "[template.digests]\nkind = \"sha256sums\"\nurl = \"https://e.invalid/{version}\"\n",
    );
    let error = render(inline, &record(), &digests(None)).expect_err("refuses");
    assert_eq!(error.kind, RenderErrorKind::Template);
    assert!(error.message.contains("array of tables"), "{error}");
}

#[test]
fn a_document_that_is_not_a_template_refuses_before_anything_else() {
    let error = render(
        "[package]\nname = \"a\"\ngroup = \"org.e\"\nkind = \"app\"\nversion = \"1.0.0\"\n",
        &record(),
        &digests(None),
    )
    .expect_err("refuses");
    assert_eq!(error.kind, RenderErrorKind::Template);
    assert!(error.message.contains("no [template] table"), "{error}");
}

/// The determinism invariant over a spread of inputs: the same template,
/// record and digests always produce the same bytes, and a different input
/// always produces different bytes.
#[test]
fn rendering_is_deterministic_over_many_inputs() {
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    for major in 0..6u64 {
        for patch in 0..6u64 {
            let version = format!("{major}.1.{patch}");
            let record = VersionRecord::plain(&version).expect("semver");
            let mut entries = BTreeMap::new();
            entries.insert(
                format!("tool-{version}-linux-x86_64.tar.gz"),
                DigestEntry {
                    sha256: format!("{major:02x}{patch:02x}{}", "d".repeat(60)),
                    size: Some(major * 1000 + patch),
                },
            );
            let table = DigestTable::from_entries(entries);
            let text = template(
                "",
                "size = \"{size:tool-{version}-{platform}-{arch}.tar.gz}\"\n",
            )
            .replace("version = \"2.3.4\"", &format!("version = \"{version}\""));
            let first = render(&text, &record, &table).expect("renders");
            let again = render(&text, &record, &table).expect("renders");
            assert_eq!(first.toml, again.toml, "{version}");
            assert_eq!(first.sha256, again.sha256, "{version}");
            if let Some(other) = seen.insert(first.sha256.clone(), version.clone()) {
                panic!("{version} and {other} rendered identically");
            }
        }
    }
    assert_eq!(seen.len(), 36);
}

#[test]
fn a_skeleton_needs_no_digest_document_and_keeps_every_row() {
    let rendered = skeleton(
        &template(
            "",
            "size = \"{size:tool-{version}-{platform}-{arch}.tar.gz}\"\n",
        ),
        &record(),
    )
    .expect("renders");
    assert!(rendered.toml.contains(super::SKELETON_DIGEST));
    assert!(rendered.toml.contains("size = 0"));
    assert!(rendered.dropped.is_empty());
}
