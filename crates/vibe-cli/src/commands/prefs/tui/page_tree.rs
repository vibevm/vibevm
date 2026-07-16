//! The settings tree widget — the left pane (PROP-041 §3 `#tree-widget`).
//! Renders the page hierarchy (groups → pages) through the same visual language
//! as the `vibe tree` TUI: `│├└─` connectors, `▾`/`▸` fold glyphs, theme
//! colours, selection highlight, and vertical scroll. Every glyph comes from
//! [`crate::commands::tree::tui::theme::Glyphs`] (no hardcoded ASCII).
//!
//! This is a small dedicated flatten over the page registry's [`PageNode`]
//! tree — it is NOT the `PackageTree` walk the tree TUI runs. The data is a
//! page registry, not a dependency graph, but the connector/glyph approach is
//! identical so the two trees read as one system (PROP-041 §1
//! `#built-on-tree-tui`). `↑`/`↓` move, `←`/`→` fold/expand a group, `Enter`
//! opens the focused leaf page.
//!
//! Each leaf page row carries an **origin hint** glyph + the winning layer when
//! one of its keys is shadowed (§3 `#tree-shows-origin-hint`) — read through
//! `ResolvedPrefs::inspect`, so the surface owns no preference logic (§1
//! `#surface-not-engine`).

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-041#tree-widget");

use std::collections::BTreeSet;

use specmark::spec;
use vibe_settings::resolver::{Origin, ResolvedPrefs};

use crate::commands::tree::tui::theme::Glyphs;

use super::registry::PageNode;

/// One flattened, rendered settings-tree row. Owns its drawn label so the
/// derived list outlives any borrow during a render pass.
#[derive(Debug, Clone)]
pub struct PageRow {
    /// The page id (or the synthetic group id), the join-key for selection.
    pub id: String,
    /// Whether this row is a group (foldable) or a leaf page (openable).
    pub is_group: bool,
    /// The drawn label: connector prefix + fold glyph (for a group) + display
    /// name + origin hint (for a shadowed leaf).
    pub label: String,
    /// The winning layer for this page, when one of its keys is shadowed (§3
    /// `#tree-shows-origin-hint`). `None` for groups and pages whose every key
    /// is still at its built-in default.
    pub origin_hint: Option<Origin>,
}

impl PageRow {
    /// Whether this row is an openable leaf page (not a group, not a header).
    pub fn is_openable(&self) -> bool {
        !self.is_group
    }
}

/// Flatten the page hierarchy into visible rows given a fold set (PROP-041 §3
/// `#tree-widget`). Connectors and fold glyphs come from `glyphs` (the theme
/// vocabulary); an origin hint is stamped on a leaf page when one of its keys
/// is shadowed (resolved above the built-in default layer).
///
/// `prefs` is read-only here — the surface never mutates preferences (§1
/// `#surface-not-engine`); it only reads provenance through `inspect` to show
/// where a value is coming from.
#[spec(implements = "spec://vibevm/modules/vibe-settings/PROP-041#tree-widget")]
pub fn flatten(
    roots: &[PageNode],
    folded: &BTreeSet<String>,
    prefs: &ResolvedPrefs,
    glyphs: &Glyphs,
) -> Vec<PageRow> {
    let mut rows = Vec::new();
    let n = roots.len();
    for (i, node) in roots.iter().enumerate() {
        walk(node, "", i + 1 == n, true, folded, prefs, glyphs, &mut rows);
    }
    rows
}

