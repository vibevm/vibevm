//! `[[application.version_file]]` grammar and refusals
//! (PROP-075 `#VERSION-FILE-FORMATS`, `#VERSION-FILE-KEYED`).

use super::Manifest;
use crate::manifest::{KeyedComment, KeyedExtra, KeyedSeparator, VersionFileDecl};

const PACKAGE: &str = r#"
[package]
name = "demo"
group = "org.vibevm"
kind = "app"
version = "1.1.0"

[application]
id = "demo"
"#;

/// `PACKAGE` with `rows` appended as `[[application.version_file]]` bodies.
fn with_rows(rows: &[&str]) -> String {
    let mut text = PACKAGE.to_owned();
    for row in rows {
        text.push_str("\n[[application.version_file]]\n");
        text.push_str(row);
        text.push('\n');
    }
    text
}

/// The refusal of a document, whether it comes from parsing (an unknown
/// member of a closed table) or from validation (a field's own rule).
fn refusal(text: &str) -> String {
    match Manifest::parse_str(text) {
        Err(error) => error.to_string(),
        Ok(manifest) => manifest
            .validate()
            .expect_err("the document must be refused")
            .to_string(),
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS")]
fn version_file_rows_parse_round_trip_and_name_their_files() {
    let manifest = Manifest::parse_str(&with_rows(&[
        "format = \"plain\"\nnames = [\".nvmrc\", \".node-version\"]",
        "format = \"tool-versions\"\nkey = \"nodejs\"",
    ]))
    .unwrap();
    manifest.validate().unwrap();
    let rows = &manifest.application.as_ref().unwrap().version_file;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].format(), "plain");
    assert_eq!(rows[0].names(), [".nvmrc", ".node-version"]);
    assert_eq!(rows[0].key(), None);
    assert_eq!(rows[1].format(), "tool-versions");
    assert_eq!(rows[1].names(), [".tool-versions"]);
    assert_eq!(rows[1].key(), Some("nodejs"));
    // Absent is the default, so every existing manifest is byte-identical.
    assert!(
        Manifest::parse_str(PACKAGE)
            .unwrap()
            .application
            .unwrap()
            .version_file
            .is_empty()
    );
    assert_eq!(
        Manifest::parse_str(&toml::to_string(&manifest).unwrap()).unwrap(),
        manifest
    );
}

