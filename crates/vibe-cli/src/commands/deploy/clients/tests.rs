//! The resolver's laws, over INJECTED directories and extensions.
//!
//! Nothing here reads the ambient environment or names a real client: the
//! pure half takes its search path as a parameter precisely so the law can
//! be proven against a temp tree, on every host, without an installed CLI.

use std::path::PathBuf;

use specmark::verifies;
use vibe_lifecycle::ClientExecutable;

use super::{locate, locate_with};

/// A directory holding the named files.
fn directory(names: &[&str]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("the fixture directory creates");
    for name in names {
        let path = dir.path().join(name);
        std::fs::write(&path, b"#!/bin/sh\n").expect("the fixture file writes");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;

            let mut permissions = std::fs::metadata(&path)
                .expect("the fixture metadata reads")
                .permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&path, permissions).expect("the fixture becomes executable");
        }
    }
    dir
}

/// The bare command word wins over any extension, and the answer is the
/// ABSOLUTE path — never the word that was searched for.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_found_client_resolves_to_an_absolute_path_and_never_to_a_command_word() {
    let dir = directory(&["claude"]);

    let found = locate("claude", &[dir.path().to_path_buf()], &[".EXE".to_owned()]);

    let ClientExecutable::Resolved { command, path } = &found else {
        panic!("expected a resolution, got: {found:?}");
    };
    assert_eq!(command, "claude");
    assert!(path.is_absolute(), "`{}` must be absolute", path.display());
    assert_eq!(path, &dir.path().join("claude"));
}

/// PATHEXT is consulted in the order it was given, and only after the bare
/// name — the shape an npm-installed `.cmd` shim needs.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn the_injected_extensions_are_tried_in_order_after_the_bare_name() {
    let dir = directory(&["codex.CMD", "codex.EXE"]);
    let search = [dir.path().to_path_buf()];

    let exe_first = locate("codex", &search, &[".EXE".to_owned(), ".CMD".to_owned()]);
    assert_eq!(
        exe_first.resolved_path(),
        Some(dir.path().join("codex.EXE").as_path()),
    );

    let cmd_first = locate("codex", &search, &[".CMD".to_owned(), ".EXE".to_owned()]);
    assert_eq!(
        cmd_first.resolved_path(),
        Some(dir.path().join("codex.CMD").as_path()),
    );

    // With no extension list at all, only the bare name is a candidate.
    assert!(matches!(
        locate("codex", &search, &[]),
        ClientExecutable::Missing { .. },
    ));
}

/// Search directories are walked in order, and the first hit wins — the
/// same precedence a shell gives `PATH`.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn the_first_search_directory_holding_the_command_wins() {
    let first = directory(&["opencode"]);
    let second = directory(&["opencode"]);

    let found = locate(
        "opencode",
        &[first.path().to_path_buf(), second.path().to_path_buf()],
        &[],
    );

    assert_eq!(
        found.resolved_path(),
        Some(first.path().join("opencode").as_path()),
    );
}

/// A RELATIVE search entry is skipped rather than honoured: it would
/// resolve against the current working directory, which is exactly the
/// ambient dependency the injected authority exists to remove.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_relative_search_directory_is_skipped() {
    let dir = directory(&["claude"]);
    let relative = PathBuf::from(".");

    let found = locate("claude", &[relative], &[]);

    assert!(
        matches!(found, ClientExecutable::Missing { ref command } if command == "claude"),
        "{found:?}",
    );
    // The same command IS found once the entry is absolute, so the refusal
    // above is about the entry's shape and not about the command.
    assert!(
        locate("claude", &[dir.path().to_path_buf()], &[])
            .resolved_path()
            .is_some(),
    );
}

/// A directory that merely SHARES the command's name is not a client, and
/// is not an error either.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_directory_named_like_the_command_is_not_a_client() {
    let dir = tempfile::tempdir().expect("the fixture directory creates");
    std::fs::create_dir(dir.path().join("claude")).expect("the decoy directory creates");

    let found = locate("claude", &[dir.path().to_path_buf()], &[]);

    assert!(
        matches!(found, ClientExecutable::Missing { .. }),
        "{found:?}"
    );
}

/// The resolver keeps searching when the platform predicate rejects a file.
/// This is cross-platform mutation evidence for the Unix execute-bit branch:
/// a resolver that merely tests `is_file` returns the decoy and fails here.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_rejected_file_never_shadows_a_later_client() {
    let decoy = directory(&["claude"]);
    let real = directory(&["claude"]);
    let decoy_path = decoy.path().join("claude");

    let found = locate_with(
        "claude",
        &[decoy.path().to_path_buf(), real.path().to_path_buf()],
        &[],
        |candidate| candidate.is_file() && candidate != decoy_path,
    );

    assert_eq!(
        found.resolved_path(),
        Some(real.path().join("claude").as_path()),
    );
}

/// Unix PATH resolution skips a non-executable file and keeps searching;
/// otherwise an unrelated text file in an earlier directory would shadow a
/// real client that the shell itself can execute from a later one.
#[cfg(unix)]
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_non_executable_file_never_shadows_a_later_client() {
    use std::os::unix::fs::PermissionsExt as _;

    let decoy = tempfile::tempdir().expect("the decoy directory creates");
    let decoy_path = decoy.path().join("claude");
    std::fs::write(&decoy_path, b"not executable\n").expect("the decoy writes");
    let mut permissions = std::fs::metadata(&decoy_path)
        .expect("the decoy metadata reads")
        .permissions();
    permissions.set_mode(0o644);
    std::fs::set_permissions(&decoy_path, permissions).expect("the decoy stays non-executable");

    let real = directory(&["claude"]);
    let found = locate(
        "claude",
        &[decoy.path().to_path_buf(), real.path().to_path_buf()],
        &[],
    );

    assert_eq!(
        found.resolved_path(),
        Some(real.path().join("claude").as_path()),
    );
}

/// An unfound client is a TYPED value carrying the command word, never a
/// silent hole and never a bare word posing as a path. This is the half a
/// client provider turns into remediation.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn an_unfound_client_names_the_command_an_operator_must_install() {
    let empty = tempfile::tempdir().expect("the fixture directory creates");

    let found = locate(
        "opencode",
        &[empty.path().to_path_buf()],
        &[".EXE".to_owned()],
    );

    let ClientExecutable::Missing { command } = &found else {
        panic!("expected a typed absence, got: {found:?}");
    };
    assert_eq!(command, "opencode");
    assert!(found.resolved_path().is_none());
}
