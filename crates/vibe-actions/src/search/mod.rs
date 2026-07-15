//! Search Everywhere — the engine, provider trait, and result model
//! (PROP-039 §10).
//!
//! One window searches every registered universe — packages, package-card
//! fields, actions — through a uniform [`SearchProvider`] contract, with a
//! hybrid **"All"** tab plus per-category tabs (§10.6). A provider only
//! *enumerates* [`Candidate`]s; the **engine** scores every candidate with the
//! single [`matcher`] so all providers share **one commensurable scale**
//! (§10.3 — the make-or-break hybrid-list rule). Results are grouped by
//! provider in `sort_weight` order, capped per provider, recency-weighted, and
//! rendered through one flat [`SearchRow`] stream (§10.5).
//!
//! Concrete providers (`PackageProvider`, `ActionProvider`, …) live in consumer
//! crates (§10.4); this module is the frontend-agnostic engine they plug into,
//! carrying **zero rendering dependencies** (§1 `#no-render-dep`).
//!
//! Spec: [PROP-039 §10](../../../../spec/modules/vibe-actions/PROP-039-action-system.md#search-everywhere).

specmark::scope!("spec://vibevm/modules/vibe-actions/PROP-039#search-everywhere");

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

mod matcher;

/// Per-provider result cap on the hybrid **"All"** tab (§10.2).
const CAP_ALL: usize = 15;
/// Per-provider result cap on a single-category tab (§10.2).
const CAP_SINGLE: usize = 30;
/// Score subtracted from a match found in the fallback lane (`extra_haystacks`)
/// so a name/primary match always outranks a description match (§10.3).
const FALLBACK_PENALTY: i64 = 300;
/// Score subtracted from a disabled candidate so it sorts below every enabled
/// one regardless of tier (§10.4 — disabled renders greyed, below the fold).
const DISABLED_PENALTY: i64 = 10_000;
/// The boost the most-recently-selected item earns; older selections decay
/// toward a floor of `1` (§10.2 recency weighting).
const RECENCY_BOOST: i64 = 50;
/// How much the recency boost decays per selection of age.
const RECENCY_DECAY: i64 = 10;

/// A provider's stable identity — a `&'static str` tab/group id (§10.1).
pub type ProviderId = &'static str;

/// The search pattern (borrowed for the lifetime of one query).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Query<'a> {
    /// The raw pattern text the user typed.
    pub text: &'a str,
}

/// The user-modifier state passed to [`SearchProvider::on_selected`] — e.g.
/// Enter vs Alt+Enter (§10.4, "perform vs navigate").
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Modifiers {
    /// Whether Alt was held.
    pub alt: bool,
    /// Whether Shift was held.
    pub shift: bool,
    /// Whether Ctrl was held.
    pub ctrl: bool,
}

/// Whether selecting an item closes the Search Everywhere window (§10.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selected {
    /// Close the window (a navigate/perform action).
    Close,
    /// Keep the window open (e.g. a toggle).
    Stay,
}

/// An opaque handle a provider uses to identify one of its own candidates — the
/// provider gives it meaning; the engine only carries it back on selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemRef(pub usize);

/// A candidate a provider yields (§10.1). The provider may cheaply pre-filter
/// but **must not rank** — the engine scores every candidate with the single
/// matcher, so all providers share one scale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// The provider's handle for this candidate.
    pub item: ItemRef,
    /// The main matchable **and** displayed text (a package name, a field
    /// value, …).
    pub primary: String,
    /// Right-aligned display text (a keybinding, a field owner, …). **Not**
    /// matched by default.
    pub secondary: Option<String>,
    /// Description / synonyms / keywords — the **fallback** match lane
    /// (§10.3), scored below `primary`.
    pub extra_haystacks: Vec<String>,
    /// Whether the item is currently actionable; disabled items rank below and
    /// render greyed.
    pub enabled: bool,
}

