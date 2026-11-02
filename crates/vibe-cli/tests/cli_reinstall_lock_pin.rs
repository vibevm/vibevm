//! `vibe reinstall --force` reproduces `vibe.lock`; it never re-records it
//! (BACKLOG B-196).
//!
//! The walk this pins: install a package from a `file://` registry, edit the
//! package's bytes at the same version, and force a reinstall. The refetch
//! used to copy the edited bytes into `vibedeps/` and stamp the slot with
//! their hash while the lock kept the old one, and `vibe check` saw nothing.
//! Now the forced reinstall refuses with `lock_hash_mismatch` before
//! anything reaches `vibedeps/`, `vibe update` is the command that moves the
//! lock, and `vibe check` reports a slot whose hash the lock does not pin.
//!
//! Isolation: every command is built by [`UserScratch`], whose
//! `$VIBE_SETTINGS` holds the machine `registry.toml` under test.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::UserScratch;
use serde_json::Value;
use vibe_core::manifest::Lockfile;

fn file_url(path: &Path) -> String {
    format!("file:///{}", path.to_string_lossy().replace('\\', "/"))
}

/// (Re)write `org.example/my-skills@0.1.0` in the monorepo registry shape,
/// its one flow file carrying `body`.
fn write_package(registry: &Path, body: &str) {
    let dir = registry
        .join("org.example")
        .join("my-skills")
        .join("v0.1.0");
    let flow = dir.join(common::spec_rel("flows/my-skills"));
    fs::create_dir_all(&flow).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        "[package]\ngroup = \"org.example\"\nname = \"my-skills\"\n\
         kind = \"flow\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(flow.join("RULES.md"), format!("# rules\n\n{body}\n")).unwrap();
}

fn slot_dir(project: &Path) -> PathBuf {
    project.join(common::slot_dir("org.example.my-skills", "0.1.0"))
}

fn slot_rules(project: &Path) -> String {
    let path = project.join(common::slot_rel(
        "org.example.my-skills",
        "0.1.0",
        common::spec_rel("flows/my-skills/RULES.md"),
    ));
    fs::read_to_string(path).unwrap()
}

fn locked_hash(project: &Path) -> String {
    let lock = Lockfile::read(project.join(Lockfile::FILENAME)).unwrap();
    lock.packages[0].content_hash.as_str().to_string()
}

/// A project installed from a machine-file `file://` registry, and the
/// registry root so the test can edit the package afterwards.
fn installed(user: &UserScratch, outer: &Path, project: &Path) -> PathBuf {
    let registry = outer.join("registry");
    write_package(&registry, "first");
    fs::write(
        user.settings.join("registry.toml"),
        format!(
            "[[registry]]\nname = \"machine\"\nurl = \"{}\"\n",
            file_url(&registry)
        ),
    )
    .unwrap();
    user.init_project(project);
    user.vibe()
        .args(["install", "org.example/my-skills", "--assume-yes", "--path"])
        .arg(project)
        .assert()
        .success();
    registry
}

/// `vibe check --json` on `project`: its exit success and its findings.
fn check(user: &UserScratch, project: &Path) -> (bool, Vec<Value>) {
    let output = user
        .vibe()
        .args(["check", "--json", "--path"])
        .arg(project)
        .output()
        .unwrap();
    let report: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout is not JSON ({error}): {}",
            String::from_utf8_lossy(&output.stdout)
        )
    });
    let findings = report["findings"].as_array().cloned().unwrap_or_default();
    (output.status.success(), findings)
}

fn lock_slot_findings(findings: &[Value]) -> Vec<&Value> {
    findings
        .iter()
        .filter(|finding| finding["check"] == "lock_slot_hash")
        .collect()
}

