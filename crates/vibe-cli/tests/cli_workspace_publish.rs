//! End-to-end tests for the publish surfaces: `vibe registry publish` against
//! a GitVerse-shaped registry (stub envelope), the direct `--repo-url` git
//! push, and the M1.17 `vibe workspace publish` walk (PROP-007 §2.7–§2.9).

mod common;

use std::fs;

use common::{git_available, init_project, vibe, write_member_pkg, write_workspace_root};
use specmark::verifies;

// ---------------------------------------------------------------------------
// `vibe registry publish` against GitVerse — stub, not API call
// ---------------------------------------------------------------------------
//
// GitVerse's public REST API does not expose org-scoped repository creation
// today, so `vibe registry publish` cannot run end to end against a GitVerse
// `[[registry]]`. The CLI short-circuits at the host-detection step with a
// clear "not implemented" envelope. This test pins that contract: targeting
// the GitVerse-shaped default registry produces the stub envelope, exits 0
// (so the operator can script around it without `||` machinery), and never
// reaches token loading or any HTTP call.

#[test]
fn publish_against_gitverse_registry_emits_stub_envelope() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    // Add the gitverse registry (defaults no longer land in the project).
    vibe()
        .arg("registry")
        .arg("add")
        .arg("vibespecs-gitverse")
        .arg("https://gitverse.ru/vibespecs")
        .arg("--naming")
        .arg("name")
        .arg("--path")
        .arg(project.path())
        .assert()
        .success();

    // Synthesize a minimal package directory the publisher can read.
    // The stub fires before any of these bytes matter — the test
    // would still pass with an empty file — but the manifest is what
    // the non-stub path would consume, so writing one keeps the test
    // honest about exercising the real argument flow.
    let pkg_dir = tempfile::tempdir().unwrap();
    fs::write(
        pkg_dir.path().join("vibe.toml"),
        r#"[package]
group = "org.vibevm"
name = "tiny"
kind = "flow"
version = "0.0.1"
"#,
    )
    .unwrap();

    let out = vibe()
        .arg("--json")
        .arg("registry")
        .arg("publish")
        .arg(pkg_dir.path())
        .arg("--registry")
        .arg("vibespecs-gitverse")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stub path returns success status; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let payload: serde_json::Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
    assert_eq!(payload["ok"], false);
    assert_eq!(payload["command"], "registry:publish");
    assert_eq!(payload["stub"], true);
    assert_eq!(payload["host"], "gitverse.ru");
    assert_eq!(payload["registry"], "vibespecs-gitverse");
    let reason = payload["reason"].as_str().expect("reason is a string");
    assert!(
        reason.contains("not implemented"),
        "reason should call out the limitation; got: {reason}"
    );
}

// ---------------------------------------------------------------------------
// `vibe registry publish --repo-url` — direct git push, no API in play
// ---------------------------------------------------------------------------
//
// The no-API path takes a single git URL and pushes the package contents
// + tag to it using whatever credentials the local `git` is wired to use.
// We exercise this end to end against a temporary `--bare` repo on the
// local filesystem: a `file:///` URL is enough to drive the same code
// path that an SSH or HTTPS URL would on a real host, without any
// network or token machinery in scope. The token-loading code is
// asserted-not-invoked by setting an obviously-bogus value for
// `VIBEVM_PUBLISH_TOKEN`/`VIBEVM_PUBLISH_TOKEN_GITHUB`: if the direct
// path were silently falling back into the registry path, the publish
// would fail with an auth error against api.github.com.

