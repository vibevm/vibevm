//! `vibe registry mirror` (PROP-002 `#registry-mirror`) and the index a
//! layout registry carries (`#registry-index`), against local fixtures
//! only: a per-package source registry of bare repositories, a bare
//! monorepo target, a working copy and a localdir. No network.
#![cfg(unix)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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

/// A per-package source registry: `<org>/org.corp.demo.git`, one tag per
/// version, the bytes of each tag chosen by the test.
struct Source {
    org: PathBuf,
    work: PathBuf,
}

impl Source {
    fn new(root: &Path) -> Self {
        let org = root.join("org");
        fs::create_dir_all(&org).unwrap();
        let bare = org.join("org.corp.demo.git");
        git(
            root,
            &["init", "-q", "--bare", "-b", "main", bare.to_str().unwrap()],
        );
        let work = root.join("source-work");
        git(
            root,
            &[
                "clone",
                "-q",
                bare.to_str().unwrap(),
                work.to_str().unwrap(),
            ],
        );
        Source { org, work }
    }

    fn url(&self) -> String {
        format!("file://{}", self.org.display())
    }

    /// Commit `payload` as `version` and (re)tag it `v<version>`.
    fn tag(&self, version: &str, payload: &str, frozen: bool) {
        fs::write(
            self.work.join("vibe.toml"),
            format!(
                "[package]\ngroup='org.corp'\nname='demo'\nkind='feat'\nversion='{version}'\n\
                 description='The quokkaword demo package'\nkeywords=['quokkaword']\n{}",
                if frozen { "frozen=true\n" } else { "" }
            ),
        )
        .unwrap();
        fs::write(self.work.join("payload"), payload).unwrap();
        git(&self.work, &["add", "-A"]);
        git(
            &self.work,
            &["commit", "-q", "--allow-empty", "-m", version],
        );
        git(&self.work, &["tag", "-f", &format!("v{version}")]);
        git(
            &self.work,
            &["push", "-q", "-f", "origin", "HEAD:refs/heads/main"],
        );
        git(&self.work, &["push", "-q", "-f", "origin", "--tags"]);
    }
}

fn project(root: &Path, body: &str) -> PathBuf {
    let project = root.join("project");
    fs::create_dir_all(&project).unwrap();
    fs::write(
        project.join("vibe.toml"),
        format!("[project]\nname='consumer'\nversion='0.1.0'\n{body}"),
    )
    .unwrap();
    project
}

fn vibe(user: &UserScratch, project: &Path, args: &[&str]) -> Output {
    user.vibe()
        .current_dir(project)
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "f@example.invalid")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "f@example.invalid")
        .args(args)
        .output()
        .unwrap()
}

fn json(output: &Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    // The report is the schema's document.
    let _: vibe_wire::generated::registry_mirror_report::RegistryMirrorReport =
        serde_json::from_value(report.clone()).unwrap();
    report
}

