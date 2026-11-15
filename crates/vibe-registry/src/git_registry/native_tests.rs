//! What a second command costs when nothing changed.
//!
//! These drive the real controlled transport against a real repository on
//! disk, through a `git` wrapper that records every invocation vibe makes.
//! The counts are the point: a closed network has no other transport, so a
//! `clone` or a `check-attr` storm per command is what the operator pays on
//! every single `vibe` they run.
#![cfg(all(test, unix))]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use crate::git_backend::ShellGit;
use crate::git_backend::shell::native_admission::{
    GitInstallation, NativeConfigSelection, NativeGitAdmission, TlsTrust,
};
use crate::{GitMonorepoRegistry, Registry};

const GIT: &str = "/usr/bin/git";
const HELPERS: &str = "/usr/lib/git-core";

/// A wrapper that appends its argv to `log` and then is the real `git`.
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "no-unwrap-in-domain: this cfg(test, unix) helper builds isolated test-only Git fixtures; a broken fixture must abort the test, never become a product fallback"
)]
fn counting_git(dir: &Path, log: &Path) -> PathBuf {
    let wrapper = dir.join("git-counting");
    fs::write(
        &wrapper,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> {log}\nexec {GIT} \"$@\"\n",
            log = log.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    wrapper
}

#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "no-unwrap-in-domain: this cfg(test, unix) helper builds isolated test-only Git fixtures; a broken fixture must abort the test, never become a product fallback"
)]
fn git_in(repo: &Path, args: &[&str]) {
    let status = Command::new(GIT)
        .args(args)
        .current_dir(repo)
        .env("HOME", repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@invalid")
        .env("GIT_COMMITTER_NAME", "fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@invalid")
        .status()
        .unwrap();
    assert!(status.success(), "fixture git {args:?}");
}

/// A bare monorepo on disk holding one package, plus a working checkout to
/// add commits with.
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "no-unwrap-in-domain: this cfg(test, unix) helper builds isolated test-only Git fixtures; a broken fixture must abort the test, never become a product fallback"
)]
fn monorepo(root: &Path) -> (PathBuf, PathBuf) {
    let work = root.join("work");
    let bare = root.join("mono.git");
    fs::create_dir_all(work.join("org.corp/tool/v1.0.0")).unwrap();
    fs::write(
        work.join("org.corp/tool/v1.0.0/vibe.toml"),
        b"[package]\nname = \"tool\"\ngroup = \"org.corp\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    git_in(&work, &["init", "-q", "-b", "main"]);
    git_in(&work, &["add", "-A"]);
    git_in(&work, &["commit", "-qm", "one package"]);
    git_in(
        root,
        &[
            "clone",
            "-q",
            "--bare",
            "--no-local",
            work.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    (bare, work)
}

#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "no-unwrap-in-domain: this cfg(test, unix) helper builds isolated test-only Git fixtures; a broken fixture must abort the test, never become a product fallback"
)]
fn backend(home: &Path, executable: &Path) -> Arc<ShellGit> {
    fs::create_dir_all(home).unwrap();
    let selection = NativeConfigSelection::new(home.into(), home.into())
        .unwrap()
        .with_variable("GIT_CONFIG_NOSYSTEM", "1".into())
        .unwrap()
        .with_variable("GIT_ATTR_NOSYSTEM", "1".into())
        .unwrap();
    let profile = NativeGitAdmission::direct_https(
        Arc::new(crate::HttpAdmission::new([] as [&str; 0]).unwrap()),
        GitInstallation::trusted(executable.into(), HELPERS.into()).unwrap(),
        TlsTrust::System,
    )
    .with_config_selection(selection);
    Arc::new(ShellGit::new().with_native_admission(Arc::new(profile)))
}

/// Every git invocation vibe made since the log was last cleared, as the
/// leading subcommand of each (the global `-c` flags skipped).
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "no-unwrap-in-domain: this cfg(test, unix) helper reads an isolated test-only log; a broken fixture must abort the test"
)]
fn subcommands(log: &Path) -> Vec<String> {
    let text = fs::read_to_string(log).unwrap_or_default();
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let mut words = line.split_whitespace().peekable();
            while let Some(word) = words.next() {
                if word == "-c" || word == "--git-dir" || word == "--work-tree" {
                    words.next();
                    continue;
                }
                if word.starts_with('-') {
                    continue;
                }
                return word.to_owned();
            }
            "<none>".to_owned()
        })
        .collect()
}

