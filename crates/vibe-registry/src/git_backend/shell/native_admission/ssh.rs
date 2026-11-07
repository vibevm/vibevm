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
    known_hosts: Vec<u8>,
    authentication: SshAuthentication,
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
            known_hosts,
            authentication,
        })
    }
    pub(super) fn capture(
        &self,
        address: &str,
        binding: &crate::HttpAdmission,
    ) -> Result<Remote, GitError> {
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
            return Err(unsupported(
                "SSH endpoint differs from the exact selected credential origin",
            ));
        }
        Ok(Remote {
            address: address.to_owned(),
            host,
            user,
            port,
        })
    }
    pub(super) fn configure(
        &self,
        command: &mut Command,
        root: &Path,
        remote: &Remote,
    ) -> Result<(), GitError> {
        #[cfg(not(unix))]
        {
            let _ = (command, root, remote);
            return Err(unsupported(
                "this SSH profile requires a separately proved platform launcher",
            ));
        }
        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::{
                ffi::OsStrExt,
                fs::{FileTypeExt, OpenOptionsExt},
            };
            let known = root.join("known_hosts");
            std::fs::write(&known, &self.known_hosts).map_err(io)?;
            let mut config = format!(
                "Host *\n HostName {}\n User {}\n Port {}\n UserKnownHostsFile {}\n GlobalKnownHostsFile {}\n StrictHostKeyChecking yes\n BatchMode yes\n PasswordAuthentication no\n KbdInteractiveAuthentication no\n GSSAPIAuthentication no\n HostbasedAuthentication no\n CanonicalizeHostname no\n VerifyHostKeyDNS no\n ProxyCommand none\n ProxyJump none\n PermitLocalCommand no\n ForwardAgent no\n ForwardX11 no\n ClearAllForwardings yes\n ControlMaster no\n ControlPath none\n UpdateHostKeys no\n RequestTTY no\n ConnectTimeout 10\n",
                remote.host.trim_start_matches('[').trim_end_matches(']'),
                remote.user,
                remote.port,
                config_path(&known)?,
                config_path(&root.join("config"))?
            );
            match &self.authentication {
                SshAuthentication::Key(bytes) => {
                    let identity = root.join("ssh-identity");
                    let mut file = std::fs::OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .mode(0o600)
                        .open(&identity)
                        .map_err(io)?;
                    file.write_all(bytes).map_err(io)?;
                    config.push_str(&format!(
                        " IdentityFile {}\n IdentitiesOnly yes\n IdentityAgent none\n",
                        config_path(&identity)?
                    ));
                }
                SshAuthentication::AgentSocket(path) => {
                    if !std::fs::metadata(path).map_err(io)?.file_type().is_socket() {
                        return Err(unsupported("selected SSH agent endpoint is not a socket"));
                    }
                    config.push_str(&format!(
                        " IdentityFile none\n IdentitiesOnly no\n IdentityAgent {}\n",
                        config_path(path)?
                    ));
                    command.env("SSH_AUTH_SOCK", path);
                }
            }
            let selected = root.join("ssh_config");
            std::fs::write(&selected, config).map_err(io)?;
            let executable = std::str::from_utf8(self.executable.as_os_str().as_bytes())
                .map_err(|_| unsupported("SSH executable path is not UTF-8"))?;
            let config = selected
                .to_str()
                .ok_or_else(|| unsupported("owned SSH config path is not UTF-8"))?;
            // Only caller-trusted executable and owned config paths enter this
            // fixed shell prefix. Package addresses are never interpolated here.
            command
                .env(
                    "GIT_SSH_COMMAND",
                    format!("{} -F {}", shell_quote(executable)?, shell_quote(config)?),
                )
                .env("GIT_SSH_VARIANT", "ssh");
            Ok(())
        }
    }
}
pub(super) struct Remote {
    pub(super) address: String,
    host: String,
    user: String,
    port: u16,
}
fn user_name(user: &str) -> bool {
    !user.is_empty()
        && !user.starts_with('-')
        && user
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
fn direct_host(host: &str) -> bool {
    host.is_ascii()
        && (host.contains('.') || host.starts_with('['))
        && host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-:[]".contains(&b))
        && vibe_core::remap::endpoint::normalized_host_key(host).is_some()
}
fn repository_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('-')
        && path
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/._-~".contains(&b))
}
#[cfg(unix)]
fn config_path(path: &Path) -> Result<String, GitError> {
    let path = path
        .to_str()
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
#[cfg(unix)]
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
