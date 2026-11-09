//! `vibe registry publish` into `monorepo` and `localdir` registries
//! (PROP-002 `#registry-type`): plain-git push over SSH with no host API,
//! immutable versions with `--replace`, one retry on a non-fast-forward
//! race, and publish targets read from `~/.vibe/registry.toml`.
#![cfg(unix)]

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::UserScratch;

fn git(cwd: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(cwd)
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=f@example.invalid",
        ])
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

fn package(root: &Path, dir: &str, version: &str, payload: &str) -> PathBuf {
    let source = root.join(dir);
    fs::create_dir_all(&source).unwrap();
    fs::write(
        source.join("vibe.toml"),
        format!("[package]\ngroup='org.corp'\nname='demo'\nkind='feat'\nversion='{version}'\n"),
    )
    .unwrap();
    fs::write(source.join("payload"), payload).unwrap();
    source
}

fn project(root: &Path, registries: &str) -> PathBuf {
    let project = root.join("project");
    fs::create_dir_all(&project).unwrap();
    fs::write(
        project.join("vibe.toml"),
        format!("[project]\nname='publisher'\nversion='0.1.0'\n{registries}"),
    )
    .unwrap();
    project
}

fn publish(
    user: &UserScratch,
    project: &Path,
    source: &Path,
    extra: &[&str],
) -> std::process::Output {
    user.vibe()
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args(["--json", "registry", "publish"])
        .arg(source)
        .arg("--path")
        .arg(project)
        .args(extra)
        .output()
        .unwrap()
}

fn report(output: &std::process::Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn localdir_publish_reads_the_user_registry_file_and_keeps_versions_immutable() {
    let user = UserScratch::new();
    let tmp = tempfile::tempdir().unwrap();
    let drop = tmp.path().join("share");
    fs::create_dir_all(&drop).unwrap();
    // The target is declared only in the machine `registry.toml`.
    fs::write(
        user.settings.join("registry.toml"),
        format!(
            "[[registry]]\nname='share'\ntype='LocalDir'\nurl='file://{}'\n",
            drop.display()
        ),
    )
    .unwrap();
    let project = project(tmp.path(), "");
    let first = package(tmp.path(), "one", "1.0.0", "one\n");

    let created = report(&publish(&user, &project, &first, &[]));
    assert_eq!(created["mode"], "localdir");
    assert_eq!(created["action"], "created");
    assert_eq!(created["package_path"], "org.corp/demo/v1.0.0");
    // The report is the schema's document (`registry_layout_publish_report`),
    // byte-for-byte what the generated type serialises.
    let typed: vibe_wire::generated::registry_layout_publish_report::RegistryLayoutPublishReport =
        serde_json::from_value(created.clone()).unwrap();
    assert_eq!(serde_json::to_value(&typed).unwrap(), created);
    assert!(
        typed.commit.is_none(),
        "a localdir publish pushes no commit"
    );
    assert_eq!(
        fs::read_to_string(drop.join("org.corp/demo/v1.0.0/payload")).unwrap(),
        "one\n"
    );
    assert_eq!(
        report(&publish(&user, &project, &first, &[]))["action"],
        "unchanged"
    );

    let changed = package(tmp.path(), "two", "1.0.0", "two\n");
    let refused = publish(&user, &project, &changed, &[]);
    assert!(!refused.status.success());
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(stderr.contains("already exists"), "{stderr}");
    assert!(stderr.contains("--replace"), "{stderr}");
    assert_eq!(
        report(&publish(&user, &project, &changed, &["--replace"]))["action"],
        "replaced"
    );
    assert_eq!(
        fs::read_to_string(drop.join("org.corp/demo/v1.0.0/payload")).unwrap(),
        "two\n"
    );

    // What was published resolves: install from the same directory.
    let consumer = tmp.path().join("consumer");
    fs::create_dir_all(&consumer).unwrap();
    fs::write(
        consumer.join("vibe.toml"),
        "[project]\nname='consumer'\nversion='0.1.0'\n[requires.packages]\n'org.corp/demo'='=1.0.0'\n",
    )
    .unwrap();
    let installed = user
        .vibe()
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .args(["--unattended", "install", "--path"])
        .arg(&consumer)
        .output()
        .unwrap();
    assert!(
        installed.status.success(),
        "{}",
        String::from_utf8_lossy(&installed.stderr)
    );
    assert!(
        fs::read_to_string(consumer.join("vibe.lock"))
            .unwrap()
            .contains("registry = \"share\"")
    );
}

/// An identical re-publish that still rewrites a stale `_index/` says so: the
/// version is unchanged, but the report does not claim nothing was written
/// (PROP-002 `#registry-index`).
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-index"
)]
fn an_unchanged_publish_that_refreshes_the_index_reports_it() {
    let user = UserScratch::new();
    let tmp = tempfile::tempdir().unwrap();
    let drop = tmp.path().join("share");
    fs::create_dir_all(&drop).unwrap();
    let project = project(
        tmp.path(),
        &format!(
            "[[registry]]\nname='share'\ntype='localdir'\nurl='file://{}'\n",
            drop.display()
        ),
    );
    let source = package(tmp.path(), "one", "1.0.0", "one\n");
    report(&publish(&user, &project, &source, &[]));
    let text = |user: &UserScratch| {
        let output = user
            .vibe()
            .env("VIBE_NO_DEFAULT_REGISTRY", "1")
            .args(["registry", "publish"])
            .arg(&source)
            .arg("--path")
            .arg(&project)
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    };
    // Nothing stale: nothing written.
    let quiet = text(&user);
    assert!(quiet.contains("nothing written"), "{quiet}");
    // A lost `_index/` is rebuilt by the identical publish, and said so.
    fs::remove_dir_all(drop.join("_index")).unwrap();
    let refreshed = text(&user);
    assert!(
        refreshed.contains("only the registry index was refreshed")
            && !refreshed.contains("nothing written"),
        "{refreshed}"
    );
    assert!(drop.join("_index/listing.txt").is_file());
}

