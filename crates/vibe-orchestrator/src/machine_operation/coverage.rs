//! Preserve the authored coverage inventory as explicit extraction gaps.
//! Source hashes and owner/schema prose are declarations, never byte proof.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-REGISTRY");

use std::collections::BTreeMap;
use std::fmt;

use vibe_wire::generated::capability::e1::coverage::{Coverage, Disposition, Source};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredSource {
    pub source_id: String,
    pub path: String,
    pub declared_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageBinding {
    pub surface_id: Option<String>,
    pub owner_prose: String,
    pub schema_ref_prose: String,
    pub source_ids: Vec<String>,
    pub declared_sources: Vec<DeclaredSource>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractionGapReason {
    UnboundDomainOwnerAndFormat,
    MultipleSurfaceBindingsUnbound,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionGap {
    pub identity: String,
    pub bindings: Vec<CoverageBinding>,
    pub reason: ExtractionGapReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineCoverageRegistry {
    operations: Vec<ExtractionGap>,
    models: Vec<ExtractionGap>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MachineCoverageError {
    InvalidInventory {
        field: &'static str,
        subject: String,
        reason: &'static str,
    },
    MalformedIdentity(String),
    OperationModelCollision(String),
}

impl fmt::Display for MachineCoverageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInventory {
                field,
                subject,
                reason,
            } => {
                write!(f, "coverage {field} ({subject}): {reason}")
            }
            Self::MalformedIdentity(id) => write!(f, "malformed machine identity: {id}"),
            Self::OperationModelCollision(id) => {
                write!(f, "operation and model identities collide: {id}")
            }
        }
    }
}

impl std::error::Error for MachineCoverageError {}

fn namespaced(id: &str) -> bool {
    !id.is_empty()
        && id.trim() == id
        && (id.contains(':') || id.contains('.'))
        && !id.chars().any(char::is_control)
        && !id.chars().any(char::is_whitespace)
}

fn binding(
    surface_id: Option<&str>,
    owner: &str,
    schema_ref: &str,
    source_ids: &[String],
    sources: &BTreeMap<&str, &Source>,
) -> CoverageBinding {
    CoverageBinding {
        surface_id: surface_id.map(str::to_owned),
        owner_prose: owner.to_owned(),
        schema_ref_prose: schema_ref.to_owned(),
        source_ids: source_ids.to_vec(),
        declared_sources: source_ids
            .iter()
            .map(|id| {
                // Coverage::validate_capability_coverage has already proved
                // every reference resolves. This is not a filesystem read.
                let source = sources[id.as_str()];
                DeclaredSource {
                    source_id: id.clone(),
                    path: source.path.clone(),
                    declared_sha256: source.sha256.clone(),
                }
            })
            .collect(),
    }
}

impl MachineCoverageRegistry {
    /// Convert each source-observed operation/model identity to one gap.
    /// Multiple surfaces may legitimately map to one operation, so all their
    /// declarations survive without inventing a single domain owner.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-REGISTRY",
        r = 1
    )]
    pub fn from_coverage(coverage: &Coverage) -> Result<Self, MachineCoverageError> {
        coverage.validate_capability_coverage().map_err(|error| {
            MachineCoverageError::InvalidInventory {
                field: error.field,
                subject: error.subject,
                reason: error.reason,
            }
        })?;
        let sources = coverage
            .sources
            .iter()
            .map(|row| (row.source_id.as_str(), row))
            .collect::<BTreeMap<_, _>>();
        let mut operations = Vec::<ExtractionGap>::new();
        let mut operation_index = BTreeMap::<&str, usize>::new();
        for surface in &coverage.surfaces {
            if surface.disposition != Disposition::Operation {
                continue;
            }
            let id = surface
                .operation_id
                .as_deref()
                .expect("validated operation ID");
            if !namespaced(id) {
                return Err(MachineCoverageError::MalformedIdentity(id.to_owned()));
            }
            let row = binding(
                Some(&surface.surface_id),
                &surface.owner,
                &surface.schema_ref,
                &surface.source_ids,
                &sources,
            );
            if let Some(&index) = operation_index.get(id) {
                let gap = &mut operations[index];
                gap.bindings.push(row);
                gap.reason = ExtractionGapReason::MultipleSurfaceBindingsUnbound;
            } else {
                operation_index.insert(id, operations.len());
                operations.push(ExtractionGap {
                    identity: id.to_owned(),
                    bindings: vec![row],
                    reason: ExtractionGapReason::UnboundDomainOwnerAndFormat,
                });
            }
        }
        let mut models = Vec::with_capacity(coverage.models.len());
        for model in &coverage.models {
            let id = model.model_id.as_str();
            if !namespaced(id) {
                return Err(MachineCoverageError::MalformedIdentity(id.to_owned()));
            }
            if operation_index.contains_key(id) {
                return Err(MachineCoverageError::OperationModelCollision(id.to_owned()));
            }
            // The owning coverage validator checked uniqueness within models.
            models.push(ExtractionGap {
                identity: id.to_owned(),
                bindings: vec![binding(
                    None,
                    &model.owner,
                    &model.schema_ref,
                    &model.source_ids,
                    &sources,
                )],
                reason: ExtractionGapReason::UnboundDomainOwnerAndFormat,
            });
        }
        Ok(Self { operations, models })
    }

    #[must_use]
    pub fn operations(&self) -> &[ExtractionGap] {
        &self.operations
    }

    #[must_use]
    pub fn models(&self) -> &[ExtractionGap] {
        &self.models
    }
}

#[cfg(test)]
#[path = "coverage/tests.rs"]
mod tests;
