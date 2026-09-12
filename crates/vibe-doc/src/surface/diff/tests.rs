//! Two surfaces with a known difference, and the pages it must name.

use std::path::Path;

use vibe_wire::generated::doc_surface::{
    DocSurface, SurfaceCommand, SurfaceFact, SurfaceFlag, SurfaceFormat, SurfaceSchema,
    SurfaceSchemaField,
};

use super::*;
use crate::surface::SCHEMA_VERSION;

const COORDINATE: &str = "org.acme/manual";

/// A package whose three pages each derive their text from a different
/// half of the product's surface.
fn package(dir: &Path) {
    std::fs::write(
        dir.join("vibe.toml"),
        "[package]\ngroup = \"org.acme\"\nname = \"manual\"\nkind = \"doc\"\n\
         version = \"0.1.0\"\n",
    )
    .expect("the manifest");
    let pages = dir.join("vibevm/vibespecs/reference");
    std::fs::create_dir_all(&pages).expect("the spec root");
    let page = |name: &str, block: &str| {
        std::fs::write(
            pages.join(name),
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                 <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
                   <title id=\"root\">Reference</title>\n  {block}\n\
                 </spec>\n"
            ),
        )
        .expect("a page");
    };
    page(
        "cli.xml",
        "<derived kind=\"cli-help\" ref=\"vibe doc build --help\"/>",
    );
    page(
        "manifest.xml",
        "<derived kind=\"manifest-field\" ref=\"org.acme/manual\"/>",
    );
    page(
        "formats.xml",
        "<derived kind=\"jtd-schema\" ref=\"doc-manifest\"/>",
    );
}

fn surface(version: &str) -> DocSurface {
    DocSurface {
        schema_version: SCHEMA_VERSION,
        version: version.to_owned(),
        commands: vec![SurfaceCommand {
            path: "vibe doc build".into(),
            summary: "Render a package".into(),
            flags: vec![SurfaceFlag {
                name: "--path".into(),
                value: "PATH".into(),
                summary: "The package".into(),
            }],
        }],
        manifest_fields: vec!["package".into(), "package.title".into()],
        lock_fields: vec!["package".into()],
        schemas: vec![SurfaceSchema {
            id: "doc-manifest".into(),
            path: "schemas/doc_manifest.jtd.json".into(),
            fields: vec![SurfaceSchemaField {
                form: String::new(),
                name: "pages".into(),
                required: true,
                shape: "elements of ref doc_page".into(),
            }],
        }],
        facts: vec![SurfaceFact {
            address: "vibevm/vibespecs/common/PROP-057.xml#CARD-FIELDS".into(),
            audiences: vec!["author".into()],
            text: "A doc package declares a title.".into(),
        }],
        formats: vec![SurfaceFormat {
            id: "doc-manifest".into(),
            epoch: 1,
            schema: "schemas/doc_manifest.jtd.json".into(),
            recoverable: true,
            foreign_parsers: "many".into(),
            corpus: "none".into(),
            sunset: "none".into(),
        }],
    }
}

/// The same surface, moved in four named places.
fn moved() -> DocSurface {
    let mut new = surface("2.0.0");
    new.commands[0].flags.push(SurfaceFlag {
        name: "--force".into(),
        value: String::new(),
        summary: "Do it anyway".into(),
    });
    new.manifest_fields.push("package.subtitle".into());
    new.schemas[0].fields.push(SurfaceSchemaField {
        form: String::new(),
        name: "sections".into(),
        required: false,
        shape: "elements of ref doc_section".into(),
    });
    new.facts.push(SurfaceFact {
        address: "vibevm/vibespecs/common/PROP-057.xml#CARD-MEDIA".into(),
        audiences: vec!["author".into()],
        text: "A card may carry pictures.".into(),
    });
    new
}

