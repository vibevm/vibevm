//! The `vibe tree` Search Everywhere providers (PROP-037 §7.3, §13.5): packages
//! by name, every package-card field, and the `vibe.tree` action catalogue. Each
//! materialises **owned** candidates when the window opens (no borrow on the
//! tree), so the [`vibe_actions::search::SearchEngine`] can own them for the
//! window's lifetime. A provider's `on_selected` returns only `Close`/`Stay`; the
//! App applies the reveal / open-card / run effect by `(provider, item)`
//! (`super::apply_effect`), since only the App may mutate the model.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#f1-search");

use vibe_actions::search::{
    Candidate, ItemRef, Modifiers, ProviderId, Query, SearchProvider, Selected,
};

use super::super::state::DisplayMode;
use crate::commands::tree::model::{
    Condition, DeclaredLink, LoadOrigin, LoadType, PackageTree, Source,
};

/// The three provider ids — the stable keys the App dispatches effects on.
pub const PACKAGES: ProviderId = "packages";
pub const FIELDS: ProviderId = "fields";
pub const ACTIONS: ProviderId = "actions";

// ---------------------------------------------------------------------------
// PackageProvider — packages by name (PROP-037 §7.3).
// ---------------------------------------------------------------------------

/// One pre-built package candidate: the tree index plus its matchable text.
struct PkgCand {
    idx: usize,
    id: String,
    load: &'static str,
    extra: Vec<String>,
}

/// Searches packages by their `group/name` id; navigate = reveal the package.
pub struct PackageProvider {
    cands: Vec<PkgCand>,
}

impl PackageProvider {
    /// Materialise one candidate per package from the analysed tree.
    pub fn build(tree: &PackageTree) -> Self {
        let cands = tree
            .packages
            .iter()
            .enumerate()
            .map(|(idx, p)| PkgCand {
                idx,
                id: p.id.clone(),
                load: load_label(p.load.load_type),
                extra: vec![
                    p.group.clone(),
                    p.name.clone(),
                    p.version.clone(),
                    p.kind.clone(),
                ],
            })
            .collect();
        Self { cands }
    }
}

impl SearchProvider for PackageProvider {
    fn id(&self) -> ProviderId {
        PACKAGES
    }
    fn group_name(&self) -> &str {
        "Packages"
    }
    fn sort_weight(&self) -> i32 {
        100
    }
    fn candidates(&self, _query: &Query) -> Vec<Candidate> {
        self.cands
            .iter()
            .map(|c| Candidate {
                item: ItemRef(c.idx),
                primary: c.id.clone(),
                secondary: Some(c.load.to_string()),
                extra_haystacks: c.extra.clone(),
                enabled: true,
            })
            .collect()
    }
    fn on_selected(&self, _item: ItemRef, _mods: Modifiers) -> Selected {
        Selected::Close
    }
}

// ---------------------------------------------------------------------------
// FieldProvider — inside every field of the package detail cards (PROP-037 §7.3).
// ---------------------------------------------------------------------------

/// One card-field candidate: which package it belongs to plus the field's
/// name and value.
struct FieldCand {
    pkg_idx: usize,
    field: &'static str,
    value: String,
    owner: String,
}

/// Searches inside every detail-card field of every package; navigate = open
/// that package's card. The item is the *package* index (the card shows all its
/// fields).
pub struct FieldProvider {
    cands: Vec<FieldCand>,
}

impl FieldProvider {
    /// Materialise one candidate per (package, non-empty field).
    pub fn build(tree: &PackageTree) -> Self {
        let mut cands = Vec::new();
        for (pkg_idx, p) in tree.packages.iter().enumerate() {
            let mut push = |field: &'static str, value: String| {
                if !value.is_empty() {
                    cands.push(FieldCand {
                        pkg_idx,
                        field,
                        value,
                        owner: p.id.clone(),
                    });
                }
            };
            push("version", p.version.clone());
            push("kind", p.kind.clone());
            push("load", load_label(p.load.load_type).to_string());
            push("origin", origin_label(p.load.origin).to_string());
            if let Some(d) = p.load.declared {
                push("declared", declared_label(d).to_string());
            }
            if let Some(raw) = condition_value(&p.condition) {
                push("condition", raw);
            }
            if let Some(url) = source_value(p.source.as_ref()) {
                push("source", url);
            }
            if let Some(hash) = p.content_hash.clone() {
                push("hash", hash);
            }
            if let Some(boot) = p.load.boot_path.clone() {
                push("boot path", boot);
            }
            for dep in &p.dependencies {
                push("dependency", dep.clone());
            }
        }
        Self { cands }
    }
}

