//! `[[application.version_file]]` grammar and refusals
//! (PROP-075 `#VERSION-FILE-FORMATS`).

use super::Manifest;

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
