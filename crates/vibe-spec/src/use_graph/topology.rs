//! The one deterministic topology/cycle walker shared by graph consumers.

use std::collections::HashMap;

use crate::SpecAddress;

use super::UseGraphError;
use super::cycle::first_illegal_cycle;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Color {
    White,
    Gray,
    Black,
}

/// The relation discovered from one seed: keys in discovery order, the edges
/// each node declared, and the walk state that keeps discovery linear.
#[derive(Default)]
struct Discovered {
    keys: Vec<String>,
    ids: HashMap<String, usize>,
    adjacency: Vec<Vec<usize>>,
    color: Vec<Color>,
}

impl Discovered {
    fn ensure(&mut self, key: &str) -> usize {
        if let Some(id) = self.ids.get(key) {
            return *id;
        }
        let id = self.keys.len();
        self.keys.push(key.to_string());
        self.ids.insert(key.to_string(), id);
        self.adjacency.push(Vec::new());
        self.color.push(Color::White);
        id
    }
}

/// Order the graph exposed by `edges`, using pinless address identity.
///
/// The callback owns loading/parsing attribution. This cell owns all graph
/// semantics: declaration-order DFS, dependency-before-dependent output, reach
/// deduplication, exact cycle paths, and contract-only cycle admission.
///
/// Discovery and judgement are two phases. The walk records the relation and
/// never rules on a back edge it happens to meet; the shared
/// [`first_illegal_cycle`] law then judges whole strongly connected components,
/// so this walker and the inter-pass verifier — rooted differently, reading the
/// same relation — cannot return different verdicts (PROP-035 §9).
pub(crate) fn order_by(
    seed: &SpecAddress,
    mut edges: impl FnMut(&SpecAddress) -> Result<Vec<SpecAddress>, UseGraphError>,
) -> Result<Vec<String>, UseGraphError> {
    let mut graph = Discovered::default();
    let mut order = Vec::new();
    let mut failure = None;
    visit(seed, &mut edges, &mut graph, &mut order, &mut failure);

    let relation: Vec<(usize, usize)> = graph
        .adjacency
        .iter()
        .enumerate()
        .flat_map(|(from, targets)| targets.iter().map(move |to| (from, *to)))
        .collect();
    if let Some(path) =
        first_illegal_cycle(&graph.keys, &relation, |id| is_contract(&graph.keys[id]))
    {
        return Err(UseGraphError::Cycle(
            path.into_iter().map(|id| graph.keys[id].clone()).collect(),
        ));
    }
    match failure {
        Some(failure) => Err(failure),
        None => Ok(order),
    }
}

/// Discovery only: load each node once, record its declared edges in order, and
/// emit the post-order. A revisit — finished or still open — simply returns, so
/// an admitted cycle costs no second load and an illegal one is not ruled on
/// here.
///
/// A load failure is *recorded*, not thrown: the relation must be complete
/// before the component law can judge it, and a cycle is a property of the
/// whole graph while an unresolved leaf is local. The first failure is kept and
/// its node contributes no edges, so a graph that is both cyclic and incomplete
/// still reports the cycle — the precedence the compiler's public contract has
/// always had.
fn visit(
    addr: &SpecAddress,
    edges: &mut impl FnMut(&SpecAddress) -> Result<Vec<SpecAddress>, UseGraphError>,
    graph: &mut Discovered,
    order: &mut Vec<String>,
    failure: &mut Option<UseGraphError>,
) {
    let key = addr.without_pin();
    let id = graph.ensure(&key);
    if graph.color[id] != Color::White {
        return;
    }

    graph.color[id] = Color::Gray;
    let targets = match edges(addr) {
        Ok(targets) => targets,
        Err(error) => {
            graph.color[id] = Color::Black;
            failure.get_or_insert(error);
            return;
        }
    };
    for target in targets {
        let target_id = graph.ensure(&target.without_pin());
        graph.adjacency[id].push(target_id);
        visit(&target, edges, graph, order, failure);
    }
    graph.color[id] = Color::Black;
    order.push(key);
}

/// Whether an address's document path has a `contract` segment — the
/// forward-declaration exception PROP-035 §9 admits for use/source cycles.
/// Shared by this walker and the inter-pass verifier so the two can never
/// drift into subtly different path logic.
pub(crate) fn is_contract_address(address: &SpecAddress) -> bool {
    address
        .doc_path
        .split('/')
        .any(|segment| segment == "contract")
}

fn is_contract(key: &str) -> bool {
    SpecAddress::parse(key).is_ok_and(|address| is_contract_address(&address))
}
