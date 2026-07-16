//! Change events, `applies`, and the file-watch abstraction (PROP-040 §10
//! `#events`).
//!
//! When a layer mutates — a `vibe prefs set`, an `$EDITOR` edit, a `git pull`
//! that changes L2 — the resolver emits a **granular change event** so a
//! subscriber can react to *only the keys it owns* (§10 `#change-events`):
//! a TUI component subscribed to `tree.*` re-renders only when `tree.*`
//! changes, not on every keystroke (the VSCode `IConfigurationChangeEvent`
//! pattern, clean-room). Each affected key also carries an [`Applies`] tag
//! (§10 `#applies`) so a surface can show "needs restart" instead of leaving
//! the user guessing (the hot-reload-vs-restart pain, §4.3.2).
//!
//! ## Frontend-agnostic file-watch (PROP-040 §10 #file-watch)
//!
//! The contract says layer files are watched and an external edit reloads +
//! re-resolves. The watch *backend* (the `notify` crate, fs events, debounce)
//! is a **host concern** — `vibe-settings` is frontend-agnostic (PROP-040 §1
//! `#frontend-agnostic`) and pulls no `notify`/`ratatui`/`crossterm` dep. So
//! [`Watcher`] is a **trait** (the contract), the host (`vibe-cli`) provides
//! the concrete `notify`-based impl, and tests use a no-op/mock. This keeps
//! the data layer testable in isolation and leaves the host free to pick its
//! fs-event engine.
//!
//! ## AI-Native Rust discipline (PROP-040 §13)
//!
//! One cell, one `scope!` (`#events`); `#[specmark::spec(implements = "…")]`
//! on the public seams; no `unwrap`/`expect` in domain logic; ≤600-line file
//! budget; doctests on every public seam (the [`Watcher`] trait is exercised
//! through a mock impl).
//!
//! Spec: [PROP-040 §10](../../../../../spec/modules/vibe-settings/PROP-040-settings.md#events).

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-040#events");

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::loader::Layer;
use crate::schema::{Applies, Schema};

// ── ChangeEvent ─────────────────────────────────────────────────────────────

/// A granular change event — which keys changed, in which file layer
/// (PROP-040 §10 `#change-events`). The resolver emits one when a layer
/// mutates; a subscriber filters by [`Self::affects`] (prefix-match) so a TUI
/// component re-renders only its own namespace, not the whole surface.
///
/// `source_layer` is the **file layer** that changed (L1/L2/L3). CLI/env
/// overrides are not file layers and do not carry a `ChangeEvent` — the host
/// applies those directly on the next resolve.
///
/// ```
/// use std::collections::BTreeSet;
/// use vibe_settings::events::ChangeEvent;
/// use vibe_settings::loader::Layer;
///
/// let mut keys = BTreeSet::new();
/// keys.insert("tree.palette".to_string());
/// keys.insert("tree.mode".to_string());
/// let ev = ChangeEvent { affected_keys: keys, source_layer: Layer::L2 };
///
/// // The `tree.*` component cares; the `node.*` one does not.
/// assert!(ev.affects("tree"));           // prefix-match catches both
/// assert!(ev.affects("tree.palette"));   // exact match
/// assert!(!ev.affects("node"));          // disjoint namespace
/// assert!(!ev.is_empty());
/// ```
#[derive(Debug, Clone)]
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#change-events")]
pub struct ChangeEvent {
    /// The dotted paths whose resolved value may have changed (PROP-040 §10
    /// `#change-events`). A key is listed when the layer's value for it
    /// changed; a subscriber reads its own subset via [`Self::affects`].
    pub affected_keys: BTreeSet<String>,
    /// The file layer that mutated (L1/L2/L3). CLI/env overrides do not emit a
    /// `ChangeEvent` — see the type docs.
    pub source_layer: Layer,
}

impl ChangeEvent {
    /// An empty event for a layer — the "load touched zero keys" path. Affects
    /// nothing; subscribers skip it. Convenient where a layer reload returns a
    /// diff that happens to be empty.
    #[must_use]
    pub fn empty(layer: Layer) -> Self {
        ChangeEvent {
            affected_keys: BTreeSet::new(),
            source_layer: layer,
        }
    }

    /// Whether no keys are affected. An empty event is still well-defined (it
    /// names a layer that changed but touched nothing observable) — subscribers
    /// skip it.
    pub fn is_empty(&self) -> bool {
        self.affected_keys.is_empty()
    }

