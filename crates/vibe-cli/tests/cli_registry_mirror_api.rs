//! `vibe registry mirror` listing a per-package source through its host's
//! organisation API (PROP-002 `#registry-mirror`), against local fixtures
//! only. No network.
//!
//! One loopback server stands in for the whole host: `GET
//! /orgs/<org>/repos` answers the repository listing (paged, optionally
//! token-only, every request head recorded), and every other path serves
//! the organisation's bare repositories over dumb HTTP, so the run needs
//! no redirection — it reads exactly the url the registry declares, as it
//! would in production. `VIBEVM_REGISTRY_API_<REGISTRY>` points the
//! listing at that same server, which is also what re-points an
//! on-premises host.
#![cfg(unix)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use common::UserScratch;
use common::org_host::{Host, Org, git};

/// The read token the fixtures configure. Never expected in any output.
const TOKEN: &str = "canary-read-token-6b1f";
/// A second token the higher-precedence leg must beat. Also never echoed.
const DECOY: &str = "decoy-read-token-a703";
/// One whole fixture: the organisation and its host, the project naming
/// them, and the target to mirror into.
struct Run {
    user: UserScratch,
    org: Org,
    project: PathBuf,
    into: PathBuf,
}

impl Run {
    /// `kind` is the registry's `type`; `rows` is appended to the
    /// `[[registry]]` row verbatim, so a test can declare the registry's
    /// own `token-env` credential or its `naming` convention.
    fn new(root: &Path, org: Org, host: &Host, kind: &str, rows: &str) -> Self {
        let project = root.join("project");
        fs::create_dir_all(&project).unwrap();
        fs::write(
            project.join("vibe.toml"),
            format!(
                "[project]\nname='consumer'\nversion='0.1.0'\n\
                 [[registry]]\nname='src'\ntype='{kind}'\nurl='{}'\nindex_url='none'\n{rows}",
                host.registry_url()
            ),
        )
        .unwrap();
        let into = root.join("target.git");
        git(
            root,
            &["init", "-q", "--bare", "-b", "main", into.to_str().unwrap()],
        );
        Run {
            user: UserScratch::new(),
            org,
            project,
            into,
        }
    }

    /// `vibe registry mirror src --into <target>`, with the fixture host
    /// as the registry's API base and `env` on top.
    fn mirror(&self, host: &Host, env: &[(&str, &str)], extra: &[&str]) -> Output {
        let into = self.into.to_str().unwrap();
        let mut args = vec!["--json", "registry", "mirror", "src", "--into", into];
        args.extend_from_slice(extra);
        let mut command = self.user.vibe();
        command
            .current_dir(&self.project)
            .env("VIBE_NO_DEFAULT_REGISTRY", "1")
            .env("VIBEVM_REGISTRY_API_SRC", &host.url)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_AUTHOR_NAME", "Fixture")
            .env("GIT_AUTHOR_EMAIL", "f@example.invalid")
            .env("GIT_COMMITTER_NAME", "Fixture")
            .env("GIT_COMMITTER_EMAIL", "f@example.invalid");
        for (name, value) in env {
            command.env(name, value);
        }
        command.args(args).output().unwrap()
    }

    /// `git --git-dir <target> <args>`.
    fn target_git(&self, args: &[&str]) -> String {
        let mut all = vec!["--git-dir", self.into.to_str().unwrap()];
        all.extend_from_slice(args);
        git(&self.org.dir, &all)
    }
}

fn json(output: &Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let _: vibe_wire::generated::registry_mirror_report::RegistryMirrorReport =
        serde_json::from_value(report.clone()).unwrap();
    report
}

