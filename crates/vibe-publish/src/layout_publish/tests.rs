//! Layout publish: immutability, identical no-op, `--replace`, and the
//! monorepo push against a local bare repository.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::*;
use vibe_index::journal::Event;

fn package(root: &Path, version: &str, payload: &str) -> PathBuf {
    let dir = root.join(format!("src-{version}-{}", payload.len()));
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        format!("[package]\ngroup='org.corp'\nname='demo'\nkind='feat'\nversion='{version}'\n"),
    )
    .unwrap();
    fs::write(dir.join("payload"), payload).unwrap();
    // Never published: a `.git` directory in the source.
    fs::create_dir_all(dir.join(".git")).unwrap();
    fs::write(dir.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
    dir
}

fn config(source: PathBuf, target: LayoutTarget, replace: bool) -> LayoutPublishConfig {
    let url = match &target {
        LayoutTarget::Monorepo { url, .. } => url.clone(),
        LayoutTarget::Localdir { root } => format!("file://{}", root.display()),
        LayoutTarget::WorkingCopy { path } => path.display().to_string(),
    };
    LayoutPublishConfig {
        source_dir: source,
        target,
        registry: LayoutRegistry {
            name: "corp".into(),
            url,
        },
        replace,
        dry_run: false,
    }
}

fn git(cwd: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn localdir_creates_then_is_immutable_unless_replaced() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("registry");
    fs::create_dir_all(&root).unwrap();
    let target = LayoutTarget::Localdir { root: root.clone() };
    let first = package(tmp.path(), "1.0.0", "one\n");

    let created = publish(
        &config(first.clone(), target.clone(), false),
        &Default::default(),
    )
    .unwrap();
    assert_eq!(created.action, LayoutAction::Created);
    assert_eq!(created.package_path, "org.corp/demo/v1.0.0");
    let version = root.join("org.corp/demo/v1.0.0");
    assert_eq!(
        fs::read_to_string(version.join("payload")).unwrap(),
        "one\n"
    );
    assert!(!version.join(".git").exists(), ".git is never published");

    let again = publish(&config(first, target.clone(), false), &Default::default()).unwrap();
    assert_eq!(again.action, LayoutAction::Unchanged);

    let changed = package(tmp.path(), "1.0.0", "two!\n");
    let refused = publish(
        &config(changed.clone(), target.clone(), false),
        &Default::default(),
    )
    .unwrap_err();
    assert!(
        matches!(refused, PublishError::VersionExists { .. }),
        "{refused}"
    );
    assert!(refused.to_string().contains("--replace"), "{refused}");
    assert_eq!(
        fs::read_to_string(version.join("payload")).unwrap(),
        "one\n"
    );

    let replaced = publish(&config(changed, target, true), &Default::default()).unwrap();
    assert_eq!(replaced.action, LayoutAction::Replaced);
    assert_eq!(
        fs::read_to_string(version.join("payload")).unwrap(),
        "two!\n"
    );
    // No temporary siblings are left beside the version directory.
    let names: Vec<String> = fs::read_dir(root.join("org.corp/demo"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["v1.0.0"]);
    // The registry index follows every write and matches the tree.
    assert_eq!(
        vibe_core::layout_registry::index_state(&root).unwrap(),
        vibe_core::layout_registry::IndexState::Current
    );
}

#[test]
fn localdir_dry_run_writes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("registry");
    fs::create_dir_all(&root).unwrap();
    let mut dry = config(
        package(tmp.path(), "1.0.0", "one\n"),
        LayoutTarget::Localdir { root: root.clone() },
        false,
    );
    dry.dry_run = true;
    let outcome = publish(&dry, &Default::default()).unwrap();
    assert_eq!(outcome.action, LayoutAction::Created);
    assert!(outcome.dry_run);
    assert!(!root.join("org.corp").exists());
}