fn count(log: &Path, subcommand: &str) -> usize {
    subcommands(log)
        .iter()
        .filter(|found| *found == subcommand)
        .count()
}

/// The whole-tree attribute probes alone. A preparation also asks one
/// `check-attr --cached` about the authoritative manifest; that one is not
/// the storm this cache exists to remove.
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "no-unwrap-in-domain: this cfg(test, unix) helper reads an isolated test-only log; a broken fixture must abort the test"
)]
fn tree_probes(log: &Path) -> usize {
    fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .filter(|line| line.contains("check-attr") && line.contains("--all"))
        .count()
}

/// Open the registry as a fresh `vibe` process would: a new backend, whose
/// capability remembers nothing, and none of this run's validations.
fn open_fresh(
    url: &str,
    cache_root: &Path,
    home: &Path,
    executable: &Path,
    ttl: u64,
) -> GitMonorepoRegistry {
    let backend = backend(home, executable);
    GitMonorepoRegistry::open_native(url, &[], "main", cache_root, backend, ttl)
        .expect("opening the controlled monorepo")
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-PREPARATION-LAW")]
fn a_second_command_asks_the_endpoint_once_and_clones_fscks_and_probes_nothing() {
    let root = tempfile::tempdir().unwrap();
    let (bare, work) = monorepo(root.path());
    let log = root.path().join("git.log");
    let executable = counting_git(root.path(), &log);
    let home = root.path().join("home");
    let cache = root.path().join("registries");
    let url = format!("file://{}", bare.display());
    let group = vibe_core::Group::parse("org.corp").unwrap();

    // First use: the capture, exactly as before this cache existed.
    let first = open_fresh(&url, &cache, &home, &executable, 3600);
    assert_eq!(
        first.list_versions(&group, "tool").unwrap(),
        vec![semver::Version::new(1, 0, 0)]
    );
    assert_eq!(count(&log, "clone"), 1, "{:?}", subcommands(&log));
    assert_eq!(count(&log, "fsck"), 2, "the creation walks every object");
    assert_eq!(tree_probes(&log), 1, "the whole tree is probed once");
    assert_eq!(
        count(&log, "ls-remote"),
        1,
        "one advertisement chooses the namespace"
    );
    let commit = first.head_commit().unwrap().unwrap();
    // A reader may outlive another process refreshing the same registry.
    let held_reader = first;
    let held_tree = held_reader.clone_dir().to_path_buf();

    // Second use, nothing changed: one question, no objects.
    fs::write(&log, "").unwrap();
    let second = open_fresh(&url, &cache, &home, &executable, 0);
    assert_eq!(
        second.list_versions(&group, "tool").unwrap(),
        vec![semver::Version::new(1, 0, 0)]
    );
    assert_eq!(second.head_commit().unwrap().unwrap(), commit);
    assert_eq!(count(&log, "clone"), 0, "{:?}", subcommands(&log));
    assert_eq!(count(&log, "fetch"), 0, "{:?}", subcommands(&log));
    assert_eq!(count(&log, "fsck"), 0, "{:?}", subcommands(&log));
    assert_eq!(tree_probes(&log), 0, "{:?}", subcommands(&log));
    assert_eq!(count(&log, "reset"), 0, "{:?}", subcommands(&log));
    assert_eq!(
        count(&log, "ls-remote"),
        1,
        "the freshness window was zero, so the endpoint was asked once"
    );
    drop(second);

    // The same, inside the freshness window: not even that one question.
    fs::write(&log, "").unwrap();
    let cached = open_fresh(&url, &cache, &home, &executable, 3600);
    assert_eq!(cached.head_commit().unwrap().unwrap(), commit);
    assert_eq!(count(&log, "ls-remote"), 0, "{:?}", subcommands(&log));
    drop(cached);

    // A new commit lands: the objects are fetched, never cloned again.
    fs::create_dir_all(work.join("org.corp/tool/v1.1.0")).unwrap();
    fs::write(
        work.join("org.corp/tool/v1.1.0/vibe.toml"),
        b"[package]\nname = \"tool\"\ngroup = \"org.corp\"\nversion = \"1.1.0\"\n",
    )
    .unwrap();
    git_in(&work, &["add", "-A"]);
    git_in(&work, &["commit", "-qm", "a second version"]);
    git_in(&work, &["push", "-q", bare.to_str().unwrap(), "main"]);

    fs::write(&log, "").unwrap();
    let moved = open_fresh(&url, &cache, &home, &executable, 0);
    assert_eq!(
        moved.list_versions(&group, "tool").unwrap(),
        vec![semver::Version::new(1, 0, 0), semver::Version::new(1, 1, 0)],
        "the refreshed snapshot holds the new version"
    );
    assert_ne!(moved.head_commit().unwrap().unwrap(), commit);
    assert_ne!(moved.clone_dir(), held_tree);
    assert_eq!(held_reader.head_commit().unwrap().unwrap(), commit);
    assert_eq!(
        held_reader.list_versions(&group, "tool").unwrap(),
        vec![semver::Version::new(1, 0, 0)],
        "refresh must not replace the selected bytes of a live reader"
    );
    assert_eq!(count(&log, "clone"), 0, "{:?}", subcommands(&log));
    assert_eq!(count(&log, "fetch"), 1, "{:?}", subcommands(&log));
    assert_eq!(
        count(&log, "fsck"),
        0,
        "the new objects were checked at receipt, so nothing is re-walked"
    );
    assert_eq!(
        tree_probes(&log),
        1,
        "one probe for the whole tree, not one per sixty-four paths"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-PREPARATION-LAW")]
fn one_invocation_captures_one_snapshot_however_many_stages_read_it() {
    let root = tempfile::tempdir().unwrap();
    let (bare, _work) = monorepo(root.path());
    let log = root.path().join("git.log");
    let executable = counting_git(root.path(), &log);
    let home = root.path().join("home");
    let cache = root.path().join("registries");
    let url = format!("file://{}", bare.display());

    let first = open_fresh(&url, &cache, &home, &executable, 3600);
    let git = backend(&home, &executable);
    fs::write(&log, "").unwrap();
    // Discovery, the solver's metadata and the held-row fetch each open the
    // registry; within one invocation they share one validated snapshot.
    for _ in 0..3 {
        let again =
            GitMonorepoRegistry::open_native(&url, &[], "main", &cache, git.clone(), 3600).unwrap();
        assert_eq!(again.head_commit().unwrap(), first.head_commit().unwrap());
    }
    assert_eq!(count(&log, "clone"), 0);
    assert_eq!(count(&log, "fetch"), 0);
    assert_eq!(count(&log, "fsck"), 0);
    assert_eq!(count(&log, "ls-remote"), 0);
    assert_eq!(tree_probes(&log), 0);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION-LAW"
)]
fn a_damaged_tree_an_unknown_object_store_and_changed_attributes_each_recapture() {
    let root = tempfile::tempdir().unwrap();
    let (bare, _work) = monorepo(root.path());
    let log = root.path().join("git.log");
    let executable = counting_git(root.path(), &log);
    let home = root.path().join("home");
    let cache = root.path().join("registries");
    let url = format!("file://{}", bare.display());
    let group = vibe_core::Group::parse("org.corp").unwrap();

    let first = open_fresh(&url, &cache, &home, &executable, 3600);
    let tree = first.clone_dir().to_path_buf();
    let manifest = tree.join("org.corp/tool/v1.0.0/vibe.toml");
    let recorded = fs::read(&manifest).unwrap();
    drop(first);

    // An edited tree is not reused at any age: it is re-staged from the
    // objects, and the edit is gone.
    fs::write(&manifest, b"[package]\nname = \"not-the-recorded-bytes\"\n").unwrap();
    fs::write(&log, "").unwrap();
    let repaired = open_fresh(&url, &cache, &home, &executable, 3600);
    assert_eq!(
        fs::read(repaired.clone_dir().join("org.corp/tool/v1.0.0/vibe.toml")).unwrap(),
        recorded
    );
    assert_eq!(
        repaired.list_versions(&group, "tool").unwrap(),
        vec![semver::Version::new(1, 0, 0)]
    );
    assert_eq!(
        count(&log, "clone"),
        0,
        "the objects were still ours: {:?}",
        subcommands(&log)
    );
    assert_eq!(tree_probes(&log), 1, "and the tree was probed again");
    drop(repaired);

    // An object store this capability cannot recognise is discarded, not
    // fetched into: the clone re-creates it with its full walk.
    let objects = walkdir::WalkDir::new(&cache)
        .into_iter()
        .map(Result::unwrap)
        .find(|entry| entry.file_name() == "objects.git")
        .unwrap()
        .into_path();
    fs::remove_file(objects.join("vibe-object-cache.txt")).unwrap();
    // Damage the current generation so the next open must acquire again.
    let state_path = objects.parent().unwrap().join("native.toml");
    let state: super::native_cache::NativeSnapshotState =
        toml::from_str(&fs::read_to_string(state_path).unwrap()).unwrap();
    fs::remove_dir_all(
        objects
            .parent()
            .unwrap()
            .join(state.generation)
            .join("tree"),
    )
    .unwrap();
    fs::write(&log, "").unwrap();
    let recloned = open_fresh(&url, &cache, &home, &executable, 3600);
    assert_eq!(
        recloned.list_versions(&group, "tool").unwrap(),
        vec![semver::Version::new(1, 0, 0)]
    );
    assert_eq!(count(&log, "clone"), 1, "{:?}", subcommands(&log));
    assert_eq!(count(&log, "fsck"), 2, "a fresh store walks every object");
    drop(recloned);

    // A changed global `gitattributes` is a different question for the
    // admission probe, so the verdict is not inherited.
    fs::write(&log, "").unwrap();
    fs::create_dir_all(home.join(".config/git")).unwrap();
    fs::write(home.join(".config/git/attributes"), "* -text\n").unwrap();
    let reprobed = open_fresh(&url, &cache, &home, &executable, 3600);
    assert_eq!(
        reprobed.list_versions(&group, "tool").unwrap(),
        vec![semver::Version::new(1, 0, 0)]
    );
    assert_eq!(
        tree_probes(&log),
        1,
        "the tree is probed again under the new attributes: {:?}",
        subcommands(&log)
    );
    assert_eq!(count(&log, "clone"), 0, "{:?}", subcommands(&log));
}

#[test]
fn offline_miss_never_contacts_and_refresh_ignores_ttl() {
    use super::native::{Posture, open_with_posture};
    let root = tempfile::tempdir().unwrap();
    let (bare, _) = monorepo(root.path());
    let log = root.path().join("git.log");
    let executable = counting_git(root.path(), &log);
    let home = root.path().join("home");
    let cache = root.path().join("registries");
    let url = format!("file://{}", bare.display());
    let open = |posture| {
        open_with_posture(
            &url,
            &[],
            "main",
            &cache,
            backend(&home, &executable),
            3600,
            posture,
        )
    };
    assert!(open(Posture::Offline).is_err());
    assert!(subcommands(&log).is_empty());
    let first = open(Posture::Cached).unwrap();
    fs::write(&log, "").unwrap();
    open(Posture::Offline).unwrap();
    assert_eq!(count(&log, "ls-remote"), 0);
    assert_eq!(count(&log, "fetch"), 0);
    fs::write(&log, "").unwrap();
    open(Posture::Refresh).unwrap();
    assert_eq!(count(&log, "ls-remote"), 1);
    assert_eq!(count(&log, "fetch"), 0);
    fs::write(
        first.clone_dir().join("org.corp/tool/v1.0.0/vibe.toml"),
        "damaged",
    )
    .unwrap();
    fs::write(&log, "").unwrap();
    assert!(open(Posture::Offline).is_err());
    assert_eq!(count(&log, "clone"), 0);
    assert_eq!(count(&log, "fetch"), 0);
    assert_eq!(count(&log, "ls-remote"), 0);
}