#[test]
fn a_forced_reinstall_refuses_bytes_the_lock_does_not_pin_and_update_moves_the_lock() {
    let user = UserScratch::new();
    let outer = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let registry = installed(&user, outer.path(), project.path());
    let lock_before = fs::read(project.path().join(Lockfile::FILENAME)).unwrap();
    let record_before = fs::read(slot_dir(project.path()).join(".vibe-slot.toml")).unwrap();
    assert!(slot_rules(project.path()).contains("first"));

    // The author edits the package at the same version.
    write_package(&registry, "second");

    let out = user
        .vibe()
        .args(["reinstall", "--force", "--assume-yes", "--json"])
        .arg(project.path())
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success(),
        "the reinstall must refuse:\n{stderr}"
    );
    let envelope: Value = stderr
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|value| value["ok"] == false)
        .unwrap_or_else(|| panic!("no error envelope on stderr:\n{stderr}"));
    assert_eq!(envelope["code"], "lock_hash_mismatch", "{envelope}");
    let message = envelope["error"].as_str().unwrap();
    assert!(message.contains("org.example/my-skills@0.1.0"), "{message}");
    assert!(message.contains(&locked_hash(project.path())), "{message}");
    assert_eq!(
        envelope["locked_hash"],
        locked_hash(project.path()).as_str()
    );
    assert_ne!(envelope["source_hash"], envelope["locked_hash"]);
    assert!(
        message.contains("vibe update org.example/my-skills"),
        "{message}"
    );

    // Nothing moved: the slot, its record and the lock are as installed.
    assert!(slot_rules(project.path()).contains("first"));
    assert_eq!(
        fs::read(slot_dir(project.path()).join(".vibe-slot.toml")).unwrap(),
        record_before
    );
    assert_eq!(
        fs::read(project.path().join(Lockfile::FILENAME)).unwrap(),
        lock_before
    );

    // The human refusal leads with the code and gives the fix line.
    let human = user
        .vibe()
        .args(["reinstall", "--force", "--assume-yes"])
        .arg(project.path())
        .output()
        .unwrap();
    let human_err = String::from_utf8_lossy(&human.stderr);
    assert!(!human.status.success());
    assert!(human_err.contains("lock_hash_mismatch"), "{human_err}");
    assert!(
        human_err.contains("vibe update org.example/my-skills"),
        "{human_err}"
    );

    // `vibe update` re-resolves and records the new bytes in both places.
    user.vibe()
        .args(["update", "org.example/my-skills", "--assume-yes", "--path"])
        .arg(project.path())
        .assert()
        .success();
    assert!(slot_rules(project.path()).contains("second"));
    let (_, findings) = check(&user, project.path());
    assert!(
        lock_slot_findings(&findings).is_empty(),
        "after update the slot and the lock agree: {findings:?}"
    );

    // With the lock re-recorded, a forced reinstall reproduces it again.
    user.vibe()
        .args(["reinstall", "--force", "--assume-yes"])
        .arg(project.path())
        .assert()
        .success();
}

#[test]
fn check_reports_a_slot_whose_hash_the_lock_does_not_pin() {
    let user = UserScratch::new();
    let outer = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    installed(&user, outer.path(), project.path());
    let (_, clean) = check(&user, project.path());
    assert!(lock_slot_findings(&clean).is_empty(), "{clean:?}");

    // A hand edit of the lock row's hash.
    let lock_path = project.path().join(Lockfile::FILENAME);
    let real = locked_hash(project.path());
    let forged = format!("sha256:{}", "0".repeat(64));
    let text = fs::read_to_string(&lock_path).unwrap();
    fs::write(&lock_path, text.replace(&real, &forged)).unwrap();

    let (ok, findings) = check(&user, project.path());
    assert!(!ok, "a disagreeing slot is an error: {findings:?}");
    let hits = lock_slot_findings(&findings);
    assert_eq!(hits.len(), 1, "{findings:?}");
    let finding = hits[0];
    assert_eq!(finding["severity"], "error");
    assert_eq!(finding["field"], "package[0].content_hash");
    assert!(
        finding["package"]
            .as_str()
            .is_some_and(|key| key.contains('/') && key.contains('@')),
        "{finding}"
    );
    assert!(
        finding["path"]
            .as_str()
            .unwrap()
            .ends_with("org.example.my-skills/0.1.0"),
        "{finding}"
    );
    let message = finding["message"].as_str().unwrap();
    assert!(
        message.contains(&real) && message.contains(&forged),
        "{message}"
    );
    assert!(
        message.contains("vibe update org.example/my-skills"),
        "{message}"
    );
}
