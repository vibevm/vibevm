//! Default Git initial redirect controls and captured credential observations.
#![cfg(all(test, unix))]
use super::tests::Fixture;
use super::*;
use crate::{GitBackend, ShellGit};
use std::process::{Command, Output};

fn profile(f: &Fixture, root: &Path, denied: Vec<&str>) -> Result<NativeGitAdmission, GitError> {
    let selection = NativeConfigSelection::new(root.into(), Path::new(f.get("root")).join("home"))?
        .with_variable("GIT_CONFIG_NOSYSTEM", "1".into())?
        .with_variable("GIT_CONFIG_GLOBAL", "/dev/null".into())?
        .with_variable("GIT_ATTR_NOSYSTEM", "1".into())?;
    Ok(NativeGitAdmission::direct_https(
        Arc::new(crate::HttpAdmission::new(denied)?),
        GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into())?,
        TlsTrust::Pem(std::fs::read(f.get("ca")).map_err(io)?),
    )
    .with_config_selection(selection))
}
fn legacy(f: &Fixture, root: &Path, args: &[&str]) -> std::io::Result<Output> {
    Command::new("/usr/bin/git")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", Path::new(f.get("root")).join("home"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_ATTR_NOSYSTEM", "1")
        .env("GIT_SSL_CAINFO", f.get("ca"))
        .env("GIT_ALLOW_PROTOCOL", "https")
        .current_dir(root)
        .args(args)
        .output()
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES-LAW")]
fn default_initial_redirect_reuses_final_base_and_preserves_logical_origin() {
    let f = Fixture::new();
    let root = tempfile::tempdir().unwrap();
    let original = format!("{}/redirect.git", f.get("a"));
    let actual = format!("{}/repo.git", f.get("b"));
    let legacy_repo = root.path().join("legacy.git");
    let result = legacy(
        &f,
        root.path(),
        &["clone", "--bare", &original, legacy_repo.to_str().unwrap()],
    )
    .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let legacy_origin = legacy(
        &f,
        root.path(),
        &[
            "--git-dir",
            legacy_repo.to_str().unwrap(),
            "config",
            "remote.origin.url",
        ],
    )
    .unwrap();
    assert!(legacy_origin.status.success());
    assert_eq!(
        String::from_utf8(legacy_origin.stdout).unwrap().trim(),
        original
    );
    let git =
        ShellGit::new().with_native_admission(Arc::new(profile(&f, root.path(), vec![]).unwrap()));
    let objects = root.path().join("owned.git");
    let destination = root.path().join("slot");
    let revision = git
        .prepare_ref(&original, "v1.0.0", None, &objects, &destination)
        .unwrap();
    assert_eq!(revision.commit, f.get("head"));
    assert_eq!(
        git.prepared_serving_endpoint(&objects).unwrap().as_deref(),
        Some(actual.as_str())
    );
    assert_eq!(git.recorded_origin_url(&objects).unwrap(), original);
    let before = (f.connections("a"), f.connections("b"));
    let stage = git
        .stage_native_materialisation(&objects, &revision)
        .unwrap()
        .unwrap();
    assert_eq!(
        std::fs::read(stage.staged_path().join("payload.txt")).unwrap(),
        b"retained fixture\n"
    );
    assert_eq!((f.connections("a"), f.connections("b")), before);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES-LAW")]
fn subsequent_object_redirect_is_refused_like_default_git_but_not_follow_true() {
    let f = Fixture::new();
    let root = tempfile::tempdir().unwrap();
    let address = format!("{}/late.git", f.get("a"));
    let ordinary = legacy(
        &f,
        root.path(),
        &[
            "clone",
            "--bare",
            &address,
            root.path().join("default.git").to_str().unwrap(),
        ],
    )
    .unwrap();
    assert!(!ordinary.status.success());
    assert_eq!(f.connections("b"), 0);
    let git =
        ShellGit::new().with_native_admission(Arc::new(profile(&f, root.path(), vec![]).unwrap()));
    assert!(
        git.prepare_ref(
            &address,
            "v1.0.0",
            None,
            &root.path().join("owned.git"),
            &root.path().join("slot")
        )
        .is_err()
    );
    assert_eq!(f.connections("b"), 0);
    assert!(!root.path().join("slot").exists());
    let nondefault = legacy(
        &f,
        root.path(),
        &[
            "-c",
            "http.followRedirects=true",
            "clone",
            "--bare",
            &address,
            root.path().join("always.git").to_str().unwrap(),
        ],
    )
    .unwrap();
    assert!(
        nondefault.status.success(),
        "{}",
        String::from_utf8_lossy(&nondefault.stderr)
    );
    assert!(f.connections("b") > 0);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES-LAW")]
fn return_to_original_origin_never_reattaches_native_token() {
    let f = Fixture::new();
    let root = tempfile::tempdir().unwrap();
    let git =
        ShellGit::new().with_native_admission(Arc::new(profile(&f, root.path(), vec![]).unwrap()));
    let address = format!(
        "https://x-access-token:SYNTHETIC_N1_AUTH@{}/roundtrip.git",
        f.get("a").trim_start_matches("https://")
    );
    let objects = root.path().join("owned.git");
    let revision = git
        .prepare_ref(
            &address,
            "v1.0.0",
            None,
            &objects,
            &root.path().join("slot"),
        )
        .unwrap();
    assert_eq!(revision.commit, f.get("head"));
    assert_eq!(
        git.prepared_serving_endpoint(&objects).unwrap(),
        Some(format!("{}/anonymous.git", f.get("a")))
    );
    let a = std::fs::read_to_string(Path::new(f.get("root")).join("a.headers.log")).unwrap();
    let b = std::fs::read_to_string(Path::new(f.get("root")).join("b.headers.log")).unwrap();
    assert!(
        a.lines()
            .any(|line| line.starts_with("/roundtrip.git/") && line.ends_with("auth=yes"))
    );
    assert!(
        a.lines()
            .filter(|line| line.starts_with("/anonymous.git/"))
            .all(|line| line.ends_with("auth=no"))
    );
    assert!(b.lines().all(|line| line.ends_with("auth=no")));
    assert!(
        !String::from_utf8_lossy(&std::fs::read(objects.join("config")).unwrap())
            .contains("SYNTHETIC")
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES-LAW")]
fn denied_initial_hop_queries_and_oversized_metadata_refuse_before_publication() {
    let f = Fixture::new();
    let root = tempfile::tempdir().unwrap();
    let git = ShellGit::new().with_native_admission(Arc::new(
        profile(&f, root.path(), vec!["127.0.0.2"]).unwrap(),
    ));
    assert!(matches!(
        git.list_tags(&format!("{}/redirect.git", f.get("a"))),
        Err(GitError::ContactRefused(_))
    ));
    assert_eq!(f.connections("b"), 0);
    let before = (f.connections("a"), f.connections("b"));
    let error = git
        .list_tags(&format!("{}/repo.git?SYNTHETIC_QUERY", f.get("a")))
        .unwrap_err();
    assert!(!error.to_string().contains("SYNTHETIC_QUERY"));
    assert_eq!((f.connections("a"), f.connections("b")), before);
    let git =
        ShellGit::new().with_native_admission(Arc::new(profile(&f, root.path(), vec![]).unwrap()));
    let result = git.prepare_ref(
        &format!("{}/oversize-start.git", f.get("a")),
        "v1.0.0",
        None,
        &root.path().join("objects.git"),
        &root.path().join("slot"),
    );
    assert!(
        matches!(result,Err(GitError::NativeContext{reason}) if reason.contains("prefix exceeds"))
    );
    assert!(!root.path().join("slot").exists());
    assert!(!root.path().join("objects.git").exists());
}
