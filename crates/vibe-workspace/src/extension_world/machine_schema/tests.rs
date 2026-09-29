use super::*;
use vibe_core::manifest::{ExtensionsControl, Manifest};
use vibe_core::{Group, PackageKind, PackageName};
use vibe_extension_registry::{
    DependencyExtensionSource, DependencyProvider, HostExtensionSource, HostProvider,
};

const PACKAGE: &str = r#"
[package]
group = "org.example"
name = "schema-fixture"
kind = "tool"
version = "1.1.0"

[[extension]]
id = "native-discover"
point = "phase:build"
[extension.handler]
kind = "native"
crate_dir = "missing-native"
[extension.machine_operation_schema]
schema_epoch = 1
operation_id = "extension:org.example/schema-fixture:discover"
input_format_id = "machine-operation-envelope"
"#;

fn manifest() -> Manifest {
    Manifest::parse_str(PACKAGE).unwrap()
}

fn dependency(
    declarations: Vec<ExtensionDecl>,
    root: &std::path::Path,
) -> DependencyExtensionSource {
    DependencyExtensionSource {
        provider: DependencyProvider {
            id: DependencyProviderId::new(
                Group::parse("org.example").unwrap(),
                PackageName::parse("schema-fixture").unwrap(),
            ),
            root: root.to_path_buf(),
            version: "1.1.0".to_owned(),
            kind: PackageKind::Tool,
            content_hash: ContentHash::parse("sha256:aa").unwrap(),
        },
        declarations,
        controls: ExtensionsControl::default(),
        mechanisms: Vec::new(),
    }
}

fn host(
    declarations: Vec<ExtensionDecl>,
    content_identity: Option<ContentHash>,
) -> HostExtensionSource {
    HostExtensionSource {
        provider: HostProvider {
            identity: HostIdentity::ungrouped_project("host"),
            root: std::path::PathBuf::from("/not-read-host"),
            version: "1.1.0".to_owned(),
            kind: None,
            content_hash: content_identity,
        },
        declarations,
        controls: ExtensionsControl::default(),
        mechanisms: Vec::new(),
    }
}

fn world(
    dependencies: Vec<DependencyExtensionSource>,
    host_decls: Vec<ExtensionDecl>,
) -> ExtensionWorld {
    ExtensionWorld {
        installed: dependencies,
        host: host(host_decls, None),
        effective_stack: None,
    }
}

