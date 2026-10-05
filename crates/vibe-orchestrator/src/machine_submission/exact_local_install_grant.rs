//! Trusted, durable host binding for the exact local Install route.
//!
//! The embedding authenticates the scope and supplies a bound source port.
//! This service only issues or looks up a C1 namespace; approval remains with
//! the owner-retained question and answer path.

use super::{
    ExactLocalInstallDecisionError, ExactLocalInstallDecisionHost, HostPolicyFacts, PolicyError,
    PolicyOwner, TrustedPolicySource,
};
use crate::ports::BoundOnePackageSource;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use vibe_core::manifest::TargetOs;
use vibe_lifecycle::process::SelectedFileWitness;
use vibe_lifecycle::{
    LifecycleLease, LifecycleLeaseError, SubmissionError, SubmissionNamespace,
    SubmissionReservations, SubmissionScope,
};
use vibe_safefs::{DirectoryDurability, DurableWrite, Project, PublishStage};

const CAP: usize = 4096;
const MAX_TTL_MS: u64 = 24 * 60 * 60 * 1000;

#[derive(Debug)]
pub enum ExactLocalInstallGrantError {
    InvalidAuthority,
    InvalidTtl,
    MissingPointer,
    AlreadyIssued,
    IssuanceIncomplete,
    ForeignBinding,
    SourceUnavailable(String),
    InvalidPointer(String),
    UncertainPublication(String),
    Infrastructure(String),
    Lease(LifecycleLeaseError),
    Namespace(SubmissionError),
    Policy(PolicyError),
    Host(ExactLocalInstallDecisionError),
}
impl std::fmt::Display for ExactLocalInstallGrantError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "exact local Install host grant: {self:?}")
    }
}
impl std::error::Error for ExactLocalInstallGrantError {}
fn infrastructure(error: impl std::fmt::Display) -> ExactLocalInstallGrantError {
    ExactLocalInstallGrantError::Infrastructure(format!("{error:#}"))
}
fn source_unavailable(error: impl std::fmt::Display) -> ExactLocalInstallGrantError {
    ExactLocalInstallGrantError::SourceUnavailable(format!("{error:#}"))
}
fn digest(bytes: Vec<u8>) -> String {
    SelectedFileWitness::from_bytes(bytes).sha256
}
fn frame(out: &mut Vec<u8>, value: &[u8]) {
    out.extend_from_slice(&(value.len() as u64).to_be_bytes());
    out.extend_from_slice(value);
}
fn valid_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scope {
    context: String,
    target: String,
    principal: String,
}
impl From<&SubmissionScope> for Scope {
    fn from(value: &SubmissionScope) -> Self {
        Self {
            context: value.context.clone(),
            target: value.target.clone(),
            principal: value.principal.clone(),
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
    root_token: String,
    scope: Scope,
    target_os: String,
    policy_source: String,
    source_id: String,
    grant: GrantState,
}

/// A trusted route composition. The namespace is data only until C1 validates
/// it against the retained scope and current lease.
pub struct ExactLocalInstallGrant {
    host: Arc<ExactLocalInstallDecisionHost>,
    namespace: SubmissionNamespace,
}
impl ExactLocalInstallGrant {
    pub fn into_parts(self) -> (Arc<ExactLocalInstallDecisionHost>, SubmissionNamespace) {
        (self.host, self.namespace)
    }
    pub fn host(&self) -> &Arc<ExactLocalInstallDecisionHost> {
        &self.host
    }
    pub fn namespace(&self) -> &SubmissionNamespace {
        &self.namespace
    }
}

/// Explicit, trusted inputs. The source port is supplied by the embedding,
/// not built from wire fields; its complete witness is pinned at construction.
pub struct ExactLocalInstallGrantService {
    project: Arc<Project>,
    root_token: String,
    scope: SubmissionScope,
    path: String,
    target_os: TargetOs,
    policy_source: TrustedPolicySource,
    facts: HostPolicyFacts,
    source: Arc<dyn BoundOnePackageSource>,
    source_id: String,
}
impl ExactLocalInstallGrantService {
    pub fn from_trusted_host(
        project: Arc<Project>,
        scope: SubmissionScope,
        target_os: TargetOs,
        policy_source: TrustedPolicySource,
        facts: HostPolicyFacts,
        source: Arc<dyn BoundOnePackageSource>,
    ) -> Result<Self, ExactLocalInstallGrantError> {
        if !valid_identity(&scope.context)
            || !valid_identity(&scope.target)
            || !valid_identity(&scope.principal)
            || facts.approve_captured_script
            || facts.allow_detach
            || facts.allow_cancel_pre_dispatch
            || facts.allow_cancel_deferred_boundary
        {
            return Err(ExactLocalInstallGrantError::InvalidAuthority);
        }
        let root_token = project.root_correlation_token().map_err(infrastructure)?;
        let mut location = b"vibevm/exact-local-install-grant/location/e1\0".to_vec();
        for value in [&scope.context, &scope.target, &scope.principal] {
            frame(&mut location, value.as_bytes());
        }
        let path = format!(
            ".vibe/exact-local-install-grants/{}.json",
            &digest(location)[7..]
        );
        let source_id = Self::capture_source_id(source.as_ref())?;
        let service = Self {
            project,
            root_token,
            scope,
            path,
            target_os,
            policy_source,
            facts,
            source,
            source_id,
        };
        service.check_root(None)?;
        Ok(service)
    }

    pub fn issue(
        &self,
        ttl_ms: u64,
    ) -> Result<ExactLocalInstallGrant, ExactLocalInstallGrantError> {
        if ttl_ms == 0 || ttl_ms > MAX_TTL_MS {
            return Err(ExactLocalInstallGrantError::InvalidTtl);
        }
        let lease = self.lease()?;
        self.check_source()?;
        if let Some(pointer) = self.read(&lease)? {
            self.validate_binding(&pointer)?;
            return Err(match pointer.grant {
                GrantState::Issuing => ExactLocalInstallGrantError::IssuanceIncomplete,
                GrantState::Issued { .. } => ExactLocalInstallGrantError::AlreadyIssued,
            });
        }
        if SubmissionReservations::namespace_exists_for_scope(&lease, &self.scope)
            .map_err(ExactLocalInstallGrantError::Namespace)?
        {
            return Err(ExactLocalInstallGrantError::MissingPointer);
        }
        let mut pointer = self.pointer(GrantState::Issuing);
        self.publish(&lease, &pointer)?;
        let snapshot = PolicyOwner::initialize_under_lease(
            &lease,
            &self.scope,
            self.policy_source.clone(),
            self.facts.clone(),
        )
        .map_err(ExactLocalInstallGrantError::Policy)?;
        self.check_source()?;
        let namespace = SubmissionReservations::issue_namespace(&lease, &self.scope, ttl_ms)
            .map_err(ExactLocalInstallGrantError::Namespace)?;
        let (expires_at_ms, _) =
            SubmissionReservations::validate_namespace(&lease, &namespace, &self.scope)
                .map_err(ExactLocalInstallGrantError::Namespace)?;
        pointer.grant = GrantState::Issued {
            namespace: namespace.id().into(),
            expires_at_ms,
            policy_generation: snapshot.generation().into(),
        };
        self.publish(&lease, &pointer)?;
        let owner = self.current_owner(&lease, &pointer)?;
        drop(lease);
        self.construct_checked(pointer, namespace, owner)
    }

    /// Lookup only. Retired and expired namespaces remain readable for C1/C2
    /// result inspection; C1 itself refuses a fresh key in either state.
    /// A retained source port can restore cold inspection after content drift.
    /// Reconstructing a strict pinned source port after process restart may
    /// itself require the original tree and is outside this service.
    pub fn reattach(&self) -> Result<ExactLocalInstallGrant, ExactLocalInstallGrantError> {
        let lease = self.lease()?;
        let pointer = self
            .read(&lease)?
            .ok_or(ExactLocalInstallGrantError::MissingPointer)?;
        let namespace = self.validate_namespace(&lease, &pointer)?;
        let owner = self.current_owner(&lease, &pointer)?;
        drop(lease);
        self.construct_checked(pointer, namespace, owner)
    }

    fn construct_checked(
        &self,
        pointer: Pointer,
        namespace: SubmissionNamespace,
        owner: Arc<PolicyOwner>,
    ) -> Result<ExactLocalInstallGrant, ExactLocalInstallGrantError> {
        let host = Arc::new(
            ExactLocalInstallDecisionHost::from_host(
                self.project.root_path().into(),
                owner.clone(),
                self.source.clone(),
                self.target_os,
            )
            .map_err(ExactLocalInstallGrantError::Host)?,
        );
        if host.readonly_scope() != &self.scope {
            return Err(ExactLocalInstallGrantError::ForeignBinding);
        }
        let lease = self.lease()?;
        if self.read(&lease)?.as_ref() != Some(&pointer) {
            return Err(ExactLocalInstallGrantError::InvalidPointer(
                "binding changed during construction".into(),
            ));
        }
        self.validate_namespace(&lease, &pointer)?;
        self.validate_current_policy(&lease, &pointer, &owner)?;
        Ok(ExactLocalInstallGrant { host, namespace })
    }

    fn capture_source_id(
        source: &dyn BoundOnePackageSource,
    ) -> Result<String, ExactLocalInstallGrantError> {
        let witness = source.source_witness();
        witness.validate().map_err(source_unavailable)?;
        let physical = Project::open(&witness.root)
            .map_err(source_unavailable)?
            .root_correlation_token()
            .map_err(source_unavailable)?;
        let mut bytes = b"vibevm/exact-local-install-grant/source/e1\0".to_vec();
        frame(&mut bytes, physical.as_bytes());
        frame(&mut bytes, witness.root.as_os_str().as_encoded_bytes());
        frame(&mut bytes, witness.package_pin.to_string().as_bytes());
        frame(&mut bytes, witness.tree_sha256.as_bytes());
        for file in &witness.files {
            frame(&mut bytes, file.relative_path.as_bytes());
            frame(&mut bytes, &file.bytes.to_be_bytes());
            frame(&mut bytes, file.sha256.as_bytes());
        }
        Ok(digest(bytes))
    }
    fn check_source(&self) -> Result<(), ExactLocalInstallGrantError> {
        if Self::capture_source_id(self.source.as_ref())? != self.source_id {
            return Err(ExactLocalInstallGrantError::ForeignBinding);
        }
        self.source
            .recheck_source_binding(self.source.source_witness())
            .map_err(source_unavailable)?;
        Ok(())
    }
    fn pointer(&self, grant: GrantState) -> Pointer {
        Pointer {
            schema: 1,
            root_token: self.root_token.clone(),
            scope: Scope::from(&self.scope),
            target_os: self.target_os.as_str().into(),
            policy_source: self.policy_source.identifier().into(),
            source_id: self.source_id.clone(),
            grant,
        }
    }
    fn validate_binding(&self, pointer: &Pointer) -> Result<(), ExactLocalInstallGrantError> {
        if pointer.schema != 1 {
            return Err(ExactLocalInstallGrantError::InvalidPointer(
                "unsupported version".into(),
            ));
        }
        if pointer.root_token != self.root_token
            || pointer.scope != Scope::from(&self.scope)
            || pointer.target_os != self.target_os.as_str()
            || pointer.policy_source != self.policy_source.identifier()
            || pointer.source_id != self.source_id
        {
            return Err(ExactLocalInstallGrantError::ForeignBinding);
        }
        Ok(())
    }
    fn validate_namespace(
        &self,
        lease: &LifecycleLease,
        pointer: &Pointer,
    ) -> Result<SubmissionNamespace, ExactLocalInstallGrantError> {
        self.validate_binding(pointer)?;
        let GrantState::Issued {
            namespace,
            expires_at_ms,
            ..
        } = &pointer.grant
        else {
            return Err(ExactLocalInstallGrantError::IssuanceIncomplete);
        };
        let namespace = SubmissionNamespace::from_host_token(namespace.clone())
            .map_err(ExactLocalInstallGrantError::Namespace)?;
        let (expiry, _) =
            SubmissionReservations::validate_namespace(lease, &namespace, &self.scope)
                .map_err(ExactLocalInstallGrantError::Namespace)?;
        if expiry != *expires_at_ms {
            return Err(ExactLocalInstallGrantError::InvalidPointer(
                "expiry differs from C1".into(),
            ));
        }
        Ok(namespace)
    }
    fn current_owner(
        &self,
        lease: &LifecycleLease,
        pointer: &Pointer,
    ) -> Result<Arc<PolicyOwner>, ExactLocalInstallGrantError> {
        let owner = Arc::new(
            PolicyOwner::attach_readonly(self.project.clone(), self.scope.clone())
                .map_err(ExactLocalInstallGrantError::Policy)?,
        );
        self.validate_current_policy(lease, pointer, &owner)?;
        Ok(owner)
    }
    fn validate_current_policy(
        &self,
        lease: &LifecycleLease,
        pointer: &Pointer,
        owner: &PolicyOwner,
    ) -> Result<(), ExactLocalInstallGrantError> {
        let snapshot = owner
            .current_under_lease(lease)
            .map_err(ExactLocalInstallGrantError::Policy)?;
        let GrantState::Issued {
            policy_generation, ..
        } = &pointer.grant
        else {
            return Err(ExactLocalInstallGrantError::IssuanceIncomplete);
        };
        if snapshot.generation() != policy_generation
            || snapshot.source() != &self.policy_source
            || snapshot.facts() != &self.facts
        {
            return Err(ExactLocalInstallGrantError::Policy(
                PolicyError::StartupDisagreement,
            ));
        }
        Ok(())
    }
    fn lease(&self) -> Result<LifecycleLease, ExactLocalInstallGrantError> {
        self.check_root(None)?;
        let lease = LifecycleLease::acquire(self.project.root_path())
            .map_err(ExactLocalInstallGrantError::Lease)?;
        self.check_root(Some(&lease))?;
        Ok(lease)
    }
    fn check_root(
        &self,
        lease: Option<&LifecycleLease>,
    ) -> Result<(), ExactLocalInstallGrantError> {
        let reopened = Project::open(self.project.root_path()).map_err(infrastructure)?;
        if reopened.root_correlation_token().map_err(infrastructure)? != self.root_token
            || self
                .project
                .root_correlation_token()
                .map_err(infrastructure)?
                != self.root_token
        {
            return Err(ExactLocalInstallGrantError::ForeignBinding);
        }
        if let Some(lease) = lease {
            if lease.root() != self.project.root_path()
                || lease
                    .publication_journal_project()
                    .root_correlation_token()
                    .map_err(infrastructure)?
                    != self.root_token
            {
                return Err(ExactLocalInstallGrantError::ForeignBinding);
            }
        }
        Ok(())
    }
    fn read(&self, lease: &LifecycleLease) -> Result<Option<Pointer>, ExactLocalInstallGrantError> {
        self.check_root(Some(lease))?;
        let bytes = lease
            .publication_journal_project()
            .read_file_bounded(&self.path, CAP)
            .map_err(infrastructure)?;
        self.check_root(Some(lease))?;
        bytes
            .map(|bytes| {
                serde_json::from_slice(&bytes)
                    .map_err(|error| ExactLocalInstallGrantError::InvalidPointer(error.to_string()))
            })
            .transpose()
    }
    fn publish(
        &self,
        lease: &LifecycleLease,
        pointer: &Pointer,
    ) -> Result<(), ExactLocalInstallGrantError> {
        self.validate_binding(pointer)?;
        let bytes = serde_json::to_vec(pointer).map_err(infrastructure)?;
        if bytes.len() > CAP {
            return Err(ExactLocalInstallGrantError::InvalidPointer(
                "candidate exceeds bound".into(),
            ));
        }
        self.check_root(Some(lease))?;
        let project = lease.publication_journal_project();
        let pinned = project.root_dir().map_err(infrastructure)?;
        let write = project
            .write_atomic_durable_in(&pinned, &self.path, &bytes)
            .map_err(|error| {
                let possibly = error.stage == PublishStage::PossiblyPublished;
                let reason = format!("{:#}", error.into_report());
                if possibly {
                    ExactLocalInstallGrantError::UncertainPublication(reason)
                } else {
                    ExactLocalInstallGrantError::Infrastructure(reason)
                }
            })?;
        let parent = self.project.root_path().join(&self.path);
        let parent = parent
            .parent()
            .ok_or_else(|| ExactLocalInstallGrantError::InvalidPointer("missing parent".into()))?;
        require_durable(&write, parent)?;
        let readback = project
            .read_file_bounded(&self.path, CAP)
            .map_err(infrastructure)?;
        self.check_root(Some(lease))?;
        if readback.as_deref() != Some(bytes.as_slice()) {
            return Err(ExactLocalInstallGrantError::UncertainPublication(
                "published readback differs".into(),
            ));
        }
        Ok(())
    }
}

fn require_durable(
    write: &DurableWrite,
    parent: &std::path::Path,
) -> Result<(), ExactLocalInstallGrantError> {
    let uncertain = || {
        ExactLocalInstallGrantError::UncertainPublication(
            "incomplete or unsupported durable directory checkpoints".into(),
        )
    };
    let mut expected = Vec::new();
    for directory in &write.published.created_directories {
        expected.push(directory.parent().ok_or_else(uncertain)?);
    }
    expected.push(parent);
    if !write.file_synced
        || write.parent != DirectoryDurability::Synced
        || write.directory_syncs.len() != expected.len()
        || write
            .directory_syncs
            .iter()
            .zip(expected)
            .any(|(sync, path)| {
                sync.directory != path || sync.durability != DirectoryDurability::Synced
            })
    {
        return Err(uncertain());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
