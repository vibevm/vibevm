//! `vibe registry sync -g` (B-227): the user home's refresh, against a
//! `git+file://` registry under a scratch settings root. Nothing here
//! reaches a hosting — the registry is a bare repository in a temp dir.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{UserScratch, git_available, make_per_package_registry, run_git, run_git_output};

/// A scratch home whose `registry.toml` names one git registry, with the
/// package `org.vibevm.world/wal@1.0.0` already cloned into its cache by a
/// project install — the state a global install leaves behind.
struct Home {
    outer: tempfile::TempDir,
    user: UserScratch,
}

impl Home {
    fn new() -> Self {
        let outer = tempfile::tempdir().unwrap();
        let org_root = make_per_package_registry(outer.path());
        let user = UserScratch::new();
        let url = format!(
            "git+file://{}",
            org_root.to_string_lossy().replace('\\', "/")
        );
        fs::write(
            user.settings.join("registry.toml"),
            format!("[[registry]]\nname = \"home\"\nurl = \"{url}\"\n"),
        )
        .unwrap();
        let project = outer.path().join("project");
        fs::create_dir_all(&project).unwrap();
        user.init_project(&project);
        user.vibe()
            .args(["install", "org.vibevm.world/wal", "--assume-yes", "--path"])
            .arg(&project)
            .assert()
            .success();
        Self { outer, user }
    }

    /// Publish `v1.1.0` of the package upstream: what the refresh must see.
    fn release_1_1_0(&self) {
        let src = self.outer.path().join("src-flow-wal");
        let manifest = src.join("vibe.toml");
        let text = fs::read_to_string(&manifest).unwrap();
        fs::write(
            &manifest,
            text.replacen("version = \"1.0.0\"", "version = \"1.1.0\"", 1),
        )
        .unwrap();
        run_git(&src, &["commit", "-am", "org.vibevm.world/wal@1.1.0"]);
        run_git(&src, &["tag", "v1.1.0"]);
        let bare = self.outer.path().join("org.vibevm.world.wal.git");
        run_git(
            &src,
            &["push", bare.to_str().unwrap(), "main", "refs/tags/v1.1.0"],
        );
    }

    /// `vibe <args>` run from a directory that is not a project.
    fn vibe(&self, args: &[&str]) -> std::process::Output {
        self.user
            .vibe()
            .current_dir(self.outer.path())
            .args(args)
            .output()
            .unwrap()
    }

    fn clone_dir(&self) -> PathBuf {
        find_dir(&self.user.cache, "org.vibevm.world.wal")
            .expect("the install left a clone in the cache")
            .join("clone")
    }
}

fn find_dir(root: &Path, name: &str) -> Option<PathBuf> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .find(|e| e.file_type().is_dir() && e.file_name() == name)
        .map(|e| e.path().to_path_buf())
}

fn json(output: &std::process::Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("one JSON report")
}

#[test]
fn sync_g_refreshes_the_cached_clones_of_the_home_registries() {
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let home = Home::new();
    home.release_1_1_0();

    let report = json(&home.vibe(&["--json", "registry", "sync", "-g"]));
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    assert_eq!(report["ok"], true);
    assert_eq!(report["command"], "registry:sync");
    assert_eq!(report["scope"], "user");
    assert_eq!(report["home"], home.user.settings.display().to_string());
    let refreshed = report["refreshed"].as_array().unwrap();
    assert_eq!(refreshed.len(), 1, "one registry in the home: {report}");
    assert_eq!(refreshed[0]["registry"], "home");
    assert_eq!(refreshed[0]["ref"], "main");
    assert_eq!(refreshed[0]["via"], "git", "a file registry has no index");
    let clones = refreshed[0]["clones"].as_array().unwrap();
    assert_eq!(clones.len(), 1);
    assert_eq!(clones[0]["group"], "org.vibevm.world");
    assert_eq!(clones[0]["name"], "wal");
    assert_eq!(clones[0]["ref"], "v1.1.0");
    assert_eq!(clones[0]["via"], "registry:home");
    assert_eq!(report["skipped"], serde_json::json!([]));

    // The clone itself now stands at the new release.
    let described = run_git_output(&home.clone_dir(), &["describe", "--tags", "--exact-match"]);
    assert_eq!(String::from_utf8_lossy(&described.stdout).trim(), "v1.1.0");

    // A second run finds nothing new and says the same thing.
    let again = json(&home.vibe(&["--json", "registry", "sync", "-g"]));
    assert_eq!(again["refreshed"], report["refreshed"]);
    assert_eq!(again["skipped"], report["skipped"]);
}

#[test]
fn plain_sync_outside_a_project_refreshes_the_home_and_draws_a_trailer() {
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let home = Home::new();
    let report = json(&home.vibe(&["--json", "registry", "sync"]));
    assert_eq!(report["scope"], "user");
    assert_eq!(report["refreshed"][0]["clones"][0]["ref"], "v1.0.0");

    let human = home.vibe(&["registry", "sync", "-g"]);
    assert!(human.status.success());
    let stdout = String::from_utf8_lossy(&human.stdout);
    assert!(
        stdout.contains("refreshed 1 registry"),
        "the trailer names the count: {stdout}"
    );
    assert!(stdout.contains("org.vibevm.world/wal @ v1.0.0"), "{stdout}");
}

#[test]
fn sync_g_refuses_offline_in_one_sentence() {
    let user = UserScratch::new();
    let out = user
        .vibe()
        .args(["--offline", "registry", "sync", "-g"])
        .output()
        .unwrap();
    assert!(!out.status.success(), "an offline refresh must refuse");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("offline posture forbids"), "{stderr}");
}
