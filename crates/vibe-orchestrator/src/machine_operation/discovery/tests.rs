use super::*;
use crate::machine_operation::PurePlannerAdmission;
use vibe_core::manifest::{ExtensionsControl, Manifest};
use vibe_core::{ContentHash, Group, PackageKind, PackageName};
use vibe_lifecycle::{
    DependencyExtensionSource, DependencyProvider, DependencyProviderId, HostExtensionSource,
    HostIdentity, HostProvider,
};
use vibe_workspace::extension_world::machine_schema::{AbsenceReason, SourceBinding, SourceStatus};

fn selected_world(operation_id: &str) -> ExtensionWorld {
    let manifest = Manifest::parse_str(&format!(
        "[package]\ngroup = 'org.example'\nname = 'schema-package'\nkind = 'tool'\nversion = '1.1.0'\n\n[[extension]]\nid = 'declared'\npoint = 'phase:build'\nhandler = {{ kind = 'builtin', name = 'log' }}\n[extension.machine_operation_schema]\nschema_epoch = 1\noperation_id = '{operation_id}'\ninput_format_id = 'migration-plan-request'\n\n[[extension]]\nid = 'without-schema'\npoint = 'phase:test'\nhandler = {{ kind = 'builtin', name = 'log' }}\n"
    ))
    .unwrap();
    let package = DependencyExtensionSource {
        provider: DependencyProvider {
            id: DependencyProviderId::new(
                Group::parse("org.example").unwrap(),
                PackageName::parse("schema-package").unwrap(),
            ),
            root: "/never-opened-package".into(),
            version: "1.1.0".to_owned(),
            kind: PackageKind::Tool,
            content_hash: ContentHash::parse("sha256:aa").unwrap(),
        },
        declarations: manifest.extensions.clone(),
        controls: ExtensionsControl::default(),
        mechanisms: Vec::new(),
    };
    let mut host_decl = manifest.extensions[1].clone();
    host_decl.id = "host-without-schema".to_owned();
    ExtensionWorld {
        installed: vec![package],
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project("host"),
                root: "/never-opened-host".into(),
                version: "1.1.0".to_owned(),
                kind: None,
                content_hash: None,
            },
            declarations: vec![host_decl],
            controls: ExtensionsControl::default(),
            mechanisms: Vec::new(),
        },
        effective_stack: None,
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-EXTENSION-SCHEMA-SOURCE",
    r = 1
)]
fn selected_world_exposes_declared_and_absent_sources_without_a_callable_route() {
    let world = selected_world("extension:example:plan");
    let service = MachineOperationService::new(Vec::new(), PurePlannerAdmission::Admitted)
        .unwrap()
        .with_selected_world(&world)
        .unwrap();
    let observations = service.selected_world_observations();
    assert_eq!(observations.len(), 3);
    let MachineSchemaObservation::Declared(row) = &observations[0] else {
        panic!("first selected declaration was not retained")
    };
    assert_eq!(row.source_status, SourceStatus::LockBound);
    assert!(
        matches!(&row.source, SourceBinding::Dependency { declaration_id, .. } if declaration_id == "declared")
    );
    assert_eq!(service.descriptors(), &[row.descriptor.clone()]);
    assert!(matches!(
        service.clone().with_selected_world(&world),
        Err(MachineOperationError::InvalidDescriptor(_))
    ));
    for (observation, name) in [
        (&observations[1], "without-schema"),
        (&observations[2], "host-without-schema"),
    ] {
        assert!(
            matches!(
                observation,
                MachineSchemaObservation::Absent {
                    source: SourceBinding::Dependency { declaration_id, .. },
                    reason: AbsenceReason::NotDeclared,
                    ..
                } if declaration_id == name
            ) || matches!(
                observation,
                MachineSchemaObservation::Absent {
                    source: SourceBinding::Host { declaration_id, .. },
                    reason: AbsenceReason::NotDeclared,
                    ..
                } if declaration_id == name
            )
        );
    }
    assert!(matches!(
        service.plan_migration(
            "extension:example:plan",
            vibe_wire::generated::migration::e1::plan_request::PlanRequest {
                schema: 1,
                source_text: "not a manifest".to_owned(),
                source_profile: None,
                target_profile: None,
                recipe: None,
            }
        ),
        Err(MachineOperationError::UnavailableOperation(_))
    ));
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-REGISTRY",
    r = 1
)]
fn selected_extension_cannot_shadow_the_separately_admitted_planner() {
    let world = selected_world("rust:vibe-workspace:migration_planning");
    let service =
        MachineOperationService::pure_text_planner(PurePlannerAdmission::Admitted).unwrap();
    assert!(matches!(
        service.with_selected_world(&world),
        Err(MachineOperationError::DuplicateIdentity(_))
    ));
}