fn actions(report: &serde_json::Value) -> Vec<(String, String)> {
    report["versions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            (
                v["version"].as_str().unwrap().to_string(),
                v["action"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

const SOURCE_ROW: &str = "[[registry]]\nname='src'\ntype='git'\nurl='{url}'\nindex_url='none'\n";

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror"
)]
fn mirror_saves_then_syncs_incrementally_with_snapshot_and_frozen_rules() {
    let user = UserScratch::new();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let source = Source::new(root);
    source.tag("1.0.0", "one\n", false);
    source.tag("1.1.0", "eleven\n", false);
    let target = root.join("target.git");
    git(
        root,
        &[
            "init",
            "-q",
            "--bare",
            "-b",
            "main",
            target.to_str().unwrap(),
        ],
    );
    let project = project(root, &SOURCE_ROW.replace("{url}", &source.url()));
    let into = target.to_str().unwrap();
    let mirror = |extra: &[&str]| {
        let mut args = vec![
            "--json",
            "registry",
            "mirror",
            "src",
            "--into",
            into,
            "--package",
            "org.corp/demo",
            "--name",
            "corp",
        ];
        args.extend_from_slice(extra);
        vibe(&user, &project, &args)
    };

    // Without an index the source cannot be listed, so `--package` is needed.
    let refused = vibe(
        &user,
        &project,
        &["registry", "mirror", "src", "--into", into],
    );
    assert!(!refused.status.success());
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("--package"),
        "{}",
        String::from_utf8_lossy(&refused.stderr)
    );

    // A dry run lists, writes nothing.
    let dry = json(&mirror(&["--dry-run"]));
    assert_eq!(
        actions(&dry),
        pairs(&[("1.0.0", "copied"), ("1.1.0", "copied")])
    );
    assert_eq!(dry["dry_run"], true);
    assert!(git(root, &["--git-dir", into, "for-each-ref"]).is_empty());

    // Save.
    let saved = json(&mirror(&[]));
    assert_eq!(
        actions(&saved),
        pairs(&[("1.0.0", "copied"), ("1.1.0", "copied")])
    );
    assert_eq!(saved["listing"], "tags");
    let head = git(root, &["--git-dir", into, "rev-parse", "main"]);
    assert_eq!(saved["commit"].as_str(), Some(head.as_str()));
    assert_eq!(
        git(
            root,
            &[
                "--git-dir",
                into,
                "show",
                "main:org.corp/demo/v1.0.0/payload"
            ]
        ),
        "one"
    );
    // Provenance: the source repository, tag and commit.
    let primary = git(
        root,
        &["--git-dir", into, "show", "main:_index/primary.jsonl"],
    );
    let first: serde_json::Value = serde_json::from_str(primary.lines().next().unwrap()).unwrap();
    assert!(
        first["source_url"]
            .as_str()
            .unwrap()
            .ends_with("org.corp.demo.git"),
        "{first}"
    );
    assert_eq!(first["source_ref"], "v1.0.0");
    assert_eq!(
        first["resolved_commit"].as_str().unwrap(),
        git(&source.work, &["rev-parse", "v1.0.0^{commit}"])
    );

    // A rerun with nothing new is a no-op.
    let again = json(&mirror(&[]));
    assert_eq!(
        actions(&again),
        pairs(&[("1.0.0", "unchanged"), ("1.1.0", "unchanged")])
    );
    assert!(again.get("commit").is_none(), "{again}");
    assert_eq!(git(root, &["--git-dir", into, "rev-parse", "main"]), head);

    // Incremental: only the new version is copied.
    source.tag("1.2.0", "twelve\n", false);
    let synced = json(&mirror(&[]));
    assert_eq!(
        actions(&synced),
        pairs(&[
            ("1.0.0", "unchanged"),
            ("1.1.0", "unchanged"),
            ("1.2.0", "copied")
        ])
    );

    // A snapshot that moved at the source is reported, and updated only on
    // request.
    source.tag("1.0.0", "one, moved\n", false);
    let moved = json(&mirror(&[]));
    assert_eq!(moved["versions"][0]["action"], "snapshot-differs");
    let head = git(root, &["--git-dir", into, "rev-parse", "main"]);
    let updated = json(&mirror(&["--update-snapshots"]));
    assert_eq!(updated["versions"][0]["action"], "updated");
    assert_ne!(git(root, &["--git-dir", into, "rev-parse", "main"]), head);
    assert_eq!(
        git(
            root,
            &[
                "--git-dir",
                into,
                "show",
                "main:org.corp/demo/v1.0.0/payload"
            ]
        ),
        "one, moved"
    );

    // A frozen version that changes is an alarm: nothing is written.
    source.tag("2.0.0", "frozen\n", true);
    json(&mirror(&[]));
    source.tag("2.0.0", "frozen, rewritten\n", true);
    let head = git(root, &["--git-dir", into, "rev-parse", "main"]);
    let alarm = mirror(&["--update-snapshots"]);
    assert!(!alarm.status.success());
    let report: serde_json::Value = serde_json::from_slice(&alarm.stdout).unwrap();
    assert_eq!(report["ok"], false);
    let row = report["versions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["version"] == "2.0.0")
        .unwrap();
    assert_eq!(row["action"], "alarm");
    assert_eq!(row["frozen"], true);
    assert_eq!(git(root, &["--git-dir", into, "rev-parse", "main"]), head);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror"
)]
fn mirror_into_a_working_copy_commits_without_pushing_and_refuses_a_dirty_one() {
    let user = UserScratch::new();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let source = Source::new(root);
    source.tag("1.0.0", "one\n", false);
    let bare = root.join("target.git");
    git(
        root,
        &["init", "-q", "--bare", "-b", "main", bare.to_str().unwrap()],
    );
    let wc = root.join("wc");
    git(
        root,
        &["clone", "-q", bare.to_str().unwrap(), wc.to_str().unwrap()],
    );
    fs::write(wc.join("README.md"), "corp registry\n").unwrap();
    git(&wc, &["add", "README.md"]);
    git(&wc, &["commit", "-q", "-m", "init"]);
    git(&wc, &["push", "-q", "origin", "HEAD:refs/heads/main"]);
    let pushed = git(&wc, &["rev-parse", "HEAD"]);

    let project = project(root, &SOURCE_ROW.replace("{url}", &source.url()));
    let into = wc.to_str().unwrap();
    let args = [
        "--json",
        "registry",
        "mirror",
        "src",
        "--into",
        into,
        "--package",
        "org.corp/demo",
    ];
    let report = json(&vibe(&user, &project, &args));
    assert_eq!(report["target_kind"], "working-copy");
    let head = git(&wc, &["rev-parse", "HEAD"]);
    assert_eq!(report["commit"].as_str(), Some(head.as_str()));
    assert_eq!(
        git(&bare, &["rev-parse", "main"]),
        pushed,
        "a working copy is never pushed"
    );
    assert_eq!(git(&wc, &["status", "--porcelain"]), "");

    source.tag("1.1.0", "eleven\n", false);
    fs::write(wc.join("notes.txt"), "work in progress\n").unwrap();
    let dirty = vibe(&user, &project, &args);
    assert!(!dirty.status.success());
    assert!(
        String::from_utf8_lossy(&dirty.stderr).contains("uncommitted"),
        "{}",
        String::from_utf8_lossy(&dirty.stderr)
    );
    assert_eq!(git(&wc, &["rev-parse", "HEAD"]), head);
}