#[test]
fn monorepo_push_creates_skips_identical_refuses_and_replaces() {
    if Command::new("git").arg("--version").output().is_err() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let bare = tmp.path().join("registry.git");
    git(
        tmp.path(),
        &["init", "-q", "--bare", bare.to_str().unwrap()],
    );
    let url = format!("file://{}", bare.display());
    let target = LayoutTarget::Monorepo {
        url: url.clone(),
        branch: "main".into(),
    };
    let first = package(tmp.path(), "1.0.0", "one\n");

    // An empty repository: the branch is created by the first publish.
    let created = publish(
        &config(first.clone(), target.clone(), false),
        &Default::default(),
    )
    .unwrap();
    assert_eq!(created.action, LayoutAction::Created);
    let head = git(&bare, &["rev-parse", "refs/heads/main"]);
    assert_eq!(created.commit.as_deref(), Some(head.as_str()));
    assert_eq!(
        git(&bare, &["show", "main:org.corp/demo/v1.0.0/payload"]),
        "one"
    );

    let again = publish(&config(first, target.clone(), false), &Default::default()).unwrap();
    assert_eq!(again.action, LayoutAction::Unchanged);
    assert_eq!(again.commit, None);
    assert_eq!(
        git(&bare, &["rev-parse", "refs/heads/main"]),
        head,
        "no new commit"
    );

    // A second version lands beside the first on the same branch.
    let second = package(tmp.path(), "1.1.0", "eleven\n");
    publish(&config(second, target.clone(), false), &Default::default()).unwrap();
    let listing = git(&bare, &["ls-tree", "--name-only", "main", "org.corp/demo/"]);
    assert_eq!(listing, "org.corp/demo/v1.0.0\norg.corp/demo/v1.1.0");
    // The index rides the same commit as the version it describes.
    let changed = git(&bare, &["show", "--name-only", "--format=", "main"]);
    assert!(
        changed.contains("org.corp/demo/v1.1.0/payload"),
        "{changed}"
    );
    assert!(changed.contains("_index/primary.jsonl"), "{changed}");
    assert!(changed.contains("_index/listing.txt"), "{changed}");
    assert!(changed.contains("_index/state/journal/"), "{changed}");
    let primary = git(&bare, &["show", "main:_index/primary.jsonl"]);
    assert_eq!(primary.lines().count(), 2, "{primary}");

    let changed = package(tmp.path(), "1.0.0", "two!\n");
    let refused = publish(
        &config(changed.clone(), target.clone(), false),
        &Default::default(),
    )
    .unwrap_err();
    assert!(
        matches!(refused, PublishError::VersionExists { .. }),
        "{refused}"
    );

    let replaced = publish(&config(changed, target, true), &Default::default()).unwrap();
    assert_eq!(replaced.action, LayoutAction::Replaced);
    assert_eq!(
        git(&bare, &["show", "main:org.corp/demo/v1.0.0/payload"]),
        "two!"
    );
    assert_eq!(
        git(&bare, &["show", "main:org.corp/demo/v1.1.0/payload"]),
        "eleven"
    );
}

fn batch(target: LayoutTarget, items: Vec<PublishItem>, facts: Vec<Event>) -> BatchConfig {
    BatchConfig {
        target,
        registry: LayoutRegistry {
            name: "corp".into(),
            url: "ssh://git@git.corp.example/vibe.git".into(),
        },
        items,
        facts,
        files: Vec::new(),
        removed: Vec::new(),
        message: Some("mirror: test".into()),
        dry_run: false,
    }
}

fn item(tree: PathBuf, version: &str) -> PublishItem {
    PublishItem {
        group: "org.corp".into(),
        name: "demo".into(),
        version: version.parse().unwrap(),
        tree,
        provenance: Some(Provenance {
            source_url: "https://github.com/vibespecs/org.corp.demo.git".into(),
            source_ref: format!("v{version}"),
            resolved_commit: Some("c0ffee".into()),
        }),
        git: None,
        replace: false,
    }
}

