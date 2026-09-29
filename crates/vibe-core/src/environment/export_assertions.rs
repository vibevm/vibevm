specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-DEFINITION");
use super::{EnvironmentDefinition, composition, definition};

/// Called by the existing definition/lock case, not another ordinary test ID.
pub(super) fn check_export_namespaces() {
    let definition = definition();
    let mut shared = composition();
    shared.service_exports.push(shared.tool_exports[0].clone());
    assert!(EnvironmentDefinition::new(definition.id().clone(), shared.clone()).is_ok());

    let mut duplicate_tools = shared.clone();
    duplicate_tools
        .tool_exports
        .push(duplicate_tools.tool_exports[0].clone());
    assert_eq!(
        EnvironmentDefinition::new(definition.id().clone(), duplicate_tools)
            .unwrap_err()
            .field,
        "tool_exports"
    );

    let mut duplicate_services = shared;
    duplicate_services
        .service_exports
        .push(duplicate_services.service_exports[0].clone());
    assert_eq!(
        EnvironmentDefinition::new(definition.id().clone(), duplicate_services)
            .unwrap_err()
            .field,
        "service_exports"
    );
}
