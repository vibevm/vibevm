//! The `#use` graph (PROP-035 §7.2) — tree-shaking by dependency edges.
//!
//! `#use <spec://…>` (and an `@spec://…` in-place use, §7.4) is a dependency
//! edge: the target must be linked *before* its user. Starting from a seed, we
//! walk the edges cascade-style — the seed's uses, then their uses, and so on —
//! and return the reachable nodes in **topological order**: every dependency
//! before its dependents, the seed last. This is the order the static compiler
//! emits in (§8 phase 2) and the set a structural load pulls; a node nothing
//! uses never enters it (tree-shaking).
//!
//! Cycles are detected via a three-colour DFS and reported with the offending
//! path (`a → b → a`). PROP-035 §9 makes a `#use` cycle *between contracts*
//! legal (the forward-declaration case) — but resolving it means emitting the
//! contracts before any source body, which is the emission layer's job (§8 /
//! §12). This layer reports every cycle; that layer will admit the contract-only
//! ones. `#embed` and `#source` are not dependency edges and are ignored here.

use std::collections::HashMap;

use crate::address::SpecAddress;
use crate::directives::{DirectiveKind, Directives};
use crate::embed::SectionSource;

/// Why the use-graph could not be ordered.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UseGraphError {
    #[error("use cycle: {}", .0.join(" -> "))]
    Cycle(Vec<String>),
    #[error("cannot resolve use {addr}: {reason}")]
    Unresolved { addr: String, reason: String },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Color {
    Gray,
    Black,
}

/// Walk the use-graph reachable from `seed` and return its node keys
/// (`SpecAddress::without_pin`) in topological order — every dependency before
/// its dependents, `seed` last. Deduplicated: a node reached by several paths
/// appears once.
pub fn topo_order_from(
    seed: &SpecAddress,
    source: &impl SectionSource,
) -> Result<Vec<String>, UseGraphError> {
    let mut state: HashMap<String, Color> = HashMap::new();
    let mut order = Vec::new();
    let mut path = Vec::new();
    visit(seed, source, &mut state, &mut order, &mut path)?;
    Ok(order)
}

