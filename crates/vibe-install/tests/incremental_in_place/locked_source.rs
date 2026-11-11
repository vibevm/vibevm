//! Pinned acquisition preserves the independently captured warm destination.
#![cfg(all(test, unix))]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE");
use super::*;

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
fn locked_native_update_retains_refs_config_and_never_fetches_live_old_origin() {
    let (_server, info, f) = fixture(false);
    vibe_install::apply(&f.source, *f.ready(), SlotIntegrity::Verify, &policy()).unwrap();
    let lock = Lockfile::read(f.project.join("vibe.lock")).unwrap();
    let previous = git(&f.slot, &["rev-parse", "HEAD"]);
    git(
        &f.slot,
        &["update-ref", "refs/tags/local-sentinel", &previous],
    );
    git(
        &f.slot,
        &["config", "user.fixture-note", "preserve this value"],
    );
    write(&f.slot, ".git/sentinel", "preserve raw metadata");
    git(
        &f.slot,
        &[
            "remote",
            "set-url",
            "origin",
            &format!("{}/old-mirror.git", info["b"].as_str().unwrap()),
        ],
    );
    let work = Path::new(info["work"].as_str().unwrap());
    let manifest = fs::read_to_string(work.join("vibe.toml"))
        .unwrap()
        .replace("version='1.0.0'", "version='1.1.0'");
    fs::write(work.join("vibe.toml"), manifest).unwrap();
    git(work, &["add", "vibe.toml"]);
    git(work, &["commit", "-m", "new held endpoint version"]);
    git(work, &["tag", "v1.1.0"]);
    let remote = Path::new(info["root"].as_str().unwrap()).join("web/repo.git");
    git(
        work,
        &[
            "push",
            remote.to_str().unwrap(),
            "HEAD:refs/heads/main",
            "refs/tags/v1.1.0",
        ],
    );
    git(&remote, &["update-server-info"]);
    let selection = NativeConfigSelection::new(
        f._temp.path().into(),
        Path::new(info["root"].as_str().unwrap()).join("home"),
    )
    .unwrap()
    .with_variable("GIT_CONFIG_NOSYSTEM", "1".into())
    .unwrap()
    .with_variable("GIT_CONFIG_GLOBAL", "/dev/null".into())
    .unwrap()
    .with_variable("GIT_ATTR_NOSYSTEM", "1".into())
    .unwrap();
    let profile = NativeGitAdmission::direct_https(
        Arc::new(vibe_registry::HttpAdmission::new(["127.0.0.2"]).unwrap()),
        GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap(),
        TlsTrust::Pem(fs::read(info["ca"].as_str().unwrap()).unwrap()),
    )
    .with_config_selection(selection);
    let resolver = vibe_registry::MultiRegistryResolver::from_manifest_with_native_admission(
        &[],
        &[],
        &[],
        f._temp.path().join("held-native-cache"),
        0,
        Arc::new(profile),
    )
    .unwrap()
    .with_locked_packages(lock.packages)
    .with_locked_updates();
    let denied_log = Path::new(info["root"].as_str().unwrap()).join("b.tcp.log");
    let denied_before = fs::read(&denied_log).unwrap_or_default();
    let selected = resolver
        .resolve(&PackageRef::parse("org.vibevm/giant@1.1.0").unwrap())
        .unwrap();
    let prepared = resolver.prepare_in_place(&selected, &f.slot).unwrap();
    let staged = prepared
        .stage_native(vibe_registry::git_backend::PayloadBytes::AsConfigured)
        .unwrap()
        .unwrap();
    let path = staged.staged_path();
    assert_eq!(
        fs::read_to_string(path.join(".git/refs/tags/local-sentinel"))
            .unwrap()
            .trim(),
        previous
    );
    assert!(
        fs::read_to_string(path.join(".git/config"))
            .unwrap()
            .contains("preserve this value")
    );
    assert_eq!(
        fs::read(path.join(".git/sentinel")).unwrap(),
        b"preserve raw metadata"
    );
    assert_eq!(fs::read(&denied_log).unwrap_or_default(), denied_before);
    assert_eq!(
        git(&f.slot, &["rev-parse", "HEAD"]),
        previous,
        "staging cannot publish"
    );
}
