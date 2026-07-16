//! The model-plane projection of the `vibe tree` TUI state — the AIUI `state()`
//! surface (PROP-039 §11.2/§11.3, prototyped on the TUI per §13). A pure,
//! serialisable snapshot of [`App`] carrying **no rendering types**, so an agent
//! reads structured state — display mode, ordering, the active tab, the
//! selection, the visible rows, which modals are open — rather than pixels. It
//! is the semantic sibling of the render plane ([`super::snapshot_headless`]):
//! same `(tree, script)`, but the model instead of the glyph grid.
//!
//! Spec: [PROP-042 §4](../../../../spec/modules/vibe-cli/PROP-042-aiui-observation.md#aiui-cli)
//! (the `state` verb), [PROP-039 §11.2](../../../../spec/modules/vibe-actions/PROP-039-action-system.md#model-view).

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-042#aiui-cli");

use serde::Serialize;

use super::menu::MenuKind;
use super::row::{RowNode, VisibleRow};
use super::state::App;

/// The serialisable `vibe tree` model view (PROP-039 §11.2): a pure projection
/// of [`App`] — the display mode, the ordering, the active tab, the selection,
/// the visible rows, and which modals are open. No rendering types; an agent
/// asserts flow/state, never pixels.
#[derive(Debug, Clone, Serialize)]
pub struct TreeModelView {
    /// The current display mode (`all` / `sub-tables` / `tabs`).
    pub display_mode: &'static str,
    /// The row ordering (`topological` / `alphabetical`).
    pub ordering: &'static str,
    /// The active partition tab in `tabs` mode.
    pub tab: usize,
    /// Whether `static` sorts before `dynamic` in the partitioned modes.
    pub static_first: bool,
    /// The selected row, if any.
    pub selection: Option<SelectionView>,
    /// The flattened visible rows in scroll order.
    pub visible_rows: Vec<RowView>,
    /// Which modals/menus are open.
    pub modals: ModalsView,
    /// The package ids the user has collapsed (`group/name`).
    pub folded: Vec<String>,
}

/// The selected row (PROP-039 §11.2 "current tree/selection").
#[derive(Debug, Clone, Serialize)]
pub struct SelectionView {
    /// The row index in the visible-rows list.
    pub index: usize,
    /// The row's id (`group/name` for a package; the edge target for a missing
    /// node; empty for the separator/subheader).
    pub id: String,
    /// What the row points at (`package` / `missing` / `separator` / `subheader`).
    pub kind: &'static str,
}

/// One visible row (PROP-039 §11.2 "visible rows").
#[derive(Debug, Clone, Serialize)]
pub struct RowView {
    pub id: String,
    /// The drawn name cell (prefix + connector + indicator + id + `(*)` marker).
    pub name: String,
    pub kind: &'static str,
    /// The effective-load column label (`static` / `dynamic` / `none`).
    pub load: &'static str,
    /// `T` — transitive-static flag.
    pub transitive: bool,
    /// `C` — `when`-condition flag.
    pub condition: bool,
    /// `S` — physically in `STATIC.md`.
    pub in_static: bool,
}

/// Which modal/menu is open (PROP-039 §11.2 "open modals"). At most one of the
/// F-key menus is open at a time; the detail modal, search, copy-settings, and
/// quit-confirm are independent flags.
#[derive(Debug, Clone, Serialize)]
pub struct ModalsView {
    /// The detail modal (`Enter` on a row).
    pub detail: bool,
    /// The F2 sort menu (the sticky multi-group menu).
    pub sort_menu: bool,
    /// The F3 mode menu (the single-group menu).
    pub mode_menu: bool,
    /// A `ComingSoon` placeholder menu.
    pub coming_soon_menu: bool,
    /// The F1 Search Everywhere window.
    pub search: bool,
    /// The Shift+F6 copy-settings modal.
    pub copy_settings: bool,
    /// The file-dest modal (depth-2 over copy-settings).
    pub file_dest: bool,
    /// The Esc quit-confirm dialog.
    pub confirm_quit: bool,
    /// The focused button in the quit dialog (`false` = OK, `true` = Cancel).
    pub confirm_cancel_focused: bool,
}

