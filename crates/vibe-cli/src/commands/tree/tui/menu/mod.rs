//! The F-key selection menus (PROP-037 §7.1 F3 mode menu, §7.2 F2 sort menu, and
//! the §2.10 ComingSoon placeholder opened through `app.menu`). A menu is a
//! Controller affordance — selecting an option calls an `App` mutator directly
//! (it does not run through the action system, which cycles rather than sets a
//! specific value).
//!
//! ## Layout
//!
//! The module is split along its responsibility seams to stay under the 600-line
//! file budget: this file holds the shared model ([`MenuState`], [`MenuKind`],
//! [`MenuEffect`]), the navigation + `confirm` policy, and the rendering;
//! [`sort`] holds the multi-group F2 constructor + group builders;
//! [`mode`] holds the single-group F3 constructor.
//!
//! ## The multi-group F2 menu (PROP-037 §7.2 `#f2-sort-menu`)
//!
//! F2's content depends on the active mode: tree & tabs modes get a "Sort by"
//! group and a "Shape" group; sub-tables mode gets those two plus a "Block
//! order" group. The menu is modelled as [`MenuKind::Groups`] — one flat cursor
//! walks every option across all groups (`↑`/`↓`), and `Enter` applies the
//! highlighted option's [`MenuEffect`]. A multi-group F2 menu is **sticky**: it
//! applies the effect, **stays open**, and re-syncs every option's mark from the
//! live [`App`] (so the user adjusts several groups then `Esc`). F3 (a single
//! group) and [`MenuKind::ComingSoon`] close on `Enter`.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#f2-sort-menu");

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::text::Line;
use specmark::spec;

use super::shape::TreeShape;
use super::state::{App, DisplayMode, Ordering};
use super::theme::Theme;
use super::ui::{ComingSoon, Group, Window};

pub mod mode;
pub mod sort;

/// What selecting a menu option does to the model. The shared `Set` prefix is
/// intentional — every variant is a "set this `App` field" effect — so the
/// `enum_variant_names` lint is allowed here rather than dropping the verb.
#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy)]
pub(super) enum MenuEffect {
    SetMode(DisplayMode),
    SetOrdering(Ordering),
    SetShape(TreeShape),
    SetStaticFirst(bool),
}

/// One option: its label, the current-value marker, and its effect.
pub(super) struct MenuOption {
    pub(super) label: String,
    pub(super) checked: bool,
    pub(super) effect: MenuEffect,
}

/// A named cluster of mutually-exclusive options ("Sort by", "Shape", …).
pub(super) struct MenuGroup {
    pub(super) name: String,
    pub(super) options: Vec<MenuOption>,
}

/// What an open menu shows.
pub(super) enum MenuKind {
    /// A stack of named groups with one flat cursor walking every option. `sticky`
    /// = F2 (apply + stay open + re-sync on `Enter`); `!sticky` = F3 (apply +
    /// close on `Enter`).
    Groups {
        groups: Vec<MenuGroup>,
        cursor: usize,
        sticky: bool,
    },
    /// The §2.10 placeholder, drawn through [`ComingSoon`]; `Enter`/`Esc` close.
    ComingSoon,
}

/// An open F-key menu.
pub struct MenuState {
    pub(super) title: String,
    pub(super) kind: MenuKind,
}

impl MenuState {
    /// The ComingSoon placeholder menu (PROP-037 §2.10), titled with `feature`.
    /// Drawn through [`ComingSoon`]; `Enter`/`Esc` close it.
    #[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#coming-soon")]
    #[allow(dead_code)] // first user: copy::png_coming_soon (§10.4); wired when copy-settings lands.
    pub fn coming_soon(feature: impl Into<String>) -> Self {
        Self {
            title: feature.into(),
            kind: MenuKind::ComingSoon,
        }
    }

    /// Move the highlight down, wrapping across every option in every group.
    pub fn select_down(&mut self) {
        if let MenuKind::Groups { groups, cursor, .. } = &mut self.kind {
            let total: usize = groups.iter().map(|g| g.options.len()).sum();
            if total > 0 {
                *cursor = (*cursor + 1) % total;
            }
        }
    }

    /// Move the highlight up, wrapping across every option in every group.
    pub fn select_up(&mut self) {
        if let MenuKind::Groups { groups, cursor, .. } = &mut self.kind {
            let total: usize = groups.iter().map(|g| g.options.len()).sum();
            if total > 0 {
                let n = total;
                *cursor = (*cursor + n - 1) % n;
            }
        }
    }
}