/// A scored, resolved result the frontend renders (§10.3, §10.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    /// The rank score on the one shared scale — higher is better.
    pub score: i64,
    /// The primary (matched + displayed) text.
    pub primary: String,
    /// The right-aligned secondary text, if any.
    pub secondary: Option<String>,
    /// Byte ranges into `primary` to highlight. Empty when the match landed in
    /// the hidden fallback lane (§10.3).
    pub match_ranges: Vec<(usize, usize)>,
    /// Whether the item is enabled (greyed if not).
    pub enabled: bool,
    /// The provider that produced the hit.
    pub provider: ProviderId,
    /// The provider's handle, echoed back for [`SearchEngine::on_selected`].
    pub item: ItemRef,
}

/// A searchable universe (PROP-039 §10.1). Concrete providers live in consumer
/// crates and plug in without engine changes (§10.4 — the reserved
/// `StructureProvider` uses this same trait).
///
/// A provider only enumerates candidates; the engine scores and ranks them, so
/// a minimal provider is just an id, presentation, and a candidate list:
///
/// ```
/// use std::collections::HashSet;
/// use vibe_actions::{
///     Candidate, ItemRef, Modifiers, Query, SearchEngine, SearchProvider, Selected,
/// };
///
/// struct Commands;
/// impl SearchProvider for Commands {
///     fn id(&self) -> &'static str { "commands" }
///     fn group_name(&self) -> &str { "Commands" }
///     fn sort_weight(&self) -> i32 { 0 }
///     fn candidates(&self, _query: &Query) -> Vec<Candidate> {
///         vec![Candidate {
///             item: ItemRef(0),
///             primary: "Copy as Markdown".to_owned(),
///             secondary: Some("Ctrl+Shift+C".to_owned()),
///             extra_haystacks: vec!["clipboard".to_owned()],
///             enabled: true,
///         }]
///     }
///     fn on_selected(&self, _item: ItemRef, _mods: Modifiers) -> Selected {
///         Selected::Close
///     }
/// }
///
/// let mut engine = SearchEngine::new(vec![Box::new(Commands)]);
/// let tab = engine.tabs().into_iter().next().unwrap(); // single provider → its own tab
/// let checked = HashSet::from(["commands"]);
/// let rows = engine.search(&Query { text: "copy" }, &tab, &checked);
/// assert!(!rows.is_empty()); // "copy" prefixes "Copy as Markdown"
/// assert_eq!(
///     engine.on_selected("commands", ItemRef(0), Modifiers::default()),
///     Selected::Close,
/// );
/// ```
pub trait SearchProvider {
    /// The provider's stable identity (tab/group id).
    fn id(&self) -> ProviderId;
    /// The tab label and group header text.
    fn group_name(&self) -> &str;
    /// Tab/group order — lower sorts earlier.
    fn sort_weight(&self) -> i32;
    /// Whether the provider gets its own tab (§10.6). Defaults to `true`.
    fn separate_tab(&self) -> bool {
        true
    }
    /// Enumerate candidates for a query. The provider may cheaply pre-filter,
    /// but **must not rank** — the engine scores everything with one matcher.
    fn candidates(&self, query: &Query) -> Vec<Candidate>;
    /// Act on a selected item; return whether to close the window.
    fn on_selected(&self, item: ItemRef, mods: Modifiers) -> Selected;
}

/// A tab in the Search Everywhere window (§10.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tab {
    /// The tab's stable id (`"all"` or a provider id).
    pub id: String,
    /// The tab's display title.
    pub title: String,
    /// The provider ids this tab searches.
    pub providers: Vec<ProviderId>,
    /// Whether this is the hybrid "All" tab (headers + category filter).
    pub is_all: bool,
}

/// A row the frontend draws: a group header (All tab only) or a hit (§10.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchRow {
    /// A group header preceding a provider's rows on the "All" tab.
    Header {
        /// The provider the group belongs to.
        provider: ProviderId,
        /// The group title (the provider's `group_name`).
        title: String,
        /// How many hit rows follow in this group.
        count: usize,
    },
    /// One scored hit.
    Hit(Hit),
}