/// Depth-first walk producing rows. A group carries a fold glyph and recurses
/// when unfolded; a leaf carries its display name + an origin hint when one of
/// its keys is shadowed. Connectors are the same `│├└─` set the tree TUI uses
/// — a root carries no connector; every child gets a `├─`/`└─` lead on top of
/// the accumulated vertical-bar prefix. Mirrors `tree::tui::flatten::walk`.
#[allow(clippy::too_many_arguments)] // matches tree::tui::flatten::walk — a fixed DFS signature.
fn walk(
    node: &PageNode,
    prefix: &str,
    is_last: bool,
    is_root: bool,
    folded: &BTreeSet<String>,
    prefs: &ResolvedPrefs,
    glyphs: &Glyphs,
    rows: &mut Vec<PageRow>,
) {
    // A root carries no connector; every deeper node gets a `├─`/`└─` lead on
    // top of the accumulated vertical-bar prefix.
    let connector = if is_root {
        String::new()
    } else if is_last {
        format!("{prefix}{}\u{2500} ", glyphs.tree_corner)
    } else {
        format!("{prefix}{}\u{2500} ", glyphs.tree_branch)
    };

    let is_folded = folded.contains(&node.id);
    // The fold indicator is shown only on a group; a leaf carries nothing (it
    // opens with Enter). Glyphs come from the theme vocabulary (PROP-037
    // §2.2.2): ▾/▸ Tier ≥ 1, +/- Tier 0 — never a hardcoded ASCII literal.
    let indicator = if node.is_group {
        if is_folded {
            format!("{} ", glyphs.fold_collapsed)
        } else {
            format!("{} ", glyphs.fold_expanded)
        }
    } else {
        String::new()
    };

    // The origin hint: the highest-precedence layer shadowing any of this
    // page's keys (§3 #tree-shows-origin-hint). Groups carry no hint.
    let origin_hint = if node.is_group {
        None
    } else {
        winning_origin(node, prefs)
    };
    let hint_str = origin_hint
        .map(|o| format!("  [{}]", o.label()))
        .unwrap_or_default();

    rows.push(PageRow {
        id: node.id.clone(),
        is_group: node.is_group,
        label: format!("{connector}{indicator}{}{hint_str}", node.display_name),
        origin_hint,
    });

    if !node.is_group || is_folded {
        return;
    }

    // Children extend the prefix with a vertical bar or a blank gutter — the
    // same scheme as the tree TUI's flatten. A root's children start at column 0
    // (the child's own `├─`/`└─` is its lead); deeper levels indent.
    let child_prefix = if is_root {
        String::new()
    } else if is_last {
        format!("{prefix}   ")
    } else {
        format!("{prefix}{}  ", glyphs.tree_vertical)
    };
    let n = node.children.len();
    for (i, child) in node.children.iter().enumerate() {
        let child_last = i + 1 == n;
        walk(
            child,
            &child_prefix,
            child_last,
            false,
            folded,
            prefs,
            glyphs,
            rows,
        );
    }
}

/// The highest-precedence [`Origin`] shadowing any of this page's keys, or
/// `None` when every key is still at its built-in default (PROP-041 §3
/// `#tree-shows-origin-hint`). Reads through `ResolvedPrefs::inspect` (PROP-040
/// §5) — the surface owns no merge logic.
fn winning_origin(node: &PageNode, prefs: &ResolvedPrefs) -> Option<Origin> {
    let page = node.page.as_ref()?;
    let mut winner: Option<Origin> = None;
    for key in &page.keys {
        if let Some(iv) = prefs.inspect(key)
            && iv.origin != Origin::Default
            && winner.is_none_or(|w| iv.origin > w)
        {
            winner = Some(iv.origin);
        }
    }
    winner
}

#[cfg(test)]
mod tests {
    use super::super::registry::{PageDecl, PageRegistry};
    use super::*;
    use vibe_settings::loader::LayeredRaw;
    use vibe_settings::resolver::resolve;
    use vibe_settings::schema::{KeyMeta, KeyType, Schema, Scope};

    fn glyph_set() -> Glyphs {
        Glyphs::rich()
    }

    /// A two-group registry: appearance(palette, tier) + tree(mode).
    fn fixture_registry() -> PageRegistry {
        PageRegistry::from(vec![
            PageDecl::new("appearance", "Appearance", "appearance group").with_weight(10),
            PageDecl::new("palette", "Palette", "the palette")
                .with_parent("appearance")
                .with_weight(10)
                .with_keys(&["vibe.tree.palette"]),
            PageDecl::new("tier", "Tier", "the tier")
                .with_parent("appearance")
                .with_weight(20)
                .with_keys(&["vibe.tree.tier"]),
            PageDecl::new("tree", "Tree", "tree group").with_weight(20),
            PageDecl::new("mode", "Mode", "the mode")
                .with_parent("tree")
                .with_weight(10)
                .with_keys(&["vibe.tree.mode"]),
        ])
    }

    /// A schema over the three keys, each with a built-in default.
    fn fixture_schema() -> Schema {
        let mut s = Schema::new();
        s.register(
            KeyMeta::new("vibe.tree.palette", KeyType::String, Scope::User, "palette")
                .unwrap()
                .with_default(toml::Value::String("rose-pine".into())),
        )
        .unwrap();
        s.register(
            KeyMeta::new("vibe.tree.tier", KeyType::Int, Scope::User, "tier")
                .unwrap()
                .with_default(toml::Value::Integer(3)),
        )
        .unwrap();
        s.register(
            KeyMeta::new("vibe.tree.mode", KeyType::String, Scope::User, "mode")
                .unwrap()
                .with_default(toml::Value::String("all".into())),
        )
        .unwrap();
        s
    }

    fn resolved_default() -> ResolvedPrefs {
        resolve(
            LayeredRaw::default(),
            &fixture_schema(),
            toml::Table::new(),
            toml::Table::new(),
        )
    }