/// The flat index of the first checked option, or 0 — where the cursor starts so
/// the menu opens on the current value of its first group.
pub(super) fn initial_cursor(groups: &[MenuGroup]) -> usize {
    let mut flat = 0;
    for g in groups {
        for o in &g.options {
            if o.checked {
                return flat;
            }
            flat += 1;
        }
    }
    0
}

/// The effect of the option at the flat `cursor`, if any.
pub(super) fn effect_at(groups: &[MenuGroup], cursor: usize) -> Option<MenuEffect> {
    let mut idx = 0;
    for g in groups {
        for o in &g.options {
            if idx == cursor {
                return Some(o.effect);
            }
            idx += 1;
        }
    }
    None
}

/// What `confirm` should do, resolved with a shared borrow so the `App` mutation
/// that follows never overlaps a borrow of `app.menu`.
enum Action {
    Close,
    ApplyClose(Option<MenuEffect>),
    ApplyResync(Option<MenuEffect>),
}

/// Apply the highlighted option to the model. A sticky multi-group menu (F2)
/// applies the effect, **stays open**, and re-syncs every option's mark from
/// the live `App`; F3 (single group) and ComingSoon close on `Enter`
/// (PROP-037 §7.1/§7.2/§2.10).
pub fn confirm(app: &mut App) {
    let action = app.menu.as_ref().map(|menu| match &menu.kind {
        MenuKind::ComingSoon => Action::Close,
        MenuKind::Groups {
            groups,
            cursor,
            sticky,
            ..
        } => {
            let effect = effect_at(groups, *cursor);
            if *sticky {
                Action::ApplyResync(effect)
            } else {
                Action::ApplyClose(effect)
            }
        }
    });
    let Some(action) = action else {
        return;
    };
    match action {
        Action::Close => {
            app.menu = None;
        }
        Action::ApplyClose(effect) => {
            if let Some(effect) = effect {
                apply_effect(app, effect);
                persist_effect(app, effect);
            }
            app.menu = None;
        }
        Action::ApplyResync(effect) => {
            if let Some(effect) = effect {
                apply_effect(app, effect);
                persist_effect(app, effect);
            }
            // Snapshot the live values the marks reflect, then re-sync every
            // option without overlapping a borrow of `app.menu`.
            let snap = Snapshot {
                display_mode: app.display_mode,
                ordering: app.ordering,
                shape: app.shape,
                static_first: app.static_first,
            };
            if let Some(menu) = app.menu.as_mut()
                && let MenuKind::Groups { groups, .. } = &mut menu.kind
            {
                resync_marks(groups, &snap);
            }
        }
    }
}

/// Persist the effect's value through the settings system (PROP-037 §9) when
/// the launch path armed the app with a [`TreeSettings`] cell. A no-op in unit
/// tests (where `app.settings` is `None`) so a model mutator never touches the
/// operator's disk. Swallowed + warned on failure — the change is already live
/// in the model for this session.
fn persist_effect(app: &App, effect: MenuEffect) {
    use super::settings::{
        KEY_MODE, KEY_SHAPE, KEY_SORT, KEY_STATIC_FIRST, mode_label, shape_label, sort_label,
    };
    let Some(s) = &app.settings else {
        return;
    };
    match effect {
        MenuEffect::SetMode(mode) => {
            s.set(KEY_MODE, toml::Value::String(mode_label(mode).into()));
        }
        MenuEffect::SetOrdering(order) => {
            s.set(KEY_SORT, toml::Value::String(sort_label(order).into()));
        }
        MenuEffect::SetShape(shape) => {
            s.set(KEY_SHAPE, toml::Value::String(shape_label(shape).into()));
        }
        MenuEffect::SetStaticFirst(static_first) => {
            s.set(KEY_STATIC_FIRST, toml::Value::Boolean(static_first));
        }
    }
}

/// The live `App` values the menu marks reflect — read once so [`resync_marks`]
/// borrows only this snapshot, not `app` (which `app.menu` already borrows).
struct Snapshot {
    display_mode: DisplayMode,
    ordering: Ordering,
    shape: TreeShape,
    static_first: bool,
}

/// Apply one effect to the model.
fn apply_effect(app: &mut App, effect: MenuEffect) {
    match effect {
        MenuEffect::SetMode(mode) => app.set_display_mode(mode),
        MenuEffect::SetOrdering(order) => app.set_ordering(order),
        MenuEffect::SetShape(shape) => app.set_shape(shape),
        MenuEffect::SetStaticFirst(static_first) => app.set_static_first(static_first),
    }
}

