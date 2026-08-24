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
//! ones. `#embed` is not an edge and is ignored. `#source` is a *fold* edge,
//! not a use edge: [`topo_order_from`] still ignores it, and the same
//! three-colour walk over `#source` alone is exposed as [`source_fold_order`]
//! for the contract→impl fold — one traverser, two edge sets.

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

/// Which edges a walk follows. The cycle law — three-colour DFS, contract-only
/// loop admission, dedup by reach — is shared verbatim; only the edge set
/// differs. One `visit` body serves both, parameterised by this.
#[derive(Clone, Copy, PartialEq, Eq)]
enum EdgeKind {
    /// `#use` directives + `@spec` in-place uses (§7.2 / §7.4) — the link edge.
    Use,
    /// `#source` directives (§7.3) — the contract→impl fold edge.
    Source,
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
    visit(
        seed,
        source,
        &mut state,
        &mut order,
        &mut path,
        EdgeKind::Use,
    )?;
    Ok(order)
}

/// Walk the `#source` edges reachable from `seed` (PROP-035 §7.3) and return
/// their node keys (`SpecAddress::without_pin`) in fold order: deepest sources
/// first, `seed` last — so a source that itself declares `#source` is folded
/// before it merges into its parent. Deduplicated: a contract reached by
/// several paths folds once. A `#source` cycle *between contracts* is legal
/// (the forward-declaration case); one touching any non-contract node is a hard
/// error, reported with its path. This is the same three-colour DFS as
/// [`topo_order_from`] — one traverser, two edge sets — following only
/// `#source` (never `#use` or `@spec`).
pub fn source_fold_order(
    seed: &SpecAddress,
    source: &impl SectionSource,
) -> Result<Vec<String>, UseGraphError> {
    let mut state: HashMap<String, Color> = HashMap::new();
    let mut order = Vec::new();
    let mut path = Vec::new();
    visit(
        seed,
        source,
        &mut state,
        &mut order,
        &mut path,
        EdgeKind::Source,
    )?;
    Ok(order)
}

fn visit(
    addr: &SpecAddress,
    source: &impl SectionSource,
    state: &mut HashMap<String, Color>,
    order: &mut Vec<String>,
    path: &mut Vec<String>,
    kind: EdgeKind,
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

    // Edges to follow, selected by `kind`. `Use` follows explicit `#use`
    // declarations (§7.2) — and ONLY those: an `@spec://…` in-place use
    // (§7.4) is the AGENT's mandatory read, not a compiler splice. The
    // address itself is the compiled artefact — AOT-splicing every `@spec`
    // target multiplied the host lane tenfold (250 KB → 2.5 MB, the
    // ##OPEN-CLOSURE-EXPLOSION risk realised, 2026-08-24), where the §2
    // equivalence invariant is carried by the agent's read obligation
    // instead. `Source` follows only `#source` (§7.3) — and every such edge
    // passes through [`source_addresses`], the ONE place that knows a
    // `#source` may carry a pattern (a glob expands to its sorted members, a
    // point address to itself), flattened in declaration order. Owning the
    // addresses (rather than borrowing the parsed directives) is what lets
    // an expanded glob contribute addresses no directive ever held.
    let edges: Vec<SpecAddress> = match kind {
        EdgeKind::Use => {
            let directives = Directives::parse(&text);
            let mut e: Vec<(usize, SpecAddress)> = directives
                .directives
                .iter()
                .filter(|d| d.kind == DirectiveKind::Use)
                .map(|d| (d.line, d.address.clone()))
                .collect();
            e.sort_by_key(|(line, _)| *line);
            e.into_iter().map(|(_, a)| a).collect()
        }
        EdgeKind::Source => source_addresses(&text, source)?,
    };

    for target in &edges {
        visit(target, source, state, order, path, kind)?;
    }

    state.insert(key.clone(), Color::Black);
    path.pop();
    order.push(key);
    Ok(())
}

