//! A registry on this machine, admitted on a workstation that carries
//! ordinary Git transport configuration for somewhere else.
//!
//! The corporate deployment this exists for is a mirrored `monorepo`
//! registry on disk and a closed network. The same workstation has a
//! `~/.gitconfig` with `credential.<url>.*` entries for the company's own
//! forge, and often an `http.proxy` — every machine behind an inspecting
//! proxy does. None of that configuration can reach a repository Git reads
//! through the filesystem, and none of it may refuse one.
#![cfg(all(test, unix))]
use super::*;

/// An invocation that selected no SSH lane: it refuses on demand, typed.
/// Its presence is what turns the transport-context validation on at all.
#[derive(Debug)]
struct NoSshLane;
impl SshConfigurationProvider for NoSshLane {
    fn configuration(&self) -> Result<Arc<SshConfiguration>, GitError> {
        Err(GitError::NativeContext {
            reason: "no SSH configuration was selected",
        })
    }
}

/// A profile whose invocation home carries transport settings for hosts
/// that have nothing to do with the registry being admitted.
fn workstation(home: &Path) -> NativeGitAdmission {
    std::fs::write(
        home.join(".gitconfig"),
        "[credential \"https://forge.invalid\"]\n\tprovider = generic\n\
         [credential \"https://bitbucket.invalid\"]\n\thelper = manager\n\
         [http]\n\tproxy = http://inspecting.invalid:3128\n",
    )
    .unwrap();
    let selection = NativeConfigSelection::new(home.into(), home.into())
        .unwrap()
        .with_variable("GIT_CONFIG_NOSYSTEM", "1".into())
        .unwrap()
        .with_variable("GIT_ATTR_NOSYSTEM", "1".into())
        .unwrap();
    NativeGitAdmission::direct_https(
        Arc::new(crate::HttpAdmission::new([] as [&str; 0]).unwrap()),
        GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap(),
        TlsTrust::System,
    )
    .with_config_selection(selection)
    .with_ssh_configuration_provider(Arc::new(NoSshLane))
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-PREPARATION")]
fn unrelated_credential_and_proxy_settings_do_not_refuse_a_registry_on_disk() {
    let home = tempfile::tempdir().unwrap();
    let profile = workstation(home.path());
    let mono = home.path().join("mono.git");
    for address in [
        mono.display().to_string(),
        format!("file://{}", mono.display()),
    ] {
        let remote = profile
            .capture_for_request(&address, false, None)
            .unwrap_or_else(|error| panic!("{address}: {error}"));
        assert!(matches!(remote, AdmittedRemote::Local { .. }), "{address}");
        assert_eq!(remote.address(), address);
    }
    // The refusal itself is intact: an address that does contact a host
    // still requires that same context to be separately admitted.
    assert!(matches!(
        profile.capture_for_request("https://forge.invalid/vibe/mono.git", false, None),
        Err(GitError::NativeContext { reason })
            if reason.starts_with("warm transport settings require")
    ));
}
