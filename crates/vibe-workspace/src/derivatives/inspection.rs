//! Complete raw inventories and optional, explicitly labelled Vibe projections.
use super::{BoundDerivative, LocalDerivativeError, MaterializedDerivative, Outcome, passive};
use std::collections::BTreeSet;
use vibe_core::manifest::{SourceInterpretation, derivation::RetainedSource};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileInspection {
    pub path: String,
    /// Exact raw source bytes, including opaque binary data and notices.
    pub raw: Vec<u8>,
    pub raw_sha256: String,
    /// None for data. Err retains diagnostics without losing the raw source.
    /// The tuple is canonical Markdown text and its own SHA-256 basis.
    pub vibe_projection: Option<Result<(String, String), String>>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivativeInspection {
    pub base_identity: String,
    pub local_identity: String,
    pub provenance_identity: String,
    pub result_identity: String,
    pub inventory_identity: String,
    pub base: Vec<FileInspection>,
    pub local: Vec<FileInspection>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryDifference {
    pub path: String,
    /// None means addition; the retained raw/projection bases are reviewable.
    pub before: Option<FileInspection>,
    /// None means removal. Unchanged paths appear in show, not diff.
    pub after: Option<FileInspection>,
}
pub(crate) fn inventory(source: &RetainedSource) -> Vec<FileInspection> {
    let mut rows: Vec<_> = source
        .files
        .iter()
        .map(|file| FileInspection {
            path: file.path.clone(),
            raw: file.bytes.clone(),
            raw_sha256: format!("sha256:{}", crate::vibedeps::sha256_bytes(&file.bytes)),
            vibe_projection: (source.descriptor.interpretation() == SourceInterpretation::VibeSpec)
                .then(|| {
                    vibe_spec::inspect_vibe_source(std::path::Path::new(&file.path), &file.bytes)
                        .projection
                }),
        })
        .collect();
    rows.sort_by(|a, b| a.path.cmp(&b.path));
    rows
}
impl BoundDerivative {
    pub fn show(&self) -> Outcome<DerivativeInspection> {
        passive(DerivativeInspection {
            base_identity: self.record().base_identity.clone(),
            local_identity: self.record().local_identity.clone(),
            provenance_identity: self.record().provenance_identity.clone(),
            result_identity: self.record().result_identity.clone(),
            inventory_identity: self.record().inventory_identity.clone(),
            base: inventory(self.binding().base()),
            local: inventory(self.binding().local()),
        })
    }
    pub fn diff(&self) -> Outcome<Vec<InventoryDifference>> {
        let shown = self.show().value;
        let paths: BTreeSet<_> = shown
            .base
            .iter()
            .chain(&shown.local)
            .map(|f| f.path.clone())
            .collect();
        passive(
            paths
                .into_iter()
                .filter_map(|path| {
                    let before = shown.base.iter().find(|f| f.path == path).cloned();
                    let after = shown.local.iter().find(|f| f.path == path).cloned();
                    (before != after).then_some(InventoryDifference {
                        path,
                        before,
                        after,
                    })
                })
                .collect(),
        )
    }
}
impl MaterializedDerivative {
    /// Inspection never silently describes a stale or incomplete output tree.
    pub fn show(&self) -> Result<Outcome<DerivativeInspection>, LocalDerivativeError> {
        self.verify_tree()?;
        Ok(self.result().show())
    }
    pub fn diff(&self) -> Result<Outcome<Vec<InventoryDifference>>, LocalDerivativeError> {
        self.verify_tree()?;
        Ok(self.result().diff())
    }
}
