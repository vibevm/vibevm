//! The F-key selection menus (PROP-037 §7.1 F3 mode menu, §7.2 F2 sort menu): a
//! small captive dropdown that lists the choices for one setting, marks the
//! current one, and applies the pick on `Enter`. This is the menu-driven path
//! the contract's F-key scheme calls for; the bare letter shortcuts (`n`/`x`)
//! still work alongside it. A menu is a Controller affordance — selecting one
//! calls an `App` mutator directly (it does not run through the action system,
//! which cycles rather than sets a specific value).

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#f3-mode-menu");

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Constraint, Flex, Layout, Rect};
use ratatui_core::style::{Color, Modifier, Style};
use ratatui_core::text::Line;
use ratatui_core::widgets::Widget;
use ratatui_widgets::block::Block;
use ratatui_widgets::clear::Clear;

use super::state::{App, DisplayMode, Ordering};

/// What selecting a menu option does to the model.
#[derive(Clone, Copy)]
enum MenuEffect {
    SetMode(DisplayMode),
    SetOrdering(Ordering),
}

/// One option: its label, the current-value marker, and its effect.
struct MenuOption {
    label: String,
    checked: bool,
    effect: MenuEffect,
}

/// An open F-key menu.
pub struct MenuState {
    title: String,
    options: Vec<MenuOption>,
    selected: usize,
}

impl MenuState {
    /// The F3 display-mode menu (PROP-037 §7.1).
    pub fn mode(app: &App) -> Self {
        let current = app.display_mode;
        let options = [DisplayMode::All, DisplayMode::SubTables, DisplayMode::Tabs]
            .into_iter()
            .map(|m| MenuOption {
                label: m.label().to_string(),
                checked: m == current,
                effect: MenuEffect::SetMode(m),
            })
            .collect::<Vec<_>>();
        let selected = options.iter().position(|o| o.checked).unwrap_or(0);
        MenuState {
            title: "Display mode".to_string(),
            options,
            selected,
        }
    }

    /// The F2 sort menu (PROP-037 §7.2) — the row ordering.
    pub fn sort(app: &App) -> Self {
        let current = app.ordering;
        let options = [Ordering::Topological, Ordering::Alphabetical]
            .into_iter()
            .map(|o| MenuOption {
                label: o.label().to_string(),
                checked: o == current,
                effect: MenuEffect::SetOrdering(o),
            })
            .collect::<Vec<_>>();
        let selected = options.iter().position(|o| o.checked).unwrap_or(0);
        MenuState {
            title: "Order rows by".to_string(),
            options,
            selected,
        }
    }

    /// Move the highlight down, wrapping.
    pub fn select_down(&mut self) {
        if !self.options.is_empty() {
            self.selected = (self.selected + 1) % self.options.len();
        }
    }

    /// Move the highlight up, wrapping.
    pub fn select_up(&mut self) {
        if !self.options.is_empty() {
            let n = self.options.len();
            self.selected = (self.selected + n - 1) % n;
        }
    }

    fn effect(&self) -> Option<MenuEffect> {
        self.options.get(self.selected).map(|o| o.effect)
    }
}

/// Apply the highlighted option to the model and close the menu.
pub fn confirm(app: &mut App) {
    let effect = app.menu.as_ref().and_then(|m| m.effect());
    if let Some(effect) = effect {
        match effect {
            MenuEffect::SetMode(mode) => app.set_display_mode(mode),
            MenuEffect::SetOrdering(order) => app.set_ordering(order),
        }
    }
    app.menu = None;
}

/// Draw the menu centered over `area` (drawn after the base, before nothing —
/// the card / search windows are separate captive modes, never open together).
pub fn draw(area: Rect, buf: &mut Buffer, menu: &MenuState) {
    if area.width < 16 || area.height < 4 {
        return;
    }
    let inner_w = menu
        .options
        .iter()
        .map(|o| o.label.chars().count())
        .chain(std::iter::once(menu.title.chars().count()))
        .max()
        .unwrap_or(10) as u16
        + 8; // radio marker + padding
    let w = inner_w.clamp(16, area.width.saturating_sub(2));
    let h = (menu.options.len() as u16 + 2).clamp(3, area.height.saturating_sub(2));

    let [mid] = Layout::vertical([Constraint::Length(h)])
        .flex(Flex::Center)
        .areas(area);
    let [popup] = Layout::horizontal([Constraint::Length(w)])
        .flex(Flex::Center)
        .areas(mid);

    Widget::render(Clear, popup, buf);
    let block = Block::bordered().title(format!(" {} ", menu.title));
    let inner = block.inner(popup);
    Widget::render(block, popup, buf);

    for (i, option) in menu.options.iter().enumerate() {
        let y = inner.y + i as u16;
        if y >= inner.y + inner.height {
            break;
        }
        let rect = Rect::new(inner.x, y, inner.width, 1);
        let selected = i == menu.selected;
        let mut style = Style::new();
        if selected {
            style = style.fg(Color::Black).bg(Color::Cyan);
        }
        buf.set_style(rect, style);
        let marker = if option.checked { "(o) " } else { "( ) " };
        let text = format!("{marker}{}", option.label);
        buf.set_stringn(rect.x, rect.y, text, rect.width as usize, style);
    }

    // A one-line hint under the last option if there is room.
    let hint_y = inner.y + menu.options.len() as u16;
    if hint_y < inner.y + inner.height {
        let hint = Line::from(" \u{2191}/\u{2193} \u{2022} Enter \u{2022} Esc")
            .style(Style::new().add_modifier(Modifier::DIM));
        Widget::render(hint, Rect::new(inner.x, hint_y, inner.width, 1), buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::tree::model::{
        Boot, Condition, HOST_NAMESPACE, IndexLane, Load, LoadOrigin, LoadType, Package,
        PackageTree, Project, SCHEMA_VERSION,
    };

    fn app() -> App {
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

    #[test]
    fn the_mode_menu_marks_the_current_mode_and_applies_the_pick() {
        let mut a = app();
        assert_eq!(a.display_mode, DisplayMode::All);
        a.menu = Some(MenuState::mode(&a));
        {
            let m = a.menu.as_ref().expect("open");
            assert_eq!(m.options.len(), 3);
            assert_eq!(m.selected, 0, "the current mode is selected");
            assert!(m.options[0].checked, "the current mode is marked");
        }
        // `↑` from the first option wraps to the last (Tabs); confirm applies it.
        a.menu.as_mut().expect("open").select_up();
        confirm(&mut a);
        assert_eq!(a.display_mode, DisplayMode::Tabs);
        assert!(a.menu.is_none(), "the menu closed on confirm");
    }

    #[test]
    fn the_sort_menu_sets_the_ordering() {
        let mut a = app();
        assert_eq!(a.ordering, Ordering::Topological);
        a.menu = Some(MenuState::sort(&a));
        a.menu.as_mut().expect("open").select_down(); // -> Alphabetical
        confirm(&mut a);
        assert_eq!(a.ordering, Ordering::Alphabetical);
    }
}
