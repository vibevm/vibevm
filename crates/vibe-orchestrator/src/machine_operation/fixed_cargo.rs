//! Trusted library route for the incumbent one-fixture offline Cargo owner.
//! The lifecycle build journal alone reserves C1 and retains effect truth.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-FIXED-CARGO-BUILD");

use std::sync::Arc;
use vibe_core::manifest::MechanismRoutes;
use vibe_extension_registry::MechanismRegistry;
use vibe_lifecycle::{
    LifecycleLease, LifecycleLeaseError, SubmissionError, SubmissionNamespace,
    SubmissionReservations, SubmissionScope,
    retained_mechanism::{
        BuildInspection, FixedCargoSubmitError, RetainedMechanismError, inspect_fixed_cargo_build,
        submit_fixed_cargo_build,
    },
};
use vibe_safefs::Project;

mod contract;
mod dispatch;
mod envelope;
pub use contract::{PUBLICATION_CAP, REQUEST_CAP};
pub use dispatch::{FixedCargoRoute, FixedCargoRouteError};
pub use envelope::{DOMAIN_OWNER, OPERATION_ID};
#[cfg(test)]
mod tests;

#[derive(Debug)]
pub enum FixedCargoHostError {
    ForeignRoot,
    ForeignTarget,
    Root(String),
    Lease(LifecycleLeaseError),
    Namespace(SubmissionError),
    Submit(FixedCargoSubmitError),
    Inspect(RetainedMechanismError),
    PostOwner {
        observation: BuildInspection,
        cause: Box<FixedCargoHostError>,
    },
}
impl std::fmt::Display for FixedCargoHostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fixed Cargo host refused: {self:?}")
    }
}
impl std::error::Error for FixedCargoHostError {}

/// A trusted embedding provides an already issued namespace and exact scope.
/// Construction never issues a grant, reserves C1, or starts Cargo.
pub struct FixedCargoHost {
    project: Arc<Project>,
    root_token: String,
    namespace: SubmissionNamespace,
    scope: SubmissionScope,
    registry: Arc<MechanismRegistry>,
    routes: Arc<MechanismRoutes>,
}
impl FixedCargoHost {
    pub fn from_trusted_host(
        project: Arc<Project>,
        namespace: SubmissionNamespace,
        scope: SubmissionScope,
        registry: Arc<MechanismRegistry>,
        routes: Arc<MechanismRoutes>,
    ) -> Result<Self, FixedCargoHostError> {
        if scope.target != "fixture-tool" {
            return Err(FixedCargoHostError::ForeignTarget);
        }
        let root_token = project.root_correlation_token().map_err(root_error)?;
        let host = Self {
            project,
            root_token,
            namespace,
            scope,
            registry,
            routes,
        };
        let lease = host.lease()?;
        SubmissionReservations::validate_namespace(&lease, &host.namespace, &host.scope)
            .map_err(FixedCargoHostError::Namespace)?;
        Ok(host)
    }

    pub fn submit(&self, key: &str) -> Result<BuildInspection, FixedCargoHostError> {
        self.submit_with_after_owner(key, |_| {})
    }
    fn submit_with_after_owner(
        &self,
        key: &str,
        after_owner: impl FnOnce(&BuildInspection),
    ) -> Result<BuildInspection, FixedCargoHostError> {
        let lease = self.lease()?;
        SubmissionReservations::validate_namespace(&lease, &self.namespace, &self.scope)
            .map_err(FixedCargoHostError::Namespace)?;
        let result = submit_fixed_cargo_build(
            &lease,
            &self.namespace,
            &self.scope,
            key,
            &self.registry,
            &self.routes,
        )
        .map_err(FixedCargoHostError::Submit)?;
        after_owner(&result);
        self.check_root(Some(&lease))
            .map_err(|cause| FixedCargoHostError::PostOwner {
                observation: result.clone(),
                cause: Box::new(cause),
            })?;
        Ok(result)
    }

    pub fn inspect(&self, key: &str) -> Result<BuildInspection, FixedCargoHostError> {
        let lease = self.lease()?;
        SubmissionReservations::validate_namespace(&lease, &self.namespace, &self.scope)
            .map_err(FixedCargoHostError::Namespace)?;
        let result = inspect_fixed_cargo_build(&lease, self.namespace.id(), &self.scope, key)
            .map_err(FixedCargoHostError::Inspect)?;
        self.check_root(Some(&lease))
            .map_err(|cause| FixedCargoHostError::PostOwner {
                observation: result.clone(),
                cause: Box::new(cause),
            })?;
        Ok(result)
    }

    fn lease(&self) -> Result<LifecycleLease, FixedCargoHostError> {
        self.check_root(None)?;
        let lease = LifecycleLease::acquire(self.project.root_path())
            .map_err(FixedCargoHostError::Lease)?;
        self.check_root(Some(&lease))?;
        Ok(lease)
    }
    fn check_root(&self, lease: Option<&LifecycleLease>) -> Result<(), FixedCargoHostError> {
        let reopened = Project::open(self.project.root_path()).map_err(root_error)?;
        if self.project.root_correlation_token().map_err(root_error)? != self.root_token
            || reopened.root_correlation_token().map_err(root_error)? != self.root_token
        {
            return Err(FixedCargoHostError::ForeignRoot);
        }
        if let Some(lease) = lease {
            if lease.root() != self.project.root_path()
                || lease
                    .publication_journal_project()
                    .root_correlation_token()
                    .map_err(root_error)?
                    != self.root_token
            {
                return Err(FixedCargoHostError::ForeignRoot);
            }
        }
        Ok(())
    }
}
fn root_error(error: impl std::fmt::Display) -> FixedCargoHostError {
    FixedCargoHostError::Root(error.to_string())
}
