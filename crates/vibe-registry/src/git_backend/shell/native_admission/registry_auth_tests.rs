//! Declared registry auth narrows credentials, not anonymous metadata transports.
#![cfg(all(test, unix))]
use super::ssh_tests::Fixture as SshFixture;
use super::tests::Fixture as HttpsFixture;
use super::*;
use crate::{GitBackend, GitPerPackageRegistry, RegistryError, ResolvedPackage, ShellGit};
use vibe_core::manifest::{AuthKind, NamingConvention};

fn mixed(https: &HttpsFixture, ssh: &SshFixture, agent: bool) -> ShellGit {
    let authentication = if agent {
        SshAuthentication::AgentSocket(ssh.get("agent_socket").into())
    } else {
        SshAuthentication::Key(std::fs::read(ssh.get("key")).unwrap())
    };
    let selected = SshProfile::explicit_direct(
        "/usr/bin/ssh".into(),
        "127.0.0.1".into(),
        ssh.get("user").into(),
        ssh.port(),
        std::fs::read(ssh.get("known_hosts")).unwrap(),
        authentication,
    )
    .unwrap();
    let profile = NativeGitAdmission::direct_https(
        Arc::new(crate::HttpAdmission::new(["127.0.0.2"]).unwrap()),
        GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap(),
        TlsTrust::Pem(std::fs::read(https.get("ca")).unwrap()),
    )
    .with_ssh_profile(selected);
    ShellGit::new().with_native_admission(Arc::new(profile))
}
fn selected() -> ResolvedPackage {
    ResolvedPackage {
        group: vibe_core::Group::parse("org.fixture").unwrap(),
        name: "demo".into(),
        version: semver::Version::new(1, 0, 0),
        source_dir: PathBuf::new(),
    }
}
fn https_token(f: &HttpsFixture) -> String {
    format!(
        "https://x-access-token:SYNTHETIC_N1_AUTH@{}/auth.git",
        f.get("a").trim_start_matches("https://")
    )
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-SSH-LAW")]
fn launch_matrix_never_uses_a_different_credential_mechanism() {
    let https = HttpsFixture::new();
    let ssh = SshFixture::new();
    let plain = format!("{}/repo.git", https.get("a"));
    let token = https_token(&https);
    for agent in [false, true] {
        let direct = mixed(&https, &ssh, agent);
        for auth in [AuthKind::None, AuthKind::TokenEnv, AuthKind::Ssh] {
            let bound = direct.bound_for_registry_auth(auth).unwrap().unwrap();
            // Anonymous HTTPS remains useful under every declared mechanism.
            assert_eq!(bound.list_tags(&plain).unwrap(), ["v1.0.0", "v2.0.0"]);
            let before_http = https.contacts("a");
            let basic = bound.list_tags(&token);
            if auth == AuthKind::TokenEnv {
                assert!(basic.is_ok());
                assert!(https.contacts("a") > before_http);
            } else {
                assert!(matches!(basic, Err(GitError::NativeContext { .. })));
                assert_eq!(https.contacts("a"), before_http);
            }
            let before_ssh = ssh.contacts("a");
            let result = bound.list_tags(ssh.get("url"));
            if auth == AuthKind::Ssh {
                assert_eq!(result.unwrap(), ["v1.0.0", "v1.1.0"]);
                assert!(ssh.contacts("a") > before_ssh);
            } else {
                assert!(matches!(result, Err(GitError::NativeContext { .. })));
                assert_eq!(ssh.contacts("a"), before_ssh);
            }
        }
        // Cloning restrictions never mutate the directly selected authority.
        assert!(direct.list_tags(&token).is_ok());
        assert!(direct.list_tags(ssh.get("url")).is_ok());
    }
    assert_eq!(https.contacts("b"), 0);
    assert_eq!(ssh.contacts("b"), 1);
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-SSH-LAW")]
fn registry_binding_is_immutable_and_anonymous_clones_cannot_be_upgraded() {
    let https = HttpsFixture::new();
    let ssh = SshFixture::new();
    let direct = mixed(&https, &ssh, false);
    let token = direct
        .bound_for_registry_auth(AuthKind::TokenEnv)
        .unwrap()
        .unwrap();
    assert!(token.bound_for_registry_auth(AuthKind::Ssh).is_err());
    let same = token
        .bound_for_registry_auth(AuthKind::TokenEnv)
        .unwrap()
        .unwrap();
    assert!(same.list_tags(&https_token(&https)).is_ok());
    let narrowed = token
        .bound_for_registry_auth(AuthKind::None)
        .unwrap()
        .unwrap();
    assert!(
        narrowed
            .bound_for_registry_auth(AuthKind::TokenEnv)
            .is_err()
    );
    assert!(narrowed.bound_for_registry_auth(AuthKind::Ssh).is_err());
    let anonymous = direct.anonymized_for_public().unwrap();
    assert!(anonymous.bound_for_registry_auth(AuthKind::Ssh).is_err());
    let before = (https.contacts("a"), ssh.contacts("a"));
    assert!(narrowed.list_tags(&https_token(&https)).is_err());
    assert!(anonymous.list_tags(ssh.get("url")).is_err());
    assert_eq!((https.contacts("a"), ssh.contacts("a")), before);
    assert!(
        narrowed
            .list_tags(&format!("{}/repo.git", https.get("a")))
            .is_ok()
    );
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-SSH-LAW")]
fn direct_constructors_enforce_mechanisms_before_actual_request() {
    let https = HttpsFixture::new();
    let ssh = SshFixture::new();
    let tmp = tempfile::tempdir().unwrap();
    for agent in [false, true] {
        let backend = Arc::new(mixed(&https, &ssh, agent));
        let open = |auth, org: &str, token: Option<String>| {
            GitPerPackageRegistry::open_with_explicit_token(
                "fixture",
                org,
                "main",
                NamingConvention::Fqdn,
                vec![],
                tmp.path(),
                backend.clone(),
                0,
                auth,
                token,
            )
            .unwrap()
        };
        // This same composed SSH address is genuinely served by the fixture.
        let allowed = open(AuthKind::Ssh, ssh.get("url"), None);
        assert_eq!(
            allowed
                .prepare_in_place(&selected(), &tmp.path().join("ssh-allowed"))
                .unwrap()
                .resolved_commit(),
            ssh.get("head")
        );
        let blocked = open(
            AuthKind::TokenEnv,
            ssh.get("url"),
            Some("SYNTHETIC_N1_AUTH".into()),
        );
        let before = ssh.contacts("a");
        assert!(matches!(
            blocked.prepare_in_place(&selected(), &tmp.path().join("ssh-blocked")),
            Err(RegistryError::Git(GitError::NativeContext { .. }))
        ));
        assert_eq!(ssh.contacts("a"), before);
        let http_org = format!("{}/auth", https.get("a"));
        let allowed = open(
            AuthKind::TokenEnv,
            &http_org,
            Some("SYNTHETIC_N1_AUTH".into()),
        );
        assert_eq!(
            allowed
                .prepare_in_place(&selected(), &tmp.path().join("token-allowed"))
                .unwrap()
                .resolved_commit(),
            https.get("head")
        );
        // Test-only explicit-token constructor must not create authority that
        // contradicts declared Ssh; the real URL-userinfo path is checked below.
        let blocked = open(AuthKind::Ssh, &http_org, Some("SYNTHETIC_N1_AUTH".into()));
        let before = https.contacts("a");
        assert!(matches!(
            blocked.prepare_in_place(&selected(), &tmp.path().join("token-blocked")),
            Err(RegistryError::Git(GitError::NativeContext { .. }))
        ));
        assert_eq!(https.contacts("a"), before);
        let url = https_token(&https);
        let blocked = GitPerPackageRegistry::open_single_package(
            "ssh-declared",
            &url,
            "main",
            tmp.path(),
            backend.clone(),
            0,
            AuthKind::Ssh,
            None,
        )
        .unwrap();
        let before = https.contacts("a");
        assert!(matches!(
            blocked.prepare_in_place(&selected(), &tmp.path().join("userinfo-blocked")),
            Err(RegistryError::Git(GitError::NativeContext { .. }))
        ));
        assert_eq!(https.contacts("a"), before);
        let plain = GitPerPackageRegistry::open_single_package(
            "public",
            &format!("{}/repo.git", https.get("a")),
            "main",
            tmp.path(),
            backend.clone(),
            0,
            AuthKind::None,
            None,
        )
        .unwrap();
        assert_eq!(
            plain
                .prepare_in_place(&selected(), &tmp.path().join("public-allowed"))
                .unwrap()
                .resolved_commit(),
            https.get("current_head")
        );
    }
}
