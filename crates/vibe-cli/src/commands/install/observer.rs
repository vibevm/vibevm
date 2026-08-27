//! The narration seam the slot lifecycle calls back into.
//!
//! It observes and renders human progress only: the machine document belongs
//! to the outermost command, which folds every row this observer saw into its
//! single report.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT");

use vibe_lifecycle::RunMetadata;

use crate::output;

pub(crate) struct LifecycleSlotObserver {
    ctx: output::Context,
    metadata: RunMetadata,
}

impl LifecycleSlotObserver {
    pub(crate) fn new(ctx: &output::Context, metadata: RunMetadata) -> Self {
        Self {
            ctx: ctx.clone(),
            metadata,
        }
    }
}

impl vibe_install::SlotLifecycleObserver for LifecycleSlotObserver {
    fn observe(&self, plan: &vibe_install::SlotLifecyclePlan) -> std::result::Result<(), String> {
        crate::commands::lifecycle::surface_slot_plan(&self.ctx, plan, &self.metadata)
            .map_err(|error| error.to_string())
    }

    fn outcome(
        &self,
        report: &vibe_install::SlotLifecycleReport,
    ) -> std::result::Result<(), String> {
        crate::commands::lifecycle::emit_slot_transition_outcome(&self.ctx, &self.metadata, report)
            .map_err(|error| error.to_string())
    }
}
