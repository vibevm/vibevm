//! Non-fatal validation over a TOML table against the schema (PROP-040 §6
//! `#schema-first`, §6 `#deprecation`).
//!
//! - [`unknown_keys`] — keys present in a file but not declared by the schema
//!   (typos, retired names), walked recursively through nested tables.
//! - [`validate`] — surfaces unknown keys and deprecated keys in use (citing
//!   each key's `replaced_by`), as a flat list of [`Diagnostic`]s. Non-fatal —
//!   the resolver reports each diagnostic, then treats the offending value as
//!   absent (§3 `#missing-is-default`); this fn never returns an error.
//!
//! Scope violations ([`DiagnosticKind::WrongScope`]) are **not** produced here
//! — they require layer context and are emitted by the resolver (phase 2.3).
//!
//! Spec: [PROP-040 §6](../../../../../../spec/modules/vibe-settings/PROP-040-settings.md#schema).

use super::registry::Schema;
use super::types::KeyType;

// ── Diagnostic ──────────────────────────────────────────────────────────────

/// The kind of non-fatal diagnostic surfaced by [`validate`] (PROP-040 §6
/// `#schema-first`, §6 `#deprecation`).
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#schema-first")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagnosticKind {
    /// A key present in a file but not declared by the schema (typo, retired
    /// name).
    UnknownKey,
    /// A deprecated key is in use — the diagnostic names its `replaced_by`
    /// target.
    Deprecated,
    /// A key was written to a layer its scope forbids (§7 `#scope-matrix`).
    /// REVIEW(phase 2.4): emitted by the resolver/set path, **not** by
    /// [`validate`] (which has no layer context). Kept on the enum so the
    /// resolver (phase 2.3) and CLI surface (phase 2.6) share one diagnostic
    /// vocabulary.
    WrongScope,
}

/// A non-fatal validation diagnostic (PROP-040 §6 `#schema-first`). Validation
/// never blocks boot — the resolver reports each diagnostic, then treats the
/// offending value as absent (§3 `#missing-is-default`).
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#schema-first")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// The dotted path the diagnostic is about.
    pub path: String,
    /// What kind of issue was found.
    pub kind: DiagnosticKind,
    /// A human-readable message citing the migration target or the misspelling.
    pub message: String,
}

// ── Validation ──────────────────────────────────────────────────────────────

/// The list of keys present in `table` but not declared by `schema`, as dotted
/// paths (PROP-040 §6 `#schema-first` — typos and retired names surface loud,
/// never a silent ignore). Walks nested TOML tables recursively; a key declared
/// with [`KeyType::Table`] is treated as a free-form namespace (its children
/// are not flagged).
///
/// ```
/// use vibe_settings::schema::{unknown_keys, KeyMeta, KeyType, Schema, Scope};
/// use toml::Table;
///
/// let mut schema = Schema::new();
/// schema.register(KeyMeta::new("tree.palette", KeyType::String, Scope::User, "palette")?)?;
///
/// let mut table: Table = toml::from_str(
///     "tree.palette = \"rosé-pine\"\n\
///      tree.palate = \"oops\"\n\
///      ghost.key = 1\n",
/// )?;
///
/// let mut unknown = unknown_keys(&schema, &table);
/// unknown.sort();
/// assert_eq!(unknown, vec!["ghost.key", "tree.palate"]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#schema-first")]
pub fn unknown_keys(schema: &Schema, table: &toml::Table) -> Vec<String> {
    let mut out = Vec::new();
    collect_unknown(schema, table, String::new(), &mut out);
    out
}

/// Validate `table` against `schema` (PROP-040 §6 `#schema-first`,
/// §6 `#deprecation`): surfaces unknown keys (typos, retired names) and
/// deprecated keys in use (citing `replaced_by`). Non-fatal — the caller
/// reports each [`Diagnostic`] and treats the offending value as absent; this
/// fn never returns an error.
///
/// Scope violations ([`DiagnosticKind::WrongScope`]) are **not** produced here
/// — they require layer context and are emitted by the resolver (phase 2.3).
///
/// ```
/// use vibe_settings::schema::{validate, Deprecation, DiagnosticKind, KeyMeta, KeyType, Schema, Scope};
///
/// let mut schema = Schema::new();
/// schema.register(KeyMeta::new("tree.palette", KeyType::String, Scope::User, "palette")?)?;
/// schema.register(
///     KeyMeta::new("node.sort", KeyType::String, Scope::User, "sort order")?
///         .with_deprecation(Deprecation::with_replacement("use `tree.sort`", "tree.sort")),
/// )?;
///
/// let table: toml::Table = toml::from_str(
///     "tree.palette = \"x\"\n\
///      node.sort = \"name\"\n\
///      typo = true\n",
/// )?;
///
/// let diags = validate(&schema, &table);
/// let kinds: Vec<_> = diags.iter().map(|d| d.kind).collect();
/// assert!(kinds.contains(&DiagnosticKind::UnknownKey));
/// assert!(kinds.contains(&DiagnosticKind::Deprecated));
/// // The deprecation message points at the replacement.
/// let dep = diags.iter().find(|d| d.kind == DiagnosticKind::Deprecated).unwrap();
/// assert!(dep.message.contains("tree.sort"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#schema-first")]
pub fn validate(schema: &Schema, table: &toml::Table) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for path in unknown_keys(schema, table) {
        out.push(Diagnostic {
            message: format!(
                "unknown setting `{path}` — not declared by the schema \
                 (likely a typo or a retired name)"
            ),
            path: path.clone(),
            kind: DiagnosticKind::UnknownKey,
        });
    }
    collect_deprecated(schema, table, String::new(), &mut out);
    out
}

