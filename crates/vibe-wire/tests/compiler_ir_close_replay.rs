//! The synthetic cases of the BUILTIN CLOSE ORDER oracle
//! (`x-corpus-producer-oracles`) — graphs no corpus golden carries, so the
//! replay is exercised on shapes the real closures cannot show. Split from
//! `compiler_ir_producer_laws.rs` only to keep both files inside the 600-line
//! cap; the machinery is shared, in `compiler_ir_close_oracle/mod.rs`.
//!
//! Still an oracle, never a conversion gate: it says what the BUILTIN `close`
//! would have emitted for these graphs, and the R6.3 decoder must accept a
//! verifier-valid carrier it would reject.

mod compiler_ir_close_oracle;

use compiler_ir_close_oracle::{Edge, Node, Use, close_faults, without_pin};
use vibe_wire::generated::compiler_ir::e1::ir::SpecAddress;

/// A host-authority address under `<genre>/<stem>` — `contract/…` when the
/// case needs `topology.rs::is_contract` to say yes.
fn address(genre: &str, stem: &str) -> SpecAddress {
    serde_json::from_value(serde_json::json!({
        "raw": format!("spec://demo/{genre}/{stem}"),
        "authority": {"kind": "host", "name": "demo"},
        "doc_path": format!("{genre}/{stem}"),
        "anchor": [],
    }))
    .unwrap()
}

/// One node with its own ordered `#use` declarations, lines minted in
/// declaration order so `use_addresses`'s stable sort keeps them.
fn node(genre: &str, stem: &str, uses: &[SpecAddress]) -> Node {
    let own = address(genre, stem);
    Node {
        key: without_pin(&own),
        contract: genre == "contract",
        uses: uses
            .iter()
            .enumerate()
            .map(|(index, target)| Use {
                line: index as u32,
                target: target.clone(),
            })
            .collect(),
    }
}

fn edge(from: u32, to: u32, target: &SpecAddress) -> Edge {
    Edge {
        from,
        to,
        target: target.clone(),
    }
}

#[test]
fn an_admitted_contract_cycle_is_builtin_output() {
    // `contract/a` uses `contract/b` and back. `topology.rs:41-49` admits the
    // Gray revisit because the target and every node on the loop suffix is a
    // contract, so `close` really returns `[b, a]` for seed `a` — carrying the
    // edge `b -> a` as `0 -> 1`, which no index comparison could allow.
    let (a, b) = (address("contract", "a"), address("contract", "b"));
    let nodes = vec![
        node("contract", "b", std::slice::from_ref(&a)),
        node("contract", "a", std::slice::from_ref(&b)),
    ];
    let edges = vec![edge(0, 1, &a), edge(1, 0, &b)];
    assert_eq!(
        close_faults(&nodes, &edges, &[(1, vec![0, 1])]),
        Vec::<String>::new(),
        "an admitted contract cycle is legal builtin output"
    );

    // The same shape through a non-contract node is `UseGraphError::Cycle` —
    // no close output exists for it at all.
    let (x, y) = (address("boot", "x"), address("boot", "y"));
    let plain = vec![
        node("boot", "y", std::slice::from_ref(&x)),
        node("boot", "x", std::slice::from_ref(&y)),
    ];
    assert!(
        close_faults(
            &plain,
            &[edge(0, 1, &x), edge(1, 0, &y)],
            &[(1, vec![0, 1])]
        )
        .iter()
        .any(|entry| entry.contains("non-contract")),
        "a cycle touching a non-contract node must be red"
    );
}

/// Two seeds reaching the independent shared nodes 0 and 1, each with its OWN
/// declaration order — seed 2 declares `[0, 1]`, seed 3 declares `[1, 0]`.
fn multi_root() -> (Vec<Node>, Vec<Edge>) {
    let (zero, one) = (address("boot", "zero"), address("boot", "one"));
    let nodes = vec![
        node("boot", "zero", &[]),
        node("boot", "one", &[]),
        node("boot", "two", &[zero.clone(), one.clone()]),
        node("boot", "three", &[one.clone(), zero.clone()]),
    ];
    let edges = vec![
        edge(2, 0, &zero),
        edge(2, 1, &one),
        edge(3, 1, &one),
        edge(3, 0, &zero),
    ];
    (nodes, edges)
}