/// The no-drift proof of PROP-075 `#VERSION-FILE-KEYED`: the short
/// spelling and the `keyed` row it expands to are the **same declared
/// row**, so there is one reader and one writer and the two cannot drift —
/// and a manifest that authored the short spelling serialises back to it.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-KEYED")]
fn tool_versions_is_the_short_spelling_of_one_keyed_row() {
    let short = Manifest::parse_str(&with_rows(&[
        "format = \"tool-versions\"\nkey = \"nodejs\"",
    ]))
    .expect("the short spelling parses");
    let long = Manifest::parse_str(&with_rows(&[
        "format = \"keyed\"\nnames = [\".tool-versions\"]\nkey = \"nodejs\"\n\
         separator = \"whitespace\"\ncomment = \"hash\"\nextra = \"first\"",
    ]))
    .expect("the long spelling parses");
    short.validate().unwrap();
    long.validate().unwrap();
    let rows = |manifest: &Manifest| manifest.application.clone().unwrap().version_file;
    assert_eq!(rows(&short), rows(&long), "one declared row, two spellings");
    assert_eq!(
        rows(&short)[0],
        VersionFileDecl::tool_versions("nodejs"),
        "and it is the row the expansion constructs"
    );
    // The spelling survives a round trip, so the real Node package's
    // manifest and the sidecar derived from it stay byte-identical.
    let text = toml::to_string(&short).unwrap();
    assert!(text.contains("format = \"tool-versions\""), "{text}");
    assert!(!text.contains("separator"), "{text}");
    assert_eq!(Manifest::parse_str(&text).unwrap(), short);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-KEYED")]
fn a_keyed_row_declares_its_line_syntax_from_the_closed_vocabulary() {
    let manifest = Manifest::parse_str(&with_rows(&[
        "format = \"keyed\"\nnames = [\".sdkmanrc\"]\nkey = \"java\"\nseparator = \"equals\"",
        "format = \"keyed\"\nnames = [\".mise.toml\"]\nkey = \"java\"\n\
         separator = \"colon\"\ncomment = \"semicolon\"\nextra = \"first\"",
    ]))
    .unwrap();
    manifest.validate().unwrap();
    let rows = manifest.application.unwrap().version_file;
    assert_eq!(rows[0].format(), "keyed");
    assert_eq!(rows[0].names(), [".sdkmanrc"]);
    assert_eq!(rows[0].key(), Some("java"));
    // `comment` and `extra` have defaults; `separator` has none.
    let first = rows[0].syntax().expect("a keyed row carries its syntax");
    assert_eq!(first.separator, KeyedSeparator::Equals);
    assert_eq!(first.comment, KeyedComment::Hash);
    assert_eq!(first.extra, KeyedExtra::Refuse);
    let second = rows[1].syntax().unwrap();
    assert_eq!(second.separator, KeyedSeparator::Colon);
    assert_eq!(second.comment, KeyedComment::Semicolon);
    assert_eq!(second.extra, KeyedExtra::First);
    assert!(!rows[1].is_tool_versions());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-KEYED")]
fn every_keyed_refusal_names_its_field() {
    for (row, needle) in [
        // the line syntax is a closed vocabulary, never a pattern
        (
            "format = \"keyed\"\nnames = [\".sdkmanrc\"]\nkey = \"java\"\nseparator = \"tabs\"",
            "application.version_file[0].separator",
        ),
        (
            "format = \"keyed\"\nnames = [\".sdkmanrc\"]\nkey = \"java\"\n\
             separator = \"equals\"\ncomment = \"slashes\"",
            "application.version_file[0].comment",
        ),
        (
            "format = \"keyed\"\nnames = [\".sdkmanrc\"]\nkey = \"java\"\n\
             separator = \"equals\"\nextra = \"last\"",
            "application.version_file[0].extra",
        ),
        // a keyed row states its key and its separator
        (
            "format = \"keyed\"\nnames = [\".sdkmanrc\"]\nseparator = \"equals\"",
            "application.version_file[0].key",
        ),
        (
            "format = \"keyed\"\nnames = [\".sdkmanrc\"]\nkey = \"java\"",
            "application.version_file[0].separator",
        ),
        // and the ordinary name and key rules bind it as they bind the rest
        (
            "format = \"keyed\"\nnames = []\nkey = \"java\"\nseparator = \"equals\"",
            "application.version_file[0].names",
        ),
        (
            "format = \"keyed\"\nnames = [\"sub/.sdkmanrc\"]\nkey = \"java\"\n\
             separator = \"equals\"",
            "application.version_file[0].names",
        ),
        (
            "format = \"keyed\"\nnames = [\".sdkmanrc\"]\nkey = \"ja va\"\n\
             separator = \"equals\"",
            "application.version_file[0].key",
        ),
        // a member of another format is unknown to this one
        (
            "format = \"keyed\"\nnames = [\".sdkmanrc\"]\nkey = \"java\"\n\
             separator = \"equals\"\npattern = \"(.*)\"",
            "pattern",
        ),
    ] {
        let error = refusal(&with_rows(&[row]));
        assert!(error.contains(needle), "`{row}` refused as `{error}`");
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS")]
fn every_version_file_refusal_names_its_field() {
    for (row, needle) in [
        // the format set is closed: nothing else is a format
        ("format = \"engines\"\nnames = [\"package.json\"]", "format"),
        // a plain row needs at least one name
        (
            "format = \"plain\"\nnames = []",
            "application.version_file[0].names",
        ),
        // a name is one plain file name of one directory
        (
            "format = \"plain\"\nnames = [\"sub/.nvmrc\"]",
            "application.version_file[0].names",
        ),
        (
            "format = \"plain\"\nnames = [\" .nvmrc\"]",
            "application.version_file[0].names",
        ),
        (
            "format = \"plain\"\nnames = [\"..\"]",
            "application.version_file[0].names",
        ),
        (
            "format = \"plain\"\nnames = [\"\"]",
            "application.version_file[0].names",
        ),
        // a tool-versions key is one word
        (
            "format = \"tool-versions\"\nkey = \"\"",
            "application.version_file[0].key",
        ),
        (
            "format = \"tool-versions\"\nkey = \"node js\"",
            "application.version_file[0].key",
        ),
        // a member of the other format is unknown to this one
        ("format = \"plain\"\nkey = \"nodejs\"", "version_file"),
        (
            "format = \"tool-versions\"\nnames = [\".nvmrc\"]",
            "version_file",
        ),
    ] {
        let error = refusal(&with_rows(&[row]));
        assert!(error.contains(needle), "`{row}` refused as `{error}`");
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS")]
fn two_rows_never_claim_the_same_file() {
    let clash = refusal(&with_rows(&[
        "format = \"plain\"\nnames = [\".nvmrc\"]",
        "format = \"plain\"\nnames = [\".node-version\", \".nvmrc\"]",
    ]));
    assert!(
        clash.contains("application.version_file[1].names") && clash.contains(".nvmrc"),
        "{clash}"
    );
    let twice = refusal(&with_rows(&[
        "format = \"tool-versions\"\nkey = \"nodejs\"",
        "format = \"tool-versions\"\nkey = \"node\"",
    ]));
    assert!(
        twice.contains("application.version_file[1].format"),
        "{twice}"
    );
    let both = refusal(&with_rows(&[
        "format = \"plain\"\nnames = [\".tool-versions\"]",
        "format = \"tool-versions\"\nkey = \"nodejs\"",
    ]));
    assert!(
        both.contains("application.version_file[1].format"),
        "{both}"
    );
}
