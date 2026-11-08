//! `type = "monorepo"` registries over SSH (PROP-002 `#registry-type`):
//! resolution through the owned loopback sshd, lock provenance, replay at
//! the held commit, `vibe update --sources-only` onto a monorepo, and the
//! Locator's mirror chain. Owned numeric-loopback SSH, never operator keys.
#![cfg(unix)]

mod common;

use std::os::unix::fs::PermissionsExt;
use std::{
    fs,
    path::{Path, PathBuf},
};

use common::ssh_monorepo::{Fixture, write_package};

fn lock_row(project: &Path) -> toml::Value {
    let lock: toml::Value =
        toml::from_str(&fs::read_to_string(project.join("vibe.lock")).unwrap()).unwrap();
    lock["package"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"].as_str() == Some("demo"))
        .cloned()
        .expect("demo lock row")
}

fn installed_payload(project: &Path) -> String {
    let deps = project.join("vibevm/vibedeps");
    let slot = fs::read_dir(&deps)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.file_name().unwrap().to_string_lossy().contains("demo"))
        .expect("demo slot");
    let mut stack = vec![slot];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().is_some_and(|name| name == "payload") {
                return fs::read_to_string(path).unwrap();
            }
        }
    }
    panic!("no payload in the demo slot");
}

/// The mandate scenario: one SSH repository holds every package, reached
/// through the user's ordinary `~/.ssh/config`; the lock records the
/// repository, the package directory, the branch and the exact commit, and
/// a held row replays from the store with no contact.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-type"
)]
fn monorepo_over_ordinary_ssh_config_records_commit_provenance_and_replays() {
    let fixture = Fixture::new();
    let home = PathBuf::from(fixture.facts["home"].as_str().unwrap());
    let ssh = home.join(".ssh");
    fs::create_dir_all(&ssh).unwrap();
    fs::set_permissions(&ssh, fs::Permissions::from_mode(0o700)).unwrap();
    fs::copy(&fixture.config, ssh.join("config")).unwrap();
    fs::set_permissions(ssh.join("config"), fs::Permissions::from_mode(0o600)).unwrap();
    // The controlled transport (selected by any exclusion) captures
    // `$HOME/.ssh/config` explicitly.
    let project = fixture.project(
        "ordinary",
        &format!(
            "[source_policy]\nexclude_hosting=['host:127.0.0.2']\n{}",
            fixture.monorepo_row()
        ),
    );
    let mut install = fixture.command(&project);
    install.env_remove("GIT_SSH_COMMAND");
    let output = install.args(["--unattended", "install"]).output().unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let row = lock_row(&project);
    assert_eq!(row["registry"].as_str(), Some("corp"));
    assert_eq!(row["source_kind"].as_str(), Some("registry"));
    assert_eq!(
        row["source_url"].as_str().unwrap(),
        format!("{}#org.fixture/demo/v1.0.0", fixture.address())
    );
    assert_eq!(row["source_ref"].as_str(), Some("main"));
    assert_eq!(
        row["resolved_commit"].as_str().unwrap(),
        fixture.main_commit()
    );
    assert_eq!(installed_payload(&project), "monorepo v1.0.0\n");

    // Held replay with the registry now unreachable: store, no contact.
    let lock = fs::read(project.join("vibe.lock")).unwrap();
    let contacts = fixture.contacts();
    let manifest = fs::read_to_string(project.join("vibe.toml")).unwrap();
    fs::write(
        project.join("vibe.toml"),
        manifest.replace(&fixture.address(), "ssh://unused.invalid/vibe.git"),
    )
    .unwrap();
    let mut replay = fixture.command(&project);
    replay.env_remove("GIT_SSH_COMMAND");
    let output = replay.args(["--unattended", "install"]).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(project.join("vibe.lock")).unwrap(), lock);
    assert_eq!(fixture.contacts(), contacts);
}