fn declared(observation: &MachineSchemaObservation) -> &ProjectedSchema {
    let MachineSchemaObservation::Declared(row) = observation else {
        panic!("expected a declared schema observation")
    };
    row
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-EXTENSION-SCHEMA-SOURCE",
    r = 1
)]
fn authored_native_schema_projects_from_lock_bound_world_without_loading_artifact() {
    let temporary = tempfile::tempdir().unwrap();
    let declaration = manifest().extensions.remove(0);
    let selected = world(
        vec![dependency(vec![declaration], temporary.path())],
        Vec::new(),
    );
    let projected = project_machine_operation_schemas(&selected).unwrap();
    assert_eq!(projected.len(), 1);
    let row = declared(&projected[0]);
    assert_eq!(row.source_status, SourceStatus::LockBound);
    assert!(matches!(row.source, SourceBinding::Dependency { .. }));
    assert_eq!(
        row.descriptor.implementation,
        Implementation::SourceObserved
    );
    assert_eq!(row.descriptor.availability, Availability::NotObserved);
    assert_eq!(row.descriptor.authorization, Authorization::NotEvaluated);
    assert_eq!(
        row.descriptor.input_format_id.as_deref(),
        Some("machine-operation-envelope")
    );
    assert!(!temporary.path().join("missing-native").exists());
    assert!(!temporary.path().join(".vibe").exists());
    assert!(!temporary.path().join("vibe.lock").exists());
    assert_eq!(std::fs::read_dir(temporary.path()).unwrap().count(), 0);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-054#EXTENSION-OPERATION-SCHEMA",
    r = 1
)]
fn unknown_pathlike_format_and_unsupported_epoch_refuse() {
    let root = std::path::Path::new("/unread-package");
    for format in ["unknown-format", "../schema/private.json", "foreign:format"] {
        let mut declaration = manifest().extensions.remove(0);
        declaration
            .machine_operation_schema
            .as_mut()
            .unwrap()
            .input_format_id = Some(format.to_owned());
        let selected = world(vec![dependency(vec![declaration], root)], Vec::new());
        assert!(matches!(
            project_machine_operation_schemas(&selected),
            Err(ProjectionError::UnregisteredFormat { .. })
        ));
    }
    let mut declaration = manifest().extensions.remove(0);
    declaration
        .machine_operation_schema
        .as_mut()
        .unwrap()
        .schema_epoch = 2;
    let selected = world(vec![dependency(vec![declaration], root)], Vec::new());
    assert!(matches!(
        project_machine_operation_schemas(&selected),
        Err(ProjectionError::InvalidDeclaration { .. })
    ));
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-EXTENSION-SCHEMA-SOURCE",
    r = 1
)]
fn cross_provider_operation_and_model_id_collisions_refuse() {
    let root = std::path::Path::new("/unread-package");
    let first = dependency(vec![manifest().extensions.remove(0)], root);
    let mut second = dependency(vec![manifest().extensions.remove(0)], root);
    second.provider.id = DependencyProviderId::new(
        Group::parse("org.example").unwrap(),
        PackageName::parse("second").unwrap(),
    );
    let selected = world(vec![first.clone(), second.clone()], Vec::new());
    assert!(matches!(
        project_machine_operation_schemas(&selected),
        Err(ProjectionError::DuplicateIdentity { .. })
    ));
    let schema = second.declarations[0]
        .machine_operation_schema
        .as_mut()
        .unwrap();
    schema.model_id = schema.operation_id.take();
    let selected = world(vec![first, second], Vec::new());
    assert!(matches!(
        project_machine_operation_schemas(&selected),
        Err(ProjectionError::DuplicateIdentity { .. })
    ));
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-EXTENSION-SCHEMA-SOURCE",
    r = 1
)]
fn host_without_content_identity_is_unverified_and_unavailable() {
    let selected = world(Vec::new(), vec![manifest().extensions.remove(0)]);
    let projected = project_machine_operation_schemas(&selected).unwrap();
    assert_eq!(projected.len(), 1);
    let row = declared(&projected[0]);
    assert_eq!(row.source_status, SourceStatus::UnverifiedHost);
    assert!(matches!(
        row.source,
        SourceBinding::Host {
            content_identity: None,
            ..
        }
    ));
    assert_eq!(
        row.descriptor.implementation,
        Implementation::SourceObserved
    );
    assert_eq!(row.descriptor.availability, Availability::Unavailable);
    assert_eq!(row.descriptor.authorization, Authorization::NotEvaluated);
    assert!(row.descriptor.reason.as_deref().unwrap().contains("lacks"));
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-DISCOVERY",
    r = 1
)]
fn absent_dependency_and_host_schema_remain_typed_unavailable_observations() {
    let temporary = tempfile::tempdir().unwrap();
    let mut dependency_decl = manifest().extensions.remove(0);
    dependency_decl.machine_operation_schema = None;
    let mut host_decl = dependency_decl.clone();
    host_decl.id = "host-without-schema".to_owned();
    let selected = world(
        vec![dependency(vec![dependency_decl], temporary.path())],
        vec![host_decl],
    );
    let observations = project_machine_operation_schemas(&selected).unwrap();
    assert_eq!(observations.len(), 2);
    assert!(matches!(
        &observations[0],
        MachineSchemaObservation::Absent {
            source: SourceBinding::Dependency { declaration_id, .. },
            source_status: SourceStatus::LockBound,
            reason: AbsenceReason::NotDeclared,
        } if declaration_id == "native-discover"
    ));
    assert!(matches!(
        &observations[1],
        MachineSchemaObservation::Absent {
            source: SourceBinding::Host { declaration_id, content_identity: None, .. },
            source_status: SourceStatus::UnverifiedHost,
            reason: AbsenceReason::NotDeclared,
        } if declaration_id == "host-without-schema"
    ));
    assert_eq!(std::fs::read_dir(temporary.path()).unwrap().count(), 0);
    assert!(!temporary.path().join("missing-native").exists());
}
