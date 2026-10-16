//! Retained, typed machine-operation discovery and one pure domain route.
//!
//! Descriptors are caller-supplied data. This module neither collects a world
//! nor grants file, provider, process, or user authority from their contents.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-SERVICE");

use std::collections::BTreeMap;
use std::fmt;

use specmark::spec;
use vibe_wire::generated::format_id::FormatId;
use vibe_wire::generated::machine::e1::operation_descriptor::{
    OperationDescriptor, OperationDescriptorAuthorization as Authorization,
    OperationDescriptorAvailability as Availability,
    OperationDescriptorImplementation as Implementation,
};
use vibe_wire::generated::migration::e1::plan_request::PlanRequest;
use vibe_workspace::extension_world::machine_schema::{MachineSchemaObservation, ProjectionError};
use vibe_workspace::migration_planning::{
    MigrationPlanningResponse, plan_migration_request, validate_preflight_error,
    validate_report_for_request,
};

/// The one admitted pure route. The caller supplies this separately from
/// retained descriptor data; no descriptor can construct it by itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PurePlannerAdmission {
    Unavailable,
    Admitted,
}

/// A finite service refusal. No variant hides an effect or fabricates a
/// normal domain report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MachineOperationError {
    InvalidDescriptor(&'static str),
    SelectedWorldProjection(ProjectionError),
    DuplicateIdentity(String),
    UnsupportedEpoch,
    UnknownOperation(String),
    UnavailableOperation(String),
    InvalidDomainResponse { original: MigrationPlanningResponse },
}

impl fmt::Display for MachineOperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDescriptor(reason) => write!(f, "invalid operation descriptor: {reason}"),
            Self::SelectedWorldProjection(error) => write!(f, "selected world: {error}"),
            Self::DuplicateIdentity(id) => write!(f, "duplicate machine identity: {id}"),
            Self::UnsupportedEpoch => write!(f, "unsupported machine operation epoch"),
            Self::UnknownOperation(id) => write!(f, "unknown machine operation: {id}"),
            Self::UnavailableOperation(id) => write!(f, "machine operation unavailable: {id}"),
            Self::InvalidDomainResponse { .. } => {
                write!(f, "domain response failed owning validation")
            }
        }
    }
}

impl std::error::Error for MachineOperationError {}

const PLANNER_ID: &str = "rust:vibe-workspace:migration_planning";
const PLANNER_SOURCE: &str = "pure-migration-transport-adapter";
const PLANNER_CAPABILITY: &str = "strict-text-only-migration-planning";
// The accepted release-impact subject, not a newly inferred runtime probe.
const PLANNER_EVIDENCE: &str = "48a29c431fedfabb0b43e9ea6a5e9a938c647f88";

/// Retained discovery rows plus a separately supplied pure-planner admission.
/// This sibling never enters RitualPlan or provider/world collection.
#[derive(Debug, Clone)]
pub struct MachineOperationService {
    descriptors: Vec<OperationDescriptor>,
    selected_world_observations: Vec<MachineSchemaObservation>,
    planner_admission: PurePlannerAdmission,
}

fn nonblank(value: &str) -> bool {
    !value.is_empty() && value.trim() == value && !value.chars().any(char::is_control)
}

fn namespaced(value: &str) -> bool {
    nonblank(value)
        && (value.contains(':') || value.contains('.'))
        && !value.chars().any(char::is_whitespace)
}

fn registered_format(value: &str) -> bool {
    FormatId::ALL
        .iter()
        .any(|format| format.id() == value && format.epoch() == 1)
}

