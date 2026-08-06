//! Tests for the query-language layer (E-A5B-QUERYLANG). Kept in a sibling
//! file so the capability stays under the per-file budget — the same shape as
//! `search/tests.rs` beside it. Every map here is hand-built JSON (the layer
//! is pure over a [`Specmap`]), so the tests assert against a fixed graph
//! without touching the filesystem.

use specmap_core::generated::specmap::Specmap;

use super::*;
use crate::search::{Filters, HitSource, search};

/// The canonical fixture graph (bipartite, directed code→spec):
///
/// ```text
///   spec: req-r ◄──implements── f ::fn
///        ◄──verifies──── t ::fn
///        ◄──informs────── d ::fn ──documents──► guide-g
///   spec: prop-p   (no edges)        code: u ::struct (orphan, no edges)
/// ```
///
/// Four edges, three spec units, four code items — enough to exercise every
/// predicate and both traversal directions without coupling the assertions.
fn map() -> Specmap {
    serde_json::from_str(
        r#"{"schema":3,
           "spec_units":[
             {"anchor":"req-r","content_hash":"h1","doc_path":"D","file":"spec/D.md","heading":"The rule","line":1,"uri":"spec://demo/D#req-r","kind":"req"},
             {"anchor":"guide-g","content_hash":"h2","doc_path":"D","file":"spec/D.md","heading":"The guide","line":9,"uri":"spec://demo/D#guide-g","kind":"guide"},
             {"anchor":"prop-p","content_hash":"h3","doc_path":"D","file":"spec/D.md","heading":"The prop","line":17,"uri":"spec://demo/D#prop-p","kind":"prop"}],
           "code_items":[
             {"crate_name":"x","file":"x/src/lib.rs","item_kind":"fn","line":1,"symbol":"x::f"},
             {"crate_name":"x","file":"x/src/lib.rs","item_kind":"fn","line":3,"symbol":"x::t"},
             {"crate_name":"x","file":"x/src/lib.rs","item_kind":"fn","line":5,"symbol":"x::d"},
             {"crate_name":"x","file":"x/src/lib.rs","item_kind":"struct","line":7,"symbol":"x::u"}],
           "edges":[
             {"file":"x/src/lib.rs","from_symbol":"x::f","line":1,"provenance":"authored","uri":"spec://demo/D#req-r","verb":"implements"},
             {"file":"x/src/lib.rs","from_symbol":"x::t","line":3,"provenance":"authored","uri":"spec://demo/D#req-r","verb":"verifies"},
             {"file":"x/src/lib.rs","from_symbol":"x::d","line":5,"provenance":"authored","uri":"spec://demo/D#guide-g","verb":"documents"},
             {"file":"x/src/lib.rs","from_symbol":"x::d","line":5,"provenance":"authored","uri":"spec://demo/D#req-r","verb":"informs"}],
           "suspects":[],"warnings":[]}"#,
    )
    .unwrap()
}

fn run(q: &str) -> SelectOut {
    select(&map(), &parse(q).unwrap(), 50)
}

fn names(out: &SelectOut) -> Vec<&str> {
    out.hits.iter().map(|h| h.hit.name.as_str()).collect()
}

// --- parsing (acceptance 3, 4, 5) ------------------------------------------

/// All seven predicates parse; `depth` defaults to 0.
#[test]
fn all_predicates_parse() {
    let q = parse("uri:spec://demo/D#req-r symbol:x kind:fn scope:spec://demo has:implements lacks:verifies depth:2").unwrap();
    assert_eq!(q.uri.as_deref(), Some("spec://demo/D#req-r"));
    assert_eq!(q.symbol.as_deref(), Some("x"));
    assert_eq!(q.kind.as_deref(), Some("fn"));
    assert_eq!(q.scope.as_deref(), Some("spec://demo"));
    assert_eq!(q.has, Some(Verb::Implements));
    assert_eq!(q.lacks, Some(Verb::Verifies));
    assert_eq!(q.depth, 2);
}