#[test]
#[verifies("spec://vibevm/modules/vibe-workspace/PROP-007#published-repos", r = 1)]
fn publish_direct_repo_url_pushes_to_local_bare_repo() {
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    // Synthesize a minimal package directory.
    let pkg_dir = tempfile::tempdir().unwrap();
    fs::write(
        pkg_dir.path().join("vibe.toml"),
        r#"[package]
group = "org.vibevm"
name = "tiny"
kind = "flow"
version = "0.0.1"
"#,
    )
    .unwrap();
    fs::create_dir_all(pkg_dir.path().join("spec")).unwrap();
    fs::write(pkg_dir.path().join("spec/PROTOCOL.md"), "hello\n").unwrap();

    // Build a bare origin we can push to. `file:///<abs>` is a real
    // git URL — the same pipeline that would handle SSH/HTTPS handles
    // file:// equivalently, so this exercises the direct-push code
    // without leaking credentials, network, or hostnames into the
    // test environment.
    let bare_dir = tempfile::tempdir().unwrap();
    let bare = bare_dir.path().join("origin.git");
    let init_status = std::process::Command::new("git")
        .args(["init", "--bare", bare.to_str().unwrap()])
        .env("LC_ALL", "C")
        .status()
        .unwrap();
    assert!(init_status.success(), "git init --bare must succeed");

    let abs_bare = bare.to_string_lossy().replace('\\', "/");
    let repo_url = format!("file:///{}", abs_bare.trim_start_matches('/'));

    let out = vibe()
        // Set bogus tokens to assert the direct path does NOT call
        // load_token_for_host. If it did, the github path would barf
        // on a 401 or 403 against api.github.com.
        .env("VIBEVM_PUBLISH_TOKEN", "should-not-be-read-on-direct-path")
        .env(
            "VIBEVM_PUBLISH_TOKEN_GITHUB",
            "should-not-be-read-on-direct-path",
        )
        .arg("--json")
        .arg("registry")
        .arg("publish")
        .arg(pkg_dir.path())
        .arg("--repo-url")
        .arg(&repo_url)
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "direct publish should succeed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let payload: serde_json::Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["command"], "registry:publish");
    assert_eq!(payload["mode"], "direct-git");
    assert_eq!(payload["repo_url"], repo_url);
    assert_eq!(payload["tag"], "v0.0.1");
    assert_eq!(payload["dry_run"], false);

    // The bare repo must now carry the tag and the main branch.
    let tags = std::process::Command::new("git")
        .args(["-C", bare.to_str().unwrap(), "tag", "--list"])
        .env("LC_ALL", "C")
        .output()
        .unwrap();
    let tag_list = String::from_utf8_lossy(&tags.stdout);
    assert!(
        tag_list.contains("v0.0.1"),
        "expected v0.0.1 in tags after direct push, got: {tag_list}"
    );

    let branches = std::process::Command::new("git")
        .args(["-C", bare.to_str().unwrap(), "branch", "--list"])
        .env("LC_ALL", "C")
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&branches.stdout).contains("main"),
        "expected main branch in bare origin after direct push"
    );
}

#[test]
fn publish_direct_repo_url_dry_run_skips_actual_push() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    let pkg_dir = tempfile::tempdir().unwrap();
    fs::write(
        pkg_dir.path().join("vibe.toml"),
        r#"[package]
group = "org.vibevm"
name = "tiny"
kind = "flow"
version = "0.0.1"
"#,
    )
    .unwrap();

    // `--dry-run` against a deliberately invalid URL must NOT fail —
    // dry-run on the direct path skips the git push entirely.
    let out = vibe()
        .arg("--json")
        .arg("registry")
        .arg("publish")
        .arg(pkg_dir.path())
        .arg("--repo-url")
        .arg("ssh://git@invalid.example/never/created.git")
        .arg("--dry-run")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "dry-run direct publish should succeed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let payload: serde_json::Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["mode"], "direct-git");
    assert_eq!(payload["dry_run"], true);
    assert_eq!(
        payload["repo_url"],
        "ssh://git@invalid.example/never/created.git"
    );
}

#[test]
fn publish_repo_url_and_registry_are_mutually_exclusive() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    let pkg_dir = tempfile::tempdir().unwrap();
    fs::write(
        pkg_dir.path().join("vibe.toml"),
        r#"[package]
group = "org.vibevm"
name = "tiny"
kind = "flow"
version = "0.0.1"
"#,
    )
    .unwrap();

    // clap surfaces `conflicts_with` violations as a usage error
    // (exit code 2). Pin that — passing both `--registry` and
    // `--repo-url` should never proceed past arg parsing.
    vibe()
        .arg("registry")
        .arg("publish")
        .arg(pkg_dir.path())
        .arg("--registry")
        .arg("vibespecs")
        .arg("--repo-url")
        .arg("ssh://git@example.org/foo.git")
        .arg("--path")
        .arg(project.path())
        .assert()
        .failure();
}