fn validate_descriptor(row: &OperationDescriptor) -> Result<(), MachineOperationError> {
    if row.schema != 1 {
        return Err(MachineOperationError::UnsupportedEpoch);
    }
    if !namespaced(&row.domain_owner)
        || !nonblank(&row.source_id)
        || row.source_id == row.domain_owner
    {
        return Err(MachineOperationError::InvalidDescriptor(
            "domain owner and distinct source identity are required",
        ));
    }
    if row.operation_id.is_none() && row.model_id.is_none() {
        return Err(MachineOperationError::InvalidDescriptor(
            "operation or model identity is required",
        ));
    }
    for id in [&row.operation_id, &row.model_id] {
        if id.as_deref().is_some_and(|value| !namespaced(value)) {
            return Err(MachineOperationError::InvalidDescriptor(
                "operation and model identities must be namespaced",
            ));
        }
    }
    for value in [
        &row.capability_id,
        &row.reason,
        &row.implementation_evidence_id,
    ] {
        if value.as_deref().is_some_and(|text| !nonblank(text)) {
            return Err(MachineOperationError::InvalidDescriptor(
                "present capability, reason and evidence identities must be nonblank",
            ));
        }
    }
    if row.capability_id.as_deref() == Some(row.source_id.as_str()) {
        return Err(MachineOperationError::InvalidDescriptor(
            "capability and source identities must remain distinct",
        ));
    }
    for value in [
        &row.input_format_id,
        &row.result_format_id,
        &row.error_format_id,
        &row.event_format_id,
    ] {
        if value.as_deref().is_some_and(|id| !registered_format(id)) {
            return Err(MachineOperationError::InvalidDescriptor(
                "format reference is not a registered epoch-one format",
            ));
        }
    }
    match row.implementation {
        Implementation::Validated if row.implementation_evidence_id.is_none() => {
            return Err(MachineOperationError::InvalidDescriptor(
                "validated implementation requires evidence identity",
            ));
        }
        Implementation::Gap if row.reason.is_none() => {
            return Err(MachineOperationError::InvalidDescriptor(
                "extraction gap requires a reason",
            ));
        }
        Implementation::SourceObserved | Implementation::Gap
            if row.availability == Availability::Available
                || row.authorization == Authorization::Admitted =>
        {
            return Err(MachineOperationError::InvalidDescriptor(
                "unvalidated source cannot be available or admitted",
            ));
        }
        _ => {}
    }
    if row.authorization == Authorization::Admitted && row.availability != Availability::Available {
        return Err(MachineOperationError::InvalidDescriptor(
            "admission requires separately recorded availability",
        ));
    }
    Ok(())
}

fn recognized_planner_identity(row: &OperationDescriptor) -> bool {
    row.operation_id.as_deref() == Some(PLANNER_ID)
        && row.domain_owner == PLANNER_ID
        && row.source_id == PLANNER_SOURCE
        && row.capability_id.as_deref() == Some(PLANNER_CAPABILITY)
        && row.implementation_evidence_id.as_deref() == Some(PLANNER_EVIDENCE)
        && row.model_id.is_none()
        && row.input_format_id.as_deref() == Some("migration-plan-request")
        && row.result_format_id.as_deref() == Some("migration-plan-report")
        && row.error_format_id.as_deref() == Some("migration-plan-error")
        && row.event_format_id.is_none()
        && row.implementation == Implementation::Validated
}

fn recognized_planner(row: &OperationDescriptor) -> bool {
    recognized_planner_identity(row)
        && row.availability == Availability::Available
        && row.authorization == Authorization::Admitted
}

/// Keep the exact produced value on an invariant failure. Surfaces may
/// render it only as diagnostic, unvalidated evidence, never a normal report.
fn validated_response(
    request: &PlanRequest,
    response: MigrationPlanningResponse,
) -> Result<MigrationPlanningResponse, MachineOperationError> {
    let valid = match &response {
        MigrationPlanningResponse::Report(report) => {
            validate_report_for_request(request, report).is_ok()
        }
        MigrationPlanningResponse::Preflight(error) => validate_preflight_error(error).is_ok(),
    };
    if !valid {
        return Err(MachineOperationError::InvalidDomainResponse { original: response });
    }
    Ok(response)
}

impl MachineOperationService {
    /// Compose the accepted text-only planner after an explicit pure request.
    /// This embeds its reviewed source/format identities; no descriptor or
    /// client payload can choose another callable route or obtain a file grant.
    pub fn pure_text_planner(
        planner_admission: PurePlannerAdmission,
    ) -> Result<Self, MachineOperationError> {
        let row = OperationDescriptor {
            schema: 1,
            operation_id: Some(PLANNER_ID.to_owned()),
            domain_owner: PLANNER_ID.to_owned(),
            source_id: PLANNER_SOURCE.to_owned(),
            capability_id: Some(PLANNER_CAPABILITY.to_owned()),
            model_id: None,
            input_format_id: Some("migration-plan-request".to_owned()),
            result_format_id: Some("migration-plan-report".to_owned()),
            error_format_id: Some("migration-plan-error".to_owned()),
            event_format_id: None,
            implementation: Implementation::Validated,
            availability: Availability::Available,
            authorization: if planner_admission == PurePlannerAdmission::Admitted {
                Authorization::Admitted
            } else {
                Authorization::NotEvaluated
            },
            reason: None,
            implementation_evidence_id: Some(PLANNER_EVIDENCE.to_owned()),
        };
        Self::new(vec![row], planner_admission)
    }

