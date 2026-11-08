//! Audited immutable OpenSSH configuration for an already selected source.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE");
use super::*;
use std::collections::BTreeMap;
mod inputs;
mod values;
mod witness;
use inputs::Frozen;
use witness::Witness;

/// Explicit capture inputs, supplied by the invocation's composition owner.
/// No ambient environment or user key content is read by construction.
/// ```
/// use std::path::PathBuf;
/// use vibe_registry::git_backend::shell::native_admission::SshConfigSelection;
/// let selected = SshConfigSelection::new(PathBuf::from("/usr/bin/ssh"), PathBuf::from("/home/example"), Some(PathBuf::from("/etc/ssh/ssh_config")))?;
/// # Ok::<(), vibe_registry::GitError>(())
/// ```
pub struct SshConfigSelection {
    executable: PathBuf,
    home: PathBuf,
    user: PathBuf,
    system: Option<PathBuf>,
    environment: BTreeMap<String, String>,
}
impl std::fmt::Debug for SshConfigSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SshConfigSelection(..)")
    }
}
impl SshConfigSelection {
    pub fn new(
        executable: PathBuf,
        home: PathBuf,
        system: Option<PathBuf>,
    ) -> Result<Self, GitError> {
        if !executable.is_absolute()
            || !home.is_absolute()
            || system.as_ref().is_some_and(|p| !p.is_absolute())
        {
            return Err(unsupported(
                "SSH capture requires explicit absolute installation/home/config paths",
            ));
        }
        Ok(Self {
            executable,
            user: home.join(".ssh/config"),
            home,
            system,
            environment: BTreeMap::new(),
        })
    }
    /// Select an alternate ordinary user configuration without changing HOME.
    pub fn with_user_config(mut self, path: PathBuf) -> Result<Self, GitError> {
        if !path.is_absolute() {
            return Err(unsupported("SSH config path must be absolute"));
        }
        self.user = path;
        Ok(self)
    }
    /// Supply a captured expansion/agent value; it is data, never a child loader,
    /// shell startup, executable selection or inherited command override.
    pub fn with_environment(mut self, key: &str, value: String) -> Result<Self, GitError> {
        if key.is_empty()
            || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            || value.contains('\0')
        {
            return Err(unsupported("invalid SSH expansion input"));
        }
        self.environment.insert(key.into(), value);
        Ok(self)
    }
    /// Audit and freeze the complete supported Include closure before -G can run.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE-LAW"
    )]
    pub fn capture(self) -> Result<SshConfiguration, GitError> {
        if !self.executable.is_file() {
            return Err(unsupported("selected OpenSSH installation is unavailable"));
        }
        let frozen = Frozen::capture(&self)?;
        Ok(SshConfiguration {
            selected: self,
            frozen,
        })
    }
}
/// An audited closure can lower newly selected child/redirect addresses without
/// refreshing ambient globals or creating a mutable credential cache.
/// ```
/// use std::sync::Arc;
/// use vibe_registry::git_backend::shell::native_admission::{NativeGitAdmission,SshConfiguration};
/// fn bind(profile: NativeGitAdmission, config:SshConfiguration)->NativeGitAdmission {
///     profile.with_ssh_configuration(Arc::new(config))
/// }
/// ```
pub struct SshConfiguration {
    selected: SshConfigSelection,
    frozen: Frozen,
}
impl std::fmt::Debug for SshConfiguration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SshConfiguration(..)")
    }
}
#[derive(Clone, PartialEq, Eq)]
pub(super) struct Authority {
    host: String,
    user: Option<String>,
    port: Option<u16>,
}
impl Authority {
    fn parse(address: &str) -> Result<Self, GitError> {
        let (host, user, port, path) = if address.starts_with("ssh://") {
            let url =
                Url::parse(address).map_err(|_| unsupported("invalid authored SSH address"))?;
            if url.password().is_some() || url.query().is_some() || url.fragment().is_some() {
                return Err(unsupported(
                    "unsupported authored SSH URL credentials or suffix",
                ));
            }
            (
                url.host_str().unwrap_or_default().to_owned(),
                (!url.username().is_empty()).then(|| url.username().to_owned()),
                url.port(),
                url.path().to_owned(),
            )
        } else {
            let (authority, path) = if let Some(end) = address.find("]:") {
                (&address[..=end], &address[end + 2..])
            } else {
                address
                    .split_once(':')
                    .ok_or_else(|| unsupported("SSH address requires a host and repository path"))?
            };
            let (user, host) = authority
                .rsplit_once('@')
                .map_or((None, authority), |(u, h)| (Some(u.to_owned()), h));
            (host.into(), user, None, path.into())
        };
        if host.is_empty()
            || host.starts_with('-')
            || !host
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-:[]".contains(&b))
            || user.as_ref().is_some_and(|u| !super::ssh::user_name(u))
            || !super::ssh::repository_path(&path)
        {
            return Err(unsupported(
                "unsupported authored SSH authority or repository syntax",
            ));
        }
        Ok(Self { host, user, port })
    }
}
impl SshConfiguration {
    fn evaluate(&self, address: &str) -> Result<(Authority, String), GitError> {
        let authority = Authority::parse(address)?;
        self.frozen.witness.recheck()?;
        let output = inputs::evaluate(&self.selected, &self.frozen, &authority)?;
        self.frozen.witness.recheck()?;
        Ok((authority, output))
    }
    /// Admit the actual evaluated endpoint without selecting identities/trust.
    /// ```
    /// use vibe_registry::{GitError, HttpAdmission};
    /// use vibe_registry::git_backend::shell::native_admission::SshConfiguration;
    /// fn preflight(config: &SshConfiguration, selected_source: &str, binding: &HttpAdmission) -> Result<(), GitError> {
    ///     config.preflight_address(selected_source, binding)
    /// }
    /// ```
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE-LAW"
    )]
    pub fn preflight_address(
        &self,
        address: &str,
        binding: &crate::HttpAdmission,
    ) -> Result<(), GitError> {
        let (_, output) = self.evaluate(address)?;
        values::endpoint(&output)?.admit(binding)
    }
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE-LAW"
    )]
    pub(super) fn profile(
        self: &Arc<Self>,
        address: &str,
        binding: &crate::HttpAdmission,
    ) -> Result<SshProfile, GitError> {
        let (authority, output) = self.evaluate(address)?;
        values::endpoint(&output)?.admit(binding)?;
        values::profile(self.clone(), authority, &output)
    }
}
pub(super) struct Configured {
    owner: Arc<SshConfiguration>,
    authored: Authority,
    identities: Vec<String>,
    agent: String,
    identities_only: bool,
    client_options: Vec<(String, String)>,
    trust: Vec<(String, Vec<u8>)>,
    host_key_alias: Option<String>,
    check_host_ip: bool,
    witness: Witness,
}
impl Configured {
    pub(super) fn matches(&self, address: &str) -> Result<bool, GitError> {
        Ok(self.authored == Authority::parse(address)?)
    }
    pub(super) fn same_authored(&self, other: &Self) -> bool {
        self.authored == other.authored
    }
    pub(super) fn configure(
        &self,
        command: &mut std::process::Command,
        root: &Path,
        config: &mut String,
    ) -> Result<(), GitError> {
        self.owner.frozen.witness.recheck()?;
        self.witness.recheck()?;
        for (key, value) in &self.client_options {
            config.push_str(&format!(" {key} {value}\n"));
        }
        config.push_str(" PubkeyAuthentication yes\n PreferredAuthentications publickey\n");
        for identity in &self.identities {
            config.push_str(&format!(
                " IdentityFile {}\n",
                if identity == "none" {
                    "none".into()
                } else {
                    super::ssh::config_path(Path::new(identity))?
                }
            ));
        }
        config.push_str(&format!(
            " IdentitiesOnly {}\n IdentityAgent {}\n AddKeysToAgent no\n",
            if self.identities_only { "yes" } else { "no" },
            if self.agent == "none" {
                "none".into()
            } else {
                super::ssh::config_path(Path::new(&self.agent))?
            }
        ));
        if self.agent != "none" {
            command.env("SSH_AUTH_SOCK", &self.agent);
        }
        for kind in ["userknownhostsfile", "globalknownhostsfile"] {
            let mut files = Vec::new();
            for (index, (_, bytes)) in self
                .trust
                .iter()
                .enumerate()
                .filter(|(_, (name, _))| name == kind)
            {
                let path = root.join(format!("trust-{index}"));
                std::fs::write(&path, bytes).map_err(io)?;
                files.push(super::ssh::config_path(&path)?);
            }
            config.push_str(&format!(
                " {kind} {}\n",
                if files.is_empty() {
                    "none".into()
                } else {
                    files.join(" ")
                }
            ));
        }
        if let Some(alias) = &self.host_key_alias {
            config.push_str(&format!(" HostKeyAlias {alias}\n"));
        }
        config.push_str(&format!(
            " CheckHostIP {}\n",
            if self.check_host_ip { "yes" } else { "no" }
        ));
        Ok(())
    }
}
#[cfg(all(test, unix))]
mod tests;