// ===================================================================
// M1.17 — vibe workspace publish (PROP-007 §2.7–§2.9)
// ===================================================================
//
// These exercise the full CLI walk of `vibe workspace publish` through
// its `--dry-run` path. Dry-run is the hermetic path: it does all the
// discovery / selection / topological ordering / staging and reports
// the plan, but creates no repository and pushes nothing — so no
// network and no publish token are involved. The real network walk
// (repo creation + push + tag against GitHub) is covered by the manual
// recipe `manual-tests/M1.17-workspace-publish-smoke.md`. The
// selection / ordering / staging logic itself is unit-tested in
// `vibe-workspace` and `vibe-cli`'s `commands::workspace` bin tests.

#[test]
fn workspace_publish_dry_run_reports_plan() {
    let project = tempfile::tempdir().unwrap();
    write_workspace_root(project.path(), &["packages/a", "packages/b"]);
    write_member_pkg(project.path(), "packages/a", "a", "flow", "");
    write_member_pkg(project.path(), "packages/b", "b", "feat", "");

    let out = vibe()
        .args(["workspace", "publish", "--dry-run", "--path"])
        .arg(project.path())
        .output()
        .expect("spawn vibe workspace publish");
    assert!(
        out.status.success(),
        "dry-run publish should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Both members appear in the would-publish output.
    assert!(
        stdout.contains("org.vibevm/a"),
        "stdout missing org.vibevm/a:\n{stdout}"
    );
    assert!(
        stdout.contains("org.vibevm/b"),
        "stdout missing org.vibevm/b:\n{stdout}"
    );
    assert!(
        stdout.contains("dry-run"),
        "dry-run summary missing:\n{stdout}"
    );
}

#[test]
#[verifies(
    "spec://vibevm/modules/vibe-workspace/PROP-007#selective-publish",
    r = 1
)]
fn workspace_publish_dry_run_json_envelope() {
    let project = tempfile::tempdir().unwrap();
    write_workspace_root(project.path(), &["packages/a", "packages/b"]);
    write_member_pkg(project.path(), "packages/a", "a", "flow", "");
    // b is publish = false — must land in `skipped`, not `published`.
    write_member_pkg(project.path(), "packages/b", "b", "feat", "false");

    let out = vibe()
        .args(["workspace", "publish", "--dry-run", "--json", "--path"])
        .arg(project.path())
        .output()
        .expect("spawn vibe workspace publish --json");
    assert!(out.status.success());
    let payload: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("valid JSON envelope");
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["command"], "workspace:publish");
    assert_eq!(payload["dry_run"], true);
    let published = payload["published"].as_array().expect("published array");
    assert_eq!(published.len(), 1, "only org.vibevm/a publishes");
    assert_eq!(published[0]["pkgref"], "org.vibevm/a");
    assert_eq!(published[0]["rel_path"], "packages/a");
    assert_eq!(published[0]["tag"], "v0.1.0");
    let skipped = payload["skipped"].as_array().expect("skipped array");
    assert_eq!(skipped.len(), 1, "org.vibevm/b is skipped");
    assert_eq!(skipped[0]["rel_path"], "packages/b");
    assert!(
        skipped[0]["reason"]
            .as_str()
            .unwrap()
            .contains("publish = false")
    );
    // No failure — remaining is empty.
    assert!(payload["remaining"].as_array().unwrap().is_empty());
}

