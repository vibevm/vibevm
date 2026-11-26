//! Public human output, with every child (including init) inside a synthetic HOME.
mod common;

use common::UserScratch;
use std::{fs, path::Path, process::Output};

struct Fixture {
    user: UserScratch,
    home: tempfile::TempDir,
    project: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let this = Self {
            user: UserScratch::new(),
            home: tempfile::tempdir().unwrap(),
            project: tempfile::tempdir().unwrap(),
        };
        let out = this
            .command()
            .args(["init", "--path"])
            .arg(this.project.path())
            .output()
            .unwrap();
        success(&out);
        this
    }

    fn command(&self) -> assert_cmd::Command {
        let mut cmd = self.user.vibe();
        cmd.env("HOME", self.home.path())
            .env("USERPROFILE", self.home.path())
            .env("VIBE_SETTINGS", &self.user.settings)
            .current_dir(self.project.path());
        cmd
    }

    fn user(&self, args: &[&str]) -> Output {
        self.command().arg("user").args(args).output().unwrap()
    }

    fn ov(&self, verb: &str, extra: &[&str]) -> Output {
        self.ov_at(self.project.path(), verb, extra)
    }

    fn ov_at(&self, project: &Path, verb: &str, extra: &[&str]) -> Output {
        self.command()
            .args(["user", "override", verb, "--path"])
            .arg(project)
            .args(extra)
            .output()
            .unwrap()
    }

    fn root(&self) -> std::path::PathBuf {
        let out = self.ov("status", &["--json"]);
        success(&out);
        let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        let root = std::path::PathBuf::from(report["root"].as_str().unwrap());
        assert!(root.starts_with(self.user.settings.join("project-overrides")));
        assert!(root.join("vibe.toml").is_file());
        root
    }

    fn mapping(&self) -> Vec<u8> {
        fs::read(self.user.settings.join("override-bindings.toml")).unwrap()
    }
}

fn success(out: &Output) {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn trailer(out: &Output, what: &str, root: &Path) {
    success(out);
    let text = String::from_utf8_lossy(&out.stdout);
    let last = text.lines().last().unwrap();
    assert!(last.starts_with(&format!("✻ {what} · ")), "{text}");
    assert!(last.ends_with(&root.display().to_string()), "{text}");
    assert_eq!(text.lines().filter(|line| line.starts_with('✻')).count(), 1);
}

#[test]
fn user_status_absent_names_the_init_command() {
    let f = Fixture::new();
    let out = f.user(&["status", "--plain"]);
    success(&out);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.lines()
            .next()
            .unwrap()
            .starts_with("○ user project  absent · "),
        "{text}"
    );
    assert!(
        text.lines().any(|line| line == "  vibe user init"),
        "{text}"
    );
    let quiet = f.user(&["status", "--quiet"]);
    success(&quiet);
    assert_eq!(String::from_utf8_lossy(&quiet.stdout).lines().count(), 1);
    assert_eq!(
        String::from_utf8_lossy(&quiet.stdout).trim(),
        text.lines().next().unwrap()
    );
}

#[test]
fn user_init_ends_with_a_trailer() {
    let f = Fixture::new();
    let root = f.user.settings.join("user-project");
    trailer(
        &f.user(&["init", "--plain"]),
        "initialized the user project",
        &root,
    );
    trailer(
        &f.user(&["init", "--plain"]),
        "user project already initialized",
        &root,
    );
    let quiet = f.user(&["init", "--quiet"]);
    trailer(&quiet, "user project already initialized", &root);
    assert_eq!(String::from_utf8_lossy(&quiet.stdout).lines().count(), 1);
    let out = f.user(&["status", "--plain"]);
    success(&out);
    assert!(
        String::from_utf8_lossy(&out.stdout)
            .trim()
            .starts_with("✓ user project  initialized · ")
    );
}

#[test]
fn override_status_unbound_is_one_row() {
    let f = Fixture::new();
    for flag in ["--plain", "--quiet"] {
        let out = f.ov("status", &[flag]);
        success(&out);
        assert_eq!(
            String::from_utf8_lossy(&out.stdout).trim(),
            "○ override  unbound"
        );
    }
    assert!(!f.user.settings.join("override-bindings.toml").exists());
}

#[test]
fn override_disable_reports_disabled() {
    let f = Fixture::new();
    success(&f.user(&["init"]));
    let init = f.ov("init", &["--alias", "myov", "--plain"]);
    let root = f.root();
    trailer(&init, "initialized override myov", &root);
    trailer(
        &f.ov("init", &["--alias", "myov"]),
        "override myov already initialized",
        &root,
    );
    trailer(&f.ov("disable", &[]), "override myov disabled", &root);
    for flag in ["--plain", "--quiet"] {
        let out = f.ov("status", &[flag]);
        success(&out);
        let text = String::from_utf8_lossy(&out.stdout);
        let row = text.trim();
        assert!(row.starts_with("✓ override myov  initialized · "), "{row}");
        assert!(row.ends_with("[disabled]"), "{row}");
        assert_eq!(text.lines().count(), 1);
    }
    trailer(
        &f.ov("disable", &[]),
        "override myov already disabled",
        &root,
    );
    trailer(&f.ov("enable", &[]), "override myov enabled", &root);
    let quiet = f.ov("enable", &["--quiet"]);
    trailer(&quiet, "override myov already enabled", &root);
    assert_eq!(String::from_utf8_lossy(&quiet.stdout).lines().count(), 1);
}