/// The Search Everywhere engine (PROP-039 §10.2): it owns the providers, scores
/// their candidates on one scale, and tracks selection recency.
pub struct SearchEngine {
    providers: Vec<Box<dyn SearchProvider>>,
    recency: HashMap<(ProviderId, usize), u64>,
    recency_counter: u64,
}

impl std::fmt::Debug for SearchEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchEngine")
            .field("providers", &self.providers.len())
            .field("recency_entries", &self.recency.len())
            .field("recency_counter", &self.recency_counter)
            .finish()
    }
}

impl SearchEngine {
    /// Build an engine over `providers`.
    pub fn new(providers: Vec<Box<dyn SearchProvider>>) -> Self {
        SearchEngine {
            providers,
            recency: HashMap::new(),
            recency_counter: 0,
        }
    }

    /// Build the tab strip (§10.6): an **"All"** tab first (only when there is
    /// more than one provider), then one tab per `separate_tab` provider,
    /// ordered by `sort_weight`.
    pub fn tabs(&self) -> Vec<Tab> {
        let mut ordered: Vec<&dyn SearchProvider> =
            self.providers.iter().map(|b| b.as_ref()).collect();
        ordered.sort_by_key(|p| p.sort_weight());

        let mut tabs = Vec::new();
        if self.providers.len() > 1 {
            tabs.push(Tab {
                id: "all".to_owned(),
                title: "All".to_owned(),
                providers: ordered.iter().map(|p| p.id()).collect(),
                is_all: true,
            });
        }
        for p in &ordered {
            if p.separate_tab() {
                tabs.push(Tab {
                    id: p.id().to_owned(),
                    title: p.group_name().to_owned(),
                    providers: vec![p.id()],
                    is_all: false,
                });
            }
        }
        tabs
    }

    /// Search within `tab`. `enabled_in_all` is the category filter for the
    /// "All" tab — a provider not in the set is skipped; it is ignored for a
    /// single tab. Returns the grouped, capped, rendered rows (§10.2–§10.3):
    /// groups in `sort_weight` order, each sorted by score desc, with a header
    /// per non-empty group on the "All" tab only.
    pub fn search(
        &self,
        query: &Query,
        tab: &Tab,
        enabled_in_all: &HashSet<ProviderId>,
    ) -> Vec<SearchRow> {
        let cap = if tab.is_all { CAP_ALL } else { CAP_SINGLE };
        let mut rows = Vec::new();
        for provider in self.active_providers(tab, enabled_in_all) {
            let mut hits = self.score_provider(provider, query);
            // Empty pattern: keep provider order untouched (§10.2); a real
            // pattern ranks by score with the shorter/lexicographic tie-break.
            if !query.text.is_empty() {
                hits.sort_by(cmp_hits);
            }
            hits.truncate(cap);
            if hits.is_empty() {
                continue;
            }
            if tab.is_all {
                rows.push(SearchRow::Header {
                    provider: provider.id(),
                    title: provider.group_name().to_owned(),
                    count: hits.len(),
                });
            }
            rows.extend(hits.into_iter().map(SearchRow::Hit));
        }
        rows
    }

    /// Dispatch a selection to the owning provider and bump recency (§10.2).
    /// An unknown provider is a no-op that keeps the window open.
    pub fn on_selected(
        &mut self,
        provider: ProviderId,
        item: ItemRef,
        mods: Modifiers,
    ) -> Selected {
        self.recency_counter += 1;
        self.recency
            .insert((provider, item.0), self.recency_counter);
        match self.provider_by_id(provider) {
            Some(p) => p.on_selected(item, mods),
            None => Selected::Stay,
        }
    }

