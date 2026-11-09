//! One slot's global state and the accessors over it: the default
//! provider, the off switch (`PROP-059#slot-off`) and the durable PATH
//! entries its link directories needed (`PROP-059#path-directories`).

use specmark::spec;

use super::{EnvEntry, Index};

/// One slot's global state: the default provider, whether the slot is
/// switched off (`PROP-059#slot-off`), and the durable PATH entries vibe
/// applied for the slot's link directories (`PROP-059#path-directories`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#slot-off")]
pub struct SlotState {
    /// The installation key the slot resolves to while it is on. It is kept
    /// across an off switch, so turning the slot back on restores it.
    pub default: Option<String>,
    /// Switched off: the slot's link directories hold no link and its
    /// public commands have no proxy, so a lookup falls through to the
    /// system program.
    pub off: bool,
    /// The durable PATH entries applied for this slot's link directories,
    /// recorded so their removal takes out exactly them.
    pub path_env: Vec<EnvEntry>,
}

impl SlotState {
    /// A slot whose default is `default` and which is on with no applied
    /// PATH entries.
    pub fn with_default(default: Option<String>) -> Self {
        Self {
            default,
            ..Self::default()
        }
    }
}

impl Index {
    pub fn default_of(&self, slot: &str) -> Option<&str> {
        self.slots
            .get(slot)
            .and_then(|state| state.default.as_deref())
    }

    /// Whether `slot` is switched off (`PROP-059#slot-off`). An unknown slot
    /// is on: nothing has been turned off.
    pub fn is_off(&self, slot: &str) -> bool {
        self.slots.get(slot).is_some_and(|state| state.off)
    }

    /// The default `slot` resolves to while it is on, off state included.
    /// Lookup through the proxies and the link directories uses this: an
    /// off slot resolves to nothing.
    pub fn effective_default_of(&self, slot: &str) -> Option<&str> {
        self.slots
            .get(slot)
            .filter(|state| !state.off)
            .and_then(|state| state.default.as_deref())
    }

    /// Set `slot`'s default, keeping its off state and applied PATH
    /// entries. Every caller that moves a default uses this, so a switch
    /// can never drop the rest of a slot's state.
    pub fn set_default(&mut self, slot: &str, default: Option<String>) {
        self.slots.entry(slot.to_owned()).or_default().default = default;
    }

    /// `slot`'s state, created empty when the slot is new.
    pub fn slot_mut(&mut self, slot: &str) -> &mut SlotState {
        self.slots.entry(slot.to_owned()).or_default()
    }

    /// The durable PATH entries recorded for `slot`'s link directories
    /// (`PROP-059#path-directories`).
    pub fn slot_path_env(&self, slot: &str) -> &[EnvEntry] {
        self.slots
            .get(slot)
            .map_or(&[][..], |state| state.path_env.as_slice())
    }
}
