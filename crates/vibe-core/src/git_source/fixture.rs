//! A hermetic `git` for test fixtures that build a repository around a
//! package source.
//!
//! Every command runs with no system config, an empty global config, an
//! empty hooks directory and none of the variables that would point `git` at
//! another repository, so a fixture commit never depends on — or acts on —
//! the developer's own setup. The code under test runs `git` with the
//! inherited environment, so [`FixtureGit::init`] and [`FixtureGit::clone`]
//! also pin `core.excludesFile` in the fixture repository's own config: a
//! repository setting overrides the user's global ignore rules.
//!
//! Compiled for this crate's tests and, through the `git-fixture` feature,
//! for the tests of the crates that hash or copy a package source.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-024#shippable-tree");

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Variables `git` reads configuration from besides its files.
const CONFIG_OVERRIDES: &[&str] = &["GIT_CONFIG", "GIT_CONFIG_PARAMETERS", "GIT_CONFIG_COUNT"];

/// One isolated `git` setup: an empty global config, an empty excludes file
/// and an empty hooks directory, removed when the value drops.
#[derive(Debug)]
pub struct FixtureGit {
    isolation: tempfile::TempDir,
}

impl Default for FixtureGit {
    fn default() -> Self {
        Self::new()
    }
}

impl FixtureGit {
    /// Create the isolation directory.
    ///
    /// # Panics
    ///
    /// When the temporary directory cannot be created.
    pub fn new() -> Self {
        let isolation = tempfile::tempdir().expect("create git fixture isolation directory");
        fs::write(isolation.path().join("config"), "").expect("write empty git config");
        fs::write(isolation.path().join("excludes"), "").expect("write empty excludes file");
        fs::create_dir(isolation.path().join("hooks")).expect("create empty hooks directory");
        Self { isolation }
    }

    fn excludes(&self) -> PathBuf {
        self.isolation.path().join("excludes")
    }

    /// `git -C dir` with the isolated environment, ready for arguments.
    pub fn command(&self, dir: &Path) -> Command {
        let mut git = Command::new("git");
        git.arg("-C")
            .arg(dir)
            .arg("-c")
            .arg(format!(
                "core.hooksPath={}",
                self.isolation.path().join("hooks").display()
            ))
            .args([
                "-c",
                "user.name=fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "tag.gpgsign=false",
                "-c",
                "init.defaultBranch=main",
            ])
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.isolation.path().join("config"))
            .stdin(Stdio::null());
        for name in super::REPOSITORY_OVERRIDES.iter().chain(CONFIG_OVERRIDES) {
            git.env_remove(name);
        }
        git
    }

    /// Run `git -C dir <args>` and require success.
    ///
    /// # Panics
    ///
    /// When `git` cannot be spawned or exits unsuccessfully.
    pub fn run(&self, dir: &Path, args: &[&str]) {
        let output = self
            .command(dir)
            .args(args)
            .output()
            .expect("spawn git for a test fixture");
        assert!(
            output.status.success(),
            "git {args:?} in {} failed: {}",
            dir.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    /// Create a repository at `dir` (creating `dir` too) and isolate it.
    ///
    /// # Panics
    ///
    /// When the directory cannot be created or `git` fails.
    pub fn init(&self, dir: &Path) {
        fs::create_dir_all(dir).expect("create fixture repository directory");
        self.run(dir, &["init", "-q"]);
        self.isolate(dir);
    }

    /// Clone `origin` into `dest` and isolate the clone.
    ///
    /// # Panics
    ///
    /// When `dest` has no parent or `git` fails.
    pub fn clone(&self, origin: &Path, dest: &Path) {
        let parent = dest.parent().expect("clone destination has a parent");
        let origin = origin.to_string_lossy();
        let target = dest.to_string_lossy();
        self.run(parent, &["clone", "-q", &origin, &target]);
        self.isolate(dest);
    }

    /// Pin `core.excludesFile` in the repository at `repo` to an empty file,
    /// so the user's global ignore rules never reach the code under test.
    ///
    /// # Panics
    ///
    /// When `git` fails.
    pub fn isolate(&self, repo: &Path) {
        let excludes = self.excludes();
        self.run(
            repo,
            &["config", "core.excludesFile", &excludes.to_string_lossy()],
        );
    }

    /// Stage everything below `dir` and commit it.
    ///
    /// # Panics
    ///
    /// When `git` fails.
    pub fn commit_all(&self, dir: &Path, message: &str) {
        self.run(dir, &["add", "-A"]);
        self.run(dir, &["commit", "-q", "-m", message]);
    }
}

/// Whether `dir` lies outside every Git worktree — the precondition of a
/// test that asserts the plain walk. A test whose temporary directory sits
/// inside a worktree has nothing to assert and returns early.
pub fn outside_any_worktree(dir: &Path) -> bool {
    super::resolve(dir).is_none_or(|dir| !super::inside_git_worktree(&dir))
}