    /// Admit already retained typed data for discovery. This checks metadata
    /// coherence only; the supplying owner still authenticates its source.
    #[spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-REGISTRY",
        r = 1
    )]
    pub fn new(
        descriptors: Vec<OperationDescriptor>,
        planner_admission: PurePlannerAdmission,
    ) -> Result<Self, MachineOperationError> {
        let mut identities = BTreeMap::<&str, &str>::new();
        for row in &descriptors {
            validate_descriptor(row)?;
            for id in [&row.operation_id, &row.model_id].into_iter().flatten() {
                if identities.insert(id, &row.domain_owner).is_some() {
                    return Err(MachineOperationError::DuplicateIdentity(id.clone()));
                }
            }
            if row.operation_id.as_deref() != Some(PLANNER_ID)
                && (row.implementation == Implementation::Validated
                    || row.availability == Availability::Available
                    || row.authorization == Authorization::Admitted)
            {
                return Err(MachineOperationError::InvalidDescriptor(
                    "no shared domain route owns a validated or available foreign operation",
                ));
            }
            if row.operation_id.as_deref() == Some(PLANNER_ID)
                && row.implementation == Implementation::Validated
                && !recognized_planner_identity(row)
            {
                return Err(MachineOperationError::InvalidDescriptor(
                    "planner route does not match its accepted source and formats",
                ));
            }
        }
        Ok(Self {
            descriptors,
            selected_world_observations: Vec::new(),
            planner_admission,
        })
    }

    /// Discovery never invokes a handler, loads a provider, or fetches a
    /// contributed schema. Missing extension schema producers stay gap rows.
    #[must_use]
    pub fn descriptors(&self) -> &[OperationDescriptor] {
        &self.descriptors
    }

    /// The sole callable route in this atom delegates to the existing pure
    /// planner and rechecks its exact report or preflight root before return.
    #[spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-VALIDATION",
        r = 1
    )]
    pub fn plan_migration(
        &self,
        operation_id: &str,
        request: PlanRequest,
    ) -> Result<MigrationPlanningResponse, MachineOperationError> {
        if request.schema != 1 {
            return Err(MachineOperationError::UnsupportedEpoch);
        }
        let row = self
            .descriptors
            .iter()
            .find(|row| row.operation_id.as_deref() == Some(operation_id))
            .ok_or_else(|| MachineOperationError::UnknownOperation(operation_id.to_owned()))?;
        if operation_id != PLANNER_ID
            || self.planner_admission != PurePlannerAdmission::Admitted
            || !recognized_planner(row)
        {
            return Err(MachineOperationError::UnavailableOperation(
                operation_id.to_owned(),
            ));
        }
        validated_response(&request, plan_migration_request(request.clone()))
    }
}

pub mod adapter_plan;
pub mod boot_debugger;
mod coverage;
mod discovery;
pub mod exact_local_install;
pub mod exchange;
pub mod fixed_cargo;
pub mod fixed_deploy;
pub mod fixed_lifecycle;
pub mod fixed_package;
pub mod input_decision;
pub mod lifecycle_view;
mod publication;
pub mod stream_message;
pub use coverage::*;
pub use publication::MachineEnvelopeValidationError;

#[cfg(test)]
mod tests {
    use super::*;

