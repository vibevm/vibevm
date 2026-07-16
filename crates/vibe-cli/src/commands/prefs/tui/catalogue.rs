//! The `vibe.prefs` action catalogue (PROP-041 §8 `#commands-are-actions`) —
//! every settings-UI command as an addressed `vibe_actions` [`Action`] in group
//! `vibe.prefs`, plus [`build_registry`] (collision-checked, gate-passing real
//! [`Registry`]) and [`build_keymap`] (the PROP-037 keymap binding each action
//! to its default key). The catalogue is the single source of truth; the
//! [`super::dispatch::dispatch_by_addr`] routing and the
//! [`super::search`] `ActionProvider` both read it, and the footer
//! ([`super::render`]) lists the enabled actions for the current context
//! (PROP-037 §5.2).
//!
//! Mirrors `tree::tui::search::catalogue` (`TREE_ACTIONS` + `build_registry` +
//! `build_keymap` + `parse_key`); each surface owns its self-contained key-parse
//! because the pure keymap resolver is frontend-agnostic and the parse is a
//! surface concern (PROP-039 §9 `#as-keymap`, the tree catalogue's recorded
//! posture). An action's `invoke` is a no-op marker — the surface applies the
//! effect by address.

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-041#commands-are-actions");

use vibe_actions::{
    Action, ActionAddr, Capability, Ctx, Enablement, InvokeOutcome, Key, KeyCode, KeyModifiers,
    Keymap, ParamValues, Registry, SearchMeta,
};

/// The enablement snapshot a `vibe.prefs` action reads (PROP-039 §6.2, the
/// `TreeCtx` analogue). Built from [`super::state::PrefsApp`] at
/// search-open time + by the keymap resolver gate. Small and `Copy` so it drops
/// straight into a [`Ctx`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrefsActionCtx {
    /// At the base screen — no page form, no lint modal, no search window.
    pub at_base: bool,
    /// A page form is open (the right pane renders the edit form).
    pub page_open: bool,
    /// A leaf page is selected in the tree (openable via `Enter`).
    pub leaf_selected: bool,
    /// The focused form field is non-text (apply/reset/layer/provenance are
    /// reachable; a text field would swallow the letter keys as typing).
    pub form_editable: bool,
    /// The form has a blocking validation error — `apply` is gated off
    /// (PROP-041 §6 `#validation-feedback`).
    pub has_blocking_error: bool,
}

/// One catalogue entry — the single source of truth. [`build_registry`] turns
/// the name/description/capability/enablement/synonyms into a real [`Action`];
/// the `key` (default binding) and the effect dispatch stay a Surface concern
/// (PROP-039 §11.1).
pub struct PrefsActionSpec {
    pub addr: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub key: &'static str,
    pub synonyms: &'static [&'static str],
    pub capability: Capability,
    pub enablement: fn(&PrefsActionCtx) -> Enablement,
}

