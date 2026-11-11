//! The lowered-value decisions of one captured OpenSSH context, on
//! synthetic input. Every Windows decision is a function of a `windows`
//! parameter, so these run on every platform.
#![cfg(test)]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE");
use super::*;

const MODERN: &str = "addressfamily any\npubkeyacceptedalgorithms ssh-ed25519\nhostkeyalgorithms ssh-ed25519\nciphers aes128-ctr\nmacs hmac-sha2-256\nkexalgorithms curve25519-sha256\nrequiredrsasize 2048\n";

#[test]
fn modern_client_options_replay_every_reported_policy() {
    let options = client_options(&fields(MODERN).unwrap()).unwrap();
    let keys: Vec<_> = options.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(
        keys,
        [
            "addressfamily",
            "pubkeyacceptedalgorithms",
            "hostkeyalgorithms",
            "ciphers",
            "macs",
            "kexalgorithms",
            "requiredrsasize"
        ]
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-WINDOWS-LAW")]
fn windows_10_openssh_options_keep_their_own_keywords() {
    // OpenSSH 8.1 (Windows 10): the pre-8.5 keyword, no RequiredRSASize.
    let old = MODERN
        .replace("pubkeyacceptedalgorithms", "pubkeyacceptedkeytypes")
        .replace("requiredrsasize 2048\n", "");
    let options = client_options(&fields(&old).unwrap()).unwrap();
    assert!(options.iter().any(|(k, _)| k == "pubkeyacceptedkeytypes"));
    assert!(!options.iter().any(|(k, _)| k == "pubkeyacceptedalgorithms"));
    assert!(!options.iter().any(|(k, _)| k == "requiredrsasize"));
    // A reported but weak or malformed size is still refused.
    for weak in ["requiredrsasize 512\n", "requiredrsasize many\n"] {
        let output = format!("{old}{weak}");
        assert!(client_options(&fields(&output).unwrap()).is_err());
    }
    // A missing algorithm policy is still refused.
    let missing = MODERN.replace("ciphers aes128-ctr\n", "");
    assert!(client_options(&fields(&missing).unwrap()).is_err());
}

/// A synthetic Windows OpenSSH selection. Nothing here touches the
/// filesystem: only the lowering of reported values is under test, so
/// the Windows decisions run on every platform.
fn win32_selection(program_data: Option<&str>) -> SshConfigSelection {
    SshConfigSelection {
        executable: PathBuf::from(r"C:\Windows\System32\OpenSSH\ssh.exe"),
        home: PathBuf::from(r"C:\Users\dev"),
        user: PathBuf::from(r"C:\Users\dev\.ssh\config"),
        system: Some(PathBuf::from(r"C:\ProgramData\ssh\ssh_config")),
        environment: program_data
            .map(|value| BTreeMap::from([("ProgramData".to_owned(), value.to_owned())]))
            .unwrap_or_default(),
        acquisition_root: None,
        default_agent: None,
    }
}

/// `ssh -G` as Win32-OpenSSH prints it, with `trust` substituted for the
/// two known-hosts records — mixed separators and all.
fn win32_output(trust: &str) -> String {
    format!(
        "hostname 10.1.2.3\nuser dev\nport 7998\n{MODERN}identityfile C:\\Users\\dev/.ssh/id_ed25519\npubkeyauthentication true\n{trust}"
    )
}

const WIN32_TRUST: &str = "userknownhostsfile C:\\Users\\dev/.ssh/known_hosts C:\\Users\\dev/.ssh/known_hosts2\nglobalknownhostsfile __PROGRAMDATA__\\ssh/ssh_known_hosts __PROGRAMDATA__\\ssh/ssh_known_hosts2\n";

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-WINDOWS-LAW")]
fn win32_openssh_program_data_trust_defaults_resolve_against_the_captured_environment() {
    let selected = win32_selection(Some(r"C:\ProgramData"));
    let output = win32_output(WIN32_TRUST);
    let fields = fields(&output).unwrap();
    // The rest of the reported context is admitted unchanged.
    client_options(&fields).unwrap();
    endpoint(&output).unwrap();
    let files = trust_files(&fields, &selected, None, true).unwrap();
    assert_eq!(
        files,
        [
            (
                "userknownhostsfile",
                PathBuf::from(r"C:\Users\dev/.ssh/known_hosts")
            ),
            (
                "userknownhostsfile",
                PathBuf::from(r"C:\Users\dev/.ssh/known_hosts2")
            ),
            (
                "globalknownhostsfile",
                PathBuf::from(r"C:\ProgramData\ssh\ssh_known_hosts")
            ),
            (
                "globalknownhostsfile",
                PathBuf::from(r"C:\ProgramData\ssh\ssh_known_hosts2")
            ),
        ]
    );
    // The same literal is expanded wherever a path may carry it.
    assert_eq!(
        path(
            r"__PROGRAMDATA__\ssh\id_ed25519",
            &selected,
            None,
            true,
            "identityfile"
        )
        .unwrap(),
        PathBuf::from(r"C:\ProgramData\ssh\id_ed25519")
    );
    // Without a captured ProgramData the global default is unresolvable
    // and selects no trust; the caller's own files still do.
    let files = trust_files(&fields, &win32_selection(None), None, true).unwrap();
    assert_eq!(files.len(), 2);
    assert!(files.iter().all(|(kind, _)| *kind == "userknownhostsfile"));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-WINDOWS-LAW")]
fn msys_rootless_global_trust_defaults_select_nothing_rather_than_refusing() {
    // The client Git for Windows bundles reports its compiled-in POSIX
    // defaults, which carry no drive. The MSYS mount table that would
    // resolve them is not captured, so they select no trust.
    let selected = win32_selection(Some(r"C:\ProgramData"));
    let output = win32_output(
        "userknownhostsfile C:/Users/dev/.ssh/known_hosts\nglobalknownhostsfile /etc/ssh/ssh_known_hosts /etc/ssh/ssh_known_hosts2\n",
    );
    let files = trust_files(&fields(&output).unwrap(), &selected, None, true).unwrap();
    assert_eq!(
        files,
        [(
            "userknownhostsfile",
            PathBuf::from("C:/Users/dev/.ssh/known_hosts")
        )]
    );
    // The same reported value on a POSIX client is an ordinary absolute
    // path and is kept.
    let posix = win32_output(
        "userknownhostsfile /home/dev/.ssh/known_hosts\nglobalknownhostsfile /etc/ssh/ssh_known_hosts /etc/ssh/ssh_known_hosts2\n",
    );
    let files = trust_files(&fields(&posix).unwrap(), &selected, None, false).unwrap();
    assert_eq!(
        files.iter().map(|(kind, _)| *kind).collect::<Vec<_>>(),
        [
            "userknownhostsfile",
            "globalknownhostsfile",
            "globalknownhostsfile"
        ]
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-WINDOWS-LAW")]
fn a_relative_user_trust_path_is_still_refused_and_the_refusal_names_its_origin() {
    let selected = win32_selection(Some(r"C:\ProgramData"));
    let output = win32_output("userknownhostsfile known_hosts\nglobalknownhostsfile none\n");
    let error = trust_files(&fields(&output).unwrap(), &selected, None, true).unwrap_err();
    let message = error.to_string();
    for named in [
        r"C:\Windows\System32\OpenSSH\ssh.exe",
        "userknownhostsfile",
        "known_hosts",
        "fix:",
        "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-PREPARATION",
    ] {
        assert!(message.contains(named), "{named} missing from {message}");
    }
    // An unexpandable value is reported the same way, not swallowed.
    let error = path("~other/.ssh/id", &selected, None, true, "identityfile").unwrap_err();
    let message = error.to_string();
    assert!(message.contains("identityfile"), "{message}");
    assert!(message.contains("~other/.ssh/id"), "{message}");
    assert!(message.contains("fix:"), "{message}");
}