/// After a mirror, a project installed from the source moves onto the
/// monorepo with `vibe update --sources-only` — same versions, same content
/// hashes — and `vibe search` finds the package by a manifest keyword in
/// the monorepo's own index.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-index"
)]
fn a_mirrored_monorepo_takes_over_the_lock_and_answers_search() {
    let user = UserScratch::new();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let source = Source::new(root);
    source.tag("1.0.0", "one\n", false);
    let target = root.join("target.git");
    git(
        root,
        &[
            "init",
            "-q",
            "--bare",
            "-b",
            "main",
            target.to_str().unwrap(),
        ],
    );
    let requires = "[requires.packages]\n'org.corp/demo'='=1.0.0'\n";
    let project = project(
        root,
        &format!("{requires}{}", SOURCE_ROW.replace("{url}", &source.url())),
    );
    let installed = vibe(&user, &project, &["--unattended", "install"]);
    assert!(
        installed.status.success(),
        "{}",
        String::from_utf8_lossy(&installed.stderr)
    );
    let lock_row = |project: &Path| -> toml::Value {
        let lock: toml::Value =
            toml::from_str(&fs::read_to_string(project.join("vibe.lock")).unwrap()).unwrap();
        lock["package"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["name"].as_str() == Some("demo"))
            .cloned()
            .unwrap()
    };
    let before = lock_row(&project);
    assert_eq!(before["registry"].as_str(), Some("src"));

    let into = target.to_str().unwrap();
    json(&vibe(
        &user,
        &project,
        &[
            "--json",
            "registry",
            "mirror",
            "src",
            "--into",
            into,
            "--package",
            "org.corp/demo",
            "--name",
            "corp",
        ],
    ));

    // The organisation now reads only the monorepo.
    fs::write(
        project.join("vibe.toml"),
        format!(
            "[project]\nname='consumer'\nversion='0.1.0'\n{requires}\
             [[registry]]\nname='corp'\ntype='monorepo'\nurl='file://{}'\n",
            target.display()
        ),
    )
    .unwrap();
    let moved = vibe(
        &user,
        &project,
        &["--unattended", "update", "--sources-only"],
    );
    assert!(
        moved.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&moved.stdout),
        String::from_utf8_lossy(&moved.stderr)
    );
    let after = lock_row(&project);
    assert_eq!(after["registry"].as_str(), Some("corp"));
    assert_eq!(after["version"], before["version"]);
    assert_eq!(after["content_hash"], before["content_hash"]);

    // Search reads the monorepo's own index: a keyword only the manifest
    // carries.
    let search = vibe(&user, &project, &["--json", "search", "quokkaword"]);
    assert!(
        search.status.success(),
        "{}",
        String::from_utf8_lossy(&search.stderr)
    );
    let found: serde_json::Value = serde_json::from_slice(&search.stdout).unwrap();
    assert_eq!(found["registries_searched"][0], "corp", "{found}");
    assert_eq!(found["hits"][0]["name"], "demo", "{found}");
    assert_eq!(found["hits"][0]["source"], "index", "{found}");

    // `vibe registry test` names the state of the index.
    let tested = vibe(&user, &project, &["--json", "registry", "test"]);
    let tested: serde_json::Value = serde_json::from_slice(&tested.stdout).unwrap();
    assert_eq!(tested["registries"][0]["status"], "reachable", "{tested}");
    assert!(
        tested["registries"][0]["note"]
            .as_str()
            .unwrap()
            .contains("index: current"),
        "{tested}"
    );
}

