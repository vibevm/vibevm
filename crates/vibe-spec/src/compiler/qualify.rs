//! The named whole-artifact qualification pass.
//!
//! READ-ONCE applicability is planned from the immutable post-embed text before
//! any alias or label rewrite. The pass then transforms each live graph node at
//! most once, preserves contribution occurrences, and carries the plan for the
//! still-legacy absorb tail.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR");

use std::collections::{BTreeMap, HashSet};

use crate::qualify::qualify_contribution;
use crate::{DocTree, SpecAddress};

use super::ir::{
    ClosureContribution, ClosureDocument, ClosureIr, ClosureNodeId, ContributionAbsorption,
    DocumentAddress, OriginRename, QualificationState, StaticCompileMode,
};
use super::pass::{Pass, PassName};

mod absorption;

pub(crate) const QUALIFY_PASS_NAME: &str = "qualify";

pub(crate) struct QualifyPass {
    name: PassName,
}

impl QualifyPass {
    pub(crate) fn new() -> Self {
        Self {
            name: PassName::new(QUALIFY_PASS_NAME)
                .expect("the static built-in qualify pass name is non-blank"),
        }
    }
}

impl Pass for QualifyPass {
    type Input = ClosureIr;
    type Output = ClosureIr;
    type Error = QualifyPassError;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, input: ClosureIr) -> Result<ClosureIr, QualifyPassError> {
        #[cfg(test)]
        QUALIFY_INVOCATIONS.with(|count| count.set(count.get() + 1));
        qualify_closure(input)
    }
}

#[cfg(test)]
std::thread_local! {
    static QUALIFY_INVOCATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_qualify_invocations() {
    QUALIFY_INVOCATIONS.with(|count| count.set(0));
}

#[cfg(test)]
pub(crate) fn qualify_invocations() -> usize {
    QUALIFY_INVOCATIONS.with(std::cell::Cell::get)
}

/// Recheck occurrence alignment at a consumer boundary. R3.3's inter-pass
/// verifier can lift this exact private invariant without changing the IR.
pub(crate) fn validate_absorption(
    plan: &super::ir::AbsorptionPlan,
    closure: &ClosureIr,
) -> Result<(), QualifyPassError> {
    absorption::validate(plan, closure)
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum QualifyPassError {
    #[error("qualify requires pending qualification state")]
    AlreadyApplied,
    #[error("qualify requires named merge and embed to consume their pending state")]
    PendingEarlierPass,
    #[error("qualify requires an empty absorption plan and rename list")]
    DirtyOutputState,
    #[error(
        "contribution {contribution} occurrence {occurrence} names missing closure node {node}"
    )]
    InvalidNodeId {
        contribution: usize,
        occurrence: usize,
        node: usize,
    },
    #[error("normal contribution {contribution} occurrence {occurrence} is not a spec document")]
    NonSpecGraphNode {
        contribution: usize,
        occurrence: usize,
    },
    #[error(
        "absorption plan is misaligned{suffix}: expected {expected} entries, got {actual}",
        suffix = contribution.map(|index| format!(" at contribution {index}")).unwrap_or_default()
    )]
    AbsorptionAlignment {
        contribution: Option<usize>,
        expected: usize,
        actual: usize,
    },
    #[error(
        "absorption contribution {contribution} identity changed: expected `{expected}`, got `{actual}`"
    )]
    AbsorptionContributionIdentity {
        contribution: usize,
        expected: String,
        actual: String,
    },
    #[error(
        "absorption contribution {contribution} seed changed: expected node {expected}, got {actual}"
    )]
    AbsorptionSeed {
        contribution: usize,
        expected: usize,
        actual: usize,
    },
    #[error(
        "absorption contribution {contribution} occurrence {occurrence} changed: expected node {expected}, got {actual}"
    )]
    AbsorptionOccurrence {
        contribution: usize,
        occurrence: usize,
        expected: usize,
        actual: usize,
    },
    #[error("absorption plan kind differs from contribution {contribution}")]
    AbsorptionKind { contribution: usize },
    #[error("simple contribution {contribution} carries forbidden alias metadata")]
    SimpleAliases { contribution: usize },
}

struct GraphUpdate {
    node: ClosureNodeId,
    tree: Option<DocTree>,
}

struct SimpleUpdate {
    contribution: usize,
    tree: Option<DocTree>,
}

struct DocumentUpdate {
    tree: Option<DocTree>,
    renames: Vec<OriginRename>,
}

