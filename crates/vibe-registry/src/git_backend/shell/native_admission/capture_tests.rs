//! What the machine-wide configuration scope means to a controlled child
//! ([`super::capture`]). Split out of that module to hold the file-length
//! budget.
use super::*;

fn context(entries: &[(&str, &str, bool)]) -> CapturedContext {
    CapturedContext {
        cwd: PathBuf::from("/"),
        warm: None,
        layout: None,
        config: Vec::new(),
        entries: entries
            .iter()
            .map(|(k, v, _)| ((*k).into(), (*v).into()))
            .collect(),
        machine: entries.iter().map(|(_, _, machine)| *machine).collect(),
        files: BTreeMap::new(),
        system_attributes: Vec::new(),
        global_attributes: Vec::new(),
        info_attributes: Vec::new(),
        effective_origin: None,
        recorded_origin: None,
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-WINDOWS-LAW")]
fn windows_installation_defaults_do_not_refuse_but_user_transport_settings_do() {
    assert!(machine_transport_exempt(b"system", true));
    assert!(!machine_transport_exempt(b"system", false));
    assert!(!machine_transport_exempt(b"global", true));
    // Git for Windows' own system configuration.
    let installation = [
        ("credential.helper", "manager", true),
        ("http.sslbackend", "schannel", true),
        (
            "http.sslcainfo",
            "C:/Program Files/Git/mingw64/etc/ssl/certs/ca-bundle.crt",
            true,
        ),
        ("core.autocrlf", "true", true),
    ];
    context(&installation)
        .validate_warm_transport(false, false)
        .unwrap();
    // The same settings chosen by the user still refuse.
    for user in [
        ("credential.helper", "store", false),
        ("http.proxy", "http://ambient:3128", false),
        ("core.sshcommand", "ssh -i key", false),
    ] {
        assert!(
            context(&[installation[0], user])
                .validate_warm_transport(false, false)
                .is_err(),
            "{user:?}"
        );
    }
}