    /// Whether this event touches the caller's namespace (PROP-040 §10
    /// `#change-events` — a subscriber filters by `affects(namespace)`). A key
    /// matches when it equals `prefix` exactly or sits under `prefix.*`, so a
    /// component subscribed to `"tree"` hears about `tree`, `tree.palette`,
    /// and `tree.mode` but not `node.fold`.
    ///
    /// An empty `prefix` is the root namespace and matches any non-empty event.
    ///
    /// ```
    /// use std::collections::BTreeSet;
    /// use vibe_settings::events::ChangeEvent;
    /// use vibe_settings::loader::Layer;
    ///
    /// let ev = ChangeEvent {
    ///     affected_keys: ["tree.palette", "node.fold"]
    ///         .iter().map(|s| s.to_string()).collect(),
    ///     source_layer: Layer::L3,
    /// };
    /// assert!(ev.affects("tree"));          // tree.palette ∈ tree.*
    /// assert!(ev.affects("tree.palette"));  // exact
    /// assert!(ev.affects("node"));          // node.fold ∈ node.*
    /// assert!(!ev.affects("edge"));         // nothing under edge.*
    /// assert!(ev.affects(""));              // root namespace → any key
    /// ```
    #[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#change-events")]
    pub fn affects(&self, prefix: &str) -> bool {
        if self.affected_keys.is_empty() {
            return false;
        }
        if prefix.is_empty() {
            // Root namespace subscription — any affected key matches.
            return true;
        }
        let dotted = format!("{prefix}.");
        self.affected_keys
            .iter()
            .any(|k| k == prefix || k.starts_with(&dotted))
    }
}

// ── applies helpers ─────────────────────────────────────────────────────────

/// The [`Applies`] tag a key declares, or [`Applies::Live`] when the key is not
/// in the schema (PROP-040 §10 `#applies`). `Live` is the safe default — most
/// preferences take effect immediately — and it is what an unknown (typo, or
/// not-yet-registered) key gets so a change to it never blocks the user behind
/// a false "needs restart".
///
/// ```
/// use vibe_settings::events::applies_of;
/// use vibe_settings::schema::{Applies, KeyMeta, KeyType, Schema, Scope};
///
/// let mut schema = Schema::new();
/// schema.register(
///     KeyMeta::new("tree.palette", KeyType::String, Scope::User, "palette")?
///         .with_applies(Applies::Reload),
/// )?;
///
/// assert_eq!(applies_of(&schema, "tree.palette"), Applies::Reload);
/// // Unknown key → Live (the safe default).
/// assert_eq!(applies_of(&schema, "tree.ghost"), Applies::Live);
/// # Ok::<(), vibe_settings::schema::SchemaError>(())
/// ```
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#applies")]
pub fn applies_of(schema: &Schema, key: &str) -> Applies {
    schema
        .get(key)
        .map(|meta| meta.applies)
        .unwrap_or(Applies::Live)
}

/// The worst-case [`Applies`] across an event's affected keys
/// (PROP-040 §10 `#applies`) — the indicator a surface shows: a change needs a
/// **restart** if any touched key needs one, a **reload** if any needs a reload
/// (and none a restart), and is fully **live** only when every touched key is
/// live.
///
/// Unknown keys contribute [`Applies::Live`] (see [`applies_of`]), so an event
/// touching only unknown keys is "live". An empty event is "live" too (nothing
/// changed → nothing to indicate). Order: `Restart` beats `Reload` beats
/// `Live`.
///
/// ```
/// use std::collections::BTreeSet;
/// use vibe_settings::events::{affected_applies, ChangeEvent};
/// use vibe_settings::loader::Layer;
/// use vibe_settings::schema::{Applies, KeyMeta, KeyType, Schema, Scope};
///
/// let mut schema = Schema::new();
/// schema.register(
///     KeyMeta::new("tree.palette", KeyType::String, Scope::User, "palette")?
///         .with_applies(Applies::Live),
/// )?;
/// schema.register(
///     KeyMeta::new("node.sort", KeyType::String, Scope::User, "sort")?
///         .with_applies(Applies::Restart),
/// )?;
///
/// let mut keys = BTreeSet::new();
/// keys.insert("tree.palette".to_string());
/// keys.insert("node.sort".to_string());
/// let ev = ChangeEvent { affected_keys: keys, source_layer: Layer::L2 };
///
/// // One Restart key → the whole event needs a restart.
/// assert_eq!(affected_applies(&ev, &schema), Applies::Restart);
/// # Ok::<(), vibe_settings::schema::SchemaError>(())
/// ```
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#applies")]
pub fn affected_applies(event: &ChangeEvent, schema: &Schema) -> Applies {
    let mut worst = Applies::Live;
    for key in &event.affected_keys {
        worst = combine_applies(worst, applies_of(schema, key));
        // Restart is the ceiling — nothing outranks it; stop early.
        if matches!(worst, Applies::Restart) {
            break;
        }
    }
    worst
}