/// A localdir filled by hand (no index at all, then a version added without
/// vibe) is still searched — through the in-memory projection — and a
/// publish into it brings the index up to date.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-index"
)]
fn a_hand_filled_localdir_is_searched_through_the_in_memory_projection() {
    let user = UserScratch::new();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let dir = root.join("shared");
    let version = dir.join("org.corp/demo/v1.0.0");
    fs::create_dir_all(&version).unwrap();
    fs::write(
        version.join("vibe.toml"),
        "[package]\ngroup='org.corp'\nname='demo'\nkind='feat'\nversion='1.0.0'\n\
         keywords=['wombatword']\n",
    )
    .unwrap();
    let project = project(
        root,
        &format!(
            "[[registry]]\nname='share'\ntype='localdir'\nurl='file://{}'\n",
            dir.display()
        ),
    );
    let search = vibe(&user, &project, &["--json", "search", "wombatword"]);
    assert!(
        search.status.success(),
        "{}",
        String::from_utf8_lossy(&search.stderr)
    );
    let found: serde_json::Value = serde_json::from_slice(&search.stdout).unwrap();
    assert_eq!(found["hits"][0]["name"], "demo", "{found}");
    assert!(!dir.join("_index").exists(), "a reader never writes");

    // A publish writes the version and the index beside it.
    let package = root.join("pkg");
    fs::create_dir_all(&package).unwrap();
    fs::write(
        package.join("vibe.toml"),
        "[package]\ngroup='org.corp'\nname='demo'\nkind='feat'\nversion='1.1.0'\n",
    )
    .unwrap();
    let published = vibe(
        &user,
        &project,
        &["--json", "registry", "publish", package.to_str().unwrap()],
    );
    assert!(
        published.status.success(),
        "{}",
        String::from_utf8_lossy(&published.stderr)
    );
    let listing = fs::read_to_string(dir.join("_index/listing.txt")).unwrap();
    assert!(listing.contains("org.corp/demo/v1.0.0\n"), "{listing}");
    assert!(listing.contains("org.corp/demo/v1.1.0\n"), "{listing}");
    let primary = fs::read_to_string(dir.join("_index/primary.jsonl")).unwrap();
    assert_eq!(primary.lines().count(), 2, "{primary}");
}

