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

mod artifact;
pub use artifact::*;
mod target;
pub use target::*;
mod emitted;
pub use emitted::*;
mod lane;
pub(crate) use lane::*;
mod link;
pub(crate) use link::*;

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

/// Runtime typestate of the closure-level qualify transform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QualificationState {
    Pending(StaticCompileMode),
    Applied(StaticCompileMode),
}

/// Stable index of a node inside one [`ClosureIr`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ClosureNodeId(pub(crate) usize);

/// One exact request for a shared logical closure node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClosureOccurrence {
    pub(crate) node: ClosureNodeId,
    pub(crate) requested_address: SpecAddress,
}

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
    pub(crate) requested_target: SpecAddress,
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
        seed_address: SpecAddress,
        emission_order: Vec<ClosureOccurrence>,
    },
    Simple {
        meta: ContributionMeta,
        document: Box<ClosureDocument>,
    },
    Elided {
        meta: ContributionMeta,
    },
    Hoisted {
        meta: ContributionMeta,
        target: SpecAddress,
    },
}

/// READ-ONCE disposition aligned to one contribution occurrence-for-occurrence.
///
/// A node id may repeat in one order or be shared by several roots, so a bool
/// mask or set of absorbed node ids would lose the identity it judged. The
/// exact spec address keeps a stable normal-document witness if the indexed
/// node vector is reordered or replaced; body text and origin intentionally
/// remain mutable. `StaticEntry` identity remains exclusive to simple
/// contributions and is unrepresentable here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AbsorptionOccurrence {
    pub(crate) node: ClosureNodeId,
    pub(crate) requested_address: SpecAddress,
    pub(crate) absorbed: bool,
}

/// The analyzed occurrence sequence bound to its exact contribution identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ContributionAbsorption {
    Normal {
        meta: ContributionMeta,
        seed: ClosureNodeId,
        seed_address: SpecAddress,
        occurrences: Vec<AbsorptionOccurrence>,
    },
    Simple {
        meta: ContributionMeta,
        address: DocumentAddress,
    },
    Elided {
        meta: ContributionMeta,
    },
    Hoisted {
        meta: ContributionMeta,
        target: SpecAddress,
    },
}

/// The immutable pre-qualification overlap judgment for one artifact,
/// including the exact qualification mode under which it was produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AbsorptionPlan {
    pub(crate) mode: StaticCompileMode,
    pub(crate) contributions: Vec<ContributionAbsorption>,
}

/// Runtime typestate of READ-ONCE absorption over contribution occurrences.
///
/// The same identity-bound plan crosses both boundaries: `Planned` proves
/// qualify judged the pre-rewrite occurrence view; `Applied` proves absorb
/// projected every normal emission order without hiding the judgment in a
/// process-local side table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AbsorptionState {
    Unplanned,
    Planned(AbsorptionPlan),
    Applied(AbsorptionPlan),
}

/// The ordered multi-seed graph for one final artifact.
///
/// A graph node may appear in more than one normal root's `emission_order`.
/// That represents shared closure identity without silently changing today's
/// per-root emission multiplicity or byte order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClosureIr {
    context: ArtifactContext,
    pub(crate) nodes: Vec<ClosureDocument>,
    pub(crate) edges: Vec<ClosureEdge>,
    pub(crate) contributions: Vec<ClosureContribution>,
    pub(crate) renames: Vec<OriginRename>,
    pub(crate) qualification: QualificationState,
    pub(crate) absorption: AbsorptionState,
    pub(crate) link: LinkState,
    pub(crate) pending_sources: Option<SourceResolutionSnapshot>,
    pub(crate) pending_embeds: Option<EmbedResolutionSnapshot>,
}

impl ClosureIr {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_plan(
        plan: &ArtifactPlan,
        nodes: Vec<ClosureDocument>,
        edges: Vec<ClosureEdge>,
        contributions: Vec<ClosureContribution>,
        renames: Vec<OriginRename>,
        qualification: QualificationState,
        absorption: AbsorptionState,
        link: LinkState,
        pending_sources: Option<SourceResolutionSnapshot>,
        pending_embeds: Option<EmbedResolutionSnapshot>,
    ) -> Self {
        Self {
            context: plan.context().clone(),
            nodes,
            edges,
            contributions,
            renames,
            qualification,
            absorption,
            link,
            pending_sources,
            pending_embeds,
        }
    }

    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn testing(
        context: ArtifactContext,
        nodes: Vec<ClosureDocument>,
        edges: Vec<ClosureEdge>,
        contributions: Vec<ClosureContribution>,
        renames: Vec<OriginRename>,
        qualification: QualificationState,
        absorption: AbsorptionState,
        link: LinkState,
        pending_sources: Option<SourceResolutionSnapshot>,
        pending_embeds: Option<EmbedResolutionSnapshot>,
    ) -> Self {
        Self::from_parts(
            context,
            nodes,
            edges,
            contributions,
            renames,
            qualification,
            absorption,
            link,
            pending_sources,
            pending_embeds,
        )
    }

    /// The checked whole-value constructor the wire conversion rebuilds a
    /// closure through. The parts must already have passed the conversion
    /// gates; the context is the one domain law this constructor owns.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_parts(
        context: ArtifactContext,
        nodes: Vec<ClosureDocument>,
        edges: Vec<ClosureEdge>,
        contributions: Vec<ClosureContribution>,
        renames: Vec<OriginRename>,
        qualification: QualificationState,
        absorption: AbsorptionState,
        link: LinkState,
        pending_sources: Option<SourceResolutionSnapshot>,
        pending_embeds: Option<EmbedResolutionSnapshot>,
    ) -> Self {
        Self {
            context,
            nodes,
            edges,
            contributions,
            renames,
            qualification,
            absorption,
            link,
            pending_sources,
            pending_embeds,
        }
    }

    pub(crate) fn context(&self) -> &ArtifactContext {
        &self.context
    }
}

#[cfg(test)]
mod tests;
