//! The collision-erroring, enumerable action registry (PROP-039 §4).
//!
//! Actions register by their [`ActionAddr`] identity. Registering a
//! `(group, name)` already present is a **hard, deterministic error** naming
//! both the incumbent and the newcomer (§4.1 — a *collision*, never a silent
//! override). A layered override is the explicit [`Registry::override_of`]
//! (§4.1 `#registry-override`). A reference to an address is validated by
//! [`Registry::check_ref`] (§4.2 referential integrity). The registry is
//! **fully enumerable** — [`Registry::iter`] reaches every action, and
//! [`Registry::aliases`] every alias (§4.3, §2.2).
//!
//! Spec: [PROP-039 §4](../../../../spec/modules/vibe-actions/PROP-039-action-system.md#registry).

specmark::scope!("spec://vibevm/modules/vibe-actions/PROP-039#registry");

use std::collections::BTreeMap;

use thiserror::Error;

use crate::action::Action;
use crate::address::ActionAddr;

/// A registry operation failure (PROP-039 §4).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[specmark::spec(implements = "spec://vibevm/modules/vibe-actions/PROP-039#registry-collision")]
pub enum RegistryError {
    /// A `(group, name)` was registered twice — the incumbent and newcomer are
    /// both named (§4.1).
    #[error(
        "action address collision at `{addr}`: `{incumbent}` is already registered there, \
         cannot also register `{newcomer}` \
         (violates spec://vibevm/modules/vibe-actions/PROP-039#registry-collision; \
          fix: give the newcomer a distinct address, or use `override_of` deliberately)"
    )]
    Collision {
        /// The colliding address.
        addr: String,
        /// The incumbent action's name.
        incumbent: String,
        /// The newcomer action's name.
        newcomer: String,
    },

    /// An alias address is already occupied by an action or another alias.
    #[error(
        "cannot register alias `{alias}`: that address is already an action or alias \
         (violates spec://vibevm/modules/vibe-actions/PROP-039#address-uniqueness; \
          fix: choose an unused alias address)"
    )]
    AliasCollision {
        /// The alias address that clashed.
        alias: String,
    },

    /// An alias points at an address that is not a registered action (§4.2).
    #[error(
        "alias `{alias}` targets `{target}`, which is not a registered action \
         (violates spec://vibevm/modules/vibe-actions/PROP-039#registry-integrity; \
          fix: register the target action before aliasing to it)"
    )]
    AliasTargetMissing {
        /// The alias address.
        alias: String,
        /// The missing target address.
        target: String,
    },

    /// [`Registry::override_of`] was called with no incumbent to override.
    #[error(
        "cannot override `{addr}`: nothing is registered there \
         (violates spec://vibevm/modules/vibe-actions/PROP-039#registry-override; \
          fix: use `register` for a new address, or register the incumbent first)"
    )]
    OverrideMissing {
        /// The address with no incumbent.
        addr: String,
    },
}

/// The enumerable action registry. Backed by ordered maps, so enumeration is
/// deterministic (address order) — friendly to goldens and the AIUI
/// `list_actions` (§11.3).
#[derive(Default)]
pub struct Registry {
    actions: BTreeMap<ActionAddr, Action>,
    aliases: BTreeMap<ActionAddr, ActionAddr>,
}

impl Registry {
    /// An empty registry.
    pub fn new() -> Self {
        Registry::default()
    }

    /// Register `action`. A duplicate `(group, name)` — as an action or an
    /// alias — is a hard [`RegistryError::Collision`] naming both parties
    /// (§4.1).
    pub fn register(&mut self, action: Action) -> Result<(), RegistryError> {
        let addr = action.addr().clone();
        if let Some(incumbent) = self.actions.get(&addr) {
            return Err(RegistryError::Collision {
                addr: addr.to_string(),
                incumbent: incumbent.presentation().name().default_en().to_owned(),
                newcomer: action.presentation().name().default_en().to_owned(),
            });
        }
        if self.aliases.contains_key(&addr) {
            return Err(RegistryError::AliasCollision {
                alias: addr.to_string(),
            });
        }
        self.actions.insert(addr, action);
        Ok(())
    }

    /// Explicitly replace the incumbent at `action`'s address (§4.1
    /// `#registry-override`). Errors with [`RegistryError::OverrideMissing`] if
    /// there is nothing to override — override is never an accidental
    /// consequence of registration order.
    pub fn override_of(&mut self, action: Action) -> Result<(), RegistryError> {
        let addr = action.addr().clone();
        if !self.actions.contains_key(&addr) {
            return Err(RegistryError::OverrideMissing {
                addr: addr.to_string(),
            });
        }
        self.actions.insert(addr, action);
        Ok(())
    }