/// The mirror keeps what a reverse mirror needs: the version directory is
/// the tagged commit's exact Git tree (modes and symlinks included), and
/// `_index/mirrors/<source>.toml` records the tag and commit facts — enough
/// to recreate, in a fresh per-package repository, a tag whose tree hash is
/// the source's. (The reverse step itself is this test's helper, not a
/// command.)
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror"
)]
fn the_mirror_record_is_enough_to_recreate_the_per_package_tag() {
    use std::os::unix::fs::PermissionsExt;

    let user = UserScratch::new();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let source = Source::new(root);
    // Every tree feature a payload copy could lose.
    let tool = source.work.join("bin/tool.sh");
    fs::create_dir_all(tool.parent().unwrap()).unwrap();
    fs::write(&tool, "#!/bin/sh\necho tool\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).unwrap();
    std::os::unix::fs::symlink("bin/tool.sh", source.work.join("tool")).unwrap();
    fs::write(source.work.join(".gitignore"), "payload\n").unwrap();
    source.tag("1.0.0", "one\n", false);
    // Re-tag as an annotated tag with its own message.
    git(
        &source.work,
        &["tag", "-f", "-a", "v1.0.0", "-m", "Release 1.0.0\n\nNotes."],
    );
    git(&source.work, &["push", "-q", "-f", "origin", "--tags"]);
    let source_tree = git(&source.work, &["rev-parse", "v1.0.0^{tree}"]);

    let target = root.join("target.git");
    git(
        root,
        &[
            "init",
            "-q",
            "--bare",
            "-b",
            "main",
            target.to_str().unwrap(),
        ],
    );
    let project = project(root, &SOURCE_ROW.replace("{url}", &source.url()));
    let into = target.to_str().unwrap();
    let report = json(&vibe(
        &user,
        &project,
        &[
            "--json",
            "registry",
            "mirror",
            "src",
            "--into",
            into,
            "--package",
            "org.corp/demo",
            "--keep-history",
        ],
    ));
    let row = &report["versions"][0];
    assert_eq!(row["tree"].as_str(), Some(source_tree.as_str()), "{report}");
    assert_eq!(row["tree_verified"], true, "{report}");
    assert_eq!(report["bundles"], 1, "{report}");

    // The monorepo directory is the source tree, byte for byte and mode for
    // mode — even the file the package's own `.gitignore` names.
    assert_eq!(
        git(
            root,
            &["--git-dir", into, "rev-parse", "main:org.corp/demo/v1.0.0"]
        ),
        source_tree
    );

    // The record names the tag exactly.
    let record: toml::Value = toml::from_str(&git(
        root,
        &["--git-dir", into, "show", "main:_index/mirrors/src.toml"],
    ))
    .unwrap();
    assert_eq!(record["source"]["name"].as_str(), Some("src"));
    assert_eq!(record["source"]["naming"].as_str(), Some("fqdn"));
    let version = &record["version"][0];
    assert_eq!(version["tag"].as_str(), Some("v1.0.0"));
    assert_eq!(version["tag_kind"].as_str(), Some("annotated"));
    assert_eq!(version["tree"].as_str(), Some(source_tree.as_str()));
    assert_eq!(
        version["commit"].as_str().unwrap(),
        git(&source.work, &["rev-parse", "v1.0.0^{commit}"])
    );

    // The reverse step: a fresh per-package repository from the monorepo
    // tree and the recorded facts.
    let reverse = root.join("reverse");
    git(
        root,
        &["init", "-q", "-b", "main", reverse.to_str().unwrap()],
    );
    git(&reverse, &["fetch", "-q", into, "main:refs/monorepo/main"]);
    let tree = git(
        &reverse,
        &["rev-parse", "refs/monorepo/main:org.corp/demo/v1.0.0"],
    );
    let person = |who: &str, field: &str| version[who][field].as_str().unwrap().to_string();
    let commit = {
        let output = Command::new("git")
            .current_dir(&reverse)
            .args([
                "commit-tree",
                &tree,
                "-m",
                version["message"].as_str().unwrap(),
            ])
            .env("GIT_AUTHOR_NAME", person("author", "name"))
            .env("GIT_AUTHOR_EMAIL", person("author", "email"))
            .env("GIT_AUTHOR_DATE", person("author", "date"))
            .env("GIT_COMMITTER_NAME", person("committer", "name"))
            .env("GIT_COMMITTER_EMAIL", person("committer", "email"))
            .env("GIT_COMMITTER_DATE", person("committer", "date"))
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    };
    let tagged = Command::new("git")
        .current_dir(&reverse)
        .args(["tag", "-a", "v1.0.0", &commit, "-m"])
        .arg(version["tag_message"].as_str().unwrap().trim_end())
        .env("GIT_COMMITTER_NAME", person("tagger", "name"))
        .env("GIT_COMMITTER_EMAIL", person("tagger", "email"))
        .env("GIT_COMMITTER_DATE", person("tagger", "date"))
        .output()
        .unwrap();
    assert!(tagged.status.success());
    assert_eq!(git(&reverse, &["rev-parse", "v1.0.0^{tree}"]), source_tree);
    assert_eq!(git(&reverse, &["cat-file", "-t", "v1.0.0"]), "tag");

    // With `--keep-history` the exact commits come back too.
    let bundle = root.join("demo.bundle");
    fs::write(
        &bundle,
        Command::new("git")
            .args([
                "--git-dir",
                into,
                "show",
                "main:_index/mirrors/bundles/src/org.corp.demo.bundle",
            ])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    let heads = git(root, &["bundle", "list-heads", bundle.to_str().unwrap()]);
    assert!(
        heads.contains(&git(&source.work, &["rev-parse", "v1.0.0"])),
        "{heads}"
    );
}

/// Into a `localdir` the tree is checked out (symlinks and modes kept) and
/// its Git tree hash verified before the version is renamed into place.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror"
)]
fn mirror_into_a_localdir_keeps_the_exact_tree() {
    let user = UserScratch::new();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let source = Source::new(root);
    std::os::unix::fs::symlink("payload", source.work.join("link")).unwrap();
    source.tag("1.0.0", "one\n", false);
    let shared = root.join("shared");
    fs::create_dir_all(&shared).unwrap();
    let project = project(
        root,
        &format!(
            "{}[[registry]]\nname='share'\ntype='localdir'\nurl='file://{}'\n",
            SOURCE_ROW.replace("{url}", &source.url()),
            shared.display()
        ),
    );
    let report = json(&vibe(
        &user,
        &project,
        &[
            "--json",
            "registry",
            "mirror",
            "src",
            "--into",
            "share",
            "--package",
            "org.corp/demo",
        ],
    ));
    assert_eq!(report["target_kind"], "localdir");
    assert_eq!(report["versions"][0]["tree_verified"], true, "{report}");
    let link = shared.join("org.corp/demo/v1.0.0/link");
    assert_eq!(fs::read_link(&link).unwrap(), PathBuf::from("payload"));
    assert!(shared.join("_index/mirrors/src.toml").is_file());
}

