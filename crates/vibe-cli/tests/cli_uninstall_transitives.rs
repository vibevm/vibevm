//! Offline uninstall prunes only the selected root's exclusive locked closure.
mod common;
use std::fs;
use vibe_core::manifest::Lockfile;

#[path = "cli_uninstall_transitives/support.rs"]
mod support;
use support::World;

#[test]
fn exclusive_chain_removed_unrelated_orphan_preserved() {
    let w = World::new(
        &["a"],
        &[("a", &["b"]), ("b", &["c"]), ("c", &[]), ("orphan", &[])],
    );
    w.remove("org.test/a", w.root());
    assert_eq!(w.names(), ["orphan"]);
    for name in ["a", "b", "c"] {
        assert!(!w.slot(name).exists());
    }
    assert!(w.slot("orphan").join("payload").exists());
}
#[test]
fn diamond_shared_root_and_direct_child_preserved() {
    let w = World::new(
        &["a", "other", "direct"],
        &[
            ("a", &["left", "right", "direct"]),
            ("left", &["shared"]),
            ("right", &["shared"]),
            ("shared", &[]),
            ("other", &["shared"]),
            ("direct", &[]),
        ],
    );
    w.remove("org.test/a", w.root());
    assert_eq!(w.names(), ["shared", "other", "direct"]);
    for name in ["a", "left", "right"] {
        assert!(!w.slot(name).exists());
    }
}
#[test]
fn unrelated_orphan_dependency_stays_valid() {
    let w = World::new(&["a"], &[("a", &["b"]), ("b", &[]), ("orphan", &["b"])]);
    w.remove("org.test/a", w.root());
    assert_eq!(w.names(), ["b", "orphan"]);
}
#[test]
fn member_keeps_selected_root_and_closure() {
    let w = World::new(&["a"], &[("a", &["b"]), ("b", &[])]);
    w.manifest(w.root(), &["a"], "[workspace]\nmembers = [\"member\"]");
    w.manifest(&w.root().join("member"), &["a"], "");
    let result = w
        .user
        .vibe()
        .args([
            "uninstall",
            "org.test/a",
            "--json",
            "--assume-yes",
            "--path",
        ])
        .arg(w.root())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report: serde_json::Value = serde_json::from_slice(&result).unwrap();
    assert_eq!(report["package_removed"], false);
    assert_eq!(report["removed_slot"], "");
    assert_eq!(w.names(), ["a", "b"]);
    assert!(w.slot("a").exists());
}
#[test]
fn selected_member_removal_preserves_other_member_root() {
    let w = World::new(&["other"], &[("a", &["b"]), ("b", &[]), ("other", &[])]);
    w.manifest(w.root(), &["other"], "[workspace]\nmembers = [\"member\"]");
    w.manifest(&w.root().join("member"), &["a"], "");
    w.remove("org.test/a", &w.root().join("member"));
    assert_eq!(w.names(), ["other"]);
    assert!(
        !fs::read_to_string(w.root().join("member/vibe.toml"))
            .unwrap()
            .contains("org.test/a")
    );
}
#[test]
fn pure_transitive_refused_without_writes() {
    let w = World::new(&["a"], &[("a", &["b"]), ("b", &[])]);
    let before = w.snapshot();
    w.user
        .vibe()
        .args(["uninstall", "org.test/b", "--json", "--path"])
        .arg(w.root())
        .assert()
        .failure();
    assert_eq!(before, w.snapshot());
    assert!(w.slot("b").exists());
}
#[test]
fn mixed_in_place_requires_explicit_opt_in_before_any_mutation() {
    let w = World::new(&["a"], &[("a", &["b"]), ("b", &[])]);
    let path = w.root().join("vibe.lock");
    let mut lock = Lockfile::read(&path).unwrap();
    lock.packages[1].materialization = vibe_core::manifest::Materialization::InPlace;
    lock.write(&path).unwrap();
    let before = w.snapshot();
    w.user
        .vibe()
        .args(["uninstall", "org.test/a", "--json", "--path"])
        .arg(w.root())
        .assert()
        .failure();
    assert_eq!(before, w.snapshot());
    assert!(w.slot("a").exists());
}
#[test]
fn malformed_graph_and_file_slot_refused_without_writes() {
    for malformed in [true, false] {
        let w = World::new(&["a"], &[("a", &["b"]), ("b", &[])]);
        if malformed {
            let path = w.root().join("vibe.lock");
            let text = fs::read_to_string(&path)
                .unwrap()
                .replace("org.test/b@=1.0.0", "org.test/b@=2.0.0");
            fs::write(path, text).unwrap();
        } else {
            fs::remove_dir_all(w.slot("b")).unwrap();
            fs::write(w.slot("b"), "file").unwrap();
        }
        let before = w.snapshot();
        w.user
            .vibe()
            .args(["uninstall", "org.test/a", "--assume-yes", "--path"])
            .arg(w.root())
            .assert()
            .failure();
        assert_eq!(before, w.snapshot());
        assert!(w.slot("a").exists());
    }
}
#[test]
fn path_source_authored_tree_preserved() {
    let w = World::new(&["a"], &[("a", &["b"]), ("b", &[])]);
    let source = w.root().join("authored");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("keep"), "owner").unwrap();
    let text = "[project]\nname = \"test\"\nversion = \"0.1.0\"\n[requires.packages]\n\"org.test/a\" = { path = \"authored\", version = \"=1.0.0\" }\n";
    fs::write(w.root().join("vibe.toml"), text).unwrap();
    w.remove("org.test/a", w.root());
    assert!(w.names().is_empty());
    assert_eq!(fs::read_to_string(source.join("keep")).unwrap(), "owner");
}

