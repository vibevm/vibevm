//! Tests for the map-search capability. Kept in a sibling file so the
//! capability's implementation stays under the per-file budget — the same
//! reason and the same shape as `fragment/tests.rs` beside it. Split out
//! 2026-08-06, when formatting put the module at 621 lines.

use super::*;
use specmap_core::config::Config;

/// The canonical "what realises this rule?" shape, in the engine's own
/// synthetic-tree format (taken whole from `lib.rs`'s fixture).
const URI: &str = "spec://demo/D#req-r";

fn tree() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("specmap.toml"),
        "namespace = \"demo\"\nscan_roots = [\"crates/*\"]\nspec_roots = [\"spec\"]\n",
    )
    .unwrap();
    std::fs::create_dir_all(root.join("spec")).unwrap();
    std::fs::write(
        root.join("spec/D.md"),
        "## The rule {#req-r}\n`req r1`\n\nIt MUST hold.\n",
    )
    .unwrap();
    let src = root.join("crates/x/src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        src.join("lib.rs"),
        concat!(
            "#[spec(implements = \"spec://demo/D#req-r\", r = 1)]\n",
            "pub fn f() {}\n\n",
            "#[verifies(\"spec://demo/D#req-r\")]\n",
            "fn t() {}\n",
        ),
    )
    .unwrap();
    tmp
}

fn map_for(root: &Path) -> Specmap {
    let cfg = Config::load(root).unwrap().unwrap_or_default();
    specmap_core::index::build(root, &cfg)
}

/// Acceptance 1: `--kind <kind>` returns code items of that kind only.
#[test]
fn kind_filter_returns_only_that_code_kind() {
    let tmp = tree();
    let map = map_for(tmp.path());
    let out = search(
        &map,
        &Filters {
            kind: Some("fn".into()),
            ..Filters::default()
        },
    );
    assert!(!out.hits.is_empty(), "`fn` items must exist in the fixture");
    assert!(
        out.hits
            .iter()
            .all(|h| h.kind.as_deref() == Some("fn") && h.source == HitSource::Code),
        "{:?}",
        out.hits
    );
    let symbols: Vec<&str> = out.hits.iter().map(|h| h.name.as_str()).collect();
    assert!(symbols.iter().any(|s| s.ends_with("::f")), "{symbols:?}");
    assert!(symbols.iter().any(|s| s.ends_with("::t")), "{symbols:?}");
}

/// Acceptance 2: `--symbol <sub>` returns items whose symbol contains it.
#[test]
fn symbol_filter_matches_substring_of_code_symbols() {
    let tmp = tree();
    let map = map_for(tmp.path());
    let out = search(
        &map,
        &Filters {
            symbol: Some("::f".into()),
            ..Filters::default()
        },
    );
    assert!(
        out.hits
            .iter()
            .all(|h| h.source == HitSource::Code && h.name.contains("::f")),
        "{:?}",
        out.hits
    );
    assert!(out.hits.iter().any(|h| h.name.ends_with("::f")));
}

/// Acceptance 3: `--uri <exact>` returns exactly that spec unit.
#[test]
fn uri_filter_returns_exactly_that_spec_unit() {
    let tmp = tree();
    let map = map_for(tmp.path());
    let out = search(
        &map,
        &Filters {
            uri: Some(URI.into()),
            ..Filters::default()
        },
    );
    assert_eq!(out.hits.len(), 1);
    assert_eq!(out.hits[0].source, HitSource::Spec);
    assert_eq!(out.hits[0].uri.as_deref(), Some(URI));
    assert_eq!(out.total_matching, 1);
}