fn yank(version: &str) -> Event {
    Event::Yanked {
        group: vibe_core::Group::parse("org.corp").unwrap(),
        name: "demo".into(),
        version: version.parse().unwrap(),
        reason: "yanked at the source".into(),
    }
}

#[test]
fn a_batch_records_provenance_and_facts_once() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("registry");
    fs::create_dir_all(&root).unwrap();
    let target = LayoutTarget::Localdir { root: root.clone() };
    let one = package(tmp.path(), "1.0.0", "one\n");
    let two = package(tmp.path(), "1.1.0", "eleven\n");
    let config = batch(
        target,
        vec![item(one, "1.0.0"), item(two, "1.1.0")],
        vec![yank("1.0.0")],
    );
    let first = publish_batch(&config, &Default::default()).unwrap();
    assert!(first.changed());
    assert_eq!(first.facts_recorded, 3, "two publications and one yank");

    let identity = vibe_index::layout::LayoutIdentity {
        registry: "corp".into(),
        registry_url: "ssh://git@git.corp.example/vibe.git".into(),
        branch: None,
    };
    let (index, state) = vibe_index::layout::load_or_project(&root, &identity).unwrap();
    assert_eq!(state, vibe_core::layout_registry::IndexState::Current);
    let group = vibe_core::Group::parse("org.corp").unwrap();
    let versions = &index.get(&group, "demo").unwrap().versions;
    assert_eq!(
        versions[0].source_url,
        "https://github.com/vibespecs/org.corp.demo.git"
    );
    assert_eq!(versions[0].resolved_commit.as_deref(), Some("c0ffee"));
    assert!(versions[0].yanked);
    assert!(!versions[1].yanked);

    // The same batch again changes nothing: no version, no fact, no index.
    let journal = vibe_core::layout_registry::journal_dir(&root);
    let before = fs::read_dir(&journal).unwrap().count();
    let again = publish_batch(&config, &Default::default()).unwrap();
    assert!(!again.changed(), "{again:?}");
    assert_eq!(fs::read_dir(&journal).unwrap().count(), before);
}

#[test]
fn a_working_copy_is_committed_into_and_a_dirty_one_refused() {
    if Command::new("git").arg("--version").output().is_err() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let wc = tmp.path().join("wc");
    fs::create_dir_all(&wc).unwrap();
    git(&wc, &["init", "-q", "--initial-branch=main"]);
    git(&wc, &["config", "user.email", "t@t.invalid"]);
    git(&wc, &["config", "user.name", "T"]);
    fs::write(wc.join("README.md"), "registry\n").unwrap();
    git(&wc, &["add", "README.md"]);
    git(&wc, &["commit", "-q", "-m", "init"]);

    let target = LayoutTarget::WorkingCopy { path: wc.clone() };
    let one = package(tmp.path(), "1.0.0", "one\n");
    let outcome = publish_batch(
        &batch(target.clone(), vec![item(one.clone(), "1.0.0")], Vec::new()),
        &Default::default(),
    )
    .unwrap();
    let head = git(&wc, &["rev-parse", "HEAD"]);
    assert_eq!(outcome.commit.as_deref(), Some(head.as_str()));
    assert_eq!(git(&wc, &["log", "-1", "--format=%s"]), "mirror: test");
    assert_eq!(
        git(&wc, &["status", "--porcelain"]),
        "",
        "nothing left behind"
    );

    fs::write(wc.join("stray.txt"), "uncommitted\n").unwrap();
    let two = package(tmp.path(), "1.1.0", "eleven\n");
    let refused = publish_batch(
        &batch(target, vec![item(two, "1.1.0")], Vec::new()),
        &Default::default(),
    )
    .unwrap_err();
    assert!(
        matches!(refused, PublishError::DirtyWorkingCopy { .. }),
        "{refused}"
    );
    assert_eq!(git(&wc, &["rev-parse", "HEAD"]), head);
}