#[test]
fn a_known_difference_names_a_known_list_of_pages() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    package(tmp.path());
    let document = diff(
        &surface("1.0.0"),
        &moved(),
        tmp.path(),
        COORDINATE,
        &SpecSources::new(),
        tmp.path(),
    )
    .expect("a diff");

    assert_eq!(document.from, "1.0.0");
    assert_eq!(document.to, "2.0.0");
    let named: Vec<&str> = document.pages.iter().map(|p| p.page.as_str()).collect();
    assert_eq!(
        named,
        vec![
            "reference/cli.xml",
            "reference/formats.xml",
            "reference/manifest.xml"
        ],
        "each of the four changes must reach the page that derives its text from it"
    );

    let reasons = |page: &str| {
        document
            .pages
            .iter()
            .find(|p| p.page == page)
            .map(|p| p.reasons.join(" | "))
            .unwrap_or_default()
    };
    assert!(
        reasons("reference/cli.xml").contains("`vibe doc build --force` appeared"),
        "{}",
        reasons("reference/cli.xml")
    );
    assert!(
        reasons("reference/manifest.xml").contains("`package.subtitle` appeared"),
        "{}",
        reasons("reference/manifest.xml")
    );
    assert!(
        reasons("reference/formats.xml").contains("member `sections` appeared"),
        "{}",
        reasons("reference/formats.xml")
    );
}

#[test]
fn a_change_no_page_answers_to_asks_for_a_new_page() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    package(tmp.path());
    let document = diff(
        &surface("1.0.0"),
        &moved(),
        tmp.path(),
        COORDINATE,
        &SpecSources::new(),
        tmp.path(),
    )
    .expect("a diff");
    let unplaced: Vec<&str> = document
        .needs_pages
        .iter()
        .map(|u| u.subject.as_str())
        .collect();
    assert_eq!(
        unplaced,
        vec!["vibevm/vibespecs/common/PROP-057.xml#CARD-MEDIA"],
        "a promise nobody tells is not silence — it is a page that does not exist"
    );
    assert!(
        document.needs_pages[0]
            .reason
            .contains("no page answers to it yet")
    );
}

#[test]
fn two_readings_of_one_surface_produce_an_empty_answer_that_says_so() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    package(tmp.path());
    let document = diff(
        &surface("1.0.0"),
        &surface("1.0.0"),
        tmp.path(),
        COORDINATE,
        &SpecSources::new(),
        tmp.path(),
    )
    .expect("a diff");
    assert!(document.changes.is_empty());
    assert!(document.pages.is_empty());
    assert!(render_md(&document).contains("The two surfaces are the same"));
}

#[test]
fn every_half_of_the_surface_is_compared() {
    let mut new = moved();
    new.commands[0].summary = "Render the package".into();
    new.lock_fields.clear();
    new.formats[0].epoch = 2;
    new.facts[0].text = "A doc package declares a title and an abstract.".into();
    let changes = compare(&surface("1.0.0"), &new);
    let kinds: Vec<&ChangeKind> = changes.iter().map(|c| &c.kind).collect();
    for expected in [
        ChangeKind::Command,
        ChangeKind::Flag,
        ChangeKind::ManifestField,
        ChangeKind::LockField,
        ChangeKind::Schema,
        ChangeKind::Fact,
        ChangeKind::Format,
    ] {
        assert!(
            kinds.contains(&&expected),
            "nothing of kind {expected:?} was compared"
        );
    }
    let format = changes
        .iter()
        .find(|c| c.kind == ChangeKind::Format)
        .expect("the format change");
    assert_eq!(format.detail, "epoch 1 → 2");
}

#[test]
fn a_removed_command_is_a_change_and_not_a_silence() {
    let mut new = surface("2.0.0");
    new.commands.clear();
    let changes = compare(&surface("1.0.0"), &new);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].change, ChangeVerb::Removed);
    assert_eq!(changes[0].subject, "vibe doc build");
}

#[test]
fn a_help_reference_quotes_its_own_command_and_its_parents() {
    assert!(quotes_command("vibe doc build --help", "vibe doc build"));
    assert!(quotes_command("vibe doc build --help", "vibe doc"));
    assert!(!quotes_command("vibe doc check --help", "vibe doc build"));
}

#[test]
fn a_whole_manifest_block_answers_to_every_field() {
    assert!(quotes_field("org.acme/manual", "package.anything"));
    assert!(quotes_field(
        "org.acme/manual#package.title",
        "package.title"
    ));
    assert!(quotes_field("title", "package.title"));
    assert!(!quotes_field("title", "package.subtitle"));
}
