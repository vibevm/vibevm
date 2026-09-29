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
    assert_eq!(registry.models().len(), 29);
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