#[test]
#[verifies(
    "spec://vibevm/modules/vibe-workspace/PROP-007#selective-publish",
    r = 1
)]
fn workspace_publish_dry_run_topological_order() {
    // b depends on a via a path-dep — a must publish first.
    let project = tempfile::tempdir().unwrap();
    write_workspace_root(project.path(), &["packages/a", "packages/b"]);
    write_member_pkg(project.path(), "packages/a", "a", "flow", "");
    // b's manifest carries a path-dep on ../a.
    fs::create_dir_all(project.path().join("packages/b")).unwrap();
    fs::write(
        project.path().join("packages/b/vibe.toml"),
        "[package]\ngroup = \"org.vibevm\"\nname = \"b\"\nkind = \"feat\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/a\" = { path = \"../a\", version = \"^0.1\" }\n",
    )
    .unwrap();

    let out = vibe()
        .args(["workspace", "publish", "--dry-run", "--json", "--path"])
        .arg(project.path())
        .output()
        .expect("spawn vibe workspace publish");
    assert!(out.status.success());
    let payload: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let published = payload["published"].as_array().unwrap();
    assert_eq!(published.len(), 2);
    // Dependency-first: a before b.
    assert_eq!(published[0]["pkgref"], "org.vibevm/a");
    assert_eq!(published[1]["pkgref"], "org.vibevm/b");
}

#[test]
#[verifies(
    "spec://vibevm/modules/vibe-workspace/PROP-007#selective-publish",
    r = 1
)]
fn workspace_publish_member_filter_narrows() {
    let project = tempfile::tempdir().unwrap();
    write_workspace_root(project.path(), &["packages/a", "packages/b"]);
    write_member_pkg(project.path(), "packages/a", "a", "flow", "");
    write_member_pkg(project.path(), "packages/b", "b", "feat", "");

    let out = vibe()
        .args([
            "workspace",
            "publish",
            "--dry-run",
            "--json",
            "--member",
            "packages/b",
            "--path",
        ])
        .arg(project.path())
        .output()
        .expect("spawn vibe workspace publish --member");
    assert!(out.status.success());
    let payload: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let published = payload["published"].as_array().unwrap();
    assert_eq!(published.len(), 1, "only the named member publishes");
    assert_eq!(published[0]["pkgref"], "org.vibevm/b");
}

#[test]
fn workspace_publish_member_filter_rejects_unknown_node() {
    let project = tempfile::tempdir().unwrap();
    write_workspace_root(project.path(), &["packages/a"]);
    write_member_pkg(project.path(), "packages/a", "a", "flow", "");

    let out = vibe()
        .args([
            "workspace",
            "publish",
            "--dry-run",
            "--member",
            "packages/ghost",
            "--path",
        ])
        .arg(project.path())
        .output()
        .expect("spawn vibe workspace publish --member ghost");
    assert!(
        !out.status.success(),
        "an unknown --member must fail loudly"
    );
}

#[test]
fn workspace_publish_detects_dependency_cycle() {
    // a depends on b, b depends on a — a hard error, no publish.
    let project = tempfile::tempdir().unwrap();
    write_workspace_root(project.path(), &["packages/a", "packages/b"]);
    fs::create_dir_all(project.path().join("packages/a")).unwrap();
    fs::write(
        project.path().join("packages/a/vibe.toml"),
        "[package]\ngroup = \"org.vibevm\"\nname = \"a\"\nkind = \"flow\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/b\" = { path = \"../b\", version = \"^0.1\" }\n",
    )
    .unwrap();
    fs::create_dir_all(project.path().join("packages/b")).unwrap();
    fs::write(
        project.path().join("packages/b/vibe.toml"),
        "[package]\ngroup = \"org.vibevm\"\nname = \"b\"\nkind = \"feat\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/a\" = { path = \"../a\", version = \"^0.1\" }\n",
    )
    .unwrap();

    let out = vibe()
        .args(["workspace", "publish", "--dry-run", "--path"])
        .arg(project.path())
        .output()
        .expect("spawn vibe workspace publish");
    assert!(
        !out.status.success(),
        "a dependency cycle must be a hard error"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.to_lowercase().contains("cycle"),
        "error should mention the cycle:\n{stderr}"
    );
}