    /// Register an alias: the retired address `old` resolves to the live action
    /// at `new` (§2.2). `new` must be a registered action, and `old` must be
    /// unused.
    pub fn alias(&mut self, old: ActionAddr, new: ActionAddr) -> Result<(), RegistryError> {
        if !self.actions.contains_key(&new) {
            return Err(RegistryError::AliasTargetMissing {
                alias: old.to_string(),
                target: new.to_string(),
            });
        }
        if self.actions.contains_key(&old) || self.aliases.contains_key(&old) {
            return Err(RegistryError::AliasCollision {
                alias: old.to_string(),
            });
        }
        self.aliases.insert(old, new);
        Ok(())
    }

    /// Look up the action at `addr`, resolving one alias hop (§2.2). Returns
    /// `None` if neither an action nor an alias is registered there.
    pub fn get(&self, addr: &ActionAddr) -> Option<&Action> {
        if let Some(action) = self.actions.get(addr) {
            return Some(action);
        }
        let target = self.aliases.get(addr)?;
        self.actions.get(target)
    }

    /// Look up the action registered *directly* at `addr`, ignoring aliases.
    pub fn get_direct(&self, addr: &ActionAddr) -> Option<&Action> {
        self.actions.get(addr)
    }

    /// The target an alias resolves to, if `addr` is an alias.
    pub fn resolve_alias(&self, addr: &ActionAddr) -> Option<&ActionAddr> {
        self.aliases.get(addr)
    }

    /// Whether a reference to `addr` resolves to a registered action —
    /// directly or through an alias (§4.2 referential integrity).
    pub fn check_ref(&self, addr: &ActionAddr) -> bool {
        self.get(addr).is_some()
    }

    /// Enumerate every registered action, in address order (§4.3).
    pub fn iter(&self) -> impl Iterator<Item = &Action> {
        self.actions.values()
    }

    /// Enumerate every alias as `(old, new)`, in address order (§2.2).
    pub fn aliases(&self) -> impl Iterator<Item = (&ActionAddr, &ActionAddr)> {
        self.aliases.iter()
    }

    /// The number of registered actions (aliases excluded).
    pub fn len(&self) -> usize {
        self.actions.len()
    }

    /// Whether no actions are registered.
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Capability;
    use crate::invoke::InvokeOutcome;

    fn addr(s: &str) -> ActionAddr {
        ActionAddr::parse(s).unwrap()
    }

    fn action(addr_str: &str, name: &'static str) -> Action {
        Action::builder(addr(addr_str))
            .name_en(name)
            .description_en("a test action")
            .invoke(|_c, _v| Box::pin(async { Ok(InvokeOutcome::Done) }))
            .build()
            .unwrap()
    }