fn visit(
    addr: &SpecAddress,
    source: &impl SectionSource,
    state: &mut HashMap<String, Color>,
    order: &mut Vec<String>,
    path: &mut Vec<String>,
) -> Result<(), UseGraphError> {
    let key = addr.without_pin();
    match state.get(&key) {
        Some(Color::Black) => return Ok(()),
        Some(Color::Gray) => {
            // Back-edge to a node still on the stack: a cycle. PROP-035 §9 makes
            // it legal when every node on the loop is a contract (the
            // forward-declaration case) — admit it by not re-entering the edge.
            // A loop touching any source node is a hard error.
            let start = path.iter().position(|k| *k == key).unwrap_or(0);
            let loop_nodes = &path[start..];
            if is_contract(&key) && loop_nodes.iter().all(|k| is_contract(k)) {
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

    let text = source
        .section_text(addr)
        .map_err(|reason| UseGraphError::Unresolved {
            addr: addr.to_string(),
            reason,
        })?;
    let directives = Directives::parse(&text);

    // Dependency edges = #use directives + @spec in-place uses, in line order.
    let mut edges: Vec<(usize, &SpecAddress)> = directives
        .directives
        .iter()
        .filter(|d| d.kind == DirectiveKind::Use)
        .map(|d| (d.line, &d.address))
        .chain(
            directives
                .in_place_uses
                .iter()
                .map(|u| (u.line, &u.address)),
        )
        .collect();
    edges.sort_by_key(|(line, _)| *line);

    for (_, target) in edges {
        visit(target, source, state, order, path)?;
    }

    state.insert(key.clone(), Color::Black);
    path.pop();
    order.push(key);
    Ok(())
}

/// A node is a contract if its doc-path has a `contract` segment (PROP-035 §4) —
/// the layer where a `#use` cycle is a legal forward declaration (§9).
fn is_contract(key: &str) -> bool {
    SpecAddress::parse(key)
        .map(|addr| addr.doc_path.split('/').any(|seg| seg == "contract"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct MockSource(HashMap<String, String>);

    impl MockSource {
        fn new(pairs: &[(&str, &str)]) -> Self {
            MockSource(
                pairs
                    .iter()
                    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                    .collect(),
            )
        }
    }

    impl SectionSource for MockSource {
        fn section_text(&self, addr: &SpecAddress) -> Result<String, String> {
            self.0
                .get(&addr.without_pin())
                .cloned()
                .ok_or_else(|| "not in mock".to_string())
        }
    }

    fn seed() -> SpecAddress {
        SpecAddress::parse("spec://vibevm/a#r").unwrap()
    }

    #[test]
    fn linear_cascade_orders_dependencies_first() {
        let src = MockSource::new(&[
            ("spec://vibevm/a#r", "#use spec://vibevm/b#r"),
            ("spec://vibevm/b#r", "#use spec://vibevm/c#r"),
            ("spec://vibevm/c#r", "leaf"),
        ]);
        let order = topo_order_from(&seed(), &src).unwrap();
        assert_eq!(
            order,
            vec![
                "spec://vibevm/c#r".to_string(),
                "spec://vibevm/b#r".to_string(),
                "spec://vibevm/a#r".to_string(),
            ]
        );
    }

    #[test]
    fn diamond_deduplicates_the_shared_dependency() {
        let src = MockSource::new(&[
            (
                "spec://vibevm/a#r",
                "#use spec://vibevm/b#r\n#use spec://vibevm/c#r",
            ),
            ("spec://vibevm/b#r", "#use spec://vibevm/d#r"),
            ("spec://vibevm/c#r", "#use spec://vibevm/d#r"),
            ("spec://vibevm/d#r", "shared leaf"),
        ]);
        let order = topo_order_from(&seed(), &src).unwrap();
        assert_eq!(order.len(), 4, "d appears once: {order:?}");
        assert_eq!(order.first().unwrap(), "spec://vibevm/d#r");
        assert_eq!(order.last().unwrap(), "spec://vibevm/a#r");
    }

    #[test]
    fn in_place_use_is_a_dependency_edge() {
        let src = MockSource::new(&[
            ("spec://vibevm/a#r", "prose @spec://vibevm/b#r here"),
            ("spec://vibevm/b#r", "leaf"),
        ]);
        let order = topo_order_from(&seed(), &src).unwrap();
        assert_eq!(
            order,
            vec![
                "spec://vibevm/b#r".to_string(),
                "spec://vibevm/a#r".to_string(),
            ]
        );
    }

    #[test]
    fn a_cycle_is_reported_with_its_path() {
        let src = MockSource::new(&[
            ("spec://vibevm/a#r", "#use spec://vibevm/b#r"),
            ("spec://vibevm/b#r", "#use spec://vibevm/a#r"),
        ]);
        let err = topo_order_from(&seed(), &src).unwrap_err();
        match err {
            UseGraphError::Cycle(path) => {
                assert_eq!(path.first().unwrap(), "spec://vibevm/a#r");
                assert_eq!(path.last().unwrap(), "spec://vibevm/a#r");
                assert!(path.contains(&"spec://vibevm/b#r".to_string()));
            }
            other => panic!("expected a cycle, got {other:?}"),
        }
    }

    #[test]
    fn an_unresolved_use_is_reported() {
        let src = MockSource::new(&[("spec://vibevm/a#r", "#use spec://vibevm/missing#r")]);
        let err = topo_order_from(&seed(), &src).unwrap_err();
        assert!(matches!(err, UseGraphError::Unresolved { .. }));
    }

    #[test]
    fn a_leaf_seed_orders_just_itself() {
        let src = MockSource::new(&[("spec://vibevm/a#r", "no uses here")]);
        let order = topo_order_from(&seed(), &src).unwrap();
        assert_eq!(order, vec!["spec://vibevm/a#r".to_string()]);
    }

    #[test]
    fn a_contract_cycle_is_admitted() {
        let src = MockSource::new(&[
            (
                "spec://org.vibevm.demo/lib/contract/a#r",
                "#use spec://org.vibevm.demo/lib/contract/b#r",
            ),
            (
                "spec://org.vibevm.demo/lib/contract/b#r",
                "#use spec://org.vibevm.demo/lib/contract/a#r",
            ),
        ]);
        let seed = SpecAddress::parse("spec://org.vibevm.demo/lib/contract/a#r").unwrap();
        let order = topo_order_from(&seed, &src).unwrap();
        assert_eq!(order.len(), 2, "both contracts present: {order:?}");
    }

    #[test]
    fn a_cycle_touching_a_source_node_is_rejected() {
        let src = MockSource::new(&[
            (
                "spec://org.vibevm.demo/lib/contract/a#r",
                "#use spec://org.vibevm.demo/lib/source/b#r",
            ),
            (
                "spec://org.vibevm.demo/lib/source/b#r",
                "#use spec://org.vibevm.demo/lib/contract/a#r",
            ),
        ]);
        let seed = SpecAddress::parse("spec://org.vibevm.demo/lib/contract/a#r").unwrap();
        assert!(matches!(
            topo_order_from(&seed, &src),
            Err(UseGraphError::Cycle(_))
        ));
    }
}
