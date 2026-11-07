//! Explicit direct HTTPS/SSH preparation; no ambient Git execution authority.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-PREPARATION");
use super::{GitError, GitPreparedRevision};
use reqwest::Url;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
mod capture;
mod child_layout;
pub(super) mod ssh;
pub use ssh::{SshAuthentication, SshProfile};
mod context;
pub use capture::NativeConfigSelection;
mod materialisation;
pub use materialisation::NativeMaterialisation;
mod gitlinks;
mod metadata;
mod preparation;
mod relocation;
pub(super) use context::AdmittedCommand;
use context::Context;
pub use relocation::HttpDiscoveryTrust;

/// Caller-trusted Git installation. Absolute paths are checked for consistency,
/// not authenticated. Never construct this from package metadata or remote data.
#[derive(Debug)]
pub struct GitInstallation {
    executable: PathBuf,
    helpers: PathBuf,
}
impl GitInstallation {
    /// Capture explicitly trusted executable and builtin-helper locations.
    pub fn trusted(executable: PathBuf, helpers: PathBuf) -> Result<Self, GitError> {
        if !executable.is_absolute()
            || !helpers.is_absolute()
            || !executable.is_file()
            || !helpers.join("git-remote-https").is_file()
        {
            return Err(unsupported(
                "trusted Git installation paths are unavailable",
            ));
        }
        Ok(Self {
            executable,
            helpers,
        })
    }
}
/// Explicit TLS trust input. Verification is always enabled.
#[derive(Debug)]
pub enum TlsTrust {
    /// The trusted Git installation's system certificate store.
    System,
    /// Captured PEM certificate authorities, copied only into the operation home.
    Pem(Vec<u8>),
}
/// Immutable native capability, distinct from HTTP-only admission.
/// HTTPS is selected initially; SSH requires an explicit additional profile.
///
/// ```
/// use std::{path::PathBuf, sync::Arc};
/// use vibe_registry::{HttpAdmission, GitError, ShellGit};
/// use vibe_registry::git_backend::shell::native_admission::{GitInstallation, NativeGitAdmission, TlsTrust};
/// fn explicit_backend(binary: PathBuf, helpers: PathBuf) -> Result<ShellGit, GitError> {
///     let binding = Arc::new(HttpAdmission::new(["excluded.invalid"])?);
///     let installation = GitInstallation::trusted(binary, helpers)?;
///     let profile = NativeGitAdmission::direct_https(binding.clone(), installation, TlsTrust::System);
///     assert!(Arc::ptr_eq(&binding, &profile.http_admission()));
///     Ok(ShellGit::new().with_native_admission(Arc::new(profile)))
/// }
/// ```
#[derive(Debug)]
pub struct NativeGitAdmission {
    binding: Arc<crate::HttpAdmission>,
    installation: GitInstallation,
    trust: TlsTrust,
    selection: Option<NativeConfigSelection>,
    ssh: Vec<SshProfile>,
    discovery: Option<HttpDiscoveryTrust>,
}
impl NativeGitAdmission {
    /// Select direct HTTPS with caller-trusted Git and TLS inputs. This does not
    /// authorize legacy checkout, SSH, redirects or recursive submodules.
    /// Native execution requires Git 2.53 or newer; the bounded recipe is
    /// integration-tested with 2.53.0, independently of legacy Git support.
    pub fn direct_https(
        binding: Arc<crate::HttpAdmission>,
        installation: GitInstallation,
        trust: TlsTrust,
    ) -> Self {
        Self {
            binding,
            installation,
            trust,
            selection: None,
            ssh: Vec::new(),
            discovery: None,
        }
    }
    /// Replace the entire SSH grant set with one explicitly selected endpoint,
    /// explicitly for fresh acquisition and warm refresh alike.
    /// Historical ambient SSH configuration is replaced by this caller choice,
    /// not captured or declared equivalent. Git origin witnesses still recheck.
    /// No agent, identity or SSH configuration is inherited.
    pub fn with_ssh_profile(mut self, ssh: SshProfile) -> Self {
        self.ssh = vec![ssh];
        self
    }
    /// Replace the exact SSH grant set. Duplicate endpoint grants refuse even
    /// when their keys differ; implicit-user/SCP ambiguity refuses at capture.
    pub fn with_ssh_profiles(mut self, profiles: Vec<SshProfile>) -> Result<Self, GitError> {
        for (index, profile) in profiles.iter().enumerate() {
            if profiles[..index]
                .iter()
                .any(|other| profile.same_endpoint(other))
            {
                return Err(unsupported("duplicate exact SSH endpoint grants"));
            }
        }
        self.ssh = profiles;
        Ok(self)
    }
    pub(crate) fn preflight_registry_auth(
        &self,
        auth: vibe_core::manifest::AuthKind,
    ) -> Result<(), GitError> {
        use vibe_core::manifest::AuthKind;
        match auth {
            AuthKind::None | AuthKind::TokenEnv => Ok(()),
            AuthKind::Ssh if !self.ssh.is_empty() => Ok(()),
            _ => Err(unsupported(
                "declared credential route requires a separate admitted profile",
            )),
        }
    }
    /// The exact immutable binding also used by raw HTTP and index clients.
    pub fn http_admission(&self) -> Arc<crate::HttpAdmission> {
        Arc::clone(&self.binding)
    }
    /// Add immutable configuration-selection inputs for local materialisation.
    pub fn with_config_selection(mut self, selection: NativeConfigSelection) -> Self {
        self.selection = Some(selection);
        self
    }
}
pub(super) fn unsupported(reason: &'static str) -> GitError {
    GitError::NativeContext { reason }
}
pub(super) fn unavailable() -> GitError {
    GitError::NativeUnavailable {
        operation: "legacy native execution or an unselected phase capability",
    }
}
fn io(source: std::io::Error) -> GitError {
    GitError::Io {
        cmd: "owned native Git context".into(),
        source,
    }
}
fn text(bytes: Vec<u8>) -> Result<String, GitError> {
    String::from_utf8(bytes)
        .map(|s| s.trim_end_matches(['\r', '\n']).to_owned())
        .map_err(|_| unsupported("non-UTF-8 Git metadata"))
}
fn full_oid(s: &str) -> bool {
    matches!(s.len(), 40 | 64) && s.bytes().all(|c| c.is_ascii_hexdigit())
}

