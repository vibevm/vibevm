//! Trusted library route for one authored, host-bound vibe-bin deployment.
//! The lifecycle deploy bridge alone owns C1 reservation and destination truth.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-SERVICE");

use crate::machine_submission::{HostPolicyFacts, PolicyError, PolicyOwner, TrustedPolicySource};
use std::sync::Arc;
use vibe_core::manifest::{DeployTarget, ExtensionHandler, MechanismRoutes, ProviderPin};
use vibe_extension_registry::{MechanismRegistry, resolve_mechanism};
use vibe_lifecycle::process::SelectedFileWitness;
use vibe_lifecycle::{
    ClientExecutables, DeployError, DeployExecution, DeploySelection, LifecycleLease,
    LifecycleLeaseError, SubmissionError, SubmissionNamespace, SubmissionReservations,
    SubmissionScope, deploy_state_home,
    retained_mechanism::{
        DeployColdHostFacts, DeployInspection, FixedDeployHostAdmission, FixedDeploySubmitError,
        ReadOnlyDeployAdmission, RetainedMechanismError, inspect_bound_vibe_bin_deploy,
        submit_fixed_bound_vibe_bin,
    },
};
use vibe_safefs::Project;

mod contract;
mod dispatch;
mod envelope;
pub use contract::{PUBLICATION_CAP, REQUEST_CAP};
pub use dispatch::{FixedDeployRoute, FixedDeployRouteError};
pub use envelope::{DOMAIN_OWNER, OPERATION_ID};
#[cfg(test)]
mod tests;

#[derive(Debug)]
pub enum FixedDeployHostError {
    ForeignRoot,
    ForeignScope,
    RequestedProfileMismatch,
    InvalidHost(String),
    Root(String),
    Lease(LifecycleLeaseError),
    Namespace(SubmissionError),
    Policy(PolicyError),
    Admission(RetainedMechanismError),
    Submit(FixedDeploySubmitError),
    Inspect(RetainedMechanismError),
    PostOwner {
        observation: DeployInspection,
        cause: Box<FixedDeployHostError>,
    },
}
impl std::fmt::Display for FixedDeployHostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fixed Deploy host refused: {self:?}")
    }
}
impl std::error::Error for FixedDeployHostError {}

type GenerationObserver = dyn Fn() -> Result<Option<String>, DeployError> + Send + Sync;

/// Constructed by a trusted embedding with three held physical roots and an
/// already issued C1 namespace. No wire value issues or extends authority.
pub struct FixedDeployHost {
    project: Arc<Project>,
    settings: Arc<Project>,
    home: Arc<Project>,
    root_tokens: [String; 3],
    namespace: SubmissionNamespace,
    scope: SubmissionScope,
    profile: String,
    target: DeployTarget,
    registry: Arc<MechanismRegistry>,
    routes: Arc<MechanismRoutes>,
    clients: ClientExecutables,
    project_identity: String,
    selected_generation_ref: Option<String>,
    binding_nonce: String,
    admission_epoch: u32,
    generation: Arc<GenerationObserver>,
    state_home: std::path::PathBuf,
    policy: Option<(Arc<PolicyOwner>, TrustedPolicySource, String, String)>,
}

pub(crate) fn deploy_policy_facts() -> HostPolicyFacts {
    HostPolicyFacts {
        approve_captured_script: false,
        approve_exact_local_install: false,
        allow_detach: false,
        allow_cancel_pre_dispatch: false,
        allow_cancel_deferred_boundary: false,
    }
}

/// Old-key recovery after the trusted host facts changed. This carrier has no
/// submit method and cannot turn retained pointer values into effect authority.
pub struct FixedDeployRecoveryHost {
    project: Arc<Project>,
    settings: Arc<Project>,
    home: Arc<Project>,
    root_tokens: [String; 3],
    namespace: SubmissionNamespace,
    scope: SubmissionScope,
    selected_generation_ref: Option<String>,
    binding_nonce: String,
    admission_epoch: u32,
}

