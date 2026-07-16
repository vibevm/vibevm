//! The F3 display-mode menu constructor (PROP-037 §7.1 `#f3-mode-menu`). F3 is
//! the single-group, non-sticky menu — pick a mode and the menu closes. The
//! shared model and `confirm` policy live in [`super`].

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#f3-mode-menu");

use specmark::spec;

use super::super::state::{App, DisplayMode};
use super::{MenuEffect, MenuGroup, MenuKind, MenuOption, MenuState, initial_cursor};

impl MenuState {
    /// The F3 display-mode menu (PROP-037 §7.1) — one group; closes on `Enter`.
    #[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#f3-mode-menu")]
    pub fn mode(app: &App) -> Self {
        let options: Vec<MenuOption> =
            [DisplayMode::All, DisplayMode::SubTables, DisplayMode::Tabs]
                .into_iter()
                .map(|m| MenuOption {
                    label: m.label().to_string(),
                    checked: app.display_mode == m,
                    effect: MenuEffect::SetMode(m),
                })
                .collect();
        let groups = vec![MenuGroup {
            name: "Display mode".to_string(),
            options,
        }];
        let cursor = initial_cursor(&groups);
        Self {
            title: "Display mode".to_string(),
            kind: MenuKind::Groups {
                groups,
                cursor,
                sticky: false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::state::DisplayMode;
    use super::super::confirm;
    use super::super::test_support::{app, groups_view};
    use super::MenuState;

    #[test]
    fn the_mode_menu_marks_the_current_mode_and_applies_the_pick() {
        let mut a = app();
        assert_eq!(a.display_mode, DisplayMode::All);
        a.menu = Some(MenuState::mode(&a));
        {
            let v = groups_view(a.menu.as_ref().expect("open"));
            assert_eq!(v.groups.len(), 1, "F3 is one group");
            assert!(!v.sticky, "F3 closes on Enter");
            assert_eq!(v.cursor, 0, "cursor on the current mode");
            assert!(v.groups[0].options[0].checked, "All is marked");
        }
        // `↑` from the first option wraps to the last (Tabs); confirm applies it.
        a.menu.as_mut().expect("open").select_up();
        confirm(&mut a);
        assert_eq!(a.display_mode, DisplayMode::Tabs);
        assert!(a.menu.is_none(), "the menu closed on confirm");
    }
}
