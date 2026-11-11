//! Real Git/OpenSSH fixtures; no operator key, agent, profile or daemon is used.
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
            .args(["-u", "-c", include_str!("ssh_fixture.py")])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        Self {
            child,
            info: serde_json::from_str(&line).expect("isolated SSH fixture must start"),
        }
    }
    pub(super) fn get(&self, key: &str) -> &str {
        self.info[key].as_str().unwrap()
    }
    pub(super) fn port(&self) -> u16 {
        self.info["port"].as_u64().unwrap() as u16
    }
    fn backend(&self, key: &str, hosts: &str, agent: bool) -> ShellGit {
        let authentication = if agent {
            SshAuthentication::AgentSocket(self.get("agent_socket").into())
        } else {
            SshAuthentication::Key(std::fs::read(self.get(key)).unwrap())
        };
        let ssh = SshProfile::explicit_direct(
            "/usr/bin/ssh".into(),
            "127.0.0.1".into(),
            self.get("user").into(),
            self.info["port"].as_u64().unwrap() as u16,
            std::fs::read(self.get(hosts)).unwrap(),
            authentication,
        )
        .unwrap();
        let selection =
            NativeConfigSelection::new(self.get("root").into(), self.get("home").into())
                .unwrap()
                .with_variable("GIT_CONFIG_NOSYSTEM", "1".into())
                .unwrap()
                .with_variable("GIT_CONFIG_GLOBAL", "/dev/null".into())
                .unwrap()
                .with_variable("GIT_ATTR_NOSYSTEM", "1".into())
                .unwrap();
        let profile = NativeGitAdmission::direct_https(
            Arc::new(crate::HttpAdmission::new(["127.0.0.2"]).unwrap()),
            GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap(),
            TlsTrust::System,
        )
        .with_ssh_profile(ssh)
        .with_config_selection(selection);
        ShellGit::new().with_native_admission(Arc::new(profile))
    }
    pub(super) fn contacts(&self, label: &str) -> usize {
        std::fs::read_to_string(Path::new(self.get("root")).join(format!("sshd-{label}.log")))
            .unwrap()
            .matches("Connection from ")
            .count()
    }
    fn legacy(&self, wrapper: &str) -> std::process::Output {
        Command::new("/usr/bin/git")
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", self.get("home"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_SSH", self.get(wrapper))
            .env("GIT_SSH_VARIANT", "ssh")
            .args(["ls-remote", self.get("url")])
            .output()
            .unwrap()
    }
    fn local(&self, repo: &Path, args: &[&str]) -> String {
        let out = Command::new("/usr/bin/git")
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", self.get("home"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .current_dir(repo)
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
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.child.stdin.take();
        let _ = self.child.wait();
    }
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-SSH-LAW")]
fn direct_key_and_explicit_agent_acquire_exact_objects_and_use_n2_staging() {
    let f = Fixture::new();
    let temp = tempfile::tempdir().unwrap();
    for (index, address, agent) in [
        (0, f.get("url"), false),
        (1, f.get("scp"), false),
        (2, f.get("url"), true),
    ] {
        let git = f.backend("key", "known_hosts", agent);
        git.preflight_registry_auth(vibe_core::manifest::AuthKind::Ssh)
            .unwrap();
        assert_eq!(git.list_tags(address).unwrap(), ["v1.0.0", "v1.1.0"]);
        let destination = temp.path().join(format!("slot-{index}"));
        let objects = temp.path().join(format!("objects-{index}.git"));
        let revision = git
            .prepare_ref(address, "v1.0.0", None, &objects, &destination)
            .unwrap();
        assert_eq!(revision.commit, f.get("head"));
        let stage = git
            .stage_native_materialisation(
                &objects,
                &revision,
                crate::git_backend::PayloadBytes::AsConfigured,
            )
            .unwrap()
            .unwrap();
        stage.validate_before_publication().unwrap();
        assert_eq!(
            std::fs::read(stage.staged_path().join("payload")).unwrap(),
            b"SSH exact source fixture\n"
        );
        assert_eq!(
            f.local(stage.staged_path(), &["rev-parse", "HEAD"]),
            f.get("head")
        );
        assert!(!destination.exists());
        let config = std::fs::read_to_string(stage.staged_path().join(".git/config")).unwrap();
        assert!(!config.contains("ssh-identity") && !config.contains("agent.sock"));
        assert!(
            git.anonymized_for_public()
                .unwrap()
                .list_tags(address)
                .is_err()
        );
    }
    assert_eq!(f.contacts("b"), 1);
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-SSH-LAW")]
fn wrong_host_key_auth_and_excluded_origin_are_distinct_and_never_fallback() {
    let f = Fixture::new();
    assert!(matches!(
        f.backend("key", "wrong_hosts", false)
            .list_tags(f.get("url")),
        Err(GitError::SshHostKeyRejected)
    ));
    assert!(matches!(
        f.backend("wrong_key", "known_hosts", false)
            .list_tags(f.get("url")),
        Err(GitError::SshAuthenticationRejected)
    ));
    let git = f.backend("key", "known_hosts", false);
    let before = (f.contacts("a"), f.contacts("b"));
    assert!(matches!(
        git.list_tags(&f.get("url").replace("127.0.0.1", "127.0.0.2")),
        Err(GitError::ContactRefused(_))
    ));
    assert!(matches!(
        git.list_tags(&f.get("url").replace("127.0.0.1", "127.0.0.3")),
        Err(GitError::NativeContext { .. })
    ));
    assert!(matches!(
        git.list_tags(&f.get("url").replace("127.0.0.1", "127.0.0.2.")),
        Err(GitError::ContactRefused(_))
    ));
    assert_eq!((f.contacts("a"), f.contacts("b")), before);
    assert!(!format!("{:?}", git).contains("PRIVATE KEY"));
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-SSH-LAW")]
fn explicitly_selected_direct_warm_route_overrides_legacy_ssh_config_without_claiming_equivalence()
{
    let f = Fixture::new();
    for wrapper in ["alias_wrapper", "proxy_wrapper"] {
        let before = f.contacts("b");
        let out = f.legacy(wrapper);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(f.contacts("b") > before);
    }
    let marker = Path::new(f.get("proxy_marker"));
    assert!(marker.exists());
    std::fs::remove_file(marker).unwrap();
    let git = f.backend("key", "known_hosts", false);
    let before_b = f.contacts("b");
    let warm = Path::new(f.get("root")).join("work");
    f.local(&warm, &["remote", "add", "origin", f.get("url")]);
    let config = std::fs::read(warm.join(".git/config")).unwrap();
    let head = f.local(&warm, &["rev-parse", "HEAD"]);
    let temp = tempfile::tempdir().unwrap();
    for (index, version, commit) in [
        (0, "v1.0.0", f.get("head")),
        (1, "v1.1.0", f.get("updated")),
    ] {
        let objects = temp.path().join(format!("warm-{index}.git"));
        let revision = git
            .prepare_ref(f.get("url"), version, Some(&warm), &objects, &warm)
            .unwrap();
        assert_eq!(revision.commit, commit);
        let stage = git
            .stage_native_materialisation(
                &objects,
                &revision,
                crate::git_backend::PayloadBytes::AsConfigured,
            )
            .unwrap()
            .unwrap();
        stage.validate_before_publication().unwrap();
        assert_eq!(f.local(stage.staged_path(), &["rev-parse", "HEAD"]), commit);
        assert_eq!(std::fs::read(warm.join(".git/config")).unwrap(), config);
        assert_eq!(f.local(&warm, &["rev-parse", "HEAD"]), head);
    }
    assert!(!marker.exists());
    assert_eq!(f.contacts("b"), before_b);
    f.local(
        &warm,
        &[
            "config",
            &format!("url.ssh://{}@127.0.0.2/.insteadOf", f.get("user")),
            &format!("ssh://{}@127.0.0.1/", f.get("user")),
        ],
    );
    // A concrete origin replacement, independent of URL rewrite matching ports.
    f.local(
        &warm,
        &[
            "remote",
            "set-url",
            "origin",
            &f.get("url").replace("127.0.0.1", "127.0.0.2"),
        ],
    );
    let before = (f.contacts("a"), f.contacts("b"));
    assert!(
        git.prepare_ref(
            f.get("url"),
            "v1.1.0",
            Some(&warm),
            &temp.path().join("wrong.git"),
            &warm
        )
        .is_err()
    );
    assert_eq!((f.contacts("a"), f.contacts("b")), before);
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-SSH-LAW")]
fn exact_profile_does_not_accept_aliases_or_root_dot_credential_aliasing() {
    let f = Fixture::new();
    let git = f.backend("key", "known_hosts", false);
    for address in [
        "fixture-alias:repo.git",
        "ssh://127.0.0.1:22/repo.git",
        "ssh://different@127.0.0.1/repo.git",
    ] {
        assert!(git.list_tags(address).is_err());
    }
    assert_eq!(f.contacts("a"), 1); // Fixture readiness connection only.
    assert_eq!(f.contacts("b"), 1);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-SSH-LAW")]
fn direct_public_ssh_constructor_refuses_before_using_selected_credentials() {
    let f = Fixture::new();
    let temp = tempfile::tempdir().unwrap();
    let before = (f.contacts("a"), f.contacts("b"));
    let registry = crate::GitPerPackageRegistry::open_single_package(
        "public",
        f.get("url"),
        "main",
        temp.path(),
        Arc::new(f.backend("key", "known_hosts", false)),
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
    assert!(matches!(
        registry.prepare_in_place(&selected, &temp.path().join("slot")),
        Err(crate::RegistryError::Git(GitError::NativeContext { .. }))
    ));
    assert_eq!((f.contacts("a"), f.contacts("b")), before);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-SSH-LAW")]
fn interpolated_agent_paths_refuse_before_ssh_or_agent_contact() {
    use std::os::unix::net::UnixListener;
    let f = Fixture::new();
    let temp = tempfile::tempdir().unwrap();
    for name in ["${HOME}-agent.sock", "%d-agent.sock"] {
        // The literal path really is a socket, so refusal cannot be attributed
        // to a missing path or an earlier wrong-file-type setup failure.
        let path = temp.path().join(name);
        let listener = UnixListener::bind(&path).unwrap();
        listener.set_nonblocking(true).unwrap();
        let ssh = SshProfile::explicit_direct(
            "/usr/bin/ssh".into(),
            "127.0.0.1".into(),
            f.get("user").into(),
            f.port(),
            std::fs::read(f.get("known_hosts")).unwrap(),
            SshAuthentication::AgentSocket(path),
        )
        .unwrap();
        let profile = NativeGitAdmission::direct_https(
            Arc::new(crate::HttpAdmission::new(["127.0.0.2"]).unwrap()),
            GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap(),
            TlsTrust::System,
        )
        .with_ssh_profile(ssh);
        let backend = ShellGit::new().with_native_admission(Arc::new(profile));
        let before = (f.contacts("a"), f.contacts("b"));
        assert!(
            matches!(backend.list_tags(f.get("url")),Err(GitError::NativeContext{reason}) if reason.contains("interpolation"))
        );
        assert_eq!((f.contacts("a"), f.contacts("b")), before);
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-SSH-LAW")]
fn explicit_direct_warm_ssh_supersedes_captured_core_ssh_command_without_executing_it() {
    let f = Fixture::new();
    let warm = Path::new(f.get("root")).join("work");
    f.local(&warm, &["remote", "add", "origin", f.get("url")]);
    f.local(
        &warm,
        &["config", "core.sshCommand", f.get("proxy_wrapper")],
    );
    let before_b = f.contacts("b");
    f.local(&warm, &["ls-remote", "origin"]);
    assert!(f.contacts("b") > before_b);
    let marker = Path::new(f.get("proxy_marker"));
    assert!(marker.exists());
    std::fs::remove_file(marker).unwrap();
    let config = std::fs::read(warm.join(".git/config")).unwrap();
    let head = f.local(&warm, &["rev-parse", "HEAD"]);
    let git = f.backend("key", "known_hosts", false);
    let temp = tempfile::tempdir().unwrap();
    let objects = temp.path().join("objects.git");
    let before = (f.contacts("a"), f.contacts("b"));
    let revision = git
        .prepare_ref(f.get("url"), "v1.0.0", Some(&warm), &objects, &warm)
        .unwrap();
    let stage = git
        .stage_native_materialisation(
            &objects,
            &revision,
            crate::git_backend::PayloadBytes::AsConfigured,
        )
        .unwrap()
        .unwrap();
    stage.validate_before_publication().unwrap();
    assert_eq!(revision.commit, f.get("head"));
    assert!(!marker.exists());
    assert_eq!(f.contacts("b"), before.1);
    assert!(f.contacts("a") > before.0);
    assert_eq!(std::fs::read(warm.join(".git/config")).unwrap(), config);
    assert_eq!(f.local(&warm, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        std::fs::read(stage.staged_path().join(".git/config")).unwrap(),
        config
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-SSH-LAW")]
fn explicit_warm_ssh_keeps_inactive_http_selectors_without_helper_or_proxy_contact() {
    let f = Fixture::new();
    let warm = Path::new(f.get("root")).join("work");
    f.local(&warm, &["remote", "add", "origin", f.get("url")]);
    let marker = Path::new(f.get("root")).join("HTTP_HELPER_EXECUTED");
    f.local(
        &warm,
        &[
            "config",
            "credential.helper",
            &format!("!touch {}", marker.display()),
        ],
    );
    f.local(
        &warm,
        &[
            "config",
            "http.proxy",
            &format!("http://127.0.0.2:{}", f.port()),
        ],
    );
    f.local(
        &warm,
        &["config", "http.extraHeader", "X-Inactive: fixture"],
    );
    let legacy = Command::new("/usr/bin/git")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", f.get("home"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_SSH", f.get("controlled_wrapper"))
        .env("GIT_SSH_VARIANT", "ssh")
        .current_dir(&warm)
        .args(["ls-remote", "origin"])
        .output()
        .unwrap();
    assert!(legacy.status.success());
    assert!(!marker.exists());
    let before_b = f.contacts("b");
    let config = std::fs::read(warm.join(".git/config")).unwrap();
    let git = f.backend("key", "known_hosts", false);
    let tmp = tempfile::tempdir().unwrap();
    let objects = tmp.path().join("objects.git");
    let revision = git
        .prepare_ref(f.get("url"), "v1.0.0", Some(&warm), &objects, &warm)
        .unwrap();
    let stage = git
        .stage_native_materialisation(
            &objects,
            &revision,
            crate::git_backend::PayloadBytes::AsConfigured,
        )
        .unwrap()
        .unwrap();
    stage.validate_before_publication().unwrap();
    assert!(!marker.exists());
    assert_eq!(f.contacts("b"), before_b);
    assert_eq!(std::fs::read(warm.join(".git/config")).unwrap(), config);
    // Having an SSH profile does not qualify these selectors for actual HTTPS.
    let http = super::tests::Fixture::new();
    f.local(
        &warm,
        &[
            "remote",
            "set-url",
            "origin",
            &format!("{}/repo.git", http.get("a")),
        ],
    );
    let before = http.contacts("a");
    let error = git
        .prepare_ref(
            &format!("{}/repo.git", http.get("a")),
            "v1.0.0",
            Some(&warm),
            &tmp.path().join("https.git"),
            &warm,
        )
        .unwrap_err();
    assert!(matches!(error,GitError::NativeContext{reason} if reason.contains("warm transport")));
    assert_eq!(http.contacts("a"), before);
}