#[test]
fn workspace_publish_errors_without_registry() {
    // A workspace root with no `[[registry]]` cannot publish anywhere.
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("vibe.toml"),
        "[project]\nname = \"mono\"\nversion = \"0.0.1\"\n\n\
         [workspace]\nmembers = [\"packages/a\"]\n",
    )
    .unwrap();
    write_member_pkg(project.path(), "packages/a", "a", "flow", "");

    let out = vibe()
        .args(["workspace", "publish", "--dry-run", "--path"])
        .arg(project.path())
        .output()
        .expect("spawn vibe workspace publish");
    assert!(!out.status.success(), "no `[[registry]]` must be an error");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("registry"),
        "error should mention the missing registry:\n{stderr}"
    );
}

#[test]
#[verifies(
    "spec://vibevm/modules/vibe-workspace/PROP-007#selective-publish",
    r = 1
)]
fn workspace_publish_all_internal_is_a_clean_noop() {
    // Every member publish = false — entirely-local workspace, a
    // first-class PROP-007 §2.7 extreme. Succeeds, publishes nothing.
    let project = tempfile::tempdir().unwrap();
    write_workspace_root(project.path(), &["packages/a", "packages/b"]);
    write_member_pkg(project.path(), "packages/a", "a", "flow", "false");
    write_member_pkg(project.path(), "packages/b", "b", "feat", "false");

    let out = vibe()
        .args(["workspace", "publish", "--dry-run", "--json", "--path"])
        .arg(project.path())
        .output()
        .expect("spawn vibe workspace publish");
    assert!(
        out.status.success(),
        "all-internal workspace must succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let payload: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(payload["ok"], true);
    assert!(payload["published"].as_array().unwrap().is_empty());
    assert_eq!(payload["skipped"].as_array().unwrap().len(), 2);
}

#[test]
fn workspace_publish_root_package_is_included() {
    // cargo-style: the root carries [package] + [workspace]. PROP-007
    // §2.9 — the root is itself a publishable node.
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("vibe.toml"),
        "[package]\ngroup = \"org.vibevm\"\nname = \"umbrella\"\nkind = \"stack\"\nversion = \"0.1.0\"\n\n\
         [workspace]\nmembers = [\"packages/a\"]\n\n\
         [[registry]]\nname = \"vibespecs\"\nurl = \"https://github.com/vibespecs\"\n",
    )
    .unwrap();
    write_member_pkg(project.path(), "packages/a", "a", "flow", "");

    let out = vibe()
        .args(["workspace", "publish", "--dry-run", "--json", "--path"])
        .arg(project.path())
        .output()
        .expect("spawn vibe workspace publish");
    assert!(out.status.success());
    let payload: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let published = payload["published"].as_array().unwrap();
    assert_eq!(published.len(), 2, "root + member both publish");
    let pkgrefs: Vec<&str> = published
        .iter()
        .map(|p| p["pkgref"].as_str().unwrap())
        .collect();
    assert!(pkgrefs.contains(&"org.vibevm/umbrella"));
    assert!(pkgrefs.contains(&"org.vibevm/a"));
    // The root node stages with rel_path ".".
    let root_entry = published
        .iter()
        .find(|p| p["pkgref"] == "org.vibevm/umbrella")
        .unwrap();
    assert_eq!(root_entry["rel_path"], ".");
}

#[test]
fn workspace_publish_standalone_package_publishes_just_itself() {
    // A standalone node — `[package]`, no `[workspace]`. `discover`
    // degenerates to "just this node"; publishing it is the root only.
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("vibe.toml"),
        "[package]\ngroup = \"org.vibevm\"\nname = \"solo\"\nkind = \"flow\"\nversion = \"0.2.0\"\n\n\
         [[registry]]\nname = \"vibespecs\"\nurl = \"https://github.com/vibespecs\"\n",
    )
    .unwrap();

    let out = vibe()
        .args(["workspace", "publish", "--dry-run", "--json", "--path"])
        .arg(project.path())
        .output()
        .expect("spawn vibe workspace publish");
    assert!(out.status.success());
    let payload: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let published = payload["published"].as_array().unwrap();
    assert_eq!(published.len(), 1);
    assert_eq!(published[0]["pkgref"], "org.vibevm/solo");
    assert_eq!(published[0]["tag"], "v0.2.0");
    assert_eq!(published[0]["rel_path"], ".");
}
