use super::*;

fn authored() -> Coverage {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/capability-coverage/coverage-e1.json"
    )))
    .unwrap()
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-REGISTRY",
    r = 1
)]
fn current_catalogue_projects_all_declared_operations_and_models_as_gaps() {
    let coverage = authored();
    let registry = MachineCoverageRegistry::from_coverage(&coverage).unwrap();
    assert_eq!(registry.operations().len(), 214);
    assert_eq!(registry.models().len(), 35);
    assert!(registry.operations().iter().all(|gap| {
        gap.reason == ExtractionGapReason::UnboundDomainOwnerAndFormat && gap.bindings.len() == 1
    }));
    assert!(registry.models().iter().all(|gap| {
        gap.reason == ExtractionGapReason::UnboundDomainOwnerAndFormat
            && gap.bindings.len() == 1
            && gap.bindings[0].surface_id.is_none()
    }));
    let first = &registry.operations()[0].bindings[0];
    let authored_surface = coverage
        .surfaces
        .iter()
        .find(|surface| surface.surface_id == first.surface_id.as_deref().unwrap())
        .unwrap();
    assert_eq!(first.owner_prose, authored_surface.owner);
    assert_eq!(first.schema_ref_prose, authored_surface.schema_ref);
    assert_eq!(first.source_ids, authored_surface.source_ids);
    for (source_id, declared) in first.source_ids.iter().zip(&first.declared_sources) {
        let authored_source = coverage
            .sources
            .iter()
            .find(|source| &source.source_id == source_id)
            .unwrap();
        assert_eq!(declared.source_id, *source_id);
        assert_eq!(declared.path, authored_source.path);
        assert_eq!(declared.declared_sha256, authored_source.sha256);
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-REGISTRY",
    r = 1
)]
fn shared_surface_mapping_stays_unbound_and_cross_kind_collision_refuses() {
    let mut coverage = authored();
    let positions = coverage
        .surfaces
        .iter()
        .enumerate()
        .filter(|(_, row)| row.disposition == Disposition::Operation)
        .map(|(index, _)| index)
        .take(2)
        .collect::<Vec<_>>();
    let operation = coverage.surfaces[positions[0]]
        .operation_id
        .clone()
        .unwrap();
    coverage.surfaces[positions[1]].operation_id = Some(operation.clone());
    let registry = MachineCoverageRegistry::from_coverage(&coverage).unwrap();
    assert_eq!(registry.operations().len(), 213);
    let shared = registry
        .operations()
        .iter()
        .find(|gap| gap.identity == operation)
        .unwrap();
    assert_eq!(shared.bindings.len(), 2);
    assert_eq!(
        shared.reason,
        ExtractionGapReason::MultipleSurfaceBindingsUnbound
    );
    assert_eq!(
        shared.bindings[1].surface_id.as_deref(),
        Some(coverage.surfaces[positions[1]].surface_id.as_str())
    );

    coverage.models[0].model_id = operation.clone();
    assert_eq!(
        MachineCoverageRegistry::from_coverage(&coverage),
        Err(MachineCoverageError::OperationModelCollision(operation))
    );
    let mut malformed = authored();
    malformed.surfaces[positions[0]].source_ids[0] = "source:missing".to_owned();
    assert!(matches!(
        MachineCoverageRegistry::from_coverage(&malformed),
        Err(MachineCoverageError::InvalidInventory { .. })
    ));
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-REGISTRY",
    r = 1
)]
fn only_reviewed_mcp_planner_row_binds_with_separate_service_admission() {
    let coverage = authored();
    let registry = MachineCoverageRegistry::from_coverage(&coverage).unwrap();
    let admitted =
        MachineOperationService::pure_text_planner(PurePlannerAdmission::Admitted).unwrap();
    let PlannerCoverageBinding::Bound {
        coverage_identity,
        mapping,
        descriptor,
        source_bytes_verified,
    } = registry.reviewed_planner_binding(&admitted).unwrap()
    else {
        panic!("reviewed planner route was not bound")
    };
    assert_eq!(coverage_identity, REVIEWED_COVERAGE_ID);
    assert_eq!(mapping.surface_id.as_deref(), Some(REVIEWED_SURFACE_ID));
    assert_eq!(mapping.source_ids.len(), 4);
    assert_eq!(descriptor.operation_id.as_deref(), Some(PLANNER_ID));
    assert!(!source_bytes_verified);
    assert_eq!(
        registry
            .operations()
            .iter()
            .filter(|gap| gap.identity != REVIEWED_COVERAGE_ID)
            .count(),
        213
    );
    assert_eq!(registry.models().len(), 35);

    let unavailable =
        MachineOperationService::pure_text_planner(PurePlannerAdmission::Unavailable).unwrap();
    let PlannerCoverageBinding::Gap {
        coverage: unbound,
        service_descriptor,
        admission,
    } = registry.reviewed_planner_binding(&unavailable).unwrap()
    else {
        panic!("unadmitted descriptor became a route")
    };
    assert_eq!(unbound.identity, REVIEWED_COVERAGE_ID);
    assert_eq!(admission, PurePlannerAdmission::Unavailable);
    assert!(service_descriptor.is_some());

    let no_service =
        MachineOperationService::new(Vec::new(), PurePlannerAdmission::Admitted).unwrap();
    assert!(matches!(
        registry.reviewed_planner_binding(&no_service),
        Ok(PlannerCoverageBinding::Gap {
            service_descriptor: None,
            ..
        })
    ));
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-REGISTRY",
    r = 1
)]
fn reviewed_mapping_mismatch_refuses_without_fallback() {
    let service =
        MachineOperationService::pure_text_planner(PurePlannerAdmission::Admitted).unwrap();
    for mismatch in ["schema", "source", "surface", "duplicate"] {
        let mut coverage = authored();
        let index = coverage
            .surfaces
            .iter()
            .position(|row| row.operation_id.as_deref() == Some(REVIEWED_COVERAGE_ID))
            .unwrap();
        match mismatch {
            "schema" => coverage.surfaces[index].schema_ref = "unbound".to_owned(),
            "source" => {
                coverage.surfaces[index].source_ids.pop();
            }
            "surface" => coverage.surfaces[index].registered = false,
            "duplicate" => {
                let other = coverage
                    .surfaces
                    .iter_mut()
                    .find(|row| {
                        row.disposition == Disposition::Operation
                            && row.operation_id.as_deref() != Some(REVIEWED_COVERAGE_ID)
                    })
                    .unwrap();
                other.operation_id = Some(REVIEWED_COVERAGE_ID.to_owned());
            }
            _ => unreachable!(),
        }
        let result = MachineCoverageRegistry::from_coverage(&coverage)
            .and_then(|registry| registry.reviewed_planner_binding(&service));
        assert!(result.is_err(), "{mismatch} unexpectedly bound");
    }
}