/// An empty query (or whitespace only) is an ERROR, not "everything", and the
/// message points at the first level (§2.2 point 9).
#[test]
fn empty_query_is_an_error_pointing_at_the_floor() {
    let err = parse("").expect_err("empty ⇒ error");
    assert!(format!("{err}").contains("empty"), "{err}");
    assert!(format!("{err}").contains("`vibe query`"), "{err}");
    parse("   ").expect_err("whitespace only ⇒ error");
}

/// Each parse error names the offending token and lists the expected.
#[test]
fn parse_errors_name_the_token_and_list_expected() {
    let unknown = parse("bogus:x").expect_err("unknown predicate");
    let m = format!("{unknown}");
    assert!(m.contains("`bogus:x`"), "{m}");
    assert!(m.contains("`uri:`") && m.contains("`depth:`"), "{m}");

    let verb = parse("has:maybe").expect_err("unknown verb");
    let m = format!("{verb}");
    assert!(m.contains("`maybe`"), "{m}");
    assert!(m.contains("`verifies`"), "{m}");

    parse("depth:4").expect_err("depth out of range");
    let d = parse("depth:four").expect_err("non-numeric depth");
    assert!(format!("{d}").contains("`four`"));

    let bare = parse("lacks:").expect_err("missing value");
    assert!(format!("{bare}").contains("`lacks:`"), "{bare}");
}

/// A repeated predicate is an error under AND (§2.2 point 5).
#[test]
fn a_repeated_predicate_is_an_error() {
    let err = parse("uri:a uri:b").expect_err("repeated");
    assert!(format!("{err}").contains("`uri:`"), "{err}");
}

// --- the seven predicates (acceptance 3) -----------------------------------

/// `uri:` returns exactly that spec unit.
#[test]
fn uri_predicate_returns_one_spec_unit() {
    let out = run("uri:spec://demo/D#req-r");
    assert_eq!(names(&out), vec!["The rule"]);
    assert_eq!(out.hits[0].hit.source, HitSource::Spec);
    assert_eq!(out.hits[0].hit.uri.as_deref(), Some("spec://demo/D#req-r"));
}

/// `symbol:` is a substring over code symbols only.
#[test]
fn symbol_predicate_matches_substring_of_code() {
    let out = run("symbol:x::");
    assert_eq!(names(&out).len(), 4); // f, t, d, u — all code, all carry "x::"
    assert!(out.hits.iter().all(|h| h.hit.source == HitSource::Code));
}

/// `kind:` unifies the two vocabularies: a code `fn` and a spec `req`.
#[test]
fn kind_predicate_unifies_code_and_spec() {
    let fns = run("kind:fn");
    assert_eq!(fns.hits.len(), 3); // f, t, d
    let reqs = run("kind:req");
    assert_eq!(names(&reqs), vec!["The rule"]);
    let structs = run("kind:struct");
    assert_eq!(names(&structs), vec!["x::u"]);
}

/// `scope:` is a uri PREFIX over spec units only — code never passes it.
#[test]
fn scope_predicate_is_a_prefix_over_spec_units() {
    let out = run("scope:spec://demo/D");
    assert_eq!(out.hits.len(), 3); // req-r, guide-g, prop-p
    assert!(out.hits.iter().all(|h| h.hit.source == HitSource::Spec));
    let none = run("scope:spec://other");
    assert!(none.hits.is_empty());
}

/// `has:<verb>` keeps only nodes an edge of that verb touches — for a spec
/// unit its INCOMING edge, for a code item its OUTGOING one.
#[test]
fn has_verb_keeps_touched_nodes_both_families() {
    let out = run("has:verifies");
    // req-r (incoming verifies from t) + t (outgoing verifies).
    assert_eq!(names(&out), vec!["The rule", "x::t"]);
}

// --- ПРОВЕРЬ-5: lacks counts the right direction ---------------------------

