//! The built-in page declarations over the `vibe.tree.*` preference keys
//! (PROP-041 §2, built on PROP-040's data layer + PROP-037 §9's declared
//! schema). The settings the `vibe tree` TUI already owns — palette, tier,
//! mode, sort, shape, static-first — each become a page in the registry,
//! grouped under an "Appearance" / "Tree" pair so the user finds them in the
//! settings tree.
//!
//! This is the one place the settings TUI's page set is declared; the registry
//! ([`super::registry`]) turns it into the tree, and adding a page means adding
//! a declaration here — no further wiring (the registry is introspectable,
//! PROP-041 §2 `#registry-is-introspectable`). Each page's `keys` are the
//! `vibe.tree.*` dotted paths declared in the tree TUI's settings cell
//! ([`crate::commands::tree::tui::settings`]); the origin hint (§3
//! `#tree-shows-origin-hint`) reads them through `ResolvedPrefs::inspect`.

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-041#registry");

use super::registry::{PageDecl, PageRegistry};

// ── the group ids (stable, #stable-id-law) ───────────────────────────────────

/// The "Appearance" group — presentation keys (palette + rendering tier).
pub const GROUP_APPEARANCE: &str = "vibe.prefs.appearance";

/// The "Tree" group — `vibe tree` behaviour keys (mode / sort / shape / order).
pub const GROUP_TREE: &str = "vibe.prefs.tree";

// ── the vibe.tree.* key paths (mirrors tree/tui/settings consts) ─────────────

/// The palette key (mirrors `tree::tui::settings::KEY_PALETTE`).
const KEY_PALETTE: &str = "vibe.tree.palette";
/// The tier-override key.
const KEY_TIER: &str = "vibe.tree.tier";
/// The display-mode key.
const KEY_MODE: &str = "vibe.tree.mode";
/// The row-ordering key.
const KEY_SORT: &str = "vibe.tree.sort";
/// The tree-shape key.
const KEY_SHAPE: &str = "vibe.tree.shape";
/// The static-first block-order key.
const KEY_STATIC_FIRST: &str = "vibe.tree.static-first";

/// Declare the built-in page set over `vibe.tree.*` and return the registry
/// (PROP-041 §2). Two groups — "Appearance" (palette + tier) and "Tree" (mode +
/// sort + shape + static-first) — each carrying leaf pages whose `keys` point
/// at the underlying preference paths. All pages are [`PageScope::Application`]
/// (the `vibe.tree.*` keys are all `Scope::User` in PROP-040 §7, so they show
/// in every session).
pub fn builtin_registry() -> PageRegistry {
    PageRegistry::from(builtin_pages())
}

/// The built-in page declarations, in a stable registration order (groups
/// before their children so the registry's parent-resolution sees them).
pub fn builtin_pages() -> Vec<PageDecl> {
    vec![
        // ── Appearance group ────────────────────────────────────────────────
        PageDecl::new(
            GROUP_APPEARANCE,
            "Appearance",
            "How the Vibe Tree looks — colour palette and rendering richness.",
        )
        .with_weight(10),
        PageDecl::new(
            "vibe.prefs.appearance.palette",
            "Palette",
            "The colour palette (rose-pine, catppuccin-mocha, …).",
        )
        .with_parent(GROUP_APPEARANCE)
        .with_weight(10)
        .with_keys(&[KEY_PALETTE]),
        PageDecl::new(
            "vibe.prefs.appearance.tier",
            "Rendering tier",
            "Colour-depth override (0=mono … 3=truecolour); absent = auto-detect.",
        )
        .with_parent(GROUP_APPEARANCE)
        .with_weight(20)
        .with_keys(&[KEY_TIER]),
        // ── Tree group ──────────────────────────────────────────────────────
        PageDecl::new(
            GROUP_TREE,
            "Tree",
            "How the Vibe Tree lays out and orders its rows.",
        )
        .with_weight(20),
        PageDecl::new(
            "vibe.prefs.tree.mode",
            "Display mode",
            "The display mode (all, sub-tables, tabs).",
        )
        .with_parent(GROUP_TREE)
        .with_weight(10)
        .with_keys(&[KEY_MODE]),
        PageDecl::new(
            "vibe.prefs.tree.sort",
            "Row ordering",
            "The row ordering (topological, alphabetical).",
        )
        .with_parent(GROUP_TREE)
        .with_weight(20)
        .with_keys(&[KEY_SORT]),
        PageDecl::new(
            "vibe.prefs.tree.shape",
            "Tree shape",
            "The forest shape (members-as-roots, load-type-forest, pruned-tree).",
        )
        .with_parent(GROUP_TREE)
        .with_weight(30)
        .with_keys(&[KEY_SHAPE]),
        PageDecl::new(
            "vibe.prefs.tree.static-first",
            "Block order",
            "Whether `static` sorts before `dynamic` in the partitioned modes.",
        )
        .with_parent(GROUP_TREE)
        .with_weight(40)
        .with_keys(&[KEY_STATIC_FIRST]),
    ]
}

#[cfg(test)]
mod tests {
    use super::super::registry::{OTHER_GROUP_ID, PageScope};
    use super::*;

    #[test]
    fn builtin_registry_has_two_groups_and_six_leaf_pages() {
        let r = builtin_registry();
        // 2 group declarations + 6 leaf pages = 8 declarations.
        assert_eq!(r.len(), 8);
        let leaves = r.pages().iter().filter(|d| d.parent_id.is_some()).count();
        assert_eq!(leaves, 6, "six leaf pages under the two groups");
    }

    #[test]
    fn builtin_tree_resolves_two_groups_with_their_children() {
        let r = builtin_registry();
        let roots = r.tree(true);
        // No unresolved parents → no synthetic "Other".
        assert!(
            !roots.iter().any(|n| n.id == OTHER_GROUP_ID),
            "built-in pages resolve cleanly, no Other group"
        );
        let group_ids: Vec<&str> = roots.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(group_ids, [GROUP_APPEARANCE, GROUP_TREE]);
        // Appearance has palette + tier; Tree has mode + sort + shape + static-first.
        let appearance = roots.iter().find(|n| n.id == GROUP_APPEARANCE).unwrap();
        assert_eq!(appearance.children.len(), 2);
        let tree = roots.iter().find(|n| n.id == GROUP_TREE).unwrap();
        assert_eq!(tree.children.len(), 4);
    }

    #[test]
    fn each_leaf_page_carries_its_preference_keys() {
        let r = builtin_registry();
        let palette = r
            .pages()
            .iter()
            .find(|d| d.id == "vibe.prefs.appearance.palette")
            .unwrap();
        assert_eq!(palette.keys, ["vibe.tree.palette"]);
    }

    #[test]
    fn builtin_pages_are_all_application_scoped() {
        // vibe.tree.* keys are Scope::User (PROP-040 §7) → the pages show in
        // every session, including a no-project (L1-only) session.
        let r = builtin_registry();
        assert!(
            r.pages().iter().all(|d| d.scope == PageScope::Application),
            "every built-in page is application-scoped"
        );
        // The tree is identical with/without a project.
        assert_eq!(r.tree(true).len(), r.tree(false).len());
    }
}
