//! Requested original-host facts over a finite complete selected observation.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-HOST-INVENTORY",
    r = 1
);
use super::declared::{DeclaredDocumentObservation, observe_source_document};
use super::*;
use vibe_spec::source_policy::PackagePolicyObservation;

pub(super) fn observe(
    root: &Path,
    coordinate: &str,
    policy: &PackagePolicyObservation,
) -> Result<AuthoredSourceObservation, RegistryError> {
    let captured = policy.capture_host_inventory(coordinate)?;
    let documents = captured
        .resources()
        .iter()
        .map(|resource| SourceFileWitness::of(resource.relative(), resource.bytes()))
        .collect::<Vec<_>>();
    let rows = policy
        .source_rows()
        .ok_or_else(|| policy.unsupported("host facts without explicit source rows"))?;
    let doc_allowed = policy
        .package()
        .is_some_and(|package| package.kind == vibe_core::PackageKind::Doc);
    let mut facts = Vec::new();
    let mut addresses = std::collections::BTreeSet::new();
    let mut native_documents = std::collections::BTreeSet::new();
    let mut issue = None;
    for resource in captured.resources() {
        let observation = observe_source_document(
            coordinate,
            rows,
            doc_allowed,
            resource.relative(),
            Some(resource.bytes()),
        )
        .map_err(|_| policy.unsupported("host facts source selection changed"))?;
        match observation {
            DeclaredDocumentObservation::Facts { facts: found, .. } => {
                if !native_documents.insert(vibe_spec::canonical_doc_path(resource.relative())) {
                    issue.get_or_insert_with(|| {
                        format!(
                            "native source address is ambiguous: {}",
                            resource.relative()
                        )
                    });
                    continue;
                }
                for fact in found {
                    if !addresses.insert(fact.address.clone()) {
                        issue.get_or_insert_with(|| {
                            format!("duplicate native fact address: {}", fact.address)
                        });
                        break;
                    }
                    facts.push(fact);
                }
            }
            DeclaredDocumentObservation::Invalid { reason, .. } => {
                issue.get_or_insert(reason);
            }
            DeclaredDocumentObservation::Raw { .. }
            | DeclaredDocumentObservation::OrdinaryMarkdown { .. }
            | DeclaredDocumentObservation::Unclassified { .. } => {}
            _ => {
                return Err(policy
                    .unsupported("host facts selected source has no supported native reader")
                    .into());
            }
        }
    }
    policy.recheck_host_inventory(&captured)?;
    Ok(match issue {
        Some(message) => AuthoredSourceObservation::Invalid {
            documents,
            issue: SourceIssue {
                path: root.into(),
                line: 0,
                message,
            },
        },
        None => {
            facts.sort_by(|a, b| a.address.cmp(&b.address));
            AuthoredSourceObservation::Available { facts, documents }
        }
    })
}