/// `lacks:verifies` for a SPEC unit counts INCOMING edges. `req-r` HAS an
/// incoming verifies edge (from `t`), so it must NOT pass; `guide-g` and
/// `prop-p` have none, so they do. If the code read OUTGOING edges for a spec
/// unit (spec units have none), `req-r` would wrongly appear — this test fails.
#[test]
fn lacks_for_a_spec_unit_counts_incoming_edges() {
    let out = run("lacks:verifies");
    let ns = names(&out);
    // Spec side: guide-g, prop-p pass; req-r does NOT (incoming verifies).
    assert!(
        !ns.contains(&"The rule"),
        "req-r has an incoming verifies edge — it must NOT pass `lacks:verifies`: {ns:?}"
    );
    assert!(
        ns.contains(&"The guide") && ns.contains(&"The prop"),
        "{ns:?}"
    );
    // Code side: f, d, u pass (no outgoing verifies); t does NOT.
    assert!(
        !ns.contains(&"x::t"),
        "t has an outgoing verifies edge — it must NOT pass: {ns:?}"
    );
    assert!(
        ns.contains(&"x::f") && ns.contains(&"x::d") && ns.contains(&"x::u"),
        "{ns:?}"
    );
}

// --- depth semantics (acceptance 6) ----------------------------------------

/// `depth:0` and the absence of `depth` are identical — seeds only.
#[test]
fn depth_zero_equals_no_depth() {
    let no_depth = run("kind:fn");
    let zero = run("kind:fn depth:0");
    assert_eq!(names(&no_depth), names(&zero));
    assert!(no_depth.hits.iter().all(|h| h.depth == 0));
}

/// ПРОВЕРЬ-6: the walk is UNDIRECTED. One step OUT of a spec unit reaches
/// code items (the items whose edges point at it).
#[test]
fn depth_one_from_a_spec_unit_reaches_code() {
    let out = run("uri:spec://demo/D#req-r depth:1");
    // d0: req-r. d1: f, t, d (all point at req-r).
    assert_eq!(out.total_matching, 4);
    let depths: Vec<u32> = out.hits.iter().map(|h| h.depth).collect();
    assert_eq!(depths, vec![0, 1, 1, 1]);
    assert_eq!(out.hits[0].hit.source, HitSource::Spec);
    let d1: Vec<&str> = out.hits[1..].iter().map(|h| h.hit.name.as_str()).collect();
    assert_eq!(d1, vec!["x::f", "x::t", "x::d"]); // index order
}

/// ПРОВЕРЬ-6 (other direction): one step OUT of a code item reaches spec
/// units. A directed (code→spec only) walk would fail ONE of the two.
#[test]
fn depth_one_from_a_code_item_reaches_spec() {
    let out = run("symbol:x::d depth:1");
    // d0: d. d1: guide-g (documents) and req-r (informs) — both spec.
    let specs: Vec<&str> = out
        .hits
        .iter()
        .filter(|h| h.depth == 1)
        .map(|h| h.hit.uri.as_deref().unwrap_or(""))
        .collect();
    assert!(specs.contains(&"spec://demo/D#guide-g"), "{specs:?}");
    assert!(specs.contains(&"spec://demo/D#req-r"), "{specs:?}");
}

/// A two-step walk crosses back across the family: req-r → d → guide-g.
#[test]
fn depth_two_crosses_back_across_the_family() {
    let out = run("uri:spec://demo/D#req-r depth:2");
    // req-r(0); f,t,d(1); from d → guide-g(2). prop-p, u never reached.
    assert_eq!(out.total_matching, 5);
    let guide = out
        .hits
        .iter()
        .find(|h| h.hit.uri.as_deref() == Some("spec://demo/D#guide-g"))
        .expect("guide-g reached at depth 2");
    assert_eq!(guide.depth, 2);
}

// --- ordering + ceiling (acceptance 7, 8, 9) -------------------------------

/// Results are ordered: depth ascending, then spec-before-code, then index.
#[test]
fn results_are_ordered_depth_then_spec_then_index() {
    let out = run("uri:spec://demo/D#req-r depth:1");
    let sources: Vec<HitSource> = out.hits.iter().map(|h| h.hit.source).collect();
    assert_eq!(
        sources,
        vec![
            HitSource::Spec,
            HitSource::Code,
            HitSource::Code,
            HitSource::Code
        ]
    );
}