/// The mirror's own Git children take the configured `[proxy]` route
/// (PROP-068 `#proxy-git`). The fake proxy answers the tag listing of a
/// plain-HTTP source with one tag, so the mirror goes on to fetch it: that
/// fetch's `POST .../git-upload-pack` reaches the proxy only when the
/// mirror's own `git fetch` is routed (unrouted, git would resolve the
/// `.invalid` host directly and fail before any request).
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-068#proxy")]
fn mirror_git_children_take_the_configured_proxy() {
    use std::io::{BufRead, BufReader, Write};
    fn pkt(line: &str) -> String {
        format!("{:04x}{line}", line.len() + 4)
    }
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (heads, seen) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut head = String::new();
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                    break;
                }
                head.push_str(&line);
            }
            let reply = if head.starts_with("GET ")
                && head.contains("/info/refs?service=git-upload-pack")
            {
                let body = format!(
                    "{}0000{}0000",
                    pkt("# service=git-upload-pack\n"),
                    pkt(&format!(
                        "{} refs/tags/v1.0.0\0multi_ack_detailed side-band-64k\n",
                        "1".repeat(40)
                    ))
                );
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/x-git-upload-pack-advertisement\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
            } else {
                "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    .to_owned()
            };
            let _ = (&stream).write_all(reply.as_bytes());
            let _ = heads.send(head);
        }
    });
    let user = UserScratch::new();
    fs::write(
        user.settings.join("config.toml"),
        format!("[proxy]\nhttp = 'http://127.0.0.1:{port}'\n"),
    )
    .unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let target = root.join("target.git");
    git(
        root,
        &[
            "init",
            "-q",
            "--bare",
            "-b",
            "main",
            target.to_str().unwrap(),
        ],
    );
    let project = project(
        root,
        &SOURCE_ROW.replace("{url}", "http://git.mirror-fixture.invalid/corp"),
    );
    let output = vibe(
        &user,
        &project,
        &[
            "--json",
            "registry",
            "mirror",
            "src",
            "--into",
            target.to_str().unwrap(),
            "--package",
            "org.corp/demo",
        ],
    );
    let heads: Vec<String> = seen.try_iter().collect();
    assert!(
        heads.iter().any(
            |head| head.starts_with("POST http://git.mirror-fixture.invalid/")
                && head.contains("/git-upload-pack ")
                && head.to_ascii_lowercase().contains("user-agent: git/")
        ),
        "the mirror's fetch did not go through the proxy: {heads:?}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
