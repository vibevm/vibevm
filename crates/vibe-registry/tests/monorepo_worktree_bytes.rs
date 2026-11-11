//! A registry's working copy holds the bytes its blobs record.
//!
//! `GitMonorepoRegistry` clones a repository into vibe's own cache and then
//! *reads* it: manifests, and the template snapshot under `_index/` whose
//! digest documents are hashed against the `digest_sha256` their rows carry
//! (PROP-075 `#MIRROR-SNAPSHOT-LAYOUT`). Those are comparisons of bytes, so
//! the checkout must not apply Git's working-tree conversion — not the one the
//! machine's configuration asks for (`core.autocrlf=true` is what the Git for
//! Windows installer writes into the *system* scope, so it is the stock
//! Windows default), and not the one the payload's own `.gitattributes` asks
//! for, which `core.autocrlf=false` does not answer at all.
//!
//! Both levers are driven here without touching this process's environment:
//! the attributes come from the repository, and the configuration lever is set
//! in the clone's own config, where a converting machine's would be read from
//! too.

#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use vibe_registry::ShellGit;
use vibe_registry::git_registry::GitMonorepoRegistry;

/// A year: every reopen inside these tests finds the cache fresh, which is the
/// state a repair has to work in — no fetch, no checkout, nothing but a read.
const FRESH: u64 = 365 * 24 * 60 * 60;

/// A digest document shaped like the ones a template snapshot carries: text,
/// LF-terminated, hashed by its bytes.
const DOCUMENT: &str = "d0f8d1 node-v26.13.0-linux-x64.tar.gz\nc1a2b3 node-v26.13.0.tar.xz\n";

struct Repository {
    root: tempfile::TempDir,
}

impl Repository {
    /// A repository carrying `attributes` as its `.gitattributes`, published
    /// as a bare clone — the shape a mirrored corporate registry has.
    fn new(attributes: &str) -> Self {
        let repository = Repository {
            root: tempfile::tempdir().unwrap(),
        };
        fs::write(repository.path("empty"), "").unwrap();
        let work = repository.path("work");
        let snapshot = work.join("_index/templates/org.vibevm.node/node/digests");
        fs::create_dir_all(&snapshot).unwrap();
        fs::write(snapshot.join("26.13.0"), DOCUMENT).unwrap();
        fs::write(work.join(".gitattributes"), attributes).unwrap();
        repository.git(None, &["init", "-q", "-b", "main", work.to_str().unwrap()]);
        // `add` under `core.autocrlf=false` so the blobs hold LF whatever the
        // developer running the suite has configured: the question here is
        // what a *checkout* writes, never what a commit recorded.
        repository.git(Some(&work), &["-c", "core.autocrlf=false", "add", "-A"]);
        repository.git(Some(&work), &["commit", "-q", "-m", "the registry"]);
        repository.git(
            None,
            &[
                "clone",
                "-q",
                "--bare",
                work.to_str().unwrap(),
                repository.path("mono.git").to_str().unwrap(),
            ],
        );
        repository
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.path().join(relative)
    }

    fn url(&self) -> String {
        format!("file://{}", self.path("mono.git").display())
    }

    /// `git status --porcelain` of a clone, for the one assertion that is
    /// about what Git *thinks* rather than about the bytes.
    fn status(&self, cwd: &Path) -> String {
        let output = Command::new("git")
            .current_dir(cwd)
            .args(["status", "--porcelain"])
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.path("empty"))
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap()
    }

    fn git(&self, cwd: Option<&Path>, args: &[&str]) {
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
    }

    /// Open the registry the way a resolution does.
    fn open(&self, cache: &Path) -> GitMonorepoRegistry {
        GitMonorepoRegistry::open_with(&self.url(), "main", cache, Arc::new(ShellGit::new()), FRESH)
            .unwrap()
    }
}