/// Git serialises concurrent publishers: a push rejected as
/// non-fast-forward is refetched and retried once; a second loss refuses.
#[test]
fn monorepo_publish_retries_one_lost_race_then_refuses() {
    let user = UserScratch::new();
    let tmp = tempfile::tempdir().unwrap();
    let bare = tmp.path().join("registry.git");
    git(
        tmp.path(),
        &["init", "-q", "--bare", "-b", "main", bare.to_str().unwrap()],
    );
    // A competitor clone with its own package, used by the hook below.
    let competitor = tmp.path().join("competitor");
    git(
        tmp.path(),
        &[
            "clone",
            "-q",
            bare.to_str().unwrap(),
            competitor.to_str().unwrap(),
        ],
    );
    fs::write(competitor.join("README"), "registry\n").unwrap();
    git(&competitor, &["add", "-A"]);
    git(&competitor, &["commit", "-q", "-m", "seed"]);
    git(
        &competitor,
        &["push", "-q", "origin", "HEAD:refs/heads/main"],
    );

    // A post-commit hook in the publisher's temporary tree: another
    // publisher pushes right after our commit and before our push.
    let hooks = tmp.path().join("hooks");
    fs::create_dir_all(&hooks).unwrap();
    let budget = tmp.path().join("races");
    let hook = hooks.join("post-commit");
    fs::write(
        &hook,
        format!(
            "#!/bin/sh\n[ -n \"$RACE_INNER\" ] && exit 0\nleft=$(cat {budget} 2>/dev/null || echo 0)\n[ \"$left\" -gt 0 ] || exit 0\necho $((left - 1)) > {budget}\nunset GIT_DIR GIT_INDEX_FILE GIT_WORK_TREE GIT_PREFIX\nexport RACE_INNER=1\ncd {competitor}\ngit fetch -q origin main && git reset -q --hard FETCH_HEAD\ngit -c user.name=R -c user.email=r@x commit -q --allow-empty -m race\ngit push -q origin HEAD:refs/heads/main\n",
            budget = budget.display(),
            competitor = competitor.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o700)).unwrap();

    let project = project(
        tmp.path(),
        &format!(
            "[[registry]]\nname='corp'\ntype='monorepo'\nurl='file://{}'\n",
            bare.display()
        ),
    );
    let raced = |source: &Path, races: u32| {
        fs::write(&budget, races.to_string()).unwrap();
        user.vibe()
            .env("VIBE_NO_DEFAULT_REGISTRY", "1")
            .env("GIT_CONFIG_COUNT", "1")
            .env("GIT_CONFIG_KEY_0", "core.hooksPath")
            .env("GIT_CONFIG_VALUE_0", &hooks)
            .args(["--json", "registry", "publish"])
            .arg(source)
            .arg("--path")
            .arg(&project)
            .output()
            .unwrap()
    };

    let first = package(tmp.path(), "one", "1.0.0", "one\n");
    let won = report(&raced(&first, 1));
    assert_eq!(won["action"], "created");
    assert_eq!(won["retried"], true);
    let head = git(&bare, &["rev-parse", "refs/heads/main"]);
    assert_eq!(won["commit"].as_str(), Some(head.as_str()));
    // Both publishers' history is kept: ours sits on top of the racer's.
    assert_eq!(git(&bare, &["log", "-1", "--format=%s", "main~1"]), "race");
    assert_eq!(
        git(&bare, &["show", "main:org.corp/demo/v1.0.0/payload"]),
        "one"
    );

    let second = package(tmp.path(), "two", "1.1.0", "two\n");
    let lost = raced(&second, 2);
    assert!(!lost.status.success());
    let stderr = String::from_utf8_lossy(&lost.stderr);
    assert!(stderr.contains("moved twice"), "{stderr}");
    assert!(
        git(&bare, &["ls-tree", "--name-only", "main", "org.corp/demo/"]) == "org.corp/demo/v1.0.0",
        "nothing of the lost publish was pushed"
    );
}

