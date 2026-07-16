//! The `vibe.tree` action catalogue (PROP-037 §13.5) — the tree commands as
//! addressed entries, plus [`build_registry`], which turns them into real,
//! collision-checked `vibe_actions` [`Action`]s (presentation + capability + a
//! typed [`Ctx`] enablement + synonyms). The live [`Registry`] lets the
//! legibility gate (PROP-039 §8.4) and the enumerable golden (§12.2) run over
//! real actions and the headless AIUI (§11.3) drive them; the search
//! `ActionProvider` reads the same catalogue. The actions' `invoke` is a no-op
//! marker — the TUI Surface applies the effect by address (`super::run_action`,
//! §11.1).

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#action-catalogue");

use vibe_actions::{
    Action, ActionAddr, Capability, Ctx, Enablement, InvokeOutcome, Key, KeyCode, KeyModifiers,
    Keymap, ParamValues, Registry, SearchMeta,
};

use super::super::state::DisplayMode;

/// The context snapshot an action's enablement reads when the window opens
/// (PROP-039 §6.2). Small and `Copy` so it drops straight into a [`Ctx`].
#[derive(Clone, Copy)]
pub struct TreeCtx {
    pub mode: DisplayMode,
    pub has_pkg_selection: bool,
}

/// One catalogue entry — the single source of truth. [`build_registry`] turns
/// the name/description/capability/enablement/synonyms into a real [`Action`];
/// the `key` (default binding) and the effect dispatch stay a Surface concern.
pub struct TreeActionSpec {
    pub addr: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub key: &'static str,
    pub synonyms: &'static [&'static str],
    pub capability: Capability,
    pub enablement: fn(&TreeCtx) -> Enablement,
}

/// The `vibe.tree` catalogue. A search `ActionProvider` `ItemRef` is an index
/// into this list; the App dispatches the effect by the entry's address.
pub const TREE_ACTIONS: &[TreeActionSpec] = &[
    TreeActionSpec {
        addr: "action://vibe.tree/ordering.cycle",
        name: "Cycle ordering",
        desc: "Switch between topological and alphabetical row ordering.",
        key: "n",
        synonyms: &["sort", "alphabetical", "topological", "order"],
        capability: Capability::Mutating,
        enablement: |_| Enablement::enabled(),
    },
    TreeActionSpec {
        addr: "action://vibe.tree/mode.cycle",
        name: "Cycle display mode",
        desc: "Switch between tree, sub-tables, and tabs display.",
        key: "x",
        synonyms: &["mode", "view", "tabs", "sub-tables", "layout"],
        capability: Capability::Mutating,
        enablement: |_| Enablement::enabled(),
    },
    TreeActionSpec {
        addr: "action://vibe.tree/priority.swap",
        name: "Swap static/dynamic priority",
        desc: "Swap whether static or dynamic sorts first in the flat modes.",
        key: "t",
        synonyms: &["static", "dynamic", "priority", "swap"],
        capability: Capability::Mutating,
        enablement: |c| match c.mode {
            DisplayMode::All => Enablement::disabled("only in sub-tables or tabs mode"),
            _ => Enablement::enabled(),
        },
    },
    TreeActionSpec {
        addr: "action://vibe.tree/fold.toggle",
        name: "Fold / unfold selected",
        desc: "Fold or unfold the selected node's subtree.",
        key: "Space",
        synonyms: &["collapse", "expand", "fold", "unfold"],
        capability: Capability::Mutating,
        enablement: |c| {
            if matches!(c.mode, DisplayMode::All) && c.has_pkg_selection {
                Enablement::enabled()
            } else {
                Enablement::disabled("select a package in tree mode")
            }
        },
    },
    TreeActionSpec {
        addr: "action://vibe.tree/fold.all",
        name: "Fold / unfold all",
        desc: "Fold or unfold every node with children.",
        key: "F",
        synonyms: &["collapse all", "expand all"],
        capability: Capability::Mutating,
        enablement: |c| match c.mode {
            DisplayMode::All => Enablement::enabled(),
            _ => Enablement::disabled("only in tree mode"),
        },
    },
    TreeActionSpec {
        addr: "action://vibe.tree/card.open",
        name: "Open details",
        desc: "Open the detail card for the selected package.",
        key: "Enter",
        synonyms: &["details", "card", "inspect"],
        capability: Capability::Safe,
        enablement: |c| {
            if c.has_pkg_selection {
                Enablement::enabled()
            } else {
                Enablement::disabled("select a package first")
            }
        },
    },
    TreeActionSpec {
        addr: "action://vibe.tree/tab.next",
        name: "Next tab",
        desc: "Move to the next tab (tabs mode).",
        key: "Tab",
        synonyms: &["forward"],
        capability: Capability::Mutating,
        enablement: |c| match c.mode {
            DisplayMode::Tabs => Enablement::enabled(),
            _ => Enablement::disabled("only in tabs mode"),
        },
    },
    TreeActionSpec {
        addr: "action://vibe.tree/tab.prev",
        name: "Previous tab",
        desc: "Move to the previous tab (tabs mode).",
        key: "[",
        synonyms: &["back", "previous"],
        capability: Capability::Mutating,
        enablement: |c| match c.mode {
            DisplayMode::Tabs => Enablement::enabled(),
            _ => Enablement::disabled("only in tabs mode"),
        },
    },
    TreeActionSpec {
        addr: "action://vibe.tree/quit",
        name: "Quit",
        desc: "Leave vibe tree.",
        key: "q",
        synonyms: &["exit", "close"],
        capability: Capability::Safe,
        enablement: |_| Enablement::enabled(),
    },
];