/// Acceptance 4: two filters together NARROW (AND), not widen.
#[test]
fn two_filters_narrow_with_and_not_widen_with_or() {
    let tmp = tree();
    let map = map_for(tmp.path());
    let just_kind = search(
        &map,
        &Filters {
            kind: Some("fn".into()),
            ..Filters::default()
        },
    );
    let kind_and_symbol = search(
        &map,
        &Filters {
            kind: Some("fn".into()),
            symbol: Some("::t".into()),
            ..Filters::default()
        },
    );
    assert!(kind_and_symbol.hits.len() <= just_kind.hits.len());
    assert_eq!(kind_and_symbol.hits.len(), 1);
    assert!(kind_and_symbol.hits[0].name.ends_with("::t"));
    // The impossible conjunction narrows to nothing.
    let none = search(
        &map,
        &Filters {
            uri: Some(URI.into()),
            symbol: Some("::t".into()),
            ..Filters::default()
        },
    );
    assert!(
        none.hits.is_empty(),
        "a node is either spec or code, never both"
    );
}

/// Acceptance 5: no filters caps at the ceiling and names the truncation.
#[test]
fn no_filters_is_capped_and_truncation_is_visible() {
    let tmp = tree();
    let map = map_for(tmp.path());
    let out = search(&map, &Filters::default());
    assert!(out.hits.len() <= DEFAULT_LIMIT);
    assert_eq!(out.total_matching, out.hits.len());
    assert!(!out.is_truncated());
    // Force the ceiling below the match count: truncation must surface.
    let many = search(
        &map,
        &Filters {
            limit: 1,
            ..Filters::default()
        },
    );
    assert_eq!(many.hits.len(), 1);
    assert!(many.total_matching > 1);
    assert!(many.is_truncated());
    let SearchView::Text(text) = render(&many, &Filters::default(), false) else {
        panic!("expected the text view");
    };
    assert!(
        text.contains("showing 1 of"),
        "truncation named in text: {text}"
    );
}

/// Acceptance 6: `--json` is a machine-readable form of the same slice.
#[test]
fn json_view_is_machine_readable_and_matches_the_text_slice() {
    let tmp = tree();
    let map = map_for(tmp.path());
    let filters = Filters {
        kind: Some("fn".into()),
        ..Filters::default()
    };
    let out = search(&map, &filters);
    let SearchView::Json(value) = render(&out, &filters, true) else {
        panic!("expected the json view");
    };
    assert_eq!(value["count"], out.hits.len());
    assert_eq!(value["total_matching"], out.total_matching);
    assert_eq!(value["truncated"], out.is_truncated());
    assert_eq!(value["filters"]["kind"], "fn");
    let results = value["results"].as_array().expect("results array");
    assert_eq!(results.len(), out.hits.len());
    assert!(
        results
            .iter()
            .all(|r| r["source"] == "code" && r["kind"] == "fn")
    );
}

/// УТОЧНИ-4: a spec unit answers `--kind` on its OWN kind. The fixture's
/// unit is marked `req` (`` `req r1` `` parses), so `--kind req` selects
/// it; a code item never matches a spec kind. (THIS tree's committed map
/// carries no marked spec units — all `kind: null` — so over the real tree
/// a spec kind yields nothing today; the path is open and proven here.)
#[test]
fn a_spec_unit_answers_kind_on_its_own_kind() {
    let tmp = tree();
    let map = map_for(tmp.path());
    let req = search(
        &map,
        &Filters {
            kind: Some("req".into()),
            ..Filters::default()
        },
    );
    assert_eq!(req.hits.len(), 1, "{:?}", req.hits);
    assert_eq!(req.hits[0].source, HitSource::Spec);
    assert_eq!(req.hits[0].kind.as_deref(), Some("req"));
    assert_eq!(req.hits[0].uri.as_deref(), Some(URI));
    let unit = search(
        &map,
        &Filters {
            uri: Some(URI.into()),
            ..Filters::default()
        },
    );
    assert_eq!(unit.hits[0].kind.as_deref(), Some("req"));
}

/// The ceiling is hard: an out-of-range `limit` is clamped, never bypassed.
#[test]
fn limit_is_clamped_to_the_hard_max() {
    let tmp = tree();
    let map = map_for(tmp.path());
    let out = search(
        &map,
        &Filters {
            limit: 100_000,
            ..Filters::default()
        },
    );
    assert_eq!(out.limit, MAX_LIMIT);
    assert!(out.hits.len() <= MAX_LIMIT);
}
