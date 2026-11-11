//! Every refusal of a download-header file names the problem and the line
//! it is on, and no refusal — and no `Debug` — ever carries a value.

use super::*;

const SECRET: &str = "s3cr3t-value-not-to-be-seen";

#[test]
fn comments_and_blank_lines_are_skipped_and_order_is_kept() {
    let headers = DownloadHeaders::parse(
        "# a comment\n\n   \nAuthorization: Bearer one\n\t# indented comment\nX-Mirror: eu\n",
        "headers.txt",
    )
    .unwrap();
    assert_eq!(
        headers
            .iter()
            .map(|header| (header.name(), header.value()))
            .collect::<Vec<_>>(),
        [("Authorization", "Bearer one"), ("X-Mirror", "eu")]
    );
}

#[test]
fn a_value_may_hold_a_colon_and_surrounding_space_is_trimmed() {
    let headers = DownloadHeaders::parse("X-Range:   bytes: 0-1   \n", "h").unwrap();
    assert_eq!(headers.iter().next().unwrap().value(), "bytes: 0-1");
}

#[test]
fn a_crlf_file_reads_as_a_lf_one() {
    let headers =
        DownloadHeaders::parse("Authorization: Bearer one\r\nX-Mirror: eu\r\n", "h").unwrap();
    assert_eq!(headers.len(), 2);
    assert_eq!(headers.iter().next().unwrap().value(), "Bearer one");
}

#[test]
fn an_empty_value_is_a_header() {
    let headers = DownloadHeaders::parse("X-Mirror:\n", "h").unwrap();
    assert_eq!(headers.iter().next().unwrap().value(), "");
}

#[test]
fn a_path_that_does_not_exist_is_refused_by_name() {
    let home = tempfile::tempdir().unwrap();
    let missing = home.path().join("absent.txt");
    let error = DownloadHeaders::read(&missing).unwrap_err();
    assert!(matches!(error, DownloadHeaderError::Unreadable { .. }));
    assert!(
        error.to_string().contains("absent.txt"),
        "the refusal names the file: {error}"
    );
}

#[test]
fn an_empty_file_is_refused() {
    assert_eq!(
        DownloadHeaders::parse("# only a comment\n\n", "h.txt").unwrap_err(),
        DownloadHeaderError::Empty {
            path: "h.txt".into()
        }
    );
    assert_eq!(
        DownloadHeaders::parse("", "h.txt").unwrap_err(),
        DownloadHeaderError::Empty {
            path: "h.txt".into()
        }
    );
}

#[test]
fn a_line_that_is_not_name_colon_value_is_refused_with_its_line_number() {
    let error = DownloadHeaders::parse(&format!("Authorization: ok\nBearer {SECRET}\n"), "h.txt")
        .unwrap_err();
    assert_eq!(
        error,
        DownloadHeaderError::Malformed {
            path: "h.txt".into(),
            line: 2
        }
    );
    assert!(!error.to_string().contains(SECRET), "{error}");
}

#[test]
fn an_invalid_field_name_is_refused_with_its_line_number() {
    for (text, line) in [
        (format!("X Mirror: {SECRET}\n"), 1),
        (format!("# c\n\nX-Mir\u{00e9}: {SECRET}\n"), 3),
        (format!(": {SECRET}\n"), 1),
    ] {
        let error = DownloadHeaders::parse(&text, "h.txt").unwrap_err();
        assert_eq!(
            error,
            DownloadHeaderError::InvalidName {
                path: "h.txt".into(),
                line
            },
            "{text:?}"
        );
        assert!(!error.to_string().contains(SECRET), "{error}");
    }
}

#[test]
fn a_control_character_or_newline_in_a_value_is_refused_by_name() {
    // `lines()` has already split on the newline, so a credential that
    // tries to carry one arrives as the escape the file actually holds.
    let error = DownloadHeaders::parse(
        &format!("X-Mirror: eu\nAuthorization: Bearer \u{0007}{SECRET}\n"),
        "h.txt",
    )
    .unwrap_err();
    assert_eq!(
        error,
        DownloadHeaderError::ControlCharacter {
            path: "h.txt".into(),
            line: 2,
            name: "Authorization".into()
        }
    );
    assert!(!error.to_string().contains(SECRET), "{error}");
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-075#DOWNLOAD-HEADERS")]
fn no_debug_rendering_of_a_header_ever_carries_its_value() {
    let headers =
        DownloadHeaders::parse(&format!("Authorization: Bearer {SECRET}\n"), "h.txt").unwrap();
    let shown = format!("{headers:?} {:?}", headers.iter().next().unwrap());
    assert!(!shown.contains(SECRET), "{shown}");
    assert!(shown.contains("Authorization"), "{shown}");
}

#[test]
fn a_file_is_read_from_disk_exactly_as_parsed_text() {
    let home = tempfile::tempdir().unwrap();
    let path = home.path().join("headers.txt");
    std::fs::write(&path, format!("Authorization: Bearer {SECRET}\n")).unwrap();
    let headers = DownloadHeaders::read(&path).unwrap();
    assert_eq!(
        headers.iter().next().unwrap().value(),
        format!("Bearer {SECRET}")
    );
}
