//! Selected program/exec-path discovery occurs only when transport is demanded.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-PREPARATION");
use super::*;
use std::io::Read;
use vibe_registry::git_backend::shell::native_admission;
pub(super) fn executable(
    input: &TransportInputs,
    name: &std::ffi::OsStr,
) -> Result<PathBuf, GitError> {
    let named = Path::new(name);
    let candidates = if named.is_absolute() {
        vec![named.into()]
    } else if named.components().count() > 1 {
        vec![input.cwd.join(named)]
    } else {
        let path = input
            .variable("PATH")
            .ok_or_else(|| unsupported("captured PATH is unavailable"))?;
        std::env::split_paths(path)
            .flat_map(|directory| {
                let directory = if directory.is_absolute() {
                    directory
                } else {
                    input.cwd.join(directory)
                };
                let plain = directory.join(named);
                #[cfg(windows)]
                {
                    vec![
                        plain.clone(),
                        plain.with_extension("exe"),
                        plain.with_extension("cmd"),
                    ]
                }
                #[cfg(not(windows))]
                {
                    vec![plain]
                }
            })
            .collect()
    };
    for path in candidates {
        if path.is_file() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if path
                    .metadata()
                    .map_err(|_| unsupported("selected executable metadata is unavailable"))?
                    .permissions()
                    .mode()
                    & 0o111
                    == 0
                {
                    continue;
                }
            }
            return std::fs::canonicalize(path)
                .map(native_admission::platform::simplified_path)
                .map_err(|_| unsupported("selected executable cannot be identified"));
        }
    }
    Err(GitError::NotInstalled)
}
pub(super) fn git(input: &TransportInputs) -> Result<(PathBuf, PathBuf), GitError> {
    let binary = executable(input, &input.git_name())?;
    if let Some(selected) = input.variable("GIT_EXEC_PATH") {
        let helpers = PathBuf::from(selected);
        if !helpers.is_absolute() {
            return Err(unsupported("selected Git exec-path must be absolute"));
        }
        return Ok((binary, helpers));
    }
    std::fs::create_dir_all(&input.acquisition_root)
        .map_err(|_| unsupported("operation acquisition root is unavailable"))?;
    let temporary = tempfile::Builder::new()
        .prefix("git-installation-")
        .tempdir_in(&input.acquisition_root)
        .map_err(|_| unsupported("Git installation observation scratch is unavailable"))?;
    let empty = temporary.path().join("empty");
    std::fs::write(&empty, b"")
        .map_err(|_| unsupported("Git installation scratch is unavailable"))?;
    let mut command = Command::new(&binary);
    command
        .env_clear()
        .env("PATH", binary.parent().unwrap_or(Path::new("/")))
        .env("HOME", temporary.path())
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", &empty)
        .env("GIT_CONFIG_SYSTEM", &empty)
        .env("LC_ALL", "C")
        .envs(native_admission::platform::platform_variables(
            input
                .environment
                .iter()
                .map(|(key, value)| (key.as_os_str(), value.as_os_str())),
            cfg!(windows),
        ))
        .arg("--exec-path")
        .current_dir(temporary.path())
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    let control = vibe_registry::git_backend::transfers::invocation_control();
    let output = vibe_registry::git_backend::shell::controlled::bounded_output(
        &mut command,
        &control,
        std::time::Duration::from_secs(5),
        65536,
    )
    .map_err(|error| {
        if error
            .get_ref()
            .is_some_and(|error| error.is::<vibe_core::cancellation::Cancelled>())
        {
            GitError::ContactRefused(vibe_registry::ContactRefusal::OperationCancelled)
        } else {
            GitError::NotInstalled
        }
    })?;
    if !output.status.success() {
        return Err(GitError::NotInstalled);
    }
    let helpers = std::str::from_utf8(&output.stdout)
        .map_err(|_| unsupported("Git exec-path output is not UTF-8"))?
        .trim();
    let helpers = PathBuf::from(helpers);
    if !helpers.is_absolute() {
        return Err(unsupported("selected Git returned a relative exec-path"));
    }
    Ok((binary, helpers))
}
/// The OpenSSH client a controlled SSH child uses and how it reads its
/// configuration.
pub(super) struct SelectedClient {
    pub executable: PathBuf,
    /// A user configuration other than `<home>/.ssh/config` (`-F`).
    pub user: Option<PathBuf>,
    pub system: Option<PathBuf>,
    /// The home the client itself expands `~` with, when it is not the
    /// caller's `HOME`: the Windows OpenSSH client uses the profile directory.
    pub home: Option<PathBuf>,
    /// The agent the client uses when `SSH_AUTH_SOCK` is unset.
    pub default_agent: Option<PathBuf>,
}
pub(super) fn ssh(input: &TransportInputs) -> Result<SelectedClient, GitError> {
    if input
        .variable("GIT_SSH_VARIANT")
        .is_some_and(|value| value != "ssh" && value != "auto")
    {
        return Err(unsupported("selected SSH variant is not qualified OpenSSH"));
    }
    // The only qualified command override is a literal executable plus -F and
    // an absolute config path. No shell operators/options are interpreted.
    if let Some(command) = input.variable("GIT_SSH_COMMAND") {
        let command = command
            .to_str()
            .ok_or_else(|| unsupported("SSH command override is not UTF-8"))?;
        let parts: Vec<_> = command.split_whitespace().collect();
        if parts.len() != 3
            || parts[1] != "-F"
            || parts
                .iter()
                .any(|p| p.contains(['\'', '"', '$', '`', ';', '|', '&', '<', '>', '\n', '\r']))
            || !Path::new(parts[2]).is_absolute()
        {
            return Err(unsupported(
                "controlled SSH command override requires a separately qualified profile",
            ));
        }
        return Ok(SelectedClient {
            executable: executable(input, std::ffi::OsStr::new(parts[0]))?,
            user: Some(parts[2].into()),
            system: None,
            home: None,
            default_agent: None,
        });
    }
    if input.variable("GIT_SSH").is_some() {
        return Err(unsupported(
            "custom GIT_SSH requires a separately qualified controlled installation",
        ));
    }
    #[cfg(unix)]
    {
        let executable = executable(input, std::ffi::OsStr::new("ssh"))?;
        let parent = executable
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| unsupported("OpenSSH installation layout is unknown"))?;
        let system = if parent == Path::new("/usr") || parent == Path::new("/") {
            PathBuf::from("/etc/ssh/ssh_config")
        } else {
            return Err(unsupported(
                "selected OpenSSH system-config layout requires qualification",
            ));
        };
        Ok(SelectedClient {
            executable,
            user: None,
            system: Some(system),
            home: None,
            default_agent: None,
        })
    }
    #[cfg(not(unix))]
    {
        // The client Git for Windows itself runs: its bundled `usr/bin/ssh`,
        // else the `ssh` on PATH (the Windows OpenSSH client).
        let git = executable(input, &input.git_name()).ok();
        let bundled = git
            .as_deref()
            .and_then(|git| native_admission::platform::bundled_ssh(git, Path::is_file));
        let found = match bundled {
            Some(path) => path,
            None => executable(input, std::ffi::OsStr::new("ssh"))?,
        };
        let client = native_admission::platform::windows_ssh_client(
            found,
            input.variable("ProgramData").map(PathBuf::from),
            input.variable("USERPROFILE").map(PathBuf::from),
            Path::is_file,
        )?;
        Ok(SelectedClient {
            executable: client.executable,
            user: None,
            system: Some(client.system),
            home: client.home,
            default_agent: client.default_agent,
        })
    }
}
pub(super) fn bounded_file(path: &Path) -> Result<Vec<u8>, GitError> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    let file = options
        .open(path)
        .map_err(|_| unsupported("selected certificate input cannot be opened"))?;
    let before = file
        .metadata()
        .map_err(|_| unsupported("certificate input metadata is unavailable"))?;
    if !before.is_file() || before.len() > 4 * 1024 * 1024 {
        return Err(unsupported(
            "certificate input is not a bounded regular file",
        ));
    }
    let mut bytes = Vec::new();
    (&file)
        .take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| unsupported("certificate input could not be read"))?;
    let after = file
        .metadata()
        .map_err(|_| unsupported("certificate input changed"))?;
    if bytes.len() > 4 * 1024 * 1024
        || bytes.len() as u64 != before.len()
        || before.len() != after.len()
        || before.modified().ok() != after.modified().ok()
    {
        return Err(unsupported("certificate input changed while captured"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let current = std::fs::metadata(path)
            .map_err(|_| unsupported("selected certificate identity changed"))?;
        if (
            before.dev(),
            before.ino(),
            before.ctime(),
            before.ctime_nsec(),
        ) != (after.dev(), after.ino(), after.ctime(), after.ctime_nsec())
            || (after.dev(), after.ino()) != (current.dev(), current.ino())
        {
            return Err(unsupported("selected certificate identity changed"));
        }
    }
    Ok(bytes)
}
