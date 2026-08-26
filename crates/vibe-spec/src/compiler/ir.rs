//! The compiler's five domain IR levels and their explicit cardinalities.
//!
//! These are Rust domain values, not the R6 native wire. They intentionally
//! carry no serde derives, schema number, JSON level tag, or base64 spelling;
//! R6 will project them into generated JTD types after the pass refactor has
//! proved which fields are real.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR");

use std::collections::BTreeMap;

use crate::{DocTree, RenameEntry, SpecAddress};

use super::embed_snapshot::EmbedResolutionSnapshot;
use super::source_snapshot::SourceResolutionSnapshot;

/// One of the five progressively lowered compiler representations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IrLevel {
    Source,
    Document,
    Closure,
    Lane,
    Emitted,
}

/// Whether one pass invocation owns one addressed document or one artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IrCardinality {
    Document,
    Artifact,
}

/// The exact level/cardinality pair accepted or returned by a pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IrShape {
    pub(crate) level: IrLevel,
    pub(crate) cardinality: IrCardinality,
}

impl IrShape {
    pub(crate) const fn new(level: IrLevel, cardinality: IrCardinality) -> Self {
        Self { level, cardinality }
    }
}

/// A document's compiler identity.
///
/// Normal documents use their real `spec://` address. A simple static entry
/// can belong to an ungrouped host and therefore have no honest spec address;
/// it keeps its existing provider/path identity rather than minting a fake
/// package coordinate or an unresolvable public URI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DocumentAddress {
    Spec(SpecAddress),
    StaticEntry { origin: String, path: String },
}

/// Open, typed identity of the frontend syntax carried by [`SourceIr`].
///
/// It is deliberately not a closed enum: R6 frontends may add formats without
/// making the R3 domain type itself a registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceFormatId(String);

impl SourceFormatId {
    pub(crate) fn canonical_markdown() -> Self {
        Self("markdown".to_string())
    }

    pub(crate) fn new(value: impl Into<String>) -> Result<Self, IrIdError> {
        let value = value.into();
        if value.trim().is_empty() {
            Err(IrIdError {
                kind: "source format",
            })
        } else {
            Ok(Self(value))
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// Open identity of one final compiler artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArtifactId(String);

impl ArtifactId {
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, IrIdError> {
        let value = value.into();
        if value.trim().is_empty() {
            Err(IrIdError { kind: "artifact" })
        } else {
            Ok(Self(value))
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// An internal IR identity must contain at least one non-whitespace character.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("compiler {kind} id must not be blank")]
pub(crate) struct IrIdError {
    kind: &'static str,
}

/// Source IR for exactly one addressed document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceIr {
    address: DocumentAddress,
    format: SourceFormatId,
    text: String,
}

impl SourceIr {
    pub(crate) fn new(
        address: DocumentAddress,
        format: SourceFormatId,
        text: impl Into<String>,
    ) -> Self {
        Self {
            address,
            format,
            text: text.into(),
        }
    }

    pub(crate) fn address(&self) -> &DocumentAddress {
        &self.address
    }

    pub(crate) fn format(&self) -> &SourceFormatId {
        &self.format
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    /// The only field a source transform may rewrite in place. Address and
    /// frontend identity remain stable across a source-level transform.
    pub(crate) fn text_mut(&mut self) -> &mut String {
        &mut self.text
    }

    pub(crate) fn into_parts(self) -> (DocumentAddress, SourceFormatId, String) {
        (self.address, self.format, self.text)
    }
}

/// Parsed document IR for exactly one addressed document.
///
/// The source stays beside the tree so exact text/newline identity remains
/// available while the current [`DocTree`] is still a source-span tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DocumentIr {
    source: SourceIr,
    tree: DocTree,
}

impl DocumentIr {
    pub(crate) fn new(source: SourceIr, tree: DocTree) -> Self {
        Self { source, tree }
    }

    pub(crate) fn source(&self) -> &SourceIr {
        &self.source
    }

    pub(crate) fn tree(&self) -> &DocTree {
        &self.tree
    }

    /// The document-level mutation seam. The paired source identity is not
    /// exposed mutably, so a tree transform cannot silently retarget a doc.
    pub(crate) fn tree_mut(&mut self) -> &mut DocTree {
        &mut self.tree
    }

    pub(crate) fn into_parts(self) -> (SourceIr, DocTree) {
        (self.source, self.tree)
    }
}

/// The parse worklist after every per-document pass has run.
///
/// This is a document-level value with artifact cardinality, not a sixth IR
/// level. Vector order is the deterministic worklist order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Documents(Vec<DocumentIr>);

impl Documents {
    pub(crate) fn new(documents: Vec<DocumentIr>) -> Self {
        Self(documents)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &DocumentIr> {
        self.0.iter()
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = &mut DocumentIr> {
        self.0.iter_mut()
    }

    pub(crate) fn into_vec(self) -> Vec<DocumentIr> {
        self.0
    }

    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl IntoIterator for Documents {
    type Item = DocumentIr;
    type IntoIter = std::vec::IntoIter<DocumentIr>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Documents {
    type Item = &'a DocumentIr;
    type IntoIter = std::slice::Iter<'a, DocumentIr>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut Documents {
    type Item = &'a mut DocumentIr;
    type IntoIter = std::slice::IterMut<'a, DocumentIr>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

/// Provenance of one top-level static contribution in effective-boot order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContributionMeta {
    pub(crate) origin: String,
    pub(crate) path: String,
}

/// The compatibility/static-lane policy carried through the whole artifact.
///
/// Both modes traverse the same named pass list. `Plain` keeps labels as
/// authored, but qualification still lowers aliases and plans READ-ONCE
/// absorption; `QualifyPerNode` additionally qualifies node-local labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StaticCompileMode {
    Plain,
    QualifyPerNode,
}

/// Runtime typestate of the closure-level qualify transform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QualificationState {
    Pending(StaticCompileMode),
    Applied(StaticCompileMode),
}

/// Immutable invocation input for compiling one artifact.
///
/// This is configuration around the five levels, not another IR level.
#[derive(Debug, Clone)]
pub(crate) struct ArtifactPlan {
    pub(crate) artifact: ArtifactId,
    pub(crate) mode: StaticCompileMode,
    pub(crate) contributions: Vec<ArtifactInput>,
}

/// One heterogeneous artifact input in effective-boot order.
#[derive(Debug, Clone)]
pub(crate) enum ArtifactInput {
    Normal {
        meta: ContributionMeta,
        seed: SpecAddress,
    },
    Simple {
        meta: ContributionMeta,
        source: SourceIr,
    },
}

/// Stable index of a node inside one [`ClosureIr`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ClosureNodeId(pub(crate) usize);

/// One closed document payload.
///
/// A value is a graph node only while it lives in [`ClosureIr::nodes`] and is
/// addressed by a [`ClosureNodeId`]. Simple contributions carry the same
/// lowered payload outside that graph, so document-pass edits survive closing
/// without inventing dependency edges or a fake `spec://` identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClosureDocument {
    pub(crate) address: DocumentAddress,
    pub(crate) origin: String,
    pub(crate) tree: DocTree,
    pub(crate) aliases: BTreeMap<String, SpecAddress>,
}

/// The three authored dependency relations represented by the closure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClosureEdgeKind {
    Use,
    Source,
    Embed,
}

/// One ordered edge of the closure graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClosureEdge {
    pub(crate) from: ClosureNodeId,
    pub(crate) to: ClosureNodeId,
    pub(crate) kind: ClosureEdgeKind,
}

/// One rename together with the origin whose namespace produced it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginRename {
    pub(crate) origin: String,
    pub(crate) rename: RenameEntry,
}

/// One top-level contribution after the document batch closes into a graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ClosureContribution {
    Normal {
        meta: ContributionMeta,
        seed: ClosureNodeId,
        emission_order: Vec<ClosureNodeId>,
    },
    Simple {
        meta: ContributionMeta,
        document: Box<ClosureDocument>,
    },
}

/// READ-ONCE disposition aligned to one contribution occurrence-for-occurrence.
///
/// A node id may repeat in one order or be shared by several roots, so a bool
/// mask or set of absorbed node ids would lose the identity it judged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AbsorptionOccurrence {
    pub(crate) node: ClosureNodeId,
    pub(crate) absorbed: bool,
}

/// The analyzed occurrence sequence bound to its exact contribution identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ContributionAbsorption {
    Normal {
        meta: ContributionMeta,
        seed: ClosureNodeId,
        occurrences: Vec<AbsorptionOccurrence>,
    },
    Simple {
        meta: ContributionMeta,
        address: DocumentAddress,
    },
}

