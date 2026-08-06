//! The simple-level map search (A5A-MAPSEARCH): independent filters, joined
//! by AND, over a hard result ceiling. The permanent grep-like floor over the
//! code↔spec map — [`explain`](crate::explain) answers one point; this answers
//! "every node that fits these criteria." A query-language layer will sit *on
//! top* of this later, so it carries no grammar/parser (Р3): a broken query
//! language cannot break the simple search.
//!
//! Three filters, none required, AND-joined (Р2): `uri` (exact spec address),
//! `symbol` (substring of a code symbol), `kind` (a code `item_kind` *or* a
//! spec unit kind — disjoint vocabularies, one filter; spec units answer on
//! their own kind, an unmarked unit matches none). Results are **nodes, not
//! edges** (Р1). The CLI (`vibe query`) and MCP (`query`) are thin surfaces
//! over [`query`] + [`render`]. Each hit carries its source ([`HitSource`]) so
//! a future second data provider can join at query time (Р6) — nothing of that
//! engine is built today, only the door stays open.

specmark::scope!("spec://core-ai-native/mechanisms/PROP-014#queries");

use std::path::Path;

use anyhow::Result;
use serde::Serialize;
use serde_json::{Value, json};
use specmap_core::generated::specmap::{CodeItem, SpecUnit, SpecUnitKind, Specmap};

/// The default result ceiling — applied whenever [`Filters::limit`] is left at
/// its default. The answer is read by an agent with a bounded context window,
/// so the floor returns a survey, never the whole map (УТОЧНИ-2).
pub const DEFAULT_LIMIT: usize = 50;

/// The hard maximum the ceiling may be raised to. The ceiling is part of the
/// design, not a convenience: an answer that does not fit an agent's context
/// is worthless, so `--limit` may be tuned within `[1, MAX]` but never removed
/// — there is no unbounded mode (Р2).
pub const MAX_LIMIT: usize = 200;

/// Where a [`Hit`] was taken from — its provenance (Р6). Two sources feed the
/// map today; a future code-quality engine joins at query time as a new
/// variant here, flowing through the same `Vec<Hit>` and renderers. The enum
/// is the documented extension point for the second data provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HitSource {
    /// A spec unit — an anchored heading span in `spec/**/*.md`.
    Spec,
    /// A code item — a tagged fn / struct / impl / mod / … in source.
    Code,
}

/// One search result: a **node** in the code↔spec map (Р1). Carries its
/// provenance so a future second data provider can join at query time (Р6).
/// `name` is the human label (a spec unit's heading or a code item's symbol);
/// `uri` / `crate_name` are present only for the family that carries them.
#[derive(Debug, Clone, Serialize)]
pub struct Hit {
    /// Provenance — which family of node this is (and, later, which engine).
    pub source: HitSource,
    /// Human label: a spec unit's heading text, or a code item's symbol.
    pub name: String,
    /// The node's kind — a code item's `item_kind` or a spec unit's kind
    /// (`req`/`prop`/…); `None` for a legacy-unmarked unit. Unified so
    /// [`Filters::kind`] is one filter over both families.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Repo-relative, forward-slash path of the source/markdown file.
    pub file: String,
    /// 1-based line of the node in `file`.
    pub line: u32,
    /// The spec unit's canonical `spec://…#anchor` address (`None` for code).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    /// The code item's crate name (`None` for spec units).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crate_name: Option<String>,
}

/// The filter set: independent predicates, none required, AND-joined (Р2).
/// Build with [`Filters::default`] (the bare ceiling) and set the fields you
/// want.
#[derive(Debug, Clone)]
pub struct Filters {
    /// Exact match against a spec unit's `spec://…#anchor` URI. Only spec
    /// units carry a URI, so this excludes every code item.
    pub uri: Option<String>,
    /// Substring of a code item's symbol (case-sensitive, like `grep`). Only
    /// code items carry a symbol, so this excludes every spec unit.
    pub symbol: Option<String>,
    /// Exact match against a node's unified `kind` (a code `item_kind` or a
    /// spec unit kind). See [`Hit::kind`].
    pub kind: Option<String>,
    /// Result ceiling, clamped to `[1, `[`MAX_LIMIT`]`]` by [`search`].
    /// [`DEFAULT_LIMIT`] applies when the caller does not say.
    pub limit: usize,
}

impl Default for Filters {
    /// No predicates and the default ceiling — "show a bounded slice of the
    /// whole map".
    fn default() -> Self {
        Filters {
            uri: None,
            symbol: None,
            kind: None,
            limit: DEFAULT_LIMIT,
        }
    }
}

