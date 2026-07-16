//! `vibe prefs` plumbing — pure logic over the loader/resolver/schema cells
//! (PROP-040 §8 `#prefs-command`). Frontend-agnostic: the host builds a
//! [`PrefsOp`], hands it to [`run_prefs`], and receives a structured
//! [`PrefsOutcome`] to format. No disk I/O, no env reads — the host loads the
//! layer files and resolves the L1/L2/L3 paths; this cell only computes.
//!
//! The split is decision D3 in `SETTINGS-SYSTEM-IMPL-PLAN-v0.1` §6: logic here,
//! surface (clap wiring, output formatting, disk persist) in
//! `vibe-cli/src/commands/prefs/`. Each operation cites the PROP-040 anchor it
//! implements via `#[specmark::spec(implements = "…")]`.
//!
//! ## Phase 2.6 scope (REVIEW)
//!
//! `set`/`migrate` return the **mutated layer table**; the host does a basic
//! TOML write. Phase 2.7 (`persist`) enriches persistence: diff-from-default
//! (write only non-default values, §6 `#diff-from-default`), comment-preserving
//! rewrite via `toml_edit`, and `.gitignore` auto-gen (§9). The surface stays
//! stable — only the host's write step changes.
//!
//! Spec: [PROP-040 §8](../../../../spec/modules/vibe-settings/PROP-040-settings.md#prefs-command).

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-040#prefs-command");

use crate::loader::{Layer, LayeredRaw};
use crate::resolver::{self, InspectValue, Origin, ResolvedPrefs};
use crate::schema::Schema;

// ── PrefsOp ─────────────────────────────────────────────────────────────────

/// One `vibe prefs` operation, frontend-agnostic (PROP-040 §8 `#prefs-command`).
/// The host builds the variant matching its CLI subcommand and passes it to
/// [`run_prefs`]. Borrows the `key` where it can; the lifetime is the host's
/// CLI-args scope (the op is built and consumed in one call).
#[derive(Debug, Clone)]
pub enum PrefsOp<'a> {
    /// `vibe prefs get <key>` — resolve one key; the outcome is its per-layer
    /// breakdown ([`PrefsOutcome::Value`]).
    Get {
        /// The dotted path to read, e.g. `"tree.palette"`.
        key: &'a str,
    },
    /// `vibe prefs set <key> <value> --layer L` — scope-check (§7) and produce
    /// the mutated layer table for the host to persist; the outcome is
    /// [`PrefsOutcome::LayerWritten`].
    Set {
        /// The dotted path to write.
        key: &'a str,
        /// The already-typed value to write (the host coerces the CLI string).
        value: toml::Value,
        /// The file layer to write into.
        layer: Layer,
    },
    /// `vibe prefs list` — every resolved leaf with its origin
    /// ([`PrefsOutcome::Keys`]).
    List,
    /// `vibe prefs check` — validate every layer against the schema
    /// (§6 `#schema-first`); the outcome is [`PrefsOutcome::Diagnostics`].
    Check,
    /// `vibe prefs migrate` — rewrite deprecated keys to their `replaced_by`
    /// (§6 `#deprecation`); the outcome is [`PrefsOutcome::Migrated`].
    Migrate,
    /// `vibe prefs show-origins [key]` — the per-layer breakdown for one key,
    /// or for every resolved leaf when `key` is `None` (§8 `#show-origins`);
    /// the outcome is [`PrefsOutcome::Origins`].
    ShowOrigins {
        /// A single key, or `None` for every resolved leaf.
        key: Option<&'a str>,
    },
}

// ── PrefsOutcome ────────────────────────────────────────────────────────────

/// The structured result of a [`PrefsOp`], for the host to format (PROP-040 §8).
#[derive(Debug, Clone)]
pub enum PrefsOutcome {
    /// `get` — `Some` breakdown when the key resolves, `None` when it is absent
    /// from every layer (and has no default).
    Value(Option<InspectValue>),
    /// `set` — the layer to write and its mutated table (the host persists it).
    /// REVIEW(phase 2.7): the persist cell will collapse default-equivalent
    /// values and preserve comments; here the table is the raw merged layer.
    LayerWritten {
        /// The layer the host should write.
        layer: Layer,
        /// The layer's fully-merged table (existing keys + the new key).
        table: toml::Table,
    },
    /// `list` — every resolved leaf path, its value, and the layer that won it.
    Keys(Vec<KeyOrigin>),
    /// `check` — human-readable diagnostics (unknown keys, deprecated in use).
    /// The host may prepend any layer load/parse failures it collected.
    Diagnostics(Vec<String>),
    /// `migrate` — one entry per layer that rewrote at least one deprecated key;
    /// the host persists each `table`. Empty when nothing was deprecated.
    Migrated(Vec<Migration>),
    /// `show-origins` — per-layer breakdowns (one for the requested key, or one
    /// per resolved leaf), each paired with its dotted path.
    Origins(Vec<OriginEntry>),
}