/// The `vibe.prefs` catalogue (PROP-041 §8 `#commands-are-actions`). A search
/// `ActionProvider` `ItemRef` is an index into this list; the App dispatches the
/// effect by the entry's address. The 8 commands cover the whole settings-UI
/// surface: open a page, apply, reset, search, check-all-layers, switch
/// write-layer, toggle provenance, quit.
pub const PREFS_ACTIONS: &[PrefsActionSpec] = &[
    PrefsActionSpec {
        addr: "action://vibe.prefs/page.open",
        name: "Open page",
        desc: "Open the focused page's edit form.",
        key: "Enter",
        synonyms: &["page", "open", "edit", "form"],
        capability: Capability::Safe,
        enablement: |c| {
            if c.at_base && c.leaf_selected {
                Enablement::enabled()
            } else {
                Enablement::disabled("select a page in the tree")
            }
        },
    },
    PrefsActionSpec {
        addr: "action://vibe.prefs/apply",
        name: "Apply",
        desc: "Write the form's edits to the chosen layer.",
        key: "a",
        synonyms: &["save", "write", "commit", "persist"],
        capability: Capability::Mutating,
        enablement: |c| {
            if !c.page_open || !c.form_editable {
                Enablement::disabled("open a page and focus a non-text field")
            } else if c.has_blocking_error {
                Enablement::disabled("a field has a validation error")
            } else {
                Enablement::enabled()
            }
        },
    },
    PrefsActionSpec {
        addr: "action://vibe.prefs/reset",
        name: "Reset",
        desc: "Reset the form to the resolved values.",
        key: "r",
        synonyms: &["revert", "undo", "discard"],
        capability: Capability::Safe,
        enablement: |c| {
            if c.page_open && c.form_editable {
                Enablement::enabled()
            } else {
                Enablement::disabled("open a page and focus a non-text field")
            }
        },
    },
    PrefsActionSpec {
        addr: "action://vibe.prefs/search",
        name: "Search settings",
        desc: "Open Search Everywhere over every setting key, name, and synonym.",
        key: "/",
        synonyms: &["find", "filter", "look", "everywhere"],
        capability: Capability::Safe,
        enablement: |c| {
            if c.at_base {
                Enablement::enabled()
            } else {
                Enablement::disabled("close the open page first")
            }
        },
    },
    PrefsActionSpec {
        addr: "action://vibe.prefs/lint",
        name: "Check all layers",
        desc: "Open the cross-layer validation modal — every warning, jump-to-field.",
        key: "c",
        synonyms: &["check", "validate", "warnings", "diagnostics"],
        capability: Capability::Safe,
        enablement: |c| {
            if c.at_base {
                Enablement::enabled()
            } else {
                Enablement::disabled("close the open page first")
            }
        },
    },
    PrefsActionSpec {
        addr: "action://vibe.prefs/layer.next",
        name: "Cycle write-layer",
        desc: "Switch the form's write layer (L1 \u{2192} L2 \u{2192} L3).",
        key: "Tab",
        synonyms: &["layer", "write", "target", "scope"],
        capability: Capability::Mutating,
        enablement: |c| {
            if c.page_open && c.form_editable {
                Enablement::enabled()
            } else {
                Enablement::disabled("open a page and focus a non-text field")
            }
        },
    },
    PrefsActionSpec {
        addr: "action://vibe.prefs/provenance.toggle",
        name: "Toggle provenance",
        desc: "Show where the focused field's value comes from (per layer).",
        key: "?",
        synonyms: &["origin", "source", "layer", "inspect", "why"],
        capability: Capability::Safe,
        enablement: |c| {
            if c.page_open && c.form_editable {
                Enablement::enabled()
            } else {
                Enablement::disabled("open a page and focus a non-text field")
            }
        },
    },
    PrefsActionSpec {
        addr: "action://vibe.prefs/quit",
        name: "Quit",
        desc: "Leave vibe prefs.",
        key: "q",
        synonyms: &["exit", "close"],
        capability: Capability::Safe,
        enablement: |c| {
            if c.at_base {
                Enablement::enabled()
            } else {
                Enablement::disabled("close the open page first")
            }
        },
    },
];

