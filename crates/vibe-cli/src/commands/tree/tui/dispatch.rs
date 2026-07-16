//! Address-routed dispatch (PROP-037 §13.2 `#as-actions`) — the single source
//! of truth for "what does a `vibe.tree` action do to the model." Both the
//! base-mode keymap path ([`super::input`]) and the Search Everywhere ACTIONS
//! provider ([`super::search`]) call [`dispatch_by_addr`] rather than each
//! doing its own string-match. The 9 catalogue addresses are the only routing
//! keys; matching is on the address's `name()` half (all live under the
//! `vibe.tree` group).
//!
//! [`enabled_in_base`] mirrors the catalogue's per-context enablement so the
//! resolver can gate bindings without reaching into the typed `Ctx` (the
//! resolver stays pure, PROP-039 §9.2).

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#action-catalogue");

use rat_salsa::Control;
use vibe_actions::ActionAddr;

use super::AppEvent;
use super::state::{App, DisplayMode, RowNode};

/// Apply a `vibe.tree` action to the model by its address (PROP-037 §13.2).
/// Returns the rat-salsa control-flow verdict — `Control::Quit` for the quit
/// action, `Control::Changed` for everything else (the App mutators handle
/// their own no-op guards internally).
pub(super) fn dispatch_by_addr(app: &mut App, addr: &ActionAddr) -> Control<AppEvent> {
    match addr.name() {
        "ordering.cycle" => app.cycle_ordering(),
        "mode.cycle" => app.cycle_display_mode(),
        "priority.swap" => app.swap_priority(),
        "fold.toggle" => app.toggle_fold_selected(),
        "fold.all" => app.toggle_fold_all(),
        "card.open" => {
            if app
                .selected_row()
                .map(|r| matches!(r.node, RowNode::Package(_) | RowNode::Missing))
                .unwrap_or(false)
            {
                app.modal_open = true;
            }
        }
        "tab.next" => app.next_tab(),
        "tab.prev" => app.prev_tab(),
        "quit" => return Control::Quit,
        _ => {}
    }
    Control::Changed
}

/// Whether `addr` is live in the base screen (no overlay open) — the resolver's
/// enablement gate. Most actions are always enabled (their App mutators carry
/// the real no-op guards); `tab.next`/`tab.prev` are only live in Tabs mode so
/// the `Tab`/`[` keys do not shadow other interactions outside Tabs.
pub(super) fn enabled_in_base(app: &App, addr: &ActionAddr) -> bool {
    match addr.name() {
        "tab.next" | "tab.prev" => app.display_mode == DisplayMode::Tabs,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::commands::tree::model::{
        Boot, Condition, HOST_NAMESPACE, IndexLane, Load, LoadOrigin, LoadType, Package,
        PackageTree, Project, SCHEMA_VERSION,
    };

    fn pkg(id: &str) -> Package {
        let (group, name) = id.split_once('/').unwrap_or(("g", id));
        Package {
            id: id.to_string(),
            group: group.to_string(),
            name: name.to_string(),
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
        }
    }

    fn tiny_app() -> App {
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
            roots: vec!["g/alpha".to_string()],
            packages: vec![pkg("g/alpha")],
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
    fn dispatch_mode_cycle_changes_the_display_mode() {
        let mut app = tiny_app();
        let before = app.display_mode;
        let addr = ActionAddr::parse("action://vibe.tree/mode.cycle").unwrap();
        let control = dispatch_by_addr(&mut app, &addr);
        assert!(matches!(control, Control::Changed));
        assert_ne!(app.display_mode, before, "the mode cycled");
    }

    #[test]
    fn dispatch_quit_returns_control_quit() {
        let mut app = tiny_app();
        let addr = ActionAddr::parse("action://vibe.tree/quit").unwrap();
        let control = dispatch_by_addr(&mut app, &addr);
        assert!(matches!(control, Control::Quit));
    }

    #[test]
    fn enabled_in_base_gates_tab_actions_on_display_mode() {
        let app = tiny_app(); // defaults to All
        let tab_next = ActionAddr::parse("action://vibe.tree/tab.next").unwrap();
        assert!(
            !enabled_in_base(&app, &tab_next),
            "tab.next is inert outside Tabs mode"
        );

        let mut app = app;
        app.display_mode = DisplayMode::Tabs;
        assert!(
            enabled_in_base(&app, &tab_next),
            "tab.next is live in Tabs mode"
        );

        let quit = ActionAddr::parse("action://vibe.tree/quit").unwrap();
        assert!(
            enabled_in_base(&app, &quit),
            "quit is always enabled in base"
        );
    }
}