#[test]
fn override_bind_reports_actual_binding_and_repeat() {
    let f = Fixture::new();
    success(&f.ov("init", &["--alias", "myov"]));
    let root = f.root();
    let second = tempfile::tempdir().unwrap();
    success(
        &f.command()
            .args(["init", "--path"])
            .arg(second.path())
            .output()
            .unwrap(),
    );
    trailer(
        &f.ov_at(second.path(), "bind", &["--alias", "myov"]),
        "bound this project to override myov",
        &root,
    );
    trailer(
        &f.ov_at(second.path(), "bind", &["--alias", "myov"]),
        "this project is already bound to override myov",
        &root,
    );
}

#[test]
fn override_bind_unknown_alias_is_refused_with_code() {
    let f = Fixture::new();
    success(&f.user(&["init"]));
    success(&f.ov("init", &["--alias", "myov"]));
    let before = f.mapping();
    let out = f.ov("bind", &["--alias", "nope"]);
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("✗ override_alias_unknown  override alias nope does not exist"),
        "{err}"
    );
    assert!(
        err.contains("vibe user override init --alias nope"),
        "{err}"
    );
    assert!(
        err.contains("spec://org.vibevm.core/vibevm/common/PROP-074#OVERRIDE-CONTROLS"),
        "{err}"
    );
    assert_eq!(f.mapping(), before);
}

#[test]
fn personal_layer_disable_ends_with_a_trailer() {
    let f = Fixture::new();
    let out = f
        .command()
        .args(["user", "disable", "--path"])
        .arg(f.project.path())
        .output()
        .unwrap();
    trailer(&out, "personal layer disabled", f.project.path());
    let out = f
        .command()
        .args(["user", "enable", "--path"])
        .arg(f.project.path())
        .output()
        .unwrap();
    trailer(&out, "personal layer enabled", f.project.path());
}

#[test]
fn unavailable_bound_root_enable_fails_without_success_and_disable_succeeds() {
    let f = Fixture::new();
    success(&f.ov("init", &["--alias", "myov"]));
    let root = f.root();
    let before = f.mapping();
    // Only move this fixture's inspected root, retaining its bytes for comparison.
    let moved = root.with_file_name("unavailable-fixture");
    assert!(!moved.exists());
    fs::rename(&root, &moved).unwrap();
    let retained = fs::read(moved.join("vibe.toml")).unwrap();
    for flag in ["--plain", "--quiet", "--json"] {
        let out = f.ov("enable", &[flag]);
        assert!(!out.status.success());
        assert!(
            out.stdout.is_empty(),
            "{}",
            String::from_utf8_lossy(&out.stdout)
        );
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains("override_unavailable"), "{err}");
        assert!(!err.contains('✻'), "{err}");
        assert_eq!(f.mapping(), before);
        assert!(!root.exists());
    }
    // Status reaches the final unsuccessful report branch, including its JSON report.
    let out = f.ov("status", &["--plain"]);
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&out.stderr).contains('✻'));
    let json = f.ov("status", &["--json"]);
    assert!(!json.status.success());
    let report: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(report["ok"], false);
    assert_eq!(report["state"], "unavailable");
    assert_eq!(f.mapping(), before);
    trailer(
        &f.ov("disable", &["--plain"]),
        "override myov disabled",
        &root,
    );
    let disabled = f.mapping();
    let quiet = f.ov("disable", &["--quiet"]);
    trailer(&quiet, "override myov already disabled", &root);
    assert_eq!(String::from_utf8_lossy(&quiet.stdout).lines().count(), 1);
    assert_eq!(f.mapping(), disabled);
    assert!(!root.exists());
    assert_eq!(fs::read(moved.join("vibe.toml")).unwrap(), retained);
    let json = f.ov("disable", &["--json"]);
    success(&json);
    let report: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(report["ok"], true);
    assert_eq!(report["state"], "unavailable");
    assert_eq!(report["enabled"], false);
}

#[test]
fn schema_upgrade_alone_does_not_claim_an_action() {
    let f = Fixture::new();
    success(&f.ov("init", &["--alias", "myov"]));
    let root = f.root();
    let mapping = f.user.settings.join("override-bindings.toml");
    for verb in ["init", "bind", "enable"] {
        let text = fs::read_to_string(&mapping)
            .unwrap()
            .replacen("schema = 2", "schema = 1", 1);
        fs::write(&mapping, text).unwrap();
        let args: &[&str] = if verb == "enable" {
            &[]
        } else {
            &["--alias", "myov"]
        };
        let expected = match verb {
            "init" => "override myov already initialized",
            "bind" => "this project is already bound to override myov",
            _ => "override myov already enabled",
        };
        trailer(&f.ov(verb, args), expected, &root);
        assert!(
            fs::read_to_string(&mapping)
                .unwrap()
                .starts_with("schema = 2")
        );
    }
}
