//! `--write-version-file` (PROP-075 `#VERSION-FILE-WRITE`): which file a
//! write lands in, the in-place keyed edit, and the refusals that all land
//! before any byte is written.
//!
//! The byte-identity assertions are the whole value of the `keyed` format,
//! so they compare **bytes** and not a re-parse: a writer that round-tripped
//! the file through a parser would normalise the spacing of every line it
//! never meant to touch, and a re-parse would not notice.

use std::fs;
use std::path::{Path, PathBuf};

use vibe_core::manifest::{KeyedComment, KeyedExtra, KeyedSeparator, KeyedSyntax, VersionFileDecl};

use super::*;
use crate::versionfile::keyed;

fn plain(names: &[&str]) -> VersionFileDecl {
    VersionFileDecl::Plain {
        names: names.iter().map(|name| (*name).to_owned()).collect(),
    }
}

/// `.sdkmanrc`'s shape: `key=value`, `#` comments, a second value refused.
fn sdkmanrc(key: &str) -> VersionFileDecl {
    VersionFileDecl::Keyed {
        names: vec![".sdkmanrc".to_owned()],
        key: key.to_owned(),
        syntax: KeyedSyntax {
            separator: KeyedSeparator::Equals,
            comment: KeyedComment::Hash,
            extra: KeyedExtra::Refuse,
        },
    }
}

