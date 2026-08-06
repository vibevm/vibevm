//! The query-language map search (E-A5B-QUERYLANG): a conjunctive predicate
//! grammar with **undirected graph traversal**, sitting ON TOP of the
//! first-level [`search`](crate::search). The floor is permanent — it has its
//! own library entry, and a broken parser here cannot reach it (the floor
//! carries no grammar by design).
//!
//! The grammar (version 1) is a whitespace-separated conjunction of
//! predicates — no `OR`, no parentheses, no precedence:
//!
//! ```text
//! uri:<exact spec:// address>   symbol:<code-symbol substring>
//! kind:<item_kind or spec kind> scope:<spec:// uri prefix>
//! has:<verb>   lacks:<verb>      depth:<0..3>
//! ```
//!
//! `uri`/`symbol`/`kind` are the floor's three filters, reused verbatim for
//! seed selection (УТОЧНИ-1); `scope` is a uri prefix (spec units only);
//! `has:<verb>`/`lacks:<verb>` keep only seeds that an edge of that verb
//! touches / does not touch; `depth:N` walks the bipartite code↔spec graph
//! undirected for N steps (seeds stay, at depth 0). The ceiling is the floor's
//! hard cap, applied AFTER the walk, and `total_matching` reports every node
//! the walk reached. Each hit carries the step it was reached at (`d0` for a
//! seed).
//!
//! This file is the engine half: the walk ([`select`]), the fresh-build entry
//! ([`query`]), and the renderers ([`render`]). The grammar half — parser,
//! [`ParsedQuery`], and [`ParseError`] — lives in [`parse`] (split along the
//! "parse the string" / "walk the graph" seam to keep each half under the
//! per-file budget). The CLI (`vibe select --where "…"`) and MCP (`select`)
//! are thin surfaces over [`parse`] → [`query`] → [`render`]; they parse the
//! string and print — no copy of the grammar, traversal, or rendering lives in
//! either surface.

specmark::scope!("spec://core-ai-native/mechanisms/PROP-014#queries");

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use serde::Serialize;
use serde_json::{Value, json};
use specmap_core::generated::specmap::{Edge, Specmap};

use crate::search::{
    Filters, Hit, HitSource, MAX_LIMIT, hit_from_code, hit_from_spec, matches_filters,
};

mod parse;
pub use parse::{ParseError, ParsedQuery, Verb, parse};
#[cfg(test)]
mod tests;

/// The grammar version this module speaks. Surfaced in every JSON answer as
/// `grammar` so a consumer can branch on the language shape; the version is
/// NOT named inside the query string (§2.2 point 11).
pub const GRAMMAR_VERSION: u32 = 1;

/// One `select` result: a node hit plus the step it was reached at (УТОЧНИ-2).
/// Wraps the floor's [`Hit`] unchanged — `#[serde(flatten)]` makes the JSON
/// entry the hit's fields plus a `depth`, so the public `Hit` type is untouched
/// and there is no second node renderer, only this layer's own answer renderer.
#[derive(Debug, Clone, Serialize)]
pub struct SelectHit {
    /// The node — same shape the floor returns.
    #[serde(flatten)]
    pub hit: Hit,
    /// Steps from a seed: 0 for a seed, 1..=depth for a reached node.
    pub depth: u32,
}

/// The walk outcome: the (capped) hits plus the facts a surface needs to report
/// truncation honestly. `total_matching` counts every node the walk reached —
/// before the ceiling — so "showing N of M" stays true (§2.2 point 7).
#[derive(Debug, Clone)]
pub struct SelectOut {
    /// Capped, ordered hits: depth ascending, then spec units (index order),
    /// then code items (index order).
    pub hits: Vec<SelectHit>,
    /// Every node the walk reached, before the ceiling — `>= hits.len()`.
    pub total_matching: usize,
    /// The effective ceiling after clamping to `[1, MAX_LIMIT]`.
    pub limit: usize,
}

impl SelectOut {
    /// Whether the ceiling dropped reached nodes the caller never saw.
    pub fn is_truncated(&self) -> bool {
        self.total_matching > self.hits.len()
    }
}