impl SearchProvider for FieldProvider {
    fn id(&self) -> ProviderId {
        FIELDS
    }
    fn group_name(&self) -> &str {
        "Card fields"
    }
    fn sort_weight(&self) -> i32 {
        200
    }
    fn candidates(&self, _query: &Query) -> Vec<Candidate> {
        self.cands
            .iter()
            .map(|c| Candidate {
                item: ItemRef(c.pkg_idx),
                primary: c.value.clone(),
                secondary: Some(format!("{} · {}", c.field, c.owner)),
                extra_haystacks: vec![c.field.to_string(), c.owner.clone()],
                enabled: true,
            })
            .collect()
    }
    fn on_selected(&self, _item: ItemRef, _mods: Modifiers) -> Selected {
        Selected::Close
    }
}

// ---------------------------------------------------------------------------
// ActionProvider — the vibe.tree action catalogue (PROP-037 §13.5).
// ---------------------------------------------------------------------------

/// A context snapshot taken when the window opens — the actions' enablement
/// reads it (PROP-039 §6.2). Kept small and `Copy`.
#[derive(Clone, Copy)]
pub struct TreeCtx {
    pub mode: DisplayMode,
    pub has_pkg_selection: bool,
}

/// A `vibe.tree` action: a stable `action://vibe.tree/<name>` address, a
/// human-readable name + description (PROP-037 §13.4/§13.5), its default key,
/// searchable synonyms, and an enablement predicate over [`TreeCtx`].
pub struct TreeActionSpec {
    pub addr: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub key: &'static str,
    pub synonyms: &'static [&'static str],
    pub enabled: fn(&TreeCtx) -> bool,
}

/// The catalogue. Both the [`ActionProvider`] (candidates) and the App
/// (effect dispatch, by index) index this list — the `ItemRef` is a catalogue
/// index. Each address is a valid `action://vibe.tree/…` (asserted in tests).
pub const TREE_ACTIONS: &[TreeActionSpec] = &[
    TreeActionSpec {
        addr: "action://vibe.tree/ordering.cycle",
        name: "Cycle ordering",
        desc: "Switch between topological and alphabetical row ordering.",
        key: "n",
        synonyms: &["sort", "alphabetical", "topological", "order"],
        enabled: |_| true,
    },
    TreeActionSpec {
        addr: "action://vibe.tree/mode.cycle",
        name: "Cycle display mode",
        desc: "Switch between tree, sub-tables, and tabs display.",
        key: "x",
        synonyms: &["mode", "view", "tabs", "sub-tables", "layout"],
        enabled: |_| true,
    },
    TreeActionSpec {
        addr: "action://vibe.tree/priority.swap",
        name: "Swap static/dynamic priority",
        desc: "Swap whether static or dynamic sorts first in the flat modes.",
        key: "t",
        synonyms: &["static", "dynamic", "priority", "swap"],
        enabled: |c| !matches!(c.mode, DisplayMode::All),
    },
    TreeActionSpec {
        addr: "action://vibe.tree/fold.toggle",
        name: "Fold / unfold selected",
        desc: "Fold or unfold the selected node's subtree.",
        key: "Space",
        synonyms: &["collapse", "expand", "fold", "unfold"],
        enabled: |c| matches!(c.mode, DisplayMode::All) && c.has_pkg_selection,
    },
    TreeActionSpec {
        addr: "action://vibe.tree/fold.all",
        name: "Fold / unfold all",
        desc: "Fold or unfold every node with children.",
        key: "F",
        synonyms: &["collapse all", "expand all"],
        enabled: |c| matches!(c.mode, DisplayMode::All),
    },
    TreeActionSpec {
        addr: "action://vibe.tree/card.open",
        name: "Open details",
        desc: "Open the detail card for the selected package.",
        key: "Enter",
        synonyms: &["details", "card", "inspect"],
        enabled: |c| c.has_pkg_selection,
    },
    TreeActionSpec {
        addr: "action://vibe.tree/tab.next",
        name: "Next tab",
        desc: "Move to the next tab (tabs mode).",
        key: "Tab",
        synonyms: &["forward"],
        enabled: |c| matches!(c.mode, DisplayMode::Tabs),
    },
    TreeActionSpec {
        addr: "action://vibe.tree/tab.prev",
        name: "Previous tab",
        desc: "Move to the previous tab (tabs mode).",
        key: "[",
        synonyms: &["back", "previous"],
        enabled: |c| matches!(c.mode, DisplayMode::Tabs),
    },
    TreeActionSpec {
        addr: "action://vibe.tree/quit",
        name: "Quit",
        desc: "Leave vibe tree.",
        key: "q",
        synonyms: &["exit", "close"],
        enabled: |_| true,
    },
];