impl FixedDeployRecoveryHost {
    /// Only a pointer owner that has checked the old binding under the
    /// incumbent lease may reconstitute these held facts.
    #[allow(
        clippy::too_many_arguments,
        reason = "recovery facts remain explicit and cannot be confused with current host policy"
    )]
    pub(crate) fn from_pinned(
        project: Arc<Project>,
        settings: Arc<Project>,
        home: Arc<Project>,
        root_tokens: [String; 3],
        namespace: SubmissionNamespace,
        scope: SubmissionScope,
        selected_generation_ref: Option<String>,
        binding_nonce: String,
        admission_epoch: u32,
    ) -> Result<Self, FixedDeployHostError> {
        let host = Self {
            project,
            settings,
            home,
            root_tokens,
            namespace,
            scope,
            selected_generation_ref,
            binding_nonce,
            admission_epoch,
        };
        host.check_roots(None)?;
        Ok(host)
    }

    pub fn inspect(&self, key: &str) -> Result<DeployInspection, FixedDeployHostError> {
        self.check_roots(None)?;
        let lease = LifecycleLease::acquire(self.project.root_path())
            .map_err(FixedDeployHostError::Lease)?;
        self.check_roots(Some(&lease))?;
        SubmissionReservations::validate_namespace(&lease, &self.namespace, &self.scope)
            .map_err(FixedDeployHostError::Namespace)?;
        let admission = ReadOnlyDeployAdmission::reissue(DeployColdHostFacts {
            host_context_id: &self.scope.context,
            instance_ref: &self.scope.target,
            selected_generation_ref: self.selected_generation_ref.as_deref(),
            principal_ref: &self.scope.principal,
            admission_epoch: self.admission_epoch,
            binding_nonce: &self.binding_nonce,
            allow_alias_resolution: false,
            target_abi: None,
            target_libc: None,
            project_root: self.project.root_path(),
            settings_root: self.settings.root_path(),
            user_home: self.home.root_path(),
        })
        .map_err(FixedDeployHostError::Admission)?;
        let result = inspect_bound_vibe_bin_deploy(
            &lease,
            self.namespace.id(),
            &self.scope,
            key,
            &admission,
        )
        .map_err(FixedDeployHostError::Inspect)?;
        self.check_roots(Some(&lease))
            .map_err(|cause| FixedDeployHostError::PostOwner {
                observation: result.clone(),
                cause: Box::new(cause),
            })?;
        Ok(result)
    }

    fn check_roots(&self, lease: Option<&LifecycleLease>) -> Result<(), FixedDeployHostError> {
        check_held_roots(
            &self.project,
            &self.settings,
            &self.home,
            &self.root_tokens,
            lease,
        )
    }
}