/// One row of the [`PrefsOutcome::Origins`] report — a dotted path plus its
/// per-layer breakdown (PROP-040 §8 `#show-origins`). The path is carried here
/// because [`InspectValue`] reports values, not the path it was queried at.
#[derive(Debug, Clone)]
pub struct OriginEntry {
    /// The dotted path this breakdown is for.
    pub path: String,
    /// The per-layer breakdown for `path`.
    pub value: InspectValue,
}

/// A resolved leaf for [`PrefsOutcome::Keys`] (PROP-040 §8 `#show-origins`).
#[derive(Debug, Clone)]
pub struct KeyOrigin {
    /// The dotted path.
    pub path: String,
    /// The resolved value.
    pub value: toml::Value,
    /// Which layer established the resolved value (§2 `#precedence-law`).
    pub origin: Origin,
}

/// One layer's migration result for [`PrefsOutcome::Migrated`] (PROP-040 §6
/// `#deprecation`). The host writes `table` to the layer's file; `rewrote`
/// carries the human-readable `old -> new` lines.
#[derive(Debug, Clone)]
pub struct Migration {
    /// The layer that changed.
    pub layer: Layer,
    /// The rewritten table (deprecated keys moved to `replaced_by` / removed).
    pub table: toml::Table,
    /// One human-readable line per rewritten key (`old -> new`, or `old removed`).
    pub rewrote: Vec<String>,
}

// ── PrefsError ──────────────────────────────────────────────────────────────

/// Why a [`PrefsOp`] refused (PROP-040 §7 `#scope-matrix`). A **missing** key is
/// never an error — `get` returns `None`, `list` omits it; only a **wrong-layer
/// write** of a *declared* key is a typed error. Unknown keys on `set` are
/// allowed (they surface as warnings at `check`, per §6 `#schema-first`), so the
/// plumbing stays usable before a schema is populated.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#scope-matrix")]
pub enum PrefsError {
    /// The key is declared, but its `scope` forbids writing to `layer` (§7
    /// `#scope-matrix`). The host reports this and does not persist.
    #[error(
        "cannot set `{key}` in {layer}: its scope `{scope}` forbids that layer \
         (violates spec://vibevm/modules/vibe-settings/PROP-040#scope-matrix; \
          fix: write to one of the allowed layers — {allowed})"
    )]
    WrongLayer {
        /// The key the caller tried to set.
        key: String,
        /// The key's scope tag (§7).
        scope: String,
        /// The layer the caller requested.
        layer: Layer,
        /// The layers this key's scope permits (comma-joined in the message).
        allowed: String,
    },
}

// ── run_prefs ───────────────────────────────────────────────────────────────

/// Run a [`PrefsOp`] against loaded layers + schema (PROP-040 §8
/// `#prefs-command`). Pure: takes borrowed layers/schema/cli/env, resolves an
/// immutable snapshot internally, and returns a structured [`PrefsOutcome`] for
/// the host to format (and persist, for `set`/`migrate`). The only hard error is
/// [`PrefsError::WrongLayer`] (a declared key written to a forbidden layer).
///
/// `cli`/`env` are the top two precedence layers (§2); the host assembles them
/// from `--set` flags and `VIBE_*` vars. For the phase-2.6 `vibe prefs` surface
/// both are empty — the resolver still honours them when a future surface fills
/// them.
///
/// ```
/// use vibe_settings::cli::{PrefsOp, PrefsOutcome, run_prefs};
/// use vibe_settings::loader::LayeredRaw;
/// use vibe_settings::schema::Schema;
///
/// let raw = LayeredRaw::default();
/// let schema = Schema::new();
/// // `get` of an unset key with an empty schema → None (no error).
/// let out = run_prefs(
///     PrefsOp::Get { key: "tree.palette" },
///     &schema,
///     &raw,
///     &toml::Table::new(),
///     &toml::Table::new(),
/// )?;
/// assert!(matches!(out, PrefsOutcome::Value(None)));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#prefs-command")]
pub fn run_prefs<'a>(
    op: PrefsOp<'a>,
    schema: &Schema,
    raw: &LayeredRaw,
    cli: &toml::Table,
    env: &toml::Table,
) -> Result<PrefsOutcome, PrefsError> {
    let resolved = resolver::resolve(raw.clone(), schema, cli.clone(), env.clone());
    match op {
        PrefsOp::Get { key } => Ok(PrefsOutcome::Value(resolved.inspect(key))),
        PrefsOp::Set { key, value, layer } => Ok(set_key(schema, raw, key, value, layer)?),
        PrefsOp::List => Ok(PrefsOutcome::Keys(collect_leaves(resolved))),
        PrefsOp::Check => Ok(PrefsOutcome::Diagnostics(check_all(schema, raw))),
        PrefsOp::Migrate => Ok(PrefsOutcome::Migrated(migrate_all(schema, raw))),
        PrefsOp::ShowOrigins { key } => Ok(PrefsOutcome::Origins(origins(resolved, key))),
    }
}

