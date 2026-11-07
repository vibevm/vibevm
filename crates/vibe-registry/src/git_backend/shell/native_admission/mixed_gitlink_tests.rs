//! Exact mixed SSH → anonymous HTTPS → distinct SSH child closure.
#![cfg(all(test, unix))]
use super::*;
use super::{ssh_tests::Fixture as SshFixture, tests::Fixture as HttpsFixture};
use crate::{GitBackend, ShellGit};
use std::process::Command;
use vibe_core::manifest::AuthKind;
fn git(cwd: &Path, args: &[&str]) -> String {
    let output = Command::new("/usr/bin/git")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", cwd)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_TERMINAL_PROMPT", "0")
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=f@example.invalid",
            "-c",
            "core.hooksPath=/dev/null",
        ])
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}
fn commit(work: &Path) -> String {
    git(work, &["add", "payload"]);
    if work.join(".gitmodules").is_file() {
        git(work, &["add", ".gitmodules"]);
    }
    git(work, &["commit", "-m", "mixed fixture"]);
    git(work, &["rev-parse", "HEAD"])
}
fn grant(f: &SshFixture, host: &str, port: u16, agent: bool) -> SshProfile {
    SshProfile::explicit_direct(
        "/usr/bin/ssh".into(),
        host.into(),
        f.get("user").into(),
        port,
        std::fs::read(f.get("known_hosts")).unwrap(),
        if agent {
            SshAuthentication::AgentSocket(f.get("agent_socket").into())
        } else {
            SshAuthentication::Key(std::fs::read(f.get("key")).unwrap())
        },
    )
    .unwrap()
}
fn base(http: &HttpsFixture, root: &Path, deny: bool) -> NativeGitAdmission {
    let selection =
        NativeConfigSelection::new(root.into(), Path::new(http.get("root")).join("home"))
            .unwrap()
            .with_variable("GIT_CONFIG_NOSYSTEM", "1".into())
            .unwrap()
            .with_variable("GIT_CONFIG_GLOBAL", "/dev/null".into())
            .unwrap()
            .with_variable("GIT_ATTR_NOSYSTEM", "1".into())
            .unwrap();
    NativeGitAdmission::direct_https(
        Arc::new(crate::HttpAdmission::new(if deny { vec!["127.0.0.2"] } else { vec![] }).unwrap()),
        GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap(),
        TlsTrust::Pem(std::fs::read(http.get("ca")).unwrap()),
    )
    .with_config_selection(selection)
}
fn denied(error: &GitError) -> bool {
    match error {
        GitError::ContactRefused(_) => true,
        GitError::PreparedValidation { source } => denied(source),
        _ => false,
    }
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES-LAW")]
fn mixed_exact_gitlinks_use_distinct_profiles_and_retain_origins_in_fresh_and_warm_stages() {
    let parent = SshFixture::new();
    let leaf = SshFixture::new();
    let http = HttpsFixture::new();
    let temp = tempfile::tempdir().unwrap();
    assert_ne!(parent.port(), leaf.port());
    assert_ne!(
        std::fs::read(parent.get("key")).unwrap(),
        std::fs::read(leaf.get("key")).unwrap()
    );
    let leaf_work = Path::new(leaf.get("root")).join("work");
    git(&leaf_work, &["rm", "vibe.toml"]);
    std::fs::write(leaf_work.join("payload"), "pinned SSH leaf\n").unwrap();
    let leaf_oid = commit(&leaf_work);
    std::fs::write(leaf_work.join("payload"), "later SSH leaf\n").unwrap();
    let later_leaf = commit(&leaf_work);
    assert_ne!(leaf_oid, later_leaf);
    let leaf_bare = Path::new(leaf.get("home")).join("repo.git");
    git(
        &leaf_bare,
        &[
            "fetch",
            leaf_work.to_str().unwrap(),
            "+HEAD:refs/heads/mixed",
        ],
    );
    git(&leaf_bare, &["symbolic-ref", "HEAD", "refs/heads/mixed"]);
    let leaf_url = leaf.get("url").replace("127.0.0.1", "127.0.0.2");
    let child = temp.path().join("child");
    std::fs::create_dir(&child).unwrap();
    git(&child, &["init"]);
    std::fs::write(
        child.join(".gitmodules"),
        format!("[submodule \"leaf\"]\n path=leaf\n url={leaf_url}\n"),
    )
    .unwrap();
    std::fs::write(child.join("payload"), "pinned HTTPS child\n").unwrap();
    git(
        &child,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{leaf_oid},leaf"),
        ],
    );
    let child_oid = commit(&child);
    assert!(
        git(&child, &["ls-tree", "-r", &child_oid]).contains(&format!("160000 commit {leaf_oid}"))
    );
    std::fs::write(child.join("payload"), "later HTTPS child\n").unwrap();
    assert_ne!(commit(&child), child_oid);
    let child_bare = Path::new(http.get("root")).join("web/mixed-child.git");
    git(
        temp.path(),
        &[
            "clone",
            "--bare",
            child.to_str().unwrap(),
            child_bare.to_str().unwrap(),
        ],
    );
    git(&child_bare, &["update-server-info"]);
    let child_url = format!("{}/mixed-child.git", http.get("a"));
    let parent_work = Path::new(parent.get("root")).join("work");
    std::fs::write(
        parent_work.join(".gitmodules"),
        format!("[submodule \"child\"]\n path=child\n url={child_url}\n"),
    )
    .unwrap();
    git(
        &parent_work,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{child_oid},child"),
        ],
    );
    let parent_oid = commit(&parent_work);
    assert!(
        git(&parent_work, &["ls-tree", "-r", &parent_oid])
            .contains(&format!("160000 commit {child_oid}"))
    );
    git(&parent_work, &["tag", "v3.0.0"]);
    let parent_bare = Path::new(parent.get("home")).join("repo.git");
    git(
        &parent_bare,
        &[
            "fetch",
            parent_work.to_str().unwrap(),
            "refs/tags/v3.0.0:refs/tags/v3.0.0",
        ],
    );
    for agent in [false, true] {
        let profiles = vec![
            grant(&parent, "127.0.0.1", parent.port(), agent),
            grant(&leaf, "127.0.0.2", leaf.port(), agent),
        ];
        let backend = ShellGit::new().with_native_admission(Arc::new(
            base(&http, temp.path(), false)
                .with_ssh_profiles(profiles)
                .unwrap(),
        ));
        let before = (parent.contacts("a"), leaf.contacts("b"));
        for auth in [AuthKind::None, AuthKind::TokenEnv] {
            assert!(
                backend
                    .bound_for_registry_auth(auth)
                    .unwrap()
                    .unwrap()
                    .list_tags(parent.get("url"))
                    .is_err()
            );
        }
        assert_eq!((parent.contacts("a"), leaf.contacts("b")), before);
        let backend = backend
            .bound_for_registry_auth(AuthKind::Ssh)
            .unwrap()
            .unwrap();
        let objects = temp.path().join(format!("objects-{agent}.git"));
        let slot = temp.path().join(format!("slot-{agent}"));
        let revision = backend
            .prepare_native_source(parent.get("url"), "v3.0.0", None, &objects)
            .unwrap()
            .unwrap()
            .bind(&slot)
            .unwrap();
        assert_eq!(revision.commit, parent_oid);
        let stage = backend
            .stage_native_materialisation(&objects, &revision)
            .unwrap()
            .unwrap();
        stage.validate_before_publication().unwrap();
        let staged = stage.staged_path();
        for (path, oid, url) in [
            (staged.to_owned(), parent_oid.as_str(), parent.get("url")),
            (staged.join("child"), child_oid.as_str(), child_url.as_str()),
            (
                staged.join("child/leaf"),
                leaf_oid.as_str(),
                leaf_url.as_str(),
            ),
        ] {
            assert_eq!(git(&path, &["rev-parse", "HEAD"]), oid);
            assert_eq!(git(&path, &["config", "--get", "remote.origin.url"]), url);
        }
        assert!(!staged.join("child/vibe.toml").exists());
        assert!(!staged.join("child/leaf/vibe.toml").exists());
        assert_eq!(
            std::fs::read(staged.join("child/payload")).unwrap(),
            b"pinned HTTPS child\n"
        );
        assert_eq!(
            std::fs::read(staged.join("child/leaf/payload")).unwrap(),
            b"pinned SSH leaf\n"
        );
        let status = git(staged, &["submodule", "status", "--recursive"]);
        assert!(status.contains(&child_oid) && status.contains(&leaf_oid));
        assert!(
            status
                .lines()
                .all(|line| !line.starts_with(['-', '+', 'U']))
        );
        assert!(parent.contacts("a") > before.0 && leaf.contacts("b") > before.1);
        assert!(!slot.exists());
        let warm = temp.path().join(format!("warm-{agent}"));
        stage.copy_for_cache(&warm).unwrap();
        let witness = materialisation::tree(&warm).unwrap();
        let warm_objects = temp.path().join(format!("warm-{agent}.git"));
        let revision = backend
            .prepare_ref(
                parent.get("url"),
                "v3.0.0",
                Some(&warm),
                &warm_objects,
                &warm,
            )
            .unwrap();
        let warm_stage = backend
            .stage_native_materialisation(&warm_objects, &revision)
            .unwrap()
            .unwrap();
        warm_stage.validate_before_publication().unwrap();
        assert_eq!(materialisation::tree(&warm).unwrap(), witness);
        assert_eq!(
            git(
                &warm_stage.staged_path().join("child/leaf"),
                &["config", "--get", "remote.origin.url"]
            ),
            leaf_url
        );
    }
    let backend = ShellGit::new()
        .with_native_admission(Arc::new(
            base(&http, temp.path(), true)
                .with_ssh_profiles(vec![
                    grant(&parent, "127.0.0.1", parent.port(), false),
                    grant(&leaf, "127.0.0.2", leaf.port(), false),
                ])
                .unwrap(),
        ))
        .bound_for_registry_auth(AuthKind::Ssh)
        .unwrap()
        .unwrap();
    let before = leaf.contacts("b");
    let slot = temp.path().join("denied-slot");
    let error = backend
        .prepare_ref(
            parent.get("url"),
            "v3.0.0",
            None,
            &temp.path().join("denied.git"),
            &slot,
        )
        .unwrap_err();
    assert!(denied(&error), "{error}");
    assert_eq!(leaf.contacts("b"), before);
    assert!(!slot.exists());
}
#[test]
fn duplicate_and_ambiguous_grants_refuse_and_single_profile_replaces_the_set() {
    let one = SshFixture::new();
    let two = SshFixture::new();
    let http = HttpsFixture::new();
    let temp = tempfile::tempdir().unwrap();
    assert!(
        base(&http, temp.path(), false)
            .with_ssh_profiles(vec![
                grant(&one, "127.0.0.1", one.port(), false),
                grant(&one, "127.0.0.1", one.port(), true)
            ])
            .is_err()
    );
    for reverse in [false, true] {
        let mut profiles = vec![
            grant(&one, "127.0.0.1", one.port(), false),
            grant(&two, "127.0.0.1", two.port(), false),
        ];
        if reverse {
            profiles.reverse();
        }
        let backend = ShellGit::new().with_native_admission(Arc::new(
            base(&http, temp.path(), false)
                .with_ssh_profiles(profiles)
                .unwrap(),
        ));
        let before = (one.contacts("a"), two.contacts("a"));
        assert!(
            matches!(backend.list_tags(one.get("scp")),Err(GitError::NativeContext{reason}) if reason.contains("ambiguous"))
        );
        assert_eq!((one.contacts("a"), two.contacts("a")), before);
    }
    let profile = base(&http, temp.path(), false)
        .with_ssh_profiles(vec![
            grant(&one, "127.0.0.1", one.port(), false),
            grant(&two, "127.0.0.1", two.port(), false),
        ])
        .unwrap()
        .with_ssh_profile(grant(&two, "127.0.0.1", two.port(), false));
    let backend = ShellGit::new().with_native_admission(Arc::new(profile));
    let before = one.contacts("a");
    assert!(backend.list_tags(one.get("url")).is_err());
    assert_eq!(one.contacts("a"), before);
    assert_eq!(
        backend.list_tags(two.get("url")).unwrap(),
        ["v1.0.0", "v1.1.0"]
    );
}
