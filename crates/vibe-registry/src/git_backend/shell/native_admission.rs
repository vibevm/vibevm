//! Explicit direct HTTPS preparation; no ambient Git execution authority.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-PREPARATION");
use super::{GitError, GitPreparedRevision};
use reqwest::Url;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
mod capture;
mod context;
pub use capture::NativeConfigSelection;
mod materialisation;
pub use materialisation::NativeMaterialisation;
mod metadata;
mod preparation;
pub(super) use context::AdmittedCommand;
use context::Context;

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
/// Immutable HTTPS capability, distinct from HTTP-only admission.
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
        operation: "local materialisation (N2), SSH (N3), redirects and submodules (N4)",
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

// Not publicly constructible and never Debug: the credential is operation-local.
struct AdmittedRemote {
    url: Url,
    authorization: Option<String>,
}
impl NativeGitAdmission {
    fn capture(&self, address: &str, anonymous: bool) -> Result<AdmittedRemote, GitError> {
        if address.chars().any(char::is_control) {
            return Err(unsupported("control character in endpoint"));
        }
        let mut url =
            Url::parse(address).map_err(|_| unsupported("direct HTTPS endpoint required"))?;
        if url.scheme() != "https" || url.query().is_some() || url.fragment().is_some() {
            return Err(unsupported(
                "only direct HTTPS without query or fragment is supported",
            ));
        }
        let authorization = if !url.username().is_empty() || url.password().is_some() {
            if anonymous || url.username() != "x-access-token" {
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
        Ok(AdmittedRemote { url, authorization })
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
#[derive(Debug)]
struct OwnedRepository {
    config: Vec<u8>,
    origin: String,
    revision: GitPreparedRevision,
    context: Option<capture::CapturedContext>,
    source: Option<PathBuf>,
    source_before: Option<materialisation::Tree>,
    destination_bound: bool,
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
        value.destination_bound = true;
        value.revision.destination = destination.into();
        Ok(value.revision.clone())
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
    ) -> Result<std::process::Output, GitError> {
        let remote = self.profile.capture(address, anonymous)?;
        Context::new(&self.profile)?.run(
            &["ls-remote", "--tags", "--", remote.url.as_str()],
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
            let requested = self.profile.capture(url, true)?;
            if remote != "origin" || requested.url.as_str() != owned.origin {
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
