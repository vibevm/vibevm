//! Unit tests for the Search Everywhere engine ([`super`]). A [`FakeProvider`]
//! backed by a fixed candidate list drives tab construction, the shared-scale
//! ranking, grouping/caps, recency, and selection dispatch — no rendering, no
//! real providers.
//!
//! Out-of-line per `#[cfg(test)] mod tests;` so the engine module stays within
//! the file-length budget; every item here is test-only.

use super::*;
use std::cell::Cell;
use std::rc::Rc;

/// A test provider backed by a fixed candidate list.
struct FakeProvider {
    id: ProviderId,
    group: String,
    weight: i32,
    separate: bool,
    items: Vec<Candidate>,
    close: bool,
    last_selected: Rc<Cell<Option<usize>>>,
}

impl FakeProvider {
    fn new(id: ProviderId, weight: i32, items: Vec<Candidate>) -> Self {
        FakeProvider {
            id,
            group: format!("Group {id}"),
            weight,
            separate: true,
            items,
            close: true,
            last_selected: Rc::new(Cell::new(None)),
        }
    }
}

impl SearchProvider for FakeProvider {
    fn id(&self) -> ProviderId {
        self.id
    }
    fn group_name(&self) -> &str {
        &self.group
    }
    fn sort_weight(&self) -> i32 {
        self.weight
    }
    fn separate_tab(&self) -> bool {
        self.separate
    }
    fn candidates(&self, _query: &Query) -> Vec<Candidate> {
        self.items.clone()
    }
    fn on_selected(&self, item: ItemRef, _mods: Modifiers) -> Selected {
        self.last_selected.set(Some(item.0));
        if self.close {
            Selected::Close
        } else {
            Selected::Stay
        }
    }
}

fn cand(item: usize, primary: &str) -> Candidate {
    Candidate {
        item: ItemRef(item),
        primary: primary.to_owned(),
        secondary: None,
        extra_haystacks: Vec::new(),
        enabled: true,
    }
}

fn q(text: &str) -> Query<'_> {
    Query { text }
}

fn all_of(ids: &[ProviderId]) -> HashSet<ProviderId> {
    ids.iter().copied().collect()
}

fn hits(rows: &[SearchRow]) -> Vec<&Hit> {
    rows.iter()
        .filter_map(|r| match r {
            SearchRow::Hit(h) => Some(h),
            SearchRow::Header { .. } => None,
        })
        .collect()
}

fn engine(providers: Vec<FakeProvider>) -> SearchEngine {
    SearchEngine::new(
        providers
            .into_iter()
            .map(|p| Box::new(p) as Box<dyn SearchProvider>)
            .collect(),
    )
}

#[test]
fn tabs_have_all_only_with_multiple_providers() {
    let one = engine(vec![FakeProvider::new("a", 0, vec![cand(0, "x")])]);
    let tabs = one.tabs();
    assert_eq!(tabs.len(), 1);
    assert!(!tabs[0].is_all); // single provider → no "All"

    let two = engine(vec![
        FakeProvider::new("a", 0, vec![]),
        FakeProvider::new("b", 1, vec![]),
    ]);
    let tabs = two.tabs();
    assert_eq!(tabs.len(), 3);
    assert!(tabs[0].is_all);
    assert_eq!(tabs[0].id, "all");
    assert_eq!(tabs[0].title, "All");
    assert_eq!(tabs[0].providers, vec!["a", "b"]);
}

#[test]
fn tabs_are_ordered_by_sort_weight() {
    let eng = engine(vec![
        FakeProvider::new("late", 30, vec![]),
        FakeProvider::new("early", 10, vec![]),
        FakeProvider::new("mid", 20, vec![]),
    ]);
    let tabs = eng.tabs();
    // [All, early, mid, late]
    let ids: Vec<&str> = tabs.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids, vec!["all", "early", "mid", "late"]);
    assert_eq!(tabs[0].providers, vec!["early", "mid", "late"]);
}

#[test]
fn tabs_respect_separate_tab_false() {
    let mut hidden = FakeProvider::new("hidden", 5, vec![]);
    hidden.separate = false;
    let eng = engine(vec![hidden, FakeProvider::new("shown", 10, vec![])]);
    let tabs = eng.tabs();
    // All + shown, but not hidden's own tab.
    let ids: Vec<&str> = tabs.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids, vec!["all", "shown"]);
    // The "All" tab still searches hidden.
    assert!(tabs[0].providers.contains(&"hidden"));
}

