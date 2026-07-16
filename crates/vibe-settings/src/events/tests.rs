//! Tests for the events cell — `ChangeEvent`, `applies`, `EventEmitter`,
//! `Watcher`/`WatchError` (PROP-040 §10 `#events` REQs).
//!
//! Split out of `mod.rs` to honour the ≤600-line AI-Native file budget.
//! Non-`#[test]` helpers carry `#[cfg(test)]` so file-grain scanners (the
//! conform frontend) scope their `unwrap`s as test code.

use super::*;
use crate::schema::{KeyMeta, KeyType, Scope};
use std::sync::{Arc, Mutex};

#[cfg(test)]
fn key(path: &str, ty: KeyType, scope: Scope, applies: Applies) -> KeyMeta {
    KeyMeta::new(path, ty, scope, "a test setting")
        .unwrap()
        .with_applies(applies)
}

#[cfg(test)]
fn schema_with(keys: &[KeyMeta]) -> Schema {
    let mut schema = Schema::new();
    for k in keys {
        schema.register(k.clone()).unwrap();
    }
    schema
}

#[cfg(test)]
fn event(layer: Layer, keys: &[&str]) -> ChangeEvent {
    ChangeEvent {
        affected_keys: keys.iter().map(|s| s.to_string()).collect(),
        source_layer: layer,
    }
}

// ── ChangeEvent::affects / is_empty (§10 #change-events) ──────────────────

#[test]
fn affects_prefix_matches_exact_and_namespace_children() {
    let ev = event(Layer::L2, &["tree.palette", "tree.mode", "node.fold"]);
    assert!(ev.affects("tree")); // namespace
    assert!(ev.affects("tree.palette")); // exact leaf
    assert!(ev.affects("tree.mode"));
    assert!(ev.affects("node")); // other namespace
    assert!(ev.affects("node.fold"));
    assert!(!ev.affects("edgy")); // would-be prefix, but nothing matches
    // A key whose name merely *starts with* the prefix string is NOT a
    // match unless it equals it or sits under `<prefix>.`.
    let ev2 = event(Layer::L1, &["treehouse.color"]);
    assert!(!ev2.affects("tree")); // `treehouse` ≠ `tree` and not `tree.*`
}

#[test]
fn affects_root_prefix_matches_any_nonempty_event() {
    let ev = event(Layer::L3, &["a.b", "c.d"]);
    assert!(ev.affects("")); // root → any key
}

#[test]
fn affects_empty_event_matches_nothing_even_root() {
    let ev = ChangeEvent::empty(Layer::L2);
    assert!(ev.is_empty());
    assert!(!ev.affects("tree"));
    assert!(!ev.affects("")); // no keys at all → not even root
}

#[test]
fn empty_constructor_carries_layer_and_no_keys() {
    let ev = ChangeEvent::empty(Layer::L3);
    assert_eq!(ev.source_layer, Layer::L3);
    assert!(ev.affected_keys.is_empty());
    assert!(ev.is_empty());
}

// ── applies_of (§10 #applies) ─────────────────────────────────────────────

#[test]
fn applies_of_reads_declared_tag() {
    let schema = schema_with(&[
        key(
            "tree.palette",
            KeyType::String,
            Scope::User,
            Applies::Reload,
        ),
        key("node.fold", KeyType::Bool, Scope::User, Applies::Restart),
    ]);
    assert_eq!(applies_of(&schema, "tree.palette"), Applies::Reload);
    assert_eq!(applies_of(&schema, "node.fold"), Applies::Restart);
}

#[test]
fn applies_of_defaults_to_live_for_unknown_key() {
    let schema = schema_with(&[]);
    assert_eq!(applies_of(&schema, "anything.undeclared"), Applies::Live);
}

// ── affected_applies (§10 #applies — worst-case indicator) ────────────────

#[test]
fn affected_applies_restart_wins_over_everything() {
    let schema = schema_with(&[
        key("a.live", KeyType::Bool, Scope::User, Applies::Live),
        key("b.reload", KeyType::Bool, Scope::User, Applies::Reload),
        key("c.restart", KeyType::Bool, Scope::User, Applies::Restart),
    ]);
    let ev = event(Layer::L2, &["a.live", "b.reload", "c.restart"]);
    assert_eq!(affected_applies(&ev, &schema), Applies::Restart);
}

#[test]
fn affected_applies_reload_when_no_restart() {
    let schema = schema_with(&[
        key("a.live", KeyType::Bool, Scope::User, Applies::Live),
        key("b.reload", KeyType::String, Scope::User, Applies::Reload),
    ]);
    let ev = event(Layer::L1, &["a.live", "b.reload"]);
    assert_eq!(affected_applies(&ev, &schema), Applies::Reload);
}

