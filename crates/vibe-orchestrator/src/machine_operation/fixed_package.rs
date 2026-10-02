//! Trusted local host for the one fixed static-file package operation.
//! The lifecycle package owner retains admission, execution and result truth.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-FIXED-PACKAGE");

use crate::machine_submission::{HostPolicyFacts, PolicyError, PolicyOwner, TrustedPolicySource};
use std::sync::Arc;

use vibe_lifecycle::{
    LifecycleLease, LifecycleLeaseError, StaticFileJournalError, StaticFileJournalInspection,
    SubmissionError, SubmissionNamespace, SubmissionReservations, SubmissionScope,
    inspect_fixed_static_file_package, submit_fixed_static_file_package,
};
use vibe_safefs::Project;

const LAW: &str = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-FIXED-PACKAGE";

mod contract;
mod dispatch;
mod envelope;
pub use contract::{PUBLICATION_CAP, REQUEST_CAP};
pub use dispatch::{FixedPackageRoute, FixedPackageRouteError};
pub use envelope::{DOMAIN_OWNER, OPERATION_ID};
#[cfg(test)]
mod native_tests;

/// Refusals retain the lifecycle owner's typed error. An error after submit
/// reaches that owner does not establish that no effect began.
///
/// ```
/// use vibe_orchestrator::machine_operation::fixed_package::FixedPackageHostError;
/// let error = FixedPackageHostError::ForeignRoot;
/// assert!(error.to_string().contains("MACHINE-FIXED-PACKAGE"));
/// ```
#[derive(Debug)]
pub enum FixedPackageHostError {
    ForeignRoot,
    Root(String),
    Lease(LifecycleLeaseError),
    Namespace(SubmissionError),
    Policy(PolicyError),
    Owner(StaticFileJournalError),
}

impl std::fmt::Display for FixedPackageHostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (why, fix) = match self {
            Self::ForeignRoot => (
                "physical root changed".to_owned(),
                "trusted host root binding",
            ),
            Self::Root(error) => (
                format!("cannot inspect physical root: {error}"),
                "trusted host root binding",
            ),
            Self::Lease(error) => (format!("lifecycle lease: {error}"), "package owner lease"),
            Self::Namespace(error) => (
                format!("C1 namespace or scope: {error}"),
                "host-issued grant",
            ),
            Self::Policy(error) => (
                format!("current package policy: {error}"),
                "trusted package policy owner",
            ),
            Self::Owner(error) => (
                format!("package owner: {error}"),
                "lifecycle package journal",
            ),
        };
        write!(f, "violates REQ {LAW}: {why}; fix surface: {fix}")
    }
}

impl std::error::Error for FixedPackageHostError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Lease(error) => Some(error),
            Self::Namespace(error) => Some(error),
            Self::Policy(error) => Some(error),
            Self::Owner(error) => Some(error),
            Self::ForeignRoot | Self::Root(_) => None,
        }
    }
}

impl From<LifecycleLeaseError> for FixedPackageHostError {
    fn from(error: LifecycleLeaseError) -> Self {
        Self::Lease(error)
    }
}
impl From<SubmissionError> for FixedPackageHostError {
    fn from(error: SubmissionError) -> Self {
        Self::Namespace(error)
    }
}
impl From<StaticFileJournalError> for FixedPackageHostError {
    fn from(error: StaticFileJournalError) -> Self {
        Self::Owner(error)
    }
}

/// One explicit physical root and host-issued C1 namespace/scope. This object
/// issues no namespace and retains no second operation result or reservation.
///
/// ```no_run
/// use std::sync::Arc;
/// use vibe_lifecycle::{LifecycleLease, SubmissionReservations, SubmissionScope};
/// use vibe_orchestrator::machine_operation::fixed_package::FixedPackageHost;
/// use vibe_safefs::Project;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let project = Arc::new(Project::open(std::path::Path::new("/trusted/project"))?);
/// let scope = SubmissionScope {
///     context: "host-context".into(), target: "launcher".into(),
///     principal: "host-principal".into(),
/// };
/// let lease = LifecycleLease::acquire(project.root_path())?;
/// let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000)?;
/// drop(lease);
/// let host = FixedPackageHost::from_trusted_host(project, namespace, scope)?;
/// let result = host.submit("copy-once")?;
/// assert_eq!(host.inspect("copy-once")?, Some(result));
/// # Ok(()) }
/// ```
pub struct FixedPackageHost {
    project: Arc<Project>,
    root_token: String,
    namespace: SubmissionNamespace,
    scope: SubmissionScope,
    policy: Option<(Arc<PolicyOwner>, TrustedPolicySource)>,
}

