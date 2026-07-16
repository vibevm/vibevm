//! The `vibe prefs` Search Everywhere provider (PROP-041 §7 `#settings-search`)
//! — a [`SearchProvider`] over the page registry + schema that enumerates every
//! declared setting key as a [`Candidate`]. The matchable haystack is the key's
//! path + display name (its owning page) + description + any synonyms; the
//! provider owns a parallel `Vec<(page_id, key)>` so a confirmed selection can
//! open the owning page focused on the field.
//!
//! **`#deprecated-discoverable`**: deprecated keys stay searchable and surface
//! the `replaced_by` migration path (from [`KeyMeta::deprecated`]) as the
//! candidate's secondary line, so a user looking for an old name is guided to
//! the new one. A present-but-deprecated key is still `enabled = true` (the
//! user can still open it); the secondary just carries the migration hint.
//!
//! The provider materialises **owned** candidates when the window opens (no
//! borrow on the registry/schema), so the [`SearchEngine`] can own them for the
//! window's lifetime. `on_selected` returns only `Close`/`Stay`; the App opens
//! the page + focuses the field by the candidate's index (`super::confirm`),
//! since only the App may mutate the model. This mirrors the tree TUI's
//! `providers.rs` shape.

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-041#settings-search");

use vibe_actions::search::{
    Candidate, ItemRef, Modifiers, ProviderId, Query, SearchProvider, Selected,
};
use vibe_actions::{ActionAddr, Ctx, Registry};
use vibe_settings::schema::Schema;

use crate::commands::prefs::tui::catalogue::{PrefsActionCtx, key_for};
use crate::commands::prefs::tui::registry::PageRegistry;

/// The provider id for setting-key candidates — the stable key the App
/// dispatches the open effect on.
pub(crate) const SETTINGS: ProviderId = "settings";

/// The provider id for the `vibe.prefs` action catalogue candidates — the App
/// dispatches a confirmed action by its address (`super::confirm`).
pub(crate) const ACTIONS: ProviderId = "actions";

/// One materialised setting candidate: the owning page + the key, plus the
/// matchable text and the optional deprecation hint.
struct SettingCand {
    page_id: String,
    page_name: String,
    key: String,
    description: String,
    /// The `replaced_by` migration path, when the key is deprecated.
    replaced_by: Option<String>,
}

/// Searches every declared setting key by its path, display name, description,
/// and synonyms; navigate = open the owning page focused on the field. A
/// deprecated key surfaces its `replaced_by` (PROP-041 §7
/// `#deprecated-discoverable`).
pub(crate) struct SettingsProvider {
    cands: Vec<SettingCand>,
}

impl SettingsProvider {
    /// Materialise one candidate per key declared in the registry, resolving
    /// each key's [`KeyMeta`] from the schema for its description + deprecation.
    /// Keys the schema does not know (a registry/schema drift) are skipped —
    /// the registry is the enumerable source (PROP-041 §2
    /// `#registry-is-introspectable`) but the schema owns the text.
    pub(crate) fn build(registry: &PageRegistry, schema: &Schema) -> Self {
        let mut cands = Vec::new();
        for page in registry.pages() {
            // Only leaf pages (those carrying keys) contribute candidates; a
            // group declaration has an empty `keys` vec and no form to open.
            if page.keys.is_empty() {
                continue;
            }
            for key in &page.keys {
                let Some(meta) = schema.get(key) else {
                    continue;
                };
                let replaced_by = meta.deprecated.as_ref().and_then(|d| d.replaced_by.clone());
                cands.push(SettingCand {
                    page_id: page.id.clone(),
                    page_name: page.display_name.clone(),
                    key: key.clone(),
                    description: meta.description.clone(),
                    replaced_by,
                });
            }
        }
        Self { cands }
    }

    /// The `(page_id, key)` pair for candidate `item`, so the App can open the
    /// owning page + focus the field on confirm.
    pub(crate) fn target(&self, item: ItemRef) -> Option<(&str, &str)> {
        self.cands
            .get(item.0)
            .map(|c| (c.page_id.as_str(), c.key.as_str()))
    }

    /// The number of materialised candidates (used to capture the parallel
    /// target list at open time, before ownership moves into the engine).
    pub(crate) fn len(&self) -> usize {
        self.cands.len()
    }
}