impl FixedDeployHost {
    #[allow(
        clippy::too_many_arguments,
        reason = "trusted fixed Deploy host facts stay explicitly named"
    )]
    pub fn from_trusted_host(
        project: Arc<Project>,
        settings: Arc<Project>,
        home: Arc<Project>,
        namespace: SubmissionNamespace,
        scope: SubmissionScope,
        profile: String,
        target: DeployTarget,
        registry: Arc<MechanismRegistry>,
        routes: Arc<MechanismRoutes>,
        clients: ClientExecutables,
        project_identity: String,
        selected_generation_ref: Option<String>,
        binding_nonce: String,
        admission_epoch: u32,
        generation: Arc<GenerationObserver>,
    ) -> Result<Self, FixedDeployHostError> {
        if profile.is_empty()
            || profile.trim() != profile
            || profile.chars().any(char::is_control)
            || project_identity.trim().is_empty()
            || admission_epoch == 0
            || binding_nonce.is_empty()
            || binding_nonce.trim() != binding_nonce
            || scope.context.trim().is_empty()
            || scope.target.trim().is_empty()
            || scope.principal.trim().is_empty()
        {
            return Err(FixedDeployHostError::InvalidHost(
                "invalid host identity or profile".into(),
            ));
        }
        if !routes.is_empty()
            || target.when.is_some()
            || target
                .depends_on
                .as_ref()
                .is_some_and(|rows| !rows.is_empty())
            || target.provider.is_some()
            || target.mechanism != "deploy:vibe-bin".parse().expect("fixed Deploy key")
        {
            return Err(FixedDeployHostError::InvalidHost(
                "only one unconditional independent builtin vibe-bin target is admitted".into(),
            ));
        }
        let expected: ProviderPin = "org.vibevm/vibe#vibe-bin"
            .parse()
            .expect("fixed builtin pin");
        let selected = resolve_mechanism(&registry, &target.mechanism, None, &routes)
            .map_err(|error| FixedDeployHostError::InvalidHost(error.to_string()))?;
        if selected.row().pin() != &expected
            || !matches!(selected.row().handler(), ExtensionHandler::Builtin { name } if name == "vibe-bin")
        {
            return Err(FixedDeployHostError::InvalidHost(
                "deploy:vibe-bin did not select incumbent builtin".into(),
            ));
        }
        let root_tokens = [
            project.root_correlation_token().map_err(root_error)?,
            settings.root_correlation_token().map_err(root_error)?,
            home.root_correlation_token().map_err(root_error)?,
        ];
        let state_home = deploy_state_home(settings.root_path());
        let host = Self {
            project,
            settings,
            home,
            root_tokens,
            namespace,
            scope,
            profile,
            target,
            registry,
            routes,
            clients,
            project_identity,
            selected_generation_ref,
            binding_nonce,
            admission_epoch,
            generation,
            state_home,
            policy: None,
        };
        let lease = host.lease()?;
        SubmissionReservations::validate_namespace(&lease, &host.namespace, &host.scope)
            .map_err(FixedDeployHostError::Namespace)?;
        Ok(host)
    }

    /// Attach the current durable policy observation to this effectful host.
    /// Every submit re-reads it under the same lease as C1 admission.
    pub(crate) fn bind_shared_policy(
        mut self,
        owner: Arc<PolicyOwner>,
        source: TrustedPolicySource,
        generation: String,
        manifest_sha256: String,
    ) -> Result<Self, FixedDeployHostError> {
        if owner.scope() != &self.scope {
            return Err(FixedDeployHostError::Policy(PolicyError::ForeignScope));
        }
        let lease = self.lease()?;
        let current = owner
            .current_under_lease(&lease)
            .map_err(FixedDeployHostError::Policy)?;
        if current.source() != &source
            || current.facts() != &deploy_policy_facts()
            || current.generation() != generation
            || self.current_manifest_hash()? != manifest_sha256
        {
            return Err(FixedDeployHostError::Policy(
                PolicyError::StartupDisagreement,
            ));
        }
        self.policy = Some((owner, source, generation, manifest_sha256));
        Ok(self)
    }

    fn facts(&self) -> DeployColdHostFacts<'_> {
        DeployColdHostFacts {
            host_context_id: &self.scope.context,
            instance_ref: &self.scope.target,
            selected_generation_ref: self.selected_generation_ref.as_deref(),
            principal_ref: &self.scope.principal,
            admission_epoch: self.admission_epoch,
            binding_nonce: &self.binding_nonce,
            allow_alias_resolution: false,
            target_abi: None,
            target_libc: None,
            project_root: self.project.root_path(),
            settings_root: self.settings.root_path(),
            user_home: self.home.root_path(),
        }
    }
    fn execution<'a>(
        &'a self,
        created_at: &'a str,
        selection: &'a DeploySelection,
    ) -> DeployExecution<'a> {
        DeployExecution {
            project_root: self.project.root_path(),
            targets: std::slice::from_ref(&self.target),
            selection,
            registry: &self.registry,
            routes: &self.routes,
            state_home: &self.state_home,
            settings_root: self.settings.root_path(),
            user_home: self.home.root_path(),
            clients: &self.clients,
            project: &self.project_identity,
            package: None,
            created_at,
        }
    }
    /// A caller that carries an explicit profile must name the one selected
    /// by this trusted host. A different name cannot reach C1 or the deploy
    /// bridge, even when it reuses an existing key.
    pub fn submit_with_requested_profile(
        &self,
        key: &str,
        requested_profile: &str,
    ) -> Result<DeployInspection, FixedDeployHostError> {
        self.check_requested_profile(requested_profile)?;
        self.submit_bound(key)
    }

    fn check_requested_profile(&self, requested_profile: &str) -> Result<(), FixedDeployHostError> {
        if requested_profile != self.profile {
            return Err(FixedDeployHostError::RequestedProfileMismatch);
        }
        Ok(())
    }

    pub fn submit(&self, key: &str) -> Result<DeployInspection, FixedDeployHostError> {
        self.submit_with_requested_profile(key, &self.profile)
    }

    fn submit_bound(&self, key: &str) -> Result<DeployInspection, FixedDeployHostError> {
        let lease = self.lease()?;
        SubmissionReservations::validate_namespace(&lease, &self.namespace, &self.scope)
            .map_err(FixedDeployHostError::Namespace)?;
        if let Some((owner, source, generation, manifest_sha256)) = &self.policy {
            let current = owner
                .current_under_lease(&lease)
                .map_err(FixedDeployHostError::Policy)?;
            if current.source() != source
                || current.facts() != &deploy_policy_facts()
                || current.generation() != generation
                || self.current_manifest_hash()? != *manifest_sha256
            {
                return Err(FixedDeployHostError::Policy(
                    PolicyError::StartupDisagreement,
                ));
            }
        }
        let selection = DeploySelection {
            profile: self.profile.clone(),
            targets: vec![self.target.id.clone()],
        };
        let created_at = vibe_core::timestamp::now_utc();
        let execution = self.execution(&created_at, &selection);
        let mut current = || (self.generation)();
        let permit = FixedDeployHostAdmission::issue_from_trusted_host(
            &lease,
            &self.namespace,
            &self.scope,
            key,
            &execution,
            self.facts(),
            &mut current,
        )
        .map_err(FixedDeployHostError::Admission)?;
        let result = submit_fixed_bound_vibe_bin(permit, key, &mut current)
            .map_err(FixedDeployHostError::Submit)?;
        self.check_roots(Some(&lease))
            .map_err(|cause| FixedDeployHostError::PostOwner {
                observation: result.clone(),
                cause: Box::new(cause),
            })?;
        Ok(result)
    }
    pub fn inspect(&self, key: &str) -> Result<DeployInspection, FixedDeployHostError> {
        let lease = self.lease()?;
        SubmissionReservations::validate_namespace(&lease, &self.namespace, &self.scope)
            .map_err(FixedDeployHostError::Namespace)?;
        let admission = ReadOnlyDeployAdmission::reissue(self.facts())
            .map_err(FixedDeployHostError::Admission)?;
        let result = inspect_bound_vibe_bin_deploy(
            &lease,
            self.namespace.id(),
            &self.scope,
            key,
            &admission,
        )
        .map_err(FixedDeployHostError::Inspect)?;
        self.check_roots(Some(&lease))
            .map_err(|cause| FixedDeployHostError::PostOwner {
                observation: result.clone(),
                cause: Box::new(cause),
            })?;
        Ok(result)
    }
    fn lease(&self) -> Result<LifecycleLease, FixedDeployHostError> {
        self.check_roots(None)?;
        let lease = LifecycleLease::acquire(self.project.root_path())
            .map_err(FixedDeployHostError::Lease)?;
        self.check_roots(Some(&lease))?;
        Ok(lease)
    }
    fn current_manifest_hash(&self) -> Result<String, FixedDeployHostError> {
        let bytes = self
            .project
            .read_file_bounded("vibe.toml", 1024 * 1024)
            .map_err(root_error)?
            .ok_or_else(|| FixedDeployHostError::InvalidHost("missing authored manifest".into()))?;
        Ok(SelectedFileWitness::from_bytes(bytes).sha256)
    }
    fn check_roots(&self, lease: Option<&LifecycleLease>) -> Result<(), FixedDeployHostError> {
        check_held_roots(
            &self.project,
            &self.settings,
            &self.home,
            &self.root_tokens,
            lease,
        )
    }
}

fn check_held_roots(
    project: &Project,
    settings: &Project,
    home: &Project,
    root_tokens: &[String; 3],
    lease: Option<&LifecycleLease>,
) -> Result<(), FixedDeployHostError> {
    for (root, token) in [
        (project, &root_tokens[0]),
        (settings, &root_tokens[1]),
        (home, &root_tokens[2]),
    ] {
        if root.root_correlation_token().map_err(root_error)? != *token
            || Project::open(root.root_path())
                .map_err(root_error)?
                .root_correlation_token()
                .map_err(root_error)?
                != *token
        {
            return Err(FixedDeployHostError::ForeignRoot);
        }
    }
    if let Some(lease) = lease {
        if lease.root() != project.root_path()
            || lease
                .publication_journal_project()
                .root_correlation_token()
                .map_err(root_error)?
                != root_tokens[0]
        {
            return Err(FixedDeployHostError::ForeignRoot);
        }
    }
    Ok(())
}
fn root_error(error: impl std::fmt::Display) -> FixedDeployHostError {
    FixedDeployHostError::Root(error.to_string())
}