#[test]
fn affected_applies_live_when_all_live() {
    let schema = schema_with(&[
        key("a.live", KeyType::Bool, Scope::User, Applies::Live),
        key("b.live", KeyType::Int, Scope::User, Applies::Live),
    ]);
    let ev = event(Layer::L3, &["a.live", "b.live"]);
    assert_eq!(affected_applies(&ev, &schema), Applies::Live);
}

#[test]
fn affected_applies_unknown_keys_default_live_so_event_is_live() {
    // Unknown keys contribute Live (applies_of), so a wholly-unknown event
    // is "live" — never falsely blocks the user behind a restart prompt.
    let schema = schema_with(&[]);
    let ev = event(Layer::L2, &["ghost.one", "ghost.two"]);
    assert_eq!(affected_applies(&ev, &schema), Applies::Live);
}

#[test]
fn affected_applies_empty_event_is_live() {
    let schema = schema_with(&[key(
        "x.restart",
        KeyType::Bool,
        Scope::User,
        Applies::Restart,
    )]);
    let ev = ChangeEvent::empty(Layer::L2);
    assert_eq!(affected_applies(&ev, &schema), Applies::Live);
}

#[test]
fn combine_applies_precedence_is_restart_then_reload_then_live() {
    assert_eq!(combine_applies(Applies::Live, Applies::Live), Applies::Live);
    assert_eq!(
        combine_applies(Applies::Live, Applies::Reload),
        Applies::Reload
    );
    assert_eq!(
        combine_applies(Applies::Reload, Applies::Live),
        Applies::Reload
    );
    assert_eq!(
        combine_applies(Applies::Reload, Applies::Restart),
        Applies::Restart
    );
    assert_eq!(
        combine_applies(Applies::Restart, Applies::Live),
        Applies::Restart
    );
}

// ── EventEmitter (§10 #change-events) ─────────────────────────────────────

#[test]
fn emit_fires_subscribers_in_registration_order() {
    let mut emitter = EventEmitter::new();
    let order = Arc::new(Mutex::new(Vec::new()));
    {
        let o = Arc::clone(&order);
        emitter.subscribe(move |_ev| o.lock().unwrap().push(1));
    }
    {
        let o = Arc::clone(&order);
        emitter.subscribe(move |_ev| o.lock().unwrap().push(2));
    }
    assert_eq!(emitter.subscriber_count(), 2);

    emitter.emit(&event(Layer::L1, &["tree.palette"]));
    assert_eq!(*order.lock().unwrap(), vec![1, 2]);
}

#[test]
fn emit_with_no_subscribers_is_a_noop() {
    let emitter = EventEmitter::new();
    emitter.emit(&event(Layer::L2, &["tree.palette"])); // must not panic
    assert_eq!(emitter.subscriber_count(), 0);
}

#[test]
fn subscriber_receives_the_event_payload() {
    let mut emitter = EventEmitter::new();
    let seen = Arc::new(Mutex::new(None));
    {
        let s = Arc::clone(&seen);
        emitter.subscribe(move |ev| *s.lock().unwrap() = Some(ev.clone()));
    }
    let ev = event(Layer::L3, &["tree.palette", "tree.mode"]);
    emitter.emit(&ev);
    let got = seen.lock().unwrap().clone().unwrap();
    assert_eq!(got.source_layer, Layer::L3);
    assert!(got.affects("tree"));
    assert!(got.affected_keys.contains("tree.palette"));
    assert!(got.affected_keys.contains("tree.mode"));
}

#[test]
fn emitter_default_is_empty() {
    let emitter = EventEmitter::default();
    assert_eq!(emitter.subscriber_count(), 0);
}

// ── Watcher / WatchError (§10 #file-watch) ─────────────────────────────────

#[test]
fn watch_error_path_missing_diagnostic_cites_file_watch() {
    let e = WatchError::PathMissing {
        path: PathBuf::from("/repo/.vibe/settings.toml"),
    };
    let msg = e.to_string();
    assert!(msg.contains("file-watch"));
    assert!(msg.contains("does not exist"));
    assert!(msg.contains("/repo/.vibe/settings.toml"));
}

#[test]
fn watch_error_backend_carries_host_message() {
    let e = WatchError::Backend {
        path: PathBuf::from("/x/settings.toml"),
        message: "notify: too many watches".to_string(),
    };
    let msg = e.to_string();
    assert!(msg.contains("too many watches"));
    assert!(msg.contains("file-watch"));
}

#[test]
fn watcher_trait_is_satisfiable_by_a_mock_impl() {
    struct Noop;
    impl Watcher for Noop {
        fn watch(
            &mut self,
            _path: &Path,
            _on_change: Box<dyn Fn() + Send + Sync>,
        ) -> Result<(), WatchError> {
            Ok(())
        }
    }
    let mut w = Noop;
    assert!(w.watch(Path::new("/anywhere"), Box::new(|| {})).is_ok());
}