impl SearchProvider for SettingsProvider {
    fn id(&self) -> ProviderId {
        SETTINGS
    }
    fn group_name(&self) -> &str {
        "Settings"
    }
    fn sort_weight(&self) -> i32 {
        100
    }
    fn candidates(&self, _query: &Query) -> Vec<Candidate> {
        self.cands
            .iter()
            .enumerate()
            .map(|(idx, c)| {
                // The matchable haystack: the key path, the owning page's
                // display name, and the key's description. A deprecated key also
                // exposes its replacement path so a search by the old name
                // still lands.
                let mut extra: Vec<String> = vec![c.key.clone(), c.page_name.clone()];
                extra.push(c.description.clone());
                if let Some(rep) = &c.replaced_by {
                    extra.push(rep.clone());
                }
                // The secondary line: the key path (where it lives), or — for a
                // deprecated key — the migration hint so the result reads
                // "replaced by <new>" at a glance.
                let secondary = match &c.replaced_by {
                    Some(rep) => format!("\u{2192} {}", rep),
                    None => format!("\u{00b7} {}", c.page_name),
                };
                // The primary line: the page's display name (what the user
                // sees in the tree) so a result reads "Palette" rather than the
                // raw dotted path; the key is in the haystack either way.
                Candidate {
                    item: ItemRef(idx),
                    primary: c.page_name.clone(),
                    secondary: Some(secondary),
                    extra_haystacks: extra,
                    enabled: true,
                }
            })
            .collect()
    }
    fn on_selected(&self, _item: ItemRef, _mods: Modifiers) -> Selected {
        Selected::Close
    }
}

// ---------------------------------------------------------------------------
// PrefsActionProvider — the vibe.prefs action catalogue (PROP-041 §8
// `#commands-are-actions`, mirrored from `tree::tui::search::providers`).
// ---------------------------------------------------------------------------

/// One materialised action candidate — resolved from a real `vibe_actions`
/// [`vibe_actions::Action`] at open: presentation, synonyms, and the enablement
/// verdict (with its "why disabled" reason).
struct ActionCand {
    name: String,
    desc: String,
    key: String,
    synonyms: Vec<String>,
    enabled: bool,
    reason: Option<String>,
}

/// Searches the `vibe.prefs` action [`Registry`] by name, description, address,
/// and synonyms; selecting runs the action in place (the App dispatches by
/// address). Enablement (and its "why disabled" reason) comes from each real
/// Action's predicate over the [`PrefsActionCtx`] snapshot (PROP-039 §6.2).
pub(crate) struct PrefsActionProvider {
    cands: Vec<ActionCand>,
}

impl PrefsActionProvider {
    /// Enumerate the registry, resolving each action's presentation + enablement
    /// over `ctx`. Returns the provider and the parallel address list — in the
    /// same order, so an `ItemRef` indexes both — the App dispatches by address.
    pub(crate) fn build(registry: &Registry, ctx: PrefsActionCtx) -> (Self, Vec<ActionAddr>) {
        let vctx = Ctx::new().with(ctx);
        let mut cands = Vec::new();
        let mut addrs = Vec::new();
        for action in registry.iter() {
            let en = action.evaluate(&vctx);
            let addr = action.addr().clone();
            cands.push(ActionCand {
                name: action.presentation().name().default_en().to_string(),
                desc: action.presentation().description().default_en().to_string(),
                key: key_for(&addr.to_string()).to_string(),
                synonyms: action.search_meta().synonyms().to_vec(),
                enabled: en.enabled,
                reason: en.reason.map(|r| r.as_str().to_string()),
            });
            addrs.push(addr);
        }
        (Self { cands }, addrs)
    }
}

