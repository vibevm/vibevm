//! An owned loopback sshd serving one writable monorepo (PROP-002
//! `#registry-type`) for the monorepo resolution and publish tests.
//! Numeric-loopback SSH with fixture keys, never the operator's.
#![cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};

pub struct Fixture {
    process: Child,
    temporary: tempfile::TempDir,
    pub facts: serde_json::Value,
    pub config: PathBuf,
}

impl Fixture {
    pub fn new() -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let script = include_str!(
            "../../../vibe-registry/src/git_backend/shell/native_admission/ssh_fixture.py"
        );
        let mut process = Command::new("python3")
            .arg("-u")
            .arg("-c")
            .arg(script)
            .env("TMPDIR", temporary.path())
            .env("VIBE_SSH_FIXTURE_WRITABLE", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(process.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let facts: serde_json::Value =
            serde_json::from_str(&line).expect("authenticated fixture startup");
        let config = temporary.path().join("ssh.config");
        fs::write(
            &config,
            format!(
                "Host ordinary-fixture\n HostName 127.0.0.1\n User {}\n Port {}\n IdentityFile {}\n IdentityAgent none\n IdentitiesOnly yes\n UserKnownHostsFile {}\n GlobalKnownHostsFile /dev/null\n StrictHostKeyChecking yes\n",
                facts["user"].as_str().unwrap(),
                facts["port"],
                facts["key"].as_str().unwrap(),
                facts["known_hosts"].as_str().unwrap()
            ),
        )
        .unwrap();
        fs::set_permissions(&config, fs::Permissions::from_mode(0o600)).unwrap();
        fs::write(temporary.path().join("empty"), "").unwrap();
        let fixture = Self {
            process,
            temporary,
            facts,
            config,
        };
        // Replace the fixture's single-package repository with a monorepo
        // layout on `main`, pushed over the local filesystem.
        let work = fixture.root().join("monorepo-work");
        fixture.git(None, &["init", "-q", "-b", "main", work.to_str().unwrap()]);
        write_package(&work, "demo", "1.0.0", "monorepo v1.0.0\n");
        write_package(&work, "demo", "1.1.0", "monorepo v1.1.0\n");
        write_package(&work, "other", "0.1.0", "other\n");
        fixture.commit_all(&work, "seed");
        fixture.git(
            Some(&work),
            &[
                "push",
                "-q",
                "--force",
                fixture.bare().to_str().unwrap(),
                "HEAD:refs/heads/main",
            ],
        );
        fixture
    }

    pub fn root(&self) -> &Path {
        self.temporary.path()
    }

    pub fn bare(&self) -> PathBuf {
        PathBuf::from(self.facts["home"].as_str().unwrap()).join("repo.git")
    }

    /// The repository url through the ordinary `ssh_config` host alias.
    pub fn address(&self) -> String {
        self.facts["url"]
            .as_str()
            .unwrap()
            .replace("127.0.0.1", "ordinary-fixture")
    }

    pub fn git(&self, cwd: Option<&Path>, args: &[&str]) -> String {
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
            .env("GIT_CONFIG_GLOBAL", self.root().join("empty"))
            .env(
                "GIT_SSH_COMMAND",
                format!("/usr/bin/ssh -F {}", self.config.display()),
            )
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    pub fn commit_all(&self, work: &Path, message: &str) {
        self.git(Some(work), &["add", "-A"]);
        self.git(Some(work), &["commit", "-q", "-m", message]);
    }

    pub fn main_commit(&self) -> String {
        self.git(
            None,
            &[
                "--git-dir",
                self.bare().to_str().unwrap(),
                "rev-parse",
                "refs/heads/main",
            ],
        )
    }

    pub fn project(&self, name: &str, registries: &str) -> PathBuf {
        let project = self.root().join(name);
        fs::create_dir_all(&project).unwrap();
        fs::write(
            project.join("vibe.toml"),
            format!(
                "[project]\nname='consumer'\nversion='1.0.0'\n[requires.packages]\n'org.fixture/demo'='=1.0.0'\n{registries}"
            ),
        )
        .unwrap();
        project
    }

    pub fn monorepo_row(&self) -> String {
        format!(
            "[[registry]]\nname='corp'\ntype='MonoRepo'\nurl='{}'\nref='main'\n",
            self.address()
        )
    }

    pub fn command(&self, project: &Path) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_vibe"));
        command
            .current_dir(project)
            .env("HOME", self.facts["home"].as_str().unwrap())
            .env("VIBE_SETTINGS", self.root().join("settings"))
            .env("VIBE_REGISTRY_CACHE", self.root().join("cache"))
            .env("VIBE_NO_DEFAULT_REGISTRY", "1")
            .env("CI", "1")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.root().join("empty"))
            .env(
                "GIT_SSH_COMMAND",
                format!("/usr/bin/ssh -F {}", self.config.display()),
            )
            .env_remove("GIT_SSH")
            .env_remove("SSH_AUTH_SOCK");
        command
    }

    pub fn vibe(&self, project: &Path, args: &[&str]) -> std::process::Output {
        let output = self.command(project).args(args).output().unwrap();
        assert!(
            output.status.success(),
            "vibe {args:?}\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    pub fn contacts(&self) -> usize {
        fs::read_to_string(Path::new(self.facts["root"].as_str().unwrap()).join("sshd-a.log"))
            .unwrap()
            .matches("Connection from ")
            .count()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        drop(self.process.stdin.take());
        let _ = self.process.wait();
    }
}

pub fn write_package(root: &Path, name: &str, version: &str, payload: &str) {
    let dir = root
        .join("org.fixture")
        .join(name)
        .join(format!("v{version}"));
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        format!(
            "[package]\ngroup='org.fixture'\nname='{name}'\nkind='feat'\nversion='{version}'\n"
        ),
    )
    .unwrap();
    fs::write(dir.join("payload"), payload).unwrap();
}
