//! The shared containment laws, pinned where they live.
//!
//! Every one of these used to be asserted only indirectly, through the
//! Cargo adapter's own refusals. Now two more providers depend on them, so
//! the laws are proven once at their home: a spelling that escapes is
//! refused before any filesystem call, a link is never mistaken for the
//! thing it names, and a digest is of the bytes that were really read.

use super::*;

#[test]
fn a_traversal_or_absolute_spelling_refuses_before_any_filesystem_call() {
    assert_eq!(checked_relative("../escape"), Err(PathFault::Traversal));
    assert_eq!(checked_relative("a/../b"), Err(PathFault::Traversal));
    assert_eq!(checked_relative(""), Err(PathFault::Empty));
    assert_eq!(checked_relative("/etc/passwd"), Err(PathFault::Absolute));
    assert_eq!(checked_relative("./a"), Err(PathFault::NonCanonical));
    assert_eq!(checked_relative("a//b"), Err(PathFault::NonCanonical));
    assert_eq!(checked_relative("a/"), Err(PathFault::NonCanonical));
    if cfg!(windows) {
        assert_eq!(checked_relative("C:/w"), Err(PathFault::Absolute));
    }
}

#[test]
fn a_lawful_spelling_canonicalises_to_forward_slashes() {
    assert_eq!(
        checked_relative("skills/demo/SKILL.md").as_deref(),
        Ok("skills/demo/SKILL.md")
    );
    assert_eq!(checked_relative("a\\b").as_deref(), Ok("a/b"));
}

#[test]
fn relative_identity_is_none_outside_the_root() {
    let root = std::path::Path::new("/w/demo/target");
    assert_eq!(
        relative_to(std::path::Path::new("/w/demo/target/release/app"), root).as_deref(),
        Some("release/app"),
    );
    assert_eq!(
        relative_to(std::path::Path::new("/w/demo/src/app"), root),
        None,
    );
    assert_eq!(
        relative_to(root, root),
        None,
        "the root itself names no tail"
    );
}

#[test]
fn absence_and_a_directory_are_distinct_faults() {
    let project = match tempfile::TempDir::new() {
        Ok(project) => project,
        Err(error) => panic!("a temp project opens: {error}"),
    };
    let ghost = project.path().join("absent");

    assert!(matches!(
        prove_regular_file(&ghost),
        Err(FileFault::Missing(_))
    ));
    assert_eq!(
        prove_regular_file(project.path()),
        Err(FileFault::NotRegular),
    );
    if let Err(error) = prove_directory(project.path()) {
        panic!("a real directory proves: {}", error.reason());
    }
}

#[cfg(unix)]
#[test]
fn a_linked_final_component_refuses_as_a_link_not_as_a_missing_file() {
    let project = match tempfile::TempDir::new() {
        Ok(project) => project,
        Err(error) => panic!("a temp project opens: {error}"),
    };
    let real = project.path().join("real.txt");
    if let Err(error) = std::fs::write(&real, b"bytes") {
        panic!("the fixture writes: {error}");
    }
    let link = project.path().join("link.txt");
    if let Err(error) = std::os::unix::fs::symlink(&real, &link) {
        panic!("the fixture links: {error}");
    }

    assert_eq!(prove_regular_file(&link), Err(FileFault::Link));
}

#[test]
fn the_digest_is_of_the_bytes_that_were_really_read() {
    let project = match tempfile::TempDir::new() {
        Ok(project) => project,
        Err(error) => panic!("a temp project opens: {error}"),
    };
    let file = project.path().join("payload.bin");
    let payload = vec![7_u8; DIGEST_WINDOW + 13];
    if let Err(error) = std::fs::write(&file, &payload) {
        panic!("the fixture writes: {error}");
    }

    match digest_file(&file) {
        Ok((digest, bytes)) => {
            assert_eq!(bytes, payload.len() as u64);
            assert_eq!(
                digest,
                format!("{:x}", Sha256::digest(&payload)),
                "streamed across windows",
            );
        }
        Err(error) => panic!("the produced file digests: {}", error.reason()),
    }
}

#[test]
fn a_resource_over_the_inline_cap_refuses_rather_than_loading() {
    let project = match tempfile::TempDir::new() {
        Ok(project) => project,
        Err(error) => panic!("a temp project opens: {error}"),
    };
    let file = project.path().join("big.md");
    if let Err(error) = std::fs::write(&file, vec![b'x'; 128]) {
        panic!("the fixture writes: {error}");
    }

    assert!(matches!(
        read_file_bounded(&file, 64),
        Err(FileFault::Read(_))
    ));
    assert!(read_file_bounded(&file, 128).is_ok());
}
