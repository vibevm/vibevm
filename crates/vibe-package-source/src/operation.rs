//! One operation's deferred source policy and concrete transport inputs.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY");
use crate::{SourceStorage, transport::TransportInputs};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};
use vibe_core::{
    GlobalRegistrySource,
    manifest::{EffectiveSourcePolicy, SourcePolicyDecl},
};
use vibe_registry::{
    HttpAdmission, RegistryError, source_candidates::SourceChoiceMode,
    source_policy_provider::SourcePolicyProvider,
};
struct BoundPolicy {
    policy: Arc<EffectiveSourcePolicy>,
    admission: Arc<HttpAdmission>,
}
pub(crate) struct Policy {
    pub(crate) global: GlobalRegistrySource,
    project: SourcePolicyDecl,
    invocation: SourcePolicyDecl,
    bound: OnceLock<BoundPolicy>,
    gate: Mutex<()>,
}
fn invalid(reason: &str) -> RegistryError {
    RegistryError::MalformedMeta {
        path: PathBuf::from("registry.toml"),
        reason: reason.into(),
    }
}
impl Policy {
    fn new(
        global: GlobalRegistrySource,
        project: SourcePolicyDecl,
        invocation: SourcePolicyDecl,
    ) -> Result<Self, RegistryError> {
        EffectiveSourcePolicy::merge(&SourcePolicyDecl::default(), &project, &invocation)?;
        let value = Self {
            global,
            project,
            invocation,
            bound: OnceLock::new(),
            gate: Mutex::new(()),
        };
        if value.global.known().is_some() {
            value.resolve()?;
        }
        Ok(value)
    }
    fn resolve(&self) -> Result<&BoundPolicy, RegistryError> {
        if self.bound.get().is_none() {
            let _guard = self
                .gate
                .lock()
                .map_err(|_| invalid("source policy initialization lock poisoned"))?;
            if self.bound.get().is_none() {
                let user = self
                    .global
                    .policy()
                    .map_err(|_| invalid("global source policy is unavailable or malformed"))?;
                let policy = Arc::new(EffectiveSourcePolicy::merge(
                    &user,
                    &self.project,
                    &self.invocation,
                )?);
                let admission = Arc::new(HttpAdmission::new(policy.denied_host_keys())?);
                let _ = self.bound.set(BoundPolicy { policy, admission });
            }
        }
        self.bound
            .get()
            .ok_or_else(|| invalid("source policy capture produced no binding"))
    }
}
impl SourcePolicyProvider for Policy {
    fn source_choice_mode(&self) -> SourceChoiceMode {
        match self.bound.get() {
            None => SourceChoiceMode::Deferred,
            Some(value)
                if value.policy.has_preference() || !value.policy.denied_host_keys().is_empty() =>
            {
                SourceChoiceMode::CandidateRequired
            }
            Some(_) => SourceChoiceMode::LegacyKnown,
        }
    }
    fn policy(&self) -> Result<Arc<EffectiveSourcePolicy>, RegistryError> {
        Ok(self.resolve()?.policy.clone())
    }
    fn admission(&self) -> Result<Arc<HttpAdmission>, RegistryError> {
        Ok(self.resolve()?.admission.clone())
    }
}
/// Share this same operation context with package sources and application HTTP.
/// Construction validates supplied declaration data, not global files or tools.
///
/// ```
/// use vibe_core::{GlobalRegistryConfig, GlobalRegistrySource, manifest::Manifest};
/// use vibe_package_source::{PackageSourceOptions, SourceStorage, source_operation};
/// use vibe_registry::source_candidates::SourceChoiceMode;
///
/// let scratch = std::env::temp_dir().join("vibe-operation-doctest");
/// let options = PackageSourceOptions {
///     storage: Some(SourceStorage { acquisition_root: scratch.clone(), read_only_store: None }),
///     ..Default::default()
/// };
/// let global = GlobalRegistrySource::ready(GlobalRegistryConfig::default());
/// let operation = source_operation(&options, &Manifest::default(), &scratch, global).unwrap();
/// // A known, neutral policy keeps the incumbent source choice...
/// assert_eq!(operation.source_choice_mode(), SourceChoiceMode::LegacyKnown);
/// // ...while operation-owned storage still binds HTTP to the admission.
/// assert!(operation.http_admission().unwrap().is_some());
/// ```
#[derive(Clone)]
pub struct SourceOperation {
    pub(crate) policy: Arc<Policy>,
    pub(crate) transport: Arc<TransportInputs>,
    pub(crate) storage: Option<SourceStorage>,
    pub(crate) ssh: Arc<crate::transport::SelectedSsh>,
    pub(crate) tls: Arc<crate::transport::SelectedPem>,
}
impl std::fmt::Debug for SourceOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceOperation(..)")
    }
}
impl SourceOperation {
    pub(crate) fn new(
        global: GlobalRegistrySource,
        project: SourcePolicyDecl,
        invocation: SourcePolicyDecl,
        transport: TransportInputs,
        storage: Option<SourceStorage>,
    ) -> Result<Self, RegistryError> {
        let transport = Arc::new(transport);
        let ssh = Arc::new(crate::transport::SelectedSsh::new(transport.clone()));
        let tls = Arc::new(crate::transport::SelectedPem::new(transport.clone()));
        Ok(Self {
            ssh,
            tls,
            policy: Arc::new(Policy::new(global, project, invocation)?),
            transport,
            storage,
        })
    }
    pub fn admission(&self) -> Result<Arc<HttpAdmission>, RegistryError> {
        self.policy.admission()
    }
    /// Resolve contact policy only when a consumer is about to contact a source.
    /// Neutral/preference-only operations retain incumbent HTTP behavior.
    pub fn http_admission(&self) -> Result<Option<Arc<HttpAdmission>>, RegistryError> {
        let binding = self.admission()?;
        Ok(self.controlled()?.then_some(binding))
    }
    pub fn source_choice_mode(&self) -> SourceChoiceMode {
        self.policy.source_choice_mode()
    }
    pub fn http_client(&self, timeout: Duration) -> OperationHttpClient {
        OperationHttpClient {
            operation: self.clone(),
            timeout,
            client: OnceLock::new(),
            gate: Mutex::new(()),
        }
    }
    pub(crate) fn controlled(&self) -> Result<bool, RegistryError> {
        Ok(self.storage.is_some() || !self.policy.policy()?.denied_host_keys().is_empty())
    }
}
/// HTTP construction is delayed until an actual payload request, preserving
/// verified-store and preview no-contact paths. No second HTTP implementation.
///
/// ```
/// use std::time::Duration;
/// use vibe_core::{GlobalRegistryConfig, GlobalRegistrySource, manifest::Manifest};
/// use vibe_package_source::{PackageSourceOptions, SourceStorage, source_operation};
///
/// let scratch = std::env::temp_dir().join("vibe-http-client-doctest");
/// let options = PackageSourceOptions {
///     storage: Some(SourceStorage { acquisition_root: scratch.clone(), read_only_store: None }),
///     ..Default::default()
/// };
/// let global = GlobalRegistrySource::ready(GlobalRegistryConfig::default());
/// let operation = source_operation(&options, &Manifest::default(), &scratch, global).unwrap();
/// // No client exists until the first request; the admission is already bound.
/// let client = operation.http_client(Duration::from_secs(30));
/// assert!(client.admission().unwrap().check_address("https://example.invalid/pkg").is_ok());
/// let _same_operation = client.source_operation();
/// ```
pub struct OperationHttpClient {
    operation: SourceOperation,
    timeout: Duration,
    client: OnceLock<vibe_registry::http_admission::ManagedHttpClient>,
    gate: Mutex<()>,
}
impl std::fmt::Debug for OperationHttpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OperationHttpClient(..)")
    }
}
/// A policy refusal or an admitted transport failure, kept distinct.
///
/// ```
/// use std::error::Error;
/// use vibe_package_source::SourceHttpError;
/// use vibe_registry::RegistryError;
///
/// let err = SourceHttpError::Policy(RegistryError::MalformedMeta {
///     path: "registry.toml".into(),
///     reason: "global source policy is unavailable or malformed".into(),
/// });
/// assert!(err.to_string().contains("registry.toml"));
/// assert!(err.source().is_some());
/// ```
#[derive(Debug)]
pub enum SourceHttpError {
    Policy(RegistryError),
    Transport(vibe_registry::http_admission::HttpGetError),
}
impl std::fmt::Display for SourceHttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Policy(error) => error.fmt(f),
            Self::Transport(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for SourceHttpError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Policy(e) => Some(e),
            Self::Transport(e) => Some(e),
        }
    }
}
impl OperationHttpClient {
    pub fn source_operation(&self) -> &SourceOperation {
        &self.operation
    }
    pub fn admission(&self) -> Result<Arc<HttpAdmission>, RegistryError> {
        self.operation.admission()
    }
    pub fn get_anonymous(
        &self,
        address: &str,
        cancellation: &vibe_core::cancellation::CancellationToken,
    ) -> Result<vibe_registry::http_admission::ManagedResponse, SourceHttpError> {
        let binding = self
            .operation
            .admission()
            .map_err(SourceHttpError::Policy)?;
        // Factory does not hide policy/config failure behind a network miss.
        if self.client.get().is_none() {
            let _guard = self.gate.lock().map_err(|_| {
                SourceHttpError::Policy(invalid("HTTP policy initialization lock poisoned"))
            })?;
            if self.client.get().is_none() {
                let client = if self
                    .operation
                    .controlled()
                    .map_err(SourceHttpError::Policy)?
                {
                    vibe_registry::http_admission::ManagedHttpClient::bound(self.timeout, binding)
                } else {
                    vibe_registry::http_admission::ManagedHttpClient::legacy(self.timeout)
                }
                .map_err(|e| SourceHttpError::Transport(e.into()))?;
                let _ = self.client.set(client);
            }
        }
        self.client
            .get()
            .ok_or_else(|| {
                SourceHttpError::Policy(invalid("HTTP client initialization produced no client"))
            })?
            .get_anonymous(address, cancellation)
            .map_err(SourceHttpError::Transport)
    }
}