    #[test]
    fn register_then_get() {
        let mut reg = Registry::new();
        reg.register(action("action://vibe.tree/copy", "Copy"))
            .unwrap();
        assert!(reg.get(&addr("action://vibe.tree/copy")).is_some());
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn duplicate_registration_is_a_collision_naming_both() {
        let mut reg = Registry::new();
        reg.register(action("action://vibe.tree/copy", "Copy Incumbent"))
            .unwrap();
        let err = reg
            .register(action("action://vibe.tree/copy", "Copy Newcomer"))
            .unwrap_err();
        match err {
            RegistryError::Collision {
                addr,
                incumbent,
                newcomer,
            } => {
                assert_eq!(addr, "action://vibe.tree/copy");
                assert_eq!(incumbent, "Copy Incumbent");
                assert_eq!(newcomer, "Copy Newcomer");
            }
            other => panic!("expected Collision, got {other:?}"),
        }
    }

    #[test]
    fn alias_resolves_to_the_target() {
        let mut reg = Registry::new();
        reg.register(action(
            "action://vibe.tree/copy.markdown",
            "Copy as Markdown",
        ))
        .unwrap();
        reg.alias(
            addr("action://vibe.tree/copy.md"), // retired short name
            addr("action://vibe.tree/copy.markdown"),
        )
        .unwrap();

        let via_alias = reg.get(&addr("action://vibe.tree/copy.md")).unwrap();
        assert_eq!(
            via_alias.presentation().name().default_en(),
            "Copy as Markdown"
        );
        assert_eq!(
            reg.resolve_alias(&addr("action://vibe.tree/copy.md"))
                .unwrap(),
            &addr("action://vibe.tree/copy.markdown")
        );
    }

    #[test]
    fn alias_to_missing_target_fails() {
        let mut reg = Registry::new();
        let err = reg
            .alias(
                addr("action://vibe.tree/old"),
                addr("action://vibe.tree/nonexistent"),
            )
            .unwrap_err();
        assert!(matches!(err, RegistryError::AliasTargetMissing { .. }));
    }

    #[test]
    fn alias_over_existing_address_collides() {
        let mut reg = Registry::new();
        reg.register(action("action://vibe.tree/a", "A")).unwrap();
        reg.register(action("action://vibe.tree/b", "B")).unwrap();
        // `a` is already an action, cannot become an alias.
        let err = reg
            .alias(addr("action://vibe.tree/a"), addr("action://vibe.tree/b"))
            .unwrap_err();
        assert!(matches!(err, RegistryError::AliasCollision { .. }));
    }

    #[test]
    fn registering_over_an_alias_collides() {
        let mut reg = Registry::new();
        reg.register(action("action://vibe.tree/copy.markdown", "Copy"))
            .unwrap();
        reg.alias(
            addr("action://vibe.tree/copy.md"),
            addr("action://vibe.tree/copy.markdown"),
        )
        .unwrap();
        let err = reg
            .register(action("action://vibe.tree/copy.md", "Clash"))
            .unwrap_err();
        assert!(matches!(err, RegistryError::AliasCollision { .. }));
    }

    #[test]
    fn check_ref_validates_referential_integrity() {
        let mut reg = Registry::new();
        reg.register(action("action://vibe.tree/copy.markdown", "Copy"))
            .unwrap();
        reg.alias(
            addr("action://vibe.tree/copy.md"),
            addr("action://vibe.tree/copy.markdown"),
        )
        .unwrap();
        assert!(reg.check_ref(&addr("action://vibe.tree/copy.markdown"))); // direct
        assert!(reg.check_ref(&addr("action://vibe.tree/copy.md"))); // via alias
        assert!(!reg.check_ref(&addr("action://vibe.tree/ghost"))); // dangling
    }

    #[test]
    fn override_of_replaces_only_an_incumbent() {
        let mut reg = Registry::new();
        reg.register(action("action://vibe.tree/copy", "Original"))
            .unwrap();
        reg.override_of(action("action://vibe.tree/copy", "Replacement"))
            .unwrap();
        assert_eq!(
            reg.get(&addr("action://vibe.tree/copy"))
                .unwrap()
                .presentation()
                .name()
                .default_en(),
            "Replacement"
        );

        // Nothing at this address — override refuses.
        let err = reg
            .override_of(action("action://vibe.tree/absent", "X"))
            .unwrap_err();
        assert!(matches!(err, RegistryError::OverrideMissing { .. }));
    }

    #[test]
    fn iter_enumerates_all_actions_in_address_order() {
        let mut reg = Registry::new();
        reg.register(action("action://vibe.tree/sort", "Sort"))
            .unwrap();
        reg.register(action("action://vibe.tree/copy", "Copy"))
            .unwrap();
        reg.register(action("action://core/quit", "Quit")).unwrap();

        let names: Vec<_> = reg
            .iter()
            .map(|a| a.presentation().name().default_en())
            .collect();
        // BTreeMap orders by (group, name): core/quit, vibe.tree/copy, vibe.tree/sort.
        assert_eq!(names, vec!["Quit", "Copy", "Sort"]);
        assert_eq!(reg.iter().count(), 3);
    }

    #[test]
    fn get_direct_ignores_aliases() {
        let mut reg = Registry::new();
        reg.register(action("action://vibe.tree/copy.markdown", "Copy"))
            .unwrap();
        reg.alias(
            addr("action://vibe.tree/copy.md"),
            addr("action://vibe.tree/copy.markdown"),
        )
        .unwrap();
        assert!(
            reg.get_direct(&addr("action://vibe.tree/copy.md"))
                .is_none()
        );
        assert!(reg.get(&addr("action://vibe.tree/copy.md")).is_some());
    }

    #[test]
    fn capability_is_carried_through_registration() {
        let mut reg = Registry::new();
        let danger = Action::builder(addr("action://vibe.tree/wipe"))
            .name_en("Wipe")
            .description_en("dangerous")
            .capability(Capability::Dangerous)
            .invoke(|_c, _v| Box::pin(async { Ok(InvokeOutcome::Done) }))
            .build()
            .unwrap();
        reg.register(danger).unwrap();
        assert_eq!(
            reg.get(&addr("action://vibe.tree/wipe"))
                .unwrap()
                .capability(),
            Capability::Dangerous
        );
    }
}