    /// The providers a tab searches, in `sort_weight` order (the "All" tab
    /// honours the `enabled_in_all` category filter).
    fn active_providers(
        &self,
        tab: &Tab,
        enabled_in_all: &HashSet<ProviderId>,
    ) -> Vec<&dyn SearchProvider> {
        let mut result: Vec<&dyn SearchProvider> = Vec::new();
        for &pid in &tab.providers {
            if tab.is_all && !enabled_in_all.contains(&pid) {
                continue;
            }
            if let Some(p) = self.provider_by_id(pid) {
                result.push(p);
            }
        }
        result.sort_by_key(|p| p.sort_weight());
        result
    }

    /// Resolve a provider by its id.
    fn provider_by_id(&self, id: ProviderId) -> Option<&dyn SearchProvider> {
        self.providers
            .iter()
            .find(|p| p.id() == id)
            .map(|b| b.as_ref())
    }

    /// Score every candidate a provider yields, dropping non-matches.
    fn score_provider(&self, provider: &dyn SearchProvider, query: &Query) -> Vec<Hit> {
        provider
            .candidates(query)
            .into_iter()
            .filter_map(|c| self.score_candidate(provider.id(), c, query))
            .collect()
    }

    /// Score one candidate on the shared scale: the `primary` lane, then the
    /// `extra_haystacks` fallback lane (penalised, ranges dropped), plus the
    /// recency boost and the disabled penalty. `None` when nothing matches.
    fn score_candidate(&self, pid: ProviderId, cand: Candidate, query: &Query) -> Option<Hit> {
        if query.text.is_empty() {
            // Empty pattern: every candidate is a zero-score keep, in order.
            return Some(Hit {
                score: 0,
                primary: cand.primary,
                secondary: cand.secondary,
                match_ranges: Vec::new(),
                enabled: cand.enabled,
                provider: pid,
                item: cand.item,
            });
        }

        let primary = matcher::score(query.text, &cand.primary);
        let best_extra = cand
            .extra_haystacks
            .iter()
            .filter_map(|h| matcher::score(query.text, h))
            .map(|(sc, _)| sc - FALLBACK_PENALTY)
            .max();
        let (base, ranges) = match (primary, best_extra) {
            // A primary match at least as good as any fallback: keep its ranges.
            (Some((ps, pr)), Some(es)) if ps >= es => (ps, pr),
            // The fallback lane won: a hidden-field match highlights nothing.
            (Some(_), Some(es)) => (es, Vec::new()),
            (Some((ps, pr)), None) => (ps, pr),
            (None, Some(es)) => (es, Vec::new()),
            (None, None) => return None,
        };

        let mut score = base + self.recency_boost(pid, cand.item.0);
        if !cand.enabled {
            score -= DISABLED_PENALTY;
        }
        Some(Hit {
            score,
            primary: cand.primary,
            secondary: cand.secondary,
            match_ranges: ranges,
            enabled: cand.enabled,
            provider: pid,
            item: cand.item,
        })
    }

    /// The recency boost for an item — full [`RECENCY_BOOST`] for the most
    /// recent selection, decaying by [`RECENCY_DECAY`] per selection of age,
    /// floored at `1` once ever selected; `0` if never selected.
    fn recency_boost(&self, provider: ProviderId, item: usize) -> i64 {
        match self.recency.get(&(provider, item)) {
            Some(&stamp) => {
                let age = self.recency_counter.saturating_sub(stamp) as i64;
                (RECENCY_BOOST - age * RECENCY_DECAY).max(1)
            }
            None => 0,
        }
    }
}

/// Order two hits: score desc, then shorter `primary`, then lexicographic
/// (§10.3). A stable sort therefore preserves provider order on full ties.
fn cmp_hits(a: &Hit, b: &Hit) -> Ordering {
    b.score
        .cmp(&a.score)
        .then_with(|| a.primary.chars().count().cmp(&b.primary.chars().count()))
        .then_with(|| a.primary.cmp(&b.primary))
}

#[cfg(test)]
mod tests;
