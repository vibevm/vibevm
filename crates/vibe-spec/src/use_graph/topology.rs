//! The one deterministic topology/cycle walker shared by graph consumers.

use std::collections::HashMap;

use crate::SpecAddress;

use super::UseGraphError;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Color {
    Gray,
    Black,
}

/// Order the graph exposed by `edges`, using pinless address identity.
///
/// The callback owns loading/parsing attribution. This cell owns all graph
/// semantics: declaration-order DFS, dependency-before-dependent output,
/// reach deduplication, exact cycle paths, and contract-only cycle admission.
pub(crate) fn order_by(
    seed: &SpecAddress,
    mut edges: impl FnMut(&SpecAddress) -> Result<Vec<SpecAddress>, UseGraphError>,
) -> Result<Vec<String>, UseGraphError> {
    let mut state = HashMap::new();
    let mut order = Vec::new();
    let mut path = Vec::new();
    visit(seed, &mut edges, &mut state, &mut order, &mut path)?;
    Ok(order)
}

fn visit(
    addr: &SpecAddress,
    edges: &mut impl FnMut(&SpecAddress) -> Result<Vec<SpecAddress>, UseGraphError>,
    state: &mut HashMap<String, Color>,
    order: &mut Vec<String>,
    path: &mut Vec<String>,
) -> Result<(), UseGraphError> {
    let key = addr.without_pin();
    match state.get(&key) {
        Some(Color::Black) => return Ok(()),
        Some(Color::Gray) => {
            let start = path
                .iter()
                .position(|candidate| *candidate == key)
                .unwrap_or(0);
            let loop_nodes = &path[start..];
            if is_contract(&key) && loop_nodes.iter().all(|node| is_contract(node)) {
                return Ok(());
            }
            let mut cycle = loop_nodes.to_vec();
            cycle.push(key);
            return Err(UseGraphError::Cycle(cycle));
        }
        None => {}
    }

    state.insert(key.clone(), Color::Gray);
    path.push(key.clone());
    for target in edges(addr)? {
        visit(&target, edges, state, order, path)?;
    }
    state.insert(key.clone(), Color::Black);
    path.pop();
    order.push(key);
    Ok(())
}

fn is_contract(key: &str) -> bool {
    SpecAddress::parse(key)
        .map(|addr| {
            addr.doc_path
                .split('/')
                .any(|segment| segment == "contract")
        })
        .unwrap_or(false)
}