// ─── validation walkers (private; recursive over nested TOML tables) ─────────

/// Walk `table`, appending every leaf path not declared by `schema` (and not
/// nested inside a registered `KeyType::Table` namespace) to `out`.
fn collect_unknown(schema: &Schema, table: &toml::Table, prefix: String, out: &mut Vec<String>) {
    for (key, value) in table {
        let path = dotted(&prefix, key);
        match value {
            toml::Value::Table(sub) => {
                // A registered Table key is a free-form namespace: anything
                // inside is allowed, so we stop recursing there.
                if let Some(meta) = schema.get(&path)
                    && meta.key_type == KeyType::Table
                {
                    continue;
                }
                collect_unknown(schema, sub, path, out);
            }
            _ => {
                if !schema.contains(&path) {
                    out.push(path);
                }
            }
        }
    }
}

/// Walk `table`, appending a [`DiagnosticKind::Deprecated`] for each present
/// key whose [`KeyMeta`](super::types::KeyMeta) carries a `Deprecation`.
fn collect_deprecated(
    schema: &Schema,
    table: &toml::Table,
    prefix: String,
    out: &mut Vec<Diagnostic>,
) {
    for (key, value) in table {
        let path = dotted(&prefix, key);
        if let Some(meta) = schema.get(&path)
            && let Some(dep) = &meta.deprecated
        {
            let target = dep.replaced_by.as_deref().unwrap_or("(no replacement)");
            out.push(Diagnostic {
                message: format!(
                    "setting `{path}` is deprecated: {} \
                     (replaced by `{target}` — run `vibe prefs migrate`)",
                    dep.message
                ),
                path: path.clone(),
                kind: DiagnosticKind::Deprecated,
            });
        }
        if let toml::Value::Table(sub) = value {
            if let Some(meta) = schema.get(&path)
                && meta.key_type == KeyType::Table
            {
                continue;
            }
            collect_deprecated(schema, sub, path, out);
        }
    }
}

