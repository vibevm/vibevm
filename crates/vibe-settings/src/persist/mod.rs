//! Persistence for settings layers — write only non-default values, and write
//! them without clobbering an operator's comments (PROP-040 §6
//! `#diff-from-default`, §3 `#role-marker`).
//!
//! - [`diff_from_default`] — IntelliJ's `SkipDefaultValuesSerializationFilters`,
//!   clean-room: a layer table reduced to the keys whose values differ from
//!   their schema default. A key equal to its default is omitted; an unknown key
//!   (no schema entry) is passed through; a sub-table that collapses to empty is
//!   dropped. The result is tiny files, clean diffs, and a trivial reset-to-
//!   default (delete the key).
//! - [`write_layer`] — comment-preserving atomic write. An existing file is
//!   loaded as a `toml_edit::DocumentMut` so its header comments (the
//!   role-marker) and footer survive; the body is replaced with the diffed table
//!   and installed via a sibling temp + rename (crash-safe). A new file is
//!   created with the layer's role-marker header + pretty TOML.
//!
//! The cell splits along responsibility seams to honour the ≤600-line AI-Native
//! file budget: the diff algorithm + the public surface here, the write engine
//! in [`write`], and the typed error in [`error`].
//!
//! Frontend-agnostic (PROP-040 §1 `#frontend-agnostic`): `std`, the `toml` crate
//! for diffing, and `toml_edit` for comment-preserving writes — zero rendering
//! deps.
//!
//! Spec: [PROP-040 §6, §3](../../../../spec/modules/vibe-settings/PROP-040-settings.md#diff-from-default).

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-040#diff-from-default");

pub mod error;
pub mod write;

pub use error::PersistError;
pub use write::write_layer;

use crate::schema::Schema;

// ── diff_from_default ───────────────────────────────────────────────────────

/// Reduce a layer table to the keys whose values differ from their schema
/// default (PROP-040 §6 `#diff-from-default`).
///
/// A declared key whose value equals its `default` is **omitted**; an unknown
/// key (no schema entry, or a declared key with no default) is **passed
/// through** — it has no default to differ from, so it stays. Nested tables are
/// diffed recursively, and a sub-table that collapses to empty is dropped
/// entirely (a file that drifts back to byte-identical-with-default becomes an
/// empty layer). The result carries only the non-default overrides, so files
/// stay small, diffs stay clean, and reset-to-default is "delete the key".
///
/// With an empty schema (the phase-2.6 state, before the TUI populates `tree.*`
/// etc.) every key is unknown → every key passes through unchanged; the diff is
/// a no-op. The instant a schema is populated, defaults start being stripped.
///
/// ```
/// use vibe_settings::persist::diff_from_default;
/// use vibe_settings::schema::{KeyMeta, KeyType, Schema, Scope};
///
/// let mut schema = Schema::new();
/// schema.register(
///     KeyMeta::new("tree.palette", KeyType::String, Scope::User, "palette")?
///         .with_default(toml::Value::String("default".into())),
/// )?;
/// schema.register(
///     KeyMeta::new("tree.fold", KeyType::Bool, Scope::User, "fold subtrees")?
///         .with_default(toml::Value::Boolean(true)),
/// )?;
///
/// let mut tree = toml::Table::new();
/// tree.insert("palette".to_string(), toml::Value::String("rosé-pine".into()));
/// tree.insert("fold".to_string(), toml::Value::Boolean(true));
/// tree.insert("ghost".to_string(), toml::Value::Integer(7));
/// let mut t = toml::Table::new();
/// t.insert("tree".to_string(), toml::Value::Table(tree));
///
/// let diff = diff_from_default(&t, &schema);
/// let tree = diff.get("tree").and_then(|v| v.as_table()).unwrap();
/// assert_eq!(tree.get("palette").and_then(|v| v.as_str()), Some("rosé-pine"));
/// assert!(tree.get("fold").is_none(), "value == default → omitted");
/// assert_eq!(tree.get("ghost").and_then(|v| v.as_integer()), Some(7), "unknown → kept");
///
/// // A table whose every child equals its default collapses to empty.
/// let mut t2 = toml::Table::new();
/// let mut tree2 = toml::Table::new();
/// tree2.insert("fold".to_string(), toml::Value::Boolean(true));
/// t2.insert("tree".to_string(), toml::Value::Table(tree2));
/// assert!(diff_from_default(&t2, &schema).is_empty(), "collapses to empty");
/// # Ok::<(), vibe_settings::schema::SchemaError>(())
/// ```
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#diff-from-default")]
pub fn diff_from_default(table: &toml::Table, schema: &Schema) -> toml::Table {
    diff_table(table, schema, String::new())
}

