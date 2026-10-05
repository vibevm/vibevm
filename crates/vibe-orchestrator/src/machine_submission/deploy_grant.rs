//! Durable fixed Deploy pointer. It carries old-key recovery identity, never
//! a serialized callback, approval, result or independent execution grant.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-FIXED-DEPLOY-REATTACH-DECISION"
);

use super::{PolicyError, PolicyOwner, TrustedPolicySource};
use crate::machine_operation::fixed_deploy::{
    FixedDeployHost, FixedDeployHostError, FixedDeployRecoveryHost, deploy_policy_facts,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use vibe_core::manifest::{DeployTarget, Manifest, MechanismRoutes};
use vibe_extension_registry::MechanismRegistry;
use vibe_lifecycle::process::SelectedFileWitness;
use vibe_lifecycle::retained_mechanism::DeployInspection;
use vibe_lifecycle::{
    ClientExecutables, DeployError, LifecycleLease, LifecycleLeaseError, SubmissionError,
    SubmissionNamespace, SubmissionReservations, SubmissionScope,
};
use vibe_safefs::Project;

const CAP: usize = 8192;
const MANIFEST_CAP: usize = 1024 * 1024;
const MAX_TTL_MS: u64 = 24 * 60 * 60 * 1000;
const LAW: &str =
    "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-FIXED-DEPLOY-REATTACH-DECISION";
type GenerationObserver = dyn Fn() -> Result<Option<String>, DeployError> + Send + Sync;
mod storage;

#[derive(Debug)]
pub enum DeployGrantError {
    InvalidAuthority(String),
    InvalidTtl,
    MissingPointer,
    AlreadyIssued,
    IssuanceIncomplete,
    ForeignBinding,
    InvalidPointer(String),
    UncertainPublication(String),
    Infrastructure(String),
    Lease(LifecycleLeaseError),
    Namespace(SubmissionError),
    Policy(PolicyError),
    Host(FixedDeployHostError),
}
impl std::fmt::Display for DeployGrantError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "violates REQ {LAW}: {self:?}; fix surface: trusted Deploy host binding and retained C1 evidence"
        )
    }
}
impl std::error::Error for DeployGrantError {}
fn infrastructure(error: impl std::fmt::Display) -> DeployGrantError {
    DeployGrantError::Infrastructure(format!("{error:#}"))
}
fn valid_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.trim() == value
        && !value.chars().any(char::is_control)
}
fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}
fn hash_fields(domain: &str, fields: &[&str]) -> String {
    let mut bytes = domain.as_bytes().to_vec();
    for field in fields {
        bytes.extend_from_slice(&(field.len() as u64).to_be_bytes());
        bytes.extend_from_slice(field.as_bytes());
    }
    SelectedFileWitness::from_bytes(bytes).sha256
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scope {
    context: String,
    target: String,
    principal: String,
}
impl From<&SubmissionScope> for Scope {
    fn from(scope: &SubmissionScope) -> Self {
        Self {
            context: scope.context.clone(),
            target: scope.target.clone(),
            principal: scope.principal.clone(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", deny_unknown_fields)]
enum GrantState {
    Issuing,
    Issued {
        namespace: String,
        expires_at_ms: u64,
        policy_generation: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pointer {
    schema: u32,
    root_tokens: [String; 3],
    scope: Scope,
    target_os: String,
    target_arch: String,
    profile: String,
    target_id: String,
    manifest_sha256: String,
    project_identity: String,
    selected_generation_ref: Option<String>,
    generation_observer_id: String,
    binding_nonce: String,
    admission_epoch: u32,
    policy_source: String,
    grant: GrantState,
}

/// A drifted pointer yields only this distinct inspection carrier. Callers
/// must pattern-match before an effectful submit is even available.
pub enum DeployGrantAttachment {
    Active(FixedDeployHost),
    RecoveryOnly(FixedDeployRecoveryHost),
}
impl DeployGrantAttachment {
    pub fn inspect(&self, key: &str) -> Result<DeployInspection, FixedDeployHostError> {
        match self {
            Self::Active(host) => host.inspect(key),
            Self::RecoveryOnly(host) => host.inspect(key),
        }
    }
}

/// Current trusted embedding. Root handles and generation callback remain
/// process-local; the pointer stores only checked correlation data.
pub struct DeployGrantService {
    project: Arc<Project>,
    settings: Arc<Project>,
    home: Arc<Project>,
    root_tokens: [String; 3],
    scope: SubmissionScope,
    path: String,
    profile: String,
    target: DeployTarget,
    registry: Arc<MechanismRegistry>,
    routes: Arc<MechanismRoutes>,
    clients: ClientExecutables,
    project_identity: String,
    selected_generation_ref: Option<String>,
    generation_observer_id: String,
    binding_nonce: String,
    admission_epoch: u32,
    generation: Arc<GenerationObserver>,
    policy_source: TrustedPolicySource,
}
impl DeployGrantService {
    #[allow(
        clippy::too_many_arguments,
        reason = "the trusted Deploy boundary must name every physical and authored input"
    )]
    pub fn from_trusted_host(
        project: Arc<Project>,
        settings: Arc<Project>,
        home: Arc<Project>,
        scope: SubmissionScope,
        profile: String,
        target: DeployTarget,
        registry: Arc<MechanismRegistry>,
        routes: Arc<MechanismRoutes>,
        clients: ClientExecutables,
        project_identity: String,
        selected_generation_ref: Option<String>,
        generation_observer_id: String,
        binding_nonce: String,
        admission_epoch: u32,
        generation: Arc<GenerationObserver>,
        policy_source: TrustedPolicySource,
    ) -> Result<Self, DeployGrantError> {
        for value in [
            &scope.context,
            &scope.target,
            &scope.principal,
            &profile,
            &target.id,
            &project_identity,
            &generation_observer_id,
            &binding_nonce,
            policy_source.identifier(),
        ] {
            if !valid_identity(value) {
                return Err(DeployGrantError::InvalidAuthority(
                    "empty, oversized or noncanonical trusted identity".into(),
                ));
            }
        }
        if admission_epoch == 0 {
            return Err(DeployGrantError::InvalidAuthority(
                "zero admission epoch".into(),
            ));
        }
        if selected_generation_ref
            .as_deref()
            .is_some_and(|value| !valid_identity(value))
        {
            return Err(DeployGrantError::InvalidAuthority(
                "invalid selected generation identity".into(),
            ));
        }
        let root_tokens = [
            project.root_correlation_token().map_err(infrastructure)?,
            settings.root_correlation_token().map_err(infrastructure)?,
            home.root_correlation_token().map_err(infrastructure)?,
        ];
        let location = hash_fields(
            "vibevm/cli-deploy-host/location/e1\0",
            &[
                &root_tokens[0],
                &scope.context,
                &scope.target,
                &scope.principal,
            ],
        );
        let service = Self {
            project,
            settings,
            home,
            root_tokens,
            scope,
            path: format!(".vibe/cli-deploy-host-grants/{}.json", &location[7..]),
            profile,
            target,
            registry,
            routes,
            clients,
            project_identity,
            selected_generation_ref,
            generation_observer_id,
            binding_nonce,
            admission_epoch,
            generation,
            policy_source,
        };
        service.check_roots(None)?;
        Ok(service)
    }

    /// A first issue durably fences the pointer before policy and C1 issuance.
    pub fn issue(&self, ttl_ms: u64) -> Result<FixedDeployHost, DeployGrantError> {
        if ttl_ms == 0 || ttl_ms > MAX_TTL_MS {
            return Err(DeployGrantError::InvalidTtl);
        }
        let lease = self.lease()?;
        if let Some(pointer) = self.read(&lease)? {
            self.validate_base(&pointer)?;
            return Err(match pointer.grant {
                GrantState::Issuing => DeployGrantError::IssuanceIncomplete,
                GrantState::Issued { .. } => DeployGrantError::AlreadyIssued,
            });
        }
        if SubmissionReservations::namespace_exists_for_scope(&lease, &self.scope)
            .map_err(DeployGrantError::Namespace)?
        {
            return Err(DeployGrantError::MissingPointer);
        }
        let manifest_sha256 = self.current_manifest()?;
        self.check_generation()?;
        let mut pointer = self.pointer(manifest_sha256, GrantState::Issuing);
        self.publish(&lease, &pointer)?;
        let policy = PolicyOwner::initialize_under_lease(
            &lease,
            &self.scope,
            self.policy_source.clone(),
            deploy_policy_facts(),
        )
        .map_err(DeployGrantError::Policy)?;
        let namespace = SubmissionReservations::issue_namespace(&lease, &self.scope, ttl_ms)
            .map_err(DeployGrantError::Namespace)?;
        let (expires_at_ms, _) =
            SubmissionReservations::validate_namespace(&lease, &namespace, &self.scope)
                .map_err(DeployGrantError::Namespace)?;
        pointer.grant = GrantState::Issued {
            namespace: namespace.id().into(),
            expires_at_ms,
            policy_generation: policy.generation().into(),
        };
        self.publish(&lease, &pointer)?;
        drop(lease);
        self.construct_active(pointer, namespace)
    }

    /// Lookup only. Changed current facts produce a separate inspection host;
    /// damaged pointer or C1 evidence never causes fresh issuance.
    pub fn reattach(&self) -> Result<DeployGrantAttachment, DeployGrantError> {
        let lease = self.lease()?;
        let pointer = self.read(&lease)?.ok_or(DeployGrantError::MissingPointer)?;
        let namespace = self.validate_namespace(&lease, &pointer)?;
        let current = self.current_matches(&lease, &pointer);
        drop(lease);
        if current {
            return self
                .construct_active(pointer, namespace)
                .map(DeployGrantAttachment::Active);
        }
        let host = FixedDeployRecoveryHost::from_pinned(
            self.project.clone(),
            self.settings.clone(),
            self.home.clone(),
            pointer.root_tokens.clone(),
            namespace,
            self.scope.clone(),
            pointer.selected_generation_ref.clone(),
            pointer.binding_nonce.clone(),
            pointer.admission_epoch,
        )
        .map_err(DeployGrantError::Host)?;
        let lease = self.lease()?;
        if self.read(&lease)?.as_ref() != Some(&pointer) {
            return Err(DeployGrantError::InvalidPointer(
                "binding changed during recovery construction".into(),
            ));
        }
        self.validate_namespace(&lease, &pointer)?;
        Ok(DeployGrantAttachment::RecoveryOnly(host))
    }

    fn construct_active(
        &self,
        pointer: Pointer,
        namespace: SubmissionNamespace,
    ) -> Result<FixedDeployHost, DeployGrantError> {
        let GrantState::Issued {
            policy_generation, ..
        } = &pointer.grant
        else {
            return Err(DeployGrantError::IssuanceIncomplete);
        };
        let owner = Arc::new(
            PolicyOwner::attach_readonly(self.project.clone(), self.scope.clone())
                .map_err(DeployGrantError::Policy)?,
        );
        let host = FixedDeployHost::from_trusted_host(
            self.project.clone(),
            self.settings.clone(),
            self.home.clone(),
            namespace,
            self.scope.clone(),
            self.profile.clone(),
            self.target.clone(),
            self.registry.clone(),
            self.routes.clone(),
            self.clients.clone(),
            self.project_identity.clone(),
            self.selected_generation_ref.clone(),
            self.binding_nonce.clone(),
            self.admission_epoch,
            self.generation.clone(),
        )
        .map_err(DeployGrantError::Host)?
        .bind_shared_policy(
            owner,
            self.policy_source.clone(),
            policy_generation.clone(),
            pointer.manifest_sha256.clone(),
        )
        .map_err(DeployGrantError::Host)?;
        let lease = self.lease()?;
        if self.read(&lease)?.as_ref() != Some(&pointer) {
            return Err(DeployGrantError::InvalidPointer(
                "binding changed during host construction".into(),
            ));
        }
        self.validate_namespace(&lease, &pointer)?;
        if !self.current_matches(&lease, &pointer) {
            return Err(DeployGrantError::ForeignBinding);
        }
        Ok(host)
    }

    fn current_matches(&self, lease: &LifecycleLease, pointer: &Pointer) -> bool {
        if self.current_manifest().ok().as_deref() != Some(&pointer.manifest_sha256)
            || self.profile != pointer.profile
            || self.target.id != pointer.target_id
            || self.project_identity != pointer.project_identity
            || self.selected_generation_ref != pointer.selected_generation_ref
            || self.generation_observer_id != pointer.generation_observer_id
            || self.binding_nonce != pointer.binding_nonce
            || self.admission_epoch != pointer.admission_epoch
            || self.policy_source.identifier() != pointer.policy_source
            || self.check_generation().is_err()
        {
            return false;
        }
        PolicyOwner::attach_readonly(self.project.clone(), self.scope.clone())
            .and_then(|owner| owner.current_under_lease(lease))
            .is_ok_and(|snapshot| {
                matches!(&pointer.grant, GrantState::Issued { policy_generation, .. }
                    if snapshot.generation() == policy_generation)
                    && snapshot.source() == &self.policy_source
                    && snapshot.facts() == &deploy_policy_facts()
            })
    }

    fn current_manifest(&self) -> Result<String, DeployGrantError> {
        self.check_roots(None)?;
        let bytes = self
            .project
            .read_file_bounded("vibe.toml", MANIFEST_CAP)
            .map_err(infrastructure)?
            .ok_or_else(|| {
                DeployGrantError::InvalidAuthority("missing authored manifest".into())
            })?;
        let source = std::str::from_utf8(&bytes)
            .map_err(|error| DeployGrantError::InvalidAuthority(error.to_string()))?;
        let manifest = Manifest::parse_str(source)
            .map_err(|error| DeployGrantError::InvalidAuthority(error.to_string()))?;
        let deploy = manifest.deploy.ok_or_else(|| {
            DeployGrantError::InvalidAuthority("missing authored Deploy section".into())
        })?;
        let selected = deploy
            .profiles
            .get(&self.profile)
            .ok_or_else(|| DeployGrantError::InvalidAuthority("profile is not authored".into()))?;
        if selected.targets != [self.target.id.clone()]
            || deploy
                .targets
                .iter()
                .find(|target| target.id == self.target.id)
                != Some(&self.target)
        {
            return Err(DeployGrantError::InvalidAuthority(
                "trusted target/profile differ from authored manifest".into(),
            ));
        }
        self.check_roots(None)?;
        Ok(SelectedFileWitness::from_bytes(bytes).sha256)
    }

    fn check_generation(&self) -> Result<(), DeployGrantError> {
        let current = (self.generation)()
            .map_err(|error| DeployGrantError::InvalidAuthority(error.to_string()))?;
        if current != self.selected_generation_ref {
            return Err(DeployGrantError::InvalidAuthority(
                "current generation differs from trusted selection".into(),
            ));
        }
        Ok(())
    }

    fn pointer(&self, manifest_sha256: String, grant: GrantState) -> Pointer {
        Pointer {
            schema: 1,
            root_tokens: self.root_tokens.clone(),
            scope: Scope::from(&self.scope),
            target_os: std::env::consts::OS.into(),
            target_arch: std::env::consts::ARCH.into(),
            profile: self.profile.clone(),
            target_id: self.target.id.clone(),
            manifest_sha256,
            project_identity: self.project_identity.clone(),
            selected_generation_ref: self.selected_generation_ref.clone(),
            generation_observer_id: self.generation_observer_id.clone(),
            binding_nonce: self.binding_nonce.clone(),
            admission_epoch: self.admission_epoch,
            policy_source: self.policy_source.identifier().into(),
            grant,
        }
    }
    fn validate_base(&self, pointer: &Pointer) -> Result<(), DeployGrantError> {
        if pointer.schema != 1
            || pointer.root_tokens != self.root_tokens
            || pointer.scope != Scope::from(&self.scope)
            || pointer.target_os != std::env::consts::OS
            || pointer.target_arch != std::env::consts::ARCH
            || pointer.admission_epoch == 0
            || !valid_identity(&pointer.profile)
            || !valid_identity(&pointer.target_id)
            || !valid_identity(&pointer.project_identity)
            || !valid_identity(&pointer.generation_observer_id)
            || !valid_identity(&pointer.binding_nonce)
            || !valid_identity(&pointer.policy_source)
            || !valid_sha256(&pointer.manifest_sha256)
            || pointer
                .selected_generation_ref
                .as_deref()
                .is_some_and(|value| !valid_identity(value))
        {
            return Err(DeployGrantError::ForeignBinding);
        }
        Ok(())
    }
    fn validate_namespace(
        &self,
        lease: &LifecycleLease,
        pointer: &Pointer,
    ) -> Result<SubmissionNamespace, DeployGrantError> {
        self.validate_base(pointer)?;
        let GrantState::Issued {
            namespace,
            expires_at_ms,
            policy_generation,
        } = &pointer.grant
        else {
            return Err(DeployGrantError::IssuanceIncomplete);
        };
        if !valid_identity(policy_generation) {
            return Err(DeployGrantError::InvalidPointer(
                "invalid policy generation".into(),
            ));
        }
        let namespace = SubmissionNamespace::from_host_token(namespace.clone())
            .map_err(DeployGrantError::Namespace)?;
        let (expiry, _) =
            SubmissionReservations::validate_namespace(lease, &namespace, &self.scope)
                .map_err(DeployGrantError::Namespace)?;
        if expiry != *expires_at_ms {
            return Err(DeployGrantError::InvalidPointer(
                "expiry differs from C1".into(),
            ));
        }
        Ok(namespace)
    }
}

#[cfg(test)]
mod tests;
