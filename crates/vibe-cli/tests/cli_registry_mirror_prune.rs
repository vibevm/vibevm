//! `vibe registry mirror --prune` — campaign phase P4 commit (c),
//! PROP-075 `#MIRROR-PRUNE`, `#MIRROR-NO-SILENT-PRUNE`.
//!
//! The plan comes first and nothing goes without a yes; a narrowed
//! selection on its own never deletes; and on a `monorepo` the removal is
//! checked against the **pushed bare repository's** `ls-tree`, because
//! only the pushed state tells apart a deletion that was staged from one
//! that merely left the working tree.
#![cfg(unix)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::UserScratch;

/// One version of a `localdir` registry: a directory whose manifest the
/// index projection derives the entry from.
fn publish(root: &Path, group: &str, name: &str, version: &str) {
    let dir = root.join(group).join(name).join(format!("v{version}"));
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        format!(
            "[package]\ngroup='{group}'\nname='{name}'\nkind='feat'\nversion='{version}'\n\
             description='The quokkaword {name} package'\nkeywords=['quokkaword']\n"
        ),
    )
    .unwrap();
}

fn git(cwd: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(cwd)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn project(root: &Path, target: &str) -> PathBuf {
    let project = root.join("project");
    fs::create_dir_all(&project).unwrap();
    fs::write(
        project.join("vibe.toml"),
        format!(
            "[project]\nname='consumer'\nversion='0.1.0'\n\
             [[registry]]\nname='src'\ntype='localdir'\nurl='file://{}'\n{target}",
            root.join("source").display()
        ),
    )
    .unwrap();
    project
}

fn vibe(user: &UserScratch, project: &Path, args: &[&str]) -> Output {
    user.vibe()
        .current_dir(project)
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args(args)
        .output()
        .unwrap()
}

fn ok(output: &Output) -> String {
    assert!(
        output.status.success(),
        "expected success:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn refusal(output: &Output) -> String {
    assert!(
        !output.status.success(),
        "expected a refusal:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
    String::from_utf8_lossy(&output.stderr).to_string()
}

/// The target's journal, whose file is named by the month it records.
fn journal(target: &Path) -> String {
    let dir = target.join("_index/state/journal");
    fs::read_dir(&dir)
        .unwrap_or_else(|error| panic!("{}: {error}", dir.display()))
        .flatten()
        .map(|entry| fs::read_to_string(entry.path()).unwrap_or_default())
        .collect()
}

fn config(root: &Path, body: &str) -> PathBuf {
    let path = root.join("mirror.toml");
    fs::write(&path, body).unwrap();
    path
}

/// A source carrying two groups, so narrowing has something to drop.
fn source(root: &Path) {
    publish(&root.join("source"), "org.corp", "demo", "1.0.0");
    publish(&root.join("source"), "org.corp", "demo", "1.1.0");
    publish(&root.join("source"), "org.other", "tool", "2.0.0");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-NO-SILENT-PRUNE")]
fn narrowing_without_prune_deletes_nothing_and_a_plan_without_yes_changes_nothing() {
    let user = UserScratch::new();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    source(root);
    let target = root.join("target");
    fs::create_dir_all(&target).unwrap();
    let project = project(
        root,
        &format!(
            "[[registry]]\nname='dst'\ntype='localdir'\nurl='file://{}'\n",
            target.display()
        ),
    );
    let wide = config(root, "schema = 1\n[select]\nall = true\n");
    let run = |user: &UserScratch, config: &Path, extra: &[&str]| {
        let mut args = vec![
            "--json",
            "registry",
            "mirror",
            "src",
            "--into",
            "dst",
            "--config",
            config.to_str().unwrap(),
        ];
        args.extend_from_slice(extra);
        vibe(user, &project, &args)
    };
    ok(&run(&user, &wide, &[]));
    assert!(target.join("org.corp/demo/v1.1.0/vibe.toml").is_file());
    assert!(target.join("org.other/tool/v2.0.0/vibe.toml").is_file());

    // Narrowing on its own never deletes, however narrow it became.
    let narrow = config(root, "schema = 1\n[select]\ngroups = ['org.corp']\n");
    ok(&run(&UserScratch::new(), &narrow, &[]));
    assert!(
        target.join("org.other/tool/v2.0.0/vibe.toml").is_file(),
        "a run without --prune must delete nothing"
    );

    // `--prune` prints the plan and refuses rather than remove without a
    // word; the tree is untouched.
    let message = refusal(&run(&UserScratch::new(), &narrow, &["--prune"]));
    assert!(message.contains("prune_consent_required"), "{message}");
    assert!(message.contains("nothing was written"), "{message}");
    assert!(message.contains("`--yes`"), "{message}");
    assert!(target.join("org.other/tool/v2.0.0/vibe.toml").is_file());

    // `--dry-run --prune` prints the plan and stops, successfully.
    let planned = common::wire::report::<
        vibe_wire::generated::registry_mirror_report::RegistryMirrorReport,
    >(&ok(&run(
        &UserScratch::new(),
        &narrow,
        &["--prune", "--dry-run"],
    )));
    assert_eq!(planned["pruned"][0]["kind"], "version", "{planned}");
    assert_eq!(
        planned["pruned"][0]["package"], "org.other/tool",
        "{planned}"
    );
    assert_eq!(
        planned["pruned"][0]["path"], "org.other/tool/v2.0.0",
        "{planned}"
    );
    assert_eq!(
        planned["pruned"][0]["reason"], "no longer selected by `[select]`",
        "{planned}"
    );
    // The plan is a plan: nothing was removed.
    assert_eq!(planned["removed"], 0, "{planned}");

    let shown = {
        let output = vibe(
            &UserScratch::new(),
            &project,
            &[
                "registry",
                "mirror",
                "src",
                "--into",
                "dst",
                "--config",
                narrow.to_str().unwrap(),
                "--prune",
                "--dry-run",
            ],
        );
        ok(&output) + &String::from_utf8_lossy(&output.stderr)
    };
    assert!(shown.contains("--prune: 1 thing(s) to remove"), "{shown}");
    assert!(
        shown.contains("version org.other/tool@2.0.0 — no longer selected by `[select]`"),
        "{shown}"
    );
    assert!(target.join("org.other/tool/v2.0.0/vibe.toml").is_file());

    // With `--yes` the bytes, the index rows and the journal fact go
    // together. The **version directory** goes: the channel's path must
    // spell the coordinate of its fact, so an emptied `<group>/<name>/`
    // stays behind, carrying no version and no catalog row.
    ok(&run(&UserScratch::new(), &narrow, &["--prune", "--yes"]));
    assert!(!target.join("org.other/tool/v2.0.0").exists());
    assert!(target.join("org.corp/demo/v1.1.0/vibe.toml").is_file());
    let catalog = fs::read_to_string(target.join("_index/primary.jsonl")).unwrap();
    assert!(!catalog.contains("org.other"), "{catalog}");
    assert!(
        journal(&target).contains("\"kind\":\"removed\""),
        "{}",
        journal(&target)
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PRUNE")]
fn a_flag_override_is_never_a_prune_and_says_so() {
    let user = UserScratch::new();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    source(root);
    let target = root.join("target");
    fs::create_dir_all(&target).unwrap();
    let project = project(
        root,
        &format!(
            "[[registry]]\nname='dst'\ntype='localdir'\nurl='file://{}'\n",
            target.display()
        ),
    );
    let message = refusal(&vibe(
        &user,
        &project,
        &[
            "registry",
            "mirror",
            "src",
            "--into",
            "dst",
            "--package",
            "org.corp/demo",
            "--prune",
            "--yes",
        ],
    ));
    assert!(message.contains("no declared selection"), "{message}");
    assert!(message.contains("per-run override"), "{message}");
    assert!(!target.join("_index").exists());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PRUNE")]
fn a_monorepo_prune_reaches_the_pushed_bare_repository() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    source(root);
    let bare = root.join("target.git");
    git(
        root,
        &["init", "-q", "--bare", "-b", "main", bare.to_str().unwrap()],
    );
    let project = project(
        root,
        &format!(
            "[[registry]]\nname='dst'\ntype='monorepo'\nurl='file://{}'\nref='main'\n",
            bare.display()
        ),
    );
    let run = |config: &Path, extra: &[&str]| {
        let mut args = vec![
            "--json",
            "registry",
            "mirror",
            "src",
            "--into",
            "dst",
            "--config",
            config.to_str().unwrap(),
        ];
        args.extend_from_slice(extra);
        vibe(&UserScratch::new(), &project, &args)
    };
    ok(&run(
        &config(root, "schema = 1\n[select]\nall = true\n"),
        &[],
    ));
    let tree = git(&bare, &["ls-tree", "-r", "--name-only", "main"]);
    assert!(tree.contains("org.other/tool/v2.0.0/vibe.toml"), "{tree}");

    let narrow = config(root, "schema = 1\n[select]\ngroups = ['org.corp']\n");
    ok(&run(&narrow, &["--prune", "--yes"]));

    // The pushed commit is the only state that tells a staged deletion
    // from one that merely left the working tree.
    let tree = git(&bare, &["ls-tree", "-r", "--name-only", "main"]);
    assert!(!tree.contains("org.other/tool"), "{tree}");
    assert!(tree.contains("org.corp/demo/v1.1.0/vibe.toml"), "{tree}");
    let catalog = git(&bare, &["show", "main:_index/primary.jsonl"]);
    assert!(!catalog.contains("org.other"), "{catalog}");
    let committed = git(
        &bare,
        &["ls-tree", "-r", "--name-only", "main", "_index/state/"],
    );
    let month = committed
        .lines()
        .find(|path| path.ends_with(".ndjson"))
        .unwrap_or_else(|| panic!("{committed}"));
    let recorded = git(&bare, &["show", &format!("main:{month}")]);
    assert!(recorded.contains("\"kind\":\"removed\""), "{recorded}");

    // A re-run of the same narrowed selection has nothing left to plan.
    let output = vibe(
        &UserScratch::new(),
        &project,
        &[
            "registry",
            "mirror",
            "src",
            "--into",
            "dst",
            "--config",
            narrow.to_str().unwrap(),
            "--prune",
        ],
    );
    let shown = ok(&output) + &String::from_utf8_lossy(&output.stderr);
    assert!(
        shown.contains("still matches everything in the target"),
        "{shown}"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PRUNE")]
fn a_version_another_source_copied_survives_this_sources_prune() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    source(root);
    let target = root.join("target");
    fs::create_dir_all(&target).unwrap();
    let project = project(
        root,
        &format!(
            "[[registry]]\nname='dst'\ntype='localdir'\nurl='file://{}'\n",
            target.display()
        ),
    );
    ok(&vibe(
        &UserScratch::new(),
        &project,
        &[
            "--json",
            "registry",
            "mirror",
            "src",
            "--into",
            "dst",
            "--config",
            config(root, "schema = 1\n[select]\nall = true\n")
                .to_str()
                .unwrap(),
        ],
    ));

    // A second source's record claims the version this run would drop.
    // The record is written by hand because a `localdir` source keeps no
    // `[[version]]` rows of its own — only a Git one does — and the rule
    // is about the record, not about how it got there.
    fs::write(
        target.join("_index/mirrors/second.toml"),
        "schema = 1\n[source]\nname = 'second'\nurl = 'ssh://host/second'\n\
         type = 'github'\nnaming = 'fqdn'\nref = 'main'\n\
         [sync]\nvibe = 'vibe 1.0.0'\nfirst = 'now'\nlast = 'now'\n\
         [[version]]\npackage = 'org.other/tool'\nversion = '2.0.0'\n\
         repository = 'ssh://host/org.other.tool.git'\nrepo = 'org.other.tool'\n\
         tree_verified = true\ntag = 'v2.0.0'\ntag_kind = 'lightweight'\n\
         commit = 'abc'\ntree = 'def'\n\
         author = { name = 'A', email = 'a@example', date = '2026-01-01' }\n\
         committer = { name = 'A', email = 'a@example', date = '2026-01-01' }\n\
         message = 'a commit'\n",
    )
    .unwrap();

    let output = vibe(
        &UserScratch::new(),
        &project,
        &[
            "registry",
            "mirror",
            "src",
            "--into",
            "dst",
            "--config",
            config(root, "schema = 1\n[select]\ngroups = ['org.corp']\n")
                .to_str()
                .unwrap(),
            "--prune",
            "--yes",
        ],
    );
    ok(&output);
    assert!(
        target.join("org.other/tool/v2.0.0/vibe.toml").is_file(),
        "a version another source recorded as copied is never pruned by this source's run"
    );
}