impl Filters {
    /// Whether *any* predicate is set. The ceiling applies either way
    /// (ПРОВЕРЬ-5); this only shapes the rendered "no filters" header.
    pub fn has_any_predicate(&self) -> bool {
        self.uri.is_some() || self.symbol.is_some() || self.kind.is_some()
    }
}

/// The search outcome: the (capped) hits plus the facts a surface needs to
/// report truncation honestly. Returning the count alongside the hits — rather
/// than a bare `Vec<Hit>` — is what lets the CLI/MCP say "showing N of M"
/// (ПРОВЕРЬ-5); the boss's `Vec<Hit>` sketch is refined only by wrapping it.
#[derive(Debug, Clone)]
pub struct SearchOut {
    /// The capped result list — the first `limit` matches in deterministic
    /// (spec-units-then-code-items, index) order.
    pub hits: Vec<Hit>,
    /// How many nodes matched *before* the ceiling — `>= hits.len()`.
    pub total_matching: usize,
    /// The effective ceiling after clamping.
    pub limit: usize,
}

impl SearchOut {
    /// Whether the ceiling dropped matches the caller never saw.
    pub fn is_truncated(&self) -> bool {
        self.total_matching > self.hits.len()
    }
}

/// One rendered search answer: the deterministic text view, or the structured
/// JSON. Mirrors [`Explain`](crate::Explain) / [`Fragment`](crate::Fragment)
/// so the three capabilities stay parallel.
#[derive(Debug)]
pub enum SearchView {
    /// The human-readable, scannable view.
    Text(String),
    /// The structured view for an agent or script.
    Json(Value),
}

/// Search an already-built `map`: collect every node passing all set
/// predicates, cap at the (clamped) ceiling, return the hits plus the pre-cap
/// total. Pure — same map + filters ⇒ same result, same order (spec units then
/// code items, index order). Callers wanting a tree built fresh use [`query`].
/// `limit` is clamped to `[1, `[`MAX_LIMIT`]`]` so a direct caller cannot bypass
/// the hard ceiling.
///
/// `search` is pure, so a doctest needs no tree on disk — hand-build a map:
///
/// ```
/// use specmap_core::generated::specmap::Specmap;
/// use vibe_trace::search::{Filters, search};
///
/// let map: Specmap = serde_json::from_str(
///     r#"{"schema":3,"code_items":[{"crate_name":"x","file":"x.rs","item_kind":"fn","line":1,"symbol":"x::f"}],"edges":[],"spec_units":[],"suspects":[],"warnings":[]}"#,
/// ).unwrap();
/// // `kind = "fn"` returns exactly the code items of that kind — none other.
/// let out = search(&map, &Filters { kind: Some("fn".into()), ..Filters::default() });
/// assert_eq!(out.hits.len(), 1);
/// assert_eq!(out.hits[0].kind.as_deref(), Some("fn"));
/// ```
pub fn search(map: &Specmap, filters: &Filters) -> SearchOut {
    let limit = filters.limit.clamp(1, MAX_LIMIT);
    let mut hits: Vec<Hit> =
        Vec::with_capacity(limit.min(map.specUnits.len() + map.codeItems.len()));
    let mut total_matching = 0usize;

    // Spec units first (index order), then code items (index order): the
    // combined stream is deterministic, and the ceiling is applied across it
    // as a whole — once `limit` hits are collected the rest are counted but
    // not kept, so the truncation total stays honest.
    for unit in &map.specUnits {
        let hit = hit_from_spec(unit);
        if matches_filters(&hit, filters) {
            total_matching += 1;
            if hits.len() < limit {
                hits.push(hit);
            }
        }
    }
    for item in &map.codeItems {
        let hit = hit_from_code(item);
        if matches_filters(&hit, filters) {
            total_matching += 1;
            if hits.len() < limit {
                hits.push(hit);
            }
        }
    }

    SearchOut {
        hits,
        total_matching,
        limit,
    }
}

/// Build the traceability index **fresh** for `root` (the same posture
/// [`explain`](crate::explain) takes — never a stale committed artefact) and
/// run [`search`] over it. The shared entry both surfaces call; the CLI/MCP add
/// no build logic of their own.
pub fn query(root: &Path, filters: &Filters) -> Result<SearchOut> {
    let cfg = specmap_core::config::Config::load(root)?.unwrap_or_default();
    let map = specmap_core::index::build(root, &cfg);
    Ok(search(&map, filters))
}

/// Render `out` for a surface. `json` selects the structured view an agent
/// consumes; the default is the deterministic, scannable text. `filters` shape
/// the text header and echo back in the JSON envelope so a consumer sees what
/// produced the answer.
pub fn render(out: &SearchOut, filters: &Filters, json: bool) -> SearchView {
    if json {
        SearchView::Json(render_json(out, filters))
    } else {
        SearchView::Text(render_text(out, filters))
    }
}

