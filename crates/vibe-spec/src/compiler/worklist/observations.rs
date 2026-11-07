//! Discovery snapshots and an explicit pre-emission admission refusal.
use super::*;

pub(super) fn spec_order(order: &[DiscoveryKey]) -> Vec<String> {
    order
        .iter()
        .filter_map(|key| match key {
            DiscoveryKey::Spec(key) => Some(key.clone()),
            DiscoveryKey::Simple(_) => None,
        })
        .collect()
}

pub(super) fn observations(
    resolved: &BTreeMap<String, DocumentIr>,
    failures: &BTreeMap<String, DocumentObservation>,
) -> BTreeMap<String, DocumentObservation> {
    let mut out = failures.clone();
    out.extend(
        resolved
            .iter()
            .map(|(key, document)| (key.clone(), DocumentObservation::resolved(document.clone()))),
    );
    out
}

pub(super) fn refused_worklist(refusal: (SpecAddress, String), owners: ErrorOwners) -> Worklist {
    Worklist {
        documents: Vec::new(),
        sources: Default::default(),
        embeds: Default::default(),
        owners,
        reference_refusal: Some(refusal),
    }
}
