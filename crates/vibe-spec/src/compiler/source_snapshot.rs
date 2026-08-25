//! Immutable external observations for the named source-merge pass.

use std::collections::{BTreeMap, BTreeSet};

use crate::SpecAddress;

use super::ir::DocumentIr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DocumentObservation {
    Resolved(DocumentIr),
    Failed {
        requested: SpecAddress,
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExpansionObservation {
    Resolved {
        requested: SpecAddress,
        targets: Vec<SpecAddress>,
    },
    Failed {
        requested: SpecAddress,
        reason: String,
    },
}

/// One artifact-wide, immutable observation of every source input.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SourceResolutionSnapshot {
    pub(crate) discovery_order: Vec<String>,
    pub(crate) documents: BTreeMap<String, DocumentObservation>,
    pub(crate) expansions: BTreeMap<String, ExpansionObservation>,
    pub(crate) explicit_use_keys: BTreeSet<String>,
}

impl SourceResolutionSnapshot {
    pub(crate) fn document(&self, address: &SpecAddress) -> Option<&DocumentObservation> {
        self.documents.get(&address.without_pin())
    }

    pub(crate) fn resolved(&self, key: &str) -> Option<&DocumentIr> {
        match self.documents.get(key) {
            Some(DocumentObservation::Resolved(document)) => Some(document),
            _ => None,
        }
    }

    pub(crate) fn expansion(&self, address: &SpecAddress) -> Option<&ExpansionObservation> {
        self.expansions.get(&address.without_pin())
    }
}