/// A forced refetch reads the held directory at the held commit even after
/// the branch moved and rewrote that directory.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
fn forced_refetch_replays_the_held_commit_not_the_branch_tip() {
    let fixture = Fixture::new();
    let project = fixture.project("replay", &fixture.monorepo_row());
    fixture.vibe(&project, &["--unattended", "install"]);
    let held = lock_row(&project);

    // Move `main`: v1.0.0's bytes change upstream.
    let work = fixture.root().join("monorepo-work");
    write_package(&work, "demo", "1.0.0", "rewritten upstream\n");
    fixture.commit_all(&work, "rewrite");
    fixture.git(
        Some(&work),
        &[
            "push",
            "-q",
            fixture.bare().to_str().unwrap(),
            "HEAD:refs/heads/main",
        ],
    );
    assert_ne!(
        fixture.main_commit(),
        held["resolved_commit"].as_str().unwrap()
    );

    let contacts = fixture.contacts();
    fixture.vibe(&project, &["reinstall", "--force", "--assume-yes"]);
    assert!(
        fixture.contacts() > contacts,
        "--force fetched from the source"
    );
    assert_eq!(lock_row(&project), held);
    assert_eq!(installed_payload(&project), "monorepo v1.0.0\n");

    // An explicit version move stays on the held repository and branch.
    let manifest = fs::read_to_string(project.join("vibe.toml")).unwrap();
    fs::write(
        project.join("vibe.toml"),
        manifest.replace("'=1.0.0'", "'^1'"),
    )
    .unwrap();
    fixture.vibe(&project, &["--unattended", "update"]);
    let moved = lock_row(&project);
    assert_eq!(moved["version"].as_str(), Some("1.1.0"));
    assert_eq!(
        moved["source_url"].as_str().unwrap(),
        format!("{}#org.fixture/demo/v1.1.0", fixture.address())
    );
    assert_eq!(moved["source_ref"].as_str(), Some("main"));
    assert_eq!(
        moved["resolved_commit"].as_str().unwrap(),
        fixture.main_commit()
    );
}

/// `vibe update --sources-only` moves a lock that points at another
/// registry onto the monorepo, keeping the version and the bytes.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
fn sources_only_moves_a_held_row_onto_the_monorepo() {
    let fixture = Fixture::new();
    let drop = fixture.root().join("drop");
    write_package(&drop, "demo", "1.0.0", "monorepo v1.0.0\n");
    let project = fixture.project(
        "migrate",
        &format!(
            "[[registry]]\nname='drop'\nurl='file://{}'\n",
            drop.display()
        ),
    );
    fixture.vibe(&project, &["--unattended", "install"]);
    let before = lock_row(&project);
    assert_eq!(before["registry"].as_str(), Some("drop"));

    fs::write(
        project.join("vibe.toml"),
        format!(
            "[project]\nname='consumer'\nversion='1.0.0'\n[requires.packages]\n'org.fixture/demo'='=1.0.0'\n{}",
            fixture.monorepo_row()
        ),
    )
    .unwrap();
    fixture.vibe(&project, &["--unattended", "update", "--sources-only"]);
    let after = lock_row(&project);
    assert_eq!(after["registry"].as_str(), Some("corp"));
    assert_eq!(after["version"], before["version"]);
    assert_eq!(after["content_hash"], before["content_hash"]);
    assert_eq!(
        after["source_url"].as_str().unwrap(),
        format!("{}#org.fixture/demo/v1.0.0", fixture.address())
    );
    assert_eq!(
        after["resolved_commit"].as_str().unwrap(),
        fixture.main_commit()
    );
}

/// The clone goes through the Locator: a remap row sends the unreachable
/// canonical repository to the SSH one, while the lock keeps the canonical
/// url; `--offline` never contacts the monorepo.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-066#locator")]
fn monorepo_clone_goes_through_remap_rows_and_offline_never_contacts_it() {
    let fixture = Fixture::new();
    let canonical = format!("file://{}/absent/vibe.git", fixture.root().display());
    let project = fixture.project(
        "remapped",
        &format!("[[registry]]\nname='corp'\ntype='monorepo'\nurl='{canonical}'\n"),
    );
    fs::create_dir_all(project.join(".vibe")).unwrap();
    fs::write(
        project.join(".vibe/settings.local.toml"),
        format!(
            "[[remap]]\nprefix = '{canonical}'\nurl = '{}'\n",
            fixture.address()
        ),
    )
    .unwrap();
    fixture.vibe(&project, &["--unattended", "install"]);
    let row = lock_row(&project);
    assert_eq!(
        row["source_url"].as_str().unwrap(),
        format!("{canonical}#org.fixture/demo/v1.0.0")
    );
    assert!(
        fixture.contacts() > 0,
        "the remapped repository served the clone"
    );

    let offline = fixture.project("offline", &fixture.monorepo_row());
    let contacts = fixture.contacts();
    let output = fixture
        .command(&offline)
        .args(["--offline", "--unattended", "install"])
        .output()
        .unwrap();
    // Whatever the warm store can serve, the monorepo is never contacted.
    drop(output);
    assert_eq!(fixture.contacts(), contacts);
}