// --- node extraction + predicate -----------------------------------------

/// Project a spec unit into a [`Hit`]. The unit's own `kind` (if any) is the
/// node's kind — spec units answer `--kind` on their own kind (Р2, УТОЧНИ-4);
/// a legacy-unmarked unit matches none.
fn hit_from_spec(u: &SpecUnit) -> Hit {
    Hit {
        source: HitSource::Spec,
        name: u.heading.clone(),
        kind: u.kind.as_deref().map(spec_kind_str).map(str::to_owned),
        file: u.file.clone(),
        line: u.line,
        uri: Some(u.uri.clone()),
        crate_name: None,
    }
}

/// Project a code item into a [`Hit`]; the item's `item_kind` is its kind.
fn hit_from_code(c: &CodeItem) -> Hit {
    Hit {
        source: HitSource::Code,
        name: c.symbol.clone(),
        kind: Some(c.itemKind.clone()),
        file: c.file.clone(),
        line: c.line,
        uri: None,
        crate_name: Some(c.crateName.clone()),
    }
}

/// A spec unit's kind enum → its wire string (mirrors the JTD serialization).
fn spec_kind_str(k: &SpecUnitKind) -> &'static str {
    use SpecUnitKind::*;
    match k {
        Design => "design",
        Guide => "guide",
        Prop => "prop",
        Req => "req",
    }
}

/// Whether `hit` passes every predicate `filters` sets — AND semantics (Р2).
/// A `None` predicate is not applied; the set ones must all pass. `uri` matches
/// spec units only and `symbol` code items only, so combining them narrows to
/// nothing rather than widening.
fn matches_filters(hit: &Hit, filters: &Filters) -> bool {
    if let Some(uri) = &filters.uri
        && (hit.source != HitSource::Spec || hit.uri.as_deref() != Some(uri.as_str()))
    {
        return false;
    }
    if let Some(symbol) = &filters.symbol
        && (hit.source != HitSource::Code || !hit.name.contains(symbol.as_str()))
    {
        return false;
    }
    if let Some(kind) = &filters.kind
        && hit.kind.as_deref() != Some(kind.as_str())
    {
        return false;
    }
    true
}

// --- rendering -----------------------------------------------------------

/// The human-readable view: a one-line header (filters + count or the
/// truncation note) then one scannable line per hit.
fn render_text(out: &SearchOut, filters: &Filters) -> String {
    let mut s = String::new();

    s.push_str("map query · ");
    if filters.has_any_predicate() {
        let mut parts: Vec<String> = Vec::new();
        if let Some(uri) = &filters.uri {
            parts.push(format!("uri={uri}"));
        }
        if let Some(symbol) = &filters.symbol {
            parts.push(format!("symbol~\"{symbol}\""));
        }
        if let Some(kind) = &filters.kind {
            parts.push(format!("kind={kind}"));
        }
        s.push_str(&parts.join(" AND "));
    } else {
        s.push_str("no filters (whole map)");
    }
    s.push('\n');

    if out.is_truncated() {
        s.push_str(&format!(
            "showing {} of {} matching — narrow the filters or raise --limit (max {})\n",
            out.hits.len(),
            out.total_matching,
            MAX_LIMIT
        ));
    } else {
        let n = out.hits.len();
        s.push_str(&format!("{} result{}\n", n, if n == 1 { "" } else { "s" }));
    }
    s.push('\n');

    for hit in &out.hits {
        let src = match hit.source {
            HitSource::Spec => "spec",
            HitSource::Code => "code",
        };
        let kind = hit.kind.as_deref().unwrap_or("-");
        let mut line = format!(
            "  {src:<4} {kind:<10} {}  {}:{}",
            hit.name, hit.file, hit.line
        );
        // Spec hits carry their canonical address — append it so a hit is
        // directly citable; code hits carry no URI.
        if hit.source == HitSource::Spec
            && let Some(uri) = &hit.uri
        {
            line.push_str("  ");
            line.push_str(uri);
        }
        s.push_str(line.trim_end());
        s.push('\n');
    }

    s
}

/// The structured view: filters, the capped results, the counts, and a
/// `truncated` flag a script can branch on (ПРОВЕРЬ-5 / УТОЧНИ-6).
fn render_json(out: &SearchOut, filters: &Filters) -> Value {
    let results = serde_json::to_value(&out.hits).unwrap_or(Value::Null);
    json!({
        "filters": {
            "uri": filters.uri,
            "symbol": filters.symbol,
            "kind": filters.kind,
            "limit": filters.limit,
        },
        "results": results,
        "count": out.hits.len(),
        "total_matching": out.total_matching,
        "limit": out.limit,
        "truncated": out.is_truncated(),
    })
}

#[cfg(test)]
mod tests;