// Neither credentials nor SSH trust/key material have a Debug surface.
#[derive(Clone, Copy)]
pub(in crate::git_backend::shell) enum NativeProtocol {
    Https,
    Ssh,
}
enum AdmittedRemote {
    Https {
        url: Url,
        authorization: Option<String>,
    },
    Ssh(ssh::Remote),
}
impl AdmittedRemote {
    fn address(&self) -> &str {
        match self {
            Self::Https { url, .. } => url.as_str(),
            Self::Ssh(remote) => &remote.address,
        }
    }
    fn protocol(&self) -> NativeProtocol {
        match self {
            Self::Https { .. } => NativeProtocol::Https,
            Self::Ssh(_) => NativeProtocol::Ssh,
        }
    }
}
impl NativeGitAdmission {
    // Metadata/address capture never grants use of an SSH credential.
    fn capture_address(
        &self,
        address: &str,
        reject_https_credentials: bool,
    ) -> Result<AdmittedRemote, GitError> {
        if address.chars().any(char::is_control) {
            return Err(unsupported("control character in endpoint"));
        }
        if address.starts_with("ssh://") || (!address.contains("://") && address.contains(':')) {
            let mut selected = None;
            for (index, profile) in self.ssh.iter().enumerate() {
                if let Some(remote) = profile.capture_candidate(address, &self.binding, index)?
                    && selected.replace(remote).is_some()
                {
                    return Err(unsupported("ambiguous exact SSH grants for this address"));
                }
            }
            return selected
                .map(AdmittedRemote::Ssh)
                .ok_or_else(|| unsupported("SSH endpoint requires one exact selected profile"));
        }
        let mut url =
            Url::parse(address).map_err(|_| unsupported("direct HTTPS endpoint required"))?;
        if url.scheme() != "https" || url.query().is_some() || url.fragment().is_some() {
            return Err(unsupported(
                "only direct HTTPS without query or fragment is supported",
            ));
        }
        let authorization = if !url.username().is_empty() || url.password().is_some() {
            if reject_https_credentials || url.username() != "x-access-token" {
                return Err(unsupported("unsupported credential route"));
            }
            let password = url
                .password()
                .ok_or_else(|| unsupported("token credential is incomplete"))?;
            // Existing token-env injection uses literal tokens. Encoded/ambiguous
            // userinfo is not silently reinterpreted as a different password.
            if password.contains('%') {
                return Err(unsupported("encoded token credential is unsupported"));
            }
            let client = reqwest::blocking::Client::new();
            let request = client
                .get("https://127.0.0.1/")
                .basic_auth("x-access-token", Some(password))
                .build()
                .map_err(|_| unsupported("invalid token credential"))?;
            Some(
                request.headers()[reqwest::header::AUTHORIZATION]
                    .to_str()
                    .map_err(|_| unsupported("invalid token header"))?
                    .to_owned(),
            )
        } else {
            None
        };
        url.set_username("")
            .map_err(|_| unsupported("invalid endpoint authority"))?;
        url.set_password(None)
            .map_err(|_| unsupported("invalid endpoint authority"))?;
        self.binding.admit_address(url.as_str())?;
        Ok(AdmittedRemote::Https { url, authorization })
    }
    fn capture_for_request(
        &self,
        address: &str,
        anonymous: bool,
        registry_auth: Option<vibe_core::manifest::AuthKind>,
    ) -> Result<AdmittedRemote, GitError> {
        use vibe_core::manifest::AuthKind;
        // Address/host checks remain shared. Registry auth narrows credential
        // use; it does not forbid credential-free HTTPS metadata reads.
        let reject_https_credentials =
            anonymous || registry_auth.is_some_and(|auth| !matches!(auth, AuthKind::TokenEnv));
        let remote = self.capture_address(address, reject_https_credentials)?;
        if matches!(remote, AdmittedRemote::Ssh(_))
            && (anonymous || registry_auth.is_some_and(|auth| !matches!(auth, AuthKind::Ssh)))
        {
            return Err(unsupported(
                "registry credential binding does not permit the selected SSH key or agent",
            ));
        }
        Ok(remote)
    }
}
pub(super) struct Preparation<'a> {
    pub reporter: &'a super::ShellGit,
    pub address: &'a str,
    pub reference: &'a str,
    pub reuse: Option<&'a Path>,
    pub objects: &'a Path,
    pub destination: &'a Path,
    pub anonymous: bool,
}
#[derive(Debug, Clone)]
struct OwnedRepository {
    config: Vec<u8>,
    origin: String,
    serving_endpoint: String,
    revision: GitPreparedRevision,
    context: Option<capture::CapturedContext>,
    source: Option<PathBuf>,
    source_before: Option<materialisation::Tree>,
    destination_bound: bool,
    children: Vec<gitlinks::Child>,
    gitlink: bool,
    child_layout: Option<capture::ManagedChild>,
}