/// One rendered `select` answer: the deterministic text view, or the structured
/// JSON. Mirrors [`SearchView`](crate::search::SearchView) so the capabilities
/// stay parallel.
#[derive(Debug)]
pub enum SelectView {
    /// The human-readable, scannable view.
    Text(String),
    /// The structured view for an agent or script.
    Json(Value),
}

/// A reached node: which family, and its index into `specUnits` / `codeItems`.
/// Internal — the public answer carries a [`Hit`], not this handle.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct NodeKey {
    spec: bool,
    idx: usize,
}

/// Run the parsed query against an already-built `map`: select the seeds
/// (the floor's `uri`/`symbol`/`kind` reused, plus `scope`/`has`/`lacks`),
/// walk the bipartite code↔spec graph undirected for `depth` steps, then cap
/// and order. Pure — same map + parsed query ⇒ same result, same order (depth
/// ascending; spec units before code items, each in index order). The ceiling
/// is the floor's hard cap, applied AFTER the walk, so `total_matching` is the
/// full reach. Callers wanting a tree built fresh use [`query`].
///
/// `select` is pure, so a doctest needs no tree on disk — hand-build a map:
///
/// ```
/// use specmap_core::generated::specmap::Specmap;
/// use vibe_trace::select::{parse, select};
///
/// let map: Specmap = serde_json::from_str(
///     r#"{"schema":3,
///        "spec_units":[
///          {"anchor":"req-r","content_hash":"h1","doc_path":"D","file":"spec/D.md","heading":"The rule","line":1,"uri":"spec://demo/D#req-r","kind":"req"},
///          {"anchor":"guide-g","content_hash":"h2","doc_path":"D","file":"spec/D.md","heading":"The guide","line":9,"uri":"spec://demo/D#guide-g","kind":"guide"}],
///        "code_items":[
///          {"crate_name":"x","file":"x/src/lib.rs","item_kind":"fn","line":1,"symbol":"x::f"},
///          {"crate_name":"x","file":"x/src/lib.rs","item_kind":"fn","line":3,"symbol":"x::t"}],
///        "edges":[
///          {"file":"x/src/lib.rs","from_symbol":"x::f","line":1,"provenance":"authored","uri":"spec://demo/D#req-r","verb":"implements"},
///          {"file":"x/src/lib.rs","from_symbol":"x::t","line":3,"provenance":"authored","uri":"spec://demo/D#req-r","verb":"verifies"}],
///        "suspects":[],"warnings":[]}"#,
/// ).unwrap();
/// // `kind:fn` seeds both code items; no depth ⇒ two hits, both at d0.
/// let out = select(&map, &parse("kind:fn").unwrap(), 50);
/// assert_eq!(out.hits.len(), 2);
/// assert!(out.hits.iter().all(|h| h.depth == 0));
/// // One step OUT of the req unit reaches the implementing + verifying code.
/// let out = select(&map, &parse("uri:spec://demo/D#req-r depth:1").unwrap(), 50);
/// assert_eq!(out.total_matching, 3); // 1 spec seed (d0) + 2 code (d1)
/// assert_eq!(out.hits[0].depth, 0);
/// ```
pub fn select(map: &Specmap, parsed: &ParsedQuery, limit: usize) -> SelectOut {
    let limit = limit.clamp(1, MAX_LIMIT);

    // Project every node once — parallel to the unit/item vectors — so seed
    // selection and the output share ONE projection (УТОЧНИ-1), not two.
    let spec_hits: Vec<Hit> = map.specUnits.iter().map(hit_from_spec).collect();
    let code_hits: Vec<Hit> = map.codeItems.iter().map(hit_from_code).collect();

    // Edge adjacency + verb touch, keyed by identity string. Both views of an
    // edge come from one `&Edge`: the verbs it touches AND the node it leads
    // to. `edges_by_uri` serves spec nodes (incoming edges); `edges_by_symbol`
    // serves code nodes (outgoing edges). Undirected: an edge `(symbol, uri)`
    // appears in both, so a spec reaches its code and vice-versa (ПРОВЕРЬ-6).
    let mut edges_by_uri: HashMap<&str, Vec<&Edge>> = HashMap::new();
    let mut edges_by_symbol: HashMap<&str, Vec<&Edge>> = HashMap::new();
    for e in &map.edges {
        edges_by_uri.entry(&e.uri).or_default().push(e);
        edges_by_symbol.entry(&e.fromSymbol).or_default().push(e);
    }
    let mut code_by_symbol: HashMap<&str, Vec<usize>> = HashMap::new();
    for (j, c) in map.codeItems.iter().enumerate() {
        code_by_symbol.entry(&c.symbol).or_default().push(j);
    }
    let mut spec_by_uri: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, u) in map.specUnits.iter().enumerate() {
        spec_by_uri.entry(&u.uri).or_default().push(i);
    }

    // The floor's three filters — built once, reused verbatim by matches_filters.
    let base = Filters {
        uri: parsed.uri.clone(),
        symbol: parsed.symbol.clone(),
        kind: parsed.kind.clone(),
        limit: MAX_LIMIT, // unused by matches_filters; seeds are never capped.
    };

    // reached-node → fewest steps. Seeds are inserted at 0 below.
    let mut visited: HashMap<NodeKey, u32> = HashMap::new();

    // --- seed selection: base filters AND scope AND has AND lacks ----------
    for (i, u) in map.specUnits.iter().enumerate() {
        if !matches_filters(&spec_hits[i], &base) {
            continue;
        }
        if let Some(scope) = &parsed.scope
            && !u.uri.starts_with(scope.as_str())
        {
            continue;
        }
        let touch = edges_by_uri.get(u.uri.as_str());
        if let Some(v) = parsed.has
            && !verb_touches(touch, v)
        {
            continue;
        }
        if let Some(v) = parsed.lacks
            && verb_touches(touch, v)
        {
            continue;
        }
        visited.insert(NodeKey { spec: true, idx: i }, 0);
    }
    for (j, c) in map.codeItems.iter().enumerate() {
        if !matches_filters(&code_hits[j], &base) {
            continue;
        }
        if parsed.scope.is_some() {
            continue; // a code item carries no uri, so it fails any scope.
        }
        let touch = edges_by_symbol.get(c.symbol.as_str());
        if let Some(v) = parsed.has
            && !verb_touches(touch, v)
        {
            continue;
        }
        if let Some(v) = parsed.lacks
            && verb_touches(touch, v)
        {
            continue;
        }
        visited.insert(
            NodeKey {
                spec: false,
                idx: j,
            },
            0,
        );
    }

    // --- undirected BFS to parsed.depth (seeds already at 0) ---------------
    let mut frontier: Vec<NodeKey> = visited.keys().copied().collect();
    for step in 1..=parsed.depth {
        let mut next: Vec<NodeKey> = Vec::new();
        for &key in &frontier {
            if key.spec {
                // spec → code: follow edges INTO this uri to their fromSymbols.
                let uri = map.specUnits[key.idx].uri.as_str();
                if let Some(edges) = edges_by_uri.get(uri) {
                    for e in edges {
                        if let Some(js) = code_by_symbol.get(e.fromSymbol.as_str()) {
                            for &j in js {
                                reach(
                                    NodeKey {
                                        spec: false,
                                        idx: j,
                                    },
                                    step,
                                    &mut visited,
                                    &mut next,
                                );
                            }
                        }
                    }
                }
            } else {
                // code → spec: follow edges OUT of this symbol to their uris.
                let sym = map.codeItems[key.idx].symbol.as_str();
                if let Some(edges) = edges_by_symbol.get(sym) {
                    for e in edges {
                        if let Some(is) = spec_by_uri.get(e.uri.as_str()) {
                            for &i in is {
                                reach(
                                    NodeKey { spec: true, idx: i },
                                    step,
                                    &mut visited,
                                    &mut next,
                                );
                            }
                        }
                    }
                }
            }
        }
        if next.is_empty() {
            break; // no new nodes this layer ⇒ nothing further can be reached.
        }
        frontier = next;
    }

    // --- order (depth, spec-before-code, index) then cap AFTER the walk ----
    let mut entries: Vec<(NodeKey, u32)> = visited.into_iter().collect();
    entries.sort_by_key(|(k, d)| (*d, !k.spec as u8, k.idx));
    let total_matching = entries.len();
    let hits = entries
        .into_iter()
        .take(limit)
        .map(|(k, d)| SelectHit {
            hit: if k.spec {
                spec_hits[k.idx].clone()
            } else {
                code_hits[k.idx].clone()
            },
            depth: d,
        })
        .collect();

    SelectOut {
        hits,
        total_matching,
        limit,
    }
}