/// The concrete `#source` addresses a document declares, in declaration order —
/// each EXPANDED to the addresses it denotes (a glob to its sorted members, a
/// point address to itself) and flattened into one list.
///
/// This is the ONE place that knows a `#source` edge may carry a pattern. The
/// fold guard ([`source_fold_order`], via [`visit`]) and the pipeline's source
/// fold ([`fold_source_closure`](crate::pipeline::fold_source_closure)) both
/// reach a document's `#source` edges through here, so the graph they walk is
/// identical — a glob that expanded one way for the guard and another for the
/// fold would build a guard over the wrong graph, and such divergence is silent.
///
/// `Directives::parse` collects directives top-to-bottom by source line, so the
/// iteration order is the author's declaration order; a glob expands *in place*
/// — its members sit where the directive sits, not shuffled to the end — and a
/// pattern matching nothing yields no addresses (the empty set is legal).
pub(crate) fn source_addresses(
    text: &str,
    source: &impl SectionSource,
) -> Result<Vec<SpecAddress>, UseGraphError> {
    let mut out = Vec::new();
    for d in Directives::parse(text).directives {
        if d.kind != DirectiveKind::Source {
            continue;
        }
        // A refusal to expand reads as "this edge's target set cannot be
        // resolved" — the same `Unresolved` a `#source` whose text won't load
        // raises below in `visit`. The pipeline normalises both to its
        // `CompileError::Unresolved` "cannot load" contract.
        let expanded =
            source
                .expand_pattern(&d.address)
                .map_err(|reason| UseGraphError::Unresolved {
                    addr: d.address.to_string(),
                    reason,
                })?;
        out.extend(expanded);
    }
    Ok(out)
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
        SpecAddress::parse("spec://org.vibevm.core/vibevm/a#r").unwrap()
    }

    #[test]
    fn linear_cascade_orders_dependencies_first() {
        let src = MockSource::new(&[
            (
                "spec://org.vibevm.core/vibevm/a#r",
                "#use spec://org.vibevm.core/vibevm/b#r",
            ),
            (
                "spec://org.vibevm.core/vibevm/b#r",
                "#use spec://org.vibevm.core/vibevm/c#r",
            ),
            ("spec://org.vibevm.core/vibevm/c#r", "leaf"),
        ]);
        let order = topo_order_from(&seed(), &src).unwrap();
        assert_eq!(
            order,
            vec![
                "spec://org.vibevm.core/vibevm/c#r".to_string(),
                "spec://org.vibevm.core/vibevm/b#r".to_string(),
                "spec://org.vibevm.core/vibevm/a#r".to_string(),
            ]
        );
    }

    #[test]
    fn diamond_deduplicates_the_shared_dependency() {
        let src = MockSource::new(&[
            (
                "spec://org.vibevm.core/vibevm/a#r",
                "#use spec://org.vibevm.core/vibevm/b#r\n#use spec://org.vibevm.core/vibevm/c#r",
            ),
            (
                "spec://org.vibevm.core/vibevm/b#r",
                "#use spec://org.vibevm.core/vibevm/d#r",
            ),
            (
                "spec://org.vibevm.core/vibevm/c#r",
                "#use spec://org.vibevm.core/vibevm/d#r",
            ),
            ("spec://org.vibevm.core/vibevm/d#r", "shared leaf"),
        ]);
        let order = topo_order_from(&seed(), &src).unwrap();
        assert_eq!(order.len(), 4, "d appears once: {order:?}");
        assert_eq!(order.first().unwrap(), "spec://org.vibevm.core/vibevm/d#r");
        assert_eq!(order.last().unwrap(), "spec://org.vibevm.core/vibevm/a#r");
    }

    #[test]
    fn in_place_use_is_the_agents_edge_not_the_compilers() {
        // §7.4 as of 2026-08-24: `@spec` is the AGENT's mandatory read; to
        // the AOT compiler it is not a splice edge (the realised
        // ##OPEN-CLOSURE-EXPLOSION: address pointers multiplied the host
        // lane tenfold). The address stays in the text — nothing splices.
        let src = MockSource::new(&[
            (
                "spec://org.vibevm.core/vibevm/a#r",
                "prose @spec://org.vibevm.core/vibevm/b#r here",
            ),
            ("spec://org.vibevm.core/vibevm/b#r", "leaf"),
        ]);
        let order = topo_order_from(&seed(), &src).unwrap();
        assert_eq!(order, vec!["spec://org.vibevm.core/vibevm/a#r".to_string()]);
    }

    #[test]
    fn a_cycle_is_reported_with_its_path() {
        let src = MockSource::new(&[
            (
                "spec://org.vibevm.core/vibevm/a#r",
                "#use spec://org.vibevm.core/vibevm/b#r",
            ),
            (
                "spec://org.vibevm.core/vibevm/b#r",
                "#use spec://org.vibevm.core/vibevm/a#r",
            ),
        ]);
        let err = topo_order_from(&seed(), &src).unwrap_err();
        match err {
            UseGraphError::Cycle(path) => {
                assert_eq!(path.first().unwrap(), "spec://org.vibevm.core/vibevm/a#r");
                assert_eq!(path.last().unwrap(), "spec://org.vibevm.core/vibevm/a#r");
                assert!(path.contains(&"spec://org.vibevm.core/vibevm/b#r".to_string()));
            }
            other => panic!("expected a cycle, got {other:?}"),
        }
    }

    #[test]
    fn an_unresolved_use_is_reported() {
        let src = MockSource::new(&[(
            "spec://org.vibevm.core/vibevm/a#r",
            "#use spec://org.vibevm.core/vibevm/missing#r",
        )]);
        let err = topo_order_from(&seed(), &src).unwrap_err();
        assert!(matches!(err, UseGraphError::Unresolved { .. }));
    }

    #[test]
    fn a_leaf_seed_orders_just_itself() {
        let src = MockSource::new(&[("spec://org.vibevm.core/vibevm/a#r", "no uses here")]);
        let order = topo_order_from(&seed(), &src).unwrap();
        assert_eq!(order, vec!["spec://org.vibevm.core/vibevm/a#r".to_string()]);
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

    // --- `#source` fold (B056-L3A): the same traverser, the fold edge set. ---

    /// Recursion along `#source`: a source that itself declares `#source` must
    /// fold before its parent, so the deepest source comes first and the seed
    /// last — `[c, b, a]`.
    #[test]
    fn source_fold_recurses_to_the_deepest_source_first() {
        let src = MockSource::new(&[
            (
                "spec://org.vibevm.core/vibevm/a#r",
                "#source spec://org.vibevm.core/vibevm/b#r",
            ),
            (
                "spec://org.vibevm.core/vibevm/b#r",
                "#source spec://org.vibevm.core/vibevm/c#r",
            ),
            ("spec://org.vibevm.core/vibevm/c#r", "deepest source"),
        ]);
        let order = source_fold_order(&seed(), &src).unwrap();
        assert_eq!(
            order,
            vec![
                "spec://org.vibevm.core/vibevm/c#r".to_string(),
                "spec://org.vibevm.core/vibevm/b#r".to_string(),
                "spec://org.vibevm.core/vibevm/a#r".to_string(),
            ]
        );
    }

    /// A diamond over `#source` folds the shared source exactly once.
    #[test]
    fn source_fold_deduplicates_a_diamond() {
        let src = MockSource::new(&[
            (
                "spec://org.vibevm.core/vibevm/a#r",
                "#source spec://org.vibevm.core/vibevm/b#r\n#source spec://org.vibevm.core/vibevm/c#r",
            ),
            (
                "spec://org.vibevm.core/vibevm/b#r",
                "#source spec://org.vibevm.core/vibevm/d#r",
            ),
            (
                "spec://org.vibevm.core/vibevm/c#r",
                "#source spec://org.vibevm.core/vibevm/d#r",
            ),
            ("spec://org.vibevm.core/vibevm/d#r", "shared source"),
        ]);
        let order = source_fold_order(&seed(), &src).unwrap();
        assert_eq!(order.len(), 4, "d appears once: {order:?}");
        assert_eq!(order.first().unwrap(), "spec://org.vibevm.core/vibevm/d#r");
        assert_eq!(order.last().unwrap(), "spec://org.vibevm.core/vibevm/a#r");
    }

    /// Two sibling `#source` directives keep their declaration order in the
    /// fold — the line-sort, not an arbitrary one.
    #[test]
    fn source_fold_preserves_declaration_order() {
        let src = MockSource::new(&[
            (
                "spec://org.vibevm.core/vibevm/a#r",
                "#source spec://org.vibevm.core/vibevm/b#r\n#source spec://org.vibevm.core/vibevm/c#r",
            ),
            ("spec://org.vibevm.core/vibevm/b#r", "first sibling"),
            ("spec://org.vibevm.core/vibevm/c#r", "second sibling"),
        ]);
        let order = source_fold_order(&seed(), &src).unwrap();
        let b = order
            .iter()
            .position(|k| *k == "spec://org.vibevm.core/vibevm/b#r")
            .unwrap();
        let c = order
            .iter()
            .position(|k| *k == "spec://org.vibevm.core/vibevm/c#r")
            .unwrap();
        assert!(b < c, "declaration order lost: {order:?}");
        assert_eq!(order.last().unwrap(), "spec://org.vibevm.core/vibevm/a#r");
    }

    /// A `#source` cycle between contracts is a legal forward declaration — no
    /// error, both folded.
    #[test]
    fn a_source_cycle_between_contracts_is_admitted() {
        let src = MockSource::new(&[
            (
                "spec://org.vibevm.demo/lib/contract/a#r",
                "#source spec://org.vibevm.demo/lib/contract/b#r",
            ),
            (
                "spec://org.vibevm.demo/lib/contract/b#r",
                "#source spec://org.vibevm.demo/lib/contract/a#r",
            ),
        ]);
        let seed = SpecAddress::parse("spec://org.vibevm.demo/lib/contract/a#r").unwrap();
        let order = source_fold_order(&seed, &src).unwrap();
        assert_eq!(order.len(), 2, "both contracts present: {order:?}");
    }

    /// A `#source` cycle that runs through an implementation (non-contract)
    /// node is a hard error, reported with the offending path.
    #[test]
    fn a_source_cycle_touching_an_impl_is_rejected() {
        let src = MockSource::new(&[
            (
                "spec://org.vibevm.demo/lib/contract/a#r",
                "#source spec://org.vibevm.demo/lib/source/b#r",
            ),
            (
                "spec://org.vibevm.demo/lib/source/b#r",
                "#source spec://org.vibevm.demo/lib/contract/a#r",
            ),
        ]);
        let seed = SpecAddress::parse("spec://org.vibevm.demo/lib/contract/a#r").unwrap();
        match source_fold_order(&seed, &src) {
            Err(UseGraphError::Cycle(path)) => {
                assert!(path.contains(&"spec://org.vibevm.demo/lib/contract/a#r".to_string()));
                assert!(path.contains(&"spec://org.vibevm.demo/lib/source/b#r".to_string()));
            }
            other => panic!("expected a cycle, got {other:?}"),
        }
    }

    /// A document carrying BOTH `#use x` and `#source y`: the source fold
    /// reaches `y` only, the use order reaches `x` only — one traverser, two
    /// disjoint edge sets.
    #[test]
    fn source_edges_and_use_edges_do_not_mix() {
        let src = MockSource::new(&[
            (
                "spec://org.vibevm.core/vibevm/a#r",
                "#use spec://org.vibevm.core/vibevm/x#r\n#source spec://org.vibevm.core/vibevm/y#r",
            ),
            ("spec://org.vibevm.core/vibevm/x#r", "use target"),
            ("spec://org.vibevm.core/vibevm/y#r", "source target"),
        ]);
        let fold = source_fold_order(&seed(), &src).unwrap();
        let uses = topo_order_from(&seed(), &src).unwrap();
        assert_eq!(
            fold,
            vec![
                "spec://org.vibevm.core/vibevm/y#r".to_string(),
                "spec://org.vibevm.core/vibevm/a#r".to_string(),
            ]
        );
        assert_eq!(
            uses,
            vec![
                "spec://org.vibevm.core/vibevm/x#r".to_string(),
                "spec://org.vibevm.core/vibevm/a#r".to_string(),
            ]
        );
    }

    /// A `#source` that names an unknown address is unresolved.
    #[test]
    fn an_unresolved_source_is_reported() {
        let src = MockSource::new(&[(
            "spec://org.vibevm.core/vibevm/a#r",
            "#source spec://org.vibevm.core/vibevm/missing#r",
        )]);
        let err = source_fold_order(&seed(), &src).unwrap_err();
        assert!(matches!(err, UseGraphError::Unresolved { .. }));
    }

    /// `@spec` is a *use*, not a source (РТ-4): the fold follows the `#source`
    /// edge to `y` and never follows the `@spec` to `b`, even when both sit in
    /// the same document.
    #[test]
    fn an_at_spec_in_place_use_is_not_a_source_edge() {
        let src = MockSource::new(&[
            (
                "spec://org.vibevm.core/vibevm/a#r",
                "prose @spec://org.vibevm.core/vibevm/b#r here\n#source spec://org.vibevm.core/vibevm/y#r",
            ),
            (
                "spec://org.vibevm.core/vibevm/b#r",
                "use target, not a source",
            ),
            ("spec://org.vibevm.core/vibevm/y#r", "source target"),
        ]);
        let order = source_fold_order(&seed(), &src).unwrap();
        assert_eq!(
            order,
            vec![
                "spec://org.vibevm.core/vibevm/y#r".to_string(),
                "spec://org.vibevm.core/vibevm/a#r".to_string(),
            ]
        );
    }
}
