//! Reading the one staleness signal the project keeps.

use super::*;

fn written(text: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temp");
    let path = dir.path().join("reviews.toml");
    std::fs::write(&path, text).expect("write");
    (dir, path)
}

/// A package that has never been read aloud has no file, and that is a
/// state, not a failure: requiring one would make the maintenance loop a
/// precondition of publishing.
#[test]
fn an_absent_file_is_an_empty_set_and_not_an_error() {
    let dir = tempfile::tempdir().expect("temp");
    let reviews = Reviews::read(dir.path().join("reviews.toml")).expect("absent is fine");
    assert!(reviews.is_empty());
}

/// The two shapes a hand-written file takes: a bare date, and a table
/// that also says who read it.
#[test]
fn both_the_bare_date_and_the_table_form_are_read() {
    let (_dir, path) = written(
        "[pages]\n\
         \"model/versions.xml\" = 2026-09-12\n\
         \"start/index.xml\" = { read = 2026-09-01, by = \"Oleg\" }\n",
    );
    let reviews = Reviews::read(&path).expect("parses");
    assert_eq!(reviews.len(), 2);
    let when = reviews.read_aloud("model/versions.xml").expect("a date");
    assert_eq!(when.to_rfc3339(), "2026-09-12T00:00:00+00:00");
    assert!(reviews.read_aloud("start/index.xml").is_some());
}

/// A quoted date means the same day as a bare one. A reader that took
/// only one of the two spellings would be a trap in a file people edit by
/// hand.
#[test]
fn a_quoted_date_is_the_same_day() {
    let (_dir, path) = written("[pages]\n\"a.xml\" = \"2026-09-12\"\n");
    let reviews = Reviews::read(&path).expect("parses");
    assert_eq!(
        reviews.read_aloud("a.xml").map(|d| d.to_rfc3339()),
        Some("2026-09-12T00:00:00+00:00".to_owned())
    );
}

/// The file belongs to the maintenance loop and will grow columns. A
/// value this reader does not understand is ignored, not refused: a
/// manifest has no business having an opinion about the loop's own data.
#[test]
fn a_value_this_reader_does_not_understand_is_ignored() {
    let (_dir, path) = written("[pages]\n\"a.xml\" = 7\n\"b.xml\" = 2026-09-12\n");
    let reviews = Reviews::read(&path).expect("parses");
    assert_eq!(reviews.len(), 1);
    assert!(reviews.read_aloud("a.xml").is_none());
}

/// A file that does not parse at all IS refused — silence there would
/// make every page look unread for a reason nobody could see.
#[test]
fn a_file_that_does_not_parse_is_refused_by_name() {
    let (_dir, path) = written("[pages\n");
    let error = Reviews::read(&path).expect_err("refused");
    assert!(error.to_string().contains("reviews.toml"), "{error}");
}
