//! Ordinary config and real fixture authentication; no operator configuration.
#![cfg(all(test, unix))]
use super::super::ssh_tests::Fixture;
use super::*;
use crate::{GitBackend, ShellGit};
#[cfg(test)]
fn write_fixture(path: impl AsRef<Path>, bytes: impl AsRef<[u8]>) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path.as_ref(), bytes)?;
    std::fs::set_permissions(path.as_ref(), std::fs::Permissions::from_mode(0o600))
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE-LAW")]
fn ordinary_custom_identity_and_hostname_capture_authenticates() {
    let f = Fixture::new();
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    std::fs::create_dir_all(home.join(".ssh")).unwrap();
    let config = format!(
        "Host source-alias\n HostName 127.0.0.1\n User {}\n Port {}\n IdentityFile = {}\n IdentityAgent none\n IdentitiesOnly yes\n UserKnownHostsFile = {}\n GlobalKnownHostsFile none\n",
        f.get("user"),
        f.port(),
        f.get("key"),
        f.get("known_hosts")
    );
    write_fixture(home.join(".ssh/config"), config).unwrap();
    let selected = SshConfigSelection::new("/usr/bin/ssh".into(), home, None)
        .unwrap()
        .capture()
        .unwrap();
    let profile = NativeGitAdmission::direct_https(
        Arc::new(crate::HttpAdmission::new(["127.0.0.2"]).unwrap()),
        GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap(),
        TlsTrust::System,
    )
    .with_ssh_configuration(Arc::new(selected));
    let git = ShellGit::new().with_native_admission(Arc::new(profile));
    let path = Url::parse(f.get("url")).unwrap().path().to_owned();
    let url = format!("ssh://source-alias{path}");
    assert_eq!(git.list_tags(&url).unwrap(), ["v1.0.0", "v1.1.0"]);
    assert_eq!(f.contacts("b"), 1);
}