/// Build the `vibe.tree` action [`Registry`] — real, collision-checked
/// [`Action`]s (PROP-037 §13.2). Each action's enablement reads a [`TreeCtx`]
/// from the [`Ctx`]; its `invoke` is a no-op marker (the Surface applies the
/// effect by address). A malformed entry is skipped rather than panicking — the
/// catalogue is asserted valid in tests, so the skip never fires.
pub fn build_registry() -> Registry {
    let mut reg = Registry::new();
    for spec in TREE_ACTIONS {
        let Ok(addr) = ActionAddr::parse(spec.addr) else {
            continue;
        };
        let enablement = spec.enablement;
        let synonyms: Vec<String> = spec.synonyms.iter().map(|s| s.to_string()).collect();
        let built = Action::builder(addr)
            .name_en(spec.name)
            .description_en(spec.desc)
            .capability(spec.capability)
            .search_meta(SearchMeta::new(synonyms, Vec::new()))
            .enablement(move |ctx: &Ctx| {
                ctx.get::<TreeCtx>()
                    .map(enablement)
                    .unwrap_or_else(Enablement::enabled)
            })
            .invoke(|_ctx, _values| Box::pin(async { Ok(InvokeOutcome::Done) }))
            .build();
        if let Ok(action) = built {
            let _ = reg.register(action);
        }
    }
    reg
}

/// The default keybinding label for an action address — the `key` a real
/// [`Action`] does not carry (a keymap concern, PROP-039 §9). Empty if unknown.
pub fn key_for(addr: &str) -> &'static str {
    TREE_ACTIONS
        .iter()
        .find(|a| a.addr == addr)
        .map(|a| a.key)
        .unwrap_or("")
}

/// Parse a catalogue `key` string into a [`Key`] (PROP-037 §13.3 `#as-keymap`).
///
/// The notation is table-driven and intentionally small:
/// - `"F1"`..`"F12"` → `KeyCode::F(n)`;
/// - named keys: `"Space"`, `"Enter"`, `"Esc"`, `"Tab"`, `"Backspace"`, `"Delete"`,
///   `"Insert"`, `"Home"`, `"End"`, `"PageUp"`, `"PageDown"`, and the arrows
///   `"Up"`/`"Down"`/`"Left"`/`"Right"`;
/// - `"↑X"` → the key `X` with `SHIFT` (the `↑` glyph is the spec's Shift prefix,
///   PROP-037 §5.2);
/// - any other single character → `KeyCode::Char(c)`. An uppercase letter implies
///   `SHIFT` (crossterm sends `Shift+f` as `Char('F')` + `SHIFT`, so the binding
///   carries `SHIFT` too to match).
///
/// Returns `None` for an unrecognised string (a malformed entry — the catalogue
/// is asserted valid in tests, so this never fires for a real entry).
pub fn parse_key(s: &str) -> Option<Key> {
    let (base, shift) = match s.strip_prefix('\u{2191}') {
        Some(rest) => (rest, true),
        None => (s, false),
    };
    let mut key = parse_key_base(base)?;
    if shift {
        key = key.with_mods(KeyModifiers::SHIFT);
    }
    Some(key)
}