/// Every tracked file of `clone`, against the bytes its blob records.
fn assert_worktree_is_its_blobs(clone: &Path) {
    let listed = Command::new("git")
        .current_dir(clone)
        .args([
            "ls-tree",
            "-r",
            "-z",
            "--format=%(objectname) %(path)",
            "HEAD",
        ])
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .unwrap();
    assert!(listed.status.success());
    let listed = String::from_utf8(listed.stdout).unwrap();
    let mut checked = 0usize;
    for row in listed.split('\0').filter(|row| !row.is_empty()) {
        let (object, path) = row.split_once(' ').expect("`<oid> <path>`");
        let blob = Command::new("git")
            .current_dir(clone)
            .args(["cat-file", "blob", object])
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .output()
            .unwrap();
        assert!(blob.status.success());
        assert_eq!(
            fs::read(clone.join(path)).unwrap(),
            blob.stdout,
            "`{path}` is not the bytes its blob records"
        );
        checked += 1;
    }
    assert_eq!(checked, 2, "the fixture's two tracked files");
}

/// `* text=auto eol=crlf` in the registry itself. This is the conversion
/// `core.autocrlf=false` does not answer: the payload asks for it, and only
/// `$GIT_DIR/info/attributes` outranks a `.gitattributes` in the tree.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION")]
fn a_payloads_own_eol_attribute_does_not_reach_a_registry_working_copy() {
    let repository = Repository::new("* text=auto eol=crlf\n");
    let cache = tempfile::tempdir().unwrap();
    let registry = repository.open(cache.path());
    let clone = registry.clone_dir().to_path_buf();

    assert_eq!(
        fs::read(clone.join("_index/templates/org.vibevm.node/node/digests/26.13.0")).unwrap(),
        DOCUMENT.as_bytes(),
        "the digest document a snapshot's `digest_sha256` describes"
    );
    assert_worktree_is_its_blobs(&clone);
}

/// The upgrade path: a working copy an earlier vibe checked out under the
/// user's conversion is repaired on first use — including when its cache is
/// still fresh, so no fetch and no checkout happen on the registry's own
/// account, and including when the clone's own configuration still says to
/// convert.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION")]
fn a_working_copy_left_converted_by_an_earlier_vibe_is_repaired_on_first_use() {
    let repository = Repository::new("# nothing to declare\n");
    let cache = tempfile::tempdir().unwrap();
    let clone = repository.open(cache.path()).clone_dir().to_path_buf();

    // Exactly what an earlier vibe leaves: no byte-exactness rule in the
    // repository, a converting configuration of its own, and files carrying
    // that conversion — written by Git, so the index records them as clean,
    // which is the state a hand-written file does not reproduce and the one
    // the repair has to survive.
    fs::remove_file(clone.join(".git/info/attributes")).unwrap();
    repository.git(
        Some(&clone),
        &["config", "--local", "core.autocrlf", "true"],
    );
    repository.git(Some(&clone), &["read-tree", "HEAD"]);
    repository.git(Some(&clone), &["checkout-index", "--all", "--force"]);
    let document = clone.join("_index/templates/org.vibevm.node/node/digests/26.13.0");
    assert_eq!(
        fs::read(&document).unwrap(),
        DOCUMENT.replace('\n', "\r\n").as_bytes(),
        "the fixture's own converted checkout"
    );
    assert!(
        repository.status(&clone).is_empty(),
        "an earlier vibe left the copy stat-clean, not visibly dirty"
    );

    // The same cache, still fresh: opening it is the only thing that happens.
    let reopened = repository.open(cache.path());
    assert_eq!(reopened.clone_dir(), clone);
    assert_eq!(
        fs::read(&document).unwrap(),
        DOCUMENT.as_bytes(),
        "the converted working copy was not repaired"
    );
    assert_worktree_is_its_blobs(&clone);

    // And the repair is not re-run on every open: the attributes are already
    // in force, so a third open finds nothing to do and leaves the bytes be.
    let again = repository.open(cache.path());
    assert_eq!(again.clone_dir(), clone);
    assert_eq!(fs::read(&document).unwrap(), DOCUMENT.as_bytes());
}