/// Whether `effect` matches the live `snap` — used to (re)set the `●` mark.
fn effect_is_active(effect: MenuEffect, snap: &Snapshot) -> bool {
    match effect {
        MenuEffect::SetMode(m) => snap.display_mode == m,
        MenuEffect::SetOrdering(o) => snap.ordering == o,
        MenuEffect::SetShape(s) => snap.shape == s,
        MenuEffect::SetStaticFirst(f) => snap.static_first == f,
    }
}

/// Re-sync every option's `checked` mark from the live `App` snapshot — the
/// sticky-F2 "apply then keep adjusting" behaviour (PROP-037 §7.2).
fn resync_marks(groups: &mut [MenuGroup], snap: &Snapshot) {
    for g in groups {
        for o in &mut g.options {
            o.checked = effect_is_active(o.effect, snap);
        }
    }
}

// --- drawing ----------------------------------------------------------------

/// Draw the menu centered over `area` (drawn after the base, before nothing —
/// the card / search windows are separate captive modes, never open together).
pub fn draw(area: Rect, buf: &mut Buffer, app: &App) {
    let Some(menu) = app.menu.as_ref() else {
        return;
    };
    if area.width < 20 || area.height < 5 {
        return;
    }
    let theme = &app.theme;
    match &menu.kind {
        MenuKind::ComingSoon => {
            ComingSoon::new(&menu.title).render(area, buf, theme);
        }
        MenuKind::Groups { groups, cursor, .. } => {
            draw_groups(area, buf, &menu.title, groups, *cursor, theme);
        }
    }
}

/// Lay the groups out inside a centered [`Window`]. A single group (F3) renders
/// as a flat list with no group chrome (§7.2 "no group chrome needed"); two or
/// more (F2) frame each group with a [`Group`] whose name sits top-right.
fn draw_groups(
    area: Rect,
    buf: &mut Buffer,
    title: &str,
    groups: &[MenuGroup],
    cursor: usize,
    theme: &Theme,
) {
    let multi = groups.len() > 1;
    let label_w = groups
        .iter()
        .flat_map(|g| g.options.iter().map(|o| o.label.chars().count()))
        .chain(groups.iter().map(|g| g.name.chars().count()))
        .chain(std::iter::once(title.chars().count()))
        .max()
        .unwrap_or(10);
    let w = (label_w as u16 + 8).clamp(24, area.width.saturating_sub(4));

    // Inner content height: the hint row + the options + group framing.
    let total_opts: usize = groups.iter().map(|g| g.options.len()).sum();
    let body_h = if multi {
        // each group = 2 border + options; one-row gaps between groups; + hint row
        groups
            .iter()
            .map(|g| g.options.len() + 2)
            .sum::<usize>()
            .saturating_add(groups.len().saturating_sub(1))
            + 1
    } else {
        total_opts + 3 // a blank row + options + hint row
    };
    let h = (body_h as u16 + 2) // + the window's own two border rows
        .clamp(5, area.height.saturating_sub(2));

    let inner = Window::centered(
        area,
        buf,
        Line::styled(format!(" {title} "), theme.title()),
        w,
        h,
        theme,
    );
    let hint_row = inner.y + inner.height.saturating_sub(1);

    let mut flat = 0usize;
    if multi {
        let mut y = inner.y;
        for group in groups {
            let gh = group.options.len() as u16 + 2;
            if y + gh > hint_row {
                break;
            }
            let garea = Rect::new(inner.x, y, inner.width, gh);
            let ginner = Group::named(&group.name).render(garea, buf, theme);
            for (oi, option) in group.options.iter().enumerate() {
                let oy = ginner.y + oi as u16;
                if oy >= hint_row {
                    break;
                }
                let rect = Rect::new(ginner.x, oy, ginner.width, 1);
                draw_option(rect, buf, option, flat == cursor, theme);
                flat += 1;
            }
            y += gh + 1; // a one-row gap between framed groups
        }
    } else {
        // Single group: flat list, no group chrome (preserves the F3 look).
        let group = &groups[0];
        let list_top = inner.y + 1; // a blank row under the title
        for (i, option) in group.options.iter().enumerate() {
            let y = list_top + i as u16;
            if y >= hint_row {
                break;
            }
            let rect = Rect::new(inner.x + 1, y, inner.width.saturating_sub(2), 1);
            draw_option(rect, buf, option, i == cursor, theme);
        }
    }

    // The key hint on the last inner row.
    buf.set_stringn(
        inner.x + 1,
        hint_row,
        " \u{2191}/\u{2193}  \u{2022}  Enter  \u{2022}  Esc",
        inner.width.saturating_sub(2) as usize,
        theme.dim(),
    );
}

