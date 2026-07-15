//! The enumerable schema registry — every declared setting key in one typed,
//! tagged tree (PROP-040 §5 `#unified-introspection`, §6 `#schema-first`).
//!
//! Backed by a `BTreeMap` so enumeration is deterministic (path order) —
//! friendly to goldens and the AIUI `list_settings` surface. An agent (or test,
//! or CLI) reaches the whole surface through [`Schema::keys`],
//! [`Schema::paths_in`], and [`Schema::get`] **without parsing files** — the
//! introspection bottleneck of IntelliJ (§3.9) is avoided by construction.
//!
//! Spec: [PROP-040 §5, §6](../../../../../../spec/modules/vibe-settings/PROP-040-settings.md#unified-introspection).

use std::collections::BTreeMap;

use super::types::{KeyMeta, SchemaError};

/// The enumerable schema registry (PROP-040 §5 `#unified-introspection`,
/// §6 `#schema-first`).
///
/// ```
/// use vibe_settings::schema::{KeyMeta, KeyType, Schema, Scope};
///
/// let mut schema = Schema::new();
/// schema.register(KeyMeta::new("tree.palette", KeyType::String, Scope::User, "palette")?)?;
/// schema.register(KeyMeta::new("tree.mode", KeyType::String, Scope::User, "display mode")?)?;
///
/// assert!(schema.contains("tree.palette"));
/// assert!(!schema.contains("tree.unknown"));
/// // Section access — all keys under `tree.*` (PROP-040 §5 #get-section).
/// let tree_keys: Vec<&str> = schema.paths_in("tree").collect();
/// assert_eq!(tree_keys, vec!["tree.mode", "tree.palette"]);
/// # Ok::<(), vibe_settings::schema::SchemaError>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct Schema {
    keys: BTreeMap<String, KeyMeta>,
}

impl Schema {
    /// An empty schema.
    pub fn new() -> Self {
        Schema::default()
    }

    /// Register a key. A duplicate path is a hard [`SchemaError::DuplicateKey`]
    /// — a *collision*, never a silent override (PROP-040 §6 `#schema-first`).
    #[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#schema-first")]
    pub fn register(&mut self, meta: KeyMeta) -> Result<(), SchemaError> {
        if self.keys.contains_key(&meta.path) {
            return Err(SchemaError::DuplicateKey {
                path: meta.path.clone(),
            });
        }
        // Checked above — contain-then-insert keeps the borrow-checker happy
        // without an extra clone of `meta`.
        self.keys.insert(meta.path.clone(), meta);
        Ok(())
    }

    /// The metadata for `path`, if declared.
    pub fn get(&self, path: &str) -> Option<&KeyMeta> {
        self.keys.get(path)
    }

    /// Whether `path` is a declared setting key.
    pub fn contains(&self, path: &str) -> bool {
        self.keys.contains_key(path)
    }

    /// Every declared path, in sorted order (PROP-040 §5
    /// `#unified-introspection`).
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.keys.keys().map(String::as_str)
    }

    /// Every path under a namespace — `paths_in("tree")` yields `tree.palette`,
    /// `tree.mode`, etc. (PROP-040 §5 `#get-section`). The namespace key
    /// itself, if declared, is included.
    pub fn paths_in(&self, namespace: &str) -> impl Iterator<Item = &str> {
        // Two predicates: the namespace itself (e.g. `tree` as a Table key),
        // or any `namespace.<leaf>` child. Build the prefix once; the closure
        // moves both owned strings.
        let ns = namespace.to_owned();
        let prefix = format!("{ns}.");
        self.keys.keys().filter_map(move |path| {
            let path = path.as_str();
            (path == ns || path.starts_with(&prefix)).then_some(path)
        })
    }

    /// The number of declared keys.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Whether no keys are declared.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{KeyType, Scope};

    fn key(path: &str, ty: KeyType, scope: Scope) -> KeyMeta {
        KeyMeta::new(path, ty, scope, "a test setting").unwrap()
    }

    #[test]
    fn schema_register_then_get_contains_keys() {
        let mut schema = Schema::new();
        assert!(schema.is_empty());
        schema
            .register(key("tree.palette", KeyType::String, Scope::User))
            .unwrap();
        schema
            .register(key("tree.mode", KeyType::String, Scope::User))
            .unwrap();
        assert_eq!(schema.len(), 2);
        assert!(schema.contains("tree.palette"));
        assert!(schema.get("tree.mode").is_some());
        assert!(schema.get("ghost").is_none());
        let all: Vec<&str> = schema.keys().collect();
        assert_eq!(all, vec!["tree.mode", "tree.palette"]); // sorted
    }

    #[test]
    fn schema_register_duplicate_is_a_hard_error() {
        let mut schema = Schema::new();
        schema
            .register(key("tree.palette", KeyType::String, Scope::User))
            .unwrap();
        let err = schema
            .register(key("tree.palette", KeyType::String, Scope::User))
            .unwrap_err();
        // Capture the Display string before the match moves `err`'s `path`.
        let msg = err.to_string();
        match err {
            SchemaError::DuplicateKey { path } => assert_eq!(path, "tree.palette"),
            other => panic!("expected DuplicateKey, got {other:?}"),
        }
        assert!(msg.contains("schema-first"));
        // The duplicate was not stored.
        assert_eq!(schema.len(), 1);
    }

    #[test]
    fn schema_paths_in_is_section_access() {
        let mut schema = Schema::new();
        schema
            .register(key("tree.palette", KeyType::String, Scope::User))
            .unwrap();
        schema
            .register(key("tree.mode", KeyType::String, Scope::User))
            .unwrap();
        schema
            .register(key("node.fold", KeyType::Bool, Scope::User))
            .unwrap();
        // Section access for `tree.*` returns both tree.* keys, sorted, and
        // excludes `node.*`.
        let tree: Vec<&str> = schema.paths_in("tree").collect();
        assert_eq!(tree, vec!["tree.mode", "tree.palette"]);
        // An unknown namespace is empty, not an error.
        assert_eq!(schema.paths_in("ghost").count(), 0);
    }

    #[test]
    fn schema_paths_in_includes_a_table_typed_namespace_key_itself() {
        // If the namespace is itself a registered key (KeyType::Table), the
        // section iterator yields it as well as its declared children.
        let mut schema = Schema::new();
        schema
            .register(key("tree", KeyType::Table, Scope::User))
            .unwrap();
        schema
            .register(key("tree.palette", KeyType::String, Scope::User))
            .unwrap();
        let tree: Vec<&str> = schema.paths_in("tree").collect();
        assert_eq!(tree, vec!["tree", "tree.palette"]);
    }
}