/// Max of two [`Applies`] tags by user-facing disruption (`Live < Reload <
/// `Restart`). Local to this cell so the schema cell (phase 2.4) need not
/// derive `Ord` on `Applies` — the precedence is fixed in one place here.
fn combine_applies(a: Applies, b: Applies) -> Applies {
    match (a, b) {
        (Applies::Restart, _) | (_, Applies::Restart) => Applies::Restart,
        (Applies::Reload, _) | (_, Applies::Reload) => Applies::Reload,
        _ => Applies::Live,
    }
}

// ── EventEmitter ────────────────────────────────────────────────────────────

/// A subscriber callback. `Send + Sync` so an [`EventEmitter`] can be shared
/// across threads by the host (the TUI runs the file-watcher on a background
/// thread and emits on the UI thread). Boxed because the emitter owns a
/// variadic list of closures.
pub type Subscriber = Box<dyn Fn(&ChangeEvent) + Send + Sync>;

/// Synchronous fan-out of [`ChangeEvent`]s to registered subscribers
/// (PROP-040 §10 `#change-events`). The resolver emits; the host (TUI, future
/// GUI, AIUI) subscribes. Emission is **synchronous** — [`Self::emit`] calls
/// each subscriber in registration order on the calling thread; async,
/// debounce, and cross-thread dispatch are host responsibilities (the data
/// layer does not pick a runtime).
///
/// Thread-safety: subscribers are `Send + Sync`, so an `EventEmitter` itself is
/// `Send + Sync` and the host can wrap it in whatever channel/lock its runtime
/// prefers.
///
/// ```
/// use std::collections::BTreeSet;
/// use std::sync::atomic::{AtomicUsize, Ordering};
/// use std::sync::Arc;
/// use vibe_settings::events::{ChangeEvent, EventEmitter};
/// use vibe_settings::loader::Layer;
///
/// let mut emitter = EventEmitter::new();
/// let fired = Arc::new(AtomicUsize::new(0));
/// let counter = Arc::clone(&fired);
/// emitter.subscribe(move |_ev| {
///     counter.fetch_add(1, Ordering::SeqCst);
/// });
///
/// let ev = ChangeEvent {
///     affected_keys: BTreeSet::from(["tree.palette".to_string()]),
///     source_layer: Layer::L2,
/// };
/// emitter.emit(&ev);
/// emitter.emit(&ev);
/// assert_eq!(fired.load(Ordering::SeqCst), 2);
/// ```
pub struct EventEmitter {
    subscribers: Vec<Subscriber>,
}

impl EventEmitter {
    /// An emitter with no subscribers.
    #[must_use]
    pub fn new() -> Self {
        EventEmitter {
            subscribers: Vec::new(),
        }
    }

    /// How many subscribers are registered (handy in tests).
    pub fn subscriber_count(&self) -> usize {
        self.subscribers.len()
    }

    /// Register a subscriber. It fires for every subsequent [`Self::emit`], in
    /// registration order (PROP-040 §10 `#change-events`). The callback is
    /// `Fn + Send + Sync + 'static` so the emitter can live on any thread.
    pub fn subscribe<F>(&mut self, callback: F)
    where
        F: Fn(&ChangeEvent) + Send + Sync + 'static,
    {
        self.subscribers.push(Box::new(callback));
    }

    /// Deliver `event` to every subscriber, synchronously, in registration
    /// order (PROP-040 §10 `#change-events`). The data layer assumes
    /// well-behaved callbacks; a panicking subscriber would unwind through
    /// `emit`, so the host is responsible for catching its own subscriber
    /// errors.
    #[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#change-events")]
    pub fn emit(&self, event: &ChangeEvent) {
        for sub in &self.subscribers {
            sub(event);
        }
    }
}

impl Default for EventEmitter {
    fn default() -> Self {
        Self::new()
    }
}

// ── Watcher trait (file-watch abstraction — frontend-agnostic) ──────────────