/// The immutable pre-qualification overlap judgment for one artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AbsorptionPlan {
    pub(crate) contributions: Vec<ContributionAbsorption>,
}

/// The ordered multi-seed graph for one final artifact.
///
/// A graph node may appear in more than one normal root's `emission_order`.
/// That represents shared closure identity without silently changing today's
/// per-root emission multiplicity or byte order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClosureIr {
    pub(crate) artifact: ArtifactId,
    pub(crate) nodes: Vec<ClosureDocument>,
    pub(crate) edges: Vec<ClosureEdge>,
    pub(crate) contributions: Vec<ClosureContribution>,
    pub(crate) renames: Vec<OriginRename>,
    pub(crate) qualification: QualificationState,
    pub(crate) absorption: Option<AbsorptionPlan>,
    pub(crate) pending_sources: Option<SourceResolutionSnapshot>,
    pub(crate) pending_embeds: Option<EmbedResolutionSnapshot>,
}

/// The single shared frame surrounding one assembled lane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LaneFrame {
    pub(crate) header: String,
    pub(crate) preamble: String,
    pub(crate) renames: Vec<OriginRename>,
}

/// Reversible compiler-marker policy of one lane node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NodeMarkers {
    None,
    Reversible { key: String },
}

/// One structured node ready for lane serialisation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LaneNode {
    pub(crate) address: DocumentAddress,
    pub(crate) origin: String,
    pub(crate) body: String,
    pub(crate) markers: NodeMarkers,
}

/// One contribution in the final artifact's declaration order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LaneContribution {
    Normal {
        meta: ContributionMeta,
        nodes: Vec<LaneNode>,
    },
    Simple {
        meta: ContributionMeta,
        node: LaneNode,
    },
}

/// One fully framed artifact, still structured and not yet serialised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LaneIr {
    pub(crate) artifact: ArtifactId,
    pub(crate) frame: LaneFrame,
    pub(crate) contributions: Vec<LaneContribution>,
}

/// The exact bytes of one final artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EmittedIr {
    pub(crate) artifact: ArtifactId,
    pub(crate) bytes: Vec<u8>,
}

#[cfg(test)]
mod tests;