/// Acquired native objects whose final destination has not yet been selected.
/// Only the registry can bind this consumed token after reading its actual
/// manifest. A bound PreparedInPlace never exposes this transition.
pub struct NativeSource {
    state: Arc<NativeState>,
    objects: PathBuf,
    revision: GitPreparedRevision,
}
impl NativeSource {
    pub(crate) fn revision(&self) -> &GitPreparedRevision {
        &self.revision
    }
    pub(crate) fn bind(self, destination: &Path) -> Result<GitPreparedRevision, GitError> {
        if !destination.is_absolute() {
            return Err(unsupported("native destination must be absolute"));
        }
        let mut owned = self
            .state
            .owned
            .lock()
            .map_err(|_| unsupported("owned source lock poisoned"))?;
        let value = owned
            .get_mut(&self.objects)
            .ok_or_else(|| unsupported("source token is no longer retained"))?;
        if value.destination_bound || value.revision != self.revision {
            return Err(unsupported(
                "native source token was already bound or changed",
            ));
        }
        let mut pending = vec![(self.objects.clone(), destination.to_path_buf())];
        while let Some((objects, destination)) = pending.pop() {
            let value = owned
                .get_mut(&objects)
                .ok_or_else(|| unsupported("retained child source is missing"))?;
            value.destination_bound = true;
            value.revision.destination = destination.clone();
            for child in &mut value.children {
                child.revision.destination = destination.join(&child.path);
                pending.push((child.objects.clone(), child.revision.destination.clone()));
            }
        }
        owned
            .get(&self.objects)
            .map(|value| value.revision.clone())
            .ok_or_else(|| unsupported("bound source is no longer retained"))
    }
}
#[derive(Debug)]
pub(super) struct NativeState {
    pub(super) profile: Arc<NativeGitAdmission>,
    owned: Mutex<HashMap<PathBuf, OwnedRepository>>,
}
impl NativeState {
    pub(in crate::git_backend::shell) fn acquire(
        self: &Arc<Self>,
        request: Preparation<'_>,
    ) -> Result<NativeSource, GitError> {
        if self.profile.selection.is_none() {
            return Err(unavailable());
        }
        let objects = request.objects.to_owned();
        let revision = self.prepare(request)?;
        self.owned
            .lock()
            .map_err(|_| unsupported("owned source lock poisoned"))?
            .get_mut(&objects)
            .ok_or_else(|| unsupported("acquired source was not retained"))?
            .destination_bound = false;
        Ok(NativeSource {
            state: self.clone(),
            objects,
            revision,
        })
    }
    pub(super) fn new(profile: Arc<NativeGitAdmission>) -> Self {
        Self {
            profile,
            owned: Mutex::new(HashMap::new()),
        }
    }
    pub(super) fn preflight(&self) -> Result<(), GitError> {
        let version = text(
            Context::new(&self.profile)?
                .run(&["--version"], None, None)?
                .stdout,
        )?;
        if !supported_version(&version) {
            return Err(unsupported("native preparation requires Git 2.53 or newer"));
        }
        Ok(())
    }
    pub(super) fn tags(
        &self,
        address: &str,
        anonymous: bool,
        registry_auth: Option<vibe_core::manifest::AuthKind>,
    ) -> Result<std::process::Output, GitError> {
        let remote = self
            .profile
            .capture_for_request(address, anonymous, registry_auth)?;
        let remote = self.profile.relocate(remote)?;
        Context::new(&self.profile)?.run(
            &["ls-remote", "--tags", "--", remote.address()],
            None,
            Some(&remote),
        )
    }
    fn with_owned<T>(
        &self,
        path: &Path,
        read: impl FnOnce(&OwnedRepository) -> Result<T, GitError>,
    ) -> Result<T, GitError> {
        let owned = self
            .owned
            .lock()
            .map_err(|_| unsupported("owned source lock poisoned"))?;
        let value = owned
            .get(path)
            .ok_or_else(|| unsupported("source is not owned by this preparation capability"))?;
        if std::fs::read(path.join("config")).map_err(io)? != value.config
            || path.join("objects/info/alternates").exists()
        {
            return Err(unsupported("prepared object context changed"));
        }
        read(value)
    }
    pub(in crate::git_backend::shell) fn serving_endpoint(
        &self,
        path: &Path,
    ) -> Result<String, GitError> {
        self.with_owned(path, |owned| Ok(owned.serving_endpoint.clone()))
    }
    pub(super) fn file(
        &self,
        objects: &Path,
        commit: &str,
        path: &str,
    ) -> Result<Vec<u8>, GitError> {
        self.with_owned(objects, |owned| {
            if commit != owned.revision.commit || !full_oid(commit) {
                return Err(unsupported("file read is not bound to the retained commit"));
            }
            let context = Context::new(&self.profile)?;
            let entry = context.run(&["ls-tree", "-z", commit, "--", path], Some(objects), None)?;
            if entry.stdout.is_empty() {
                return Err(GitError::FileNotFoundInRef {
                    url: owned.origin.clone(),
                    refname: owned.revision.reference.clone(),
                    path: path.into(),
                });
            }
            let entries: Vec<_> = entry
                .stdout
                .split(|byte| *byte == 0)
                .filter(|entry| !entry.is_empty())
                .collect();
            if entries.len() != 1
                || !(entries[0].starts_with(b"100644 blob ")
                    || entries[0].starts_with(b"100755 blob "))
                || entries[0].split(|byte| *byte == b'\t').nth(1) != Some(path.as_bytes())
            {
                return Err(unsupported(
                    "retained metadata path is not one exact regular file",
                ));
            }
            Context::new(&self.profile)?
                .run(
                    &["cat-file", "blob", &format!("{commit}:{path}")],
                    Some(objects),
                    None,
                )
                .map(|o| o.stdout)
        })
    }
    pub(super) fn set_origin(
        &self,
        objects: &Path,
        remote: &str,
        url: &str,
    ) -> Result<(), GitError> {
        self.with_owned(objects, |owned| {
            let requested = self.profile.capture_address(url, true)?;
            if remote != "origin" || requested.address() != owned.origin {
                return Err(unsupported("owned source origin cannot be rebound"));
            }
            Ok(())
        })
    }
}
fn supported_version(version: &str) -> bool {
    let Some(version) = version.strip_prefix("git version ") else {
        return false;
    };
    let mut fields = version.split('.');
    match (
        fields.next().and_then(|s| s.parse::<u32>().ok()),
        fields.next().and_then(|s| s.parse::<u32>().ok()),
    ) {
        (Some(major), Some(minor)) => (major, minor) >= (2, 53),
        _ => false,
    }
}
#[cfg(test)]
mod tests;

#[cfg(all(test, unix))]
mod ssh_tests;

#[cfg(all(test, unix))]
mod registry_auth_tests;
#[cfg(all(test, unix))]
mod relocation_tests;

#[cfg(all(test, unix))]
mod mixed_gitlink_tests;
