//! What the builder remembers, and what it refuses to.

use chrono::{TimeZone, Utc};

use super::*;

fn row(name: &str, version: &str, hash: &str) -> RenderedVersion {
    RenderedVersion {
        source: "vibespecs".into(),
        group: vibe_core::Group::parse("org.example").expect("a group"),
        name: name.into(),
        version: version.parse().expect("a version"),
        content_hash: hash.into(),
        rendered_at: Utc.with_ymd_and_hms(2026, 9, 12, 8, 0, 0).unwrap(),
        files: 56,
        failed: false,
    }
}

#[test]
fn a_written_state_reads_back_as_it_was_written() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let mut state = empty();
    state.rendered.push(row("docs", "1.0.0", "sha256:aa"));
    state.host_rendered_at = Some(Utc.with_ymd_and_hms(2026, 9, 12, 7, 0, 0).unwrap());
    write(tmp.path(), &state).expect("writing");
    assert_eq!(read(tmp.path()).expect("reading"), state);
}

/// Two runs over an unchanged site must write the same bytes, so the row
/// order cannot be «whichever package finished first».
#[test]
fn the_rows_are_sorted_however_they_arrived() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let mut state = empty();
    state.rendered.push(row("z", "1.0.0", "sha256:zz"));
    state.rendered.push(row("a", "2.0.0", "sha256:aa"));
    state.rendered.push(row("a", "10.0.0", "sha256:bb"));
    write(tmp.path(), &state).expect("writing");
    let first = std::fs::read_to_string(path(tmp.path())).expect("the file");

    let mut shuffled = empty();
    shuffled.rendered.push(row("a", "10.0.0", "sha256:bb"));
    shuffled.rendered.push(row("z", "1.0.0", "sha256:zz"));
    shuffled.rendered.push(row("a", "2.0.0", "sha256:aa"));
    write(tmp.path(), &shuffled).expect("writing");
    assert_eq!(
        first,
        std::fs::read_to_string(path(tmp.path())).expect("the file")
    );
}

/// A fresh output directory has rendered nothing, and «nothing» is the
/// correct answer rather than a failure.
#[test]
fn an_output_directory_with_no_state_has_rendered_nothing() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let state = read(tmp.path()).expect("the empty state");
    assert_eq!(state, empty());
}

#[test]
fn a_state_this_build_does_not_read_says_what_to_do_about_it() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    std::fs::create_dir_all(tmp.path().join(STATE_DIR)).expect("the directory");
    std::fs::write(path(tmp.path()), "{\"schema\":99,\"rendered\":[]}\n").expect("writing");
    let message = read(tmp.path()).expect_err("a refusal").to_string();
    assert!(message.contains("schema` 99"), "{message}");
    assert!(message.contains("delete the file"), "{message}");
}

#[test]
fn a_file_that_is_not_a_state_file_is_refused_by_name() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    std::fs::create_dir_all(tmp.path().join(STATE_DIR)).expect("the directory");
    std::fs::write(path(tmp.path()), "not json at all").expect("writing");
    let message = read(tmp.path()).expect_err("a refusal").to_string();
    assert!(message.contains("state.json"), "{message}");
}

#[test]
fn a_row_is_found_by_coordinate_and_version_and_not_by_source() {
    let mut state = empty();
    state.rendered.push(row("docs", "1.0.0", "sha256:aa"));
    assert!(rendered(&state, "org.example", "docs", "1.0.0").is_some());
    assert!(rendered(&state, "org.example", "docs", "1.0.1").is_none());
    assert!(rendered(&state, "org.other", "docs", "1.0.0").is_none());
}