fn qualify_closure(input: ClosureIr) -> Result<ClosureIr, QualifyPassError> {
    let mode = match input.qualification {
        QualificationState::Pending(mode) => mode,
        QualificationState::Applied(_) => return Err(QualifyPassError::AlreadyApplied),
    };
    if input.pending_sources.is_some() || input.pending_embeds.is_some() {
        return Err(QualifyPassError::PendingEarlierPass);
    }
    if input.absorption.is_some() || !input.renames.is_empty() {
        return Err(QualifyPassError::DirtyOutputState);
    }

    // Stage one is immutable by construction: no alias/label rewrite occurs
    // until every occurrence has been judged on exact post-embed text.
    let plan = absorption::analyze(&input)?;
    absorption::validate(&plan, &input)?;

    let mut seen_nodes = HashSet::new();
    let mut graph_updates = Vec::new();
    let mut simple_updates = Vec::new();
    let mut renames = Vec::new();

    for (contribution_index, (contribution, disposition)) in input
        .contributions
        .iter()
        .zip(&plan.contributions)
        .enumerate()
    {
        match (contribution, disposition) {
            (
                ClosureContribution::Normal { emission_order, .. },
                ContributionAbsorption::Normal { occurrences, .. },
            ) => {
                for (occurrence, (node_id, disposition)) in
                    emission_order.iter().copied().zip(occurrences).enumerate()
                {
                    debug_assert_eq!(node_id, disposition.node);
                    if disposition.absorbed || !seen_nodes.insert(node_id) {
                        continue;
                    }
                    let node =
                        input
                            .nodes
                            .get(node_id.0)
                            .ok_or(QualifyPassError::InvalidNodeId {
                                contribution: contribution_index,
                                occurrence,
                                node: node_id.0,
                            })?;
                    let origin = node_qualification_origin(node)?;
                    let update = transform_document(node, &origin, &node.origin, mode);
                    renames.extend(update.renames);
                    graph_updates.push(GraphUpdate {
                        node: node_id,
                        tree: update.tree,
                    });
                }
            }
            (
                ClosureContribution::Simple { meta, document },
                ContributionAbsorption::Simple { .. },
            ) => {
                if !document.aliases.is_empty() {
                    return Err(QualifyPassError::SimpleAliases {
                        contribution: contribution_index,
                    });
                }
                let update = transform_document(document, &meta.origin, &meta.origin, mode);
                renames.extend(update.renames);
                simple_updates.push(SimpleUpdate {
                    contribution: contribution_index,
                    tree: update.tree,
                });
            }
            _ => {
                return Err(QualifyPassError::AbsorptionKind {
                    contribution: contribution_index,
                });
            }
        }
    }

    // Finalization is one transaction over the owned carrier. No graph or
    // occurrence identity changes; only live document payloads and metadata do.
    let mut output = input;
    for update in graph_updates {
        let node = &mut output.nodes[update.node.0];
        if let Some(tree) = update.tree {
            node.tree = tree;
        }
        node.aliases.clear();
    }
    for update in simple_updates {
        let ClosureContribution::Simple { document, .. } =
            &mut output.contributions[update.contribution]
        else {
            unreachable!("the staged simple contribution kept its identity")
        };
        if let Some(tree) = update.tree {
            document.tree = tree;
        }
        document.aliases.clear();
    }
    output.renames = renames;
    output.absorption = Some(plan);
    output.qualification = QualificationState::Applied(mode);
    Ok(output)
}

fn transform_document(
    document: &ClosureDocument,
    qualification_origin: &str,
    rename_origin: &str,
    mode: StaticCompileMode,
) -> DocumentUpdate {
    let original = document.tree.text(document.tree.root());
    let aliased = rewrite_at_bang(&original, &document.aliases);
    let (text, entries) = match mode {
        StaticCompileMode::Plain => (aliased, Vec::new()),
        StaticCompileMode::QualifyPerNode => qualify_contribution(&aliased, qualification_origin),
    };
    let tree = (text != original).then(|| DocTree::parse(&text));
    let renames = entries
        .into_iter()
        .map(|rename| OriginRename {
            origin: rename_origin.to_string(),
            rename,
        })
        .collect();
    DocumentUpdate { tree, renames }
}

fn node_qualification_origin(document: &ClosureDocument) -> Result<String, QualifyPassError> {
    let DocumentAddress::Spec(address) = &document.address else {
        return Err(QualifyPassError::NonSpecGraphNode {
            contribution: 0,
            occurrence: 0,
        });
    };
    if address.doc_path.starts_with("boot/")
        || address.doc_path.starts_with("contract/")
        || address.doc_path.is_empty()
    {
        Ok(document.origin.clone())
    } else {
        Ok(format!(
            "{}/{}",
            document.origin,
            address.doc_path.replace('/', ".")
        ))
    }
}

fn rewrite_at_bang(text: &str, aliases: &BTreeMap<String, SpecAddress>) -> String {
    if aliases.is_empty() {
        return text.to_string();
    }
    let lines: Vec<String> = text.split('\n').map(String::from).collect();
    let fenced = crate::doctree::fence_mask(&lines);
    let out_lines: Vec<String> = lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            if fenced[i] {
                line.clone()
            } else {
                rewrite_at_bang_line(line, aliases)
            }
        })
        .collect();
    out_lines.join("\n")
}

fn rewrite_at_bang_line(line: &str, aliases: &BTreeMap<String, SpecAddress>) -> String {
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut last = 0usize;
    let mut i = 0usize;
    while i + 1 < bytes.len() {
        if bytes[i] == b'@' && bytes[i + 1] == b'!' {
            let id = crate::directives::identifier_run(&line[i + 2..]);
            if !id.is_empty()
                && let Some(target) = aliases.get(id)
            {
                out.push_str(&line[last..i]);
                out.push('@');
                out.push_str(&target.without_pin());
                let after = i + 2 + id.len();
                last = after;
                i = after;
                continue;
            }
        }
        i += 1;
    }
    out.push_str(&line[last..]);
    out
}

#[cfg(test)]
#[path = "qualify/tests.rs"]
mod tests;
