//! Per-invocation provenance evidence. Lane contributions remain the mutable
//! working surface: carriers may move, disappear or copy known provenance.
use super::TransitionError;
use crate::SpecAddress;
use crate::compiler::ir::{
    ClosureContribution, ClosureIr, ClosureNodeId, ContributionMeta, DocumentAddress, LaneChunk,
    LaneContribution, LaneIr, LaneNode,
};
use crate::embed::occurrences::EmbedTape;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Owner {
    ClosureDocument(DocumentAddress),
    ClosureSimple(ContributionMeta, DocumentAddress),
    LaneNormal(ContributionMeta, ClosureNodeId, SpecAddress),
    LaneSimple(ContributionMeta, DocumentAddress),
}
#[derive(Debug, Clone, Copy)]
enum Surface {
    Closure,
    Lane,
}
#[derive(Debug, Clone)]
pub(super) struct Witness {
    rows: Vec<(Owner, Option<EmbedTape>)>,
    surface: Surface,
}

impl Witness {
    pub(super) fn closure(value: &ClosureIr) -> Self {
        let mut rows = value
            .nodes
            .iter()
            .map(|node| {
                (
                    Owner::ClosureDocument(node.address.clone()),
                    node.embeds.clone(),
                )
            })
            .collect::<Vec<_>>();
        for contribution in &value.contributions {
            if let ClosureContribution::Simple { meta, document } = contribution {
                rows.push((
                    Owner::ClosureSimple(meta.clone(), document.address.clone()),
                    document.embeds.clone(),
                ));
            }
        }
        Self {
            rows,
            surface: Surface::Closure,
        }
    }
    pub(super) fn lane(value: &LaneIr) -> Self {
        let mut rows = Vec::new();
        for contribution in &value.contributions {
            let (meta, chunks) = match contribution {
                LaneContribution::Normal { meta, chunks, .. }
                | LaneContribution::Simple { meta, chunks, .. } => (meta, chunks),
                _ => continue,
            };
            for chunk in chunks {
                if let LaneChunk::Node(node) = chunk {
                    let (owner, tape) = match node.as_ref() {
                        LaneNode::Normal {
                            node,
                            requested_address,
                            embeds,
                            ..
                        } => (
                            Owner::LaneNormal(meta.clone(), *node, requested_address.clone()),
                            embeds,
                        ),
                        LaneNode::Simple {
                            address, embeds, ..
                        } => (Owner::LaneSimple(meta.clone(), address.clone()), embeds),
                    };
                    rows.push((owner, Some(tape.clone())));
                }
            }
        }
        Self {
            rows,
            surface: Surface::Lane,
        }
    }
    pub(super) fn verify(&self, after: &Self, may_mint: bool) -> Result<(), TransitionError> {
        if matches!(self.surface, Surface::Closure) {
            // Closure expansion preserves its known annotated source rows.
            // The lane working surface below has no such cardinality rule.
            for (owner, tape) in &self.rows {
                if tape
                    .as_ref()
                    .is_some_and(|tape| !tape.occurrences.is_empty())
                    && !after.rows.iter().any(|(next_owner, next)| {
                        next_owner == owner && compatible(tape.as_ref(), next.as_ref(), may_mint)
                    })
                {
                    return Err(refused(owner));
                }
            }
        }
        for (owner, tape) in &after.rows {
            let mut known = self
                .rows
                .iter()
                .filter(|(old_owner, _)| old_owner == owner)
                .peekable();
            if known.peek().is_none() {
                if !may_mint
                    && tape
                        .as_ref()
                        .is_some_and(|tape| !tape.occurrences.is_empty())
                {
                    return Err(refused(owner));
                }
                continue;
            }
            // Each returned carrier needs known origins, not a unique consumed
            // predecessor. This admits copying and removal of whole lane
            // carriers while refusing stripping or forging a retained one.
            if !known.any(|(_, old)| compatible(old.as_ref(), tape.as_ref(), may_mint)) {
                return Err(refused(owner));
            }
        }
        Ok(())
    }
}
fn refused(owner: &Owner) -> TransitionError {
    TransitionError::EmbedProvenance {
        owner: format!("{owner:?}"),
    }
}
fn compatible(before: Option<&EmbedTape>, after: Option<&EmbedTape>, may_mint: bool) -> bool {
    match (before, after) {
        (None, _) if may_mint => true,
        (None, None) => true,
        (Some(before), Some(after)) => {
            before.occurrences.len() == after.occurrences.len()
                && before
                    .occurrences
                    .iter()
                    .zip(&after.occurrences)
                    .all(|(old, new)| {
                        old.path == new.path
                            && old.target == new.target
                            && old.source == new.source
                            && old.directive_line == new.directive_line
                            && old.intrinsic_empty == new.intrinsic_empty
                            && old.owner.kind == new.owner.kind
                            && (!old.suppressed || new.suppressed)
                            && (before.body_digest != after.body_digest || old == new)
                    })
        }
        (None, Some(after)) => after.occurrences.is_empty(),
        (Some(before), None) => before.occurrences.is_empty(),
    }
}
