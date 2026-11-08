//! Actual unbound Git must not substitute a tampered warm origin after token selection.
use super::*;
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};
fn git(root: &Path, home: &Path, args: &[&str]) {
    let output = Command::new("/usr/bin/git")
        .current_dir(root)
        .env("HOME", home)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
}
#[test]
fn retained_registry_acquisition_ignores_changed_legacy_warm_origin() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir(&home).unwrap();
    let registry = temp.path().join("registry");
    fs::create_dir(&registry).unwrap();
    let mut remotes = Vec::new();
    for marker in ["selected", "substitute"] {
        let work = temp.path().join(marker);
        fs::create_dir(&work).unwrap();
        git(&work, &home, &["init", "-b", "main"]);
        fs::write(
            work.join("vibe.toml"),
            "[package]\ngroup='org.test'\nname='item'\nversion='1.0.0'\nkind='tool'\n",
        )
        .unwrap();
        // Identical metadata makes manifest parity alone incapable of detecting substitution.
        fs::write(work.join("payload.txt"), marker).unwrap();
        git(&work, &home, &["add", "."]);
        git(&work, &home, &["commit", "-m", "fixture"]);
        git(&work, &home, &["tag", "v1.0.0"]);
        let bare = if marker == "selected" {
            registry.join("org.test.item.git")
        } else {
            temp.path().join("substitute.git")
        };
        git(
            &work,
            &home,
            &[
                "clone",
                "--bare",
                work.to_str().unwrap(),
                bare.to_str().unwrap(),
            ],
        );
        remotes.push(bare);
    }
    let wrapper = temp.path().join("git-fixture");
    let trace = temp.path().join("trace");
    fs::write(&wrapper,format!("#!/bin/sh\nexport HOME='{}'\nexport GIT_CONFIG_NOSYSTEM=1\nexport GIT_CONFIG_GLOBAL=/dev/null\nexport GIT_TRACE='{}'\nexec /usr/bin/git -c protocol.allow=never -c protocol.file.allow=always \"$@\"\n",home.display(),trace.display())).unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o700)).unwrap();
    let manifest=Manifest::parse_str(&format!("[project]\nname='test'\nversion='1.0.0'\n[[registry]]\nname='chosen'\nurl='git+file://{}'\nindex_url='none'\n",registry.display())).unwrap();
    let owner = crate::MultiRegistryResolver::from_manifest(
        &manifest.registries,
        &[],
        &[],
        temp.path().join("cache"),
        Arc::new(crate::git_backend::ShellGit::new().with_binary(wrapper)),
        0,
    )
    .unwrap();
    let group = Group::parse("org.test").unwrap();
    let version = "1.0.0".parse().unwrap();
    let progress = vibe_core::progress::Progress::default();
    let initial = owner
        .retain_default_candidate(&group, "item", &version)
        .unwrap()
        .candidate;
    owner
        .fetch_candidate_for_install(
            &initial,
            &temp.path().join("store-first"),
            None,
            &temp.path().join("slot-first"),
            &progress,
        )
        .unwrap();
    let clone_dir = &initial.0.resolution.resolved.source_dir;
    if !clone_dir.join(".git").exists() {
        fs::create_dir_all(clone_dir.parent().unwrap()).unwrap();
        git(
            temp.path(),
            &home,
            &[
                "clone",
                remotes[0].to_str().unwrap(),
                clone_dir.to_str().unwrap(),
            ],
        );
    }
    let retained = owner
        .retain_default_candidate(&group, "item", &version)
        .unwrap()
        .candidate;
    let checkout = &retained.0.resolution.resolved.source_dir;
    assert!(
        checkout.join(".git").exists(),
        "fixture must actually exercise warm cache"
    );
    git(
        checkout,
        &home,
        &["remote", "set-url", "origin", remotes[1].to_str().unwrap()],
    );
    fs::write(&trace, "").unwrap();
    let acquired = owner
        .fetch_candidate_for_install(
            &retained,
            &temp.path().join("store-second"),
            None,
            &temp.path().join("slot-second"),
            &progress,
        )
        .unwrap();
    let crate::PackageAcquisition::Cached(cached) = acquired else {
        panic!("copy fixture")
    };
    assert_eq!(
        fs::read_to_string(cached.cache_dir.join("payload.txt")).unwrap(),
        "selected"
    );
    let calls = fs::read_to_string(trace).unwrap();
    assert!(
        !calls.contains(remotes[1].to_str().unwrap()),
        "substitute remote contacted: {calls}"
    );
}