/// The inner parser — resolves the non-Shift part.
fn parse_key_base(s: &str) -> Option<Key> {
    // F1..F12 — distinguish from the letter 'F' by checking the suffix is digits.
    if let Some(rest) = s.strip_prefix('F')
        && let Ok(n) = rest.parse::<u8>()
        && (1..=12).contains(&n)
    {
        return Some(Key::new(KeyCode::F(n)));
    }
    let code = match s {
        "Space" => KeyCode::Space,
        "Enter" => KeyCode::Enter,
        "Esc" => KeyCode::Esc,
        "Tab" => KeyCode::Tab,
        "Backspace" => KeyCode::Backspace,
        "Delete" => KeyCode::Delete,
        "Insert" => KeyCode::Insert,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "PageUp" => KeyCode::PageUp,
        "PageDown" => KeyCode::PageDown,
        "Up" => KeyCode::Up,
        "Down" => KeyCode::Down,
        "Left" => KeyCode::Left,
        "Right" => KeyCode::Right,
        // Arrow glyphs (PROP-037 §5.2) — the same characters the footer hint
        // renders. `↑` (U+2191) never reaches here: it is stripped as the Shift
        // prefix before `parse_key_base` is called.
        "\u{2190}" => KeyCode::Left,
        "\u{2192}" => KeyCode::Right,
        "\u{2193}" => KeyCode::Down,
        _ => {
            let mut chars = s.chars();
            let c = chars.next()?;
            if chars.next().is_none() {
                let mut key = Key::new(KeyCode::Char(c));
                if c.is_ascii_uppercase() {
                    key = key.with_mods(KeyModifiers::SHIFT);
                }
                return Some(key);
            }
            return None;
        }
    };
    Some(Key::new(code))
}

