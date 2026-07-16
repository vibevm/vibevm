//! The F2 sort/shape menu constructor + its group builders (PROP-037 §7.2
//! `#f2-sort-menu`). F2 is the multi-group, sticky menu — apply an effect,
//! stay open, re-sync the marks (the "adjust several groups then `Esc`" UX).
//!
//! The shared model ([`MenuState`], [`MenuEffect`], [`MenuGroup`]) and the
//! `confirm` policy live in [`super`]; this file only composes the F2 groups
//! from the live [`App`] and gives them their [`MenuEffect`]s.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#f2-sort-menu");

use specmark::spec;

use super::super::shape::TreeShape;
use super::super::state::{App, DisplayMode, Ordering};
use super::{MenuEffect, MenuGroup, MenuKind, MenuOption, MenuState, initial_cursor};

impl MenuState {
    /// The F2 sort/shape menu (PROP-037 §7.2) — multi-group and sticky. The
    /// groups depend on the active mode: tree & tabs get "Sort by" + "Shape";
    /// sub-tables adds "Block order".
    #[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#f2-sort-menu")]
    pub fn sort(app: &App) -> Self {
        let mut groups = vec![sort_group(app), shape_group(app)];
        if app.display_mode == DisplayMode::SubTables {
            groups.push(block_order_group(app));
        }
        let cursor = initial_cursor(&groups);
        Self {
            title: "Sort & shape".to_string(),
            kind: MenuKind::Groups {
                groups,
                cursor,
                sticky: true,
            },
        }
    }
}

/// The "Sort by" group — alphabetical / topological (the sibling order,
/// PROP-037 §3.2/§7.2).
fn sort_group(app: &App) -> MenuGroup {
    MenuGroup {
        name: "Sort by".to_string(),
        options: [Ordering::Alphabetical, Ordering::Topological]
            .into_iter()
            .map(|o| MenuOption {
                label: o.label().to_string(),
                checked: app.ordering == o,
                effect: MenuEffect::SetOrdering(o),
            })
            .collect(),
    }
}

/// The "Shape" group — the three forest shapes (PROP-037 §3.3).
fn shape_group(app: &App) -> MenuGroup {
    MenuGroup {
        name: "Shape".to_string(),
        options: [
            TreeShape::MembersAsRoots,
            TreeShape::LoadTypeForest,
            TreeShape::PrunedTree,
        ]
        .into_iter()
        .map(|s| MenuOption {
            label: shape_label(s).to_string(),
            checked: app.shape == s,
            effect: MenuEffect::SetShape(s),
        })
        .collect(),
    }
}

/// The "Block order" group — static-first / dynamic-first (sub-tables mode only,
/// PROP-037 §7.2). Backs the `static_first` partition order.
fn block_order_group(app: &App) -> MenuGroup {
    MenuGroup {
        name: "Block order".to_string(),
        options: vec![
            MenuOption {
                label: "static-first".to_string(),
                checked: app.static_first,
                effect: MenuEffect::SetStaticFirst(true),
            },
            MenuOption {
                label: "dynamic-first".to_string(),
                checked: !app.static_first,
                effect: MenuEffect::SetStaticFirst(false),
            },
        ],
    }
}

/// The menu label for a [`TreeShape`] (PROP-037 §3.3). Kept here rather than on
/// the enum so the shape pipeline module carries no display strings.
fn shape_label(shape: TreeShape) -> &'static str {
    match shape {
        TreeShape::MembersAsRoots => "members as roots",
        TreeShape::LoadTypeForest => "load-type forest",
        TreeShape::PrunedTree => "pruned tree",
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::shape::TreeShape;
    use super::super::super::state::DisplayMode;
    use super::super::test_support::{app, groups_view};
    use super::super::{MenuEffect, confirm, effect_at};
    use super::MenuState;

    #[test]
    fn the_sort_menu_has_sort_and_shape_groups_in_tree_mode() {
        let mut a = app();
        a.menu = Some(MenuState::sort(&a));
        let v = groups_view(a.menu.as_ref().expect("open"));
        assert_eq!(v.groups.len(), 2, "tree mode: Sort by + Shape");
        assert_eq!(v.groups[0].name, "Sort by");
        assert_eq!(v.groups[1].name, "Shape");
        assert!(v.sticky, "F2 stays open on Enter");
        assert_eq!(v.groups[1].options.len(), 3, "three shapes");
    }

    #[test]
    fn the_sort_menu_adds_block_order_in_subtables_mode() {
        let mut a = app();
        a.set_display_mode(DisplayMode::SubTables);
        a.menu = Some(MenuState::sort(&a));
        let v = groups_view(a.menu.as_ref().expect("open"));
        assert_eq!(v.groups.len(), 3, "sub-tables adds Block order");
        assert_eq!(v.groups[2].name, "Block order");
        assert_eq!(v.groups[2].options.len(), 2);
    }

    #[test]
    fn the_flat_cursor_walks_across_groups() {
        let mut a = app();
        a.menu = Some(MenuState::sort(&a));
        // Cursor starts on the checked Sort-by option (Topological, flat idx 1).
        let start = groups_view(a.menu.as_ref().expect("open")).cursor;
        assert_eq!(start, 1);
        // One `↓` crosses into the Shape group (flat idx 2 = members-as-roots).
        a.menu.as_mut().expect("open").select_down();
        let v = groups_view(a.menu.as_ref().expect("open"));
        assert_eq!(v.cursor, 2);
        let crossed = effect_at(v.groups, v.cursor);
        assert!(
            matches!(
                crossed,
                Some(MenuEffect::SetShape(TreeShape::MembersAsRoots))
            ),
            "cursor crossed into the Shape group"
        );
    }

    #[test]
    fn sticky_sort_menu_applies_and_resyncs_and_stays_open() {
        let mut a = app();
        assert_eq!(a.shape, TreeShape::MembersAsRoots);
        a.menu = Some(MenuState::sort(&a));
        // Walk to the Shape group's "load-type forest" option (flat idx 3).
        for _ in 0..2 {
            a.menu.as_mut().expect("open").select_down();
        }
        let before = groups_view(a.menu.as_ref().expect("open")).cursor;
        assert_eq!(before, 3);
        confirm(&mut a);
        // Applied to the model …
        assert_eq!(a.shape, TreeShape::LoadTypeForest);
        // … the menu STAYED open …
        let v = groups_view(a.menu.as_ref().expect("still open"));
        // … and the mark re-synced: load-type-forest is now the checked shape.
        let shape_opts = &v.groups[1].options;
        let forest = shape_opts
            .iter()
            .find(|o| matches!(o.effect, MenuEffect::SetShape(TreeShape::LoadTypeForest)))
            .expect("option");
        assert!(forest.checked, "the picked shape is now marked");
        let members = shape_opts
            .iter()
            .find(|o| matches!(o.effect, MenuEffect::SetShape(TreeShape::MembersAsRoots)))
            .expect("option");
        assert!(!members.checked, "the previous shape is unmarked");
    }

    #[test]
    fn block_order_pick_applies_set_static_first() {
        let mut a = app();
        a.set_display_mode(DisplayMode::SubTables);
        assert!(a.static_first);
        a.menu = Some(MenuState::sort(&a));
        // Flat layout: Sort by(2) + Shape(3) + Block order(2). Cursor starts on
        // the checked Sort-by option (idx 1). Walk to Block order's
        // "dynamic-first" (idx 6).
        for _ in 0..5 {
            a.menu.as_mut().expect("open").select_down();
        }
        confirm(&mut a);
        assert!(!a.static_first, "dynamic-first applied");
    }
}
