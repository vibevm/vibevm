//! Two user routes do not become shared project provenance.
#![cfg(unix)]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#provenance");
mod common;
use common::UserScratch;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
use vibe_core::manifest::Lockfile;

fn git(root: &Path, home: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(root)
        .env("HOME", home)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_ALLOW_PROTOCOL", "file")
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=f@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}
fn run(user: &UserScratch, root: &Path, home: &Path, trace: &Path, args: &[&str]) -> Output {
    user.vibe()
        .current_dir(root)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_ALLOW_PROTOCOL", "file")
        .env("GIT_TRACE2_EVENT", trace)
        .args(args)
        .args([
            "--path",
            root.to_str().unwrap(),
            "--assume-yes",
            "--unattended",
            "--json",
        ])
        .output()
        .unwrap()
}
fn ok(out: Output) {
    assert!(
        out.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
fn two_cold_developer_routes_preserve_shared_bytes_then_update_and_uninstall_prune_records() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let project = temp.path().join("project");
    let org = temp.path().join("canonical");
    let repo = org.join("org.example.pkg.git");
    for path in [&home, &project, &repo] {
        fs::create_dir_all(path).unwrap();
    }
    let manifest = "[package]\ngroup='org.example'\nname='pkg'\nkind='feat'\nversion='1.0.0'\n";
    fs::write(repo.join("vibe.toml"), manifest).unwrap();
    fs::write(repo.join("payload"), "identical source bytes\n").unwrap();
    git(&repo, &home, &["init", "--initial-branch=main"]);
    git(&repo, &home, &["add", "."]);
    git(&repo, &home, &["commit", "-m", "source"]);
    git(&repo, &home, &["tag", "v1.0.0"]);
    fs::write(project.join("vibe.toml"),format!("[project]\nname='shared'\nversion='0.1.0'\n[[registry]]\nname='origin'\nurl='git+file://{}'\nindex_url='none'\n[requires.packages]\n'org.example/pkg'='^1'\n",org.display())).unwrap();
    let seed = UserScratch::new();
    ok(run(
        &seed,
        &project,
        &home,
        &temp.path().join("seed.trace"),
        &["install"],
    ));
    let lock_path = project.join("vibe.lock");
    let before = fs::read(&lock_path).unwrap();
    let old = Lockfile::read(&lock_path).unwrap();
    let sidecar = project.join("vibe.lock.provenance");
    let shared = "# project-recorded fact\n[[package]]\npackage='org.example/pkg'\nversion='1.0.0'\nfetched_from='https://project-recorded.invalid/pkg.git'\n";
    fs::write(&sidecar, shared).unwrap();
    for developer in ["one", "two"] {
        let replica = temp.path().join(format!("developer-{developer}.git"));
        git(
            temp.path(),
            &home,
            &[
                "clone",
                "--bare",
                repo.to_str().unwrap(),
                replica.to_str().unwrap(),
            ],
        );
        let user = UserScratch::new();
        fs::write(user.settings.join("settings.toml"),format!("[[remap]]\npackage='org.example/pkg'\nurl='git+file://{}'\nreason='developer local route'\n",replica.display())).unwrap();
        let slot = project
            .join(common::deps_root())
            .join("org.example.pkg/1.0.0");
        if slot.exists() {
            fs::remove_dir_all(slot).unwrap();
        }
        let trace = temp.path().join(format!("{developer}.trace"));
        ok(run(&user, &project, &home, &trace, &["install"]));
        assert!(
            fs::read_to_string(trace)
                .unwrap()
                .contains(&format!("developer-{developer}.git")),
            "cold acquisition must actually use selected operator route"
        );
        assert_eq!(fs::read(&lock_path).unwrap(), before);
        assert_eq!(fs::read_to_string(&sidecar).unwrap(), shared);
        assert_eq!(
            Lockfile::read(&lock_path).unwrap().packages[0].source_url,
            old.packages[0].source_url
        );
    }
    fs::write(repo.join("vibe.toml"), manifest.replace("1.0.0", "1.1.0")).unwrap();
    git(&repo, &home, &["add", "vibe.toml"]);
    git(&repo, &home, &["commit", "-m", "new version"]);
    git(&repo, &home, &["tag", "v1.1.0"]);
    ok(run(
        &seed,
        &project,
        &home,
        &temp.path().join("update.trace"),
        &["update", "org.example/pkg"],
    ));
    assert_eq!(
        Lockfile::read(&lock_path).unwrap().packages[0]
            .version
            .to_string(),
        "1.1.0"
    );
    assert!(
        !sidecar.exists(),
        "scoped update must prune old-version project record"
    );
    fs::write(&sidecar, shared.replace("1.0.0", "1.1.0")).unwrap();
    ok(run(
        &seed,
        &project,
        &home,
        &temp.path().join("uninstall.trace"),
        &["uninstall", "org.example/pkg"],
    ));
    assert!(!sidecar.exists());
}
