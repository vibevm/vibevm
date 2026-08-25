//! Owned canonical handler transition input.

use crate::ExtensionRegistryRow;
use specmark::spec;
pub use vibe_wire::generated::lifecycle::e1::context::SlotTarget;

#[derive(Debug, Clone)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENVELOPE-MAPPING")]
pub struct HandlerExecution {
    row: ExtensionRegistryRow,
    slot_target: Option<SlotTarget>,
}

impl HandlerExecution {
    #[must_use]
    pub fn from_row(row: &ExtensionRegistryRow) -> Self {
        Self {
            row: row.clone(),
            slot_target: None,
        }
    }

    #[must_use]
    pub const fn row(&self) -> &ExtensionRegistryRow {
        &self.row
    }

    #[must_use]
    pub fn key(&self) -> String {
        match &self.slot_target {
            Some(target) => format!(
                "{}@slot({}/{}@{})",
                self.row.key(),
                target.group,
                target.name,
                target.version
            ),
            None => self.reference(),
        }
    }

    #[must_use]
    pub fn reference(&self) -> String {
        self.row.key().to_string()
    }

    #[must_use]
    pub fn with_slot_target(mut self, target: SlotTarget) -> Self {
        self.slot_target = Some(target);
        self
    }

    #[must_use]
    pub const fn slot_target(&self) -> Option<&SlotTarget> {
        self.slot_target.as_ref()
    }
}
