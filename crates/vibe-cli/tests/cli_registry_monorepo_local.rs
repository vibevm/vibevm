//! A `type = "monorepo"` registry on this machine's filesystem while source
//! policy excludes hostings (PROP-073 `#SOURCE-POLICY`): the exclusion
//! selects the controlled transport, which reads a local repository over
//! Git's `file` transport — there is no host to exclude. Covers both the
//! `file://` url and the bare absolute path, `vibe install` and
//! `vibe update --sources-only`, and that `--offline` reads a local monorepo
//! but never a remote one (PROP-002 `#registry-type`).
#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct Site {
    root: tempfile::TempDir,
}

impl Site {
    fn new() -> Self {
        let site = Site {
            root: tempfile::tempdir().unwrap(),
        };
        fs::write(site.path("empty"), "").unwrap();
        for dir in ["home", "settings"] {
            fs::create_dir_all(site.path(dir)).unwrap();
        }
        let work = site.path("work");
        site.git(None, &["init", "-q", "-b", "main", work.to_str().unwrap()]);
        write_package(&work, "1.0.0", "local monorepo v1.0.0\n");
        site.git(Some(&work), &["add", "-A"]);
        site.git(Some(&work), &["commit", "-q", "-m", "seed"]);
        site.git(
            None,
            &[
                "clone",
                "-q",
                "--bare",
                work.to_str().unwrap(),
                site.path("mono.git").to_str().unwrap(),
            ],
        );
        site
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.path().join(relative)
    }

    fn git(&self, cwd: Option<&Path>, args: &[&str]) -> String {
        let mut command = Command::new("git");
        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        }
        let output = command
            .args([
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=f@example.invalid",
            ])
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.path("empty"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn head(&self) -> String {
        self.git(
            None,
            &[
                "--git-dir",
                self.path("mono.git").to_str().unwrap(),
                "rev-parse",
                "refs/heads/main",
            ],
        )
    }

    /// `~/.vibe/registry.toml`: the local monorepo, with the public hostings
    /// excluded — the corporate closed-network shape.
    fn user_registries(&self, url: &str) {
        fs::write(
            self.path("settings/registry.toml"),
            format!(
                "[[registry]]\nname='corp'\ntype='monorepo'\nurl='{url}'\nref='main'\n\
                 [source_policy]\nexclude_hosting=['github','gitverse']\n"
            ),
        )
        .unwrap();
    }

    fn project(&self, name: &str, extra: &str) -> PathBuf {
        let project = self.path(name);
        fs::create_dir_all(&project).unwrap();
        fs::write(
            project.join("vibe.toml"),
            format!(
                "[project]\nname='consumer'\nversion='1.0.0'\n\
                 [requires.packages]\n'org.fixture/demo'='=1.0.0'\n{extra}"
            ),
        )
        .unwrap();
        project
    }

    fn vibe(&self, project: &Path, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_vibe"))
            .current_dir(project)
            .args(args)
            .env("HOME", self.path("home"))
            .env("VIBE_SETTINGS", self.path("settings"))
            .env("VIBE_REGISTRY_CACHE", self.path("cache"))
            .env("VIBE_NO_DEFAULT_REGISTRY", "1")
            .env("CI", "1")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.path("empty"))
            .env_remove("GIT_SSH")
            .env_remove("GIT_SSH_COMMAND")
            .env_remove("SSH_AUTH_SOCK")
            .output()
            .unwrap()
    }

