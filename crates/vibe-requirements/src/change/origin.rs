//! Exact authored declarations joined to the incumbent retained M01 observation.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-RETAINED-ORIGIN");
use std::{collections::BTreeMap, sync::Arc};
use vibe_core::{
    PackageRef,
    manifest::change::{ChangeOrigin, ChangeOriginDocument},
};
use vibe_workspace::source_query::{CanonicalNodeIdentity, RetainedOriginCapture, SourceStrength};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OriginUnavailable {
    Unbound,
    ContentMismatch,
    AlreadyBound,
    Selection,
    MissingRetained,
    MissingCurrent,
    Pin,
    Manifest,
    Document(String),
    OriginalUnavailable(String),
    Fact(String),
    Coordinate,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginFactIdentity {
    pub document_sha256: String,
    pub canonical: CanonicalNodeIdentity,
    pub source_path: String,
    pub output_path: String,
    pub materialized_media: Option<String>,
    pub recorded_converter_recipe: Option<String>,
    pub strength: SourceStrength,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginCaptureIdentity {
    pub package: String,
    /// Independent selected-world identity, not proof by an editable row.
    pub selected_content_hash: String,
    pub manifest_sha256: String,
    pub facts: BTreeMap<String, OriginFactIdentity>,
}
#[derive(Debug, Clone)]
pub struct AcceptedOrigin {
    declaration: ChangeOrigin,
    capture: Arc<RetainedOriginCapture>,
    identity: OriginCaptureIdentity,
}
impl AcceptedOrigin {
    pub fn declaration(&self) -> &ChangeOrigin {
        &self.declaration
    }
    pub fn capture(&self) -> &RetainedOriginCapture {
        &self.capture
    }
    pub fn identity(&self) -> &OriginCaptureIdentity {
        &self.identity
    }
}
fn identity(
    capture: &RetainedOriginCapture,
    addresses: impl Iterator<Item = String>,
) -> Result<OriginCaptureIdentity, OriginUnavailable> {
    let mut facts = BTreeMap::new();
    for address in addresses {
        let fact = capture
            .facts()
            .get(&address)
            .ok_or_else(|| OriginUnavailable::Fact(address.clone()))?;
        let canonical = fact
            .canonical
            .clone()
            .ok_or_else(|| OriginUnavailable::Fact(address.clone()))?;
        let doc = capture
            .documents()
            .get(&fact.document)
            .ok_or_else(|| OriginUnavailable::Document(address.clone()))?;
        let provenance = doc.provenance();
        if matches!(
            doc.disposition(),
            Some(vibe_workspace::vibedeps::SlotFileDisposition::Converted)
        ) {
            return Err(OriginUnavailable::OriginalUnavailable(address));
        }
        facts.insert(
            address,
            OriginFactIdentity {
                document_sha256: format!("sha256:{}", doc.snapshot().sha256),
                canonical,
                source_path: provenance.source_path.clone(),
                output_path: provenance.output_path.clone(),
                materialized_media: provenance.materialized_media.clone(),
                recorded_converter_recipe: provenance.recorded_converter_recipe.clone(),
                strength: provenance.strength,
            },
        );
    }
    Ok(OriginCaptureIdentity {
        package: capture.pin().to_string(),
        selected_content_hash: capture.content_hash().to_string(),
        manifest_sha256: format!("sha256:{}", capture.manifest().sha256),
        facts,
    })
}
/// Missing retained evidence is never replaced with a current/latest capture.
pub fn accept_origin(
    declaration: &ChangeOrigin,
    retained: Option<Arc<RetainedOriginCapture>>,
) -> Result<AcceptedOrigin, OriginUnavailable> {
    let (capture, identity) = verified_leaves(declaration, retained)?;
    let authored = declaration
        .selected_content_hash
        .as_deref()
        .ok_or(OriginUnavailable::Unbound)?;
    if vibe_core::ContentHash::parse(authored).ok().as_ref() != Some(capture.content_hash()) {
        return Err(OriginUnavailable::ContentMismatch);
    }
    Ok(AcceptedOrigin {
        declaration: declaration.clone(),
        capture,
        identity,
    })
}
fn verified_leaves(
    declaration: &ChangeOrigin,
    retained: Option<Arc<RetainedOriginCapture>>,
) -> Result<(Arc<RetainedOriginCapture>, OriginCaptureIdentity), OriginUnavailable> {
    let capture = retained.ok_or(OriginUnavailable::MissingRetained)?;
    let addresses: std::collections::BTreeSet<_> =
        declaration.documents.iter().map(|d| &d.address).collect();
    if addresses.is_empty() || addresses.len() != declaration.documents.len() {
        return Err(OriginUnavailable::Selection);
    }
    if PackageRef::parse(&declaration.package).ok().as_ref() != Some(capture.pin()) {
        return Err(OriginUnavailable::Pin);
    }
    let identity = identity(
        &capture,
        declaration.documents.iter().map(|d| d.address.clone()),
    )?;
    if declaration.manifest_sha256 != identity.manifest_sha256 {
        return Err(OriginUnavailable::Manifest);
    }
    for doc in &declaration.documents {
        if identity.facts[&doc.address].document_sha256 != doc.sha256 {
            return Err(OriginUnavailable::Document(doc.address.clone()));
        }
    }
    Ok((capture, identity))
}
#[derive(Debug)]
pub struct InitialOriginBinding {
    previous: ChangeOrigin,
    next: AcceptedOrigin,
}
impl InitialOriginBinding {
    pub fn previous(&self) -> &ChangeOrigin {
        &self.previous
    }
    pub fn next_origin(&self) -> &AcceptedOrigin {
        &self.next
    }
}
/// Explicit legacy-to-bound proposal. Verifies the leaf join first, rather
/// than circularly requiring an already bound whole-package AcceptedOrigin.
pub fn prepare_initial_origin_binding(
    declaration: &ChangeOrigin,
    retained: Option<Arc<RetainedOriginCapture>>,
) -> Result<InitialOriginBinding, OriginUnavailable> {
    if declaration.selected_content_hash.is_some() {
        return Err(OriginUnavailable::AlreadyBound);
    }
    let (capture, identity) = verified_leaves(declaration, retained)?;
    let mut next = declaration.clone();
    next.selected_content_hash = Some(identity.selected_content_hash);
    let next = accept_origin(&next, Some(capture))?;
    Ok(InitialOriginBinding {
        previous: declaration.clone(),
        next,
    })
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OriginChangeKind {
    Added,
    Removed,
    Changed,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginFactChange {
    pub address: String,
    pub kind: OriginChangeKind,
}
fn delta(old: &OriginCaptureIdentity, new: &OriginCaptureIdentity) -> Vec<OriginFactChange> {
    let addresses: std::collections::BTreeSet<_> =
        old.facts.keys().chain(new.facts.keys()).collect();
    addresses
        .into_iter()
        .filter_map(|address| {
            let kind = match (old.facts.get(address), new.facts.get(address)) {
                (None, Some(_)) => OriginChangeKind::Added,
                (Some(_), None) => OriginChangeKind::Removed,
                (Some(a), Some(b)) if a.canonical != b.canonical => OriginChangeKind::Changed,
                _ => return None,
            };
            Some(OriginFactChange {
                address: address.clone(),
                kind,
            })
        })
        .collect()
}
#[derive(Debug)]
pub enum OriginComparison {
    Unchanged {
        accepted: OriginCaptureIdentity,
        current: OriginCaptureIdentity,
    },
    OriginDrift {
        accepted: OriginCaptureIdentity,
        current: OriginCaptureIdentity,
        affected: Vec<OriginFactChange>,
    },
    Unavailable(OriginUnavailable),
}
pub fn compare_origin(
    accepted: &AcceptedOrigin,
    current: Option<&RetainedOriginCapture>,
) -> OriginComparison {
    let Some(current) = current else {
        return OriginComparison::Unavailable(OriginUnavailable::MissingCurrent);
    };
    let new = match identity(current, accepted.identity.facts.keys().cloned()) {
        Ok(value) => value,
        Err(e) => return OriginComparison::Unavailable(e),
    };
    if new == accepted.identity {
        OriginComparison::Unchanged {
            accepted: accepted.identity.clone(),
            current: new,
        }
    } else {
        let affected = delta(&accepted.identity, &new);
        OriginComparison::OriginDrift {
            accepted: accepted.identity.clone(),
            current: new,
            affected,
        }
    }
}
#[derive(Debug)]
pub struct OriginRebaseReview {
    pub old: OriginCaptureIdentity,
    pub new: OriginCaptureIdentity,
    pub affected: Vec<OriginFactChange>,
    next: AcceptedOrigin,
}
impl OriginRebaseReview {
    pub fn next_origin(&self) -> &AcceptedOrigin {
        &self.next
    }
}
/// The new capture's finite explicitly requested address inventory is the new
/// selection. This is not a complete arbitrary package fact-universe claim.
pub fn review_origin_rebase(
    old: &AcceptedOrigin,
    new: Arc<RetainedOriginCapture>,
) -> Result<OriginRebaseReview, OriginUnavailable> {
    let a = old.capture.pin();
    let b = new.pin();
    if a.group != b.group || a.name != b.name || a.kind != b.kind {
        return Err(OriginUnavailable::Coordinate);
    }
    let next_identity = identity(&new, new.facts().keys().cloned())?;
    let declaration = ChangeOrigin {
        package: next_identity.package.clone(),
        selected_content_hash: Some(next_identity.selected_content_hash.clone()),
        manifest_sha256: next_identity.manifest_sha256.clone(),
        documents: next_identity
            .facts
            .iter()
            .map(|(address, fact)| ChangeOriginDocument {
                address: address.clone(),
                sha256: fact.document_sha256.clone(),
            })
            .collect(),
    };
    let next = accept_origin(&declaration, Some(new))?;
    Ok(OriginRebaseReview {
        old: old.identity.clone(),
        affected: delta(&old.identity, &next_identity),
        new: next_identity,
        next,
    })
}
