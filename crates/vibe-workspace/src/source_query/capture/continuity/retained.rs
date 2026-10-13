//! Memory-held historical observations, minted only at the current M01 boundary.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-RETAINED-ORIGIN");
use super::*;
use std::sync::Arc;
use vibe_core::{ContentHash, PackageRef};
use vibe_spec::{Authority, SpecAddress};

#[derive(Debug, Clone)]
pub struct RetainedOriginDocument {
    snapshot: Arc<StableFileSnapshot>,
    provenance: ExtractionProvenance,
    disposition: Option<crate::vibedeps::SlotFileDisposition>,
}
impl RetainedOriginDocument {
    pub fn disposition(&self) -> Option<&crate::vibedeps::SlotFileDisposition> {
        self.disposition.as_ref()
    }
    pub fn snapshot(&self) -> &StableFileSnapshot {
        &self.snapshot
    }
    pub fn provenance(&self) -> &ExtractionProvenance {
        &self.provenance
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedOriginFact {
    pub document: String,
    /// None is an exact selected-address gap, never a complete fact universe.
    pub canonical: Option<CanonicalNodeIdentity>,
}
#[derive(Debug, Clone)]
pub struct RetainedOriginCapture {
    pin: PackageRef,
    content_hash: ContentHash,
    manifest: Arc<StableFileSnapshot>,
    documents: BTreeMap<String, RetainedOriginDocument>,
    facts: BTreeMap<String, RetainedOriginFact>,
}
impl RetainedOriginCapture {
    pub fn pin(&self) -> &PackageRef {
        &self.pin
    }
    pub fn content_hash(&self) -> &ContentHash {
        &self.content_hash
    }
    pub fn manifest(&self) -> &StableFileSnapshot {
        &self.manifest
    }
    pub fn documents(&self) -> &BTreeMap<String, RetainedOriginDocument> {
        &self.documents
    }
    pub fn facts(&self) -> &BTreeMap<String, RetainedOriginFact> {
        &self.facts
    }
}
impl WitnessedSourceWorld {
    /// Explicit finite retention, not a source store or new read/proof grant.
    /// After successful mint, historical getters never inspect current disk.
    pub fn retain_origin_capture(
        &self,
        group: &str,
        name: &str,
        addresses: &[String],
    ) -> Result<RetainedOriginCapture, SourceQueryUnavailable> {
        if addresses.is_empty() || addresses.len() > 4096 {
            return Err(SourceQueryUnavailable::WorldUnavailable(
                "retained address bound",
            ));
        }
        let prepared = self.revalidate_current()?;
        let (pin, content_hash) = self.selected_source_pin(group, name)?;
        let capture = prepared
            .captured
            .get(&CaptureOwner::Dependency {
                group: group.into(),
                name: name.into(),
            })
            .ok_or(SourceQueryUnavailable::WorldUnavailable(
                "retained selected owner",
            ))?;
        let mut documents = BTreeMap::new();
        let mut facts = BTreeMap::new();
        let mut remaining = 256 * 1024 * 1024usize;
        for address in addresses {
            let parsed = SpecAddress::parse(address)
                .map_err(|_| SourceQueryUnavailable::WorldUnavailable("retained fact address"))?;
            if !matches!(&parsed.authority, Authority::Package { group:g, name:n, version:None } if g==group && n==name)
                || parsed.pinned_r.is_some()
                || parsed.anchor.is_empty()
            {
                return Err(SourceQueryUnavailable::WorldUnavailable(
                    "retained fact coordinate/revision",
                ));
            }
            if facts.contains_key(address) {
                return Err(SourceQueryUnavailable::WorldUnavailable(
                    "duplicate retained address",
                ));
            }
            let (output, recorded) =
                self.selected_document_output(group, name, &parsed.doc_path)?;
            let mut canonical = canonical_node_of_output(&recorded, &parsed.anchor.join("."))?;
            if canonical.is_some() {
                let selected = self
                    .read_spec(
                        Some((group, name)),
                        &output,
                        Some(&parsed.anchor.join(".")),
                        false,
                    )
                    .map_err(|_| {
                        SourceQueryUnavailable::WorldUnavailable("retained fact selection")
                    })?;
                if selected.selection.kind != vibe_specdoc::selection::SelectedNodeKind::Fact {
                    canonical = None;
                }
            }
            if !documents.contains_key(&parsed.doc_path) {
                let extracted =
                    self.extract_source(Some((group, name)), &output)
                        .map_err(|_| {
                            SourceQueryUnavailable::WorldUnavailable("retained classified output")
                        })?;
                remaining = remaining.checked_sub(extracted.capture.bytes.len()).ok_or(
                    SourceQueryUnavailable::WorldUnavailable("retained byte bound"),
                )?;
                documents.insert(
                    parsed.doc_path.clone(),
                    RetainedOriginDocument {
                        snapshot: Arc::new(extracted.capture.clone()),
                        provenance: extracted.provenance,
                        disposition: recorded.disposition.clone(),
                    },
                );
            }
            facts.insert(
                address.clone(),
                RetainedOriginFact {
                    document: parsed.doc_path,
                    canonical,
                },
            );
        }
        self.revalidate_current()?;
        Ok(RetainedOriginCapture {
            pin,
            content_hash,
            manifest: Arc::new(capture.manifest.clone()),
            documents,
            facts,
        })
    }
}
