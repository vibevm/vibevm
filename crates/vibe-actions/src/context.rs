//! The typed context snapshot and the enablement verdict (PROP-039 §6).
//!
//! [`Ctx`] is a `TypeId`-keyed typemap: a surface publishes strongly-typed
//! values (the current selection, the active mode, …) and readers recover them
//! by type with `get::<T>()` — no stringly keys, no unchecked casts (§6.1,
//! closing VSCode's stringly `when` and IntelliJ's phantom `DataKey`). It is
//! **introspectable**: [`Ctx::keys`] enumerates the present type names (§6.3).
//!
//! [`Enablement`] is the pure verdict an action's predicate returns over a
//! `Ctx` — the independent `visible` / `enabled` axes plus an optional
//! localized `reason` ("why disabled", §6.2). Pure: no rendering, no threads,
//! no mutation.
//!
//! Spec: [PROP-039 §6](../../../../spec/modules/vibe-actions/PROP-039-action-system.md#context).

specmark::scope!("spec://vibevm/modules/vibe-actions/PROP-039#context");

use std::any::{Any, TypeId, type_name};
use std::collections::HashMap;

use crate::i18n::Localized;

/// One stored context value plus the type name kept for introspection.
struct Entry {
    type_name: &'static str,
    value: Box<dyn Any + Send + Sync>,
}

/// A typed context snapshot — a `TypeId`-keyed typemap holding at most one
/// value per type. Immutable in spirit (a surface builds it, readers only
/// borrow); `Send + Sync` so a headless surface can carry it across threads.
#[derive(Default)]
pub struct Ctx {
    entries: HashMap<TypeId, Entry>,
}

impl Ctx {
    /// An empty context.
    pub fn new() -> Self {
        Ctx::default()
    }

    /// Publish `value`, replacing any prior value of the same type. Chaining
    /// form: see [`Ctx::with`].
    pub fn insert<T: Any + Send + Sync>(&mut self, value: T) {
        self.entries.insert(
            TypeId::of::<T>(),
            Entry {
                type_name: type_name::<T>(),
                value: Box::new(value),
            },
        );
    }

    /// Publish `value`, chaining.
    #[must_use]
    pub fn with<T: Any + Send + Sync>(mut self, value: T) -> Self {
        self.insert(value);
        self
    }

    /// Borrow the value of type `T`, if present, with its real type recovered.
    pub fn get<T: Any>(&self) -> Option<&T> {
        self.entries
            .get(&TypeId::of::<T>())
            .and_then(|entry| entry.value.downcast_ref::<T>())
    }

    /// Whether a value of type `T` is present.
    pub fn contains<T: Any>(&self) -> bool {
        self.entries.contains_key(&TypeId::of::<T>())
    }

    /// The type names of the values present, sorted for deterministic
    /// introspection (§6.3).
    pub fn keys(&self) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = self.entries.values().map(|e| e.type_name).collect();
        names.sort_unstable();
        names
    }

    /// The number of values present.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the context is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// The verdict an action's enablement predicate returns over a [`Ctx`]
/// (PROP-039 §6.2). `visible` (hide) and `enabled` (grey-out) are two
/// independent axes; `reason` carries the localized "why disabled" text a
/// surface (and the AIUI) can show (§6.3).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Enablement {
    /// Whether the action is shown at all.
    pub visible: bool,
    /// Whether the action can be invoked.
    pub enabled: bool,
    /// The localized reason it is disabled, if it is.
    pub reason: Option<Localized>,
}

impl Enablement {
    /// Visible and enabled — the common case.
    pub fn enabled() -> Self {
        Enablement {
            visible: true,
            enabled: true,
            reason: None,
        }
    }

    /// Hidden (and therefore not enabled) — the visibility axis is off.
    pub fn hidden() -> Self {
        Enablement {
            visible: false,
            enabled: false,
            reason: None,
        }
    }

    /// Visible but greyed out, with a localized reason.
    pub fn disabled(reason: impl Into<Localized>) -> Self {
        Enablement {
            visible: true,
            enabled: false,
            reason: Some(reason.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Selection(Vec<String>);

    #[derive(Debug, PartialEq)]
    struct Mode(&'static str);

    #[test]
    fn insert_and_get_recover_the_real_type() {
        let mut ctx = Ctx::new();
        ctx.insert(Selection(vec!["a".into(), "b".into()]));
        ctx.insert(Mode("browse"));

        assert_eq!(ctx.get::<Selection>().unwrap().0.len(), 2);
        assert_eq!(ctx.get::<Mode>().unwrap().0, "browse");
    }

    #[test]
    fn get_missing_type_is_none() {
        let ctx = Ctx::new();
        assert!(ctx.get::<Selection>().is_none());
        assert!(!ctx.contains::<Selection>());
    }

    #[test]
    fn insert_replaces_same_type() {
        let ctx = Ctx::new().with(Mode("browse")).with(Mode("search"));
        assert_eq!(ctx.get::<Mode>().unwrap().0, "search");
        assert_eq!(ctx.len(), 1);
    }

    #[test]
    fn keys_introspect_present_type_names() {
        let ctx = Ctx::new().with(Selection(vec![])).with(Mode("browse"));
        let keys = ctx.keys();
        assert_eq!(keys.len(), 2);
        assert!(keys.iter().any(|k| k.contains("Selection")));
        assert!(keys.iter().any(|k| k.contains("Mode")));
    }

    #[test]
    fn empty_context_reports_empty() {
        let ctx = Ctx::new();
        assert!(ctx.is_empty());
        assert_eq!(ctx.len(), 0);
        assert!(ctx.keys().is_empty());
    }

    #[test]
    fn enablement_constructors() {
        assert!(Enablement::enabled().enabled);
        assert!(!Enablement::hidden().visible);
        let d = Enablement::disabled("no selection");
        assert!(d.visible && !d.enabled);
        assert_eq!(d.reason.unwrap().as_str(), "no selection");
    }
}