/// The recursive core of [`diff_from_default`], carrying the dotted path prefix
/// accumulated by descent so a leaf can be looked up in the schema by its full
/// dotted path (e.g. `tree.palette`). Pure — builds a fresh table, never mutates
/// the input.
fn diff_table(table: &toml::Table, schema: &Schema, prefix: String) -> toml::Table {
    let mut out = toml::Table::new();
    for (key, value) in table {
        let dotted = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        match value {
            // Nested table: recurse, then drop the sub-table if it collapsed to
            // empty (§6 #diff-from-default — collapse-to-empty).
            toml::Value::Table(inner) => {
                let diffed = diff_table(inner, schema, dotted);
                if !diffed.is_empty() {
                    out.insert(key.clone(), toml::Value::Table(diffed));
                }
            }
            // Scalar/array leaf: omit when it equals the schema default; keep
            // when there is no default to compare against (unknown key, or a
            // declared key with no default → both have no "default" to be).
            leaf => {
                let equals_default = match schema.get(&dotted) {
                    Some(meta) => matches!(&meta.default, Some(default) if default == leaf),
                    None => false,
                };
                if !equals_default {
                    out.insert(key.clone(), leaf.clone());
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{KeyMeta, KeyType, Scope};

    fn schema_with(defaults: &[(&str, toml::Value)]) -> Schema {
        let mut schema = Schema::new();
        for (path, default) in defaults {
            schema
                .register(
                    KeyMeta::new(*path, KeyType::String, Scope::User, "a test setting")
                        .unwrap()
                        .with_default(default.clone()),
                )
                .unwrap();
        }
        schema
    }

    #[test]
    fn diff_omits_scalar_equal_to_default() {
        let schema = schema_with(&[("tree.palette", toml::Value::String("default".into()))]);
        let mut t = toml::Table::new();
        t.insert(
            "tree.palette".to_string(),
            toml::Value::String("default".into()),
        );
        // Flat key (no nesting) → equal to default → omitted → empty.
        assert!(diff_from_default(&t, &schema).is_empty());
    }

    #[test]
    fn diff_keeps_scalar_differing_from_default() {
        let schema = schema_with(&[("tree.palette", toml::Value::String("default".into()))]);
        let mut t = toml::Table::new();
        t.insert(
            "tree.palette".to_string(),
            toml::Value::String("rosé-pine".into()),
        );
        let diff = diff_from_default(&t, &schema);
        assert_eq!(
            diff.get("tree.palette").and_then(|v| v.as_str()),
            Some("rosé-pine")
        );
    }

    #[test]
    fn diff_keeps_unknown_keys_with_empty_schema() {
        // Empty schema → every key unknown → all pass through (phase-2.6 state).
        let schema = Schema::new();
        let mut t = toml::Table::new();
        t.insert("anything".to_string(), toml::Value::Boolean(true));
        let diff = diff_from_default(&t, &schema);
        assert_eq!(diff.get("anything").and_then(|v| v.as_bool()), Some(true));
    }

    #[test]
    fn diff_keeps_declared_key_with_no_default() {
        // A declared key without a default has no "default" to be → kept.
        let mut schema = Schema::new();
        schema
            .register(KeyMeta::new("tree.x", KeyType::Bool, Scope::User, "doc").unwrap())
            .unwrap();
        let mut t = toml::Table::new();
        t.insert("tree.x".to_string(), toml::Value::Boolean(false));
        let diff = diff_from_default(&t, &schema);
        assert_eq!(diff.get("tree.x").and_then(|v| v.as_bool()), Some(false));
    }

    #[test]
    fn diff_recurses_into_nested_tables() {
        let mut schema = Schema::new();
        schema
            .register(
                KeyMeta::new("tree.palette", KeyType::String, Scope::User, "palette")
                    .unwrap()
                    .with_default(toml::Value::String("default".into())),
            )
            .unwrap();
        schema
            .register(
                KeyMeta::new("tree.fold", KeyType::Bool, Scope::User, "fold")
                    .unwrap()
                    .with_default(toml::Value::Boolean(true)),
            )
            .unwrap();

        let mut tree = toml::Table::new();
        tree.insert(
            "palette".to_string(),
            toml::Value::String("rosé-pine".into()),
        );
        tree.insert("fold".to_string(), toml::Value::Boolean(true)); // == default
        tree.insert("ghost".to_string(), toml::Value::Integer(7)); // unknown
        let mut t = toml::Table::new();
        t.insert("tree".to_string(), toml::Value::Table(tree));

        let diff = diff_from_default(&t, &schema);
        let tree = diff.get("tree").and_then(|v| v.as_table()).unwrap();
        assert_eq!(
            tree.get("palette").and_then(|v| v.as_str()),
            Some("rosé-pine")
        );
        assert!(tree.get("fold").is_none());
        assert_eq!(tree.get("ghost").and_then(|v| v.as_integer()), Some(7));
    }

    #[test]
    fn diff_collapses_empty_subtable() {
        let schema = schema_with(&[("tree.fold", toml::Value::Boolean(true))]);
        let mut tree = toml::Table::new();
        tree.insert("fold".to_string(), toml::Value::Boolean(true)); // == default
        let mut t = toml::Table::new();
        t.insert("tree".to_string(), toml::Value::Table(tree));
        let diff = diff_from_default(&t, &schema);
        assert!(diff.is_empty(), "empty sub-table collapsed away");
        assert!(diff.get("tree").is_none());
    }

    #[test]
    fn diff_compares_arrays_by_value() {
        let mut schema = Schema::new();
        schema
            .register(
                KeyMeta::new("tags", KeyType::Array, Scope::User, "tags")
                    .unwrap()
                    .with_default(toml::Value::Array(vec![
                        toml::Value::String("a".into()),
                        toml::Value::String("b".into()),
                    ])),
            )
            .unwrap();
        // Equal array → omitted.
        let mut t = toml::Table::new();
        t.insert(
            "tags".to_string(),
            toml::Value::Array(vec![
                toml::Value::String("a".into()),
                toml::Value::String("b".into()),
            ]),
        );
        assert!(diff_from_default(&t, &schema).is_empty());
        // Different array → kept.
        let mut t = toml::Table::new();
        t.insert(
            "tags".to_string(),
            toml::Value::Array(vec![toml::Value::String("c".into())]),
        );
        let diff = diff_from_default(&t, &schema);
        assert!(diff.contains_key("tags"));
    }
}