/// Build the `vibe.tree` [`Keymap`] from [`TREE_ACTIONS`] (PROP-037 §13.3
/// `#as-keymap`). Each entry's `key` string is parsed into a [`Key`] and bound
/// to its address with default weight. Entries whose key fails to parse are
/// skipped — the catalogue is asserted valid in tests, so the skip never fires.
pub fn build_keymap() -> Keymap {
    let mut km = Keymap::new();
    for spec in TREE_ACTIONS {
        let Ok(addr) = ActionAddr::parse(spec.addr) else {
            continue;
        };
        if let Some(key) = parse_key(spec.key) {
            km.bind([key], addr, ParamValues::new(), 0);
        }
    }
    km
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_address_is_a_valid_unique_action_uri() {
        let mut seen = std::collections::BTreeSet::new();
        for a in TREE_ACTIONS {
            let addr =
                ActionAddr::parse(a.addr).unwrap_or_else(|e| panic!("bad address {}: {e}", a.addr));
            assert_eq!(addr.to_string(), a.addr, "address round-trips");
            assert!(seen.insert(a.addr), "duplicate address {}", a.addr);
        }
    }

    #[test]
    fn every_action_has_a_nonempty_name_and_description() {
        for a in TREE_ACTIONS {
            assert!(!a.name.trim().is_empty(), "{} has a name", a.addr);
            assert!(!a.desc.trim().is_empty(), "{} has a description", a.addr);
        }
    }

    #[test]
    fn build_registry_registers_every_action_without_collision() {
        let reg = build_registry();
        assert_eq!(reg.len(), TREE_ACTIONS.len());
    }

    #[test]
    fn enablement_reads_the_context() {
        let all = TreeCtx {
            mode: DisplayMode::All,
            has_pkg_selection: true,
        };
        let tabs = TreeCtx {
            mode: DisplayMode::Tabs,
            has_pkg_selection: false,
        };
        let reg = build_registry();
        let fold = reg
            .get(&ActionAddr::parse("action://vibe.tree/fold.toggle").expect("addr"))
            .expect("registered");
        assert!(fold.evaluate(&Ctx::new().with(all)).enabled);
        let disabled = fold.evaluate(&Ctx::new().with(tabs));
        assert!(!disabled.enabled);
        assert!(
            disabled.reason.is_some(),
            "a disabled action gives a reason"
        );
        let next = reg
            .get(&ActionAddr::parse("action://vibe.tree/tab.next").expect("addr"))
            .expect("registered");
        assert!(next.evaluate(&Ctx::new().with(tabs)).enabled);
        assert!(!next.evaluate(&Ctx::new().with(all)).enabled);
    }

    #[test]
    fn the_catalogue_passes_the_legibility_and_reachability_gates() {
        let reg = build_registry();
        vibe_actions::gate::legibility(&reg).expect("every vibe.tree action is legible (§8.4)");
        vibe_actions::gate::reachable(&reg).expect("every vibe.tree action is reachable (§12.2)");
    }

    #[test]
    fn the_headless_aiui_enumerates_the_actions() {
        let reg = build_registry();
        let ctx = Ctx::new().with(TreeCtx {
            mode: DisplayMode::Tabs,
            has_pkg_selection: false,
        });
        let views = vibe_actions::aiui::list_actions(&reg, &ctx);
        assert_eq!(
            views.len(),
            TREE_ACTIONS.len(),
            "every action is AI-enumerable"
        );
        assert!(
            views
                .iter()
                .all(|v| !v.name.is_empty() && !v.description.is_empty())
        );
        let fold = views
            .iter()
            .find(|v| v.address == "action://vibe.tree/fold.toggle")
            .expect("present");
        assert!(
            !fold.enabled && fold.reason.is_some(),
            "a disabled action carries its reason for the AI"
        );
    }

    #[test]
    fn build_keymap_binds_every_action_and_resolves_each_by_key() {
        use vibe_actions::Match;
        let km = build_keymap();
        assert_eq!(
            km.bindings().len(),
            TREE_ACTIONS.len(),
            "one binding per catalogue entry"
        );
        for spec in TREE_ACTIONS {
            let key = parse_key(spec.key).unwrap_or_else(|| panic!("parses {}", spec.key));
            let addr = ActionAddr::parse(spec.addr).unwrap_or_else(|e| panic!("{e}"));
            match km.resolve(std::slice::from_ref(&key), |_| true) {
                Match::Found(resolved, _) => assert_eq!(resolved, addr),
                other => panic!("{} not resolved by key {:?}: {:?}", spec.addr, key, other),
            }
        }
    }

    #[test]
    fn parse_key_handles_f_keys_named_keys_shift_prefix_and_chars() {
        assert_eq!(parse_key("F1"), Some(Key::new(KeyCode::F(1))));
        assert_eq!(parse_key("F12"), Some(Key::new(KeyCode::F(12))));
        assert_eq!(parse_key("Space"), Some(Key::new(KeyCode::Space)));
        assert_eq!(parse_key("Enter"), Some(Key::new(KeyCode::Enter)));
        assert_eq!(parse_key("Esc"), Some(Key::new(KeyCode::Esc)));
        assert_eq!(parse_key("q"), Some(Key::new(KeyCode::Char('q'))));
        // Uppercase implies Shift.
        assert_eq!(
            parse_key("F"),
            Some(Key::new(KeyCode::Char('F')).with_mods(KeyModifiers::SHIFT))
        );
        // Shift prefix.
        assert_eq!(
            parse_key("\u{2191}F6"),
            Some(Key::new(KeyCode::F(6)).with_mods(KeyModifiers::SHIFT))
        );
        assert_eq!(
            parse_key("\u{2191}\u{2192}"),
            Some(Key::new(KeyCode::Right).with_mods(KeyModifiers::SHIFT))
        );
        // F is the letter, not a function-key prefix (no trailing digits).
        assert_eq!(parse_key("Fred"), None);
    }
}
