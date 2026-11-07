//! Numeric-loopback TLS integration, with real Git and isolated fixture inputs.
#![cfg(all(test, unix))]
use super::*;
use crate::{GitBackend, ShellGit};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
pub(super) struct Fixture {
    child: std::process::Child,
    info: serde_json::Value,
}
impl Fixture {
    pub(super) fn new() -> Self {
        let mut child = Command::new("python3")
            .args(["-u", "-c", include_str!("tls_fixture.py")])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let info = serde_json::from_str(&line).expect("fixture must initialise successfully");
        Self { child, info }
    }
    pub(super) fn get(&self, key: &str) -> &str {
        self.info[key].as_str().unwrap()
    }
    fn backend(&self) -> ShellGit {
        let profile = NativeGitAdmission::direct_https(
            Arc::new(crate::HttpAdmission::new(["127.0.0.2", "blocked.invalid"]).unwrap()),
            GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap(),
            TlsTrust::Pem(std::fs::read(self.get("ca")).unwrap()),
        );
        ShellGit::new().with_native_admission(Arc::new(profile))
    }
    pub(super) fn contacts(&self, host: &str) -> usize {
        std::fs::read_to_string(Path::new(self.get("root")).join(format!("{host}.log")))
            .unwrap_or_default()
            .lines()
            .count()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.child.stdin.take();
        let _ = self.child.wait();
    }
}
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-PREPARATION")]
fn direct_https_prepares_exact_objects_with_tls_auth_and_no_legacy_escape() {
    let f = Fixture::new();
    let git = f.backend();
    let temp = tempfile::tempdir().unwrap();
    let url = format!("{}/repo.git", f.get("a"));
    assert_eq!(git.list_tags(&url).unwrap(), ["v1.0.0", "v2.0.0"]);
    let objects = temp.path().join("objects.git");
    let dest = temp.path().join("slot");
    let revision = git
        .prepare_ref(&url, "v1.0.0", None, &objects, &dest)
        .unwrap();
    assert_eq!(revision.commit, f.get("head"));
    assert_ne!(f.get("head"), f.get("current_head"));
    assert_eq!(
        git.head_commit(&objects).unwrap().as_deref(),
        Some(f.get("current_head"))
    );
    assert!(
        matches!(git.prepare_ref(&url, "v2.0.0", None, &temp.path().join("submodules.git"), &dest), Err(GitError::PreparedValidation { source }) if matches!(source.as_ref(),GitError::NativeUnavailable {..}))
    );
    assert_eq!(
        git.file_at_commit(&objects, &revision.commit, "payload.txt")
            .unwrap(),
        b"retained fixture\n"
    );
    assert_eq!(git.recorded_origin_url(&objects).unwrap(), url);
    git.set_remote_url(&objects, "origin", &url).unwrap();
    assert!(matches!(
        git.apply_prepared_ref(&objects, &revision, &url, None, &dest, false),
        Err(GitError::NativeUnavailable { .. })
    ));
    assert!(!dest.exists());
    assert!(git.bootstrap(&url, "v1.0.0", &dest).is_err());
    assert!(git.update(&dest, "v1.0.0").is_err());
    assert!(!dest.exists());
    let before = f.contacts("b");
    assert!(matches!(
        git.list_tags(&format!("{}/repo.git", f.get("b"))),
        Err(GitError::ContactRefused(_))
    ));
    assert!(matches!(
        git.list_tags("https://blocked.invalid./repo.git"),
        Err(GitError::ContactRefused(_))
    ));
    assert_eq!(f.contacts("b"), before);
    let redirected = format!("{}/redirect.git", f.get("a"));
    assert!(git.list_tags(&redirected).is_err());
    assert_eq!(f.contacts("b"), before);
    let credentialed = format!(
        "https://x-access-token:SYNTHETIC_N1_AUTH@{}/auth.git",
        f.get("a").trim_start_matches("https://")
    );
    assert!(git.list_tags(&format!("{}/auth.git", f.get("a"))).is_err());
    assert_eq!(git.list_tags(&credentialed).unwrap(), ["v1.0.0", "v2.0.0"]);
    let auth_objects = temp.path().join("auth.git");
    let auth = git
        .prepare_ref(&credentialed, "v1.0.0", None, &auth_objects, &dest)
        .unwrap();
    assert_eq!(auth.commit, f.get("head"));
    let config = std::fs::read_to_string(auth_objects.join("config")).unwrap();
    assert!(!config.contains("SYNTHETIC_N1_AUTH") && !config.contains("extraHeader"));
    let anon = git.anonymized_for_public().unwrap();
    assert!(anon.list_tags(&credentialed).is_err());
    assert_eq!(anon.list_tags(&url).unwrap(), ["v1.0.0", "v2.0.0"]);
    assert!(anon.list_tags(&format!("{}/repo.git", f.get("b"))).is_err());
    assert_eq!(f.contacts("b"), 0);
    assert!(
        git.file_at_commit(&objects, &"0".repeat(40), "payload.txt")
            .is_err()
    );
    assert!(f.contacts("a") > 0);
}
#[test]
fn admitted_installation_cannot_be_replaced_by_ordinary_binary_override() {
    let f = Fixture::new();
    let git = f.backend().with_binary("/definitely/not/git");
    assert_eq!(
        git.list_tags(&format!("{}/repo.git", f.get("a"))).unwrap(),
        ["v1.0.0", "v2.0.0"]
    );
    assert!(git.run(&["--version"], None).is_err());
    assert!(git.run_raw(&["--version"], None).is_err());
}
#[test]
fn wrong_tls_trust_fails_instead_of_disabling_verification() {
    let f = Fixture::new();
    let profile = NativeGitAdmission::direct_https(
        Arc::new(crate::HttpAdmission::new(Vec::<String>::new()).unwrap()),
        GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap(),
        TlsTrust::System,
    );
    let git = ShellGit::new().with_native_admission(Arc::new(profile));
    assert!(git.list_tags(&format!("{}/repo.git", f.get("a"))).is_err());
    assert_eq!(f.contacts("a"), 0);
}
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-PREPARATION")]
fn token_registry_preparation_uses_incumbent_basic_auth_and_refuses_apply_before_effects() {
    use crate::{GitPerPackageRegistry, ResolvedPackage};
    use vibe_core::manifest::NamingConvention;
    let f = Fixture::new();
    let temp = tempfile::tempdir().unwrap();
    let slot = temp.path().join("slot");
    let registry = GitPerPackageRegistry::open_with_explicit_token(
        "fixture",
        &format!("{}/auth", f.get("a")),
        "main",
        NamingConvention::Fqdn,
        vec![],
        &temp.path().join("cache"),
        Arc::new(f.backend()),
        3600,
        vibe_core::manifest::AuthKind::TokenEnv,
        Some("SYNTHETIC_N1_AUTH".into()),
    )
    .unwrap();
    let selected = ResolvedPackage {
        group: vibe_core::Group::parse("org.fixture").unwrap(),
        name: "demo".into(),
        version: semver::Version::new(1, 0, 0),
        source_dir: PathBuf::new(),
    };
    let prepared = registry.prepare_in_place(&selected, &slot).unwrap();
    assert_eq!(prepared.resolved_commit(), f.get("head"));
    assert_eq!(prepared.manifest().package.as_ref().unwrap().name, "demo");
    assert!(
        !serde_json::to_string(&prepared.source_evidence())
            .unwrap()
            .contains("SYNTHETIC")
    );
    assert!(matches!(
        prepared.materialise(),
        Err(crate::RegistryError::Git(
            GitError::NativeUnavailable { .. }
        ))
    ));
    assert!(!slot.exists());
    std::fs::create_dir(&slot).unwrap();
    std::fs::write(slot.join("keep"), "untouched debris").unwrap();
    assert!(registry.materialise_in_place(&selected, &slot).is_err());
    assert_eq!(
        std::fs::read_to_string(slot.join("keep")).unwrap(),
        "untouched debris"
    );
    for entry in walkdir::WalkDir::new(temp.path())
        .into_iter()
        .map(Result::unwrap)
        .filter(|e| e.file_type().is_file())
    {
        let bytes = std::fs::read(entry.path()).unwrap();
        assert!(
            !String::from_utf8_lossy(&bytes).contains("SYNTHETIC_N1_AUTH"),
            "secret persisted"
        );
    }
    assert_eq!(f.contacts("b"), 0);
}
fn fixture_git(info: &serde_json::Value, args: &[&str], cwd: &Path) -> std::process::Output {
    Command::new("/usr/bin/git")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env(
            "HOME",
            Path::new(info["root"].as_str().unwrap()).join("home"),
        )
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_SSL_CAINFO", info["ca"].as_str().unwrap())
        .env("GIT_TERMINAL_PROMPT", "0")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap()
}
#[test]
fn warm_rewrite_and_alternate_objects_have_real_legacy_b_contact_controls() {
    let f = Fixture::new();
    let temp = tempfile::tempdir().unwrap();
    let git = f.backend();
    let work = Path::new(f.get("work"));
    assert_eq!(
        git.head_commit(work).unwrap().as_deref(),
        Some(f.get("current_head"))
    );
    let before_head = std::fs::read(work.join(".git/HEAD")).unwrap();
    let a = format!("{}/repo.git", f.get("a"));
    let b = format!("{}/repo.git", f.get("b"));
    assert!(
        fixture_git(&f.info, &["remote", "add", "origin", &a], work)
            .status
            .success()
    );
    let include = temp.path().join("rewrite.config");
    std::fs::write(
        &include,
        format!("[url \"{}/\"]\n insteadOf = {}/\n", f.get("b"), f.get("a")),
    )
    .unwrap();
    assert!(
        fixture_git(
            &f.info,
            &["config", "include.path", include.to_str().unwrap()],
            work
        )
        .status
        .success()
    );
    let legacy = fixture_git(&f.info, &["ls-remote", "origin"], work);
    assert!(
        legacy.status.success(),
        "{}",
        String::from_utf8_lossy(&legacy.stderr)
    );
    assert!(f.contacts("b") > 0);
    let metadata: std::collections::BTreeMap<_, _> = walkdir::WalkDir::new(work.join(".git"))
        .into_iter()
        .map(Result::unwrap)
        .filter(|e| e.file_type().is_file())
        .map(|e| (e.path().to_path_buf(), std::fs::read(e.path()).unwrap()))
        .collect();
    let before_a = f.contacts("a");
    let before_b = f.contacts("b");
    assert!(matches!(
        GitBackend::origin_url(&git, work),
        Err(GitError::NativeContext { .. })
    ));
    assert!(matches!(
        git.prepare_ref(
            &a,
            "v1.0.0",
            Some(work),
            &temp.path().join("warm.git"),
            &temp.path().join("slot")
        ),
        Err(GitError::NativeContext { .. })
    ));
    assert_eq!((f.contacts("a"), f.contacts("b")), (before_a, before_b));
    for (path, bytes) in metadata {
        assert_eq!(
            std::fs::read(path).unwrap(),
            bytes,
            "warm metadata changed during refusal"
        );
    }
    let alternate = format!("{}/alternate.git", f.get("a"));
    let legacy = fixture_git(
        &f.info,
        &[
            "-c",
            "http.followRedirects=true",
            "clone",
            "--bare",
            &alternate,
            temp.path().join("legacy.git").to_str().unwrap(),
        ],
        temp.path(),
    );
    assert!(
        legacy.status.success(),
        "{}",
        String::from_utf8_lossy(&legacy.stderr)
    );
    assert!(f.contacts("b") > before_b);
    let before = f.contacts("b");
    assert!(
        git.prepare_ref(
            &alternate,
            "v1.0.0",
            None,
            &temp.path().join("bounded.git"),
            &temp.path().join("slot")
        )
        .is_err()
    );
    assert_eq!(f.contacts("b"), before);
    assert_eq!(git.list_tags(&a).unwrap(), ["v1.0.0", "v2.0.0"]);
    assert!(!b.is_empty());
    assert_eq!(std::fs::read(work.join(".git/HEAD")).unwrap(), before_head);
}
#[test]
fn clean_child_environment_ignores_global_proxy_helper_and_config_injection() {
    let f = Fixture::new();
    let temp = tempfile::tempdir().unwrap();
    let global = temp.path().join("global");
    std::fs::write(
        &global,
        format!("[url \"{}/\"]\n insteadOf = {}/\n", f.get("b"), f.get("a")),
    )
    .unwrap();
    let legacy = Command::new("/usr/bin/git")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", temp.path())
        .env("GIT_CONFIG_GLOBAL", &global)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_SSL_CAINFO", f.get("ca"))
        .args(["ls-remote", &format!("{}/repo.git", f.get("a"))])
        .output()
        .unwrap();
    assert!(legacy.status.success());
    assert!(f.contacts("b") > 0);
    let before = f.contacts("b");
    let out = Command::new(std::env::current_exe().unwrap())
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", temp.path())
        .env("GIT_CONFIG_GLOBAL", &global)
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "http.extraHeader")
        .env("GIT_CONFIG_VALUE_0", "Authorization: SYNTHETIC_BAD")
        .env("HTTPS_PROXY", f.get("b"))
        .env("GIT_EXEC_PATH", "/nonexistent")
        .env("GIT_ASKPASS", "/nonexistent")
        .env("GIT_SSH_COMMAND", "/nonexistent")
        .env(
            "VIBE_NATIVE_TEST_INFO",
            serde_json::to_string(&f.info).unwrap(),
        )
        .args([
            "--exact",
            "git_backend::shell::native_admission::tests::environment_child",
            "--nocapture",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{} {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(f.contacts("b"), before);
    assert!(f.contacts("a") > 0);
}
#[test]
fn environment_child() {
    let Ok(info) = std::env::var("VIBE_NATIVE_TEST_INFO") else {
        return;
    };
    let info: serde_json::Value = serde_json::from_str(&info).unwrap();
    let profile = NativeGitAdmission::direct_https(
        Arc::new(crate::HttpAdmission::new(["127.0.0.2"]).unwrap()),
        GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap(),
        TlsTrust::Pem(std::fs::read(info["ca"].as_str().unwrap()).unwrap()),
    );
    let git = ShellGit::new().with_native_admission(Arc::new(profile));
    assert_eq!(
        git.list_tags(&format!("{}/repo.git", info["a"].as_str().unwrap()))
            .unwrap(),
        ["v1.0.0", "v2.0.0"]
    );
}

#[test]
fn native_version_floor_is_explicit_and_legacy_is_not_restricted() {
    assert!(supported_version("git version 2.53.0"));
    assert!(supported_version("git version 2.53.1.windows.1"));
    assert!(!supported_version("git version 2.52.9"));
    assert!(!supported_version("unrecognised"));
}
#[test]
fn unsupported_registry_auth_refuses_before_contact() {
    let f = Fixture::new();
    let git = f.backend();
    for kind in [
        vibe_core::manifest::AuthKind::CredentialHelper,
        vibe_core::manifest::AuthKind::Ssh,
    ] {
        assert!(matches!(
            git.preflight_registry_auth(kind),
            Err(GitError::NativeContext { .. })
        ));
    }
    assert_eq!(f.contacts("a"), 0);
    assert_eq!(f.contacts("b"), 0);
}

#[test]
fn credentials_keep_exact_origins_while_denial_uses_normalized_hosts() {
    let installation =
        GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap();
    let profile = NativeGitAdmission::direct_https(
        Arc::new(crate::HttpAdmission::new(["blocked.invalid"]).unwrap()),
        installation,
        TlsTrust::System,
    );
    assert!(matches!(
        profile.capture_address("https://x-access-token:SYNTHETIC@blocked.invalid./r", false),
        Err(GitError::ContactRefused(_))
    ));
    let a = profile
        .capture_address("https://x-access-token:SYNTHETIC@example.invalid/r", false)
        .unwrap();
    let dotted = profile
        .capture_address("https://x-access-token:SYNTHETIC@example.invalid./r", false)
        .unwrap();
    let port = profile
        .capture_address(
            "https://x-access-token:SYNTHETIC@example.invalid:444/r",
            false,
        )
        .unwrap();
    assert_ne!(
        Url::parse(a.address()).unwrap().origin(),
        Url::parse(dotted.address()).unwrap().origin()
    );
    assert_ne!(
        Url::parse(a.address()).unwrap().origin(),
        Url::parse(port.address()).unwrap().origin()
    );
    assert_eq!(a.address(), "https://example.invalid/r");
    assert!(
        profile
            .capture_address("https://x-access-token:SYNTHETIC@example.invalid/r", true)
            .is_err()
    );
}

#[test]
fn smart_https_preparation_keeps_existing_transfer_journal_observation() {
    use vibe_core::progress::transfer::{TransferJournal, Transfers};
    let f = Fixture::new();
    let temp = tempfile::tempdir().unwrap();
    let git = f.backend();
    let journal = TransferJournal::new(temp.path());
    git.attach_transfers(&Transfers::new(
        Some(journal.clone()),
        None,
        "native fixture",
    ));
    let url = format!("{}/smart/repo.git", f.get("a"));
    let revision = git
        .prepare_ref(
            &url,
            "v1.0.0",
            None,
            &temp.path().join("smart.git"),
            &temp.path().join("slot"),
        )
        .unwrap();
    assert_eq!(revision.commit, f.get("head"));
    let history = journal.history();
    assert!(
        !history.is_empty(),
        "real smart HTTPS clone must report transfer bytes"
    );
    assert!(history.iter().any(|entry| entry.bytes > 0));
    assert!(
        history
            .iter()
            .any(|entry| entry.written.is_some_and(|bytes| bytes > 0))
    );
    assert!(history.iter().all(|entry| entry.record.url == url));
    assert_eq!(f.contacts("b"), 0);
    assert!(
        std::fs::read_to_string(Path::new(f.get("root")).join("a.log"))
            .unwrap()
            .contains("POST /smart/repo.git/git-upload-pack")
    );
}
#[test]
fn denied_mirror_stays_typed_after_primary_absence_and_stops_the_next_route() {
    use crate::{GitPerPackageRegistry, ResolvedPackage};
    use vibe_core::manifest::NamingConvention;
    let f = Fixture::new();
    let temp = tempfile::tempdir().unwrap();
    assert_eq!(
        f.backend()
            .list_tags(&format!("{}/public/org.fixture.demo.git", f.get("a")))
            .unwrap(),
        ["v1.0.0", "v2.0.0"]
    );
    std::fs::write(Path::new(f.get("root")).join("a.log"), "").unwrap();
    let registry = GitPerPackageRegistry::open_with_explicit_token(
        "fixture",
        &format!("{}/missing", f.get("a")),
        "main",
        NamingConvention::Fqdn,
        vec![
            format!("{}/public", f.get("b")),
            format!("{}/public", f.get("a")),
        ],
        &temp.path().join("cache"),
        Arc::new(f.backend()),
        3600,
        vibe_core::manifest::AuthKind::None,
        None,
    )
    .unwrap();
    let selected = ResolvedPackage {
        group: vibe_core::Group::parse("org.fixture").unwrap(),
        name: "demo".into(),
        version: semver::Version::new(1, 0, 0),
        source_dir: PathBuf::new(),
    };
    assert!(matches!(
        registry.prepare_in_place(&selected, &temp.path().join("slot")),
        Err(crate::RegistryError::ContactRefused(
            crate::ContactRefusal::ExcludedHost { .. }
        ))
    ));
    assert_eq!(f.contacts("b"), 0);
    let requests = std::fs::read_to_string(Path::new(f.get("root")).join("a.log")).unwrap();
    assert!(requests.contains("/missing/"));
    assert!(!requests.contains("/public/"));
}

#[path = "materialisation_tests.rs"]
mod materialisation_tests;

#[test]
fn direct_public_https_constructor_retains_useful_anonymous_preparation() {
    let f = Fixture::new();
    let temp = tempfile::tempdir().unwrap();
    let registry = crate::GitPerPackageRegistry::open_single_package(
        "public",
        &format!("{}/repo.git", f.get("a")),
        "main",
        temp.path(),
        Arc::new(f.backend()),
        0,
        vibe_core::manifest::AuthKind::None,
        None,
    )
    .unwrap();
    let selected = crate::ResolvedPackage {
        group: vibe_core::Group::parse("org.fixture").unwrap(),
        name: "demo".into(),
        version: semver::Version::new(1, 0, 0),
        source_dir: PathBuf::new(),
    };
    assert_eq!(
        registry
            .prepare_in_place(&selected, &temp.path().join("slot"))
            .unwrap()
            .resolved_commit(),
        f.get("head")
    );
    assert_eq!(f.contacts("b"), 0);
}
