//! Host-owned fixed Validate authority binding, separate from operation truth.
//!
//! INVARIANT: C1 alone issues namespaces. A durable Issuing fence precedes
//! issuance; any existing pointer or C1 scope row prevents another issue.
//! Reattach is lookup only, including for retired reserved-key recovery.
//! The embedding authenticates root/target/principal/source and assigns outer
//! connection IDs in a separate namespace from the `cli-host:e1:` inner scope.
//! This library does not authenticate an OS user. Cooperating writers must use
//! LifecycleLease; root identity ABA and deletion/rollback of all host records
//! are outside the incumbent same-host filesystem guarantee.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-CLI-HOST-REATTACH");

use super::{
    FixedDecisionError, FixedLifecycleDecisionHost, HostPolicyFacts, PolicyError, PolicyOwner,
    TrustedPolicySource,
};
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
const LAW: &str = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-CLI-HOST-REATTACH";

/// Refusals preserve C1 and domain evidence. Reconcile host records at the
/// trusted embedding; a refusal never authorizes automatic reissuance.
///
/// ```
/// use vibe_orchestrator::machine_submission::cli_grant::CliGrantError;
/// assert!(CliGrantError::MissingPointer.to_string().contains("MACHINE-CLI-HOST-REATTACH"));
/// ```
#[derive(Debug)]
pub enum CliGrantError {
    InvalidAuthority,
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
    Host(FixedDecisionError),
}
impl std::fmt::Display for CliGrantError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "violates REQ {LAW}: {self:?}; fix surface: trusted host grant binding and retained C1 evidence"
        )
    }
}
impl std::error::Error for CliGrantError {}
fn infrastructure(error: impl std::fmt::Display) -> CliGrantError {
    CliGrantError::Infrastructure(format!("{error:#}"))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scope {
    context: String,
    target: String,
    principal: String,
}
impl Scope {
    fn from_host(scope: &SubmissionScope) -> Self {
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
    grant: GrantState,
}
fn hash_fields(domain: &str, fields: &[&str]) -> String {
    let mut bytes = domain.as_bytes().to_vec();
    for field in fields {
        bytes.extend_from_slice(&(field.len() as u64).to_be_bytes());
        bytes.extend_from_slice(field.as_bytes());
    }
    SelectedFileWitness::from_bytes(bytes).sha256
}
fn valid_identity(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 256
        && text.trim() == text
        && !text.chars().any(char::is_control)
}

/// Explicit trusted embedding inputs, retained as a pinned root capability.
/// Source and facts are checked against current PolicyOwner on every attach.
/// Target/principal spelling alone confers no host authentication.
///
/// ```no_run
/// use std::sync::Arc;
/// use vibe_core::manifest::TargetOs;
/// use vibe_orchestrator::machine_submission::{HostPolicyFacts, TrustedPolicySource};
/// use vibe_orchestrator::machine_submission::cli_grant::CliGrantService;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // The embedding has already authenticated this physical root and identities.
/// let project = Arc::new(vibe_safefs::Project::open(std::path::Path::new("/authenticated/project"))?);
/// let service = CliGrantService::from_trusted_host(project, "target".into(), "principal".into(),
///     TargetOs::Linux, TrustedPolicySource::from_host_identifier("host-config".into())?,
///     HostPolicyFacts { approve_captured_script: true, approve_exact_local_install: false,
///         allow_detach: false,
///         allow_cancel_pre_dispatch: false, allow_cancel_deferred_boundary: false })?;
/// let first_host = service.issue(60_000)?;
/// drop(first_host);
/// let restarted_host = service.reattach()?;
/// assert!(restarted_host.readonly_scope().context.starts_with("cli-host:e1:"));
/// # Ok(()) }
/// ```
pub struct CliGrantService {
    project: Arc<Project>,
    root_token: String,
    scope: SubmissionScope,
    path: String,
    target_os: TargetOs,
    source: TrustedPolicySource,
    facts: HostPolicyFacts,
}
impl CliGrantService {
    /// Host-only construction; no ambient root or principal lookup occurs.
    pub fn from_trusted_host(
        project: Arc<Project>,
        target: String,
        principal: String,
        target_os: TargetOs,
        source: TrustedPolicySource,
        facts: HostPolicyFacts,
    ) -> Result<Self, CliGrantError> {
        if !valid_identity(&target) || !valid_identity(&principal) {
            return Err(CliGrantError::InvalidAuthority);
        }
        let root_token = project.root_correlation_token().map_err(infrastructure)?;
        let location = hash_fields("vibevm/cli-host/location/e1\0", &[&target, &principal]);
        let context = hash_fields(
            "vibevm/cli-host/context/e1\0",
            &[&root_token, &target, &principal],
        );
        let service = Self {
            project,
            root_token,
            scope: SubmissionScope {
                context: format!("cli-host:e1:{context}"),
                target,
                principal,
            },
            path: format!(".vibe/cli-host-grants/{}.json", &location[7..]),
            target_os,
            source,
            facts,
        };
        service.check_root(None)?;
        Ok(service)
    }

    /// Genuinely first issue only. Existing, uncertain, damaged or missing-after-
    /// issuance bindings refuse; callers explicitly choose `reattach` thereafter.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-CLI-HOST-REATTACH",
        r = 1
    )]
    pub fn issue(&self, ttl_ms: u64) -> Result<FixedLifecycleDecisionHost, CliGrantError> {
        if ttl_ms == 0 || ttl_ms > MAX_TTL_MS {
            return Err(CliGrantError::InvalidTtl);
        }
        let lease = self.lease()?;
        if let Some(pointer) = self.read(&lease)? {
            self.validate_binding(&pointer)?;
            return Err(match pointer.grant {
                GrantState::Issuing => CliGrantError::IssuanceIncomplete,
                GrantState::Issued { .. } => CliGrantError::AlreadyIssued,
            });
        }
        if SubmissionReservations::namespace_exists_for_scope(&lease, &self.scope)
            .map_err(CliGrantError::Namespace)?
        {
            return Err(CliGrantError::MissingPointer);
        }
        let mut pointer = self.pointer(GrantState::Issuing);
        self.publish(&lease, &pointer)?;
        PolicyOwner::initialize_under_lease(
            &lease,
            &self.scope,
            self.source.clone(),
            self.facts.clone(),
        )
        .map_err(CliGrantError::Policy)?;
        let namespace = SubmissionReservations::issue_namespace(&lease, &self.scope, ttl_ms)
            .map_err(CliGrantError::Namespace)?;
        let (expires_at_ms, _) =
            SubmissionReservations::validate_namespace(&lease, &namespace, &self.scope)
                .map_err(CliGrantError::Namespace)?;
        pointer.grant = GrantState::Issued {
            namespace: namespace.id().into(),
            expires_at_ms,
        };
        self.publish(&lease, &pointer)?;
        let owner = self.current_owner(&lease)?;
        drop(lease);
        self.construct_checked(pointer, namespace, owner)
    }

    /// Lookup only, including retired/expired namespaces for reserved-key
    /// observation. Fresh dispatch remains fenced by C1's current expiry and
    /// retirement checks; this never restores a question or approval.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-CLI-HOST-REATTACH",
        r = 1
    )]
    pub fn reattach(&self) -> Result<FixedLifecycleDecisionHost, CliGrantError> {
        let lease = self.lease()?;
        let pointer = self.read(&lease)?.ok_or(CliGrantError::MissingPointer)?;
        let namespace = self.validate_namespace(&lease, &pointer)?;
        let owner = self.current_owner(&lease)?;
        drop(lease);
        self.construct_checked(pointer, namespace, owner)
    }

    fn construct_checked(
        &self,
        pointer: Pointer,
        namespace: SubmissionNamespace,
        owner: Arc<PolicyOwner>,
    ) -> Result<FixedLifecycleDecisionHost, CliGrantError> {
        // The incumbent constructor acquires and checks its own lease. No outer
        // lease may span that call. Before returning, check the pointer again.
        let host = FixedLifecycleDecisionHost::from_shared_owner(
            self.project.root_path().into(),
            namespace,
            self.scope.clone(),
            self.target_os,
            owner.clone(),
        )
        .map_err(CliGrantError::Host)?;
        let lease = self.lease()?;
        if self.read(&lease)?.as_ref() != Some(&pointer) {
            return Err(CliGrantError::InvalidPointer(
                "binding changed during construction".into(),
            ));
        }
        self.validate_namespace(&lease, &pointer)?;
        self.validate_current_policy(&lease, &owner)?;
        self.check_root(Some(&lease))?;
        Ok(host)
    }

    fn pointer(&self, grant: GrantState) -> Pointer {
        Pointer {
            schema: 1,
            root_token: self.root_token.clone(),
            scope: Scope::from_host(&self.scope),
            target_os: self.target_os.as_str().into(),
            policy_source: self.source.identifier().into(),
            grant,
        }
    }
    fn validate_binding(&self, pointer: &Pointer) -> Result<(), CliGrantError> {
        if pointer.schema != 1 {
            return Err(CliGrantError::InvalidPointer("unsupported version".into()));
        }
        if pointer.root_token != self.root_token
            || pointer.scope != Scope::from_host(&self.scope)
            || pointer.target_os != self.target_os.as_str()
            || pointer.policy_source != self.source.identifier()
        {
            return Err(CliGrantError::ForeignBinding);
        }
        Ok(())
    }
    fn validate_namespace(
        &self,
        lease: &LifecycleLease,
        pointer: &Pointer,
    ) -> Result<SubmissionNamespace, CliGrantError> {
        self.validate_binding(pointer)?;
        let GrantState::Issued {
            namespace,
            expires_at_ms,
        } = &pointer.grant
        else {
            return Err(CliGrantError::IssuanceIncomplete);
        };
        let namespace = SubmissionNamespace::from_host_token(namespace.clone())
            .map_err(CliGrantError::Namespace)?;
        let (expiry, _) =
            SubmissionReservations::validate_namespace(lease, &namespace, &self.scope)
                .map_err(CliGrantError::Namespace)?;
        if expiry != *expires_at_ms {
            return Err(CliGrantError::InvalidPointer(
                "expiry differs from C1".into(),
            ));
        }
        Ok(namespace)
    }
    fn current_owner(&self, lease: &LifecycleLease) -> Result<Arc<PolicyOwner>, CliGrantError> {
        let owner = Arc::new(
            PolicyOwner::attach_readonly(self.project.clone(), self.scope.clone())
                .map_err(CliGrantError::Policy)?,
        );
        self.validate_current_policy(lease, &owner)?;
        Ok(owner)
    }
    fn validate_current_policy(
        &self,
        lease: &LifecycleLease,
        owner: &PolicyOwner,
    ) -> Result<(), CliGrantError> {
        let snapshot = owner
            .current_under_lease(lease)
            .map_err(CliGrantError::Policy)?;
        if snapshot.source() != &self.source || snapshot.facts() != &self.facts {
            return Err(CliGrantError::Policy(PolicyError::StartupDisagreement));
        }
        Ok(())
    }
    fn lease(&self) -> Result<LifecycleLease, CliGrantError> {
        self.check_root(None)?;
        let lease =
            LifecycleLease::acquire(self.project.root_path()).map_err(CliGrantError::Lease)?;
        self.check_root(Some(&lease))?;
        Ok(lease)
    }
    fn check_root(&self, lease: Option<&LifecycleLease>) -> Result<(), CliGrantError> {
        let current = Project::open(self.project.root_path()).map_err(infrastructure)?;
        if self
            .project
            .root_correlation_token()
            .map_err(infrastructure)?
            != self.root_token
            || current.root_correlation_token().map_err(infrastructure)? != self.root_token
        {
            return Err(CliGrantError::ForeignBinding);
        }
        if let Some(lease) = lease {
            if lease.root() != self.project.root_path()
                || lease
                    .publication_journal_project()
                    .root_correlation_token()
                    .map_err(infrastructure)?
                    != self.root_token
            {
                return Err(CliGrantError::ForeignBinding);
            }
        }
        Ok(())
    }
    fn read(&self, lease: &LifecycleLease) -> Result<Option<Pointer>, CliGrantError> {
        self.check_root(Some(lease))?;
        let bytes = lease
            .publication_journal_project()
            .read_file_bounded(&self.path, CAP)
            .map_err(infrastructure)?;
        self.check_root(Some(lease))?;
        bytes
            .map(|bytes| {
                serde_json::from_slice(&bytes)
                    .map_err(|error| CliGrantError::InvalidPointer(error.to_string()))
            })
            .transpose()
    }
    fn publish(&self, lease: &LifecycleLease, pointer: &Pointer) -> Result<(), CliGrantError> {
        self.validate_binding(pointer)?;
        let bytes = serde_json::to_vec(pointer).map_err(infrastructure)?;
        if bytes.len() > CAP {
            return Err(CliGrantError::InvalidPointer(
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
                    CliGrantError::UncertainPublication(reason)
                } else {
                    CliGrantError::Infrastructure(reason)
                }
            })?;
        let absolute = self.project.root_path().join(&self.path);
        let parent = absolute
            .parent()
            .ok_or_else(|| CliGrantError::InvalidPointer("missing parent".into()))?;
        require_durable(&write, parent)?;
        let verified = (|| {
            self.check_root(Some(lease))?;
            let readback = project
                .read_file_bounded(&self.path, CAP)
                .map_err(infrastructure)?;
            self.check_root(Some(lease))?;
            if readback.as_deref() != Some(bytes.as_slice()) {
                return Err(CliGrantError::InvalidPointer(
                    "published readback differs".into(),
                ));
            }
            Ok(())
        })();
        verified
            .map_err(|error: CliGrantError| CliGrantError::UncertainPublication(error.to_string()))
    }
}

// A weak or failed directory flush cannot establish a durable issuance fence.
fn require_durable(write: &DurableWrite, parent: &std::path::Path) -> Result<(), CliGrantError> {
    let uncertain = || {
        CliGrantError::UncertainPublication(
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

#[cfg(all(test, target_os = "linux"))]
mod native_tests;
#[cfg(test)]
mod tests;