/// Whether any edge in `touch` carries `verb` — the seed predicate behind
/// `has:` (true ⇒ keep) and `lacks:` (true ⇒ drop). For a spec unit `touch` is
/// its incoming edges (`edges_by_uri`); for a code item its outgoing edges
/// (`edges_by_symbol`) — one reading for both families (§2.2 point 3).
fn verb_touches(touch: Option<&Vec<&Edge>>, verb: Verb) -> bool {
    touch.is_some_and(|es| es.iter().any(|e| Verb::from_edge(&e.verb) == verb))
}

/// Record an unreached neighbour at `step`; ignore it if already reached at a
/// shallower step (first arrival wins — BFS gives the minimum depth).
fn reach(key: NodeKey, step: u32, visited: &mut HashMap<NodeKey, u32>, next: &mut Vec<NodeKey>) {
    if visited.contains_key(&key) {
        return;
    }
    visited.insert(key, step);
    next.push(key);
}

/// Build the traceability index **fresh** for `root` and run [`select`] over
/// it — the same posture [`search::query`](crate::search::query) takes. The
/// shared entry both surfaces call; the CLI/MCP add no build logic of their own.
pub fn query(root: &Path, parsed: &ParsedQuery, limit: usize) -> Result<SelectOut> {
    let cfg = specmap_core::config::Config::load(root)?.unwrap_or_default();
    let map = specmap_core::index::build(root, &cfg);
    Ok(select(&map, parsed, limit))
}

