//! Narrow placement boundaries retained by the existing steps owner.
use super::{AppError, Run};
impl Run<'_> {
    /// Persist a narrow operation boundary in the same steps owner.
    pub fn operation_phase(
        &mut self,
        phase: vibe_wire::generated::shared::ApplicationPendingOperationPhase,
    ) -> Result<(), AppError> {
        if let Some(operation) = &mut self.state.operation {
            operation.phase = phase;
            self.save()?;
        }
        Ok(())
    }
    pub fn has_placement(&self) -> bool {
        use vibe_wire::generated::shared::ApplicationPendingOperationPhase as P;
        self.state.operation.as_ref().is_some_and(|op| {
            matches!(
                op.phase,
                P::Placed | P::Publishing | P::Published | P::Complete
            )
        })
    }
}