/// A request with the two optional inputs absent, which is a bare
/// `--write-version-file`.
fn request<'a>(
    rows: &'a [VersionFileDecl],
    version: &'a str,
    create_in: &'a Path,
) -> WriteRequest<'a> {
    WriteRequest {
        slot: "java",
        package: "org.example/jdk",
        version,
        rows,
        path: None,
        found: None,
        create_in,
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-WRITE")]
fn a_keyed_write_edits_one_line_and_leaves_every_other_byte_alone() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".sdkmanrc");
    let before = "# the team's toolchain\r\njava=21.0.4-tem\r\nmaven = 3.9.9  # pinned\r\n\r\n";
    fs::write(&path, before).unwrap();
    let rows = [sdkmanrc("java")];
    let written = write(&request(&rows, "25.0.1-tem", dir.path())).unwrap();
    assert_eq!(written.path, path);
    assert_eq!(written.version, "25.0.1-tem");
    assert_eq!(written.format, "keyed");
    assert!(!written.created && !written.appended);
    // Byte for byte: the comment, the other key's spacing and its own
    // comment, the blank line and the CRLF endings all survive.
    assert_eq!(
        fs::read(&path).unwrap(),
        "# the team's toolchain\r\njava=25.0.1-tem\r\nmaven = 3.9.9  # pinned\r\n\r\n".as_bytes()
    );
    // And the file still answers for the other slot, unchanged.
    let maven = sdkmanrc("maven");
    let text = fs::read_to_string(&path).unwrap();
    let found = keyed(&text, "maven", maven.syntax().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(found.taken, "3.9.9");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-WRITE")]
fn the_spacing_the_key_line_had_is_the_spacing_it_keeps() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".sdkmanrc");
    fs::write(&path, "  java  =  21  # old\nmaven=3.9.9\n").unwrap();
    let rows = [sdkmanrc("java")];
    write(&request(&rows, "25", dir.path())).unwrap();
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "  java  =  25  # old\nmaven=3.9.9\n"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-WRITE")]
fn an_absent_key_is_appended_and_an_absent_file_is_created() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".sdkmanrc");
    let rows = [sdkmanrc("java")];
    // An absent file is created holding that one line.
    let created = write(&request(&rows, "21", dir.path())).unwrap();
    assert!(created.created && !created.appended);
    assert_eq!(fs::read_to_string(&path).unwrap(), "java=21\n");

    // A key the file does not carry is appended as the last line, and the
    // last line's missing ending is supplied.
    fs::write(&path, "maven=3.9.9").unwrap();
    let appended = write(&request(&rows, "21", dir.path())).unwrap();
    assert!(appended.appended && !appended.created);
    assert_eq!(fs::read_to_string(&path).unwrap(), "maven=3.9.9\njava=21\n");

    // The other two separators spell a created line their own way.
    for (separator, line) in [
        (KeyedSeparator::Whitespace, "java 21\n"),
        (KeyedSeparator::Colon, "java: 21\n"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let rows = [VersionFileDecl::Keyed {
            names: vec![".versions".to_owned()],
            key: "java".to_owned(),
            syntax: KeyedSyntax {
                separator,
                comment: KeyedComment::Hash,
                extra: KeyedExtra::First,
            },
        }];
        write(&request(&rows, "21", dir.path())).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join(".versions")).unwrap(),
            line
        );
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-WRITE")]
fn a_plain_write_is_the_one_line_and_nothing_else() {
    let dir = tempfile::tempdir().unwrap();
    let rows = [plain(&[".java-version", ".jdkrc"])];
    // With no file found, the first name of the first row, in `create_in`.
    let written = write(&request(&rows, "21.0.4", dir.path())).unwrap();
    assert_eq!(written.path, dir.path().join(".java-version"));
    assert_eq!(written.format, "plain");
    assert!(written.created);
    assert_eq!(
        fs::read_to_string(&written.path).unwrap(),
        "21.0.4\n",
        "the one line and nothing else, with its trailing newline"
    );
    // An existing file's other content is not a thing to preserve here.
    fs::write(&written.path, "# pinned\n17\n").unwrap();
    let again = write(&request(&rows, "21.0.4", dir.path())).unwrap();
    assert!(!again.created);
    assert_eq!(fs::read_to_string(&written.path).unwrap(), "21.0.4\n");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-WRITE")]
fn the_nearest_found_file_is_the_one_a_bare_flag_edits() {
    let dir = tempfile::tempdir().unwrap();
    let deep = dir.path().join("sub");
    fs::create_dir_all(&deep).unwrap();
    let found = deep.join(".jdkrc");
    fs::write(&found, "17\n").unwrap();
    let rows = [plain(&[".java-version", ".jdkrc"])];
    let written = write(&WriteRequest {
        found: Some(&found),
        ..request(&rows, "21", dir.path())
    })
    .unwrap();
    assert_eq!(
        written.path, found,
        "`--save` edits the file the project is actually using, not the \
         first declared name"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-WRITE")]
fn a_path_decides_its_format_by_name_and_a_name_no_row_claims_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let rows = [plain(&[".java-version"]), sdkmanrc("java")];
    let keyed_path = dir.path().join(".sdkmanrc");
    let written = write(&WriteRequest {
        path: Some(&keyed_path),
        ..request(&rows, "21", dir.path())
    })
    .unwrap();
    assert_eq!(written.format, "keyed");
    assert_eq!(fs::read_to_string(&keyed_path).unwrap(), "java=21\n");

    let stranger = dir.path().join(".jvmrc");
    let refusal = write(&WriteRequest {
        path: Some(&stranger),
        ..request(&rows, "21", dir.path())
    })
    .expect_err("a path matching no declared name is never a guess at a format");
    let message = refusal.to_string();
    assert!(message.contains(".jvmrc"), "{message}");
    assert!(message.contains("`.java-version`"), "{message}");
    assert!(message.contains("`.sdkmanrc`"), "{message}");
    assert!(!stranger.exists(), "the refusal lands before any write");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-WRITE")]
fn a_duplicated_key_refuses_naming_both_lines_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".sdkmanrc");
    let before = "java=21\nmaven=3.9.9\njava=17\n";
    fs::write(&path, before).unwrap();
    let rows = [sdkmanrc("java")];
    let refusal = write(&request(&rows, "25", dir.path()))
        .expect_err("guessing which duplicate to keep would discard a declaration");
    let message = refusal.to_string();
    assert!(message.contains("lines 1 and 3"), "{message}");
    assert_eq!(fs::read_to_string(&path).unwrap(), before);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-WRITE")]
fn a_slot_with_no_declared_version_file_refuses_naming_the_slot_and_the_package() {
    let dir = tempfile::tempdir().unwrap();
    let refusal =
        write(&request(&[], "21", dir.path())).expect_err("there is nothing to write into");
    let message = refusal.to_string();
    assert!(message.contains("`java`"), "{message}");
    assert!(message.contains("org.example/jdk"), "{message}");
}

#[cfg(unix)]
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-WRITE")]
fn a_symbolic_link_refuses_naming_it_and_leaves_the_target_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("real");
    fs::write(&real, "17\n").unwrap();
    let link = dir.path().join(".java-version");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let rows = [plain(&[".java-version"])];
    let refusal = write(&request(&rows, "21", dir.path()))
        .expect_err("vibe never writes through a link it did not make");
    let message = refusal.to_string();
    assert!(message.contains(".java-version"), "{message}");
    assert_eq!(fs::read_to_string(&real).unwrap(), "17\n");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-WRITE")]
fn two_slots_write_one_file_without_either_destroying_the_others_answer() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".sdkmanrc");
    let java = [sdkmanrc("java")];
    let maven = [sdkmanrc("maven")];
    write(&request(&java, "21.0.4-tem", dir.path())).unwrap();
    write(&WriteRequest {
        slot: "maven",
        rows: &maven,
        ..request(&maven, "3.9.9", dir.path())
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "java=21.0.4-tem\nmaven=3.9.9\n",
        "which is the whole reason the keyed format exists"
    );
    let paths: Vec<PathBuf> = fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(paths, [path]);
}
