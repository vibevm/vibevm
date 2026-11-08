//! Layout publish: immutability, identical no-op, `--replace`, and the
//! monorepo push against a local bare repository.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::*;

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
    LayoutPublishConfig {
        source_dir: source,
        target,
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
