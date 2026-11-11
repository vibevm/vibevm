//! Platform facts of a controlled Git child: the search path and the few OS
//! variables a Windows process needs to start, and the text form of a path
//! handed to a POSIX shell or an OpenSSH configuration. Each decision is a
//! pure function of its inputs (`windows` is a parameter, not a `cfg`), so
//! the Windows layout is tested on every platform.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-WINDOWS");
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The OS variables a controlled child keeps on Windows. Without
/// `SystemRoot` Winsock and the TLS providers cannot initialise; `TEMP` /
/// `TMP` are where Git and OpenSSH put their scratch. None of them selects
/// Git or SSH configuration, a helper, a proxy or a credential.
pub const WINDOWS_PLATFORM_VARIABLES: &[&str] = &[
    "SystemRoot",
    "windir",
    "SystemDrive",
    "TEMP",
    "TMP",
    "PATHEXT",
];

/// The platform variables of `environment` a child keeps: on Windows the
/// [`WINDOWS_PLATFORM_VARIABLES`] (names compared without case, as Windows
/// does); elsewhere none.
pub fn platform_variables<'a>(
    environment: impl IntoIterator<Item = (&'a std::ffi::OsStr, &'a std::ffi::OsStr)>,
    windows: bool,
) -> Vec<(OsString, OsString)> {
    if !windows {
        return Vec::new();
    }
    let mut kept: Vec<(OsString, OsString)> = Vec::new();
    for (key, value) in environment {
        let Some(name) = key.to_str() else { continue };
        if let Some(canonical) = WINDOWS_PLATFORM_VARIABLES
            .iter()
            .find(|known| known.eq_ignore_ascii_case(name))
            && !kept.iter().any(|(k, _)| k == canonical)
        {
            kept.push((OsString::from(canonical), value.to_owned()));
        }
    }
    kept
}

/// The directories a controlled child may search for programs, in order.
/// Elsewhere only the selected helper directory. On Windows a Git for
/// Windows installation (`<prefix>/mingw64/libexec/git-core`) also needs its
/// own `mingw64/bin` (the DLLs `git-remote-https` loads) and `usr/bin` (the
/// `sh` Git runs `GIT_SSH_COMMAND` with); both belong to the same trusted
/// installation, and only those that exist are kept.
pub(super) fn runtime_directories(
    helpers: &Path,
    windows: bool,
    exists: impl Fn(&Path) -> bool,
) -> Vec<PathBuf> {
    let mut directories = vec![helpers.to_path_buf()];
    if windows {
        let mingw = helpers.parent().and_then(Path::parent);
        let candidates = [
            mingw.map(|root| root.join("bin")),
            mingw
                .and_then(Path::parent)
                .map(|prefix| prefix.join("usr").join("bin")),
        ];
        for candidate in candidates.into_iter().flatten() {
            if exists(&candidate) && !directories.contains(&candidate) {
                directories.push(candidate);
            }
        }
    }
    directories
}

/// The helper `git-remote-https` in `directory`, by either spelling: Git
/// for Windows names it `git-remote-https.exe`.
pub(super) fn has_https_helper(helpers: &Path) -> bool {
    ["git-remote-https", "git-remote-https.exe"]
        .iter()
        .any(|name| helpers.join(name).is_file())
}

/// `path` without a Windows verbatim prefix: `\\?\C:\x` is `C:\x` and
/// `\\?\UNC\server\share` is `\\server\share`. `std::fs::canonicalize`
/// returns the verbatim form on Windows, which a POSIX shell, an OpenSSH
/// configuration and the Git for Windows launcher do not all accept.
/// Another path is returned unchanged.
pub fn simplified_path(path: PathBuf) -> PathBuf {
    let Some(text) = path.to_str() else {
        return path;
    };
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{rest}"));
    }
    if let Some(rest) = text.strip_prefix(r"\\?\")
        && rest.as_bytes().get(1) == Some(&b':')
        && rest.as_bytes()[0].is_ascii_alphabetic()
    {
        return PathBuf::from(rest);
    }
    path
}

/// The text a path takes in a shell command line or an OpenSSH
/// configuration: UTF-8, no verbatim prefix, and on Windows forward slashes
/// only — both the Git for Windows `sh` and every Windows OpenSSH release
/// read `C:/x/y` as `C:\x\y`, while a backslash is an escape to some of them.
pub(super) fn path_text(path: &Path, windows: bool) -> Option<String> {
    let text = simplified_path(path.to_path_buf()).to_str()?.to_owned();
    Some(if windows {
        text.replace('\\', "/")
    } else {
        text
    })
}