    fn resolved_with_l2_override() -> ResolvedPrefs {
        let l2: toml::Table = toml::from_str(r#"vibe.tree.palette = "catppuccin-mocha""#).unwrap();
        resolve(
            LayeredRaw {
                l1: toml::Table::new(),
                l2,
                l3: toml::Table::new(),
            },
            &fixture_schema(),
            toml::Table::new(),
            toml::Table::new(),
        )
    }

    fn row_labels(rows: &[PageRow]) -> Vec<String> {
        rows.iter().map(|r| r.label.clone()).collect()
    }

    #[test]
    fn flatten_produces_groups_then_children_with_connectors() {
        let r = fixture_registry();
        let prefs = resolved_default();
        let rows = flatten(&r.tree(true), &BTreeSet::new(), &prefs, &glyph_set());
        let labels = row_labels(&rows);
        // appearance (expanded) + palette + tier + tree (expanded) + mode.
        assert_eq!(rows.len(), 5);
        // The two groups carry the expanded glyph.
        let g = glyph_set();
        assert!(
            labels[0].contains(g.fold_expanded),
            "appearance shows the expanded glyph: {}",
            labels[0]
        );
        // Children carry a connector (├ or └).
        assert!(
            labels[1].contains(g.tree_branch) || labels[1].contains(g.tree_corner),
            "palette carries a connector: {}",
            labels[1]
        );
    }

    #[test]
    fn folding_a_group_hides_its_children_and_shows_collapsed_glyph() {
        let r = fixture_registry();
        let prefs = resolved_default();
        let mut folded = BTreeSet::new();
        folded.insert("appearance".to_string());
        let rows = flatten(&r.tree(true), &folded, &prefs, &glyph_set());
        // appearance (folded) + tree (expanded) + mode = 3 rows (palette, tier
        // hidden).
        assert_eq!(rows.len(), 3);
        let g = glyph_set();
        assert!(
            rows[0].label.contains(g.fold_collapsed),
            "folded group shows the collapsed glyph"
        );
        assert_eq!(rows[0].id, "appearance");
    }

    #[test]
    fn no_ascii_connectors_in_rich_glyphs() {
        // #built-on-tree-tui: every glyph via theme.glyphs(); rich set has no
        // ASCII `+`/`|`/`-` tree connectors.
        let r = fixture_registry();
        let prefs = resolved_default();
        let rows = flatten(&r.tree(true), &BTreeSet::new(), &prefs, &glyph_set());
        let g = glyph_set();
        assert_ne!(g.tree_branch, "+");
        assert_ne!(g.tree_corner, "+");
        assert_ne!(g.tree_vertical, "|");
        // A group row uses the unicode fold glyph, not `+`/`-`.
        assert!(rows[0].label.contains('▾') || rows[0].label.contains('▸'));
    }

    #[test]
    fn origin_hint_appears_when_a_key_is_shadowed() {
        // L2 overrides palette → the Palette page shows [L2].
        let r = fixture_registry();
        let prefs = resolved_with_l2_override();
        let rows = flatten(&r.tree(true), &BTreeSet::new(), &prefs, &glyph_set());
        let palette_row = rows.iter().find(|r| r.id == "palette").unwrap();
        assert_eq!(palette_row.origin_hint, Some(Origin::L2));
        assert!(
            palette_row.label.contains("[L2]"),
            "the origin hint is stamped on the label: {}",
            palette_row.label
        );
    }

    #[test]
    fn no_origin_hint_when_every_key_is_at_default() {
        let r = fixture_registry();
        let prefs = resolved_default();
        let rows = flatten(&r.tree(true), &BTreeSet::new(), &prefs, &glyph_set());
        // No leaf carries an origin hint when nothing is shadowed.
        assert!(
            rows.iter()
                .filter(|r| !r.is_group)
                .all(|r| r.origin_hint.is_none()),
            "no origin hint when every key is at its default"
        );
    }

    #[test]
    fn groups_carry_no_origin_hint() {
        let r = fixture_registry();
        let prefs = resolved_with_l2_override();
        let rows = flatten(&r.tree(true), &BTreeSet::new(), &prefs, &glyph_set());
        assert!(
            rows.iter()
                .filter(|r| r.is_group)
                .all(|r| r.origin_hint.is_none())
        );
    }

    #[test]
    fn is_openable_distinguishes_leaves_from_groups() {
        let r = fixture_registry();
        let prefs = resolved_default();
        let rows = flatten(&r.tree(true), &BTreeSet::new(), &prefs, &glyph_set());
        assert!(rows.iter().filter(|r| r.is_group).all(|r| !r.is_openable()));
        assert!(rows.iter().filter(|r| !r.is_group).all(|r| r.is_openable()));
    }
}