#[test]
fn a_shared_multi_root_order_is_builtin_output() {
    let (nodes, edges) = multi_root();
    assert_eq!(
        close_faults(&nodes, &edges, &[(2, vec![0, 1, 2]), (3, vec![1, 0, 3])]),
        Vec::<String>::new(),
        "each root walks the shared ids in its own declaration order"
    );
    // Declaration order is load-bearing: seed 3 handed seed 2's order is a
    // valid topological sort and still not what the builtin would write.
    assert!(
        close_faults(&nodes, &edges, &[(3, vec![0, 1, 3])])
            .iter()
            .any(|entry| entry.contains("differs from the declaration-order DFS")),
        "a topological permutation is not the builtin's own output"
    );
}

#[test]
fn illegal_close_orders_are_red() {
    let (nodes, edges) = multi_root();
    let case = |orders: &[(u32, Vec<u32>)], needle: &str| {
        let faults = close_faults(&nodes, &edges, orders);
        assert!(
            faults.iter().any(|entry| entry.contains(needle)),
            "expected {needle}, got {faults:?}"
        );
    };
    case(
        &[(3, vec![3, 1, 0])],
        "differs from the declaration-order DFS",
    );
    case(&[(2, vec![0, 0, 2])], "repeats a key");
    case(&[(2, vec![])], "is empty");
    case(&[(2, vec![0, 9, 2])], "out of arena bounds");
    case(&[(9, vec![0, 1, 9])], "seed 9 is out of arena bounds");
}

#[test]
fn an_edge_vector_that_the_directives_do_not_mint_is_red() {
    let (nodes, edges) = multi_root();
    let orders = [(2u32, vec![0u32, 1, 2]), (3, vec![1, 0, 3])];
    // Reordering the vector is red even though every tuple is individually
    // right: `close` appends per reached node, in traversal order.
    let mut shuffled = edges.clone();
    shuffled.swap(0, 1);
    assert!(
        close_faults(&nodes, &shuffled, &orders)
            .iter()
            .any(|entry| entry.contains("the carried directives mint")),
        "the edge vector order is part of the witness"
    );
    // And an edge nobody declared is red.
    let mut extra = edges.clone();
    extra.push(edge(0, 1, &address("boot", "one")));
    assert!(
        close_faults(&nodes, &extra, &orders)
            .iter()
            .any(|entry| entry.contains("the carried directives mint")),
        "an undeclared edge must be red"
    );
}

#[test]
fn a_repeated_declaration_is_minted_once() {
    // `close.rs:187` suppresses a byte-identical repeat (`edges.contains`),
    // and the DFS colour table already deduplicates the reach — so a document
    // declaring the same target twice yields ONE edge and one occurrence.
    let zero = address("boot", "zero");
    let nodes = vec![
        node("boot", "zero", &[]),
        node("boot", "one", &[zero.clone(), zero.clone()]),
    ];
    assert_eq!(
        close_faults(&nodes, &[edge(1, 0, &zero)], &[(1, vec![0, 1])]),
        Vec::<String>::new(),
        "a repeated identical `#use` mints one edge"
    );
    // A second target differing only in its pin is a DIFFERENT request, so
    // domain `ClosureEdge` equality keeps both.
    let pinned: SpecAddress = serde_json::from_value(serde_json::json!({
        "raw": "spec://demo/boot/zero~r2",
        "authority": {"kind": "host", "name": "demo"},
        "doc_path": "boot/zero",
        "anchor": [],
        "pinned_r": 2,
    }))
    .unwrap();
    let pinned_nodes = vec![
        node("boot", "zero", &[]),
        node("boot", "one", &[zero.clone(), pinned.clone()]),
    ];
    assert_eq!(
        close_faults(
            &pinned_nodes,
            &[edge(1, 0, &zero), edge(1, 0, &pinned)],
            &[(1, vec![0, 1])],
        ),
        Vec::<String>::new(),
        "a pin-distinct request stays its own edge"
    );
}
