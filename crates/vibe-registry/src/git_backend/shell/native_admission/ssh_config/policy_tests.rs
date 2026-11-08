//! Effective SSH algorithm/routing policy uses the same owned fixture.
#![cfg(all(test, unix))]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE");
use super::*;
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE-LAW")]
fn selected_address_family_and_source_binding_do_not_fall_back_to_any_route() {
    let f = Fixture::new();
    let (_root, home) = setup(&f);
    for (option, success, precontact) in [
        ("AddressFamily inet", true, false),
        ("AddressFamily inet6", false, false),
        ("BindAddress 127.0.0.1", false, true),
        ("BindInterface lo", false, true),
    ] {
        write_fixture(
            home.join(".ssh/config"),
            fixture_config(
                &f,
                &home,
                &format!(
                    "IdentityFile {}\nIdentityAgent none\n{option}",
                    f.get("key")
                ),
            ),
        )
        .unwrap();
        let before = f.contacts("a");
        let result = backend(&home, None).list_tags(&address(&f));
        assert_eq!(result.is_ok(), success, "{result:?}");
        if precontact {
            assert!(matches!(result, Err(GitError::NativeContext { .. })));
            assert_eq!(f.contacts("a"), before);
        }
    }
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE-LAW")]
fn captured_environment_agent_is_used_only_when_not_disabled_and_host_key_alias_is_preserved() {
    let f = Fixture::new();
    let (_root, home) = setup(&f);
    let public = home.join(".ssh/agent-key.pub");
    std::fs::copy(format!("{}.pub", f.get("key")), &public).unwrap();
    let known = std::fs::read_to_string(f.get("known_hosts")).unwrap();
    let key = known.lines().next().unwrap().split_once(' ').unwrap().1;
    write_fixture(
        home.join(".ssh/known_hosts"),
        format!("trusted-alias {key}\n[trusted-alias]:{} {key}\n", f.port()),
    )
    .unwrap();
    for (agent, success) in [
        ("", true),
        ("IdentityAgent none", false),
        ("IdentityAgent $FIXTURE_AGENT", true),
    ] {
        write_fixture(
            home.join(".ssh/config"),
            fixture_config(
                &f,
                &home,
                &format!(
                    "IdentityFile {}\nIdentitiesOnly yes\nHostKeyAlias trusted-alias\n{agent}",
                    public.display()
                ),
            ),
        )
        .unwrap();
        let config = SshConfigSelection::new("/usr/bin/ssh".into(), home.clone(), None)
            .unwrap()
            .with_environment("SSH_AUTH_SOCK", f.get("agent_socket").into())
            .unwrap()
            .with_environment("FIXTURE_AGENT", f.get("agent_socket").into())
            .unwrap()
            .capture()
            .unwrap();
        let git = ShellGit::new().with_native_admission(Arc::new(
            NativeGitAdmission::direct_https(
                Arc::new(crate::HttpAdmission::new(Vec::<String>::new()).unwrap()),
                GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into())
                    .unwrap(),
                TlsTrust::System,
            )
            .with_ssh_configuration(Arc::new(config)),
        ));
        let result = git.list_tags(&address(&f));
        assert_eq!(
            result.is_ok(),
            success,
            "agent setting {agent:?}; fixture {}; result {result:?}",
            f.get("root")
        );
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE-LAW")]
fn selected_cipher_mac_and_kex_exclusions_fail_actual_negotiation() {
    let f = Fixture::new();
    let (_root, home) = setup(&f);
    let key = format!("IdentityFile {}\nIdentityAgent none", f.get("key"));
    write_fixture(home.join(".ssh/config"), fixture_config(&f, &home, &key)).unwrap();
    assert!(backend(&home, None).list_tags(&address(&f)).is_ok());
    for policy in [
        "Ciphers aes256-cbc",
        "Ciphers aes128-ctr\nMACs hmac-md5",
        "KexAlgorithms diffie-hellman-group1-sha1",
    ] {
        write_fixture(
            home.join(".ssh/config"),
            fixture_config(&f, &home, &format!("{key}\n{policy}")),
        )
        .unwrap();
        let before = f.contacts("a");
        let result = backend(&home, None).list_tags(&address(&f));
        assert!(
            matches!(result, Err(GitError::CommandFailed { .. })),
            "policy={policy}, result={result:?}"
        );
        assert!(f.contacts("a") > before);
    }
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE-LAW")]
fn selected_minimum_rsa_size_restricts_existing_identity() {
    use std::process::Command;
    let f = Fixture::new();
    let (root, home) = setup(&f);
    let key = root.path().join("rsa");
    assert!(
        Command::new("/usr/bin/ssh-keygen")
            .args(["-q", "-t", "rsa", "-b", "2048", "-N", "", "-f"])
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
    for (minimum, success) in [(2048, true), (4096, false)] {
        write_fixture(home.join(".ssh/config"),fixture_config(&f,&home,&format!("IdentityFile {}\nIdentityAgent none\nIdentitiesOnly yes\nRequiredRSASize {minimum}",key.display()))).unwrap();
        let before = f.contacts("a");
        let result = backend(&home, None).list_tags(&address(&f));
        assert_eq!(result.is_ok(), success, "{result:?}");
        assert!(f.contacts("a") > before);
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE-LAW")]
fn unsafe_original_config_permissions_are_not_repaired_by_freezing() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    let (_root, home) = setup(&f);
    let path = home.join(".ssh/config");
    write_fixture(
        &path,
        fixture_config(
            &f,
            &home,
            &format!("IdentityFile {}\nIdentityAgent none", f.get("key")),
        ),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o666)).unwrap();
    let before = f.contacts("a");
    let selected = SshConfigSelection::new("/usr/bin/ssh".into(), home, None).unwrap();
    assert!(
        matches!(selected.capture(),Err(GitError::NativeContext{reason}) if reason.contains("permissions"))
    );
    assert_eq!(f.contacts("a"), before);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE-LAW")]
fn excluded_effective_alias_precedes_missing_credentials_and_trust_in_both_paths() {
    let f = Fixture::new();
    let (root, home) = setup(&f);
    write_fixture(home.join(".ssh/config"),format!("Host excluded-alias\nHostName 127.0.0.2\nUser {}\nPort {}\nIdentityFile {}\nIdentityAgent none\nUserKnownHostsFile none\nGlobalKnownHostsFile none\n",f.get("user"),f.port(),root.path().join("missing-key").display())).unwrap();
    let configuration = Arc::new(
        SshConfigSelection::new("/usr/bin/ssh".into(), home, None)
            .unwrap()
            .capture()
            .unwrap(),
    );
    let binding = Arc::new(crate::HttpAdmission::new(["127.0.0.2"]).unwrap());
    let address = address(&f).replace("source-alias", "excluded-alias");
    let before = f.contacts("b");
    assert!(matches!(
        configuration.preflight_address(&address, &binding),
        Err(GitError::ContactRefused(_))
    ));
    let git = ShellGit::new().with_native_admission(Arc::new(
        NativeGitAdmission::direct_https(
            binding,
            GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap(),
            TlsTrust::System,
        )
        .with_ssh_configuration(configuration),
    ));
    assert!(matches!(
        git.list_tags(&address),
        Err(GitError::ContactRefused(_))
    ));
    assert_eq!(f.contacts("b"), before);
}
