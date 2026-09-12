//! The card a package that never wrote one gets.

use super::*;

fn manifest(text: &str) -> Manifest {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    std::fs::write(tmp.path().join("vibe.toml"), text).expect("writing");
    read(tmp.path()).expect("a manifest")
}

#[test]
fn a_declared_card_is_used_as_declared() {
    let card = manifest(
        "[package]\nname = \"a\"\ngroup = \"org.example\"\nversion = \"1.0.0\"\n\
         kind = \"doc\"\ntitle = \"The manual\"\nabstract = \"  What it covers.  \"\n",
    );
    assert_eq!(card.title(), "The manual");
    assert_eq!(card.abstract_(), "What it covers.");
    assert_eq!(card.kind(), "doc");
}

#[test]
fn a_title_nobody_wrote_is_the_name_a_shelf_would_show_anyway() {
    let card =
        manifest("[package]\nname = \"wal\"\ngroup = \"org.example\"\nversion = \"1.0.0\"\n");
    assert_eq!(card.title(), "wal");
}

#[test]
fn an_abstract_falls_back_to_the_one_line_summary_before_it_invents_one() {
    let card = manifest(
        "[package]\nname = \"wal\"\ngroup = \"org.example\"\nversion = \"1.0.0\"\n\
         description = \"A log.\"\n",
    );
    assert_eq!(card.abstract_(), "A log.");
}

#[test]
fn an_abstract_nobody_wrote_says_so_rather_than_being_empty() {
    let card = manifest(
        "[package]\nname = \"wal\"\ngroup = \"org.example\"\nversion = \"1.0.0\"\nkind = \"flow\"\n",
    );
    let composed = card.abstract_();
    assert!(composed.contains("org.example/wal"), "{composed}");
    assert!(composed.contains("flow"), "{composed}");
    assert!(composed.contains("wrote no abstract"), "{composed}");
}

/// The composed card carries prose, and prose carries quotes.
#[test]
fn a_quotation_mark_in_a_card_survives_into_the_manifest() {
    let card = manifest(
        "[package]\nname = \"a\"\ngroup = \"org.example\"\nversion = \"1.0.0\"\n\
         title = 'The \"real\" one'\n",
    );
    let written = synthesise(&card);
    let back: toml::Value = toml::from_str(&written).expect("the synthesised manifest parses");
    assert_eq!(
        back.get("package")
            .and_then(|p| p.get("title"))
            .and_then(toml::Value::as_str),
        Some("The \"real\" one")
    );
}

#[test]
fn a_manifest_that_names_no_coordinate_is_refused() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    std::fs::write(tmp.path().join("vibe.toml"), "[package]\nname = \"a\"\n").expect("writing");
    let message = read(tmp.path()).expect_err("a refusal").to_string();
    assert!(message.contains("names no"), "{message}");
}

/// An array of tables must come out as one, or the relation would change
/// shape on the way through.
#[test]
fn an_array_of_tables_is_re_emitted_as_an_array_of_tables() {
    let card = manifest(
        "[package]\nname = \"a\"\ngroup = \"org.example\"\nversion = \"1.0.0\"\nkind = \"doc\"\n\
         title = \"T\"\nabstract = \"A\"\n\
         \n[[documents]]\npackage = \"org.example/x\"\nversion = \"^1.0\"\n\
         \n[[documents]]\npackage = \"org.example/y\"\nversion = \"^2.0\"\n",
    );
    let written = synthesise(&card);
    assert!(written.contains("[[documents]]"), "{written}");
    let back: toml::Value = toml::from_str(&written).expect("it parses");
    assert_eq!(
        back.get("documents")
            .and_then(toml::Value::as_array)
            .map(Vec::len),
        Some(2)
    );
}
