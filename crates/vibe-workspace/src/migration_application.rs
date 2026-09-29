//! Bounded application admission and immutable journal identities.
//!
//! These APIs validate a caller's explicit one-file grant; they neither apply
//! a plan nor create recovery state. Coordinated publication is a later owner.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE"
);

use std::path::{Component, Path, PathBuf};
use vibe_safefs::{ExternalStore, Project, ensure_safe_component, paths_overlap, split_relative};
use vibe_wire::generated::migration::e1::{
    operation_report::{ApplicationError, ApplicationErrorCode, OperationReportOperation},
    operation_request::{OperationRequestApply, OperationRequestInspect, OperationRequestRecover},
    plan_report::Plan,
};

pub mod identity;
pub mod report;
pub mod validation;

pub use report::{ReportBuilder, validate_operation_report};
pub use validation::{decode_journal_json, decode_operation_json, validate_journal};

/// This constructor is the admitting caller's explicit grant for ONE owned
/// authored file. A payload, path spelling or version never infers ownership.
/// Callers must exclude fixtures, upstream data and historical captured state.
/// Opening the existing root is no-follow; no target/store entry is created.
#[derive(Debug)]
pub struct OwnedManifestMigrationFile {
    pub(crate) project: Project,
    pub(crate) project_spelling: String,
    pub(crate) target_relative: String,
    pub(crate) recovery_store: PathBuf,
    pub(crate) store_spelling: String,
    pub(crate) journal_read_limit: usize,
    protected_machine_cache: Option<PathBuf>,
}

impl OwnedManifestMigrationFile {
    pub fn new(
        project_root: &Path,
        target_relative: &str,
        recovery_store: &Path,
    ) -> Result<Self, ApplicationValidationError> {
        let project_spelling = absolute_spelling(project_root)?;
        let store_spelling = absolute_spelling(recovery_store)?;
        split_relative(target_relative).map_err(|_| {
            ApplicationValidationError::new(
                ApplicationErrorCode::AuthorityRefused,
                "authority.target_relative",
                "target is not a portable relative operand",
            )
        })?;
        let project = Project::open(project_root).map_err(|_| {
            ApplicationValidationError::new(
                ApplicationErrorCode::AuthorityRefused,
                "authority.project_root",
                "existing project capability could not be opened no-follow",
            )
        })?;
        Ok(Self {
            project,
            project_spelling,
            target_relative: target_relative.to_owned(),
            recovery_store: recovery_store.to_owned(),
            store_spelling,
            journal_read_limit: 8 * 1024 * 1024,
            protected_machine_cache: None,
        })
    }

    /// Optional server-composed cache exclusion. Direct trusted callers carry
    /// no global/environment-derived policy. Admission and rechecks create nothing.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-SURFACES",
        r = 1
    )]
    pub fn with_protected_machine_cache_root(
        mut self,
        cache: &Path,
    ) -> Result<Self, ApplicationValidationError> {
        absolute_spelling(cache)?;
        self.protected_machine_cache = Some(cache.to_owned());
        self.validate_recovery_store_policy()?;
        Ok(self)
    }

    /// Recheck immediately before a creating store open and EVERY store
    /// mutation. This read-only policy is not an unconditional external-race
    /// exclusion; filesystem capabilities and the runtime own actual writes.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-SURFACES",
        r = 1
    )]
    pub fn validate_recovery_store_policy(&self) -> Result<(), ApplicationValidationError> {
        let Some(cache) = &self.protected_machine_cache else {
            return Ok(());
        };
        let refused = || {
            ApplicationValidationError::new(
                ApplicationErrorCode::AuthorityRefused,
                "authority.recovery_store_root",
                "recovery store intersects or cannot exclude the protected machine cache",
            )
        };
        if paths_overlap(&self.recovery_store, cache) {
            return Err(refused());
        }
        let cache_project = match Project::open(cache) {
            Ok(project) => project,
            Err(error) => {
                let absent = error
                    .chain()
                    .filter_map(|cause| cause.downcast_ref::<std::io::Error>())
                    .any(|cause| cause.kind() == std::io::ErrorKind::NotFound);
                if absent {
                    return Ok(());
                }
                return Err(refused());
            }
        };
        ExternalStore::open_existing_disjoint(&self.recovery_store, &cache_project)
            .map_err(|_| refused())?;
        Ok(())
    }

    /// Configurable allocation ceiling, not a boot quota or OS exclusion.
    pub fn with_journal_read_limit(mut self, bytes: usize) -> Self {
        self.journal_read_limit = bytes;
        self
    }
    pub fn journal_read_limit(&self) -> usize {
        self.journal_read_limit
    }
    pub fn project(&self) -> &Project {
        &self.project
    }
    pub fn recovery_store_root(&self) -> &Path {
        &self.recovery_store
    }
    pub fn target_relative(&self) -> &str {
        &self.target_relative
    }
}

fn absolute_spelling(path: &Path) -> Result<String, ApplicationValidationError> {
    let invalid = || {
        ApplicationValidationError::new(
            ApplicationErrorCode::AuthorityRefused,
            "authority",
            "root must be an explicit portable absolute spelling",
        )
    };
    if !path.is_absolute() {
        return Err(invalid());
    }
    for component in path.components() {
        match component {
            Component::Normal(name) => {
                ensure_safe_component(name.to_str().ok_or_else(invalid)?).map_err(|_| invalid())?
            }
            Component::RootDir | Component::Prefix(_) => {}
            _ => return Err(invalid()),
        }
    }
    path.to_str().map(str::to_owned).ok_or_else(invalid)
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("migration application field {field}: {reason}")]
pub struct ApplicationValidationError {
    pub code: ApplicationErrorCode,
    pub field: &'static str,
    pub reason: &'static str,
    pub operation: Option<OperationReportOperation>,
    pub operation_id: Option<String>,
}
impl ApplicationValidationError {
    pub(crate) fn new(
        code: ApplicationErrorCode,
        field: &'static str,
        reason: &'static str,
    ) -> Self {
        Self {
            code,
            field,
            reason,
            operation: None,
            operation_id: None,
        }
    }
    pub fn generated_error(&self) -> ApplicationError {
        ApplicationError {
            code: self.code.clone(),
            field: self.field.to_owned(),
            reason: self.reason.to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedApply {
    pub request: OperationRequestApply,
    pub plan: Plan,
    pub is_no_op: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidatedOperation {
    Apply(ValidatedApply),
    Inspect(OperationRequestInspect),
    Recover(OperationRequestRecover),
}

#[cfg(test)]
#[path = "migration_application/validation_tests.rs"]
mod validation_tests;
