//! Hermetic tests for the publish loop (mock `RepoCreator`, real bare
//! repos under a tempdir — no network).

specmark::scope!("spec://vibevm/modules/vibe-workspace/PROP-007#surface");

use super::*;
use crate::commands::workspace::origin::root_identity_name;
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use vibe_core::manifest::NamingConvention;
use vibe_publish::{CreateOpts, PublishError, RepoCreator, RepoInfo};

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// Non-`#[test]` helpers carry `#[cfg(test)]` so the file-grain conform
// frontend scopes their `unwrap`s as test code (the idiom documented on
// rust-ai-native-conform-frontend's own lib/tests.rs).
#[cfg(test)]
fn write(dir: &Path, rel: &str, body: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

fn package(name: &str, kind: &str) -> String {
    format!(
        "[package]\ngroup = \"org.vibevm\"\nname = \"{name}\"\nkind = \"{kind}\"\nversion = \"0.1.0\"\n"
    )
}

fn origin_info() -> OriginInfo {
    OriginInfo {
        upstream: "https://github.com/you/monorepo".to_string(),
        commit: Some("abc123".to_string()),
        generated_by: "vibe test".to_string(),
        generated_at: "2026-05-21T00:00:00Z".to_string(),
    }
}

/// A hermetic mock [`RepoCreator`]. Each `create_repo` provisions a
/// real bare git repo under `bare_root` so `Publisher::publish`'s
/// `push_release` has a working `file://` push target — no network.
/// `fail_on` makes `create_repo` for that exact repo name return a
/// `PublishError`, so the stop-on-first-failure path can be exercised
/// deterministically.
struct MockCreator {
    bare_root: PathBuf,
    /// Repo name whose `create_repo` should fail. `None` = never fail.
    fail_on: Option<String>,
    /// Names passed to `create_repo`, in call order.
    created: RefCell<Vec<String>>,
}

impl MockCreator {
    fn new(bare_root: PathBuf) -> Self {
        MockCreator {
            bare_root,
            fail_on: None,
            created: RefCell::new(Vec::new()),
        }
    }

    fn failing_on(bare_root: PathBuf, repo: &str) -> Self {
        MockCreator {
            bare_root,
            fail_on: Some(repo.to_string()),
            created: RefCell::new(Vec::new()),
        }
    }

    fn bare_path(&self, name: &str) -> PathBuf {
        self.bare_root.join(format!("{name}.git"))
    }
}

impl RepoCreator for MockCreator {
    fn host_name(&self) -> &str {
        "mock-host"
    }

    fn repo_exists(&self, _org: &str, _name: &str) -> Result<bool, PublishError> {
        // Fresh workspace publish — nothing exists yet.
        Ok(false)
    }

    fn create_repo(
        &self,
        _org: &str,
        name: &str,
        _opts: &CreateOpts,
    ) -> Result<RepoInfo, PublishError> {
        self.created.borrow_mut().push(name.to_string());
        if self.fail_on.as_deref() == Some(name) {
            return Err(PublishError::Git(format!(
                "mock: create_repo deliberately failed for `{name}`"
            )));
        }
        // Provision a real bare repo so the subsequent push lands.
        let bare = self.bare_path(name);
        // Pass the path as an `OsStr` arg rather than `to_str().unwrap()` —
        // avoids the lossy round-trip and the unwrap.
        let init = Command::new("git")
            .arg("init")
            .arg("--bare")
            .arg(&bare)
            .env("LC_ALL", "C")
            .status()
            .map_err(|e| PublishError::Git(format!("git init --bare: {e}")))?;
        if !init.success() {
            return Err(PublishError::Git("git init --bare failed".into()));
        }
        // The created bare repo defaults HEAD to whatever git's
        // `init.defaultBranch` is; force `main` so the push matches.
        let _ = Command::new("git")
            .arg("-C")
            .arg(&bare)
            .args(["symbolic-ref", "HEAD", "refs/heads/main"])
            .env("LC_ALL", "C")
            .status();
        let url = format!("file://{}", bare.to_string_lossy().replace('\\', "/"));
        Ok(RepoInfo {
            html_url: url.clone(),
            clone_url: url,
        })
    }

    fn push_url(&self, _org: &str, name: &str) -> String {
        format!(
            "file://{}",
            self.bare_path(name).to_string_lossy().replace('\\', "/")
        )
    }
}

fn plan(bare_root: &Path, dry_run: bool) -> PublishPlan {
    PublishPlan {
        // Org URL only matters for the dry-run synth path; the mock
        // creator overrides the push URL on the real path.
        org_url: format!("file://{}", bare_root.to_string_lossy().replace('\\', "/")),
        naming: NamingConvention::KindName,
        dry_run,
        origin: origin_info(),
    }
}

#[cfg(test)]
fn input(src_root: &Path, rel: &str, kind: &str, name: &str) -> PublishInput {
    PublishInput {
        node: PublishNode {
            rel_path: rel.to_string(),
            kind: match kind {
                "flow" => vibe_core::PackageKind::Flow,
                "feat" => vibe_core::PackageKind::Feat,
                "stack" => vibe_core::PackageKind::Stack,
                _ => vibe_core::PackageKind::Tool,
            },
            group: vibe_core::Group::parse("org.vibevm").unwrap(),
            name: name.to_string(),
        },
        source_dir: src_root.join(rel),
    }
}

#[test]
fn publish_loop_publishes_every_node_in_order() {
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let src = tempfile::tempdir().unwrap();
    write(src.path(), "packages/a/vibe.toml", &package("a", "flow"));
    write(src.path(), "packages/b/vibe.toml", &package("b", "feat"));
    let bare_root = tempfile::tempdir().unwrap();
    let creator = MockCreator::new(bare_root.path().to_path_buf());
    let inputs = vec![
        input(src.path(), "packages/a", "flow", "a"),
        input(src.path(), "packages/b", "feat", "b"),
    ];
    let plan = plan(bare_root.path(), false);
    let mut seen: Vec<String> = Vec::new();
    let published = publish_loop(Some(&creator), &inputs, &plan, &mut |e, _| {
        seen.push(e.pkgref.clone());
    })
    .expect("publish loop should succeed");
    assert_eq!(published.len(), 2);
    assert_eq!(published[0].pkgref, "org.vibevm/a");
    assert_eq!(published[1].pkgref, "org.vibevm/b");
    // Progress callback fired once per node, in order.
    assert_eq!(seen, vec!["org.vibevm/a", "org.vibevm/b"]);
    // Repos created in order: flow-a then feat-b (kind-name naming).
    assert_eq!(*creator.created.borrow(), vec!["flow-a", "feat-b"]);
}

#[test]
fn publish_loop_stops_on_first_failure_and_reports_partial_progress() {
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let src = tempfile::tempdir().unwrap();
    write(src.path(), "packages/a/vibe.toml", &package("a", "flow"));
    write(src.path(), "packages/b/vibe.toml", &package("b", "feat"));
    write(src.path(), "packages/c/vibe.toml", &package("c", "tool"));
    let bare_root = tempfile::tempdir().unwrap();
    // The middle node (feat-b) fails. a publishes, b fails, c is
    // never reached.
    let creator = MockCreator::failing_on(bare_root.path().to_path_buf(), "feat-b");
    let inputs = vec![
        input(src.path(), "packages/a", "flow", "a"),
        input(src.path(), "packages/b", "feat", "b"),
        input(src.path(), "packages/c", "tool", "c"),
    ];
    let plan = plan(bare_root.path(), false);
    let failure = publish_loop(Some(&creator), &inputs, &plan, &mut |_, _| {})
        .expect_err("publish loop should fail on the middle node");
    // Only `a` published before the failure.
    assert_eq!(failure.published.len(), 1);
    assert_eq!(failure.published[0].pkgref, "org.vibevm/a");
    // The failed node is index 1 (`b`) — `b` and `c` are the
    // `remaining` set when finish_failure slices `ordered[1..]`.
    assert_eq!(failure.failed_idx, 1);
    let msg = format!("{}", failure.error);
    assert!(
        msg.contains("org.vibevm/b"),
        "error names the failed node: {msg}"
    );
    // `c` was never reached — create_repo never called for it.
    assert!(!creator.created.borrow().iter().any(|n| n == "tool-c"));
}

#[test]
fn publish_loop_dry_run_makes_no_network_calls() {
    // No creator at all — the dry-run path must synthesise the
    // outcome from the staged manifest without touching the network.
    let src = tempfile::tempdir().unwrap();
    write(src.path(), "packages/a/vibe.toml", &package("a", "flow"));
    write(src.path(), "packages/b/vibe.toml", &package("b", "stack"));
    let bare_root = tempfile::tempdir().unwrap();
    let inputs = vec![
        input(src.path(), "packages/a", "flow", "a"),
        input(src.path(), "packages/b", "stack", "b"),
    ];
    let plan = plan(bare_root.path(), true);
    let published = publish_loop(None, &inputs, &plan, &mut |_, _| {})
        .expect("dry-run publish loop should succeed");
    assert_eq!(published.len(), 2);
    assert_eq!(published[0].pkgref, "org.vibevm/a");
    assert_eq!(published[0].repo_name, "flow-a");
    assert_eq!(published[0].tag, "v0.1.0");
    assert_eq!(published[1].repo_name, "stack-b");
    // No bare repos were provisioned — dry-run wrote nothing.
    assert!(
        fs::read_dir(bare_root.path()).unwrap().next().is_none(),
        "dry-run must not create any repo on disk"
    );
}

#[test]
fn publish_loop_staged_copy_carries_origin_and_banner() {
    // Exercise the staging side of the loop through dry-run and
    // confirm the staged content is correct by staging directly.
    let src = tempfile::tempdir().unwrap();
    write(src.path(), "packages/a/vibe.toml", &package("a", "flow"));
    write(src.path(), "packages/a/README.md", "# upstream readme\n");
    let staged = stage_node(&src.path().join("packages/a"), "packages/a", &origin_info()).unwrap();
    // [origin] present and correct.
    let manifest = Manifest::read(staged.staging.path().join("vibe.toml")).unwrap();
    let origin = manifest.origin.as_ref().expect("[origin] present");
    assert_eq!(origin.path, "packages/a");
    assert_eq!(origin.upstream, "https://github.com/you/monorepo");
    // README banner prepended.
    let readme = fs::read_to_string(staged.staging.path().join("README.md")).unwrap();
    assert!(readme.starts_with("<!-- vibevm:generated-copy -->"));
    assert!(readme.contains("# upstream readme"));
    // PR template written.
    assert!(
        staged
            .staging
            .path()
            .join(".github/PULL_REQUEST_TEMPLATE.md")
            .is_file()
    );
}

#[test]
fn dry_run_outcome_reads_staged_manifest() {
    let staged = tempfile::tempdir().unwrap();
    write(staged.path(), "vibe.toml", &package("wal", "flow"));
    let config = PublishConfig {
        source_dir: staged.path().to_path_buf(),
        org_url: "https://github.com/vibespecs".to_string(),
        naming: NamingConvention::KindName,
        tag_prefix: "v".to_string(),
        dry_run: true,
    };
    let outcome = dry_run_outcome(&config, "https://github.com/vibespecs").unwrap();
    assert_eq!(outcome.repo_name, "flow-wal");
    assert_eq!(outcome.tag, "v0.1.0");
    assert_eq!(
        outcome.repo_url,
        "https://github.com/vibespecs/flow-wal.git"
    );
    assert!(outcome.dry_run);
}

#[test]
fn root_identity_name_prefers_project_then_package() {
    let proj = Manifest::parse_str("[project]\nname = \"mono\"\nversion = \"0.0.1\"\n").unwrap();
    assert_eq!(root_identity_name(&proj), "mono");
    let pkg = Manifest::parse_str(&package("umbrella", "stack")).unwrap();
    assert_eq!(root_identity_name(&pkg), "umbrella");
    let virt = Manifest::parse_str("[workspace]\nmembers = []\n").unwrap();
    assert_eq!(root_identity_name(&virt), "unknown");
}
