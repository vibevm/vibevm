//! The named whole-artifact cross-node short-link pass.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR");

use std::collections::BTreeMap;

use crate::doctree::{FenceSnapshot as MarkdownFenceSnapshot, FenceTracker};
use crate::qualify::read_anchor_id;

use super::absorb::{AbsorbPassError, validate_applied_absorption};
use super::ir::{
    AbsorptionState, ClosureContribution, ClosureIr, ClosureNodeId, ContributionMeta,
    DocumentAddress, LinkChunk, LinkContributionWitness, LinkFenceSnapshot, LinkInputDigest,
    LinkLiteralKind, LinkResult, LinkState, OriginRename, QualificationState, StaticCompileMode,
};
use super::pass::{Pass, PassName};

pub(crate) const LINK_PASS_NAME: &str = "link";

mod digest;
use digest::digest_input;

pub(crate) struct LinkPass {
    name: PassName,
}

impl LinkPass {
    pub(crate) fn new() -> Self {
        Self {
            name: PassName::new(LINK_PASS_NAME)
                .expect("the static built-in link pass name is non-blank"),
        }
    }
}

impl Pass for LinkPass {
    type Input = ClosureIr;
    type Output = ClosureIr;
    type Error = LinkPassError;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, input: ClosureIr) -> Result<ClosureIr, LinkPassError> {
        #[cfg(test)]
        LINK_INVOCATIONS.with(|count| count.set(count.get() + 1));
        link_closure(input)
    }
}

#[cfg(test)]
std::thread_local! {
    static LINK_INVOCATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_link_invocations() {
    LINK_INVOCATIONS.with(|count| count.set(0));
}

#[cfg(test)]
pub(crate) fn link_invocations() -> usize {
    LINK_INVOCATIONS.with(std::cell::Cell::get)
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum LinkPassError {
    #[error("link requires applied qualification state")]
    QualificationPending,
    #[error("link requires named merge and embed to consume their pending state")]
    PendingEarlierPass,
    #[error("link requires valid applied absorption state: {0}")]
    InvalidAbsorption(#[source] Box<AbsorbPassError>),
    #[error("link cannot apply an already linked closure")]
    AlreadyLinked,
    #[error("link contribution {contribution} seed names missing closure node {node}")]
    MissingSeedNode { contribution: usize, node: usize },
    #[error("link contribution {contribution} seed node {node} is not a spec document")]
    NonSpecSeedNode { contribution: usize, node: usize },
    #[error(
        "link contribution {contribution} occurrence {occurrence} names missing closure node {node}"
    )]
    MissingNode {
        contribution: usize,
        occurrence: usize,
        node: usize,
    },
    #[error(
        "link contribution {contribution} occurrence {occurrence} node {node} is not a spec document"
    )]
    NonSpecNode {
        contribution: usize,
        occurrence: usize,
        node: usize,
    },
    #[error("ambiguous short link `{label}`: defined by {}", .candidates.join(", "))]
    AmbiguousShortLink {
        label: String,
        candidates: Vec<String>,
    },
    #[error("linked closure replay differs at {field}")]
    ReplayMismatch { field: &'static str },
    #[error("linked closure validator requires linked state")]
    Unlinked,
}