    fn source_row() -> OperationDescriptor {
        serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../formats/corpora/machine-operation-descriptor/e1/valid/source-bound-migration-shape.json"
        )))
        .unwrap()
    }

    fn admitted_row() -> OperationDescriptor {
        let mut row = source_row();
        row.implementation = Implementation::Validated;
        row.availability = Availability::Available;
        row.authorization = Authorization::Admitted;
        row.implementation_evidence_id = Some(PLANNER_EVIDENCE.to_owned());
        row
    }

    fn request() -> PlanRequest {
        PlanRequest {
            schema: 1,
            source_text: "not a manifest".to_owned(),
            recipe: None,
            source_profile: None,
            target_profile: None,
            source_decisions: Vec::new(),
        }
    }

    #[test]
    fn discovery_keeps_source_and_model_gaps_without_execution() {
        let model: OperationDescriptor = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../formats/corpora/machine-operation-descriptor/e1/valid/model-only-source-gap.json"
        )))
        .unwrap();
        let service =
            MachineOperationService::new(vec![source_row(), model], PurePlannerAdmission::Admitted)
                .unwrap();
        assert_eq!(service.descriptors().len(), 2);
        assert_eq!(service.descriptors()[1].operation_id, None);
        assert!(matches!(
            service.plan_migration(PLANNER_ID, request()),
            Err(MachineOperationError::UnavailableOperation(_))
        ));
    }

    #[test]
    fn descriptor_duplicate_epoch_and_unregistered_format_refuse() {
        let row = source_row();
        assert!(matches!(
            MachineOperationService::new(
                vec![row.clone(), row.clone()],
                PurePlannerAdmission::Unavailable
            ),
            Err(MachineOperationError::DuplicateIdentity(_))
        ));
        let mut bad_epoch = row.clone();
        bad_epoch.schema = 2;
        assert!(matches!(
            MachineOperationService::new(vec![bad_epoch], PurePlannerAdmission::Unavailable),
            Err(MachineOperationError::UnsupportedEpoch)
        ));
        let mut bad_format = row;
        bad_format.input_format_id = Some("unregistered-domain-format".to_owned());
        assert!(matches!(
            MachineOperationService::new(vec![bad_format], PurePlannerAdmission::Unavailable),
            Err(MachineOperationError::InvalidDescriptor(_))
        ));
        let mut false_implementation = source_row();
        false_implementation.operation_id = Some("rust:foreign:unbound".to_owned());
        false_implementation.implementation = Implementation::Validated;
        false_implementation.implementation_evidence_id = Some(PLANNER_EVIDENCE.to_owned());
        assert!(matches!(
            MachineOperationService::new(
                vec![false_implementation],
                PurePlannerAdmission::Unavailable
            ),
            Err(MachineOperationError::InvalidDescriptor(_))
        ));
    }

    #[test]
    fn admitted_pure_planner_matches_owning_report_and_replay_validator() {
        let service =
            MachineOperationService::pure_text_planner(PurePlannerAdmission::Admitted).unwrap();
        assert_eq!(service.descriptors(), &[admitted_row()]);
        let request = request();
        let expected = plan_migration_request(request.clone());
        let actual = service.plan_migration(PLANNER_ID, request.clone()).unwrap();
        assert_eq!(actual, expected);
        match actual {
            MigrationPlanningResponse::Report(report) => {
                validate_report_for_request(&request, &report).unwrap();
            }
            MigrationPlanningResponse::Preflight(error) => {
                validate_preflight_error(&error).unwrap();
            }
        }
    }

    #[test]
    fn arbitrary_route_and_descriptor_without_separate_admission_refuse() {
        let service =
            MachineOperationService::new(vec![admitted_row()], PurePlannerAdmission::Unavailable)
                .unwrap();
        assert!(matches!(
            service.plan_migration("eval:shell", request()),
            Err(MachineOperationError::UnknownOperation(_))
        ));
        assert!(matches!(
            service.plan_migration(PLANNER_ID, request()),
            Err(MachineOperationError::UnavailableOperation(_))
        ));
        let mut unsupported = request();
        unsupported.schema = 2;
        assert_eq!(
            service.plan_migration(PLANNER_ID, unsupported),
            Err(MachineOperationError::UnsupportedEpoch)
        );
        let mut unavailable = admitted_row();
        unavailable.availability = Availability::Unavailable;
        unavailable.authorization = Authorization::Denied;
        let service =
            MachineOperationService::new(vec![unavailable], PurePlannerAdmission::Admitted)
                .unwrap();
        assert!(matches!(
            service.plan_migration(PLANNER_ID, request()),
            Err(MachineOperationError::UnavailableOperation(_))
        ));
    }

    #[test]
    fn invalid_domain_report_retains_original_unvalidated_evidence() {
        let request = request();
        let mut original = plan_migration_request(request.clone());
        match &mut original {
            MigrationPlanningResponse::Report(report) => report.schema = 2,
            MigrationPlanningResponse::Preflight(error) => error.schema = 2,
        }
        assert_eq!(
            validated_response(&request, original.clone()),
            Err(MachineOperationError::InvalidDomainResponse { original })
        );
    }
}

pub mod client_deploy;