/// The literal the Windows OpenSSH client prints where `%ProgramData%`
/// belongs. `ssh -G` reports its compiled-in system defaults as
/// `__PROGRAMDATA__\ssh/ssh_known_hosts`; the token is expanded by the
/// client itself when it opens the file, not by the environment, so no
/// ordinary `${NAME}` expansion resolves it.
pub const PROGRAM_DATA_TOKEN: &str = "__PROGRAMDATA__";

/// `path` with a leading [`PROGRAM_DATA_TOKEN`] component replaced by
/// `program_data`, written with the Windows separator throughout. `None`
/// when `path` does not begin with that token followed by a separator —
/// the token is a whole leading component, never a prefix of a name.
pub fn expand_program_data(path: &str, program_data: &str) -> Option<String> {
    let tail = path
        .strip_prefix(PROGRAM_DATA_TOKEN)?
        .strip_prefix(['\\', '/'])?;
    Some(format!(
        r"{}\{}",
        program_data
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_owned(),
        tail.replace('/', "\\")
    ))
}

/// Whether `path` is absolute for a client running on `windows`. There a
/// path is absolute only with a drive (`C:\x`, `C:/x`) or a `\\` root (a
/// UNC share, or a device namespace such as the OpenSSH agent's named
/// pipe). A rootless POSIX path — `/etc/ssh/ssh_known_hosts`, which the
/// MSYS client bundled with Git for Windows prints for its compiled-in
/// defaults — is *not* absolute on Windows: it names a different file on
/// every drive the process might be on. `windows` is a parameter, not a
/// `cfg`, so the Windows decision is tested on every platform; with
/// `windows` false this is exactly [`Path::is_absolute`].
pub fn absolute_path(path: &Path, windows: bool) -> bool {
    if !windows {
        return path.is_absolute();
    }
    let simplified = simplified_path(path.to_path_buf());
    let Some(text) = simplified.to_str() else {
        return false;
    };
    let bytes = text.as_bytes();
    (bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/'))
        || text.starts_with(r"\\")
        || text.starts_with("//")
}

/// A Windows named-pipe path (`\\.\pipe\name`), by either separator
/// spelling: the endpoint of the Windows OpenSSH agent.
pub fn named_pipe(path: &Path) -> bool {
    path.to_str().is_some_and(|text| {
        let text = text.replace('/', "\\");
        text.len() > 9
            && text
                .get(..9)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case(r"\\.\pipe\"))
    })
}

/// The OpenSSH bundled with the Git for Windows installation `git` belongs
/// to (`<prefix>/cmd/git.exe`, `<prefix>/bin/git.exe` or
/// `<prefix>/mingw64/bin/git.exe` → `<prefix>/usr/bin/ssh.exe`): the client
/// Git for Windows itself runs unless it was installed to use another.
pub fn bundled_ssh(git: &Path, exists: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    git.ancestors()
        .skip(1)
        .take(3)
        .map(|prefix| prefix.join("usr").join("bin").join("ssh.exe"))
        .find(|candidate| exists(candidate))
}

/// How a selected Windows OpenSSH client reads its configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsSshClient {
    pub executable: PathBuf,
    pub system: PathBuf,
    /// The home the client expands `~` with when it is not the caller's
    /// `HOME`: the Windows OpenSSH client uses the profile directory.
    pub home: Option<PathBuf>,
    /// The agent the client uses when `SSH_AUTH_SOCK` is unset.
    pub default_agent: Option<PathBuf>,
}

/// Git for Windows' bundled client (an MSYS2 program in `<prefix>/usr/bin`,
/// beside `msys-2.0.dll`) reads `<prefix>/etc/ssh/ssh_config` and the
/// caller's `HOME`, like Git itself. The Windows OpenSSH client reads
/// `%ProgramData%\ssh\ssh_config`, the profile directory, and the agent's
/// named pipe `\\.\pipe\openssh-ssh-agent`.
pub fn windows_ssh_client(
    executable: PathBuf,
    program_data: Option<PathBuf>,
    profile: Option<PathBuf>,
    exists: impl Fn(&Path) -> bool,
) -> Result<WindowsSshClient, super::GitError> {
    let layout = || super::unsupported("OpenSSH installation layout is unknown");
    let bin = executable.parent().ok_or_else(layout)?;
    let named = |path: &Path, name: &str| {
        path.file_name()
            .and_then(|found| found.to_str())
            .is_some_and(|found| found.eq_ignore_ascii_case(name))
    };
    if named(bin, "bin")
        && bin.parent().is_some_and(|usr| named(usr, "usr"))
        && exists(&bin.join("msys-2.0.dll"))
    {
        let prefix = bin.parent().and_then(Path::parent).ok_or_else(layout)?;
        return Ok(WindowsSshClient {
            system: prefix.join("etc").join("ssh").join("ssh_config"),
            executable,
            home: None,
            default_agent: None,
        });
    }
    let program_data = program_data.filter(|path| path.has_root()).ok_or_else(|| {
        super::unsupported("the Windows OpenSSH client needs the caller's absolute ProgramData")
    })?;
    Ok(WindowsSshClient {
        system: program_data.join("ssh").join("ssh_config"),
        executable,
        home: profile.filter(|path| path.has_root()),
        default_agent: Some(PathBuf::from(r"\\.\pipe\openssh-ssh-agent")),
    })
}

/// The TLS backend a controlled HTTPS child on Windows is told to use. Its
/// system configuration is not read (`GIT_CONFIG_NOSYSTEM`), so without
/// this Git for Windows would trust only its bundled Mozilla roots and
/// refuse a corporate inspecting proxy whose root is deployed to the
/// Windows store. With a selected CA file (`ca_file`, `GIT_SSL_CAINFO`) the
/// default backend reads that file instead.
pub(super) fn windows_tls_backend(windows: bool, selected_ca_file: bool) -> Option<&'static str> {
    (windows && !selected_ca_file).then_some("http.sslBackend=schannel")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    #[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-WINDOWS-LAW")]
    fn windows_children_keep_only_startup_variables() {
        let environment = [
            ("SYSTEMROOT", r"C:\Windows"),
            ("windir", r"C:\Windows"),
            ("TEMP", r"C:\Users\me\AppData\Local\Temp"),
            ("GIT_SSH_COMMAND", "evil"),
            ("HTTPS_PROXY", "http://ambient:3128"),
            ("PATH", r"C:\anything"),
            ("USERPROFILE", r"C:\Users\me"),
        ];
        let pairs = environment
            .iter()
            .map(|(k, v)| (OsStr::new(k), OsStr::new(v)));
        let kept = platform_variables(pairs.clone(), true);
        let names: Vec<_> = kept.iter().map(|(k, _)| k.to_str().unwrap()).collect();
        assert_eq!(names, ["SystemRoot", "windir", "TEMP"]);
        assert!(platform_variables(pairs, false).is_empty());
    }

    #[test]
    #[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-WINDOWS-LAW")]
    fn git_for_windows_layout_adds_its_own_runtime_directories() {
        let helpers = Path::new("/c/Program Files/Git/mingw64/libexec/git-core");
        let present = [
            PathBuf::from("/c/Program Files/Git/mingw64/bin"),
            PathBuf::from("/c/Program Files/Git/usr/bin"),
        ];
        let found = runtime_directories(helpers, true, |p| present.iter().any(|q| q == p));
        assert_eq!(
            found,
            [
                helpers.to_path_buf(),
                present[0].clone(),
                present[1].clone()
            ]
        );
        // A layout without them (MinGit without usr/bin) keeps what exists.
        let found = runtime_directories(helpers, true, |p| p == present[0]);
        assert_eq!(found, [helpers.to_path_buf(), present[0].clone()]);
        // Elsewhere the helper directory alone, whatever exists.
        assert_eq!(
            runtime_directories(helpers, false, |_| true),
            [helpers.to_path_buf()]
        );
    }

    #[test]
    fn verbatim_prefixes_are_removed_and_windows_text_uses_forward_slashes() {
        assert_eq!(
            simplified_path(PathBuf::from(r"\\?\C:\Program Files\Git\cmd\git.exe")),
            PathBuf::from(r"C:\Program Files\Git\cmd\git.exe")
        );
        assert_eq!(
            simplified_path(PathBuf::from(r"\\?\UNC\srv\share\git.exe")),
            PathBuf::from(r"\\srv\share\git.exe")
        );
        // A device path is not a disk path: left alone.
        assert_eq!(
            simplified_path(PathBuf::from(r"\\?\Volume{x}\git.exe")),
            PathBuf::from(r"\\?\Volume{x}\git.exe")
        );
        assert_eq!(
            path_text(Path::new(r"\\?\C:\Windows\System32\OpenSSH\ssh.exe"), true).unwrap(),
            "C:/Windows/System32/OpenSSH/ssh.exe"
        );
        assert_eq!(
            path_text(Path::new(r"\\.\pipe\openssh-ssh-agent"), true).unwrap(),
            "//./pipe/openssh-ssh-agent"
        );
        assert_eq!(
            path_text(Path::new("/usr/bin/ssh"), false).unwrap(),
            "/usr/bin/ssh"
        );
    }

    #[test]
    #[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-WINDOWS-LAW")]
    fn windows_ssh_selection_follows_the_client_git_for_windows_runs() {
        let prefix = Path::new("/c/Program Files/Git");
        let bundled = prefix.join("usr/bin/ssh.exe");
        for git in ["cmd/git.exe", "bin/git.exe", "mingw64/bin/git.exe"] {
            assert_eq!(
                bundled_ssh(&prefix.join(git), |p| p == bundled).as_deref(),
                Some(bundled.as_path()),
                "{git}"
            );
        }
        // "Use external OpenSSH": no bundled client.
        assert_eq!(bundled_ssh(&prefix.join("cmd/git.exe"), |_| false), None);

        let msys = windows_ssh_client(bundled.clone(), None, None, |p| p.ends_with("msys-2.0.dll"))
            .unwrap();
        assert_eq!(msys.system, prefix.join("etc/ssh/ssh_config"));
        assert_eq!((msys.home, msys.default_agent), (None, None));

        let system = PathBuf::from("/c/Windows/System32/OpenSSH/ssh.exe");
        let win32 = windows_ssh_client(
            system.clone(),
            Some(PathBuf::from("/c/ProgramData")),
            Some(PathBuf::from("/c/Users/me")),
            |_| false,
        )
        .unwrap();
        assert_eq!(win32.system, PathBuf::from("/c/ProgramData/ssh/ssh_config"));
        assert_eq!(win32.home, Some(PathBuf::from("/c/Users/me")));
        assert!(named_pipe(win32.default_agent.as_deref().unwrap()));
        // Without ProgramData the system configuration is unknown: refused.
        assert!(windows_ssh_client(system, None, None, |_| false).is_err());
    }

    #[test]
    #[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-WINDOWS-LAW")]
    fn program_data_tokens_expand_and_only_drive_or_unc_paths_are_absolute_on_windows() {
        // The exact shape Win32-OpenSSH prints, mixed separators and all.
        assert_eq!(
            expand_program_data(r"__PROGRAMDATA__\ssh/ssh_known_hosts", r"C:\ProgramData"),
            Some(r"C:\ProgramData\ssh\ssh_known_hosts".to_owned())
        );
        assert_eq!(
            expand_program_data("__PROGRAMDATA__/ssh/ssh_config", "C:/ProgramData/"),
            Some(r"C:\ProgramData\ssh\ssh_config".to_owned())
        );
        // A whole leading component only: a prefix of a name is not the token.
        assert_eq!(
            expand_program_data(r"__PROGRAMDATA__X\ssh", r"C:\ProgramData"),
            None
        );
        assert_eq!(
            expand_program_data("__PROGRAMDATA__", r"C:\ProgramData"),
            None
        );
        assert_eq!(
            expand_program_data(r"C:\ProgramData\ssh", r"C:\ProgramData"),
            None
        );

        for absolute in [
            r"C:\ProgramData\ssh\ssh_known_hosts",
            "C:/Users/dev/.ssh/known_hosts",
            r"\\server\share\known_hosts",
            r"\\.\pipe\openssh-ssh-agent",
            r"\\?\C:\Users\dev\.ssh\known_hosts",
        ] {
            assert!(absolute_path(Path::new(absolute), true), "{absolute}");
        }
        for relative in [
            "known_hosts",
            // A rootless POSIX path names a different file on every drive.
            "/etc/ssh/ssh_known_hosts",
            r"__PROGRAMDATA__\ssh\ssh_known_hosts",
            r"C:known_hosts",
        ] {
            assert!(!absolute_path(Path::new(relative), true), "{relative}");
        }
        // Off Windows this is exactly `Path::is_absolute`.
        assert!(absolute_path(Path::new("/etc/ssh/ssh_known_hosts"), false));
        assert!(!absolute_path(Path::new(r"C:\Users\dev"), false));
    }

    #[test]
    fn named_pipes_are_recognised_by_either_separator() {
        assert!(named_pipe(Path::new(r"\\.\pipe\openssh-ssh-agent")));
        assert!(named_pipe(Path::new("//./pipe/openssh-ssh-agent")));
        assert!(!named_pipe(Path::new(r"\\.\pipe\")));
        assert!(!named_pipe(Path::new("/tmp/agent.sock")));
    }

    #[test]
    #[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-WINDOWS-LAW")]
    fn windows_https_uses_the_system_store_unless_a_ca_file_is_selected() {
        assert_eq!(
            windows_tls_backend(true, false),
            Some("http.sslBackend=schannel")
        );
        assert_eq!(windows_tls_backend(true, true), None);
        assert_eq!(windows_tls_backend(false, false), None);
    }
}
