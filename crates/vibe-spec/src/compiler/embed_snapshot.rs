//! Immutable external observations for the named embed pass.

use std::collections::{BTreeMap, BTreeSet};

use crate::SpecAddress;

use super::source_snapshot::DocumentObservation;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EmbedResolutionSnapshot {
    pub(crate) discovery_order: Vec<String>,
    pub(crate) documents: BTreeMap<String, DocumentObservation>,
    pub(crate) explicit_use_keys: BTreeSet<String>,
}

impl EmbedResolutionSnapshot {
    pub(crate) fn document(&self, address: &SpecAddress) -> Option<&DocumentObservation> {
        self.documents.get(&address.without_pin())
    }
}
