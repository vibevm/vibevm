//! Internal complete metadata observation over one opaque retained M01 inventory.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-COMPLETE-FACT-SCOPE");
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use vibe_core::manifest::{Manifest, SourceSelector};
use vibe_facts::{
    AuthoredFact, CapturedDeclaredDocument, DeclaredDocumentObservation as Observed,
    SourceFileWitness,
};
use vibe_workspace::{source_query::RetainedSourceInventory, vibedeps::SlotFileDisposition};
mod selection;
pub use selection::{
    FactPage, FactSelection, PageAssembly, PageError, SelectedFactState, SelectionGap,
    SelectionGapKind,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentState {
    Native,
    Inert,
    Unclassified,
    Unavailable,
    Invalid,
    Unsupported,
    Ambiguous,
}
impl DocumentState {
    fn label(&self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Inert => "inert",
            Self::Unclassified => "unclassified",
            Self::Unavailable => "unavailable",
            Self::Invalid => "invalid",
            Self::Unsupported => "unsupported",
            Self::Ambiguous => "ambiguous",
        }
    }
}
#[derive(Debug, Clone)]
pub struct DocumentObservation {
    pub path: String,
    pub address_prefix: String,
    pub state: DocumentState,
    pub witness: Option<SourceFileWitness>,
    pub reader: Option<&'static str>,
    pub reason: Option<String>,
    pub original_unavailable: bool,
}
#[derive(Debug, Clone)]
pub struct CapturedFact {
    pub capture_id: String,
    pub authored: AuthoredFact,
    pub document: SourceFileWitness,
    pub reader: &'static str,
    pub span_basis: &'static str,
    pub numeric_position_unavailable: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureGap {
    NoDescriptors,
    DirectoryCensus(String),
}
#[derive(Debug, thiserror::Error)]
pub enum CompleteObservationError {
    #[error("retained manifest is unavailable")]
    Manifest,
    #[error("retained authored observation refused: {0:?}")]
    Observation(vibe_facts::DeclaredWalkError),
    #[error("retained fact source has conflicting document identity")]
    DocumentIdentity,
}
#[derive(Debug)]
pub struct CapturedFactSet {
    inventory: Arc<RetainedSourceInventory>,
    id: String,
    facts: BTreeMap<String, CapturedFact>,
    documents: BTreeMap<String, DocumentObservation>,
    gaps: Vec<CaptureGap>,
}
impl CapturedFactSet {
    /// Uses held bytes only. The manifest's declarations determine expected
    /// membership; output count and any provider completeness flag are absent.
    pub fn observe(
        inventory: Arc<RetainedSourceInventory>,
    ) -> Result<Self, CompleteObservationError> {
        let text = std::str::from_utf8(&inventory.manifest().bytes)
            .map_err(|_| CompleteObservationError::Manifest)?;
        let manifest = Manifest::parse_str(text).map_err(|_| CompleteObservationError::Manifest)?;
        let package = manifest.package.ok_or(CompleteObservationError::Manifest)?;
        if package.as_package_ref().ok().as_ref() != Some(inventory.pin()) {
            return Err(CompleteObservationError::Manifest);
        }
        let coordinate = format!("{}/{}", package.group, package.name);
        let mut gaps = vec![];
        if package.sources.is_empty() {
            gaps.push(CaptureGap::NoDescriptors)
        }
        for row in &package.sources {
            if let SourceSelector::Directory(path) = row.selector() {
                gaps.push(CaptureGap::DirectoryCensus(path.clone()))
            }
        }
        let captured: Vec<_> = inventory
            .documents()
            .iter()
            .map(|(path, doc)| CapturedDeclaredDocument {
                path,
                bytes: if matches!(doc.disposition(), Some(SlotFileDisposition::Converted)) {
                    None
                } else {
                    Some(doc.snapshot().bytes.as_slice())
                },
            })
            .collect();
        let observed = vibe_facts::observe_declared_source(&package, &captured)
            .map_err(CompleteObservationError::Observation)?;
        let mut documents = BTreeMap::new();
        let mut facts = BTreeMap::new();
        for (path, observation) in observed.documents {
            let (provenance, state, reason, rows) = match observation {
                Observed::Facts { provenance, facts } => {
                    (provenance, DocumentState::Native, None, facts)
                }
                Observed::Unavailable { provenance } => (
                    provenance,
                    DocumentState::Unavailable,
                    Some("captured bytes unavailable".into()),
                    vec![],
                ),
                Observed::Unclassified { provenance, .. } => {
                    (provenance, DocumentState::Unclassified, None, vec![])
                }
                Observed::NeedsInput { provenance, .. } => (
                    provenance,
                    DocumentState::Ambiguous,
                    Some("descriptor selection needs input".into()),
                    vec![],
                ),
                Observed::Invalid {
                    provenance, reason, ..
                } => (provenance, DocumentState::Invalid, Some(reason), vec![]),
                Observed::Unsupported {
                    provenance, reason, ..
                } => (
                    provenance,
                    DocumentState::Unsupported,
                    Some(reason.into()),
                    vec![],
                ),
                Observed::Raw { provenance, .. } | Observed::Structure { provenance, .. } => {
                    (provenance, DocumentState::Inert, None, vec![])
                }
            };
            let prefix = vibe_facts::address_prefix(&coordinate, &path)
                .map_err(|_| CompleteObservationError::DocumentIdentity)?;
            let original_unavailable = inventory.documents().get(&path).is_some_and(|doc| {
                matches!(doc.disposition(), Some(SlotFileDisposition::Converted))
            });
            for authored in rows {
                let document = provenance
                    .witness
                    .clone()
                    .ok_or(CompleteObservationError::DocumentIdentity)?;
                let reader = provenance
                    .reader
                    .ok_or(CompleteObservationError::DocumentIdentity)?;
                facts.insert(
                    authored.address.clone(),
                    CapturedFact {
                        capture_id: String::new(),
                        authored,
                        document,
                        reader,
                        span_basis: "captured-original-bytes/1",
                        numeric_position_unavailable: true,
                    },
                );
            }
            documents.insert(
                path.clone(),
                DocumentObservation {
                    path,
                    address_prefix: prefix,
                    state,
                    witness: provenance.witness,
                    reader: provenance.reader,
                    reason,
                    original_unavailable,
                },
            );
        }
        let mut framed = crate::digest::seed(b"vibe-requirements-complete-capture\0epoch=1\0");
        crate::digest::field(&mut framed, b"pin", inventory.pin().to_string().as_bytes());
        crate::digest::field(
            &mut framed,
            b"selected-content",
            inventory.content_hash().as_str().as_bytes(),
        );
        crate::digest::field(
            &mut framed,
            b"manifest",
            inventory.manifest().sha256.as_bytes(),
        );
        crate::digest::count(&mut framed, b"documents", documents.len());
        for (path, doc) in &documents {
            crate::digest::field(&mut framed, b"path", path.as_bytes());
            crate::digest::field(&mut framed, b"state", doc.state.label().as_bytes());
            crate::digest::presence(&mut framed, b"witness", doc.witness.is_some());
            if let Some(w) = &doc.witness {
                crate::digest::field(&mut framed, b"bytes", w.bytes.to_string().as_bytes());
                crate::digest::field(&mut framed, b"digest", w.digest.as_bytes());
            }
        }
        crate::digest::count(&mut framed, b"held-outputs", inventory.documents().len());
        for (path, doc) in inventory.documents() {
            crate::digest::field(&mut framed, b"source-path", path.as_bytes());
            crate::digest::field(
                &mut framed,
                b"output-path",
                doc.provenance().output_path.as_bytes(),
            );
            crate::digest::field(
                &mut framed,
                b"output-digest",
                doc.snapshot().sha256.as_bytes(),
            );
        }
        let id = crate::digest::finish(framed);
        for fact in facts.values_mut() {
            fact.capture_id = id.clone();
        }
        Ok(Self {
            inventory,
            id,
            facts,
            documents,
            gaps,
        })
    }
    pub fn capture_id(&self) -> &str {
        &self.id
    }
    pub fn inventory(&self) -> &RetainedSourceInventory {
        &self.inventory
    }
    pub fn facts(&self) -> &BTreeMap<String, CapturedFact> {
        &self.facts
    }
    pub fn documents(&self) -> &BTreeMap<String, DocumentObservation> {
        &self.documents
    }
    pub fn capture_gaps(&self) -> &[CaptureGap] {
        &self.gaps
    }
    pub fn select_all(&self) -> FactSelection<'_> {
        selection::all(self)
    }
    pub fn select_exact(&self, addresses: &[String]) -> FactSelection<'_> {
        selection::exact(self, addresses)
    }
}
#[cfg(test)]
mod m_02_c_2_cases;