impl SearchProvider for PrefsActionProvider {
    fn id(&self) -> ProviderId {
        ACTIONS
    }
    fn group_name(&self) -> &str {
        "Actions"
    }
    fn sort_weight(&self) -> i32 {
        200
    }
    fn candidates(&self, _query: &Query) -> Vec<Candidate> {
        self.cands
            .iter()
            .enumerate()
            .map(|(idx, c)| {
                let mut extra = vec![c.desc.clone()];
                extra.extend(c.synonyms.clone());
                // A disabled action surfaces its "why disabled" reason; an
                // enabled one shows its keybinding.
                let secondary = match (c.enabled, &c.reason) {
                    (false, Some(reason)) => Some(reason.clone()),
                    _ => Some(c.key.clone()),
                };
                Candidate {
                    item: ItemRef(idx),
                    primary: c.name.clone(),
                    secondary,
                    extra_haystacks: extra,
                    enabled: c.enabled,
                }
            })
            .collect()
    }
    fn on_selected(&self, _item: ItemRef, _mods: Modifiers) -> Selected {
        Selected::Close
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::commands::prefs::tui::registry::PageDecl;
    use vibe_settings::schema::{Deprecation, KeyMeta, KeyType, Scope};

    fn schema_with(palette_default: &str) -> Schema {
        let mut s = Schema::new();
        s.register(
            KeyMeta::new(
                "vibe.tree.palette",
                KeyType::String,
                Scope::User,
                "the Vibe Tree palette",
            )
            .unwrap()
            .with_default(toml::Value::String(palette_default.into())),
        )
        .unwrap();
        s.register(
            KeyMeta::new(
                "vibe.tree.mode",
                KeyType::String,
                Scope::User,
                "the display mode",
            )
            .unwrap(),
        )
        .unwrap();
        // A deprecated key — still discoverable, surfaces replaced_by.
        s.register(
            KeyMeta::new(
                "node.sort",
                KeyType::String,
                Scope::User,
                "the old sort key",
            )
            .unwrap()
            .with_deprecation(Deprecation::with_replacement("use tree.sort", "tree.sort")),
        )
        .unwrap();
        s
    }

    fn registry() -> PageRegistry {
        PageRegistry::from(vec![
            PageDecl::new("appearance", "Appearance", "group").with_keys(&[]),
            PageDecl::new("palette", "Palette", "the colour palette")
                .with_keys(&["vibe.tree.palette", "node.sort"]),
            PageDecl::new("mode", "Display mode", "the display mode")
                .with_keys(&["vibe.tree.mode"]),
        ])
    }

    #[test]
    fn build_enumerates_one_candidate_per_declared_key() {
        let provider = SettingsProvider::build(&registry(), &schema_with("rose-pine"));
        // Three keys across two leaf pages (the Appearance group has none).
        assert_eq!(provider.cands.len(), 3);
        let keys: Vec<&str> = provider.cands.iter().map(|c| c.key.as_str()).collect();
        assert!(keys.contains(&"vibe.tree.palette"));
        assert!(keys.contains(&"vibe.tree.mode"));
        assert!(keys.contains(&"node.sort"));
    }

    #[test]
    fn candidates_match_by_key_path_display_name_and_description() {
        let provider = SettingsProvider::build(&registry(), &schema_with("rose-pine"));
        let cands = provider.candidates(&Query { text: "" });
        // Every candidate carries the key + page name + description as haystacks.
        let palette = cands
            .iter()
            .find(|c| c.extra_haystacks.iter().any(|h| h == "vibe.tree.palette"))
            .expect("palette candidate present");
        assert!(palette.extra_haystacks.iter().any(|h| h == "Palette"));
        assert!(
            palette
                .extra_haystacks
                .iter()
                .any(|h| h.contains("palette"))
        );
    }

    #[test]
    fn a_deprecated_key_surfaces_its_replaced_by() {
        let provider = SettingsProvider::build(&registry(), &schema_with("rose-pine"));
        let node_sort = provider
            .cands
            .iter()
            .find(|c| c.key == "node.sort")
            .expect("the deprecated key is still enumerated");
        assert_eq!(
            node_sort.replaced_by.as_deref(),
            Some("tree.sort"),
            "the replaced_by migration path is carried"
        );
        // And it appears as the candidate's secondary line.
        let cands = provider.candidates(&Query { text: "" });
        let hit = cands
            .iter()
            .find(|c| c.extra_haystacks.iter().any(|h| h == "node.sort"))
            .expect("present");
        assert!(
            hit.secondary.as_deref().unwrap_or("").contains("tree.sort"),
            "the secondary surfaces the migration target"
        );
    }

    #[test]
    fn target_resolves_the_owning_page_and_key_for_a_selection() {
        let provider = SettingsProvider::build(&registry(), &schema_with("rose-pine"));
        // Find the palette candidate's index.
        let idx = provider
            .cands
            .iter()
            .position(|c| c.key == "vibe.tree.mode")
            .unwrap();
        let (page, key) = provider.target(ItemRef(idx)).unwrap();
        assert_eq!(page, "mode", "the owning page is the display-mode page");
        assert_eq!(key, "vibe.tree.mode");
    }

    #[test]
    fn the_action_provider_enumerates_the_catalogue_with_enablement() {
        use crate::commands::prefs::tui::catalogue::build_registry;
        let reg = build_registry();
        let ctx = PrefsActionCtx {
            at_base: true,
            page_open: false,
            leaf_selected: true,
            form_editable: false,
            has_blocking_error: false,
        };
        let (provider, addrs) = PrefsActionProvider::build(&reg, ctx);
        assert_eq!(provider.cands.len(), addrs.len());
        assert!(!provider.cands.is_empty(), "the catalogue enumerates");
        // At base with a leaf, page.open is enabled and carries its keybinding.
        let page_open = provider
            .cands
            .iter()
            .position(|c| c.name == "Open page")
            .expect("Open page is in the catalogue");
        assert!(provider.cands[page_open].enabled);
        assert_eq!(provider.cands[page_open].key, "Enter");
        // Apply is disabled at base (no form) and surfaces a reason.
        let apply = provider
            .cands
            .iter()
            .find(|c| c.name == "Apply")
            .expect("Apply is in the catalogue");
        assert!(!apply.enabled);
        assert!(apply.reason.is_some(), "a disabled action gives a reason");
    }
}
