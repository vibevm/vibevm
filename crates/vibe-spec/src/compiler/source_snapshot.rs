//! Immutable external observations for the named source-merge pass.

use std::collections::{BTreeMap, BTreeSet};

use crate::SpecAddress;

use super::ir::DocumentIr;

/// What one requested source input turned out to be.
///
/// The resolved payload is BOXED: a whole [`DocumentIr`] is an order of
/// magnitude larger than a failure's address plus reason, and every entry of
/// every snapshot map — resolved or failed — would otherwise be sized by the
/// document arm. It is the same indirection `ClosureContribution::Simple`
/// already makes for the same reason. [`DocumentObservation::resolved`] is the
/// constructor, so the representation choice stays inside this cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DocumentObservation {
    Resolved(Box<DocumentIr>),
    Failed {
        requested: SpecAddress,
        reason: String,
    },
}

impl DocumentObservation {
    /// Observe one resolved document.
    pub(crate) fn resolved(document: DocumentIr) -> Self {
        Self::Resolved(Box::new(document))
    }
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
        self.expansions.get(&address.to_string())
    }
}