/// Render `out` for a surface. `json` selects the structured view an agent
/// consumes; the default is the deterministic, scannable text. `parsed` shapes
/// the text header and echoes back in the JSON envelope alongside `grammar` and
/// the raw `query`, so a consumer sees what produced the answer.
pub fn render(out: &SelectOut, parsed: &ParsedQuery, query: &str, json: bool) -> SelectView {
    if json {
        SelectView::Json(render_json(out, parsed, query))
    } else {
        SelectView::Text(render_text(out, parsed))
    }
}

// --- rendering ------------------------------------------------------------

/// The human-readable view: a header (grammar + parsed predicates + count or
/// the truncation note) then one scannable line per hit, each prefixed with
/// its step (`d0`, `d1`, …).
fn render_text(out: &SelectOut, parsed: &ParsedQuery) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "map select · grammar v{} · {}\n",
        GRAMMAR_VERSION,
        parsed.header()
    ));
    if out.is_truncated() {
        s.push_str(&format!(
            "showing {} of {} matching — narrow the query or raise --limit (max {})\n",
            out.hits.len(),
            out.total_matching,
            MAX_LIMIT
        ));
    } else {
        let n = out.hits.len();
        s.push_str(&format!("{} result{}\n", n, if n == 1 { "" } else { "s" }));
    }
    s.push('\n');
    for h in &out.hits {
        let src = match h.hit.source {
            HitSource::Spec => "spec",
            HitSource::Code => "code",
        };
        let kind = h.hit.kind.as_deref().unwrap_or("-");
        let mut line = format!(
            "  d{} {:<4} {:<10} {}  {}:{}",
            h.depth, src, kind, h.hit.name, h.hit.file, h.hit.line
        );
        if h.hit.source == HitSource::Spec
            && let Some(uri) = &h.hit.uri
        {
            line.push_str("  ");
            line.push_str(uri);
        }
        s.push_str(line.trim_end());
        s.push('\n');
    }
    s
}

/// The structured view: `grammar`, the raw `query`, the `parsed` predicates,
/// the capped `results` (each a flattened hit + `depth`), and the counts a
/// script branches on (§2.3, §2.2 point 7/10).
fn render_json(out: &SelectOut, parsed: &ParsedQuery, query: &str) -> Value {
    let results = serde_json::to_value(&out.hits).unwrap_or(Value::Null);
    json!({
        "grammar": GRAMMAR_VERSION,
        "query": query,
        "parsed": serde_json::to_value(parsed).unwrap_or(Value::Null),
        "results": results,
        "count": out.hits.len(),
        "total_matching": out.total_matching,
        "limit": out.limit,
        "truncated": out.is_truncated(),
    })
}
