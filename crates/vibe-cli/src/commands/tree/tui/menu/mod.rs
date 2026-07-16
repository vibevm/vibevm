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
//! [`MenuEffect`]) and the focus-group navigation + `confirm` policy; [`sort`]
//! holds the multi-group F2 constructor + group builders; [`mode`] holds the
//! single-group F3 constructor; [`draw`] holds the rendering (the active
//! focus-group is accent-framed there).
//!
//! ## The multi-group F2 menu — focus groups + Tab Order
//! (PROP-037 §5.4 `#focus-groups`, §7.2 `#f2-sort-menu`)
//!
//! F2's content depends on the active mode: tree & tabs modes get a "Sort by"
//! group and a "Shape" group; sub-tables mode gets those two plus a "Block
//! order" group. Each group is a **focus group** (§5.4): `Tab`/`Shift+Tab`
//! cycle the **active group** (Sort by → Shape → Block order → Sort by …), and
//! `↑`/`↓` move the selection **within** the active group only (wrapping inside
//! that group's options, never crossing into another). `Enter` applies the
//! active group's selected option. A multi-group F2 menu is **sticky**: it
//! applies the effect, **stays open**, and re-syncs every option's mark from the
//! live [`App`] (so the user adjusts several groups then `Esc`). F3 (a single
//! group — no Tab Order, `Tab` is inert) and [`MenuKind::ComingSoon`] close on
//! `Enter`.
//!
//! The model: [`MenuGroup`] owns its own within-group `cursor` (the highlight
//! position), and [`MenuKind::Groups`] carries `active_group` (which group
//! `Tab` has focus in). This mirrors the copy-settings focus-group pattern
//! (`focus` + per-`RadioGroup` selection) — per-group memory so `Tab` away and
//! back leaves the cursor exactly where it was.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#f2-sort-menu");

use specmark::spec;

use super::shape::TreeShape;
use super::state::{App, DisplayMode, Ordering};

mod draw;
pub(super) use self::draw::draw;

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

/// A named focus group of mutually-exclusive options ("Sort by", "Shape", …).
/// Owns its own within-group `cursor` (the highlight position, PROP-037 §5.4) so
/// `Tab` away and back leaves it where the user left it — the same split as
/// copy-settings' `RadioGroup::selected`.
pub(super) struct MenuGroup {
    pub(super) name: String,
    pub(super) options: Vec<MenuOption>,
    /// The highlighted option within this group (the row `Enter` applies when
    /// this group is the active one). Wraps within `options.len()` on `↑`/`↓`.
    pub(super) cursor: usize,
}