/// Build a dotted path from a parent prefix and a child key.
fn dotted(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        key.to_owned()
    } else {
        format!("{prefix}.{key}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Deprecation, KeyMeta, MergeStrategy, Scope};

    fn key(path: &str, ty: KeyType, scope: Scope) -> KeyMeta {
        KeyMeta::new(path, ty, scope, "a test setting").unwrap()
    }

    // ── unknown_keys ───────────────────────────────────────────────────────

    #[test]
    fn unknown_keys_flags_typos_and_retired_names_recursively() {
        let mut schema = Schema::new();
        schema
            .register(key("tree.palette", KeyType::String, Scope::User))
            .unwrap();
        let table: toml::Table = toml::from_str(
            "tree.palette = \"x\"\n\
             tree.palate = \"oops\"\n\
             [node]\n\
             fold = true\n\
             hidden = 1\n",
        )
        .unwrap();
        let mut found = unknown_keys(&schema, &table);
        found.sort();
        assert_eq!(found, vec!["node.fold", "node.hidden", "tree.palate"]);
    }

    #[test]
    fn unknown_keys_skips_inside_a_registered_table_key() {
        // A KeyType::Table key is a free-form namespace — its children are
        // never flagged, even when they look undeclared.
        let mut schema = Schema::new();
        schema
            .register(key("tree", KeyType::Table, Scope::User))
            .unwrap();
        let table: toml::Table =
            toml::from_str("[tree]\nanything = 1\nnested = { x = 2 }\n").unwrap();
        assert!(unknown_keys(&schema, &table).is_empty());
    }

    #[test]
    fn unknown_keys_empty_for_empty_schema_and_empty_table() {
        let schema = Schema::new();
        assert!(unknown_keys(&schema, &toml::Table::new()).is_empty());
        let table: toml::Table = toml::from_str("a = 1\nb = 2\n").unwrap();
        let mut found = unknown_keys(&schema, &table);
        found.sort();
        assert_eq!(found, vec!["a", "b"]);
    }

    // ── validate ───────────────────────────────────────────────────────────

    #[test]
    fn validate_surfaces_unknown_and_deprecated_in_one_pass() {
        let mut schema = Schema::new();
        schema
            .register(key("tree.palette", KeyType::String, Scope::User))
            .unwrap();
        schema
            .register(
                key("node.sort", KeyType::String, Scope::User)
                    .with_deprecation(Deprecation::with_replacement("use tree.sort", "tree.sort")),
            )
            .unwrap();
        let table: toml::Table = toml::from_str(
            "tree.palette = \"x\"\n\
             node.sort = \"name\"\n\
             typo = true\n",
        )
        .unwrap();
        let diags = validate(&schema, &table);
        let mut kinds: Vec<_> = diags.iter().map(|d| d.kind).collect();
        kinds.sort();
        // Variant-declaration order: UnknownKey < Deprecated < WrongScope.
        assert_eq!(
            kinds,
            vec![DiagnosticKind::UnknownKey, DiagnosticKind::Deprecated]
        );
        // Deprecation message names the migration target.
        let dep = diags
            .iter()
            .find(|d| d.kind == DiagnosticKind::Deprecated)
            .unwrap();
        assert!(dep.message.contains("tree.sort"));
        assert_eq!(dep.path, "node.sort");
    }

    #[test]
    fn validate_never_emits_wrong_scope_no_layer_context() {
        // WrongScope requires layer context and is emitted by the resolver
        // (phase 2.3), not by validate(). REVIEW(phase 2.4) — see DiagnosticKind.
        let mut schema = Schema::new();
        schema
            .register(key("tree.palette", KeyType::String, Scope::User))
            .unwrap();
        let table: toml::Table = toml::from_str("tree.palette = \"x\"\n").unwrap();
        let diags = validate(&schema, &table);
        assert!(!diags.iter().any(|d| d.kind == DiagnosticKind::WrongScope));
    }

    #[test]
    fn validate_clean_table_yields_no_diagnostics() {
        let mut schema = Schema::new();
        schema
            .register(key("tree.palette", KeyType::String, Scope::User))
            .unwrap();
        let table: toml::Table = toml::from_str("tree.palette = \"x\"\n").unwrap();
        assert!(validate(&schema, &table).is_empty());
    }

    #[test]
    fn validate_surfaces_deprecated_with_no_replacement_target() {
        // A Deprecation::new (no replaced_by) still surfaces, and points at
        // "(no replacement)".
        let mut schema = Schema::new();
        schema
            .register(
                key("legacy.flag", KeyType::Bool, Scope::User)
                    .with_deprecation(Deprecation::new("retired, no successor")),
            )
            .unwrap();
        let table: toml::Table = toml::from_str("legacy.flag = true\n").unwrap();
        let diags = validate(&schema, &table);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].kind, DiagnosticKind::Deprecated);
        assert!(diags[0].message.contains("(no replacement)"));
    }

    #[test]
    fn validate_walks_deprecated_inside_nested_tables() {
        let mut schema = Schema::new();
        schema
            .register(
                key("node.sort", KeyType::String, Scope::User)
                    .with_deprecation(Deprecation::with_replacement("use tree.sort", "tree.sort")),
            )
            .unwrap();
        // node.sort nested under a `[node]` table — still surfaced.
        let table: toml::Table = toml::from_str("[node]\nsort = \"name\"\n").unwrap();
        let diags = validate(&schema, &table);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].kind, DiagnosticKind::Deprecated);
        assert_eq!(diags[0].path, "node.sort");
    }

    // ── sanity: MergeStrategy carried through (smoke) ──────────────────────

    #[test]
    fn key_meta_round_trips_a_full_optional_set() {
        // A quick end-to-end of the builder — covers the with_* seams the
        // resolver (phase 2.3) will read.
        let k = KeyMeta::new("tree.glyphs", KeyType::Array, Scope::Project, "the glyphs")
            .unwrap()
            .with_default(toml::Value::Array(vec![
                toml::Value::String("●".into()),
                toml::Value::String("○".into()),
            ]))
            .with_merge(MergeStrategy::MergeByKey);
        assert_eq!(k.scope, Scope::Project);
        assert_eq!(k.merge, MergeStrategy::MergeByKey);
        assert!(matches!(k.default, Some(toml::Value::Array(_))));
    }
}
