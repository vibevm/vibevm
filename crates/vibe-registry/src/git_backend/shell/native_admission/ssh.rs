//! Exact-endpoint OpenSSH capability. No ambient SSH configuration is interpreted.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-SSH");
use super::*;
use std::process::Command;

/// Explicit SSH authentication input. Agent state remains owned by that agent;
/// selecting its socket never inherits the process's SSH_AUTH_SOCK.
pub enum SshAuthentication {
    Key(Vec<u8>),
    AgentSocket(PathBuf),
}
impl std::fmt::Debug for SshAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SshAuthentication(..)")
    }
}
/// Caller-trusted OpenSSH plus exact credential/trust destination. Fields are
/// private and cannot be populated by deserializing a package manifest.
pub struct SshProfile {
    executable: PathBuf,
    host: String,
    user: String,
    port: u16,
    mode: Mode,
}
enum Mode {
    Direct {
        known_hosts: Vec<u8>,
        authentication: SshAuthentication,
    },
    Captured(Box<super::ssh_config::Configured>),
}
impl std::fmt::Debug for SshProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SshProfile(..)")
    }
}
impl SshProfile {
    /// Select one exact SSH endpoint and captured trust/authentication inputs.
    /// This explicitly selects the direct endpoint for fresh acquisition AND
    /// warm refresh. It overrides historical ambient SSH routing by caller
    /// choice; it does not infer that choice from an origin URL or claim to
    /// capture/migrate the old ~/.ssh/config meaning. N2 still rechecks Git's
    /// captured effective origin and original source witnesses.
    /// Paths identify caller-trusted programs/services, not authenticated binaries.
    pub fn explicit_direct(
        executable: PathBuf,
        host: String,
        user: String,
        port: u16,
        known_hosts: Vec<u8>,
        authentication: SshAuthentication,
    ) -> Result<Self, GitError> {
        if !executable.is_absolute()
            || !executable.is_file()
            || port == 0
            || !direct_host(&host)
            || !user_name(&user)
            || known_hosts.is_empty()
        {
            return Err(unsupported(
                "SSH profile needs an explicit trusted executable, direct host/user/port and known hosts",
            ));
        }
        match &authentication {
            SshAuthentication::Key(bytes) if bytes.is_empty() => {
                return Err(unsupported("SSH identity is empty"));
            }
            SshAuthentication::AgentSocket(path) if !path.is_absolute() => {
                return Err(unsupported(
                    "SSH agent selection requires an absolute socket path",
                ));
            }
            _ => (),
        }
        Ok(Self {
            executable,
            host,
            user,
            port,
            mode: Mode::Direct {
                known_hosts,
                authentication,
            },
        })
    }
    pub(super) fn captured(
        host: String,
        user: String,
        port: u16,
        configured: super::ssh_config::Configured,
    ) -> Self {
        Self {
            executable: configured.executable().to_path_buf(),
            host,
            user,
            port,
            mode: Mode::Captured(Box::new(configured)),
        }
    }
    pub(super) fn same_endpoint(&self, other: &Self) -> bool {
        match (&self.mode, &other.mode) {
            (Mode::Captured(a), Mode::Captured(b)) => a.same_authored(b),
            (Mode::Direct { .. }, Mode::Direct { .. }) => {
                (self.host.as_str(), self.user.as_str(), self.port)
                    == (other.host.as_str(), other.user.as_str(), other.port)
            }
            _ => false,
        }
    }
    pub(super) fn capture_candidate(
        &self,
        address: &str,
        binding: &crate::HttpAdmission,
        profile_index: usize,
    ) -> Result<Option<Remote>, GitError> {
        if let Mode::Captured(configured) = &self.mode {
            if !configured.matches(address)? {
                return Ok(None);
            }
            let authority = Url::parse(&format!("ssh://{}:{}/", self.host, self.port))
                .map_err(|_| unsupported("invalid captured SSH authority"))?;
            binding.admit_host(&authority)?;
            return Ok(Some(Remote {
                binding: ProfileBinding::Direct(profile_index),
                address: address.into(),
                host: self.host.clone(),
                user: self.user.clone(),
                port: self.port,
            }));
        }
        let (host, user, port, path) = if address.starts_with("ssh://") {
            let url = Url::parse(address).map_err(|_| unsupported("invalid SSH URL"))?;
            if url.password().is_some() || url.query().is_some() || url.fragment().is_some() {
                return Err(unsupported(
                    "SSH URL password/query/fragment is unsupported",
                ));
            }
            (
                url.host_str().unwrap_or_default().to_owned(),
                if url.username().is_empty() {
                    self.user.clone()
                } else {
                    url.username().to_owned()
                },
                url.port().unwrap_or(22),
                url.path().to_owned(),
            )
        } else {
            let (authority, path) = if let Some(end) = address.find("]:") {
                (&address[..=end], &address[end + 2..])
            } else {
                address
                    .split_once(':')
                    .ok_or_else(|| unsupported("direct SSH URL or SCP address required"))?
            };
            let (user, host) = authority
                .rsplit_once('@')
                .map_or((self.user.as_str(), authority), |(u, h)| (u, h));
            (host.to_owned(), user.to_owned(), self.port, path.to_owned())
        };
        if !direct_host(&host) || !user_name(&user) || !repository_path(&path) {
            return Err(unsupported(
                "opaque SSH aliases or unsupported endpoint syntax require explicit capture",
            ));
        }
        let authority = Url::parse(&format!("ssh://{host}:{port}/"))
            .map_err(|_| unsupported("invalid SSH authority"))?;
        binding.admit_host(&authority)?;
        // Root-dot normalization is solely for denial, never for credential or
        // known-host selection. The selected endpoint must match exactly.
        if host != self.host || user != self.user || port != self.port {
            return Ok(None);
        }
        Ok(Some(Remote {
            binding: ProfileBinding::Direct(profile_index),
            address: address.to_owned(),
            host,
            user,
            port,
        }))
    }
    /// Write the owned client configuration and select it for this child.
    /// On Windows the same configuration is used; Git runs
    /// `GIT_SSH_COMMAND` with the installation's own `sh`
    /// (`platform::runtime_directories`), and every path is written with
    /// forward slashes (`platform::path_text`).
    pub(super) fn configure(
        &self,
        command: &mut Command,
        root: &Path,
        remote: &Remote,
    ) -> Result<(), GitError> {
        use std::io::Write;
        let mut config = format!(
            "Host *\n HostName {}\n User {}\n Port {}\n StrictHostKeyChecking yes\n BatchMode yes\n PasswordAuthentication no\n KbdInteractiveAuthentication no\n GSSAPIAuthentication no\n HostbasedAuthentication no\n CanonicalizeHostname no\n VerifyHostKeyDNS no\n ProxyCommand none\n ProxyJump none\n PermitLocalCommand no\n ForwardAgent no\n ForwardX11 no\n ClearAllForwardings yes\n ControlMaster no\n ControlPath none\n UpdateHostKeys no\n RequestTTY no\n ConnectTimeout 10\n",
            remote.host.trim_start_matches('[').trim_end_matches(']'),
            remote.user,
            remote.port
        );
        match &self.mode {
            Mode::Captured(configured) => configured.configure(command, root, &mut config)?,
            Mode::Direct {
                known_hosts,
                authentication,
            } => {
                let known = root.join("known_hosts");
                std::fs::write(&known, known_hosts).map_err(io)?;
                config.push_str(&format!(
                    " UserKnownHostsFile {}\n GlobalKnownHostsFile {}\n",
                    config_path(&known)?,
                    config_path(&root.join("config"))?
                ));
                match authentication {
                    SshAuthentication::Key(bytes) => {
                        let identity = root.join("ssh-identity");
                        let mut options = std::fs::OpenOptions::new();
                        options.write(true).create(true).truncate(true);
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::OpenOptionsExt;
                            options.mode(0o600);
                        }
                        let mut file = options.open(&identity).map_err(io)?;
                        file.write_all(bytes).map_err(io)?;
                        config.push_str(&format!(
                            " IdentityFile {}\n IdentitiesOnly yes\n IdentityAgent none\n",
                            config_path(&identity)?
                        ));
                    }
                    SshAuthentication::AgentSocket(path) => {
                        if !agent_endpoint(path)? {
                            return Err(unsupported("selected SSH agent endpoint is not a socket"));
                        }
                        config.push_str(&format!(
                            " IdentityFile none\n IdentitiesOnly no\n IdentityAgent {}\n",
                            config_path(path)?
                        ));
                        command.env("SSH_AUTH_SOCK", path);
                    }
                }
            }
        }
        let selected = root.join("ssh_config");
        std::fs::write(&selected, config).map_err(io)?;
        let executable = platform::path_text(&self.executable, cfg!(windows))
            .ok_or_else(|| unsupported("SSH executable path is not UTF-8"))?;
        let config = platform::path_text(&selected, cfg!(windows))
            .ok_or_else(|| unsupported("owned SSH config path is not UTF-8"))?;
        // Only caller-trusted executable and owned config paths enter this
        // fixed shell prefix. Package addresses are never interpolated here.
        command
            .env(
                "GIT_SSH_COMMAND",
                format!("{} -F {}", shell_quote(&executable)?, shell_quote(&config)?),
            )
            .env("GIT_SSH_VARIANT", "ssh");
        Ok(())
    }
}
pub(super) struct Remote {
    binding: ProfileBinding,
    pub(super) address: String,
    host: String,
    user: String,
    port: u16,
}
enum ProfileBinding {
    Direct(usize),
    Captured(Arc<SshProfile>),
}
impl Remote {
    pub(super) fn configure(
        &self,
        profiles: &[SshProfile],
        command: &mut Command,
        root: &Path,
    ) -> Result<(), GitError> {
        match &self.binding {
            ProfileBinding::Direct(index) => profiles
                .get(*index)
                .ok_or_else(|| unsupported("captured SSH profile disappeared"))?
                .configure(command, root, self),
            ProfileBinding::Captured(profile) => profile.configure(command, root, self),
        }
    }
}
pub(super) fn captured_remote(
    profile: Arc<SshProfile>,
    address: &str,
    binding: &crate::HttpAdmission,
) -> Result<Remote, GitError> {
    let mut remote = profile
        .capture_candidate(address, binding, 0)?
        .ok_or_else(|| unsupported("captured SSH address changed"))?;
    remote.binding = ProfileBinding::Captured(profile);
    Ok(remote)
}
pub(super) fn user_name(user: &str) -> bool {
    !user.is_empty()
        && !user.starts_with('-')
        && user
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
pub(super) fn direct_host(host: &str) -> bool {
    host.is_ascii()
        && (host.contains('.') || host.starts_with('['))
        && host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-:[]".contains(&b))
        && vibe_core::remap::endpoint::normalized_host_key(host).is_some()
}
pub(super) fn repository_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('-')
        && path
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/._-~".contains(&b))
}
pub(super) fn config_path(path: &Path) -> Result<String, GitError> {
    let path = platform::path_text(path, cfg!(windows))
        .ok_or_else(|| unsupported("SSH path is not UTF-8"))?;
    if path.contains(['%', '$']) || path.chars().any(char::is_control) {
        return Err(unsupported(
            "SSH path contains interpolation or control syntax",
        ));
    }
    Ok(format!(
        "\"{}\"",
        path.replace('\\', "\\\\").replace('"', "\\\"")
    ))
}
/// Whether `path` is an SSH agent endpoint: a Unix socket, or on Windows a
/// named pipe (`\\.\pipe\...`, the Windows OpenSSH agent).
fn agent_endpoint(path: &Path) -> Result<bool, GitError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        Ok(std::fs::metadata(path).map_err(io)?.file_type().is_socket())
    }
    #[cfg(not(unix))]
    {
        Ok(platform::named_pipe(path) && path.exists())
    }
}
fn shell_quote(value: &str) -> Result<String, GitError> {
    if value.chars().any(char::is_control) {
        return Err(unsupported("SSH launcher path contains control syntax"));
    }
    Ok(format!("'{}'", value.replace('\'', "'\"'\"'")))
}
pub(in crate::git_backend::shell) fn failure(stderr: &[u8]) -> Option<GitError> {
    let text = String::from_utf8_lossy(stderr);
    if text.contains("Host key verification failed") {
        Some(GitError::SshHostKeyRejected)
    } else if text.contains("Permission denied (") {
        Some(GitError::SshAuthenticationRejected)
    } else {
        None
    }
}