#[test]
fn search_ranks_prefix_then_substring_then_subsequence() {
    let eng = engine(vec![FakeProvider::new(
        "p",
        0,
        vec![
            cand(0, "copyfile"), // prefix of "cop"
            cand(1, "scope"),    // substring
            cand(2, "c_o_x_p"),  // subsequence
        ],
    )]);
    let tab = &eng.tabs()[0];
    let rows = eng.search(&q("cop"), tab, &all_of(&["p"]));
    let h = hits(&rows);
    assert_eq!(h.len(), 3);
    assert_eq!(h[0].item, ItemRef(0));
    assert_eq!(h[1].item, ItemRef(1));
    assert_eq!(h[2].item, ItemRef(2));
    assert!(h[0].score > h[1].score && h[1].score > h[2].score);
}

#[test]
fn search_exact_tops_all() {
    let eng = engine(vec![FakeProvider::new(
        "p",
        0,
        vec![cand(0, "copyfile"), cand(1, "copy")],
    )]);
    let tab = &eng.tabs()[0];
    let rows = eng.search(&q("copy"), tab, &all_of(&["p"]));
    let h = hits(&rows);
    assert_eq!(h[0].item, ItemRef(1)); // exact "copy" beats prefix "copyfile"
}

#[test]
fn fallback_lane_ranks_below_primary_match() {
    let mut fallback = cand(1, "totally-unrelated");
    fallback.extra_haystacks = vec!["widget".to_owned()]; // matches only via keyword
    let eng = engine(vec![FakeProvider::new(
        "p",
        0,
        vec![cand(0, "widget"), fallback],
    )]);
    let tab = &eng.tabs()[0];
    let rows = eng.search(&q("widget"), tab, &all_of(&["p"]));
    let h = hits(&rows);
    assert_eq!(h.len(), 2);
    assert_eq!(h[0].item, ItemRef(0)); // primary exact match wins
    assert_eq!(h[1].item, ItemRef(1)); // still surfaces via fallback lane
    assert!(h[0].score > h[1].score);
    assert!(h[1].match_ranges.is_empty()); // hidden-field match highlights nothing
    assert!(!h[0].match_ranges.is_empty());
}

#[test]
fn empty_query_returns_candidates_in_provider_order() {
    let eng = engine(vec![FakeProvider::new(
        "p",
        0,
        vec![cand(0, "gamma"), cand(1, "alpha"), cand(2, "beta")],
    )]);
    let tab = Tab {
        id: "p".to_owned(),
        title: "p".to_owned(),
        providers: vec!["p"],
        is_all: false,
    };
    let rows = eng.search(&q(""), &tab, &HashSet::new());
    let items: Vec<usize> = hits(&rows).iter().map(|h| h.item.0).collect();
    assert_eq!(items, vec![0, 1, 2]); // untouched provider order
    assert!(hits(&rows).iter().all(|h| h.score == 0));
}

#[test]
fn per_provider_cap_is_15_in_all_tab() {
    let many: Vec<Candidate> = (0..40).map(|i| cand(i, &format!("item{i}"))).collect();
    let eng = engine(vec![
        FakeProvider::new("big", 0, many),
        FakeProvider::new("other", 1, vec![cand(99, "unrelated")]),
    ]);
    let tab = &eng.tabs()[0]; // "All"
    assert!(tab.is_all);
    let rows = eng.search(&q("item"), tab, &all_of(&["big", "other"]));
    let big_hits = hits(&rows)
        .into_iter()
        .filter(|h| h.provider == "big")
        .count();
    assert_eq!(big_hits, CAP_ALL); // 15
}

#[test]
fn per_provider_cap_is_30_in_single_tab() {
    let many: Vec<Candidate> = (0..40).map(|i| cand(i, &format!("item{i}"))).collect();
    let eng = engine(vec![FakeProvider::new("p", 0, many)]);
    let tab = Tab {
        id: "p".to_owned(),
        title: "p".to_owned(),
        providers: vec!["p"],
        is_all: false,
    };
    let rows = eng.search(&q("item"), &tab, &HashSet::new());
    assert_eq!(hits(&rows).len(), CAP_SINGLE); // 30
}

