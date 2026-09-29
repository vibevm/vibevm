//! Compose selected-world declarations without collecting a second world.
//! The supplying caller still owns source-byte authentication.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-EXTENSION-SCHEMA-SOURCE");

use super::{MachineOperationError, MachineOperationService};
use vibe_lifecycle::ExtensionWorld;
use vibe_workspace::extension_world::machine_schema::{
    MachineSchemaObservation, project_machine_operation_schemas,
};

impl MachineOperationService {
    /// Add already-selected extension observations, preserving absent tables
    /// as typed source facts rather than inventing operation or model IDs.
    /// This never authenticates selected-world bytes or admits a handler.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-EXTENSION-SCHEMA-SOURCE",
        r = 1
    )]
    pub fn with_selected_world(
        self,
        world: &ExtensionWorld,
    ) -> Result<Self, MachineOperationError> {
        if !self.selected_world_observations.is_empty() {
            return Err(MachineOperationError::InvalidDescriptor(
                "a selected world was already attached to this service",
            ));
        }
        let observations = project_machine_operation_schemas(world)
            .map_err(MachineOperationError::SelectedWorldProjection)?;
        let mut descriptors = self.descriptors;
        descriptors.extend(
            observations
                .iter()
                .filter_map(|observation| match observation {
                    MachineSchemaObservation::Declared(row) => Some(row.descriptor.clone()),
                    MachineSchemaObservation::Absent { .. } => None,
                }),
        );
        // Reuse the one registry validator for descriptor shape, registered
        // formats and identity collisions, including with the pure planner.
        let mut service = Self::new(descriptors, self.planner_admission)?;
        service.selected_world_observations = observations;
        Ok(service)
    }

    /// Exact selected provider/declaration/source status for every row,
    /// including declarations with no machine schema. Neither these values
    /// nor `descriptors()` imply source-byte or effect authority.
    #[must_use]
    pub fn selected_world_observations(&self) -> &[MachineSchemaObservation] {
        &self.selected_world_observations
    }
}

#[cfg(test)]
#[path = "discovery/tests.rs"]
mod tests;
