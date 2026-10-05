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
use std::{path::Path, sync::Arc};
use vibe_core::{PackageRef, manifest::TargetOs};
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_selection: Option<SourceSelection>,
    grant: GrantState,
}

/// Metadata retained at issue for cold inspection after source bytes drift.
/// No file digest or wire-supplied value can turn this into an effect port.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceSelection {
    registry_path_id: String,
    registry_token: String,
    source_token: String,
    package_pin: String,
    files: Vec<String>,
}
impl SourceSelection {
    fn capture(
        registry: &Path,
        pin: &PackageRef,
        files: &[String],
    ) -> Result<Self, ExactLocalInstallGrantError> {
        if !registry.is_absolute()
            || files.is_empty()
            || files.len() > 8
            || !files.iter().any(|file| file == "vibe.toml")
            || !pin.is_qualified()
            || !pin.version.is_exact_pin()
            || pin.kind.is_none()
        {
            return Err(ExactLocalInstallGrantError::InvalidAuthority);
        }
        let group = pin
            .group
            .as_ref()
            .ok_or(ExactLocalInstallGrantError::InvalidAuthority)?;
        let version = pin.version.to_string();
        let version = version
            .strip_prefix('=')
            .ok_or(ExactLocalInstallGrantError::InvalidAuthority)?;
        let source_root = registry
            .join(group.as_str())
            .join(pin.name.as_str())
            .join(format!("v{version}"));
        let registry_token = Project::open(registry)
            .map_err(source_unavailable)?
            .root_correlation_token()
            .map_err(source_unavailable)?;
        let source_token = Project::open(&source_root)
            .map_err(source_unavailable)?
            .root_correlation_token()
            .map_err(source_unavailable)?;
        let mut path_bytes = b"vibevm/exact-local-install-grant/registry-path/e1\0".to_vec();
        frame(&mut path_bytes, registry.as_os_str().as_encoded_bytes());
        let supplied_len = files.len();
        let mut files = files.to_vec();
        files.sort();
        files.dedup();
        if files.len() != supplied_len {
            return Err(ExactLocalInstallGrantError::InvalidAuthority);
        }
        Ok(Self {
            registry_path_id: digest(path_bytes),
            registry_token,
            source_token,
            package_pin: pin.to_string(),
            files,
        })
    }
    fn from_source(
        source: &dyn BoundOnePackageSource,
    ) -> Result<Self, ExactLocalInstallGrantError> {
        let witness = source.source_witness();
        witness.validate().map_err(source_unavailable)?;
        let pin = &witness.package_pin;
        let registry = witness
            .root
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .ok_or(ExactLocalInstallGrantError::InvalidAuthority)?;
        let files = witness
            .files
            .iter()
            .map(|file| file.relative_path.clone())
            .collect::<Vec<_>>();
        let selected = Self::capture(registry, pin, &files)?;
        let group = pin
            .group
            .as_ref()
            .ok_or(ExactLocalInstallGrantError::InvalidAuthority)?;
        let version = pin.version.to_string();
        let version = version
            .strip_prefix('=')
            .ok_or(ExactLocalInstallGrantError::InvalidAuthority)?;
        if witness.root
            != registry
                .join(group.as_str())
                .join(pin.name.as_str())
                .join(format!("v{version}"))
        {
            return Err(ExactLocalInstallGrantError::InvalidAuthority);
        }
        Ok(selected)
    }
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
    source: Option<Arc<dyn BoundOnePackageSource>>,
    source_id: Option<String>,
    source_selection: SourceSelection,
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
        let source_selection = SourceSelection::from_source(source.as_ref())?;
        let service = Self {
            project,
            root_token,
            scope,
            path,
            target_os,
            policy_source,
            facts,
            source: Some(source),
            source_id: Some(source_id),
            source_selection,
        };
        service.check_root(None)?;
        Ok(service)
    }

    /// Read-only cross-process reattach. Exact host selection and physical
    /// registry/source roots are compared with the issued pointer, but source
    /// file bytes are neither captured nor accepted as a fresh effect port.
    pub fn from_trusted_recovery_host(
        project: Arc<Project>,
        scope: SubmissionScope,
        target_os: TargetOs,
        policy_source: TrustedPolicySource,
        facts: HostPolicyFacts,
        registry_root: &Path,
        package_pin: &PackageRef,
        source_files: &[String],
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
        let source_selection = SourceSelection::capture(registry_root, package_pin, source_files)?;
        let service = Self {
            project,
            root_token,
            scope,
            path,
            target_os,
            policy_source,
            facts,
            source: None,
            source_id: None,
            source_selection,
        };
        service.check_root(None)?;
        Ok(service)
    }

    pub fn issue(
        &self,
        ttl_ms: u64,
    ) -> Result<ExactLocalInstallGrant, ExactLocalInstallGrantError> {
        if self.source.is_none() {
            return Err(ExactLocalInstallGrantError::InvalidAuthority);
        }
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
        self.construct_checked(pointer, namespace, owner, false)
    }

    /// Lookup only. Retired and expired namespaces remain readable for C1/C2
    /// result inspection; C1 itself refuses a fresh key in either state.
    /// An explicit recovery service can restore cold inspection after content
    /// drift using retained selection metadata, with no source port.
    pub fn reattach(&self) -> Result<ExactLocalInstallGrant, ExactLocalInstallGrantError> {
        let lease = self.lease()?;
        let pointer = self
            .read(&lease)?
            .ok_or(ExactLocalInstallGrantError::MissingPointer)?;
        let namespace = self.validate_namespace(&lease, &pointer)?;
        let owner = self.current_owner(&lease, &pointer)?;
        drop(lease);
        self.construct_checked(pointer, namespace, owner, self.source.is_none())
    }

    /// A legacy schema-1 pointer can be looked up only while its original
    /// source still validates. The returned host remains inspection-only.
    pub fn reattach_readonly_legacy(
        &self,
    ) -> Result<ExactLocalInstallGrant, ExactLocalInstallGrantError> {
        if self.source.is_none() {
            return Err(ExactLocalInstallGrantError::InvalidAuthority);
        }
        self.check_source()?;
        let lease = self.lease()?;
        let pointer = self
            .read(&lease)?
            .ok_or(ExactLocalInstallGrantError::MissingPointer)?;
        if pointer.schema != 1 {
            return Err(ExactLocalInstallGrantError::InvalidPointer(
                "not a legacy pointer".into(),
            ));
        }
        let namespace = self.validate_namespace(&lease, &pointer)?;
        let owner = self.current_owner(&lease, &pointer)?;
        drop(lease);
        self.construct_checked(pointer, namespace, owner, true)
    }

    fn construct_checked(
        &self,
        pointer: Pointer,
        namespace: SubmissionNamespace,
        owner: Arc<PolicyOwner>,
        recovery_only: bool,
    ) -> Result<ExactLocalInstallGrant, ExactLocalInstallGrantError> {
        let host = Arc::new(
            match &self.source {
                _ if recovery_only => ExactLocalInstallDecisionHost::from_recovery_host(
                    self.project.root_path().into(),
                    owner.clone(),
                    self.target_os,
                ),
                Some(source) => ExactLocalInstallDecisionHost::from_host(
                    self.project.root_path().into(),
                    owner.clone(),
                    source.clone(),
                    self.target_os,
                ),
                None => ExactLocalInstallDecisionHost::from_recovery_host(
                    self.project.root_path().into(),
                    owner.clone(),
                    self.target_os,
                ),
            }
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
        if recovery_only && self.source.is_some() {
            self.check_source()?;
        }
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
        let source = self
            .source
            .as_ref()
            .ok_or(ExactLocalInstallGrantError::InvalidAuthority)?;
        if Some(Self::capture_source_id(source.as_ref())?) != self.source_id
            || SourceSelection::from_source(source.as_ref())? != self.source_selection
        {
            return Err(ExactLocalInstallGrantError::ForeignBinding);
        }
        source
            .recheck_source_binding(source.source_witness())
            .map_err(source_unavailable)?;
        Ok(())
    }
    fn pointer(&self, grant: GrantState) -> Pointer {
        Pointer {
            schema: 2,
            root_token: self.root_token.clone(),
            scope: Scope::from(&self.scope),
            target_os: self.target_os.as_str().into(),
            policy_source: self.policy_source.identifier().into(),
            source_id: self.source_id.clone().expect("issue requires bound source"),
            source_selection: Some(self.source_selection.clone()),
            grant,
        }
    }
    fn validate_binding(&self, pointer: &Pointer) -> Result<(), ExactLocalInstallGrantError> {
        if pointer.schema != 1 && pointer.schema != 2 {
            return Err(ExactLocalInstallGrantError::InvalidPointer(
                "unsupported version".into(),
            ));
        }
        if pointer.schema == 1 && self.source.is_none() {
            return Err(ExactLocalInstallGrantError::InvalidPointer(
                "legacy pointer has no cold source selection".into(),
            ));
        }
        if pointer.schema == 2 && pointer.source_selection.as_ref() != Some(&self.source_selection)
        {
            return Err(ExactLocalInstallGrantError::ForeignBinding);
        }
        if pointer.schema == 1 && pointer.source_selection.is_some() {
            return Err(ExactLocalInstallGrantError::InvalidPointer(
                "legacy selection field".into(),
            ));
        }
        let valid_source_id = pointer
            .source_id
            .strip_prefix("sha256:")
            .is_some_and(|hex| {
                hex.len() == 64
                    && hex
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
            });
        if !valid_source_id {
            return Err(ExactLocalInstallGrantError::InvalidPointer(
                "source ID shape".into(),
            ));
        }
        if pointer.root_token != self.root_token
            || pointer.scope != Scope::from(&self.scope)
            || pointer.target_os != self.target_os.as_str()
            || pointer.policy_source != self.policy_source.identifier()
            || self
                .source_id
                .as_ref()
                .is_some_and(|source_id| &pointer.source_id != source_id)
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