/// Why a [`Watcher`] could not start watching a layer file (PROP-040 §10
/// `#file-watch`). The concrete fs-event backend (the `notify` crate) lives in
/// the host; `vibe-settings` defines only the typed contract so a surface can
/// report the failure without depending on the backend's error type. Each
/// variant cites `#file-watch` so a diagnostic points at the contract clause.
///
/// ```
/// use std::path::PathBuf;
/// use vibe_settings::events::WatchError;
///
/// let e = WatchError::PathMissing { path: PathBuf::from("/x/.vibe/settings.toml") };
/// assert!(e.to_string().contains("file-watch"));
/// assert!(e.to_string().contains("does not exist"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#file-watch")]
pub enum WatchError {
    /// The layer file does not exist, so there is nothing to watch. (A missing
    /// layer is *not* an error for the loader — §3 `#missing-is-default` — but
    /// it is an error for the watcher, which needs a path that exists today.
    /// The host typically catches this and retries after the first edit, or
    /// starts the watch once the file is created.)
    #[error(
        "cannot watch `{path}`: file does not exist \
         (spec://vibevm/modules/vibe-settings/PROP-040#file-watch; \
          fix: create the layer file, or start watching after the first edit)"
    )]
    PathMissing {
        /// The file that could not be watched.
        path: PathBuf,
    },

    /// The host's fs-event backend failed to install a watch (permissions, OS
    /// resource limit, backend-specific error). The opaque `message` carries
    /// the backend's diagnostic — `vibe-settings` does not depend on the
    /// backend, so it surfaces the host's wording verbatim.
    #[error(
        "cannot watch `{path}`: {message} \
         (spec://vibevm/modules/vibe-settings/PROP-040#file-watch; \
          fix: see the host (vibe-cli) watcher-backend diagnostic)"
    )]
    Backend {
        /// The file whose watch failed.
        path: PathBuf,
        /// The host backend's error message (e.g. `notify`'s rendering).
        message: String,
    },
}

/// File-watch abstraction (PROP-040 §10 `#file-watch`). The contract: a layer
/// file is watched, and when it changes on disk, `on_change` is called so the
/// host can reload the layer and re-resolve (the VSCode debounced file-watcher,
/// §2.3 — edit your `~/.vibe/settings.toml` in `$EDITOR` and the TUI picks it
/// up).
///
/// `vibe-settings` is **frontend-agnostic** (PROP-040 §1 `#frontend-agnostic`)
/// and depends on no fs-event library; the concrete impl (the `notify` crate +
/// debounce) lives in the host (`vibe-cli`). Tests use a no-op/mock impl. This
/// keeps the data layer testable in isolation and leaves the host free to pick
/// its fs-event engine.
///
/// `on_change` is `Fn() + Send + Sync` so the host can route the callback
/// across threads (watcher thread → UI thread). The trait method is
/// synchronous: the backend owns its own threading/debounce internally and
/// merely *calls* `on_change` when a real change is observed.
///
/// The doctest exercises the contract through a mock impl — the shape any host
/// (or test double) satisfies:
///
/// ```
/// use std::path::Path;
/// use vibe_settings::events::{Watcher, WatchError};
///
/// // A test double that records the last path and decides whether to fail.
/// struct MockWatcher { fail: bool }
/// impl Watcher for MockWatcher {
///     fn watch(
///         &mut self,
///         path: &Path,
///         _on_change: Box<dyn Fn() + Send + Sync>,
///     ) -> Result<(), WatchError> {
///         if self.fail {
///             return Err(WatchError::PathMissing { path: path.to_path_buf() });
///         }
///         Ok(())
///     }
/// }
///
/// let mut ok = MockWatcher { fail: false };
/// assert!(ok.watch(Path::new("/anywhere/settings.toml"), Box::new(|| {})).is_ok());
///
/// let mut bad = MockWatcher { fail: true };
/// let err = bad
///     .watch(Path::new("/no/such.toml"), Box::new(|| {}))
///     .unwrap_err();
/// assert!(matches!(err, WatchError::PathMissing { .. }));
/// ```
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#file-watch")]
pub trait Watcher {
    /// Begin watching `path`; call `on_change` when the file changes on disk
    /// (PROP-040 §10 `#file-watch`). Returns an error if the watch could not
    /// be installed (see [`WatchError`]). The backend owns debounce and its own
    /// threading; `on_change` fires only for a real, settled change.
    #[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#file-watch")]
    fn watch(
        &mut self,
        path: &Path,
        on_change: Box<dyn Fn() + Send + Sync>,
    ) -> Result<(), WatchError>;
}

#[cfg(test)]
mod tests;
