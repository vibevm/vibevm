//! Lazy caller-selected inputs and operation-owned scratch.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE");
use super::*;
/// Explicit TLS trust input. Verification is always enabled.
#[derive(Debug)]
pub enum TlsTrust {
    /// Explicit file trust initialized only for an actual HTTPS request.
    DeferredPem(Arc<dyn TlsTrustProvider>),
    /// The trusted Git installation's system certificate store.
    System,
    /// Captured PEM certificate authorities, copied only into the operation home.
    Pem(Vec<u8>),
}
/// Demand supplier of explicit PEM trust, read only for an actual HTTPS request.
///
/// ```
/// use std::sync::Arc;
/// use vibe_registry::GitError;
/// use vibe_registry::git_backend::shell::native_admission::{TlsTrust, TlsTrustProvider};
///
/// #[derive(Debug)]
/// struct Captured(Arc<Vec<u8>>);
/// impl TlsTrustProvider for Captured {
///     fn pem(&self) -> Result<Arc<Vec<u8>>, GitError> {
///         Ok(self.0.clone())
///     }
/// }
///
/// let provider = Arc::new(Captured(Arc::new(b"-----BEGIN CERTIFICATE-----".to_vec())));
/// let trust = TlsTrust::DeferredPem(provider.clone());
/// assert!(matches!(trust, TlsTrust::DeferredPem(_)));
/// assert!(provider.pem().unwrap().starts_with(b"-----BEGIN"));
/// ```
pub trait TlsTrustProvider: Send + Sync + std::fmt::Debug {
    fn pem(&self) -> Result<Arc<Vec<u8>>, GitError>;
}
/// An invocation-owned supplier: capture only when an SSH address is selected.
///
/// ```
/// use std::sync::Arc;
/// use vibe_registry::GitError;
/// use vibe_registry::git_backend::shell::native_admission::{
///     SshConfiguration, SshConfigurationProvider,
/// };
///
/// // An operation that selected no SSH lane refuses on demand, typed.
/// #[derive(Debug)]
/// struct NoSshLane;
/// impl SshConfigurationProvider for NoSshLane {
///     fn configuration(&self) -> Result<Arc<SshConfiguration>, GitError> {
///         Err(GitError::NativeContext { reason: "no SSH configuration was selected" })
///     }
/// }
///
/// let provider: Arc<dyn SshConfigurationProvider> = Arc::new(NoSshLane);
/// assert!(provider.configuration().is_err());
/// ```
pub trait SshConfigurationProvider: Send + Sync + std::fmt::Debug {
    fn configuration(&self) -> Result<Arc<SshConfiguration>, GitError>;
}

impl NativeGitAdmission {
    /// Select a lazy captured SSH lane; HTTPS requests do not inspect it.
    pub fn with_ssh_configuration_provider(
        mut self,
        provider: Arc<dyn SshConfigurationProvider>,
    ) -> Self {
        self.ssh_provider = Some(provider);
        self
    }
    pub fn with_acquisition_root(mut self, root: PathBuf) -> Result<Self, GitError> {
        if !root.is_absolute() {
            return Err(unsupported("acquisition root must be absolute"));
        }
        self.acquisition_root = Some(root);
        Ok(self)
    }
    pub(in crate::git_backend::shell) fn temporary(
        &self,
        prefix: &str,
    ) -> Result<tempfile::TempDir, GitError> {
        let mut builder = tempfile::Builder::new();
        builder.prefix(prefix);
        match &self.acquisition_root {
            None => builder.tempdir(),
            Some(root) => {
                std::fs::create_dir_all(root).map_err(io)?;
                builder.tempdir_in(root)
            }
        }
        .map_err(io)
    }
    pub(super) fn selected_pem(&self) -> Result<Option<Arc<Vec<u8>>>, GitError> {
        match &self.trust {
            TlsTrust::System => Ok(None),
            TlsTrust::Pem(bytes) => Ok(Some(Arc::new(bytes.clone()))),
            TlsTrust::DeferredPem(provider) => provider.pem().map(Some),
        }
    }
    pub(super) fn selected_ssh_configuration(
        &self,
    ) -> Result<Option<Arc<SshConfiguration>>, GitError> {
        match (&self.ssh_configuration, &self.ssh_provider) {
            (Some(_), Some(_)) => Err(unsupported("eager and deferred SSH authorities overlap")),
            (Some(value), None) => Ok(Some(value.clone())),
            (None, Some(provider)) => provider.configuration().map(Some),
            (None, None) => Ok(None),
        }
    }
}
