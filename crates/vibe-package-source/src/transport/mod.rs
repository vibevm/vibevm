//! Deferred installation discovery from an explicit invocation environment snapshot.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE");
use crate::operation::SourceOperation;
use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex, OnceLock},
};
use vibe_registry::git_backend::{
    deferred::{DeferredShellGit, ShellGitFactory, TransportRequirement},
    shell::native_admission::{
        GitInstallation, NativeConfigSelection, NativeGitAdmission, SshConfigSelection,
        SshConfiguration, SshConfigurationProvider, TlsTrust, TlsTrustProvider,
    },
};
use vibe_registry::{GitBackend, GitError, ShellGit};
mod installation;
fn unsupported(reason: &'static str) -> GitError {
    GitError::NativeContext { reason }
}
#[derive(Clone)]
pub(crate) struct TransportInputs {
    pub cwd: PathBuf,
    pub acquisition_root: PathBuf,
    pub environment: BTreeMap<OsString, OsString>,
}
impl std::fmt::Debug for TransportInputs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TransportInputs(..)")
    }
}
impl TransportInputs {
    fn variable(&self, key: &str) -> Option<&std::ffi::OsStr> {
        self.environment
            .get(std::ffi::OsStr::new(key))
            .map(OsString::as_os_str)
    }
    fn home(&self) -> Result<PathBuf, GitError> {
        self.variable("HOME")
            .or_else(|| self.variable("USERPROFILE"))
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .ok_or_else(|| {
                unsupported("selected native transport requires an absolute caller home")
            })
    }
    fn git_name(&self) -> OsString {
        self.variable("VIBE_GIT_BINARY")
            .unwrap_or(std::ffi::OsStr::new("git"))
            .to_owned()
    }
}
struct Factory {
    operation: SourceOperation,
    ssh: Arc<SelectedSsh>,
    tls: Arc<SelectedPem>,
}
impl Factory {
    fn native(&self) -> Result<ShellGit, GitError> {
        #[cfg(not(unix))]
        {
            return Err(unsupported(
                "controlled native preparation is not qualified on this platform; no legacy fallback",
            ));
        }
        let input = &self.operation.transport;
        let (binary, helpers) = installation::git(input)?;
        let mut selection = NativeConfigSelection::new(input.cwd.clone(), input.home()?)?;
        for (key, value) in &input.environment {
            let Some(key) = key.to_str() else { continue };
            if matches!(
                key,
                "HOME"
                    | "XDG_CONFIG_HOME"
                    | "GIT_CONFIG_SYSTEM"
                    | "GIT_CONFIG_GLOBAL"
                    | "GIT_CONFIG_NOSYSTEM"
                    | "GIT_ATTR_NOSYSTEM"
                    | "GIT_CONFIG_PARAMETERS"
                    | "GIT_CONFIG_COUNT"
                    | "GIT_CONFIG"
            ) || key.starts_with("GIT_CONFIG_KEY_")
                || key.starts_with("GIT_CONFIG_VALUE_")
            {
                selection = selection.with_variable(key, value.clone())?;
            }
        }
        let binding = self
            .operation
            .admission()
            .map_err(|_| unsupported("source policy could not be captured"))?;
        let trust = if input.variable("GIT_SSL_CAINFO").is_some() {
            TlsTrust::DeferredPem(self.tls.clone())
        } else {
            TlsTrust::System
        };
        let profile = NativeGitAdmission::direct_https(
            binding,
            GitInstallation::trusted(binary, helpers)?,
            trust,
        )
        .with_config_selection(selection)
        .with_ssh_configuration_provider(self.ssh.clone())
        .with_acquisition_root(input.acquisition_root.clone())?;
        Ok(ShellGit::new().with_native_admission(Arc::new(profile)))
    }
}
impl ShellGitFactory for Factory {
    fn requirement(&self) -> Result<TransportRequirement, GitError> {
        self.operation
            .controlled()
            .map(|value| {
                if value {
                    TransportRequirement::Controlled
                } else {
                    TransportRequirement::Incumbent
                }
            })
            .map_err(|_| unsupported("source policy could not be captured"))
    }
    fn preflight(&self, address: &str) -> Result<(), GitError> {
        if self.requirement()? == TransportRequirement::Incumbent {
            return Ok(());
        }
        let raw = address.strip_prefix("git+").unwrap_or(address);
        let binding = self
            .operation
            .admission()
            .map_err(|_| unsupported("source policy could not be captured"))?;
        if raw.starts_with("https://") {
            binding.check_address(raw)?;
            for name in [
                "HTTPS_PROXY",
                "https_proxy",
                "ALL_PROXY",
                "all_proxy",
                "GIT_PROXY_COMMAND",
                "GIT_SSL_CAPATH",
                "SSL_CERT_DIR",
                "GIT_SSL_NO_VERIFY",
            ] {
                if self
                    .operation
                    .transport
                    .variable(name)
                    .is_some_and(|value| !value.is_empty())
                {
                    return Err(unsupported(
                        "selected HTTPS proxy or TLS environment requires a separately qualified controlled route",
                    ));
                }
            }
            return Ok(());
        }
        if raw.starts_with("ssh://")
            || (!raw.contains("://") && raw.contains(':') && !Path::new(raw).is_absolute())
        {
            binding.check_git_host(raw)?;
            return self.ssh.configuration()?.preflight_address(raw, &binding);
        }
        if raw.starts_with("http://") {
            // Plain HTTP is the caller's chosen unprotected transport; it is
            // still admitted, redirect-bounded and proxy-free like HTTPS.
            binding.check_address(raw)?;
            for name in [
                "HTTP_PROXY",
                "http_proxy",
                "ALL_PROXY",
                "all_proxy",
                "GIT_PROXY_COMMAND",
            ] {
                if self
                    .operation
                    .transport
                    .variable(name)
                    .is_some_and(|value| !value.is_empty())
                {
                    return Err(unsupported(
                        "selected HTTP proxy environment requires a separately qualified controlled route",
                    ));
                }
            }
            return Ok(());
        }
        Err(unsupported(
            "selected source requires a separately qualified controlled transport",
        ))
    }
    fn open(&self, requirement: TransportRequirement) -> Result<ShellGit, GitError> {
        match requirement {
            TransportRequirement::Controlled => self.native(),
            TransportRequirement::Incumbent => {
                Ok(ShellGit::new().with_binary(installation::executable(
                    &self.operation.transport,
                    &self.operation.transport.git_name(),
                )?))
            }
        }
    }
}
impl SourceOperation {
    pub(crate) fn backend(&self) -> Arc<dyn GitBackend> {
        Arc::new(DeferredShellGit::new(Arc::new(Factory {
            operation: self.clone(),
            ssh: self.ssh.clone(),
            tls: self.tls.clone(),
        })))
    }
}
#[derive(Debug)]
pub(crate) struct SelectedSsh {
    input: Arc<TransportInputs>,
    value: OnceLock<Arc<SshConfiguration>>,
    gate: Mutex<()>,
}
impl SshConfigurationProvider for SelectedSsh {
    fn configuration(&self) -> Result<Arc<SshConfiguration>, GitError> {
        if self.value.get().is_none() {
            let _guard = self
                .gate
                .lock()
                .map_err(|_| unsupported("SSH input initialization lock poisoned"))?;
            if self.value.get().is_none() {
                let (executable, user, system) = installation::ssh(&self.input)?;
                let mut selection =
                    SshConfigSelection::new(executable, self.input.home()?, system)?
                        .with_acquisition_root(self.input.acquisition_root.clone())?;
                if let Some(user) = user {
                    selection = selection.with_user_config(user)?;
                }
                for (key, value) in &self.input.environment {
                    if let (Some(key), Some(value)) = (key.to_str(), value.to_str())
                        && key.bytes().enumerate().all(|(i, b)| {
                            b == b'_' || b.is_ascii_alphabetic() || (i > 0 && b.is_ascii_digit())
                        })
                    {
                        selection = selection.with_environment(key, value.into())?;
                    }
                }
                let _ = self.value.set(Arc::new(selection.capture()?));
            }
        }
        self.value
            .get()
            .cloned()
            .ok_or_else(|| unsupported("SSH capture produced no configuration"))
    }
}
#[derive(Debug)]
pub(crate) struct SelectedPem {
    input: Arc<TransportInputs>,
    value: OnceLock<Arc<Vec<u8>>>,
    gate: Mutex<()>,
}
impl TlsTrustProvider for SelectedPem {
    fn pem(&self) -> Result<Arc<Vec<u8>>, GitError> {
        if self.value.get().is_none() {
            let _guard = self
                .gate
                .lock()
                .map_err(|_| unsupported("TLS input initialization lock poisoned"))?;
            if self.value.get().is_none() {
                let raw = self
                    .input
                    .variable("GIT_SSL_CAINFO")
                    .ok_or_else(|| unsupported("selected Git CA file is absent"))?;
                let path = PathBuf::from(raw);
                let path = if path.is_absolute() {
                    path
                } else {
                    self.input.cwd.join(path)
                };
                let _ = self.value.set(Arc::new(installation::bounded_file(&path)?));
            }
        }
        self.value
            .get()
            .cloned()
            .ok_or_else(|| unsupported("TLS capture produced no certificate input"))
    }
}

