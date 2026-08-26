//! The named whole-artifact cross-node short-link pass.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR");

use std::collections::{BTreeMap, HashSet};

use crate::doctree::{FenceSnapshot as MarkdownFenceSnapshot, FenceTracker};
use crate::qualify::read_anchor_id;

use super::absorb::{AbsorbPassError, validate_applied_absorption};
use super::ir::{
    AbsorptionState, ArtifactFrame, ArtifactTarget, ClosureContribution, ClosureEdgeKind,
    ClosureIr, ClosureNodeId, ContributionMeta, DocumentAddress, LinkContributionWitness,
    LinkFenceSnapshot, LinkInputDigest, LinkMarkerKey, LinkOccurrence, LinkResult, LinkState,
    QualificationState, StaticCompileMode,
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
    #[error("link contribution {contribution} occurrence {occurrence} is not a spec document")]
    NonSpecNode {
        contribution: usize,
        occurrence: usize,
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
    occurrences: Vec<InputOccurrence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum InputOccurrence {
    Normal {
        contribution: usize,
        occurrence: usize,
        node: ClosureNodeId,
        address: crate::SpecAddress,
        marker: LinkMarkerKey,
        body: String,
    },
    Simple {
        contribution: usize,
        occurrence: usize,
        address: DocumentAddress,
        body: String,
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
    let occurrences = resolve_stream(&plan, closure)?;
    Ok(LinkResult {
        mode: plan.mode,
        input_digest,
        contributions: plan.contributions,
        occurrences,
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
    let mut occurrences = Vec::new();
    for (contribution, current) in closure.contributions.iter().enumerate() {
        match current {
            ClosureContribution::Normal {
                meta,
                seed,
                seed_address,
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
                let DocumentAddress::Spec(node_seed_address) = &seed_node.address else {
                    return Err(LinkPassError::NonSpecSeedNode {
                        contribution,
                        node: seed.0,
                    });
                };
                debug_assert_eq!(node_seed_address.without_pin(), seed_address.without_pin());
                contributions.push(LinkContributionWitness::Normal {
                    meta: meta.clone(),
                    seed: *seed,
                    seed_address: seed_address.clone(),
                    occurrence_count: emission_order.len(),
                });
                for (occurrence, current) in emission_order.iter().enumerate() {
                    let document =
                        closure
                            .nodes
                            .get(current.node.0)
                            .ok_or(LinkPassError::MissingNode {
                                contribution,
                                occurrence,
                                node: current.node.0,
                            })?;
                    let DocumentAddress::Spec(address) = &document.address else {
                        return Err(LinkPassError::NonSpecNode {
                            contribution,
                            occurrence,
                        });
                    };
                    debug_assert_eq!(
                        address.without_pin(),
                        current.requested_address.without_pin()
                    );
                    occurrences.push(InputOccurrence::Normal {
                        contribution,
                        occurrence,
                        node: current.node,
                        address: current.requested_address.clone(),
                        marker: LinkMarkerKey::from_address(&current.requested_address),
                        body: document.tree.text(document.tree.root()),
                    });
                }
            }
            ClosureContribution::Simple { meta, document } => {
                contributions.push(LinkContributionWitness::Simple {
                    meta: meta.clone(),
                    address: document.address.clone(),
                });
                occurrences.push(InputOccurrence::Simple {
                    contribution,
                    occurrence: 0,
                    address: document.address.clone(),
                    body: document.tree.text(document.tree.root()),
                });
            }
            ClosureContribution::Elided { meta } => {
                contributions.push(LinkContributionWitness::Elided { meta: meta.clone() });
            }
            ClosureContribution::Hoisted { meta, target } => {
                contributions.push(LinkContributionWitness::Hoisted {
                    meta: meta.clone(),
                    target: target.clone(),
                });
            }
        }
    }
    Ok(PlannedLink {
        mode,
        contributions,
        occurrences,
    })
}

type Definitions = BTreeMap<String, Vec<(String, String)>>;

fn definitions_for(closure: &ClosureIr, contribution: usize) -> Result<Definitions, LinkPassError> {
    let ClosureContribution::Normal { emission_order, .. } = &closure.contributions[contribution]
    else {
        return Ok(BTreeMap::new());
    };
    let mut seen = HashSet::new();
    let mut definitions = BTreeMap::new();
    for (occurrence, current) in emission_order.iter().enumerate() {
        if !seen.insert(current.node) {
            continue;
        }
        let document = closure
            .nodes
            .get(current.node.0)
            .ok_or(LinkPassError::MissingNode {
                contribution,
                occurrence,
                node: current.node.0,
            })?;
        for entry in &closure.renames {
            if entry.origin != document.origin
                || !document
                    .tree
                    .anchored()
                    .any(|(_, id)| id == entry.rename.qualified)
            {
                continue;
            }
            let rename = &entry.rename;
            let candidate = (document.origin.clone(), rename.qualified.clone());
            let candidates = definitions
                .entry(rename.original.clone())
                .or_insert_with(Vec::new);
            if !candidates.contains(&candidate) {
                candidates.push(candidate);
            }
        }
    }
    Ok(definitions)
}

fn resolve_stream(
    plan: &PlannedLink,
    closure: &ClosureIr,
) -> Result<Vec<LinkOccurrence>, LinkPassError> {
    let qualified = matches!(plan.mode, StaticCompileMode::QualifyPerNode);
    let mut output = Vec::with_capacity(plan.occurrences.len());
    let mut current_contribution = None;
    let mut fence = FenceTracker::default();
    let mut definitions = BTreeMap::new();
    for occurrence in &plan.occurrences {
        let contribution = match occurrence {
            InputOccurrence::Normal { contribution, .. }
            | InputOccurrence::Simple { contribution, .. } => *contribution,
        };
        if current_contribution != Some(contribution) {
            current_contribution = Some(contribution);
            fence = FenceTracker::default();
            definitions = definitions_for(closure, contribution)?;
        }
        match occurrence {
            InputOccurrence::Normal {
                contribution,
                occurrence,
                node,
                address,
                marker,
                body,
            } => {
                let fence_before = link_fence_snapshot(fence.snapshot());
                let body = link_text(body, &mut fence, &definitions, qualified)?;
                let fence_after = link_fence_snapshot(fence.snapshot());
                output.push(LinkOccurrence::Normal {
                    contribution: *contribution,
                    occurrence: *occurrence,
                    node: *node,
                    address: address.clone(),
                    marker: marker.clone(),
                    fence_before,
                    fence_after,
                    trailing_newline_required: !body.ends_with('\n'),
                    body,
                });
            }
            InputOccurrence::Simple {
                contribution,
                occurrence,
                address,
                body,
            } => {
                let fence_before = link_fence_snapshot(fence.snapshot());
                let body = link_text(body, &mut fence, &definitions, qualified)?;
                let fence_after = link_fence_snapshot(fence.snapshot());
                output.push(LinkOccurrence::Simple {
                    contribution: *contribution,
                    occurrence: *occurrence,
                    address: address.clone(),
                    fence_before,
                    fence_after,
                    trailing_newline_required: !body.ends_with('\n'),
                    body,
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
    if actual.occurrences != expected.occurrences {
        return Err(LinkPassError::ReplayMismatch {
            field: "linked occurrences",
        });
    }
    Ok(())
}

/// Temporary compatibility serializer. Concrete markers live here only until
/// named assemble/emit replace this one-seed public-API tail.
pub(crate) fn linked_text(closure: &ClosureIr) -> Result<String, LinkPassError> {
    validate_linked(closure)?;
    debug_assert!(matches!(
        closure.context().frame(),
        ArtifactFrame::CompatibilityFragment
    ));
    let LinkState::Linked(result) = &closure.link else {
        unreachable!("linked validator accepted only Linked")
    };
    let mut output = String::new();
    for occurrence in &result.occurrences {
        match occurrence {
            LinkOccurrence::Normal {
                marker,
                body,
                trailing_newline_required,
                ..
            } => {
                output.push_str(&crate::markers::open(marker.as_str()));
                output.push('\n');
                output.push_str(body);
                if *trailing_newline_required {
                    output.push('\n');
                }
                output.push_str(&crate::markers::close(marker.as_str()));
                output.push('\n');
            }
            LinkOccurrence::Simple {
                body,
                trailing_newline_required,
                ..
            } => {
                output.push_str(body);
                if *trailing_newline_required {
                    output.push('\n');
                }
            }
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