impl FixedPackageHost {
    /// Pin and check the authenticated host inputs against the incumbent C1 index.
    #[track_caller]
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-FIXED-PACKAGE",
        r = 1
    )]
    pub fn from_trusted_host(
        project: Arc<Project>,
        namespace: SubmissionNamespace,
        scope: SubmissionScope,
    ) -> Result<Self, FixedPackageHostError> {
        let root_token = project.root_correlation_token().map_err(root_error)?;
        let host = Self {
            project,
            root_token,
            namespace,
            scope,
            policy: None,
        };
        let lease = host.lease()?;
        SubmissionReservations::validate_namespace(&lease, &host.namespace, &host.scope)?;
        Ok(host)
    }

    /// Attach a current shared policy guard to a trusted package host.
    ///
    /// ```no_run
    /// # use std::sync::Arc;
    /// # use vibe_orchestrator::machine_operation::fixed_package::FixedPackageHost;
    /// # use vibe_orchestrator::machine_submission::{PolicyOwner, TrustedPolicySource};
    /// # use vibe_lifecycle::{SubmissionNamespace, SubmissionScope};
    /// # use vibe_safefs::Project;
    /// # fn guarded(project: Arc<Project>, namespace: SubmissionNamespace, scope: SubmissionScope, owner: Arc<PolicyOwner>, source: TrustedPolicySource) -> Result<FixedPackageHost, Box<dyn std::error::Error>> {
    /// let host = FixedPackageHost::with_shared_policy(project, namespace, scope, owner, source)?;
    /// # Ok(host) }
    /// ```
    pub fn with_shared_policy(
        project: Arc<Project>,
        namespace: SubmissionNamespace,
        scope: SubmissionScope,
        owner: Arc<PolicyOwner>,
        source: TrustedPolicySource,
    ) -> Result<Self, FixedPackageHostError> {
        if owner.scope() != &scope {
            return Err(FixedPackageHostError::Policy(PolicyError::ForeignScope));
        }
        let mut host = Self::from_trusted_host(project, namespace, scope)?;
        let lease = host.lease()?;
        let snapshot = owner
            .current_under_lease(&lease)
            .map_err(FixedPackageHostError::Policy)?;
        if snapshot.source() != &source || snapshot.facts() != &package_policy_facts() {
            return Err(FixedPackageHostError::Policy(
                PolicyError::StartupDisagreement,
            ));
        }
        host.policy = Some((owner, source));
        Ok(host)
    }

    /// Submit the fixed package copy with the host clock. The owner performs
    /// its own C1 reservation and returns its exact retained result state.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-FIXED-PACKAGE",
        r = 1
    )]
    pub fn submit(&self, key: &str) -> Result<StaticFileJournalInspection, FixedPackageHostError> {
        let lease = self.lease()?;
        SubmissionReservations::validate_namespace(&lease, &self.namespace, &self.scope)?;
        if let Some((owner, source)) = &self.policy {
            let current = owner
                .current_under_lease(&lease)
                .map_err(FixedPackageHostError::Policy)?;
            if current.source() != source || current.facts() != &package_policy_facts() {
                return Err(FixedPackageHostError::Policy(
                    PolicyError::StartupDisagreement,
                ));
            }
        }
        let created_at = vibe_core::timestamp::now_utc();
        let result = submit_fixed_static_file_package(
            &lease,
            &self.namespace,
            &self.scope,
            key,
            &created_at,
        )?;
        self.check_root(Some(&lease))?;
        Ok(result)
    }

    /// Cold-inspect one key through the lifecycle package journal. `None`
    /// means the owner found no retained key under this exact scope.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-FIXED-PACKAGE",
        r = 1
    )]
    pub fn inspect(
        &self,
        key: &str,
    ) -> Result<Option<StaticFileJournalInspection>, FixedPackageHostError> {
        let lease = self.lease()?;
        SubmissionReservations::validate_namespace(&lease, &self.namespace, &self.scope)?;
        let result =
            inspect_fixed_static_file_package(&lease, self.namespace.id(), &self.scope, key)?;
        self.check_root(Some(&lease))?;
        Ok(result)
    }

    fn lease(&self) -> Result<LifecycleLease, FixedPackageHostError> {
        self.check_root(None)?;
        let lease = LifecycleLease::acquire(self.project.root_path())?;
        self.check_root(Some(&lease))?;
        Ok(lease)
    }

    fn check_root(&self, lease: Option<&LifecycleLease>) -> Result<(), FixedPackageHostError> {
        let reopened = Project::open(self.project.root_path()).map_err(root_error)?;
        if self.project.root_correlation_token().map_err(root_error)? != self.root_token
            || reopened.root_correlation_token().map_err(root_error)? != self.root_token
        {
            return Err(FixedPackageHostError::ForeignRoot);
        }
        if let Some(lease) = lease {
            if lease.root() != self.project.root_path()
                || lease
                    .publication_journal_project()
                    .root_correlation_token()
                    .map_err(root_error)?
                    != self.root_token
            {
                return Err(FixedPackageHostError::ForeignRoot);
            }
        }
        Ok(())
    }
}

fn root_error(error: impl std::fmt::Display) -> FixedPackageHostError {
    FixedPackageHostError::Root(error.to_string())
}

pub(crate) fn package_policy_facts() -> HostPolicyFacts {
    HostPolicyFacts {
        approve_captured_script: false,
        allow_detach: false,
        allow_cancel_pre_dispatch: false,
        allow_cancel_deferred_boundary: false,
    }
}

#[cfg(test)]
#[path = "fixed_package/tests.rs"]
mod tests;