fn rows(report: &serde_json::Value) -> Vec<(String, String, String)> {
    report["versions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|version| {
            (
                version["package"].as_str().unwrap().to_string(),
                version["version"].as_str().unwrap().to_string(),
                version["action"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

fn row(package: &str, version: &str, action: &str) -> (String, String, String) {
    (package.to_string(), version.to_string(), action.to_string())
}

/// Neither the real token nor the decoy may reach either stream.
fn assert_tokens_not_echoed(output: &Output) {
    for stream in [&output.stdout, &output.stderr] {
        let text = String::from_utf8_lossy(stream);
        for secret in [TOKEN, DECOY] {
            assert!(!text.contains(secret), "a token value leaked: {text}");
        }
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror"
)]
fn a_gitverse_organisation_listing_replaces_the_package_flag() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let org = Org::new(root);
    org.repository(
        "org.corp.demo",
        &["1.0.0", "1.1.0"],
        Some(("org.corp", "demo")),
    );
    org.repository("org.corp.tool", &["2.0.0"], Some(("org.corp", "tool")));
    // Named like a package, tagged like a package, no manifest: not one.
    org.repository("org.corp.hollow", &["1.0.0"], None);
    // Named like a package, never tagged: no version to copy.
    org.repository("org.corp.fresh", &[], Some(("org.corp", "fresh")));

    // Page 1 is full, so the listing must ask for page 2. The filler
    // names carry no group, so the naming convention drops them.
    let mut first: Vec<String> = (0..96).map(|n| format!("tool-{n:02}")).collect();
    first.push("org.corp.demo".to_string());
    first.push("org.corp.hollow".to_string());
    first.push("org.corp.fresh".to_string());
    first.push("docs".to_string());
    assert_eq!(first.len(), 100);
    let host = Host::new(
        &org,
        vec![first, vec!["org.corp.tool".to_string()]],
        Some(TOKEN),
    );
    // The registry declares where its own read token lives; the
    // host-agnostic publish variable is a decoy the first leg outranks.
    let run = Run::new(
        root,
        org,
        &host,
        "gitverse",
        "auth='token-env'\ntoken_env='VIBEVM_TEST_READ_TOKEN'\n",
    );

    let output = run.mirror(
        &host,
        &[
            ("VIBEVM_TEST_READ_TOKEN", TOKEN),
            ("VIBEVM_PUBLISH_TOKEN", DECOY),
        ],
        &[],
    );
    assert_tokens_not_echoed(&output);
    let report = json(&output);

    assert_eq!(report["listing"], "api");
    assert_eq!(
        rows(&report),
        vec![
            row("org.corp/demo", "1.0.0", "copied"),
            row("org.corp/demo", "1.1.0", "copied"),
            row("org.corp/tool", "2.0.0", "copied"),
        ],
        "{report}"
    );
    assert_eq!(
        run.target_git(&["show", "main:org.corp/tool/v2.0.0/payload"]),
        "org.corp.tool 2.0.0"
    );

    // The registry's own token reached the API as a bearer — the decoy
    // never did — and both pages were read.
    let listings = host.listings();
    assert!(
        listings.len() >= 2,
        "both pages are asked for: {listings:?}"
    );
    assert!(
        listings
            .iter()
            .all(|head| head.contains(&format!("authorization: bearer {TOKEN}"))),
        "{listings:?}"
    );
    assert!(
        !listings.iter().any(|head| head.contains(DECOY)),
        "the registry's own declaration outranks the publish chain: {listings:?}"
    );
    assert!(
        listings[0].contains("accept: application/vnd.gitverse.object+json;version=1"),
        "{}",
        listings[0]
    );
    assert!(listings[1].contains("page=2"), "{}", listings[1]);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror"
)]
fn an_api_that_refuses_an_anonymous_listing_still_requires_the_package_flag() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let org = Org::new(root);
    org.repository("org.corp.demo", &["1.0.0"], Some(("org.corp", "demo")));
    let host = Host::new(&org, vec![vec!["org.corp.demo".to_string()]], Some(TOKEN));
    let run = Run::new(root, org, &host, "gitverse", "");

    // No token configured anywhere: the listing is anonymous and refused.
    let refused = run.mirror(&host, &[], &[]);
    assert!(!refused.status.success());
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(stderr.contains("401"), "{stderr}");
    assert!(
        stderr.contains("no read token was configured"),
        "the refusal says a token was not sent: {stderr}"
    );
    assert!(
        stderr.contains("--package"),
        "the refusal names the fix: {stderr}"
    );
    assert!(run.target_git(&["for-each-ref"]).is_empty());

    // `--package` still works against the very same registry.
    let report = json(&run.mirror(&host, &[], &["--package", "org.corp/demo"]));
    assert_eq!(report["listing"], "tags");
    assert_eq!(rows(&report), vec![row("org.corp/demo", "1.0.0", "copied")]);

    // The second leg of the chain — the per-host publish token, here in
    // its host-agnostic form — opens the same listing.
    let listed = run.mirror(&host, &[("VIBEVM_PUBLISH_TOKEN", TOKEN)], &[]);
    assert_tokens_not_echoed(&listed);
    let report = json(&listed);
    assert_eq!(report["listing"], "api");
    assert_eq!(
        rows(&report),
        vec![row("org.corp/demo", "1.0.0", "unchanged")],
        "{report}"
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror"
)]
fn a_github_organisation_is_listed_through_the_github_api_shape() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let org = Org::new(root);
    org.repository("org.corp.demo", &["1.0.0"], Some(("org.corp", "demo")));
    // GitHub's org listing is readable without a token.
    let host = Host::new(
        &org,
        vec![vec!["org.corp.demo".to_string(), "index".to_string()]],
        None,
    );
    let run = Run::new(root, org, &host, "github", "");

    let report = json(&run.mirror(&host, &[], &[]));
    assert_eq!(report["listing"], "api");
    assert_eq!(rows(&report), vec![row("org.corp/demo", "1.0.0", "copied")]);
    let listings = host.listings();
    assert_eq!(
        listings.len(),
        1,
        "one short page ends the listing: {listings:?}"
    );
    assert!(
        listings[0].contains("accept: application/vnd.github+json"),
        "{}",
        listings[0]
    );
    assert!(
        !listings[0].contains("authorization"),
        "no token is configured, so none is sent: {}",
        listings[0]
    );

    // A `--group` filter narrows the same listing.
    let narrowed = json(&run.mirror(&host, &[], &["--group", "org.other", "--dry-run"]));
    assert!(
        rows(&narrowed).is_empty(),
        "no package of `org.other` is listed: {narrowed}"
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror"
)]
fn a_name_convention_registry_takes_each_coordinate_from_the_manifest() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let org = Org::new(root);
    // The built-in `vibespecs-gitverse` shape: `naming = "name"`, so the
    // repository name carries no group and only the manifest can say
    // which package a repository holds.
    org.repository(
        "wal",
        &["1.0.0", "1.1.0"],
        Some(("org.vibevm.world", "wal")),
    );
    // Tagged and manifested, but the manifest names a package this
    // registry would publish as `other`, not `zap`: not this repository's.
    org.repository("zap", &["1.0.0"], Some(("org.vibevm.world", "other")));
    // Tagged, no manifest at all.
    org.repository("docs", &["1.0.0"], None);
    let host = Host::new(
        &org,
        vec![vec![
            "wal".to_string(),
            "zap".to_string(),
            "docs".to_string(),
        ]],
        None,
    );
    let run = Run::new(root, org, &host, "gitverse", "naming='name'\n");

    let report = json(&run.mirror(&host, &[], &[]));
    assert_eq!(report["listing"], "api");
    assert_eq!(
        rows(&report),
        vec![
            row("org.vibevm.world/wal", "1.0.0", "copied"),
            row("org.vibevm.world/wal", "1.1.0", "copied"),
        ],
        "{report}"
    );
    assert_eq!(
        run.target_git(&["show", "main:org.vibevm.world/wal/v1.1.0/payload"]),
        "wal 1.1.0"
    );
}