/// Draw one option row: the theme on/off mark plus the label, on the selection
/// bar when this is the cursor row. Marks come from the theme vocabulary
/// (`flag_on`/`flag_off` glyphs) — never a literal.
fn draw_option(rect: Rect, buf: &mut Buffer, option: &MenuOption, is_cursor: bool, theme: &Theme) {
    let mark = if option.checked {
        theme.glyphs().flag_on
    } else {
        theme.glyphs().flag_off
    };
    if is_cursor {
        buf.set_style(rect, theme.selection());
        buf.set_stringn(
            rect.x,
            rect.y,
            format!("{mark} {}", option.label),
            rect.width as usize,
            theme.selection(),
        );
    } else {
        let mark_style = if option.checked {
            theme.accent()
        } else {
            theme.dim()
        };
        buf.set_stringn(rect.x, rect.y, mark, rect.width as usize, mark_style);
        buf.set_stringn(
            rect.x + 2,
            rect.y,
            &option.label,
            rect.width.saturating_sub(2) as usize,
            theme.text(),
        );
    }
}

/// Shared test scaffolding for the menu submodules: a minimal [`App`] over one
/// package, and a [`groups_view`] projector onto a `Groups` menu's internals.
#[cfg(test)]
pub(super) mod test_support {
    use super::*;
    use crate::commands::tree::model::{
        Boot, Condition, HOST_NAMESPACE, IndexLane, Load, LoadOrigin, LoadType, Package,
        PackageTree, Project, SCHEMA_VERSION,
    };

    /// A minimal app: one `g/a` package, declared as a root, default settings.
    pub fn app() -> App {
        let pkg = Package {
            id: "g/a".to_string(),
            group: "g".to_string(),
            name: "a".to_string(),
            kind: "flow".to_string(),
            version: "0.1.0".to_string(),
            content_hash: None,
            source: None,
            load: Load {
                load_type: LoadType::None,
                transitive: false,
                declared: None,
                origin: LoadOrigin::None,
                in_static_md: false,
                in_index_md: false,
                boot_path: None,
            },
            condition: Condition::absent(),
            dependencies: Vec::new(),
        };
        let tree = PackageTree {
            schema_version: SCHEMA_VERSION,
            generated_at: None,
            tool_version: None,
            project: Project {
                root: "/tmp/x".to_string(),
                name: None,
                is_workspace: false,
                host_namespace: HOST_NAMESPACE.to_string(),
            },
            roots: vec!["g/a".to_string()],
            packages: vec![pkg],
            boot: Boot {
                static_md: None,
                index_md: IndexLane {
                    present: false,
                    path: "spec/boot/INDEX.md".to_string(),
                    static_pointer: None,
                    entries: Vec::new(),
                },
            },
            in_place_specs: Vec::new(),
            diagnostics: Vec::new(),
        };
        App::new(tree)
    }

    /// A read-only view of a `Groups` menu's group list, flat cursor, and sticky
    /// flag (panics on other kinds — which is the point in a Groups-menu test).
    pub struct GroupsView<'a> {
        pub groups: &'a [MenuGroup],
        pub cursor: usize,
        pub sticky: bool,
    }

    pub fn groups_view(menu: &MenuState) -> GroupsView<'_> {
        match &menu.kind {
            MenuKind::Groups {
                groups,
                cursor,
                sticky,
            } => GroupsView {
                groups,
                cursor: *cursor,
                sticky: *sticky,
            },
            _ => panic!("expected a Groups menu"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::app;
    use super::*;

    #[test]
    fn coming_soon_confirm_closes_without_an_effect() {
        let mut a = app();
        a.menu = Some(MenuState::coming_soon("PNG export"));
        assert!(matches!(
            a.menu.as_ref().expect("open").kind,
            MenuKind::ComingSoon
        ));
        assert_eq!(a.menu.as_ref().expect("open").title, "PNG export");
        confirm(&mut a);
        assert!(a.menu.is_none(), "ComingSoon closes on confirm");
    }

    /// A ComingSoon menu is a no-op for navigation (only `Enter`/`Esc` act).
    #[test]
    fn coming_soon_navigation_is_a_noop() {
        let mut m = MenuState::coming_soon("PNG export");
        m.select_up();
        m.select_down();
        assert!(matches!(m.kind, MenuKind::ComingSoon));
    }
}
