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
    Action, ActionAddr, Capability, Ctx, Enablement, InvokeOutcome, Registry, SearchMeta,
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
}