impl SelectedSsh {
    pub(crate) fn new(input: Arc<TransportInputs>) -> Self {
        Self {
            input,
            value: OnceLock::new(),
            gate: Mutex::new(()),
        }
    }
}
impl SelectedPem {
    pub(crate) fn new(input: Arc<TransportInputs>) -> Self {
        Self {
            input,
            value: OnceLock::new(),
            gate: Mutex::new(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibe_core::GlobalRegistrySource;

    fn factory(root: &Path) -> Factory {
        factory_with(root, BTreeMap::new())
    }

    fn factory_with(root: &Path, environment: BTreeMap<OsString, OsString>) -> Factory {
        let file = root.join("registry.toml");
        std::fs::write(
            &file,
            "[source_policy]\nexclude_hosting=['host:blocked.invalid']\n",
        )
        .unwrap();
        let operation = SourceOperation::new(
            GlobalRegistrySource::deferred(Some(file), Default::default()),
            Default::default(),
            Default::default(),
            TransportInputs {
                cwd: root.into(),
                acquisition_root: root.join("must-not-exist"),
                environment,
            },
            None,
        )
        .unwrap();
        Factory {
            ssh: operation.ssh.clone(),
            tls: operation.tls.clone(),
            operation,
        }
    }

    #[test]
    #[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY-LAW")]
    fn excluded_plain_http_registry_reports_its_exclusion() {
        let root = tempfile::tempdir().unwrap();
        let factory = factory(root.path());
        assert_eq!(
            factory.requirement().unwrap(),
            TransportRequirement::Controlled
        );
        assert!(matches!(
            factory.preflight("http://blocked.invalid/org/repo.git"),
            Err(GitError::ContactRefused(
                vibe_registry::ContactRefusal::ExcludedHost { .. }
            ))
        ));
        // An admitted plain-HTTP address is a controlled route like HTTPS.
        factory
            .preflight("http://admitted.invalid/org/repo.git")
            .unwrap();
        // Other schemes remain outside the controlled transport.
        for refused in [
            "git://admitted.invalid/org/repo.git",
            "file:///srv/org/repo.git",
            "ftp://admitted.invalid/org/repo.git",
        ] {
            assert!(
                matches!(
                    factory.preflight(refused),
                    Err(GitError::NativeContext { .. })
                ),
                "{refused}"
            );
        }
        assert!(!root.path().join("must-not-exist").exists());
    }

    #[test]
    #[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY-LAW")]
    fn plain_http_controlled_route_refuses_a_selected_proxy() {
        let root = tempfile::tempdir().unwrap();
        for name in ["HTTP_PROXY", "http_proxy", "ALL_PROXY", "GIT_PROXY_COMMAND"] {
            let factory = factory_with(
                root.path(),
                BTreeMap::from([(OsString::from(name), OsString::from("http://127.0.0.9:1"))]),
            );
            assert!(
                matches!(
                    factory.preflight("http://admitted.invalid/org/repo.git"),
                    Err(GitError::NativeContext { .. })
                ),
                "{name}"
            );
            // An excluded host still reports its exclusion first.
            assert!(matches!(
                factory.preflight("http://blocked.invalid/org/repo.git"),
                Err(GitError::ContactRefused(
                    vibe_registry::ContactRefusal::ExcludedHost { .. }
                ))
            ));
        }
        assert!(!root.path().join("must-not-exist").exists());
    }
}
