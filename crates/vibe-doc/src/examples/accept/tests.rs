//! `--accept` writes the capture and nothing else.

use std::fs;
use std::path::PathBuf;

use super::{Golden, write};

const PAGE: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<spec xmlns=\"https://vibevm.org/spec/1\">\n  \
  <title id=\"root\">T</title>\n  \
  <p>Prose the accept must not touch.</p>\n  \
  <example id=\"one\" fixture=\"empty\">\n    \
    <run>vibe init hello</run>\n    \
    <expect></expect>\n  \
  </example>\n  \
  <example id=\"two\" fixture=\"empty\">\n    \
    <run>vibe list</run>\n    \
    <expect>already signed</expect>\n  \
  </example>\n\
</spec>\n";

fn page() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("page.xml");
    fs::write(&path, PAGE).expect("write");
    (tmp, path)
}

#[test]
fn an_empty_golden_is_filled_and_the_rest_of_the_page_is_byte_identical() {
    let (_tmp, path) = page();
    write(
        &path,
        "one",
        &Golden {
            expect: "created 3 files".into(),
            stderr: None,
            exit: 0,
        },
        false,
    )
    .expect("fills");
    let after = fs::read_to_string(&path).expect("read");
    assert!(after.contains("<expect>created 3 files</expect>"));
    assert!(after.contains("<p>Prose the accept must not touch.</p>"));
    assert_eq!(
        after.replace("created 3 files", ""),
        PAGE,
        "nothing but the golden moved"
    );
}

#[test]
fn a_golden_that_already_holds_text_is_not_replaced_without_force() {
    let (_tmp, path) = page();
    let refused = write(
        &path,
        "two",
        &Golden {
            expect: "something else".into(),
            stderr: None,
            exit: 0,
        },
        false,
    )
    .expect_err("refused");
    let text = refused.to_string();
    assert!(text.contains("--force"), "{text}");
    assert!(text.contains("PROP-057#INV-EXAMPLES-RUN"), "{text}");
    assert_eq!(fs::read_to_string(&path).expect("read"), PAGE);
}

#[test]
fn force_re_blesses_deliberately() {
    let (_tmp, path) = page();
    write(
        &path,
        "two",
        &Golden {
            expect: "something else".into(),
            stderr: None,
            exit: 0,
        },
        true,
    )
    .expect("re-blesses");
    assert!(
        fs::read_to_string(&path)
            .expect("read")
            .contains("<expect>something else</expect>")
    );
}

#[test]
fn a_non_zero_exit_and_a_stderr_are_recorded_in_the_writers_own_order() {
    let (_tmp, path) = page();
    write(
        &path,
        "one",
        &Golden {
            expect: String::new(),
            stderr: Some("error: no registry configured".into()),
            exit: 1,
        },
        false,
    )
    .expect("records");
    let after = fs::read_to_string(&path).expect("read");
    assert!(after.contains("<example id=\"one\" fixture=\"empty\" exit=\"1\">"));
    assert!(after.contains("<expect></expect>"));
    assert!(after.contains("<stderr>error: no registry configured</stderr>"));
    let expect_at = after.find("<expect>").expect("expect");
    let stderr_at = after.find("<stderr>").expect("stderr");
    assert!(expect_at < stderr_at, "stderr follows expect");
}

#[test]
fn the_markup_characters_of_a_capture_are_escaped_the_writers_way() {
    let (_tmp, path) = page();
    write(
        &path,
        "one",
        &Golden {
            expect: "lane INDEX.md: 737 -> 854 B & <done>".into(),
            stderr: None,
            exit: 0,
        },
        false,
    )
    .expect("escapes");
    let after = fs::read_to_string(&path).expect("read");
    assert!(after.contains("737 -&gt; 854 B &amp; &lt;done&gt;"));
}

#[test]
fn an_unknown_id_is_refused_by_name() {
    let (_tmp, path) = page();
    let refused = write(
        &path,
        "nope",
        &Golden {
            expect: "x".into(),
            stderr: None,
            exit: 0,
        },
        false,
    )
    .expect_err("refused");
    assert!(refused.to_string().contains("no `<example>` with that id"));
}
