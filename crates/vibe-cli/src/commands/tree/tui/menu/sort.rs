//! The F2 sort/shape menu constructor + its group builders (PROP-037 §5.4
//! `#focus-groups`, §7.2 `#f2-sort-menu`). F2 is the multi-group, sticky menu —
//! apply an effect, stay open, re-sync the marks (the "adjust several groups
//! then `Esc`" UX). Each group is a focus group: `Tab`/`Shift+Tab` cycle the
//! active group; `↑`/`↓` move within the active group only.
//!
//! The shared model ([`MenuState`], [`MenuEffect`], [`MenuGroup`]) and the
//! `confirm` + focus-group navigation policy live in [`super`]; this file only
//! composes the F2 groups from the live [`App`] and gives them their
//! [`MenuEffect`]s.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#f2-sort-menu");

use specmark::spec;

use super::super::shape::TreeShape;
use super::super::state::{App, DisplayMode, Ordering};
use super::{
    MenuEffect, MenuGroup, MenuKind, MenuOption, MenuState, initial_active_group,
    initial_group_cursor,
};

impl MenuState {
    /// The F2 sort/shape menu (PROP-037 §7.2) — multi-group and sticky. The
    /// groups depend on the active mode: tree & tabs get "Sort by" + "Shape";
    /// sub-tables adds "Block order". Each group's cursor starts on its current
    /// value; the active group starts on the first group holding a checked
    /// option (PROP-037 §5.4 — the focus-group model).
    #[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#f2-sort-menu")]
    pub fn sort(app: &App) -> Self {
        let mut groups = vec![sort_group(app), shape_group(app)];
        if app.display_mode == DisplayMode::SubTables {
            groups.push(block_order_group(app));
        }
        for g in &mut groups {
            g.cursor = initial_group_cursor(&g.options);
        }
        let active_group = initial_active_group(&groups);
        Self {
            title: "Sort & shape".to_string(),
            kind: MenuKind::Groups {
                groups,
                active_group,
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
        cursor: 0,
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
        cursor: 0,
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
        cursor: 0,
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
    use super::super::{MenuEffect, confirm};
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

    /// Cursors start on each group's current value: Sort by on Topological
    /// (default, idx 1), Shape on members-as-roots (default, idx 0). The active
    /// group is the first with a checked option = Sort by (PROP-037 §5.4).
    #[test]
    fn cursors_start_on_each_groups_current_value() {
        let mut a = app();
        a.menu = Some(MenuState::sort(&a));
        let v = groups_view(a.menu.as_ref().expect("open"));
        assert_eq!(v.active_group, 0, "active group = Sort by (first checked)");
        assert_eq!(v.groups[0].cursor, 1, "Sort by cursor on Topological");
        assert_eq!(v.groups[1].cursor, 0, "Shape cursor on members-as-roots");
    }

    /// `Tab` advances the active group (0→1→0…); the within-group cursors do
    /// NOT move on Tab (PROP-037 §5.4 — focus groups have per-group memory).
    #[test]
    fn tab_advances_active_group_without_moving_cursors() {
        let mut a = app();
        a.menu = Some(MenuState::sort(&a));
        // Sort by cursor starts on Topological (idx 1); Shape on idx 0.
        a.menu.as_mut().expect("open").focus_next_group();
        let v = groups_view(a.menu.as_ref().expect("open"));
        assert_eq!(v.active_group, 1, "Tab → Shape group");
        assert_eq!(v.groups[0].cursor, 1, "Sort by cursor unchanged");
        assert_eq!(v.groups[1].cursor, 0, "Shape cursor unchanged");
        // Tab wraps back to Sort by.
        a.menu.as_mut().expect("open").focus_next_group();
        let v = groups_view(a.menu.as_ref().expect("open"));
        assert_eq!(v.active_group, 0, "Tab wraps to Sort by");
        // Shift+Tab goes back to Shape.
        a.menu.as_mut().expect("open").focus_prev_group();
        let v = groups_view(a.menu.as_ref().expect("open"));
        assert_eq!(v.active_group, 1, "Shift+Tab → Shape group");
    }

    /// `↑`/`↓` move the selection WITHIN the active group only (wrapping), never
    /// crossing into another group (PROP-037 §5.4).
    #[test]
    fn arrows_move_only_within_the_active_group() {
        let mut a = app();
        a.menu = Some(MenuState::sort(&a));
        // Active = Sort by (2 options). Cursor on idx 1 (Topological). ↓ wraps to 0.
        a.menu.as_mut().expect("open").select_down();
        let v = groups_view(a.menu.as_ref().expect("open"));
        assert_eq!(v.groups[0].cursor, 0, "Sort by cursor wrapped to 0");
        assert_eq!(
            v.groups[1].cursor, 0,
            "Shape cursor untouched — arrows stay in the active group"
        );
        // Tab to Shape (3 options, cursor 0); ↓ twice moves within Shape only.
        a.menu.as_mut().expect("open").focus_next_group();
        a.menu.as_mut().expect("open").select_down();
        a.menu.as_mut().expect("open").select_down();
        let v = groups_view(a.menu.as_ref().expect("open"));
        assert_eq!(v.active_group, 1);
        assert_eq!(v.groups[1].cursor, 2, "Shape cursor at idx 2 (pruned tree)");
        assert_eq!(v.groups[0].cursor, 0, "Sort by cursor still untouched");
        // ↑ wraps within Shape (idx 2 → 1 → 0 → 2).
        a.menu.as_mut().expect("open").select_up();
        a.menu.as_mut().expect("open").select_up();
        a.menu.as_mut().expect("open").select_up();
        assert_eq!(
            groups_view(a.menu.as_ref().expect("open")).groups[1].cursor,
            2,
            "↑ wraps within the Shape group"
        );
    }

    #[test]
    fn sticky_sort_menu_applies_the_active_groups_option_and_resyncs() {
        let mut a = app();
        assert_eq!(a.shape, TreeShape::MembersAsRoots);
        a.menu = Some(MenuState::sort(&a));
        // Tab to the Shape group, ↓ to "load-type forest" (idx 1), Enter applies.
        a.menu.as_mut().expect("open").focus_next_group();
        a.menu.as_mut().expect("open").select_down();
        let v = groups_view(a.menu.as_ref().expect("open"));
        assert_eq!(v.active_group, 1);
        assert_eq!(v.groups[1].cursor, 1, "Shape cursor on load-type forest");
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
        // Three groups: Sort by(0) / Shape(1) / Block order(2). Tab twice to
        // Block order, ↓ to "dynamic-first" (idx 1), Enter applies.
        a.menu.as_mut().expect("open").focus_next_group();
        a.menu.as_mut().expect("open").focus_next_group();
        a.menu.as_mut().expect("open").select_down();
        let v = groups_view(a.menu.as_ref().expect("open"));
        assert_eq!(v.active_group, 2, "Block order is the active group");
        assert_eq!(v.groups[2].cursor, 1, "cursor on dynamic-first");
        confirm(&mut a);
        assert!(!a.static_first, "dynamic-first applied");
    }
}
