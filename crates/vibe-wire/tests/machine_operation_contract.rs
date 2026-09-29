//! Machine operation generated-reader shape proof. Epoch values and embedded
//! payload_json are semantic obligations of the later facade/domain owner.
//! These tests neither dispatch an operation nor grant authority.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-VALIDATION");

use serde::de::DeserializeOwned;
use vibe_wire::generated::machine::e1::{
    operation_descriptor::{OperationDescriptor, OperationDescriptorImplementation},
    operation_envelope::{Body, OperationEnvelope},
};

#[path = "wire_support/mod.rs"]
mod wire_support;

fn fixture(family: &str, class: &str, name: &str) -> String {
    format!("formats/corpora/machine-operation-{family}/e1/{class}/{name}.json")
}

fn reject<T: DeserializeOwned>(family: &str, name: &str) {
    let path = fixture(family, "structural-invalid", name);
    let authored = wire_support::read_json(&path);
    assert!(
        serde_json::from_value::<T>(authored).is_err(),
        "{path} unexpectedly decoded"
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-REGISTRY",
    r = 1
)]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-ENVELOPE",
    r = 1
)]
fn descriptor_and_six_envelope_arms_roundtrip() {
    let descriptor_path = fixture("descriptor", "valid", "source-bound-migration-shape");
    let descriptor: OperationDescriptor = wire_support::round_trip(&descriptor_path);
    assert_eq!(descriptor.schema, 1);
    assert_eq!(
        descriptor.operation_id.as_deref(),
        Some("rust:vibe-workspace:migration_planning")
    );
    assert_eq!(descriptor.source_id, "pure-migration-transport-adapter");
    assert_eq!(
        descriptor.capability_id.as_deref(),
        Some("strict-text-only-migration-planning")
    );
    assert_eq!(
        descriptor.input_format_id.as_deref(),
        Some("migration-plan-request")
    );
    assert_eq!(
        descriptor.result_format_id.as_deref(),
        Some("migration-plan-report")
    );
    assert_eq!(
        descriptor.error_format_id.as_deref(),
        Some("migration-plan-error")
    );
    assert_eq!(
        descriptor.implementation,
        OperationDescriptorImplementation::SourceObserved
    );
    assert_eq!(
        serde_json::to_value(&descriptor).unwrap(),
        wire_support::read_json(&descriptor_path),
        "all descriptor states, source identities and nullable fields survive"
    );

    let model_path = fixture("descriptor", "valid", "model-only-source-gap");
    let model: OperationDescriptor = wire_support::round_trip(&model_path);
    assert!(
        model.operation_id.is_none(),
        "model has no invented operation"
    );
    assert_eq!(model.model_id.as_deref(), Some("model.action-invocation"));
    assert!(model.capability_id.is_none());
    assert_eq!(model.source_id, "source:crates/vibe-actions/src/invoke.rs");
    assert_eq!(
        model.implementation,
        OperationDescriptorImplementation::SourceObserved
    );
    assert_eq!(
        serde_json::to_value(model).unwrap(),
        wire_support::read_json(&model_path)
    );

    for (name, expected_stage) in [
        ("prepared", "prepared"),
        ("input", "input"),
        ("admission", "admission"),
        ("result", "result"),
        ("error", "error"),
        ("event", "event"),
    ] {
        let path = fixture("envelope", "valid", name);
        let envelope: OperationEnvelope = wire_support::round_trip(&path);
        assert_eq!(envelope.schema, 1, "{path}");
        assert_eq!(
            envelope.operation_id.as_deref(),
            Some("rust:vibe-workspace:migration_planning")
        );
        let observed_stage = match &envelope.body {
            Body::Prepared(_) => "prepared",
            Body::Input(_) => "input",
            Body::Admission(_) => "admission",
            Body::Result(_) => "result",
            Body::Error(_) => "error",
            Body::Event(_) => "event",
        };
        assert_eq!(observed_stage, expected_stage, "{path}");
        assert_eq!(
            serde_json::to_value(&envelope).unwrap(),
            wire_support::read_json(&path),
            "{path}: full generated arm, effect and completeness fields survive"
        );
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-VALIDATION",
    r = 1
)]
fn unknown_root_and_arm_fields_refuse() {
    for name in ["unknown-root", "wrong-type"] {
        reject::<OperationDescriptor>("descriptor", name);
        reject::<OperationEnvelope>("envelope", name);
    }
    reject::<OperationEnvelope>("envelope", "unknown-arm-field");
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-VALIDATION",
    r = 1
)]
fn required_nullable_omission_refuses_but_explicit_null_survives() {
    reject::<OperationDescriptor>("descriptor", "missing-required-nullable");
    reject::<OperationEnvelope>("envelope", "missing-required-nullable");

    let descriptor: OperationDescriptor = wire_support::round_trip(&fixture(
        "descriptor",
        "valid",
        "source-bound-migration-shape",
    ));
    assert!(descriptor.model_id.is_none());
    assert!(descriptor.reason.is_none());
    assert!(descriptor.implementation_evidence_id.is_none());
    // Both IDs are required keys; a model-only row supplies operation_id:null,
    // while the later owner rejects a descriptor with BOTH identities null.
    let model_path = fixture("descriptor", "valid", "model-only-source-gap");
    let model: OperationDescriptor = wire_support::round_trip(&model_path);
    assert!(model.operation_id.is_none());
    assert!(model.capability_id.is_none());
    for missing in ["operation_id", "capability_id"] {
        let mut row = wire_support::read_json(&model_path);
        row.as_object_mut().unwrap().remove(missing);
        assert!(serde_json::from_value::<OperationDescriptor>(row).is_err());
    }

    let path = fixture("envelope", "valid", "explicit-null-unavailable");
    let envelope: OperationEnvelope = wire_support::round_trip(&path);
    assert!(envelope.operation_id.is_none());
    assert!(envelope.domain_owner.is_none());
    assert!(envelope.source_id.is_none());
    assert!(envelope.evidence_id.is_none());
    let Body::Admission(admission) = &envelope.body else {
        panic!("explicit-null admission branch was not retained");
    };
    assert!(admission.reason.is_none());
    assert!(admission.authority_evidence_id.is_none());
    assert_eq!(
        serde_json::to_value(&envelope).unwrap(),
        wire_support::read_json(&path)
    );

    // The arm's nullable properties are required too; removing them is a
    // structural error even though explicit JSON null becomes Rust None.
    let mut missing_arm = wire_support::read_json(&path);
    missing_arm["body"]
        .as_object_mut()
        .unwrap()
        .remove("reason");
    assert!(serde_json::from_value::<OperationEnvelope>(missing_arm).is_err());
}