    fn vibe_ok(&self, project: &Path, args: &[&str]) {
        let output = self.vibe(project, args);
        assert!(
            output.status.success(),
            "vibe {args:?}\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn write_package(root: &Path, version: &str, payload: &str) {
    let dir = root.join("org.fixture/demo").join(format!("v{version}"));
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        format!("[package]\ngroup='org.fixture'\nname='demo'\nkind='feat'\nversion='{version}'\n"),
    )
    .unwrap();
    fs::write(dir.join("payload"), payload).unwrap();
}

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

fn assert_on_monorepo(site: &Site, project: &Path, url: &str) {
    let row = lock_row(project);
    assert_eq!(row["registry"].as_str(), Some("corp"));
    assert_eq!(
        row["source_url"].as_str().unwrap(),
        format!("{url}#org.fixture/demo/v1.0.0")
    );
    assert_eq!(row["source_ref"].as_str(), Some("main"));
    assert_eq!(row["resolved_commit"].as_str().unwrap(), site.head());
}

/// Install and `--sources-only` from a `file://` monorepo and from the same
/// repository named by its absolute path, with `exclude_hosting` active.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY")]
fn local_monorepo_serves_install_and_sources_only_under_exclude_hosting() {
    let site = Site::new();
    let bare = site.path("mono.git");
    for (label, url) in [
        ("file-url", format!("file://{}", bare.display())),
        ("absolute-path", bare.display().to_string()),
    ] {
        site.user_registries(&url);
        let _ = fs::remove_dir_all(site.path("cache"));
        let project = site.project(&format!("install-{label}"), "");
        site.vibe_ok(&project, &["--unattended", "install"]);
        assert_on_monorepo(&site, &project, &url);

        // A held row on another (local-directory) registry moves onto the
        // monorepo with its version and bytes.
        let drop = site.path(&format!("drop-{label}"));
        write_package(&drop, "1.0.0", "local monorepo v1.0.0\n");
        let migrate = site.project(
            &format!("migrate-{label}"),
            &format!(
                "[[registry]]\nname='drop'\nurl='file://{}'\n",
                drop.display()
            ),
        );
        site.vibe_ok(&migrate, &["--unattended", "install"]);
        let before = lock_row(&migrate);
        assert_eq!(before["registry"].as_str(), Some("drop"));
        fs::write(
            migrate.join("vibe.toml"),
            "[project]\nname='consumer'\nversion='1.0.0'\n\
             [requires.packages]\n'org.fixture/demo'='=1.0.0'\n",
        )
        .unwrap();
        site.vibe_ok(&migrate, &["--unattended", "update", "--sources-only"]);
        assert_on_monorepo(&site, &migrate, &url);
        let after = lock_row(&migrate);
        assert_eq!(after["version"], before["version"]);
        assert_eq!(after["content_hash"], before["content_hash"]);
    }
}

/// One operation captures the monorepo once (PROP-073
/// `#NATIVE-PREPARATION`): version discovery, solver metadata and the fetch
/// of every held row share the one snapshot of the commit, counted by the
/// capture trace the controlled transport emits per capture.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-PREPARATION")]
fn one_operation_captures_the_monorepo_once() {
    let site = Site::new();
    let work = site.path("work");
    let other = work.join("org.fixture/other/v0.1.0");
    fs::create_dir_all(&other).unwrap();
    fs::write(
        other.join("vibe.toml"),
        "[package]\ngroup='org.fixture'\nname='other'\nkind='feat'\nversion='0.1.0'\n",
    )
    .unwrap();
    site.git(Some(&work), &["add", "-A"]);
    site.git(Some(&work), &["commit", "-q", "-m", "other"]);
    site.git(
        Some(&work),
        &[
            "push",
            "-q",
            site.path("mono.git").to_str().unwrap(),
            "HEAD:refs/heads/main",
        ],
    );
    let url = format!("file://{}", site.path("mono.git").display());
    site.user_registries(&url);
    let project = site.project("twice", "'org.fixture/other'='=0.1.0'\n");
    let captures = |args: &[&str]| {
        let output = Command::new(env!("CARGO_BIN_EXE_vibe"))
            .current_dir(&project)
            .args(args)
            .env("HOME", site.path("home"))
            .env("VIBE_SETTINGS", site.path("settings"))
            .env("VIBE_REGISTRY_CACHE", site.path("cache"))
            .env("VIBE_NO_DEFAULT_REGISTRY", "1")
            .env("VIBE_LOG", "vibe_registry::monorepo=debug")
            .env("NO_COLOR", "1")
            .env("CI", "1")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", site.path("empty"))
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        assert!(output.status.success(), "vibe {args:?}\n{stderr}");
        stderr.matches("capturing monorepo snapshot").count()
    };
    assert_eq!(captures(&["--unattended", "install"]), 1, "cold install");
    let lock: toml::Value =
        toml::from_str(&fs::read_to_string(project.join("vibe.lock")).unwrap()).unwrap();
    assert_eq!(lock["package"].as_array().unwrap().len(), 2);
    // The corporate cold install: the lock is held, but the store, the
    // registry cache and the slots are gone — every held row is fetched
    // again, all from the same commit.
    for gone in ["settings/cache", "cache"] {
        fs::remove_dir_all(site.path(gone)).unwrap();
    }
    fs::remove_dir_all(project.join("vibevm/vibedeps")).unwrap();
    assert_eq!(
        captures(&["--unattended", "install"]),
        1,
        "cold install of two held rows"
    );
    assert_eq!(
        captures(&["--unattended", "update", "--sources-only"]),
        1,
        "sources-only re-resolution"
    );
}

/// `--offline` reads a local monorepo like a `localdir` registry — no
/// network is involved — while a held row still replays from the store, and
/// a remote monorepo is still never contacted.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-type"
)]
fn offline_reads_a_local_monorepo_but_never_a_remote_one() {
    let site = Site::new();
    let url = format!("file://{}", site.path("mono.git").display());
    site.user_registries(&url);
    let project = site.project("held", "");
    site.vibe_ok(&project, &["--unattended", "install"]);
    let lock = fs::read(project.join("vibe.lock")).unwrap();
    // The held row replays from the store with the repository gone.
    fs::rename(site.path("mono.git"), site.path("moved.git")).unwrap();
    site.vibe_ok(&project, &["--offline", "--unattended", "install"]);
    assert_eq!(fs::read(project.join("vibe.lock")).unwrap(), lock);

    // With the repository back and no lock, offline resolves from it, by
    // its `file://` url and by its absolute path alike.
    fs::rename(site.path("moved.git"), site.path("mono.git")).unwrap();
    for (label, url) in [
        ("file-url", url.clone()),
        ("absolute-path", site.path("mono.git").display().to_string()),
    ] {
        site.user_registries(&url);
        let _ = fs::remove_dir_all(site.path("cache"));
        let fresh = site.project(&format!("fresh-{label}"), "");
        site.vibe_ok(&fresh, &["--offline", "--unattended", "install"]);
        assert_on_monorepo(&site, &fresh, &url);
    }

    // A remote monorepo is not contacted offline: the resolution fails
    // without a lock, before any connection.
    site.user_registries("ssh://git@corp-git.invalid/vibe/registry.git");
    let remote = site.project("fresh-remote", "");
    let output = site.vibe(&remote, &["--offline", "--unattended", "install"]);
    assert!(!output.status.success(), "offline used a remote monorepo");
    assert!(!remote.join("vibe.lock").exists());
}