/// Searches the action catalogue by name, description, address, and synonyms;
/// selecting runs the action in place.
pub struct ActionProvider {
    ctx: TreeCtx,
}

impl ActionProvider {
    /// Build over a context snapshot (enablement is evaluated per candidate).
    pub fn build(ctx: TreeCtx) -> Self {
        Self { ctx }
    }
}

impl SearchProvider for ActionProvider {
    fn id(&self) -> ProviderId {
        ACTIONS
    }
    fn group_name(&self) -> &str {
        "Actions"
    }
    fn sort_weight(&self) -> i32 {
        300
    }
    fn candidates(&self, _query: &Query) -> Vec<Candidate> {
        TREE_ACTIONS
            .iter()
            .enumerate()
            .map(|(idx, a)| {
                let mut extra: Vec<String> = vec![a.desc.to_string(), a.addr.to_string()];
                extra.extend(a.synonyms.iter().map(|s| s.to_string()));
                Candidate {
                    item: ItemRef(idx),
                    primary: a.name.to_string(),
                    secondary: Some(a.key.to_string()),
                    extra_haystacks: extra,
                    enabled: (a.enabled)(&self.ctx),
                }
            })
            .collect()
    }
    fn on_selected(&self, _item: ItemRef, _mods: Modifiers) -> Selected {
        Selected::Close
    }
}

// ---------------------------------------------------------------------------
// Small field-label helpers (self-contained so no `modal`/`state` visibility
// is needed).
// ---------------------------------------------------------------------------

fn load_label(t: LoadType) -> &'static str {
    match t {
        LoadType::Static => "static",
        LoadType::Dynamic => "dynamic",
        LoadType::None => "none",
    }
}

fn origin_label(o: LoadOrigin) -> &'static str {
    match o {
        LoadOrigin::Declared => "declared",
        LoadOrigin::Suggested => "suggested",
        LoadOrigin::Default => "default",
        LoadOrigin::StaticTransitive => "static-transitive",
        LoadOrigin::WhenForced => "when-forced",
        LoadOrigin::None => "none",
    }
}

fn declared_label(d: DeclaredLink) -> &'static str {
    match d {
        DeclaredLink::Static => "static",
        DeclaredLink::Dynamic => "dynamic",
        DeclaredLink::StaticTransitive => "static-transitive",
        DeclaredLink::StaticHard => "static-hard",
    }
}

fn condition_value(c: &Condition) -> Option<String> {
    if !c.present {
        return None;
    }
    c.raw.clone().filter(|s| !s.is_empty())
}

fn source_value(source: Option<&Source>) -> Option<String> {
    source.and_then(|s| s.url.clone()).filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use vibe_actions::ActionAddr;

    use super::*;

    #[test]
    fn every_tree_action_address_is_a_valid_action_uri() {
        for a in TREE_ACTIONS {
            let addr =
                ActionAddr::parse(a.addr).unwrap_or_else(|e| panic!("bad address {}: {e}", a.addr));
            assert_eq!(addr.to_string(), a.addr, "address round-trips");
        }
    }

    #[test]
    fn action_addresses_are_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for a in TREE_ACTIONS {
            assert!(seen.insert(a.addr), "duplicate address {}", a.addr);
        }
    }

    #[test]
    fn every_action_has_a_nonempty_name_and_description() {
        for a in TREE_ACTIONS {
            assert!(!a.name.is_empty(), "{} has a name", a.addr);
            assert!(!a.desc.is_empty(), "{} has a description", a.addr);
        }
    }

    #[test]
    fn enablement_reads_the_context() {
        let all = TreeCtx {
            mode: DisplayMode::All,
            has_pkg_selection: true,
        };
        let tabs = TreeCtx {
            mode: DisplayMode::Tabs,
            has_pkg_selection: false,
        };
        let fold = TREE_ACTIONS
            .iter()
            .find(|a| a.addr == "action://vibe.tree/fold.toggle")
            .unwrap();
        assert!((fold.enabled)(&all), "fold enabled in All with a selection");
        assert!(!(fold.enabled)(&tabs), "fold disabled outside All");
        let next = TREE_ACTIONS
            .iter()
            .find(|a| a.addr == "action://vibe.tree/tab.next")
            .unwrap();
        assert!((next.enabled)(&tabs), "tab.next enabled in Tabs");
        assert!(!(next.enabled)(&all), "tab.next disabled outside Tabs");
    }
}
