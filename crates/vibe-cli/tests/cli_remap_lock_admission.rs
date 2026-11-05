//! Project lock admission precedes remap consent and fixes actual layer precedence.
#![cfg(target_os = "linux")]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#layers-packages");
mod common;

use common::UserScratch;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;
use vibe_core::manifest::{CURRENT_SCHEMA_VERSION, Lockfile};
use vibe_core::remap::{ConsentLedger, InstalledRemaps, OfferState};

const PROJECT: &str = "org.example/project-route";
const USER: &str = "org.example/user-route";
const ORIGIN: &str = "https://remap-lock.invalid/artifact";
const PROJECT_TARGET: &str = "file:///project-artifacts/artifact";
const USER_TARGET: &str = "file:///user-artifacts/artifact";

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}
fn ok(output: Output) -> Value {
    assert!(
        output.status.success(),
        "status={} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("public JSON response")
}
struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    project: PathBuf,
    user: UserScratch,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().to_owned();
        let project = root.join("project");
        write(
            &project.join("vibe.toml"),
            "[project]\nname='remap-lock'\nversion='0.1.0'\n",
        );
        fs::create_dir(root.join("home")).unwrap();
        let f = Self {
            _temp: temp,
            root,
            project,
            user: UserScratch::new(),
        };
        for (key, priority, target) in [
            (PROJECT, 0, "file:///project-artifacts/"),
            (USER, 100, "file:///user-artifacts/"),
        ] {
            let (group, name) = key.split_once('/').unwrap();
            let package = f
                .root
                .join("registry")
                .join(group)
                .join(name)
                .join("v1.0.0");
            write(
                &package.join("vibe.toml"),
                &format!(
                    "[package]\ngroup='{group}'\nname='{name}'\nversion='1.0.0'\nkind='remap'\n\
                 [remap]\ndescription='local fixture offer'\npriority={priority}\n\
                 [[remap.rows]]\nprefix='https://remap-lock.invalid/'\nurl='{target}'\nreason='fixture precedence'\n"
                ),
            );
            ok(f.run(&[
                "install",
                "-g",
                key,
                "--registry",
                f.root.join("registry").to_str().unwrap(),
            ]));
        }
        f.assert_offered();
        f
    }
    fn run_at(&self, path: &Path, args: &[&str]) -> Output {
        let mut command = self.user.vibe();
        command
            .current_dir(path)
            .env("HOME", self.root.join("home"))
            .env("USERPROFILE", self.root.join("home"));
        for name in [
            "VIBE_REMAP",
            "VIBEVM_RELEASE_ORIGIN",
            "VIBE_AGENT_COMMAND",
            "VIBE_AGENT_ARGS",
            "VIBE_INVOKED_BY",
        ] {
            command.env_remove(name);
        }
        command
            .args(args)
            .args(["--offline", "--unattended", "--json"])
            .output()
            .unwrap()
    }
    fn run(&self, args: &[&str]) -> Output {
        self.run_at(&self.project, args)
    }
    fn state_bytes(&self) -> BTreeMap<String, Vec<u8>> {
        ["installed.json", "consent.json", "providers.json"]
            .into_iter()
            .map(|name| {
                (
                    name.into(),
                    fs::read(self.user.settings.join("remap").join(name)).unwrap(),
                )
            })
            .collect()
    }
    fn assert_offered(&self) {
        let installed = InstalledRemaps::load(&self.user.settings).unwrap();
        let ledger = ConsentLedger::load(&self.user.settings).unwrap();
        for key in [PROJECT, USER] {
            let hash = &installed.find(key).unwrap().package.content_hash;
            assert_eq!(ledger.state(key, hash), OfferState::Offered);
        }
    }
    fn accept(&self) {
        ok(self.run(&["remap", "accept", PROJECT, USER]));
    }
    fn lock_text(&self, epoch: u32) -> String {
        let installed = InstalledRemaps::load(&self.user.settings).unwrap();
        let package = &installed.find(PROJECT).unwrap().package;
        format!(
            "[meta]\ngenerated_by='remap fixture'\ngenerated_at='2026-11-05T00:00:00Z'\nschema_version={epoch}\n\
            [[remap]]\npackage='{PROJECT}'\nversion='{}'\ncontent_hash='{}'\n",
            package.version, package.content_hash
        )
    }
    fn explain(&self) -> Value {
        ok(self.run(&["remap", "explain", ORIGIN]))
    }
    fn refuse_before_consent(&self) {
        let state = self.state_bytes();
        let lock = self.project.join("vibe.lock");
        let lock_before = fs::read(&lock).ok();
        let output = self.run(&[
            "install",
            "--path",
            self.project.to_str().unwrap(),
            "--accept-remap",
        ]);
        let text = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!output.status.success(), "unreadable lock accepted: {text}");
        assert!(text.contains("PROP-066#layers-packages"), "{text}");
        assert!(text.contains("project lock"), "{text}");
        assert_eq!(
            self.state_bytes(),
            state,
            "consent/installed/provider bytes changed before refusal"
        );
        assert_eq!(fs::read(lock).ok(), lock_before);
        self.assert_offered();
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-066#layers-packages")]
fn faithful_schema7_and_current_project_layer_beat_higher_priority_user_offer() {
    let f = Fixture::new();
    write(&f.project.join("vibe.lock"), &f.lock_text(7));
    let offered = f.state_bytes();
    ok(f.run(&[
        "install",
        "--path",
        f.project.to_str().unwrap(),
        "--accept-remap",
    ]));
    assert_ne!(
        f.state_bytes(),
        offered,
        "valid schema7 permits actual consent saving"
    );
    for epoch in [7, CURRENT_SCHEMA_VERSION] {
        let text = f.lock_text(epoch);
        write(&f.project.join("vibe.lock"), &text);
        let parsed = Lockfile::read(f.project.join("vibe.lock")).unwrap();
        assert_eq!(parsed.meta.schema_version, epoch);
        let response = f.explain();
        assert_eq!(response["chosen"], PROJECT_TARGET);
        assert_eq!(response["matches"][0]["layer"], "project_package");
        assert_eq!(response["matches"][0]["source"], format!("{PROJECT}@1.0.0"));
        assert_eq!(response["matches"][1]["layer"], "user_package");
        assert_eq!(
            fs::read_to_string(f.project.join("vibe.lock")).unwrap(),
            text
        );
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-066#layers-packages")]
fn missing_lock_and_directory_without_project_keep_the_user_layer() {
    let f = Fixture::new();
    f.accept();
    let before = f.state_bytes();
    let response = f.explain();
    assert_eq!(response["chosen"], USER_TARGET);
    assert!(
        response["matches"]
            .as_array()
            .unwrap()
            .iter()
            .all(|m| m["layer"] == "user_package")
    );
    let no_project = f.root.join("no-project");
    fs::create_dir(&no_project).unwrap();
    let response = ok(f.run_at(&no_project, &["remap", "explain", ORIGIN]));
    assert_eq!(response["chosen"], USER_TARGET);
    assert_eq!(f.state_bytes(), before);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-066#consent")]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PERSISTENCE")]
fn malformed_future_and_old_epoch_rich_locks_refuse_before_accept_all_is_saved() {
    let f = Fixture::new();
    let lock = f.project.join("vibe.lock");
    for text in [
        "not valid [ toml".to_owned(),
        f.lock_text(CURRENT_SCHEMA_VERSION + 1),
        f.lock_text(0),
    ] {
        write(&lock, &text);
        assert!(Lockfile::read(&lock).is_err());
        f.refuse_before_consent();
    }
    // Structurally decoded rich data must not reach the old1..6 Unsupported
    // classification. It is intentionally invalid lock state, not a ready result.
    for epoch in [6, 7] {
        let text = format!(
            "{}\n[configurations.'org.example/configured']\nfeatures=[]\nactive_dependencies=[]\n\
            [configurations.'org.example/configured'.consumers]\n\
            [configurations.'org.example/configured'.definition]\n\
            [configurations.'org.example/configured'.declarations]\n\
            [configurations.'org.example/configured'.dependency_causes]\n",
            f.lock_text(epoch)
        );
        write(&lock, &text);
        assert!(
            matches!(
                Lockfile::read(&lock),
                Err(vibe_core::Error::InvalidLockfile { .. })
            ),
            "rich state must not be classified as regenerable legacy state"
        );
        f.refuse_before_consent();
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-066#consent")]
fn nonregular_and_broken_symlink_locks_are_not_absent_and_do_not_save_consent() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let lock = f.project.join("vibe.lock");
    fs::create_dir(&lock).unwrap();
    f.refuse_before_consent();
    assert!(lock.is_dir());
    fs::remove_dir(&lock).unwrap();
    let missing = f.root.join("missing-lock");
    symlink(&missing, &lock).unwrap();
    f.refuse_before_consent();
    assert_eq!(fs::read_link(&lock).unwrap(), missing);
    fs::remove_file(&lock).unwrap();
    let target = f.root.join("outside-lock");
    let bytes = f.lock_text(7);
    write(&target, &bytes);
    symlink(&target, &lock).unwrap();
    f.refuse_before_consent();
    assert_eq!(fs::read_link(&lock).unwrap(), target);
    assert_eq!(fs::read_to_string(target).unwrap(), bytes);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-066#consent")]
fn legacy_schema6_install_retains_regeneration_and_historical_empty_remap_layer() {
    let f = Fixture::new();
    write(&f.project.join("vibe.lock"), &f.lock_text(6));
    assert!(matches!(
        Lockfile::read(f.project.join("vibe.lock")),
        Err(vibe_core::Error::UnsupportedLockfile { found: 6, .. })
    ));
    let before = f.state_bytes();
    ok(f.run(&[
        "install",
        "--path",
        f.project.to_str().unwrap(),
        "--accept-remap",
    ]));
    assert_ne!(
        f.state_bytes(),
        before,
        "valid legacy control must actually exercise consent saving"
    );
    assert_eq!(
        Lockfile::read(f.project.join("vibe.lock"))
            .unwrap()
            .meta
            .schema_version,
        7,
        "successful empty regeneration must publish a supported lock before callback"
    );
    let response = f.explain();
    assert_eq!(
        response["chosen"], USER_TARGET,
        "old unsupported epochs had an empty project remap layer"
    );
    assert_eq!(response["matches"][0]["layer"], "user_package");
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#LF-CONFIGURED-STATE"
)]
fn ordinary_empty_install_does_not_create_an_absent_lock() {
    let f = Fixture::new();
    f.accept();
    assert!(!f.project.join("vibe.lock").exists());
    let output = f.run(&["install", "--path", f.project.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!f.project.join("vibe.lock").exists());
}