#[cfg(windows)]
#[test]
fn later_stage_failure_rolls_back_earlier_staged_slot_and_world() {
    use std::os::windows::fs::OpenOptionsExt;
    let w = World::new(&["a"], &[("a", &["b"]), ("b", &[])]);
    // Open the second directory without FILE_SHARE_DELETE: Windows refuses
    // its rename after the first slot has already staged successfully.
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(3)
        .custom_flags(0x02000000)
        .open(w.slot("b"))
        .unwrap();
    let before = w.snapshot();
    w.user
        .vibe()
        .args(["uninstall", "org.test/a", "--assume-yes", "--path"])
        .arg(w.root())
        .assert()
        .failure();
    assert_eq!(before, w.snapshot());
    assert_eq!(
        fs::read_to_string(w.slot("a").join("payload")).unwrap(),
        "a"
    );
    assert!(w.slot("b").exists());
    drop(held);
}
#[test]
fn outside_link_slot_is_refused_before_any_mutation() {
    let w = World::new(&["a"], &[("a", &["b"]), ("b", &[])]);
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("owner"), "untouched").unwrap();
    fs::remove_dir_all(w.slot("b")).unwrap();
    #[cfg(windows)]
    {
        let result = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(w.slot("b"))
            .arg(outside.path())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.path(), w.slot("b")).unwrap();
    let before = w.snapshot();
    w.user
        .vibe()
        .args(["uninstall", "org.test/a", "--assume-yes", "--path"])
        .arg(w.root())
        .assert()
        .failure();
    assert_eq!(before, w.snapshot());
    assert!(w.slot("a").exists());
    assert_eq!(
        fs::read_to_string(outside.path().join("owner")).unwrap(),
        "untouched"
    );
    #[cfg(windows)]
    fs::remove_dir(w.slot("b")).unwrap();
    #[cfg(unix)]
    fs::remove_file(w.slot("b")).unwrap();
}

#[test]
fn corrupt_lock_graph_variants_refuse_without_writes() {
    for variant in 0..6 {
        let w = World::new(&["a"], &[("a", &["b"]), ("b", &[])]);
        let path = w.root().join("vibe.lock");
        let mut lock = Lockfile::read(&path).unwrap();
        match variant {
            0 => {
                let duplicate = lock.packages[1].clone();
                lock.packages.push(duplicate);
            }
            1 => {
                let duplicate = lock.packages[0].dependencies[0].clone();
                lock.packages[0].dependencies.push(duplicate);
            }
            2 => {
                lock.packages.pop();
            }
            3 => {
                lock.packages[0].dependencies[0] =
                    vibe_core::PackageRef::parse("b@=1.0.0").unwrap();
            }
            4 => {
                lock.packages[0].dependencies[0] =
                    vibe_core::PackageRef::parse("org.test/b@^1.0.0").unwrap();
            }
            _ => {
                let duplicate = lock.meta.root_dependencies[0].clone();
                lock.meta.root_dependencies.push(duplicate);
            }
        }
        lock.write(path).unwrap();
        let before = w.snapshot();
        w.user
            .vibe()
            .args(["uninstall", "org.test/a", "--assume-yes", "--path"])
            .arg(w.root())
            .assert()
            .failure();
        assert_eq!(before, w.snapshot());
        assert!(w.slot("a").exists());
        assert!(w.slot("b").exists());
    }
}
#[test]
fn missing_slot_is_cleaned_from_lock_without_touching_survivor() {
    let w = World::new(
        &["a", "other"],
        &[("a", &["b"]), ("b", &[]), ("other", &[])],
    );
    fs::remove_dir_all(w.slot("b")).unwrap();
    w.remove("org.test/a", w.root());
    assert_eq!(w.names(), ["other"]);
    assert!(w.slot("other").exists());
}

#[path = "cli_uninstall_transitives/declarations.rs"]
mod declarations;
