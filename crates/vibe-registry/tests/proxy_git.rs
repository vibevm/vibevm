//! Child `git` through an explicit `[proxy]` with `[proxy] ca_file`
//! (`PROP-068#proxy`): both the incumbent shell backend and the controlled
//! native transport clone / list over HTTPS through the proxy, trusting
//! the private CA only because the route names it; the user's git
//! configuration is not touched. One test: it installs the process's
//! network session.
#![cfg(unix)]

#[path = "../../vibe-net/tests/support/mod.rs"]
mod support;

use std::{path::Path, process::Command, sync::Arc};
use support::{FIXTURE_HOST, https_server, proxy, requests, test_ca};
use vibe_core::{cancellation::CancellationToken, net::ProxyRoute, user_config::ProxyConfig};
use vibe_registry::git_backend::GitBackend;
use vibe_registry::git_backend::shell::native_admission::{
    GitInstallation, NativeGitAdmission, TlsTrust,
};
use vibe_registry::{HttpAdmission, ShellGit};

fn git(args: &[&str], cwd: &Path) {
    let out = Command::new("git")
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=f@example.invalid",
        ])
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn git_clones_over_https_through_the_proxy_with_the_ca_file() {
    let dir = tempfile::tempdir().unwrap();
    let ca = test_ca(dir.path(), "corp");
    let work = dir.path().join("work");
    std::fs::create_dir(&work).unwrap();
    git(&["init", "--initial-branch=main", "."], &work);
    std::fs::write(work.join("vibe.toml"), "[package]\nname = 'demo'\n").unwrap();
    git(&["add", "."], &work);
    git(&["commit", "-m", "fixture"], &work);
    git(&["tag", "v1.0.0"], &work);
    let root = dir.path().join("www");
    std::fs::create_dir(&root).unwrap();
    git(
        &[
            "clone",
            "--bare",
            work.to_str().unwrap(),
            root.join("repo.git").to_str().unwrap(),
        ],
        dir.path(),
    );
    let server = https_server(&root, Some(&ca), dir.path());
    let log = dir.path().join("proxy.log");
    let proxy = proxy(&log);
    let cfg: ProxyConfig = toml::from_str(&format!(
        "https = 'http://127.0.0.1:{}'\nca_file = '{}'",
        proxy.port,
        ca.ca_pem.display()
    ))
    .unwrap();
    let token = CancellationToken::new();
    let _network = vibe_net::enter(&token, ProxyRoute::from_config(&cfg).unwrap());
    let url = format!("https://{FIXTURE_HOST}:{}/git/repo.git", server.port);

    // Incumbent shell backend.
    let dest = dir.path().join("clone");
    ShellGit::new().bootstrap(&url, "v1.0.0", &dest).unwrap();
    assert!(dest.join("vibe.toml").is_file());
    let seen = requests(&log);
    assert!(
        !seen.is_empty()
            && seen
                .iter()
                .all(|line| line == &format!("CONNECT {FIXTURE_HOST}:{} HTTP/1.1", server.port)),
        "{seen:?}"
    );

    // Controlled native transport under source policy.
    let before = requests(&log).len();
    let profile = NativeGitAdmission::direct_https(
        Arc::new(HttpAdmission::new(["github.com"]).unwrap()),
        GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap(),
        TlsTrust::System,
    );
    let native = ShellGit::new().with_native_admission(Arc::new(profile));
    let tags = native.list_tags(&url).unwrap();
    assert!(tags.iter().any(|tag| tag == "v1.0.0"), "{tags:?}");
    assert!(
        requests(&log).len() > before,
        "native git did not use the proxy"
    );
}