/// A pruned version's bytes, its index rows and its journal fact leave in
/// ONE commit (PROP-075 `#MIRROR-PRUNE`): the target is never left
/// advertising bytes it no longer has, not even for one commit.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PRUNE")]
fn a_removal_drops_the_bytes_the_rows_and_the_fact_together() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("registry");
    fs::create_dir_all(&root).unwrap();
    let one = package(tmp.path(), "1.0.0", "one");
    let two = package(tmp.path(), "1.1.0", "eleven");
    let target = LayoutTarget::Localdir { root: root.clone() };
    publish_batch(
        &batch(
            target.clone(),
            vec![item(one, "1.0.0"), item(two, "1.1.0")],
            Vec::new(),
        ),
        &Default::default(),
    )
    .unwrap();
    assert!(root.join("org.corp/demo/v1.0.0/payload").is_file());
    let primary = fs::read_to_string(root.join("_index/primary.jsonl")).unwrap();
    assert!(primary.contains("1.0.0"), "{primary}");

    let mut prune = batch(target, Vec::new(), Vec::new());
    prune.removed = vec![Removal {
        path: "org.corp/demo/v1.0.0".into(),
        version: Some(RemovedVersion {
            group: vibe_core::Group::parse("org.corp").unwrap(),
            name: "demo".into(),
            version: "1.0.0".parse().unwrap(),
        }),
    }];
    let outcome = publish_batch(&prune, &Default::default()).unwrap();
    assert_eq!(outcome.removed, 1);
    assert!(outcome.changed());

    // The bytes are gone, the catalog no longer advertises them, and the
    // journal says why.
    assert!(!root.join("org.corp/demo/v1.0.0").exists());
    let primary = fs::read_to_string(root.join("_index/primary.jsonl")).unwrap();
    assert!(!primary.contains("\"1.0.0\""), "{primary}");
    assert!(primary.contains("\"1.1.0\""), "{primary}");
    let listing = fs::read_to_string(root.join("_index/listing.txt")).unwrap();
    assert!(!listing.contains("v1.0.0"), "{listing}");
    assert!(listing.contains("v1.1.0"), "{listing}");
    let journal = fs::read_dir(root.join("_index/state/journal"))
        .unwrap()
        .flatten()
        .map(|entry| fs::read_to_string(entry.path()).unwrap())
        .collect::<String>();
    assert!(journal.contains("removed"), "{journal}");
}

/// A dry run plans the removal and deletes nothing.
#[test]
fn a_dry_run_counts_the_removal_and_deletes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("registry");
    fs::create_dir_all(&root).unwrap();
    let one = package(tmp.path(), "1.0.0", "one");
    let target = LayoutTarget::Localdir { root: root.clone() };
    publish_batch(
        &batch(target.clone(), vec![item(one, "1.0.0")], Vec::new()),
        &Default::default(),
    )
    .unwrap();

    let mut prune = batch(target, Vec::new(), Vec::new());
    prune.dry_run = true;
    prune.removed = vec![Removal {
        path: "org.corp/demo/v1.0.0".into(),
        version: Some(RemovedVersion {
            group: vibe_core::Group::parse("org.corp").unwrap(),
            name: "demo".into(),
            version: "1.0.0".parse().unwrap(),
        }),
    }];
    let outcome = publish_batch(&prune, &Default::default()).unwrap();
    assert_eq!(outcome.removed, 1);
    assert!(root.join("org.corp/demo/v1.0.0/payload").is_file());
}