/// Build the `vibe.prefs` action [`Registry`] — real, collision-checked
/// [`Action`]s (PROP-039 §4). Each action's enablement reads a
/// [`PrefsActionCtx`] from the [`Ctx`]; its `invoke` is a no-op marker (the
/// Surface applies the effect by address, [`super::dispatch::dispatch_by_addr`]).
/// A malformed entry is skipped rather than panicking — the catalogue is
/// asserted valid in tests, so the skip never fires.
pub fn build_registry() -> Registry {
    let mut reg = Registry::new();
    for spec in PREFS_ACTIONS {
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
                ctx.get::<PrefsActionCtx>()
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
    PREFS_ACTIONS
        .iter()
        .find(|a| a.addr == addr)
        .map(|a| a.key)
        .unwrap_or("")
}

/// The `(key, description)` pairs for the actions **enabled** in `ctx`, in
/// catalogue order — the footer (PROP-037 §5.2) renders these so the surface
/// shows only what the current context can do.
pub fn enabled_footer_keys(ctx: PrefsActionCtx) -> Vec<(&'static str, &'static str)> {
    PREFS_ACTIONS
        .iter()
        .filter(|spec| (spec.enablement)(&ctx).enabled)
        .map(|spec| (spec.key, spec.desc))
        .collect()
}

/// Parse a catalogue `key` string into a [`Key`] (PROP-037 §13.3 `#as-keymap`).
///
/// Notation (mirrors `tree::tui::search::catalogue::parse_key` — each surface
/// owns its self-contained parse):
/// - `"F1"`..`"F12"` \u{2192} `KeyCode::F(n)`;
/// - named keys: `"Space"`, `"Enter"`, `"Esc"`, `"Tab"`, `"Backspace"`, `"Delete"`,
///   `"Insert"`, `"Home"`, `"End"`, `"PageUp"`, `"PageDown"`, and the arrows;
/// - any other single character \u{2192} `KeyCode::Char(c)` (an uppercase letter
///   implies `SHIFT`, so `Shift+f` bindings match crossterm's `Char('F')` + SHIFT).
///
/// Returns `None` for an unrecognised string (the catalogue is asserted valid
/// in tests, so this never fires for a real entry).
pub fn parse_key(s: &str) -> Option<Key> {
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

/// Build the `vibe.prefs` [`Keymap`] from [`PREFS_ACTIONS`] (PROP-037 §13.3
/// `#as-keymap`). Each entry's `key` string is parsed into a [`Key`] and bound
/// to its address with default weight. Entries whose key fails to parse are
/// skipped — the catalogue is asserted valid in tests, so the skip never fires.
pub fn build_keymap() -> Keymap {
    let mut km = Keymap::new();
    for spec in PREFS_ACTIONS {
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
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn every_address_is_a_valid_unique_vibe_prefs_action_uri() {
        let mut seen = std::collections::BTreeSet::new();
        for a in PREFS_ACTIONS {
            let addr =
                ActionAddr::parse(a.addr).unwrap_or_else(|e| panic!("bad address {}: {e}", a.addr));
            assert_eq!(addr.to_string(), a.addr, "address round-trips");
            assert_eq!(
                addr.group(),
                "vibe.prefs",
                "every prefs action is in the vibe.prefs group"
            );
            assert!(seen.insert(a.addr), "duplicate address {}", a.addr);
        }
    }

    #[test]
    fn every_action_has_a_nonempty_name_and_description_and_parseable_key() {
        for a in PREFS_ACTIONS {
            assert!(!a.name.trim().is_empty(), "{} has a name", a.addr);
            assert!(!a.desc.trim().is_empty(), "{} has a description", a.addr);
            assert!(
                parse_key(a.key).is_some(),
                "{} key '{}' parses",
                a.addr,
                a.key
            );
        }
    }

    #[test]
    fn build_registry_registers_every_action_without_collision() {
        let reg = build_registry();
        assert_eq!(reg.len(), PREFS_ACTIONS.len());
    }

    #[test]
    fn the_catalogue_passes_the_legibility_and_reachability_gates() {
        let reg = build_registry();
        vibe_actions::gate::legibility(&reg)
            .expect("every vibe.prefs action is legible (PROP-039 §8.4)");
        vibe_actions::gate::reachable(&reg).expect("every vibe.prefs action is reachable (§12.2)");
    }

    #[test]
    fn the_headless_aiui_enumerates_the_actions() {
        let reg = build_registry();
        let ctx = Ctx::new().with(PrefsActionCtx {
            at_base: true,
            page_open: false,
            leaf_selected: true,
            form_editable: false,
            has_blocking_error: false,
        });
        let views = vibe_actions::aiui::list_actions(&reg, &ctx);
        assert_eq!(
            views.len(),
            PREFS_ACTIONS.len(),
            "every action is AI-enumerable"
        );
        assert!(
            views
                .iter()
                .all(|v| !v.name.is_empty() && !v.description.is_empty())
        );
    }

    #[test]
    fn enablement_gates_apply_on_a_blocking_error() {
        let reg = build_registry();
        let apply = reg
            .get(&ActionAddr::parse("action://vibe.prefs/apply").expect("addr"))
            .expect("registered");
        // Happy path: page open, non-text field, no error.
        let ok = PrefsActionCtx {
            at_base: false,
            page_open: true,
            leaf_selected: false,
            form_editable: true,
            has_blocking_error: false,
        };
        assert!(apply.evaluate(&Ctx::new().with(ok)).enabled);
        // Blocked: a field has a validation error.
        let blocked = PrefsActionCtx {
            has_blocking_error: true,
            ..ok
        };
        let verdict = apply.evaluate(&Ctx::new().with(blocked));
        assert!(!verdict.enabled);
        assert!(verdict.reason.is_some(), "a disabled apply gives a reason");
    }

    #[test]
    fn enabled_footer_keys_lists_only_live_actions_for_a_context() {
        // At base with a leaf selected: page.open + search + lint + quit are live.
        let base = PrefsActionCtx {
            at_base: true,
            page_open: false,
            leaf_selected: true,
            form_editable: false,
            has_blocking_error: false,
        };
        let keys = enabled_footer_keys(base);
        let labels: Vec<&str> = keys.iter().map(|(k, _)| *k).collect();
        assert!(
            labels.contains(&"Enter"),
            "page.open is live at base with a leaf"
        );
        assert!(labels.contains(&"/"), "search is live at base");
        assert!(labels.contains(&"c"), "lint is live at base");
        assert!(labels.contains(&"q"), "quit is live at base");
        assert!(!labels.contains(&"a"), "apply is not live at base");
        // On an open form: apply/reset/layer/provenance are live, search/quit are not.
        let form = PrefsActionCtx {
            at_base: false,
            page_open: true,
            leaf_selected: false,
            form_editable: true,
            has_blocking_error: false,
        };
        let keys = enabled_footer_keys(form);
        let labels: Vec<&str> = keys.iter().map(|(k, _)| *k).collect();
        assert!(labels.contains(&"a"), "apply is live on the form");
        assert!(labels.contains(&"r"), "reset is live on the form");
        assert!(labels.contains(&"Tab"), "layer.next is live on the form");
        assert!(
            labels.contains(&"?"),
            "provenance.toggle is live on the form"
        );
        assert!(!labels.contains(&"/"), "search is not live on the form");
    }

    #[test]
    fn build_keymap_binds_every_action_and_resolves_each_by_key() {
        use vibe_actions::Match;
        let km = build_keymap();
        assert_eq!(
            km.bindings().len(),
            PREFS_ACTIONS.len(),
            "one binding per catalogue entry"
        );
        for spec in PREFS_ACTIONS {
            let key = parse_key(spec.key).unwrap_or_else(|| panic!("parses {}", spec.key));
            let addr = ActionAddr::parse(spec.addr).unwrap_or_else(|e| panic!("{e}"));
            match km.resolve(std::slice::from_ref(&key), |_| true) {
                Match::Found(resolved, _) => assert_eq!(resolved, addr),
                other => panic!("{} not resolved by key {:?}: {:?}", spec.addr, key, other),
            }
        }
    }

    #[test]
    fn parse_key_handles_named_keys_chars_and_function_keys() {
        assert_eq!(parse_key("F1"), Some(Key::new(KeyCode::F(1))));
        assert_eq!(parse_key("Enter"), Some(Key::new(KeyCode::Enter)));
        assert_eq!(parse_key("Tab"), Some(Key::new(KeyCode::Tab)));
        assert_eq!(parse_key("/"), Some(Key::new(KeyCode::Char('/'))));
        assert_eq!(parse_key("?"), Some(Key::new(KeyCode::Char('?'))));
        assert_eq!(parse_key("q"), Some(Key::new(KeyCode::Char('q'))));
        // Uppercase implies Shift.
        assert_eq!(
            parse_key("A"),
            Some(Key::new(KeyCode::Char('A')).with_mods(KeyModifiers::SHIFT))
        );
        // Two characters do not parse as a single key.
        assert_eq!(parse_key("abc"), None);
    }
}
