//! Pure selected-world projection of authored extension operation-schema refs.
//!
//! The caller owns the selected world's authenticity. A content pin here is
//! logical lock-bound provenance, never a physical byte or execution grant.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-EXTENSION-SCHEMA-SOURCE");

use std::collections::BTreeMap;
use std::fmt;

use specmark::spec;
use vibe_core::ContentHash;
use vibe_core::manifest::{ExtensionDecl, MachineOperationSchemaDecl};
use vibe_extension_registry::{DependencyProviderId, ExtensionWorld, HostIdentity};
use vibe_wire::generated::format_id::FormatId;
use vibe_wire::generated::machine::e1::operation_descriptor::{
    OperationDescriptor, OperationDescriptorAuthorization as Authorization,
    OperationDescriptorAvailability as Availability,
    OperationDescriptorImplementation as Implementation,
};

/// The typed selected provider and declaration from which a schema ref came.
/// Roots and artifact paths are deliberately absent: this value is not a read
/// capability and cannot admit a handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceBinding {
    Dependency {
        provider: DependencyProviderId,
        version: String,
        content_pin: ContentHash,
        declaration_id: String,
    },
    Host {
        provider: HostIdentity,
        version: String,
        content_identity: Option<ContentHash>,
        declaration_id: String,
    },
}

/// What selected-world data establishes before source-byte authentication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceStatus {
    /// The selected installed row carries a lock-bound logical content pin.
    LockBound,
    /// The selected host has a retained content identity, not byte CAS proof.
    HostRetained,
    /// The selected host has no retained content identity.
    UnverifiedHost,
}

/// A generated discovery descriptor and the source facts it must not hide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedSchema {
    pub descriptor: OperationDescriptor,
    pub source: SourceBinding,
    pub source_status: SourceStatus,
}

/// Why an owned declaration has no schema descriptor at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbsenceReason {
    NotDeclared,
}

/// Every selected declaration is observable. An absent table retains its
/// typed source binding but has no invented operation/model ID or descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MachineSchemaObservation {
    Declared(ProjectedSchema),
    Absent {
        source: SourceBinding,
        source_status: SourceStatus,
        reason: AbsenceReason,
    },
}

/// A malformed selected declaration cannot be silently listed as a validated
/// schema. The error contains no filesystem path or arbitrary provider data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectionError {
    InvalidDeclaration {
        declaration_id: String,
        reason: String,
    },
    UnregisteredFormat {
        declaration_id: String,
        format_id: String,
    },
    DuplicateIdentity {
        identity: String,
    },
}

impl fmt::Display for ProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDeclaration {
                declaration_id,
                reason,
            } => {
                write!(
                    f,
                    "extension {declaration_id} has invalid machine schema: {reason}"
                )
            }
            Self::UnregisteredFormat {
                declaration_id,
                format_id,
            } => write!(
                f,
                "extension {declaration_id} references unregistered epoch-one format {format_id}"
            ),
            Self::DuplicateIdentity { identity } => {
                write!(f, "duplicate selected-world machine identity {identity}")
            }
        }
    }
}

impl std::error::Error for ProjectionError {}

fn registered_epoch_one(id: &str) -> bool {
    FormatId::ALL
        .iter()
        .any(|format| format.id() == id && format.epoch() == 1)
}

fn check_schema(
    declaration: &ExtensionDecl,
    schema: &MachineOperationSchemaDecl,
    identities: &mut BTreeMap<String, String>,
    owner: &str,
) -> Result<(), ProjectionError> {
    schema
        .validate()
        .map_err(|reason| ProjectionError::InvalidDeclaration {
            declaration_id: declaration.id.clone(),
            reason,
        })?;
    for id in schema.format_ids() {
        if !registered_epoch_one(id) {
            return Err(ProjectionError::UnregisteredFormat {
                declaration_id: declaration.id.clone(),
                format_id: id.to_owned(),
            });
        }
    }
    for id in [schema.operation_id.as_ref(), schema.model_id.as_ref()]
        .into_iter()
        .flatten()
    {
        if identities.insert(id.clone(), owner.to_owned()).is_some() {
            return Err(ProjectionError::DuplicateIdentity {
                identity: id.clone(),
            });
        }
    }
    Ok(())
}