impl LinkPassError {
    pub(crate) fn into_compile_error(self) -> crate::pipeline::CompileError {
        match self {
            Self::AmbiguousShortLink { label, candidates } => {
                crate::pipeline::CompileError::AmbiguousShortLink { label, candidates }
            }
            other => panic!("the built-in link pass returned invalid private state: {other}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlannedLink {
    mode: StaticCompileMode,
    contributions: Vec<LinkContributionWitness>,
    chunks: Vec<InputChunk>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum InputChunk {
    Literal {
        kind: LinkLiteralKind,
        bytes: String,
    },
    NormalOccurrence {
        contribution: usize,
        occurrence: usize,
        node: ClosureNodeId,
        address: crate::SpecAddress,
        bytes: String,
    },
    SimpleOccurrence {
        contribution: usize,
        occurrence: usize,
        address: DocumentAddress,
        bytes: String,
    },
}

fn link_closure(mut input: ClosureIr) -> Result<ClosureIr, LinkPassError> {
    if matches!(input.link, LinkState::Linked(_)) {
        return Err(LinkPassError::AlreadyLinked);
    }
    let result = derive_result(&input)?;
    input.link = LinkState::Linked(result);
    validate_linked(&input)?;
    Ok(input)
}

fn derive_result(closure: &ClosureIr) -> Result<LinkResult, LinkPassError> {
    let plan = plan_stream(closure)?;
    let input_digest = digest_input(closure, &plan);
    let chunks = resolve_stream(&plan, &closure.renames)?;
    Ok(LinkResult {
        mode: plan.mode,
        input_digest,
        contributions: plan.contributions,
        chunks,
    })
}

fn plan_stream(closure: &ClosureIr) -> Result<PlannedLink, LinkPassError> {
    if closure.pending_sources.is_some() || closure.pending_embeds.is_some() {
        return Err(LinkPassError::PendingEarlierPass);
    }
    let mode = match closure.qualification {
        QualificationState::Applied(mode) => mode,
        QualificationState::Pending(_) => return Err(LinkPassError::QualificationPending),
    };
    validate_applied_absorption(closure)
        .map_err(|error| LinkPassError::InvalidAbsorption(Box::new(error)))?;
    let AbsorptionState::Applied(absorption) = &closure.absorption else {
        unreachable!("applied absorption validator accepted only Applied")
    };
    debug_assert_eq!(absorption.mode, mode);

    let mut contributions = Vec::with_capacity(closure.contributions.len());
    let mut chunks = Vec::new();
    for (contribution, current) in closure.contributions.iter().enumerate() {
        match current {
            ClosureContribution::Normal {
                meta,
                seed,
                emission_order,
            } => {
                let seed_node =
                    closure
                        .nodes
                        .get(seed.0)
                        .ok_or(LinkPassError::MissingSeedNode {
                            contribution,
                            node: seed.0,
                        })?;
                let DocumentAddress::Spec(seed_address) = &seed_node.address else {
                    return Err(LinkPassError::NonSpecSeedNode {
                        contribution,
                        node: seed.0,
                    });
                };
                contributions.push(LinkContributionWitness::Normal {
                    meta: meta.clone(),
                    seed: *seed,
                    seed_address: seed_address.clone(),
                    occurrence_count: emission_order.len(),
                });
                for (occurrence, node) in emission_order.iter().copied().enumerate() {
                    let document = closure
                        .nodes
                        .get(node.0)
                        .ok_or(LinkPassError::MissingNode {
                            contribution,
                            occurrence,
                            node: node.0,
                        })?;
                    let DocumentAddress::Spec(address) = &document.address else {
                        return Err(LinkPassError::NonSpecNode {
                            contribution,
                            occurrence,
                            node: node.0,
                        });
                    };
                    chunks.push(InputChunk::Literal {
                        kind: LinkLiteralKind::NormalOpen,
                        bytes: format!("{}\n", crate::markers::open(&address.without_pin())),
                    });
                    let body = document.tree.text(document.tree.root());
                    let needs_newline = !body.ends_with('\n');
                    chunks.push(InputChunk::NormalOccurrence {
                        contribution,
                        occurrence,
                        node,
                        address: address.clone(),
                        bytes: body,
                    });
                    if needs_newline {
                        chunks.push(InputChunk::Literal {
                            kind: LinkLiteralKind::ForcedNewline,
                            bytes: "\n".to_string(),
                        });
                    }
                    chunks.push(InputChunk::Literal {
                        kind: LinkLiteralKind::NormalClose,
                        bytes: format!("{}\n", crate::markers::close(&address.without_pin())),
                    });
                }
            }
            ClosureContribution::Simple { meta, document } => {
                contributions.push(LinkContributionWitness::Simple {
                    meta: meta.clone(),
                    address: document.address.clone(),
                });
                let body = document.tree.text(document.tree.root());
                let needs_newline = !body.ends_with('\n');
                chunks.push(InputChunk::SimpleOccurrence {
                    contribution,
                    occurrence: 0,
                    address: document.address.clone(),
                    bytes: body,
                });
                if needs_newline {
                    chunks.push(InputChunk::Literal {
                        kind: LinkLiteralKind::ForcedNewline,
                        bytes: "\n".to_string(),
                    });
                }
            }
        }
    }
    Ok(PlannedLink {
        mode,
        contributions,
        chunks,
    })
}

type Definitions = BTreeMap<String, Vec<(String, String)>>;

fn definitions(renames: &[OriginRename]) -> Definitions {
    let mut definitions = BTreeMap::new();
    for entry in renames {
        definitions
            .entry(entry.rename.original.clone())
            .or_insert_with(Vec::new)
            .push((entry.origin.clone(), entry.rename.qualified.clone()));
    }
    definitions
}

fn resolve_stream(
    plan: &PlannedLink,
    renames: &[OriginRename],
) -> Result<Vec<LinkChunk>, LinkPassError> {
    let definitions = definitions(renames);
    let mut fence = FenceTracker::default();
    let qualified = matches!(plan.mode, StaticCompileMode::QualifyPerNode);
    let mut output = Vec::with_capacity(plan.chunks.len());
    for chunk in &plan.chunks {
        match chunk {
            InputChunk::Literal { kind, bytes } => output.push(LinkChunk::Literal {
                kind: *kind,
                bytes: link_text(bytes, &mut fence, &definitions, qualified)?,
            }),
            InputChunk::NormalOccurrence {
                contribution,
                occurrence,
                node,
                address,
                bytes,
            } => {
                let fence_before = link_fence_snapshot(fence.snapshot());
                let bytes = link_text(bytes, &mut fence, &definitions, qualified)?;
                let fence_after = link_fence_snapshot(fence.snapshot());
                output.push(LinkChunk::NormalOccurrence {
                    contribution: *contribution,
                    occurrence: *occurrence,
                    node: *node,
                    address: address.clone(),
                    fence_before,
                    fence_after,
                    bytes,
                });
            }
            InputChunk::SimpleOccurrence {
                contribution,
                occurrence,
                address,
                bytes,
            } => {
                let fence_before = link_fence_snapshot(fence.snapshot());
                let bytes = link_text(bytes, &mut fence, &definitions, qualified)?;
                let fence_after = link_fence_snapshot(fence.snapshot());
                output.push(LinkChunk::SimpleOccurrence {
                    contribution: *contribution,
                    occurrence: *occurrence,
                    address: address.clone(),
                    fence_before,
                    fence_after,
                    bytes,
                });
            }
        }
    }
    Ok(output)
}

fn link_fence_snapshot(snapshot: MarkdownFenceSnapshot) -> LinkFenceSnapshot {
    match snapshot {
        MarkdownFenceSnapshot::Closed => LinkFenceSnapshot::Closed,
        MarkdownFenceSnapshot::Open { delimiter, run } => {
            LinkFenceSnapshot::Open { delimiter, run }
        }
    }
}

fn link_text(
    text: &str,
    fence: &mut FenceTracker,
    definitions: &Definitions,
    qualified: bool,
) -> Result<String, LinkPassError> {
    let mut output = Vec::new();
    for line in text.split('\n') {
        if fence.classify(line) || !qualified {
            output.push(line.to_string());
        } else {
            output.push(rewrite_line(line, definitions)?);
        }
    }
    Ok(output.join("\n"))
}

fn rewrite_line(line: &str, definitions: &Definitions) -> Result<String, LinkPassError> {
    let bytes = line.as_bytes();
    let mut output = String::with_capacity(line.len());
    let mut last = 0;
    let mut index = 0;
    let mut in_code = false;
    while index < bytes.len() {
        if bytes[index] == b'`' {
            in_code = !in_code;
            index += 1;
            continue;
        }
        if !in_code
            && bytes[index] == b'('
            && bytes.get(index + 1) == Some(&b'#')
            && let Some((id, after_id)) = read_anchor_id(bytes, index + 2)
            && bytes.get(after_id) == Some(&b')')
        {
            match definitions.get(id) {
                Some(candidates) if candidates.len() == 1 => {
                    output.push_str(&line[last..index]);
                    output.push_str("(#");
                    output.push_str(&candidates[0].1);
                    output.push(')');
                    last = after_id + 1;
                    index = after_id + 1;
                    continue;
                }
                Some(candidates) => {
                    let mut candidates: Vec<String> = candidates
                        .iter()
                        .map(|(origin, qualified)| format!("{qualified} ({origin})"))
                        .collect();
                    candidates.sort();
                    return Err(LinkPassError::AmbiguousShortLink {
                        label: id.to_string(),
                        candidates,
                    });
                }
                None => {}
            }
        }
        index += 1;
    }
    output.push_str(&line[last..]);
    Ok(output)
}

pub(crate) fn validate_linked(closure: &ClosureIr) -> Result<(), LinkPassError> {
    let expected = derive_result(closure)?;
    let LinkState::Linked(actual) = &closure.link else {
        return Err(LinkPassError::Unlinked);
    };
    if actual.mode != expected.mode {
        return Err(LinkPassError::ReplayMismatch { field: "mode" });
    }
    if actual.input_digest != expected.input_digest {
        return Err(LinkPassError::ReplayMismatch {
            field: "input digest",
        });
    }
    if actual.contributions != expected.contributions {
        return Err(LinkPassError::ReplayMismatch {
            field: "contribution witnesses",
        });
    }
    if actual.chunks != expected.chunks {
        return Err(LinkPassError::ReplayMismatch {
            field: "linked chunks",
        });
    }
    Ok(())
}

pub(crate) fn linked_text(closure: &ClosureIr) -> Result<String, LinkPassError> {
    validate_linked(closure)?;
    let LinkState::Linked(result) = &closure.link else {
        unreachable!("linked validator accepted only Linked")
    };
    let mut output = String::new();
    for chunk in &result.chunks {
        match chunk {
            LinkChunk::Literal { bytes, .. }
            | LinkChunk::NormalOccurrence { bytes, .. }
            | LinkChunk::SimpleOccurrence { bytes, .. } => output.push_str(bytes),
        }
    }
    Ok(output)
}

#[cfg(test)]
#[path = "link/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "link/fence_tests.rs"]
mod fence_tests;

#[cfg(test)]
#[path = "link/test_support.rs"]
mod test_support;