/// What an open menu shows.
pub(super) enum MenuKind {
    /// A stack of named focus groups (PROP-037 §5.4). `active_group` is the one
    /// `Tab` has focus in (its cursor is live, its frame is accent-marked);
    /// `sticky` = F2 (apply + stay open + re-sync on `Enter`), `!sticky` = F3
    /// (apply + close on `Enter`, single group so `Tab` is inert).
    Groups {
        groups: Vec<MenuGroup>,
        active_group: usize,
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

    /// Move the highlight down within the **active group**, wrapping inside that
    /// group's options (PROP-037 §5.4 — `↑`/`↓` never cross into another group).
    pub fn select_down(&mut self) {
        if let MenuKind::Groups {
            groups,
            active_group,
            ..
        } = &mut self.kind
            && let Some(g) = groups.get_mut(*active_group)
            && !g.options.is_empty()
        {
            g.cursor = (g.cursor + 1) % g.options.len();
        }
    }

    /// Move the highlight up within the **active group**, wrapping inside that
    /// group's options (PROP-037 §5.4).
    pub fn select_up(&mut self) {
        if let MenuKind::Groups {
            groups,
            active_group,
            ..
        } = &mut self.kind
            && let Some(g) = groups.get_mut(*active_group)
            && !g.options.is_empty()
        {
            let n = g.options.len();
            g.cursor = (g.cursor + n - 1) % n;
        }
    }

    /// Cycle the active focus group forward (`Tab`, PROP-037 §5.4). The
    /// within-group cursors are untouched — only which group holds the live
    /// `↑`/`↓` + `Enter` changes. A single-group menu (F3) is a no-op (there is
    /// no Tab Order — `Tab` is inert there).
    #[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#focus-groups")]
    pub fn focus_next_group(&mut self) {
        if let MenuKind::Groups {
            groups,
            active_group,
            ..
        } = &mut self.kind
            && groups.len() > 1
        {
            *active_group = (*active_group + 1) % groups.len();
        }
    }

    /// Cycle the active focus group backward (`Shift+Tab`, PROP-037 §5.4).
    #[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#focus-groups")]
    pub fn focus_prev_group(&mut self) {
        if let MenuKind::Groups {
            groups,
            active_group,
            ..
        } = &mut self.kind
            && groups.len() > 1
        {
            let n = groups.len();
            *active_group = (*active_group + n - 1) % n;
        }
    }
}

/// The within-group index of the first checked option, or 0 — where each group's
/// cursor starts so a group opens on its current value. Called per group by the
/// F2/F3 constructors after building the options.
pub(super) fn initial_group_cursor(options: &[MenuOption]) -> usize {
    options.iter().position(|o| o.checked).unwrap_or(0)
}

/// The index of the first group that has a checked option, or 0 — where the
/// `active_group` starts so the menu opens focused on the group holding a
/// current value.
pub(super) fn initial_active_group(groups: &[MenuGroup]) -> usize {
    groups
        .iter()
        .position(|g| g.options.iter().any(|o| o.checked))
        .unwrap_or(0)
}

/// The effect of the `active_group`'s currently-selected option, if any.
pub(super) fn active_effect(groups: &[MenuGroup], active_group: usize) -> Option<MenuEffect> {
    groups
        .get(active_group)
        .and_then(|g| g.options.get(g.cursor).map(|o| o.effect))
}

/// What `confirm` should do, resolved with a shared borrow so the `App` mutation
/// that follows never overlaps a borrow of `app.menu`.
enum Action {
    Close,
    ApplyClose(Option<MenuEffect>),
    ApplyResync(Option<MenuEffect>),
}

/// Apply the active group's highlighted option to the model. A sticky
/// multi-group menu (F2) applies the effect, **stays open**, and re-syncs every
/// option's mark from the live `App`; F3 (single group) and ComingSoon close on
/// `Enter` (PROP-037 §5.4/§7.1/§7.2/§2.10).
pub fn confirm(app: &mut App) {
    let action = app.menu.as_ref().map(|menu| match &menu.kind {
        MenuKind::ComingSoon => Action::Close,
        MenuKind::Groups {
            groups,
            active_group,
            sticky,
            ..
        } => {
            let effect = active_effect(groups, *active_group);
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
/// sticky-F2 "apply then keep adjusting" behaviour (PROP-037 §7.2). The
/// per-group cursors are untouched: only the `●`/`○` value marks move.
fn resync_marks(groups: &mut [MenuGroup], snap: &Snapshot) {
    for g in groups {
        for o in &mut g.options {
            o.checked = effect_is_active(o.effect, snap);
        }
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

    /// A read-only view of a `Groups` menu's group list, active-group index, and
    /// sticky flag (panics on other kinds — which is the point in a Groups-menu
    /// test). Each group carries its own within-group `cursor`.
    pub struct GroupsView<'a> {
        pub groups: &'a [MenuGroup],
        pub active_group: usize,
        pub sticky: bool,
    }

    pub fn groups_view(menu: &MenuState) -> GroupsView<'_> {
        match &menu.kind {
            MenuKind::Groups {
                groups,
                active_group,
                sticky,
            } => GroupsView {
                groups,
                active_group: *active_group,
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
        m.focus_next_group();
        m.focus_prev_group();
        assert!(matches!(m.kind, MenuKind::ComingSoon));
    }
}