fn descriptor(
    declaration: &ExtensionDecl,
    schema: &MachineOperationSchemaDecl,
    source: &SourceBinding,
    source_status: SourceStatus,
) -> OperationDescriptor {
    let owner = match source {
        SourceBinding::Dependency { provider, .. } => format!("package:{provider}"),
        SourceBinding::Host { provider, .. } => format!("host:{provider}"),
    };
    let source_id = match source {
        SourceBinding::Dependency {
            provider, version, ..
        } => format!("extension:{provider}@{version}#{}", declaration.id),
        SourceBinding::Host {
            provider, version, ..
        } => format!("extension:host:{provider}@{version}#{}", declaration.id),
    };
    let reason = match source_status {
        SourceStatus::LockBound => {
            "Selected package has a lock-bound content pin; source bytes and domain service are not authenticated here"
        }
        SourceStatus::HostRetained => {
            "Selected host has a retained content identity; source bytes and domain service are not authenticated here"
        }
        SourceStatus::UnverifiedHost => {
            "Selected host lacks a retained content identity; schema source and service are unavailable"
        }
    };
    OperationDescriptor {
        schema: 1,
        operation_id: schema.operation_id.clone(),
        model_id: schema.model_id.clone(),
        domain_owner: owner,
        source_id,
        capability_id: None,
        input_format_id: schema.input_format_id.clone(),
        result_format_id: schema.result_format_id.clone(),
        error_format_id: schema.error_format_id.clone(),
        event_format_id: schema.event_format_id.clone(),
        implementation: Implementation::SourceObserved,
        availability: if source_status == SourceStatus::UnverifiedHost {
            Availability::Unavailable
        } else {
            Availability::NotObserved
        },
        authorization: Authorization::NotEvaluated,
        reason: Some(reason.to_owned()),
        implementation_evidence_id: None,
    }
}

/// Project already-selected retained declarations in installed lock order,
/// then host order. This performs no file read, schema fetch, native load, or
/// second collection and grants no callable extension service.
#[spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-EXTENSION-SCHEMA-SOURCE",
    r = 1
)]
pub fn project_machine_operation_schemas(
    world: &ExtensionWorld,
) -> Result<Vec<MachineSchemaObservation>, ProjectionError> {
    let mut projected = Vec::new();
    let mut identities = BTreeMap::new();
    for dependency in &world.installed {
        let owner = dependency.provider.id.to_string();
        for declaration in &dependency.declarations {
            let source = SourceBinding::Dependency {
                provider: dependency.provider.id.clone(),
                version: dependency.provider.version.clone(),
                content_pin: dependency.provider.content_hash.clone(),
                declaration_id: declaration.id.clone(),
            };
            match &declaration.machine_operation_schema {
                Some(schema) => {
                    check_schema(declaration, schema, &mut identities, &owner)?;
                    projected.push(MachineSchemaObservation::Declared(ProjectedSchema {
                        descriptor: descriptor(
                            declaration,
                            schema,
                            &source,
                            SourceStatus::LockBound,
                        ),
                        source,
                        source_status: SourceStatus::LockBound,
                    }));
                }
                None => projected.push(MachineSchemaObservation::Absent {
                    source,
                    source_status: SourceStatus::LockBound,
                    reason: AbsenceReason::NotDeclared,
                }),
            }
        }
    }
    let owner = world.host.provider.identity.to_string();
    for declaration in &world.host.declarations {
        let source = SourceBinding::Host {
            provider: world.host.provider.identity.clone(),
            version: world.host.provider.version.clone(),
            content_identity: world.host.provider.content_hash.clone(),
            declaration_id: declaration.id.clone(),
        };
        let source_status = if world.host.provider.content_hash.is_some() {
            SourceStatus::HostRetained
        } else {
            SourceStatus::UnverifiedHost
        };
        match &declaration.machine_operation_schema {
            Some(schema) => {
                check_schema(declaration, schema, &mut identities, &owner)?;
                projected.push(MachineSchemaObservation::Declared(ProjectedSchema {
                    descriptor: descriptor(declaration, schema, &source, source_status),
                    source,
                    source_status,
                }));
            }
            None => projected.push(MachineSchemaObservation::Absent {
                source,
                source_status,
                reason: AbsenceReason::NotDeclared,
            }),
        }
    }
    Ok(projected)
}

#[cfg(test)]
#[path = "machine_schema/tests.rs"]
mod tests;