/// ПРОВЕРЬ-7: the ceiling applies AFTER the walk; `total_matching` counts
/// every reached node, and the cap drops only the shown tail.
#[test]
fn ceiling_applies_after_the_walk_and_total_is_honest() {
    // Reaches req-r(0) + f,t,d(1) = 4 nodes; cap at 1 ⇒ the spec seed shows.
    let out = select(
        &map(),
        &parse("uri:spec://demo/D#req-r depth:1").unwrap(),
        1,
    );
    assert_eq!(out.hits.len(), 1);
    assert_eq!(out.total_matching, 4);
    assert!(out.is_truncated());
    assert_eq!(out.hits[0].depth, 0);

    // Both views report the truncation.
    let SelectView::Text(text) = render(
        &out,
        &parse("uri:spec://demo/D#req-r depth:1").unwrap(),
        "uri:spec://demo/D#req-r depth:1",
        false,
    ) else {
        panic!("text view");
    };
    assert!(text.contains("showing 1 of 4"), "{text}");
    let SelectView::Json(v) = render(
        &out,
        &parse("uri:spec://demo/D#req-r depth:1").unwrap(),
        "uri:spec://demo/D#req-r depth:1",
        true,
    ) else {
        panic!("json view");
    };
    assert_eq!(v["count"], 1);
    assert_eq!(v["total_matching"], 4);
    assert_eq!(v["truncated"], true);
}

/// Each hit carries its step; `d0` for a seed (acceptance 9), and the text line
/// leads with it.
#[test]
fn every_hit_carries_its_step_and_text_leads_with_it() {
    let out = run("uri:spec://demo/D#req-r depth:1");
    assert_eq!(out.hits[0].depth, 0);
    assert!(out.hits[1..].iter().all(|h| h.depth == 1));
    let SelectView::Text(text) = render(
        &out,
        &parse("uri:spec://demo/D#req-r depth:1").unwrap(),
        "uri:spec://demo/D#req-r depth:1",
        false,
    ) else {
        panic!("text view");
    };
    assert!(text.contains(" d0 "), "seed line leads with d0: {text}");
    assert!(text.contains(" d1 "), "reached line leads with d1: {text}");
}

// --- JSON shape (acceptance 10) --------------------------------------------

/// JSON carries `grammar: 1`, the raw `query`, and `depth` per result.
#[test]
fn json_carries_grammar_query_and_depth() {
    let out = run("symbol:x::d");
    let parsed = parse("symbol:x::d").unwrap();
    let SelectView::Json(v) = render(&out, &parsed, "symbol:x::d", true) else {
        panic!("json view");
    };
    assert_eq!(v["grammar"], 1);
    assert_eq!(v["query"], "symbol:x::d");
    assert_eq!(v["parsed"]["symbol"], "x::d");
    let r = &v["results"][0];
    assert_eq!(r["depth"], 0);
    assert_eq!(r["source"], "code");
}

/// The text header prints the PARSED query, not the raw string.
#[test]
fn header_prints_the_parsed_query_not_the_raw_string() {
    let parsed = parse("kind:fn").unwrap();
    assert_eq!(parsed.header(), "kind=fn AND depth=0");
    let out = select(&map(), &parsed, 50);
    let SelectView::Text(text) = render(&out, &parsed, "kind:fn", false) else {
        panic!("text view");
    };
    assert!(text.contains("kind=fn AND depth=0"), "{text}");
    // The raw token shape (`kind:fn`) does not appear as a header echo.
    assert!(!text.starts_with("map select · grammar v1 · kind:fn"));
}

// --- ПРОВЕРЬ-8: the first level is untouched -------------------------------

/// The floor behaves exactly as before: same `Filters`, same `search`. The
/// query-language layer reused its predicate by visibility, not by editing it,
/// so the floor's own tests (run by `cargo test -p vibe-trace`) and this call
/// keep their behaviour.
#[test]
fn the_first_level_search_is_unchanged() {
    let out = search(
        &map(),
        &Filters {
            kind: Some("fn".into()),
            ..Filters::default()
        },
    );
    assert_eq!(out.hits.len(), 3);
    assert!(out.hits.iter().all(|h| h.kind.as_deref() == Some("fn")));
    // The query language reuses the same projection: a select seed for
    // `kind:fn` is the same three nodes.
    let sel = run("kind:fn");
    assert_eq!(sel.total_matching, out.total_matching);
}
