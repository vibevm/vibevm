//! The mechanism layer's ONE dependency-order walk.
//!
//! Both executors walk the same graph shape — targets that declare
//! outputs and consume artifact ids — so they walk it with the same code.
//! What genuinely differs between them is ONE decision, and it is a
//! parameter rather than a second copy of the walk: a build input naming
//! an artifact no build target produces is an error (the build graph is
//! closed under itself), while a package input naming one is ordinary
//! (the phase-forward law says it is a BUILD output, and the input
//! resolver finds it in the engine's record or refuses there, where the
//! refusal can name the missing record).
//!
//! The walk is depth-first in declaration order: deterministic, and it
//! names the cycle it refuses. A manifest that parsed is already acyclic —
//! this is the executor refusing to be the place where a programmatically
//! built target set turns into an infinite loop.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY");

use std::collections::BTreeMap;

use vibe_core::manifest::{ArtifactInput, ArtifactOutput};

/// One node of a producer graph, as the walk needs to see it.
pub(crate) trait GraphNode {
    fn id(&self) -> &str;
    fn outputs(&self) -> &[ArtifactOutput];
    fn inputs(&self) -> Option<&[ArtifactInput]>;
}

/// What the walk does with a consumed artifact no node in the set
/// produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unresolved {
    /// Refuse — the graph is closed under itself.
    Refuse,
    /// Leave it alone — a later law owns it.
    Defer,
}

/// Why a target set has no dependency order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OrderFault {
    Cycle { cycle: String },
    UnknownInput { target: String, input: String },
}

/// Dependency order over one declared target set.
pub(crate) fn dag_order<N: GraphNode>(
    nodes: &[N],
    unresolved: Unresolved,
) -> Result<Vec<usize>, OrderFault> {
    let mut producer: BTreeMap<&str, usize> = BTreeMap::new();
    for (index, node) in nodes.iter().enumerate() {
        for output in node.outputs() {
            producer.insert(output.id.as_str(), index);
        }
    }
    let mut state = vec![Visit::Unseen; nodes.len()];
    let mut order = Vec::with_capacity(nodes.len());
    for root in 0..nodes.len() {
        visit(
            root,
            nodes,
            &producer,
            unresolved,
            &mut state,
            &mut order,
            &mut Vec::new(),
        )?;
    }
    Ok(order)
}

/// Colour of one node in the depth-first walk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Visit {
    Unseen,
    OnStack,
    Done,
}

#[allow(clippy::too_many_arguments, reason = "one walk's frame, all named")]
fn visit<N: GraphNode>(
    index: usize,
    nodes: &[N],
    producer: &BTreeMap<&str, usize>,
    unresolved: Unresolved,
    state: &mut [Visit],
    order: &mut Vec<usize>,
    stack: &mut Vec<usize>,
) -> Result<(), OrderFault> {
    match state.get(index) {
        Some(Visit::Done) => return Ok(()),
        Some(Visit::OnStack) => {
            let mut cycle: Vec<String> = stack
                .iter()
                .skip_while(|entry| **entry != index)
                .filter_map(|entry| nodes.get(*entry).map(|node| node.id().to_owned()))
                .collect();
            if let Some(node) = nodes.get(index) {
                cycle.push(node.id().to_owned());
            }
            return Err(OrderFault::Cycle {
                cycle: cycle.join(" -> "),
            });
        }
        Some(Visit::Unseen) => {}
        None => return Ok(()),
    }
    let Some(node) = nodes.get(index) else {
        return Ok(());
    };
    state[index] = Visit::OnStack;
    stack.push(index);
    for input in node.inputs().into_iter().flatten() {
        let Some(consumed) = input.artifact_ref() else {
            continue;
        };
        let Some(upstream) = producer.get(consumed) else {
            match unresolved {
                Unresolved::Refuse => {
                    return Err(OrderFault::UnknownInput {
                        target: node.id().to_owned(),
                        input: consumed.to_owned(),
                    });
                }
                Unresolved::Defer => continue,
            }
        };
        if *upstream != index {
            visit(*upstream, nodes, producer, unresolved, state, order, stack)?;
        }
    }
    stack.pop();
    state[index] = Visit::Done;
    order.push(index);
    Ok(())
}
