//! Manifest-local extension machine-schema grammar; no registry or native load.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#EXTENSION-OPERATION-SCHEMA");
use super::*;
use crate::manifest::Manifest;

const BASE: &str = "[project]\nname = 'demo'\nversion = '1.1.0'\n\n[[extension]]\nid = 'one'\npoint = 'phase:build'\nhandler = { kind = 'builtin', name = 'log' }\n";
fn authored(table: &str) -> String {
    format!("{BASE}\n[extension.machine_operation_schema]\n{table}\n")
}
fn refused(text: &str) -> String {
    Manifest::parse_str(text).unwrap_err().to_string()
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-054#EXTENSION-OPERATION-SCHEMA",
    r = 1
)]
fn authored_operation_and_model_tables_roundtrip_without_authentication() {
    let manifest = Manifest::parse_str(&authored(
        "schema_epoch = 1\noperation_id = 'rust:demo:operation'\nmodel_id = 'model.demo-result'\ninput_format_id = 'migration-plan-request'\nresult_format_id = 'migration-plan-report'",
    )).unwrap();
    let schema = manifest.extensions[0]
        .machine_operation_schema
        .as_ref()
        .unwrap();
    assert_eq!(schema.schema_epoch, 1);
    assert_eq!(schema.operation_id.as_deref(), Some("rust:demo:operation"));
    assert_eq!(schema.model_id.as_deref(), Some("model.demo-result"));
    assert_eq!(
        schema.format_ids().collect::<Vec<_>>(),
        ["migration-plan-request", "migration-plan-report"]
    );
    assert!(manifest.extensions[0].config.is_none());
    let rendered = toml::to_string_pretty(&manifest).unwrap();
    assert!(rendered.contains("machine_operation_schema"));
    let reparsed = Manifest::parse_str(&rendered).unwrap();
    assert_eq!(reparsed.extensions[0], manifest.extensions[0]);

    let model = Manifest::parse_str(&authored(
        "schema_epoch = 1\nmodel_id = 'model.demo-only'\nevent_format_id = 'machine-operation-envelope'",
    )).unwrap();
    assert!(
        model.extensions[0]
            .machine_operation_schema
            .as_ref()
            .unwrap()
            .operation_id
            .is_none()
    );
    // Core retains a name as authored data. Known host FormatId/epoch is a
    // separate selected-world decision, so this is not a schema admission.
    let future = Manifest::parse_str(&authored(
        "schema_epoch = 1\nmodel_id = 'model.future'\ninput_format_id = 'future-host-format'",
    ))
    .unwrap();
    assert_eq!(
        future.extensions[0]
            .machine_operation_schema
            .as_ref()
            .unwrap()
            .input_format_id
            .as_deref(),
        Some("future-host-format")
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-054#EXTENSION-OPERATION-SCHEMA",
    r = 1
)]
fn absent_table_stays_absent_and_handler_config_cannot_promote_it() {
    let old = format!(
        "{BASE}config = {{ machine_operation_schema = {{ schema_epoch = 1, operation_id = 'rust:fake', input_format_id = 'migration-plan-request' }} }}\n"
    );
    let manifest = Manifest::parse_str(&old).unwrap();
    assert!(manifest.extensions[0].machine_operation_schema.is_none());
    assert!(
        manifest.extensions[0]
            .config
            .as_ref()
            .unwrap()
            .as_table()
            .contains_key("machine_operation_schema")
    );
    let rendered = toml::to_string_pretty(&manifest).unwrap();
    assert!(!rendered.contains("[extension.machine_operation_schema]"));
    let reparsed = Manifest::parse_str(&rendered).unwrap();
    assert_eq!(reparsed.extensions[0], manifest.extensions[0]);
    assert_eq!(toml::to_string_pretty(&reparsed).unwrap(), rendered);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-054#EXTENSION-OPERATION-SCHEMA",
    r = 1
)]
fn unknown_type_epoch_missing_ids_and_whitespace_refuse_locally() {
    for (table, needle) in [
        (
            "schema_epoch = 1\noperation_id = 'rust:demo'\ninput_format_id = 'migration-plan-request'\nextra = true",
            "unknown field",
        ),
        (
            "schema_epoch = '1'\noperation_id = 'rust:demo'\ninput_format_id = 'migration-plan-request'",
            "schema_epoch",
        ),
        (
            "schema_epoch = 2\noperation_id = 'rust:demo'\ninput_format_id = 'migration-plan-request'",
            "schema_epoch",
        ),
        (
            "schema_epoch = 1\ninput_format_id = 'migration-plan-request'",
            "operation_id",
        ),
        (
            "schema_epoch = 1\noperation_id = 'plain'\ninput_format_id = 'migration-plan-request'",
            "operation_id",
        ),
        (
            "schema_epoch = 1\noperation_id = 'rust:demo bad'\ninput_format_id = 'migration-plan-request'",
            "operation_id",
        ),
        (
            "schema_epoch = 1\noperation_id = ''\ninput_format_id = 'migration-plan-request'",
            "operation_id",
        ),
        ("schema_epoch = 1\noperation_id = 'rust:demo'", "format ID"),
        (
            "schema_epoch = 1\noperation_id = 'rust:demo'\ninput_format_id = 'bad id'",
            "input_format_id",
        ),
        (
            "schema_epoch = 1\noperation_id = 'rust:demo'\ninput_format_id = 5",
            "input_format_id",
        ),
        (
            "schema_epoch = 1\noperation_id = 'rust:demo'\ninput_format_id = 'migration-plan-request'\ndomain_owner = 'forged'",
            "unknown field",
        ),
    ] {
        let error = refused(&authored(table));
        assert!(error.contains(needle), "expected {needle:?}: {error}");
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-054#EXTENSION-OPERATION-SCHEMA",
    r = 1
)]
fn duplicate_machine_identity_and_programmatic_invalid_values_refuse() {
    let first = authored(
        "schema_epoch = 1\noperation_id = 'rust:duplicate'\ninput_format_id = 'migration-plan-request'",
    );
    for (second_key, id) in [
        ("operation_id", "rust:duplicate"),
        ("model_id", "rust:duplicate"),
    ] {
        let second = format!(
            "{first}\n[[extension]]\nid = 'two'\npoint = 'phase:test'\nhandler = {{ kind = 'builtin', name = 'log' }}\n[extension.machine_operation_schema]\nschema_epoch = 1\n{second_key} = '{id}'\nresult_format_id = 'migration-plan-report'\n"
        );
        assert!(refused(&second).contains("duplicate `machine_operation_schema`"));
    }
    let mut manifest = Manifest::parse_str(BASE).unwrap();
    manifest.extensions[0].machine_operation_schema = Some(MachineOperationSchemaDecl {
        schema_epoch: 2,
        operation_id: Some("rust:invalid".into()),
        model_id: None,
        input_format_id: Some("migration-plan-request".into()),
        result_format_id: None,
        error_format_id: None,
        event_format_id: None,
    });
    assert!(
        manifest
            .validate()
            .unwrap_err()
            .to_string()
            .contains("schema_epoch")
    );
    assert!(
        toml::to_string_pretty(&manifest)
            .unwrap_err()
            .to_string()
            .contains("schema_epoch")
    );
}
