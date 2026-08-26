//! The one deterministic cycle law, shared by the engine walkers and the
//! inter-pass verifier.
//!
//! A three-colour DFS reports *the cycle it happened to discover*, so its
//! verdict depends on the traversal root and the order edges were declared: the
//! tree path from a cycle's first-discovered node to the back-edge tail need
//! not be the offending cycle, and a second cycle through an already-finished
//! node is never examined at all. Two walkers rooted differently therefore
//! disagree on the same graph.
//!
//! This cell decides on **strongly connected components** instead. An SCC is a
//! property of the relation, not of a walk, so `topology::order_by` (rooted at
//! a contribution seed, in `#use` declaration order) and
//! `compiler::verify::graph` (rooted at arena index 0, in closure-edge order)
//! reach the same verdict on every graph, and PROP-035 §9's contract-only
//! forward-declaration exception is judged over the whole component rather than
//! over one arbitrary loop through it.

use std::collections::HashSet;

/// The first illegal cycle of one relation, as a closed path (`first == last`)
/// anchored on the node that makes it illegal, or `None` when every cycle is
/// admitted.
///
/// `keys` gives every node its **stable semantic identity** — the pinless
/// address, not its arena position. Node ids are handed out by whichever walk
/// discovered the graph, so choosing an offender or a neighbour by id would make
/// the report depend on declaration order; ordering by key makes it a property
/// of the relation. `edges` are `(from, to)` pairs; a pair naming a node outside
/// `keys` is ignored, so a caller that has not yet bounds-checked cannot be
/// panicked here. `admits` answers, per node, whether that node may take part in
/// a cycle — PROP-035 §9's contract exception for use/source, a flat `false` for
/// embed. A component is legal exactly when every one of its members admits.
///
/// Deterministic in every part: components come from Tarjan rather than from a
/// walk, the offender is the key-least non-admitting node in the whole relation,
/// and the path is the first cycle a depth-first walk from that node finds while
/// visiting neighbours in key order.
pub(crate) fn first_illegal_cycle<K: Ord>(
    keys: &[K],
    edges: &[(usize, usize)],
    admits: impl Fn(usize) -> bool,
) -> Option<Vec<usize>> {
    let node_count = keys.len();
    let mut adjacency = vec![Vec::new(); node_count];
    for &(from, to) in edges {
        if from < node_count && to < node_count {
            adjacency[from].push(to);
        }
    }

    let mut offence: Option<(usize, Vec<usize>)> = None;
    for component in components(&adjacency) {
        if !is_cyclic(&component, &adjacency) {
            continue;
        }
        let Some(witness) = component
            .iter()
            .copied()
            .filter(|node| !admits(*node))
            .min_by(|left, right| keys[*left].cmp(&keys[*right]))
        else {
            continue;
        };
        if offence
            .as_ref()
            .is_none_or(|(current, _)| keys[witness] < keys[*current])
        {
            offence = Some((witness, component));
        }
    }
    let (witness, component) = offence?;
    Some(cycle_path(witness, &component, &adjacency, keys))
}

/// A component is cyclic when it holds more than one node, or one node that
/// names itself.
fn is_cyclic(component: &[usize], adjacency: &[Vec<usize>]) -> bool {
    match component {
        [] => false,
        [only] => adjacency[*only].contains(only),
        _ => true,
    }
}

/// Tarjan's strongly connected components, iteratively — a deep `#use` chain
/// must not be able to overflow the stack inside the law itself. Each returned
/// component is sorted ascending.
fn components(adjacency: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let count = adjacency.len();
    let mut index: Vec<Option<usize>> = vec![None; count];
    let mut low = vec![0usize; count];
    let mut on_stack = vec![false; count];
    let mut stack: Vec<usize> = Vec::new();
    let mut frames: Vec<(usize, usize)> = Vec::new();
    let mut next = 0usize;
    let mut out = Vec::new();

    for root in 0..count {
        if index[root].is_some() {
            continue;
        }
        index[root] = Some(next);
        low[root] = next;
        next += 1;
        stack.push(root);
        on_stack[root] = true;
        frames.push((root, 0));

        while let Some(&(node, cursor)) = frames.last() {
            if cursor < adjacency[node].len() {
                frames.last_mut().expect("the frame was just observed").1 += 1;
                let target = adjacency[node][cursor];
                match index[target] {
                    None => {
                        index[target] = Some(next);
                        low[target] = next;
                        next += 1;
                        stack.push(target);
                        on_stack[target] = true;
                        frames.push((target, 0));
                    }
                    Some(seen) => {
                        if on_stack[target] {
                            low[node] = low[node].min(seen);
                        }
                    }
                }
                continue;
            }

            frames.pop();
            if low[node] == index[node].expect("a framed node was indexed on entry") {
                let mut component = Vec::new();
                while let Some(member) = stack.pop() {
                    on_stack[member] = false;
                    component.push(member);
                    if member == node {
                        break;
                    }
                }
                component.sort_unstable();
                out.push(component);
            }
            if let Some(&(parent, _)) = frames.last() {
                low[parent] = low[parent].min(low[node]);
            }
        }
    }
    out
}

/// One concrete closed path through `start` inside its cyclic component: a
/// depth-first walk restricted to the component, visiting neighbours in **key**
/// order so the answer is a property of the relation rather than of the order
/// edges were declared or ids were handed out. Anchoring on the witness makes
/// the rendered path show *why* the component is illegal instead of some
/// admitted sub-loop of it.
fn cycle_path<K: Ord>(
    start: usize,
    component: &[usize],
    adjacency: &[Vec<usize>],
    keys: &[K],
) -> Vec<usize> {
    let members: HashSet<usize> = component.iter().copied().collect();
    let neighbours = |node: usize| {
        let mut targets: Vec<usize> = adjacency[node]
            .iter()
            .copied()
            .filter(|target| members.contains(target))
            .collect();
        targets.sort_unstable();
        targets.dedup();
        targets.sort_by(|left, right| keys[*left].cmp(&keys[*right]));
        targets
    };

    let mut path = vec![start];
    let mut frontier = vec![(neighbours(start), 0usize)];
    let mut visited: HashSet<usize> = HashSet::from([start]);
    while let Some((targets, cursor)) = frontier.last_mut() {
        let Some(&target) = targets.get(*cursor) else {
            frontier.pop();
            path.pop();
            continue;
        };
        *cursor += 1;
        if target == start {
            path.push(start);
            return path;
        }
        if visited.insert(target) {
            path.push(target);
            frontier.push((neighbours(target), 0));
        }
    }
    // Unreachable for a cyclic component; a bare pair keeps this total.
    vec![start, start]
}

#[cfg(test)]
mod tests;
