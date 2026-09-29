//! Descriptor-only extension metadata does not rewrite handler identity.

use super::declaration::{base_decl, host_row, prepared, project};
use vibe_core::manifest::MachineOperationSchemaDecl;

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-054#EXTENSION-OPERATION-SCHEMA",
    r = 1
)]
fn discovery_schema_metadata_preserves_frozen_execution_declaration_identity() {
    let root = project();
    let original = base_decl("phase:build");
    let before = prepared(root.path(), &host_row(original.clone()));
    let mut with_schema = original;
    with_schema.machine_operation_schema = Some(MachineOperationSchemaDecl {
        schema_epoch: 1,
        operation_id: Some("org.example.machine:plan".into()),
        model_id: None,
        input_format_id: Some("migration-plan-request".into()),
        result_format_id: None,
        error_format_id: None,
        event_format_id: None,
    });
    assert_eq!(
        prepared(root.path(), &host_row(with_schema)),
        before,
        "descriptor-only metadata cannot change the frozen handler fingerprint"
    );
}
