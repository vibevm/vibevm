//! `vibe registry publish` updating a local checkout of the git-hosted
//! index (B-227, PROP-005 `##INT-PUBLISH-INDEX-CHECKOUT`). The package is
//! pushed to a bare repository over `file://`; the index checkout is
//! initialised by the index library itself and given a bare upstream, so
//! "pushed" and "not pushed" are both observable. Nothing reaches a host.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{UserScratch, git_available, run_git, run_git_output};

struct Fixture {
    root: tempfile::TempDir,
    user: UserScratch,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let user = UserScratch::new();
        fs::create_dir_all(root.path().join("project")).unwrap();
        user.init_project(&root.path().join("project"));
        run_git(root.path(), &["init", "--bare", "--quiet", "tiny.git"]);
        run_git(
            root.path(),
            &["init", "--bare", "--quiet", "index-remote.git"],
        );

        let checkout = root.path().join("index");
        vibe_index::cli::init::run(vibe_index::cli::init::Args {
            data_dir: checkout.clone(),
            registry: "vibespecs".into(),
            registry_url: "file:///registry".into(),
            naming: "fqdn".into(),
            force: false,
        })
        .unwrap();
        for args in [
            &["init", "--quiet", "--initial-branch=main"][..],
            &["config", "user.email", "t@example.com"][..],
            &["config", "user.name", "Test"][..],
            &["add", "-A"][..],
            &["commit", "--quiet", "-m", "init"][..],
        ] {
            run_git(&checkout, args);
        }
        let remote = root.path().join("index-remote.git");
        run_git(
            &checkout,
            &["remote", "add", "origin", remote.to_str().unwrap()],
        );
        run_git(&checkout, &["push", "--quiet", "-u", "origin", "main"]);
        Self { root, user }
    }

    fn checkout(&self) -> PathBuf {
        self.root.path().join("index")
    }

    /// A package directory `org.vibevm/tiny@<version>`.
    fn package(&self, version: &str) -> PathBuf {
        let dir = self.root.path().join(format!("tiny-{version}"));
        fs::create_dir_all(dir.join(vibe_core::layout::current_specs_root())).unwrap();
        fs::write(
            dir.join("vibe.toml"),
            format!(
                "[package]\ngroup = \"org.vibevm\"\nname = \"tiny\"\nkind = \"flow\"\nversion = \"{version}\"\n"
            ),
        )
        .unwrap();
        fs::write(dir.join(common::spec_rel("PROTOCOL.md")), "hello\n").unwrap();
        dir
    }

    fn publish(&self, version: &str, extra: &[&str]) -> serde_json::Value {
        let bare = self.root.path().join("tiny.git");
        let url = format!(
            "file:///{}",
            bare.to_string_lossy()
                .replace('\\', "/")
                .trim_start_matches('/')
        );
        let out = self
            .user
            .vibe()
            .args(["--json", "registry", "publish"])
            .arg(self.package(version))
            .args(["--repo-url", &url, "--path"])
            .arg(self.root.path().join("project"))
            .args(extra)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "stdout={} stderr={}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).expect("one JSON report")
    }
}

fn head(repo: &Path, rev: &str) -> String {
    String::from_utf8_lossy(&run_git_output(repo, &["rev-parse", rev]).stdout)
        .trim()
        .to_string()
}

fn indexed_versions(checkout: &Path) -> Vec<String> {
    let path = checkout.join("by-name/tiny.json");
    let Ok(bytes) = fs::read(path) else {
        return Vec::new();
    };
    let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    parsed["packages"][0]["versions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["version"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn publish_adds_the_release_to_the_index_checkout_and_commits_without_pushing() {
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let fx = Fixture::new();
    let checkout = fx.checkout();
    let checkout_arg = checkout.to_str().unwrap();

    let report = fx.publish("0.0.1", &["--index-checkout", checkout_arg]);
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    let step = &report["index_checkout"];
    assert_eq!(step["path"], checkout_arg);
    assert_eq!(step["added"], true);
    assert_eq!(step["committed"], true);
    assert_eq!(step["pushed"], false);
    assert_eq!(step["entry"], "org.vibevm/tiny@0.0.1");
    assert_eq!(indexed_versions(&checkout), ["0.0.1"]);
    let subject = run_git_output(&checkout, &["log", "-1", "--format=%s"]);
    assert_eq!(
        String::from_utf8_lossy(&subject.stdout).trim(),
        "index: org.vibevm/tiny@0.0.1"
    );
    let repomd: serde_json::Value =
        serde_json::from_slice(&fs::read(checkout.join("repomd.json")).unwrap()).unwrap();
    assert!(
        repomd["files"]["by-name/tiny.json"]["sha256"].is_string(),
        "repomd.json was rewritten and vouches for the new file: {repomd}"
    );
    let remote = fx.root.path().join("index-remote.git");
    assert_ne!(
        head(&checkout, "HEAD"),
        head(&remote, "main"),
        "without --push-index the commit stays local"
    );

    // A dry run names what it would add and touches nothing.
    let before = head(&checkout, "HEAD");
    let dry = fx.publish("0.0.2", &["--index-checkout", checkout_arg, "--dry-run"]);
    assert_eq!(dry["index_checkout"]["would_add"], "org.vibevm/tiny@0.0.2");
    assert_eq!(dry["index_checkout"]["added"], false);
    assert_eq!(head(&checkout, "HEAD"), before);
    assert_eq!(indexed_versions(&checkout), ["0.0.1"]);

    // With --push-index the checkout's commit reaches its upstream.
    let pushed = fx.publish("0.0.2", &["--index-checkout", checkout_arg, "--push-index"]);
    assert_eq!(pushed["index_checkout"]["committed"], true);
    assert_eq!(pushed["index_checkout"]["pushed"], true);
    assert_eq!(indexed_versions(&checkout), ["0.0.1", "0.0.2"]);
    assert_eq!(head(&checkout, "HEAD"), head(&remote, "main"));
}

#[test]
fn publish_reads_the_checkout_from_the_user_registry_file_and_skips_without_one() {
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let fx = Fixture::new();
    let skipped = fx.publish("0.0.1", &[]);
    let step = &skipped["index_checkout"];
    assert_eq!(step["path"], serde_json::Value::Null);
    assert_eq!(step["added"], false);
    assert!(
        step["skipped"]
            .as_str()
            .unwrap()
            .contains("no index checkout is configured"),
        "{step}"
    );

    let checkout = fx.checkout();
    fs::write(
        fx.user.settings.join("registry.toml"),
        format!(
            "[index]\ncheckout = \"{}\"\n",
            checkout.to_string_lossy().replace('\\', "/")
        ),
    )
    .unwrap();
    let report = fx.publish("0.0.2", &[]);
    assert_eq!(report["index_checkout"]["entry"], "org.vibevm/tiny@0.0.2");
    assert_eq!(indexed_versions(&checkout), ["0.0.2"]);
}