/// What a batch may and may not be asked to delete.
#[test]
fn a_removal_outside_a_version_directory_or_the_index_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("registry");
    fs::create_dir_all(root.join("org.corp/demo/v1.0.0")).unwrap();
    let target = LayoutTarget::Localdir { root: root.clone() };
    let attempt = |removal: Removal| {
        let mut prune = batch(target.clone(), Vec::new(), Vec::new());
        prune.removed = vec![removal];
        publish_batch(&prune, &Default::default())
    };
    let bare = |path: &str| Removal {
        path: path.into(),
        version: None,
    };
    // Neither a version directory nor inside `_index/`.
    assert!(attempt(bare("README.md")).is_err());
    assert!(attempt(bare("org.corp/demo")).is_err());
    // A climb out of the tree, however spelled.
    assert!(attempt(bare("_index/../../etc")).is_err());
    assert!(attempt(bare("/etc/passwd")).is_err());
    assert!(attempt(bare("")).is_err());
    // A path that claims a coordinate it does not spell.
    assert!(
        attempt(Removal {
            path: "org.corp/demo/v1.0.0".into(),
            version: Some(RemovedVersion {
                group: vibe_core::Group::parse("org.corp").unwrap(),
                name: "demo".into(),
                version: "9.9.9".parse().unwrap(),
            }),
        })
        .is_err()
    );
    // Nothing was deleted by any refusal.
    assert!(root.join("org.corp/demo/v1.0.0").is_dir());

    // A path the tree does not hold is dropped, not refused: someone else
    // removing it first is the outcome asked for.
    let outcome = attempt(Removal {
        path: "org.corp/demo/v7.0.0".into(),
        version: Some(RemovedVersion {
            group: vibe_core::Group::parse("org.corp").unwrap(),
            name: "demo".into(),
            version: "7.0.0".parse().unwrap(),
        }),
    })
    .unwrap();
    assert_eq!(outcome.removed, 0);
}

/// Into a `monorepo` the deletion is **staged**, so it lands in the same
/// pushed commit as the rewritten index. This is the backend where a
/// removal could silently not happen: the file is gone from the working
/// tree and the commit still carries it.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PRUNE")]
fn a_monorepo_removal_lands_in_the_pushed_commit() {
    if Command::new("git").arg("--version").output().is_err() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let bare = tmp.path().join("registry.git");
    git(
        tmp.path(),
        &["init", "-q", "--bare", bare.to_str().unwrap()],
    );
    let target = LayoutTarget::Monorepo {
        url: format!("file://{}", bare.display()),
        branch: "main".into(),
    };
    publish_batch(
        &batch(
            target.clone(),
            vec![
                item(package(tmp.path(), "1.0.0", "one\n"), "1.0.0"),
                item(package(tmp.path(), "1.1.0", "eleven\n"), "1.1.0"),
            ],
            Vec::new(),
        ),
        &Default::default(),
    )
    .unwrap();
    assert_eq!(
        git(
            tmp.path(),
            &[
                "--git-dir",
                bare.to_str().unwrap(),
                "show",
                "main:org.corp/demo/v1.0.0/payload"
            ]
        ),
        "one"
    );

    let mut prune = batch(target, Vec::new(), Vec::new());
    prune.removed = vec![Removal {
        path: "org.corp/demo/v1.0.0".into(),
        version: Some(RemovedVersion {
            group: vibe_core::Group::parse("org.corp").unwrap(),
            name: "demo".into(),
            version: "1.0.0".parse().unwrap(),
        }),
    }];
    let outcome = publish_batch(&prune, &Default::default()).unwrap();
    assert_eq!(outcome.removed, 1);
    assert!(outcome.commit.is_some());

    // The pushed tree no longer carries the version…
    let listed = git(
        tmp.path(),
        &[
            "--git-dir",
            bare.to_str().unwrap(),
            "ls-tree",
            "-r",
            "--name-only",
            "main",
            "--",
            "org.corp/demo",
        ],
    );
    assert!(!listed.contains("v1.0.0"), "{listed}");
    assert!(listed.contains("v1.1.0"), "{listed}");
    // …and the index committed beside it no longer advertises it.
    let primary = git(
        tmp.path(),
        &[
            "--git-dir",
            bare.to_str().unwrap(),
            "show",
            "main:_index/primary.jsonl",
        ],
    );
    assert!(!primary.contains("\"1.0.0\""), "{primary}");
    assert!(primary.contains("\"1.1.0\""), "{primary}");
}