#[test]
fn all_tab_emits_group_headers() {
    let eng = engine(vec![
        FakeProvider::new("a", 0, vec![cand(0, "apple")]),
        FakeProvider::new("b", 1, vec![cand(0, "apricot")]),
    ]);
    let tab = &eng.tabs()[0];
    let rows = eng.search(&q("ap"), tab, &all_of(&["a", "b"]));
    // Header, hit, Header, hit — headers in sort_weight order.
    assert!(
        matches!(&rows[0], SearchRow::Header { provider, count, .. } if *provider == "a" && *count == 1)
    );
    assert!(matches!(&rows[1], SearchRow::Hit(_)));
    assert!(matches!(&rows[2], SearchRow::Header { provider, .. } if *provider == "b"));
    assert!(matches!(&rows[3], SearchRow::Hit(_)));
}

#[test]
fn single_tab_emits_no_headers() {
    let eng = engine(vec![FakeProvider::new("p", 0, vec![cand(0, "apple")])]);
    let tab = Tab {
        id: "p".to_owned(),
        title: "p".to_owned(),
        providers: vec!["p"],
        is_all: false,
    };
    let rows = eng.search(&q("ap"), &tab, &HashSet::new());
    assert!(rows.iter().all(|r| matches!(r, SearchRow::Hit(_))));
    assert_eq!(rows.len(), 1);
}

#[test]
fn disabled_candidate_sorts_below_enabled() {
    let mut disabled = cand(0, "apple"); // exact-ish, but disabled
    disabled.enabled = false;
    let enabled = cand(1, "application"); // weaker prefix match, but enabled
    let eng = engine(vec![FakeProvider::new("p", 0, vec![disabled, enabled])]);
    let tab = Tab {
        id: "p".to_owned(),
        title: "p".to_owned(),
        providers: vec!["p"],
        is_all: false,
    };
    let rows = eng.search(&q("app"), &tab, &HashSet::new());
    let h = hits(&rows);
    assert_eq!(h[0].item, ItemRef(1)); // enabled first despite weaker match
    assert!(h[0].enabled && !h[1].enabled);
    assert!(h[0].score > h[1].score);
}

#[test]
fn on_selected_dispatches_to_the_owning_provider() {
    let mut a = FakeProvider::new("a", 0, vec![cand(7, "x")]);
    a.close = true;
    let a_rec = a.last_selected.clone();
    let mut b = FakeProvider::new("b", 1, vec![cand(3, "y")]);
    b.close = false;
    let b_rec = b.last_selected.clone();

    let mut eng = engine(vec![a, b]);
    assert_eq!(
        eng.on_selected("a", ItemRef(7), Modifiers::default()),
        Selected::Close
    );
    assert_eq!(a_rec.get(), Some(7));
    assert_eq!(b_rec.get(), None); // b untouched

    assert_eq!(
        eng.on_selected("b", ItemRef(3), Modifiers::default()),
        Selected::Stay
    );
    assert_eq!(b_rec.get(), Some(3));

    // Unknown provider is a safe no-op.
    assert_eq!(
        eng.on_selected("ghost", ItemRef(0), Modifiers::default()),
        Selected::Stay
    );
}

#[test]
fn recency_boost_floats_a_reselected_item() {
    let eng_items = vec![cand(0, "report"), cand(1, "report")];
    let mut eng = engine(vec![FakeProvider::new("p", 0, eng_items)]);
    let tab = Tab {
        id: "p".to_owned(),
        title: "p".to_owned(),
        providers: vec!["p"],
        is_all: false,
    };
    // Before any selection, the tie breaks by provider order → item 0 first.
    let rows = eng.search(&q("report"), &tab, &HashSet::new());
    assert_eq!(hits(&rows)[0].item, ItemRef(0));

    // Select item 1; it should now float above the identical item 0.
    eng.on_selected("p", ItemRef(1), Modifiers::default());
    let rows = eng.search(&q("report"), &tab, &HashSet::new());
    assert_eq!(hits(&rows)[0].item, ItemRef(1));
}

#[test]
fn all_tab_filter_skips_unchecked_provider() {
    let eng = engine(vec![
        FakeProvider::new("a", 0, vec![cand(0, "apple")]),
        FakeProvider::new("b", 1, vec![cand(0, "apricot")]),
    ]);
    let tab = &eng.tabs()[0];
    // Only "a" is checked in the category filter.
    let rows = eng.search(&q("ap"), tab, &all_of(&["a"]));
    assert!(hits(&rows).iter().all(|h| h.provider == "a"));
    assert!(
        !rows
            .iter()
            .any(|r| matches!(r, SearchRow::Header { provider, .. } if *provider == "b"))
    );
}