#[cfg(test)]
fn fixture_config(f: &Fixture, home: &Path, auth: &str) -> String {
    format!(
        "Host source-alias\n HostName 127.0.0.1\n User {}\n Port {}\n UserKnownHostsFile {}/.ssh/known_hosts\n GlobalKnownHostsFile none\n{auth}\n",
        f.get("user"),
        f.port(),
        home.display()
    )
}
#[cfg(test)]
fn setup(f: &Fixture) -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    std::fs::create_dir_all(home.join(".ssh")).unwrap();
    std::fs::copy(f.get("known_hosts"), home.join(".ssh/known_hosts")).unwrap();
    (root, home)
}
#[cfg(test)]
fn backend(home: &Path, system: Option<PathBuf>) -> ShellGit {
    let config = SshConfigSelection::new("/usr/bin/ssh".into(), home.into(), system)
        .unwrap()
        .capture()
        .unwrap();
    ShellGit::new().with_native_admission(Arc::new(
        NativeGitAdmission::direct_https(
            Arc::new(crate::HttpAdmission::new(["127.0.0.2"]).unwrap()),
            GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap(),
            TlsTrust::System,
        )
        .with_ssh_configuration(Arc::new(config)),
    ))
}
#[cfg(test)]
fn address(f: &Fixture) -> String {
    format!(
        "ssh://source-alias{}",
        Url::parse(f.get("url")).unwrap().path()
    )
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE-LAW")]
fn default_identity_and_selected_home_trust_authenticate_without_key_setup_api() {
    let f = Fixture::new();
    let (_root, home) = setup(&f);
    std::fs::copy(f.get("key"), home.join(".ssh/id_ed25519")).unwrap();
    write_fixture(home.join(".ssh/config"),format!("Host source-alias\nHostName 127.0.0.1\nUser {}\nPort {}\nIdentityAgent none\nGlobalKnownHostsFile none\n",f.get("user"),f.port())).unwrap();
    assert_eq!(
        backend(&home, None).list_tags(&address(&f)).unwrap(),
        ["v1.0.0", "v1.1.0"]
    );
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE-LAW")]
fn ordered_includes_scalar_precedence_and_url_overrides_match_safe_openssh() {
    let f = Fixture::new();
    let (root, home) = setup(&f);
    let conf = home.join(".ssh/conf.d");
    std::fs::create_dir(&conf).unwrap();
    write_fixture(
        conf.join("10.conf"),
        format!(
            "HostName 127.0.0.1\nUser {}\nPort {}\nIdentityFile {}\n",
            f.get("user"),
            f.port(),
            f.get("wrong_key")
        ),
    )
    .unwrap();
    write_fixture(
        conf.join("20.conf"),
        format!(
            "User ignored-later\nPort 9\nIdentityFile {}\n",
            f.get("key")
        ),
    )
    .unwrap();
    write_fixture(home.join(".ssh/config"),"Host source-alias !excluded-alias\nInclude conf.d/*.conf\nIdentityAgent none\nIdentitiesOnly yes\nHost not-this-host\nPort 8\n").unwrap();
    let system = root.path().join("system.config");
    write_fixture(&system, "GlobalKnownHostsFile none\nUser ignored-system\n").unwrap();
    let config = Arc::new(
        SshConfigSelection::new("/usr/bin/ssh".into(), home.clone(), Some(system.clone()))
            .unwrap()
            .capture()
            .unwrap(),
    );
    let authority = Authority::parse(&address(&f)).unwrap();
    let output = inputs::evaluate(&config.selected, &config.frozen, &authority).unwrap();
    // Independent safe reference preserves the original selected-home include
    // meaning with an absolute fixture Include; no user's config is evaluated.
    let reference_user = root.path().join("reference-user");
    write_fixture(
        &reference_user,
        std::fs::read_to_string(home.join(".ssh/config"))
            .unwrap()
            .replace(
                "Include conf.d/*.conf",
                &format!("Include {}/*.conf", conf.display()),
            ),
    )
    .unwrap();
    let reference = root.path().join("reference-config");
    write_fixture(
        &reference,
        format!(
            "Include {}\nHost *\nInclude {}\n",
            reference_user.display(),
            system.display()
        ),
    )
    .unwrap();
    let baseline = std::process::Command::new("/usr/bin/ssh")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", &home)
        .args(["-G", "-F"])
        .arg(&reference)
        .arg("source-alias")
        .output()
        .unwrap();
    assert!(baseline.status.success());
    let selected_lines = |text: &str| {
        text.lines()
            .filter(|line| {
                ["hostname ", "user ", "port ", "identityfile "]
                    .iter()
                    .any(|key| line.starts_with(key))
            })
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        selected_lines(&output),
        selected_lines(&String::from_utf8(baseline.stdout).unwrap())
    );

    assert!(output.contains(&format!("user {}\n", f.get("user"))));
    assert!(output.find(f.get("wrong_key")).unwrap() < output.find(f.get("key")).unwrap());
    let git = ShellGit::new().with_native_admission(Arc::new(
        NativeGitAdmission::direct_https(
            Arc::new(crate::HttpAdmission::new(Vec::<String>::new()).unwrap()),
            GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap(),
            TlsTrust::System,
        )
        .with_ssh_configuration(config),
    ));
    assert!(git.list_tags(&address(&f)).is_ok());
    let bad_port = address(&f).replace("source-alias", "source-alias:9");
    let p = config_profile(&home, Some(system), &bad_port);
    assert_eq!(p.port, 9);
}
#[cfg(test)]
fn config_profile(home: &Path, system: Option<PathBuf>, url: &str) -> Observed {
    let config = Arc::new(
        SshConfigSelection::new("/usr/bin/ssh".into(), home.into(), system)
            .unwrap()
            .capture()
            .unwrap(),
    );
    let authority = Authority::parse(url).unwrap();
    let output = inputs::evaluate(&config.selected, &config.frozen, &authority).unwrap();
    Observed {
        port: output
            .lines()
            .find_map(|s| s.strip_prefix("port "))
            .unwrap()
            .parse()
            .unwrap(),
    }
}
#[cfg(test)]
struct Observed {
    port: u16,
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE-LAW")]
fn identities_only_keeps_agent_restricted_to_selected_public_identity() {
    let f = Fixture::new();
    let (_root, home) = setup(&f);
    let public = home.join(".ssh/selected.pub");
    for (key, only, success) in [
        ("key", "yes", true),
        ("wrong_key", "yes", false),
        ("wrong_key", "no", true),
    ] {
        std::fs::copy(format!("{}.pub", f.get(key)), &public).unwrap();
        write_fixture(
            home.join(".ssh/config"),
            fixture_config(
                &f,
                &home,
                &format!(
                    "IdentityFile {}\nIdentityAgent {}\nIdentitiesOnly {only}",
                    public.display(),
                    f.get("agent_socket")
                ),
            ),
        )
        .unwrap();
        let result = backend(&home, None).list_tags(&address(&f));
        assert_eq!(result.is_ok(), success, "{result:?}");
    }
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE-LAW")]
fn active_commands_and_disabled_publickey_refuse_before_contact_but_inactive_proxy_is_harmless() {
    let f = Fixture::new();
    let (root, home) = setup(&f);
    let marker = root.path().join("EXECUTED");
    for setting in [
        "PubkeyAuthentication no".into(),
        "PreferredAuthentications password".into(),
        format!("ProxyCommand touch {}", marker.display()),
        format!("KnownHostsCommand touch {}", marker.display()),
    ] {
        write_fixture(
            home.join(".ssh/config"),
            fixture_config(
                &f,
                &home,
                &format!(
                    "IdentityFile {}\nIdentityAgent none\n{setting}",
                    f.get("key")
                ),
            ),
        )
        .unwrap();
        let before = f.contacts("a");
        assert!(matches!(
            backend(&home, None).list_tags(&address(&f)),
            Err(GitError::NativeContext { .. })
        ));
        assert_eq!(f.contacts("a"), before);
        assert!(!marker.exists());
    }
    write_fixture(
        home.join(".ssh/config"),
        format!(
            "{}\nHost other\nProxyCommand touch {}\n",
            fixture_config(
                &f,
                &home,
                &format!("IdentityFile {}\nIdentityAgent none", f.get("key"))
            ),
            marker.display()
        ),
    )
    .unwrap();
    assert!(backend(&home, None).list_tags(&address(&f)).is_ok());
    assert!(!marker.exists());
    for body in [
        format!("Match exec \"touch {}\"\n", marker.display()),
        "CanonicalizeHostname yes\n".into(),
        "Include %h.conf\n".into(),
    ] {
        write_fixture(home.join(".ssh/config"), body).unwrap();
        assert!(
            SshConfigSelection::new("/usr/bin/ssh".into(), home.clone(), None)
                .unwrap()
                .capture()
                .is_err()
        );
        assert!(!marker.exists());
    }
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE-LAW")]
fn selected_algorithm_restrictions_are_replayed_not_replaced_by_defaults() {
    let f = Fixture::new();
    let (_root, home) = setup(&f);
    for key in ["HostKeyAlgorithms", "PubkeyAcceptedAlgorithms"] {
        write_fixture(
            home.join(".ssh/config"),
            fixture_config(
                &f,
                &home,
                &format!(
                    "IdentityFile {}\nIdentityAgent none\n{key} rsa-sha2-512",
                    f.get("key")
                ),
            ),
        )
        .unwrap();
        let before = f.contacts("a");
        assert!(
            matches!(
                backend(&home, None).list_tags(&address(&f)),
                Err(GitError::SshAuthenticationRejected | GitError::CommandFailed { .. })
            ),
            "{key} must reach the actual restricted SSH negotiation"
        );
        assert!(f.contacts("a") > before);
    }
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE-LAW")]
fn config_fifo_oversize_and_recheck_refuse_without_unbounded_read() {
    let f = Fixture::new();
    let (root, home) = setup(&f);
    let file = home.join(".ssh/config");
    let make = || SshConfigSelection::new("/usr/bin/ssh".into(), home.clone(), None).unwrap();
    let giant = std::fs::File::create(&file).unwrap();
    giant.set_len(1024 * 1024 + 1).unwrap();
    assert!(make().capture().is_err());
    drop(giant);
    std::fs::remove_file(&file).unwrap();
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&file)
            .status()
            .unwrap()
            .success()
    );
    assert!(make().capture().is_err());
    std::fs::remove_file(&file).unwrap();
    write_fixture(
        &file,
        fixture_config(
            &f,
            &home,
            &format!("IdentityFile {}\nIdentityAgent none", f.get("key")),
        ),
    )
    .unwrap();
    let git = backend(&home, None);
    std::fs::remove_file(&file).unwrap();
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&file)
            .status()
            .unwrap()
            .success()
    );
    let before = f.contacts("a");
    assert!(git.list_tags(&address(&f)).is_err());
    assert_eq!(f.contacts("a"), before);
    let _ = root;
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE-LAW")]
fn encrypted_identity_uses_permitted_loaded_agent_and_never_prompts_without_it() {
    use std::os::unix::fs::PermissionsExt;
    use std::process::{Command, Stdio};
    let f = Fixture::new();
    let (root, home) = setup(&f);
    let key = root.path().join("encrypted");
    assert!(
        Command::new("/usr/bin/ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "synthetic-passphrase", "-f"])
            .arg(&key)
            .status()
            .unwrap()
            .success()
    );
    let public = std::fs::read_to_string(key.with_extension("pub")).unwrap();
    write_fixture(
        Path::new(f.get("root")).join("authorized_keys"),
        format!("restrict {public}"),
    )
    .unwrap();
    let ask = root.path().join("askpass");
    write_fixture(&ask, "#!/bin/sh\nprintf '%s\\n' synthetic-passphrase\n").unwrap();
    std::fs::set_permissions(&ask, std::fs::Permissions::from_mode(0o700)).unwrap();
    let add = Command::new("/usr/bin/ssh-add")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", &home)
        .env("SSH_AUTH_SOCK", f.get("agent_socket"))
        .env("SSH_ASKPASS", &ask)
        .env("SSH_ASKPASS_REQUIRE", "force")
        .env("DISPLAY", "fixture")
        .arg(&key)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(add.status.success());
    for (agent, success) in [(f.get("agent_socket"), true), ("none", false)] {
        write_fixture(
            home.join(".ssh/config"),
            fixture_config(
                &f,
                &home,
                &format!(
                    "IdentityFile {}\nIdentityAgent {agent}\nIdentitiesOnly yes",
                    key.display()
                ),
            ),
        )
        .unwrap();
        let result = backend(&home, None).list_tags(&address(&f));
        assert_eq!(result.is_ok(), success);
        if let Err(error) = result {
            assert!(matches!(error, GitError::SshAuthenticationRejected));
            assert!(!error.to_string().contains("synthetic-passphrase"));
        }
    }
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE-LAW")]
fn immutable_config_lowers_newly_selected_addresses_and_refuses_effective_denied_host() {
    let f = Fixture::new();
    let other = Fixture::new();
    let (_root, home) = setup(&f);
    let first = fixture_config(
        &f,
        &home,
        &format!("IdentityFile {}\nIdentityAgent none", f.get("key")),
    );
    let second = format!(
        "Host later-alias\nHostName 127.0.0.1\nUser {}\nPort {}\nIdentityFile {}\nIdentityAgent none\nUserKnownHostsFile {}\nGlobalKnownHostsFile none\n",
        other.get("user"),
        other.port(),
        other.get("key"),
        other.get("known_hosts")
    );
    write_fixture(home.join(".ssh/config"),format!("{first}\n{second}\nHost excluded-alias\nHostName 127.0.0.2\nUser {}\nPort {}\nIdentityFile {}\nUserKnownHostsFile {}\nGlobalKnownHostsFile none\n",f.get("user"),f.port(),f.get("key"),f.get("known_hosts"))).unwrap();
    let git = backend(&home, None);
    assert!(git.list_tags(&address(&f)).is_ok());
    let later = format!(
        "ssh://later-alias{}",
        Url::parse(other.get("url")).unwrap().path()
    );
    assert!(git.list_tags(&later).is_ok());
    let denied = address(&f).replace("source-alias", "excluded-alias");
    let before = f.contacts("b");
    assert!(matches!(
        git.list_tags(&denied),
        Err(GitError::ContactRefused(_))
    ));
    assert_eq!(f.contacts("b"), before);
    let objects = Path::new(f.get("root")).join("configured.git");
    let revision = git
        .prepare_ref(
            &address(&f),
            "v1.0.0",
            None,
            &objects,
            &objects.with_extension("slot"),
        )
        .unwrap();
    assert_eq!(revision.commit, f.get("head"));
    assert_eq!(git.recorded_origin_url(&objects).unwrap(), address(&f));
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE-LAW")]
fn identity_and_include_addition_witnesses_refuse_changed_inputs() {
    let f = Fixture::new();
    let (_root, home) = setup(&f);
    let includes = home.join(".ssh/parts");
    std::fs::create_dir(&includes).unwrap();
    write_fixture(
        home.join(".ssh/config"),
        format!(
            "Host source-alias\nInclude parts/*.conf\n{}",
            fixture_config(
                &f,
                &home,
                &format!("IdentityFile {}\nIdentityAgent none", f.get("key"))
            )
        ),
    )
    .unwrap();
    let git = backend(&home, None);
    write_fixture(includes.join("first.conf"), "Port 9\n").unwrap();
    let before = f.contacts("a");
    assert!(git.list_tags(&address(&f)).is_err());
    assert_eq!(f.contacts("a"), before);
    let owner = Arc::new(
        SshConfigSelection::new("/usr/bin/ssh".into(), home, None)
            .unwrap()
            .capture()
            .unwrap(),
    );
    let profile = owner
        .profile(
            &address(&f),
            &crate::HttpAdmission::new(Vec::<String>::new()).unwrap(),
        )
        .unwrap();
    // Retained profile validation happens again at the actual command boundary.
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(f.get("key"), std::fs::Permissions::from_mode(0o640)).unwrap();
    let selected = ShellGit::new().with_native_admission(Arc::new(
        NativeGitAdmission::direct_https(
            Arc::new(crate::HttpAdmission::new(Vec::<String>::new()).unwrap()),
            GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap(),
            TlsTrust::System,
        )
        .with_ssh_profile(profile),
    ));
    assert!(
        matches!(selected.list_tags(&address(&f)),Err(GitError::NativeContext{reason}) if reason.contains("identity reference changed"))
    );
    assert_eq!(f.contacts("a"), before);
}

#[path = "policy_tests.rs"]
mod policy_tests;