impl App {
    /// Project the current TUI state into a serialisable [`TreeModelView`]
    /// (PROP-039 §11.2/§11.3). Pure — reads only; no mutation, no rendering.
    /// Drives no input; pair with the `--send` script at the CLI/handler layer.
    pub fn model_view(&self) -> TreeModelView {
        let selection = self.table.selected().map(|index| {
            let row = self.rows.get(index);
            SelectionView {
                index,
                id: row.map(|r| r.id.clone()).unwrap_or_default(),
                kind: row.map(row_kind).unwrap_or("none"),
            }
        });
        TreeModelView {
            display_mode: self.display_mode.label(),
            ordering: self.ordering.label(),
            tab: self.tab,
            static_first: self.static_first,
            selection,
            visible_rows: self.rows.iter().map(row_view).collect(),
            modals: self.modals_view(),
            folded: self.folded.iter().cloned().collect(),
        }
    }

    fn modals_view(&self) -> ModalsView {
        // F2 is the sticky multi-group menu; F3 is the single-group (!sticky)
        // menu; `ComingSoon` is the placeholder. `sticky` distinguishes them.
        let (sort_menu, mode_menu, coming_soon_menu) = match &self.menu {
            Some(m) => match &m.kind {
                MenuKind::Groups { sticky: true, .. } => (true, false, false),
                MenuKind::Groups { sticky: false, .. } => (false, true, false),
                MenuKind::ComingSoon => (false, false, true),
            },
            None => (false, false, false),
        };
        ModalsView {
            detail: self.modal_open,
            sort_menu,
            mode_menu,
            coming_soon_menu,
            search: self.search.is_some(),
            copy_settings: self.copy_settings.is_some(),
            file_dest: self.file_dest.is_some(),
            confirm_quit: self.confirm_quit,
            confirm_cancel_focused: self.confirm_cancel_focused,
        }
    }
}

fn row_view(r: &VisibleRow) -> RowView {
    RowView {
        id: r.id.clone(),
        name: r.name.clone(),
        kind: row_kind(r),
        load: r.load,
        transitive: r.transitive,
        condition: r.condition,
        in_static: r.in_static,
    }
}

fn row_kind(r: &VisibleRow) -> &'static str {
    match r.node {
        RowNode::Package(_) => "package",
        RowNode::Missing => "missing",
        RowNode::Separator => "separator",
        RowNode::Subheader => "subheader",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::tree::tui::keyscript;
    use crate::commands::tree::tui::menu::test_support::fixture_tree;

    /// Drive `script` over the fixture and project the resulting state.
    fn state_after(script: &str) -> TreeModelView {
        let evs = keyscript::parse(script).expect("script");
        let mut app = App::new(fixture_tree());
        if !app.rows.is_empty() {
            app.table.select(Some(0));
        }
        for ev in &evs {
            let _ = crate::commands::tree::tui::input::handle(ev, &mut app);
        }
        app.model_view()
    }

    #[test]
    fn base_state_projects_display_mode_and_selection() {
        let v = state_after("");
        assert_eq!(v.display_mode, "all");
        assert_eq!(v.ordering, "topological");
        assert_eq!(v.tab, 0);
        // The fixture is one package `g/a`, declared as a root → one row.
        assert_eq!(v.visible_rows.len(), 1);
        assert_eq!(v.visible_rows[0].id, "g/a");
        assert_eq!(v.visible_rows[0].kind, "package");
        // The selection anchors on the first row.
        let sel = v.selection.expect("a selection");
        assert_eq!(sel.index, 0);
        assert_eq!(sel.id, "g/a");
        assert_eq!(sel.kind, "package");
        // Nothing is open at the base screen.
        assert!(!v.modals.detail);
        assert!(!v.modals.sort_menu);
        assert!(!v.modals.confirm_quit);
    }

    #[test]
    fn f2_opens_the_sort_menu() {
        let v = state_after("F2");
        assert!(v.modals.sort_menu, "F2 opens the sticky sort menu");
        assert!(!v.modals.mode_menu);
    }

    #[test]
    fn f3_opens_the_mode_menu() {
        let v = state_after("F3");
        assert!(v.modals.mode_menu, "F3 opens the single-group mode menu");
        assert!(!v.modals.sort_menu);
    }

    #[test]
    fn esc_at_base_opens_the_quit_dialog() {
        let v = state_after("Esc");
        assert!(v.modals.confirm_quit, "a bare Esc opens the quit dialog");
        // OK is the default-focused button.
        assert!(!v.modals.confirm_cancel_focused);
    }

    #[test]
    fn model_view_is_serialisable() {
        let v = state_after("F2");
        let json = serde_json::to_string(&v).expect("serialize");
        assert!(json.contains("\"display_mode\":\"all\""));
        assert!(json.contains("\"sort_menu\":true"));
        assert!(json.contains("\"visible_rows\""));
    }
}