// ── set ─────────────────────────────────────────────────────────────────────

/// Scope-check a declared key (§7 `#scope-matrix`) and produce the mutated layer
/// table. Unknown keys are allowed (they warn at `check`); a declared key in a
/// forbidden layer is [`PrefsError::WrongLayer`].
fn set_key(
    schema: &Schema,
    raw: &LayeredRaw,
    key: &str,
    value: toml::Value,
    layer: Layer,
) -> Result<PrefsOutcome, PrefsError> {
    if let Some(meta) = schema.get(key) {
        let allowed = meta.scope.writable_layers();
        if !allowed.contains(&layer) {
            return Err(PrefsError::WrongLayer {
                key: key.to_owned(),
                scope: meta.scope.label().to_owned(),
                layer,
                allowed: allowed
                    .iter()
                    .map(|l| l.label())
                    .collect::<Vec<_>>()
                    .join(", "),
            });
        }
    }
    let mut table = raw.layer(layer).clone();
    set_dotted(&mut table, key, value);
    Ok(PrefsOutcome::LayerWritten { layer, table })
}

// ── list / origins ──────────────────────────────────────────────────────────

/// Collect every leaf of the resolved tree as a [`KeyOrigin`] (PROP-040 §8 —
/// `list` shows what is actually set, with provenance).
fn collect_leaves(resolved: ResolvedPrefs) -> Vec<KeyOrigin> {
    let mut paths: Vec<(String, toml::Value)> = Vec::new();
    walk_leaves(resolved.merged(), String::new(), &mut |path, value| {
        paths.push((path, value.clone()))
    });
    paths.sort_by(|a, b| a.0.cmp(&b.0));
    paths
        .into_iter()
        .map(|(path, value)| KeyOrigin {
            origin: resolved.origin(&path).unwrap_or(Origin::Default),
            path,
            value,
        })
        .collect()
}

/// Build the [`PrefsOutcome::Origins`] list: one breakdown for a named key, or
/// one per resolved leaf (PROP-040 §8 `#show-origins`).
fn origins(resolved: ResolvedPrefs, key: Option<&str>) -> Vec<OriginEntry> {
    match key {
        Some(k) => resolved
            .inspect(k)
            .map(|value| OriginEntry {
                path: k.to_owned(),
                value,
            })
            .into_iter()
            .collect(),
        None => {
            let mut paths = Vec::new();
            walk_leaves(resolved.merged(), String::new(), &mut |path, _| {
                paths.push(path);
            });
            paths.sort();
            paths
                .into_iter()
                .filter_map(|p| {
                    resolved
                        .inspect(&p)
                        .map(|value| OriginEntry { path: p, value })
                })
                .collect()
        }
    }
}

// ── check ───────────────────────────────────────────────────────────────────

/// Validate every layer against the schema (PROP-040 §6 `#schema-first`,
/// `#deprecation`). Unknown keys and deprecated keys in use become diagnostics;
/// the host prepends any load/parse failures it collected. Never errors.
fn check_all(schema: &Schema, raw: &LayeredRaw) -> Vec<String> {
    let mut out = Vec::new();
    for (layer, table) in [
        (Layer::L1, &raw.l1),
        (Layer::L2, &raw.l2),
        (Layer::L3, &raw.l3),
    ] {
        for diag in crate::schema::validate(schema, table) {
            out.push(format!("{}: {}", layer.label(), diag.message));
        }
    }
    out
}

// ── migrate ─────────────────────────────────────────────────────────────────

