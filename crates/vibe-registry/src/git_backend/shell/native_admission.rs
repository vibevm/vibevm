//! Explicit direct HTTP(S)/SSH preparation; no ambient Git execution authority.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-PREPARATION");
use super::{GitError, GitPreparedRevision};
use reqwest::Url;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
mod capture;
mod ssh_config;
pub use ssh_config::{SshConfigSelection, SshConfiguration};
mod child_layout;
pub mod platform;
pub(super) mod ssh;
pub use ssh::{SshAuthentication, SshProfile};
mod context;
mod operation_inputs;
pub use capture::NativeConfigSelection;
pub use operation_inputs::{SshConfigurationProvider, TlsTrust, TlsTrustProvider};
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
    /// The child's whole `PATH`: the helper directory and, on Windows, the
    /// same installation's runtime directories (`platform::runtime_directories`).
    search_path: std::ffi::OsString,
    /// OS startup variables a Windows child keeps (`platform::platform_variables`).
    platform: Vec<(std::ffi::OsString, std::ffi::OsString)>,
}
impl GitInstallation {
    /// Capture explicitly trusted executable and builtin-helper locations.
    pub fn trusted(executable: PathBuf, helpers: PathBuf) -> Result<Self, GitError> {
        if !executable.is_absolute() || !helpers.is_absolute() || !executable.is_file() {
            return Err(unsupported(
                "trusted Git installation paths are unavailable",
            ));
        }
        let directories = platform::runtime_directories(&helpers, cfg!(windows), Path::is_dir);
        // Git for Windows 2.4x keeps `git-remote-https.exe` in `mingw64/bin`
        // (its `etc/libexec-moved.txt`), which Git finds on the search path.
        if !directories
            .iter()
            .any(|directory| platform::has_https_helper(directory))
        {
            return Err(unsupported(
                "trusted Git installation paths are unavailable",
            ));
        }
        let search_path = match directories.as_slice() {
            [only] => only.clone().into_os_string(),
            many => std::env::join_paths(many)
                .map_err(|_| unsupported("trusted Git installation paths cannot be searched"))?,
        };
        Ok(Self {
            executable,
            helpers,
            search_path,
            platform: Vec::new(),
        })
    }
    /// Keep the OS startup variables of the invocation's captured
    /// `environment` that a Windows child needs (`SystemRoot`, `TEMP`, ...).
    /// Elsewhere, and for every other name, nothing is kept: no ambient
    /// Git, SSH, proxy or credential input enters this way.
    pub fn with_platform_environment<'a>(
        mut self,
        environment: impl IntoIterator<Item = (&'a std::ffi::OsStr, &'a std::ffi::OsStr)>,
    ) -> Self {
        self.platform = platform::platform_variables(environment, cfg!(windows));
        self
    }
    /// Give an already cleared child this installation's search path,
    /// helper directory and platform variables.
    pub(super) fn prepare(&self, command: &mut std::process::Command) {
        command
            .env("PATH", &self.search_path)
            .env("GIT_EXEC_PATH", &self.helpers);
        for (key, value) in &self.platform {
            command.env(key, value);
        }
    }
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
    ssh_configuration: Option<Arc<SshConfiguration>>,
    ssh_provider: Option<Arc<dyn SshConfigurationProvider>>,
    acquisition_root: Option<PathBuf>,
    discovery: Option<HttpDiscoveryTrust>,
}
impl NativeGitAdmission {
    /// Select direct HTTPS with caller-trusted Git and TLS inputs. A configured
    /// plain `http://` endpoint is admitted under the same address, redirect,
    /// proxy and credential controls, without TLS and never with credentials.
    /// This does not authorize legacy checkout, SSH, redirects or recursive
    /// submodules.
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
            ssh_configuration: None,
            ssh_provider: None,
            acquisition_root: None,
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
    /// Bind audited ordinary SSH inputs for each source selected later by the
    /// existing registry/lock/gitlink owners. This creates no ambient cache.
    pub fn with_ssh_configuration(mut self, configuration: Arc<SshConfiguration>) -> Self {
        self.ssh_configuration = Some(configuration);
        self
    }
    pub(crate) fn preflight_registry_auth(
        &self,
        auth: vibe_core::manifest::AuthKind,
    ) -> Result<(), GitError> {
        use vibe_core::manifest::AuthKind;
        match auth {
            AuthKind::None | AuthKind::TokenEnv => Ok(()),
            AuthKind::Ssh
                if !self.ssh.is_empty()
                    || (self.ssh_configuration.is_some() || self.ssh_provider.is_some()) =>
            {
                Ok(())
            }
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
    /// Plain HTTP the caller configured; admitted like HTTPS but without TLS.
    Http,
    Https,
    Ssh,
    /// A repository on this machine's filesystem: no network, no host to
    /// admit or exclude, so source policy has nothing to decide.
    File,
}
mod address;
use address::AdmittedRemote;
pub use address::{LocalRepository, local_repository, local_repository_for};
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SourceAddress {
    WarmOrigin,
    Pinned,
}
pub(super) struct Preparation<'a> {
    pub source_address: SourceAddress,
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
#[cfg(all(test, unix))]
mod status_tests;

mod acquisition;
pub(super) use acquisition::acquire_source;
