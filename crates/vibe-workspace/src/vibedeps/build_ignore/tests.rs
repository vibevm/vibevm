//! Byte, concurrency, and Git-behaviour oracles for the generated ignore file.

use std::fs;
use std::path::Path;
use std::sync::{Arc, Barrier};

use specmark::verifies;

use super::*;

const FRESH: &[u8] = b"# Build output produced inside materialised dependency slots.\n\
# Managed by vibe; additional entries are preserved until `vibe clean`.\n\
**/target/\n\
**/node_modules/\n";

fn root() -> (tempfile::TempDir, std::path::PathBuf) {
    let project = tempfile::tempdir().unwrap();
    let root = project.path().join("vibedeps");
    (project, root)
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IN-SLOT-BUILD")]
fn a_fresh_root_gets_the_exact_managed_file_and_clean_recreates_it() {
    let (_project, root) = root();
    assert!(ensure_build_output_ignores(&root).unwrap());
    assert_eq!(fs::read(root.join(".gitignore")).unwrap(), FRESH);

    fs::remove_dir_all(&root).unwrap();
    assert!(ensure_build_output_ignores(&root).unwrap());
    assert_eq!(fs::read(root.join(".gitignore")).unwrap(), FRESH);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IN-SLOT-BUILD")]
fn existing_prefix_comments_custom_bytes_and_missing_final_newline_survive() {
    let (_project, root) = root();
    fs::create_dir_all(&root).unwrap();
    let path = root.join(".gitignore");
    let prefix = b"# operator comment\n/custom/\nraw-\xff-byte";
    fs::write(&path, prefix).unwrap();

    assert!(ensure_build_output_ignores(&root).unwrap());
    let after = fs::read(path).unwrap();
    assert!(after.starts_with(prefix));
    assert_eq!(&after[prefix.len()..], b"\n**/target/\n**/node_modules/\n");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IN-SLOT-BUILD")]
fn one_active_rule_appends_only_the_other_and_keeps_crlf() {
    let (_project, root) = root();
    fs::create_dir_all(&root).unwrap();
    let path = root.join(".gitignore");
    let before = b"# operator\r\n**/target/\r\n";
    fs::write(&path, before).unwrap();

    assert!(ensure_build_output_ignores(&root).unwrap());
    let after = fs::read(path).unwrap();
    assert!(after.starts_with(before));
    assert_eq!(&after[before.len()..], b"**/node_modules/\r\n");
    assert_eq!(count_line(&after, b"**/target/"), 1);
    assert_eq!(count_line(&after, b"**/node_modules/"), 1);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IN-SLOT-BUILD")]
fn comments_and_negations_do_not_masquerade_as_active_rules() {
    let (_project, root) = root();
    fs::create_dir_all(&root).unwrap();
    let path = root.join(".gitignore");
    let before = b"# **/target/\n!**/node_modules/\n";
    fs::write(&path, before).unwrap();

    assert!(ensure_build_output_ignores(&root).unwrap());
    let after = fs::read(path).unwrap();
    assert!(after.starts_with(before));
    assert_eq!(count_line(&after, b"**/target/"), 1);
    assert_eq!(count_line(&after, b"**/node_modules/"), 1);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IN-SLOT-BUILD")]
fn complete_file_is_an_exact_idempotent_no_op() {
    let (_project, root) = root();
    assert!(ensure_build_output_ignores(&root).unwrap());
    let path = root.join(".gitignore");
    let mut before = fs::read(&path).unwrap();
    before.extend_from_slice(b"# trailing operator comment\n\n   \n");
    fs::write(&path, &before).unwrap();
    assert!(!ensure_build_output_ignores(&root).unwrap());
    assert_eq!(fs::read(path).unwrap(), before);
}

#[cfg(windows)]
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IN-SLOT-BUILD")]
fn a_complete_readonly_file_needs_no_writable_handle() {
    let (_project, root) = root();
    ensure_build_output_ignores(&root).unwrap();
    let path = root.join(".gitignore");
    let original = fs::metadata(&path).unwrap().permissions();
    let mut permissions = original.clone();
    permissions.set_readonly(true);
    fs::set_permissions(&path, permissions).unwrap();

    assert!(!ensure_build_output_ignores(&root).unwrap());

    fs::set_permissions(path, original).unwrap();
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IN-SLOT-BUILD")]
fn concurrent_creators_append_each_rule_once() {
    let (_project, root) = root();
    let root = Arc::new(root);
    let barrier = Arc::new(Barrier::new(8));
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let root = Arc::clone(&root);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                ensure_build_output_ignores(&root)
            })
        })
        .collect();

    let changed = handles
        .into_iter()
        .map(|handle| handle.join().unwrap().unwrap())
        .filter(|changed| *changed)
        .count();
    assert_eq!(changed, 1);
    let bytes = fs::read(root.join(".gitignore")).unwrap();
    assert_eq!(count_line(&bytes, b"**/target/"), 1);
    assert_eq!(count_line(&bytes, b"**/node_modules/"), 1);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IN-SLOT-BUILD")]
fn later_negation_forces_both_managed_rules_back_to_the_effective_suffix() {
    let (project, root) = root();
    fs::create_dir_all(&root).unwrap();
    let path = root.join(".gitignore");
    let before = b"**/target/\n**/node_modules/\n!**/target/\n";
    fs::write(&path, before).unwrap();
    let probe = "vibedeps/org.demo.pkg/1.0.0/target/debug/probe";
    let probe_path = project.path().join(probe);
    fs::create_dir_all(probe_path.parent().unwrap()).unwrap();
    fs::write(&probe_path, b"build output").unwrap();
    let git_available = std::process::Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success());
    if git_available {
        assert!(
            std::process::Command::new("git")
                .args(["init", "--quiet"])
                .current_dir(project.path())
                .status()
                .unwrap()
                .success()
        );
        assert!(
            !git_check_ignored(project.path(), probe),
            "fixture negation did not expose {probe} before repair"
        );
    }

    assert!(ensure_build_output_ignores(&root).unwrap());
    let after = fs::read(&path).unwrap();
    assert!(after.starts_with(before));
    assert_eq!(count_line(&after, b"**/target/"), 2);
    assert_eq!(count_line(&after, b"**/node_modules/"), 2);
    assert!(after.ends_with(b"**/target/\n**/node_modules/\n"));
    if git_available {
        assert!(
            git_check_ignored(project.path(), probe),
            "managed suffix did not re-ignore {probe}"
        );
    }
    assert!(!ensure_build_output_ignores(&root).unwrap());
    assert_eq!(fs::read(path).unwrap(), after);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IN-SLOT-BUILD")]
fn a_directory_at_the_file_path_is_refused() {
    let (_project, root) = root();
    fs::create_dir_all(root.join(".gitignore")).unwrap();
    let error = ensure_build_output_ignores(&root).unwrap_err().to_string();
    assert!(error.contains("non-file"), "{error}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IN-SLOT-BUILD")]
fn a_hardlinked_file_is_refused_without_mutating_its_alias() {
    let (_project, root) = root();
    fs::create_dir_all(&root).unwrap();
    let alias = root.join("operator-ignore");
    fs::write(&alias, b"/operator/\n").unwrap();
    fs::hard_link(&alias, root.join(".gitignore")).unwrap();

    let error = ensure_build_output_ignores(&root).unwrap_err().to_string();
    assert!(error.contains("hardlinks"), "{error}");
    assert_eq!(fs::read(alias).unwrap(), b"/operator/\n");
}

#[test]
fn open_handle_identity_is_stable_and_distinguishes_another_path() {
    let (_project, root) = root();
    fs::create_dir_all(&root).unwrap();
    let first = root.join("first");
    let second = root.join("second");
    fs::write(&first, b"first").unwrap();
    fs::write(&second, b"second").unwrap();

    let first_a = safe_file::open_existing_read(&first).unwrap();
    let first_b = safe_file::open_existing_read(&first).unwrap();
    let second_file = safe_file::open_existing_read(&second).unwrap();
    let identity = safe_file::identity(&first_a).unwrap();
    assert_eq!(identity, safe_file::identity(&first_b).unwrap());
    assert_ne!(identity, safe_file::identity(&second_file).unwrap());
    assert_path_identity(&first, identity).unwrap();
}

#[test]
fn a_substituted_path_is_rejected_against_the_open_handle_identity() {
    let (_project, root) = root();
    fs::create_dir_all(&root).unwrap();
    let path = root.join("first");
    let moved = root.join("moved");
    fs::write(&path, b"first").unwrap();

    let opened = safe_file::open_existing_read(&path).unwrap();
    let identity = safe_file::identity(&opened).unwrap();
    fs::rename(&path, &moved).unwrap();
    fs::write(&path, b"substitute").unwrap();

    let error = assert_path_identity(&path, identity)
        .unwrap_err()
        .to_string();
    assert!(error.contains("pathname no longer names"), "{error}");
    assert_eq!(fs::read(moved).unwrap(), b"first");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IN-SLOT-BUILD")]
fn a_symbolic_link_at_the_file_path_is_refused() {
    let (_project, root) = root();
    fs::create_dir_all(&root).unwrap();
    let target = root.join("operator-ignore");
    fs::write(&target, b"/operator/\n").unwrap();
    let link = root.join(".gitignore");
    if let Err(error) = make_symlink(&target, &link) {
        eprintln!("symlink creation unavailable; skipping refusal probe: {error}");
        return;
    }

    let error = ensure_build_output_ignores(&root).unwrap_err().to_string();
    assert!(error.contains("symbolic-link"), "{error}");
    assert_eq!(fs::read(target).unwrap(), b"/operator/\n");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IN-SLOT-BUILD")]
fn git_ignores_nested_rust_and_node_build_outputs() {
    if !std::process::Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
    {
        eprintln!("git unavailable; skipping check-ignore probe");
        return;
    }
    let project = tempfile::tempdir().unwrap();
    let root = project.path().join("vibedeps");
    ensure_build_output_ignores(&root).unwrap();
    for relative in [
        "vibedeps/org.demo.pkg/1.0.0/target/debug/probe",
        "vibedeps/org.demo.pkg/1.0.0/node_modules/pkg/probe",
    ] {
        let path = project.path().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"build output").unwrap();
    }
    let init = std::process::Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(project.path())
        .status()
        .unwrap();
    assert!(init.success());
    for relative in [
        "vibedeps/org.demo.pkg/1.0.0/target/debug/probe",
        "vibedeps/org.demo.pkg/1.0.0/node_modules/pkg/probe",
    ] {
        let status = std::process::Command::new("git")
            .args(["check-ignore", "--quiet", "--", relative])
            .current_dir(project.path())
            .status()
            .unwrap();
        assert!(status.success(), "not ignored: {relative}");
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IN-SLOT-BUILD")]
fn managed_eof_suffix_reasserts_after_specific_operator_reinclusions() {
    if !std::process::Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
    {
        eprintln!("git unavailable; skipping re-inclusion probe");
        return;
    }
    let project = tempfile::tempdir().unwrap();
    let root = project.path().join("vibedeps");
    fs::create_dir_all(&root).unwrap();
    let ignore = root.join(".gitignore");
    fs::write(
        &ignore,
        b"**/target/\n\
**/node_modules/\n\
!org.demo.pkg/1.0.0/target/\n\
!org.demo.pkg/1.0.0/target/**\n\
!org.demo.pkg/1.0.0/node_modules/\n\
!org.demo.pkg/1.0.0/node_modules/**\n",
    )
    .unwrap();
    let probes = [
        "vibedeps/org.demo.pkg/1.0.0/target/debug/probe",
        "vibedeps/org.demo.pkg/1.0.0/node_modules/pkg/probe",
    ];
    for relative in probes {
        let path = project.path().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"build output").unwrap();
    }
    assert!(
        std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(project.path())
            .status()
            .unwrap()
            .success()
    );
    for relative in probes {
        assert!(
            !git_check_ignored(project.path(), relative),
            "fixture re-inclusion did not expose {relative} before repair"
        );
    }

    assert!(ensure_build_output_ignores(&root).unwrap());
    for relative in probes {
        assert!(
            git_check_ignored(project.path(), relative),
            "not reasserted: {relative}"
        );
    }
    let repaired = fs::read(&ignore).unwrap();
    assert!(!ensure_build_output_ignores(&root).unwrap());
    assert_eq!(fs::read(ignore).unwrap(), repaired);
}

fn git_check_ignored(project: &Path, relative: &str) -> bool {
    std::process::Command::new("git")
        .args(["check-ignore", "--quiet", "--", relative])
        .current_dir(project)
        .status()
        .unwrap()
        .success()
}

fn count_line(bytes: &[u8], expected: &[u8]) -> usize {
    bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| line.strip_suffix(b"\r").unwrap_or(line) == expected)
        .count()
}

#[cfg(unix)]
fn make_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn make_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(target, link)
}