/// Rewrite deprecated keys to their `replaced_by` (or drop retired keys) in each
/// layer (PROP-040 §6 `#deprecation`). Returns one [`Migration`] per layer that
/// changed; the host persists each. Empty when no key is deprecated.
///
/// ```
/// use vibe_settings::cli::{run_prefs, Migration, PrefsOp, PrefsOutcome};
/// use vibe_settings::loader::LayeredRaw;
/// use vibe_settings::schema::{Deprecation, KeyMeta, KeyType, Schema, Scope};
///
/// let mut schema = Schema::new();
/// schema.register(
///     KeyMeta::new("node.sort", KeyType::String, Scope::User, "sort order")?
///         .with_deprecation(Deprecation::with_replacement("use tree.sort", "tree.sort")),
/// )?;
/// let mut l2: toml::Table = toml::from_str(r#"node.sort = "name""#)?;
/// let raw = LayeredRaw { l1: toml::Table::new(), l2, l3: toml::Table::new() };
///
/// let out = run_prefs(PrefsOp::Migrate, &schema, &raw, &toml::Table::new(), &toml::Table::new())?;
/// let migrated = match out {
///     PrefsOutcome::Migrated(m) => m,
///     _ => panic!("expected Migrated"),
/// };
/// assert_eq!(migrated.len(), 1);
/// let Migration { layer, table, rewrote } = &migrated[0];
/// assert!(rewrote.iter().any(|r| r.contains("node.sort") && r.contains("tree.sort")));
/// // The new key is at the replacement path; the old is gone.
/// assert_eq!(table.get("tree").and_then(|t| t.as_table())
///     .and_then(|t| t.get("sort")).and_then(|v| v.as_str()), Some("name"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
fn migrate_all(schema: &Schema, raw: &LayeredRaw) -> Vec<Migration> {
    let mut out = Vec::new();
    for (layer, table) in [
        (Layer::L1, &raw.l1),
        (Layer::L2, &raw.l2),
        (Layer::L3, &raw.l3),
    ] {
        let mut new_table = table.clone();
        let mut rewrote = Vec::new();
        // Collect leaf paths from the *original* table first so the borrow ends
        // before we mutate `new_table`.
        let mut leaves = Vec::new();
        walk_leaves(table, String::new(), &mut |path, _| leaves.push(path));
        leaves.sort();
        for path in &leaves {
            let Some(meta) = schema.get(path) else {
                continue;
            };
            let Some(dep) = &meta.deprecated else {
                continue;
            };
            if let Some(value) = remove_dotted(&mut new_table, path) {
                match &dep.replaced_by {
                    Some(target) => {
                        set_dotted(&mut new_table, target, value);
                        rewrote.push(format!("{path} -> {target}"));
                    }
                    None => rewrote.push(format!("{path} removed (retired, no replacement)")),
                }
            }
        }
        if !rewrote.is_empty() {
            out.push(Migration {
                layer,
                table: new_table,
                rewrote,
            });
        }
    }
    out
}

// ── TOML path helpers (private; pure, no panic) ─────────────────────────────

/// Walk `table` recursively, calling `f` with each leaf's dotted path + value.
fn walk_leaves(table: &toml::Table, prefix: String, f: &mut impl FnMut(String, &toml::Value)) {
    for (key, value) in table {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        match value {
            toml::Value::Table(sub) => walk_leaves(sub, path, f),
            _ => f(path, value),
        }
    }
}

/// Insert `value` at a dotted path, creating intermediate tables as needed. A
/// kind conflict at an intermediate segment (non-table where descent is needed)
/// is a silent no-op — the caller's `check` surfaces the inconsistency.
fn set_dotted(table: &mut toml::Table, path: &str, value: toml::Value) {
    let mut segments: Vec<&str> = path.split('.').collect();
    let Some(last) = segments.pop() else {
        return;
    };
    let mut current = table;
    for seg in segments {
        let entry = current
            .entry(seg.to_owned())
            .or_insert_with(|| toml::Value::Table(toml::Table::new()));
        let toml::Value::Table(sub) = entry else {
            return;
        };
        current = sub;
    }
    current.insert(last.to_owned(), value);
}

/// Remove the value at a dotted path, returning it. `None` when any segment is
/// absent or an intermediate segment is not a table.
fn remove_dotted(table: &mut toml::Table, path: &str) -> Option<toml::Value> {
    let mut segments: Vec<&str> = path.split('.').collect();
    let last = segments.pop()?;
    let mut current = table;
    for seg in segments {
        current = current.get_mut(seg)?.as_table_mut()?;
    }
    current.remove(last)
}

#[cfg(test)]
mod tests;