/// The corporate case end to end: publish into the SSH monorepo with no
/// host API, then consume it from another project.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-type"
)]
fn monorepo_publish_over_ssh_then_install_from_it() {
    let fixture = common::ssh_monorepo::Fixture::new();
    let tmp = fixture.root().to_path_buf();
    let project = tmp.join("publisher");
    fs::create_dir_all(&project).unwrap();
    fs::write(
        project.join("vibe.toml"),
        format!(
            "[project]\nname='publisher'\nversion='0.1.0'\n{}",
            fixture.monorepo_row()
        ),
    )
    .unwrap();
    let source = package(&tmp, "src", "2.0.0", "published over ssh\n");
    let output = fixture
        .command(&project)
        .args(["--json", "registry", "publish"])
        .arg(&source)
        .output()
        .unwrap();
    let published = report(&output);
    assert_eq!(published["mode"], "monorepo");
    assert_eq!(published["action"], "created");
    assert_eq!(published["commit"].as_str().unwrap(), fixture.main_commit());

    let consumer = tmp.join("consumer");
    fs::create_dir_all(&consumer).unwrap();
    fs::write(
        consumer.join("vibe.toml"),
        format!(
            "[project]\nname='consumer'\nversion='0.1.0'\n[requires.packages]\n'org.corp/demo'='=2.0.0'\n{}",
            fixture.monorepo_row()
        ),
    )
    .unwrap();
    fixture.vibe(&consumer, &["--unattended", "install"]);
    let lock = fs::read_to_string(consumer.join("vibe.lock")).unwrap();
    assert!(lock.contains("#org.corp/demo/v2.0.0"), "{lock}");
}
