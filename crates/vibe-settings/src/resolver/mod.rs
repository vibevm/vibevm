//! The resolver — `ResolvedPrefs` + deep-merge + `inspect` (PROP-040 §4
//! `#merge`, §5 `#resolver`). Pure functions over TOML: [`resolve`] composes
//! `default + L1 + L2 + L3 + cli + env` into one immutable snapshot; a change
//! is a fresh [`resolve`] (prediction P1 — no mutation, no interior mutability).
//!
//! ## The precedence law, encoded (PROP-040 §2 #precedence-law)
//!
//! ```text
//! built-in default  ⊂  L1 user-machine  ⊂  L2 repo-shared  ⊂  L3 user-project
//!                   ⊂  CLI flag          ⊂  env var
//! ```
//!
//! Each higher layer deep-merges over the lower ([`merge::merge_layer`]);
//! provenance records which layer established each leaf value so
//! [`ResolvedPrefs::inspect`] can name the winner (prediction P3 — origin
//! round-trips). The algorithm itself lives in [`merge`] (split out to honour
//! the ≤600-line AI-Native file budget).
//!
//! ## TOML has no null (PROP-040 §4 #null-semantics — TOML-specific note)
//!
//! TOML carries **no `null` literal**, so the spec's two-way distinction —
//! `null` = "explicitly unset, shadow the parent with empty" vs absence =
//! "fall back to the parent" — is expressed not by a value here but by a
//! *key* in a layer file: an absent key falls back (the layer is treated as
//! not setting it), and an "explicit unset" is the persist-cell's job (phase
//! 2.7 writes/removes the key per `#diff-from-default`). REVIEW(phase 2.7):
//! the persist cell owns the "explicit unset" encoding; the resolver never
//! sees a sentinel and treats every present value as "set". This keeps §4
//! `#null-semantics` honest against TOML's type system.
//!
//! ## AI-Native Rust discipline (PROP-040 §13)
//!
//! Cells with single registration points; this file's scope is `#resolver`;
//! `#[specmark::spec(implements = "spec://…")]` on the public seams; no
//! `unwrap`/`expect` in domain logic; non-fatal merge conflicts surface as
//! [`ResolvedPrefs::diagnostics`] (R2 — mixed scalar/table/array at one path
//! is rare and skipped, never a panic); ≤600-line file budget.
//!
//! Spec: [PROP-040 §4, §5](../../../../../spec/modules/vibe-settings/PROP-040-settings.md#resolver).

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-040#resolver");

use std::collections::BTreeMap;
use std::fmt;

use crate::loader::LayeredRaw;
use crate::schema::Schema;

mod merge;

use merge::merge_layer;

// ── Origin ──────────────────────────────────────────────────────────────────

/// Which layer established a resolved value (PROP-040 §2 `#precedence-law`).
///
/// The variant order is the precedence order: `Default < L1 < L2 < L3 < Cli <
/// Env`. [`ResolvedPrefs::inspect`] returns the variant for the layer that won
/// each leaf; a higher-layer win replaces a lower one per the merge semantics
/// (§4 `#merge-algorithm`).
///
/// ```
/// use vibe_settings::resolver::Origin;
///
/// // The wire/diagnostic tags match PROP-040 §2's spelling.
/// assert_eq!(Origin::L2.label(), "L2");
/// assert_eq!(Origin::Default.to_string(), "default");
/// assert_eq!(Origin::Env.label(), "env");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Origin {
    /// A built-in default from the schema's `KeyMeta.default` (the lowest
    /// layer). Distinct from "absent" — `Default` means the schema supplies a
    /// value where no file layer does.
    Default,
    /// User-machine (`~/.vibe/settings.toml`).
    L1,
    /// Repo-shared (`<repo>/.vibe/settings.toml`, committed).
    L2,
    /// User-project (`<repo>/.vibe/settings.local.toml`, gitignored).
    L3,
    /// A `--set`/`--config` CLI flag.
    Cli,
    /// A `VIBE_*` environment variable.
    Env,
}

impl Origin {
    /// The short tag used in diagnostics and `--show-origins` (PROP-040 §2
    /// spellings).
    pub const fn label(self) -> &'static str {
        match self {
            Origin::Default => "default",
            Origin::L1 => "L1",
            Origin::L2 => "L2",
            Origin::L3 => "L3",
            Origin::Cli => "cli",
            Origin::Env => "env",
        }
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

// ── InspectValue ────────────────────────────────────────────────────────────

/// The per-layer breakdown for one key (PROP-040 §5 `#inspect` — the key AIUI
/// API, the VSCode `IConfigurationValue<T>` shape clean-room). One call yields
/// the effective value *and* each layer's contribution *and* which layer won.
///
/// `default`/`l1`/`l2`/`l3`/`cli`/`env` are `None` when that layer does not set
/// the path. `value` is the resolved value (from the merged tree); `origin` is
/// the winner ([`Origin::Default`] when the path is a non-leaf container with
/// no single provenance — inspect is most informative for leaf paths).
///
/// ```
/// use vibe_settings::loader::LayeredRaw;
/// use vibe_settings::resolver::{resolve, Origin};
/// use vibe_settings::schema::{KeyMeta, KeyType, Schema, Scope};
///
/// let mut schema = Schema::new();
/// schema.register(
///     KeyMeta::new("tree.palette", KeyType::String, Scope::User, "palette")?
///           .with_default(toml::Value::String("def".into())),
/// )?;
/// let l2: toml::Table = toml::from_str(r#"tree.palette = "rosé-pine""#)?;
/// let raw = LayeredRaw { l1: toml::Table::new(), l2, l3: toml::Table::new() };
/// let rp = resolve(raw, &schema, toml::Table::new(), toml::Table::new());
///
/// let iv = rp.inspect("tree.palette").unwrap();
/// // `value` is the resolved value; the layer fields are `Option<Value>`.
/// assert_eq!(iv.value.as_str(), Some("rosé-pine"));   // L2 wins.
/// assert_eq!(
///     iv.default.as_ref().and_then(toml::Value::as_str),
///     Some("def"),
/// );
/// assert!(iv.l1.is_none());                            // L1 unset.
/// assert_eq!(
///     iv.l2.as_ref().and_then(toml::Value::as_str),
///     Some("rosé-pine"),
/// );
/// assert_eq!(iv.origin, Origin::L2);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct InspectValue {
    /// The resolved value (from the merged tree).
    pub value: toml::Value,
    /// Built-in default from the schema, if any.
    pub default: Option<toml::Value>,
    /// L1 user-machine value, if L1 sets this path.
    pub l1: Option<toml::Value>,
    /// L2 repo-shared value, if L2 sets this path.
    pub l2: Option<toml::Value>,
    /// L3 user-project value, if L3 sets this path.
    pub l3: Option<toml::Value>,
    /// CLI override, if any.
    pub cli: Option<toml::Value>,
    /// Env override, if any.
    pub env: Option<toml::Value>,
    /// Which layer established the resolved `value`. [`Origin::Default`] when
    /// the path is a non-leaf container with no recorded leaf provenance.
    pub origin: Origin,
}

// ── ResolvedPrefs ───────────────────────────────────────────────────────────

/// The composed, immutable view over `default + L1 + L2 + L3 + CLI + env`
/// (PROP-040 §5 `#resolved-prefs`). Built once by [`resolve`]; never mutated.
/// Consumers (the TUI, a future vibe app, the AIUI) read resolved values
/// through [`get`](Self::get)/[`get_section`](Self::get_section)/
/// [`inspect`](Self::inspect) — no consumer reads a raw layer.
///
/// Provenance (which layer set each leaf) is carried alongside so
/// [`inspect`](Self::inspect) and [`origin`](Self::origin) can name the winner
/// for the AIUI and `vibe prefs --show-origins` (§8). Non-fatal merge notes
/// (mixed scalar/table/array at one path — risk R2) accumulate in
/// [`diagnostics`](Self::diagnostics).
#[derive(Debug, Clone)]
pub struct ResolvedPrefs {
    /// Deep-merged resolved values (default ⊂ L1 ⊂ L2 ⊂ L3 ⊂ cli ⊂ env).
    merged: toml::Table,
    /// Leaf dotted-path → the layer that won it.
    provenance: BTreeMap<String, Origin>,
    /// Owned raw layers (for per-layer [`Self::inspect`] lookups).
    layers: LayeredRaw,
    /// Built-in defaults materialised from the schema's `KeyMeta.default`.
    defaults: toml::Table,
    /// The CLI layer.
    cli: toml::Table,
    /// The env layer.
    env: toml::Table,
    /// Non-fatal merge notes (risk R2: mixed kinds at one path).
    diagnostics: Vec<String>,
}

impl ResolvedPrefs {
    /// The resolved value at a dotted path, e.g. `"tree.palette"`
    /// (PROP-040 §5 `#resolved-prefs`). Returns the [`toml::Value`] for leaf
    /// paths; for a table path returns the composed subtree as
    /// [`toml::Value::Table`]. `None` when the path is absent.
    ///
    /// ```
    /// use vibe_settings::loader::LayeredRaw;
    /// use vibe_settings::resolver::resolve;
    ///
    /// let l1: toml::Table = toml::from_str("tree.palette = \"x\"\n")?;
    /// let raw = LayeredRaw { l1, l2: toml::Table::new(), l3: toml::Table::new() };
    /// let rp = resolve(raw, &Default::default(), toml::Table::new(), toml::Table::new());
    /// assert_eq!(rp.get("tree.palette").and_then(|v| v.as_str()), Some("x"));
    /// assert!(rp.get("tree.missing").is_none());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#resolved-prefs")]
    pub fn get(&self, path: &str) -> Option<&toml::Value> {
        lookup_dotted(&self.merged, path)
    }

    /// A whole namespace as a typed subtree (PROP-040 §5 `#get-section`) —
    /// `get_section("tree")` returns the composed `tree.*` table. `None` when
    /// the namespace is absent or names a non-table value.
    ///
    /// ```
    /// use vibe_settings::loader::LayeredRaw;
    /// use vibe_settings::resolver::resolve;
    ///
    /// let l1: toml::Table =
    ///     toml::from_str("[tree]\npalette = \"x\"\nmode = \"compact\"\n")?;
    /// let raw = LayeredRaw { l1, l2: toml::Table::new(), l3: toml::Table::new() };
    /// let rp = resolve(raw, &Default::default(), toml::Table::new(), toml::Table::new());
    /// let tree = rp.get_section("tree").unwrap();
    /// assert_eq!(tree.len(), 2);
    /// assert!(tree.contains_key("palette"));
    /// // A leaf path is not a section.
    /// assert!(rp.get_section("tree.palette").is_none());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#get-section")]
    pub fn get_section(&self, namespace: &str) -> Option<&toml::Table> {
        lookup_dotted(&self.merged, namespace).and_then(toml::Value::as_table)
    }

    /// The per-layer breakdown for one key (PROP-040 §5 `#inspect`) — see
    /// [`InspectValue`] for the full shape. `None` when the path is absent
    /// from the merged tree.
    ///
    /// `inspect` is most informative for **leaf** paths. For a table path the
    /// `value`/`default`/`l1`/… fields carry each layer's subtree and `origin`
    /// is [`Origin::Default`] (a container has no single provenance; ask
    /// [`origin`](Self::origin) per leaf instead).
    #[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#inspect")]
    pub fn inspect(&self, path: &str) -> Option<InspectValue> {
        let value = lookup_dotted(&self.merged, path)?.clone();
        let origin = self
            .provenance
            .get(path)
            .copied()
            .unwrap_or(Origin::Default);
        Some(InspectValue {
            value,
            default: lookup_dotted(&self.defaults, path).cloned(),
            l1: lookup_dotted(&self.layers.l1, path).cloned(),
            l2: lookup_dotted(&self.layers.l2, path).cloned(),
            l3: lookup_dotted(&self.layers.l3, path).cloned(),
            cli: lookup_dotted(&self.cli, path).cloned(),
            env: lookup_dotted(&self.env, path).cloned(),
            origin,
        })
    }

    /// Which layer established the resolved value at a leaf path, or `None`
    /// when the path is absent or is a non-leaf container with no recorded
    /// provenance (PROP-040 §5 `#inspect`). Prediction P3: for any leaf,
    /// this is the layer that won it.
    ///
    /// ```
    /// use vibe_settings::loader::LayeredRaw;
    /// use vibe_settings::resolver::{resolve, Origin};
    ///
    /// let l2: toml::Table = toml::from_str("a = 1\n")?;
    /// let raw = LayeredRaw { l1: toml::Table::new(), l2, l3: toml::Table::new() };
    /// let rp = resolve(raw, &Default::default(), toml::Table::new(), toml::Table::new());
    /// assert_eq!(rp.origin("a"), Some(Origin::L2));
    /// assert!(rp.origin("missing").is_none());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#inspect")]
    pub fn origin(&self, path: &str) -> Option<Origin> {
        self.provenance.get(path).copied()
    }

    /// The composed resolved tree (read-only). Useful for serialising the
    /// snapshot or walking the surface for the AIUI; mutation is a fresh
    /// [`resolve`] (prediction P1).
    pub fn merged(&self) -> &toml::Table {
        &self.merged
    }

    /// Non-fatal merge notes accumulated during [`resolve`] (risk R2: mixed
    /// scalar/table/array at one path — the conflicting value is skipped,
    /// never a panic). Empty when no layer disagreed on a path's kind.
    pub fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }
}

// ── resolve ─────────────────────────────────────────────────────────────────

/// Compose `default + L1 + L2 + L3 + CLI + env` into one immutable
/// [`ResolvedPrefs`] (PROP-040 §2 `#precedence-law`, §5 `#resolved-prefs`).
///
/// Layers are deep-merged lowest-to-highest (§4 `#merge-algorithm`):
/// scalars last-win, tables recurse, arrays merge per the schema's declared
/// [`MergeStrategy`](crate::schema::MergeStrategy) (default `Replace`).
/// Built-in defaults come from the schema's `KeyMeta.default`; the `cli`/`env`
/// tables are caller-assembled (the CLI/env plumbing lands in phase 2.6 —
/// here they are just the top two layers).
///
/// Pure: takes ownership of the raw layers + cli + env, returns a snapshot,
/// touches no ambient state. A change is a fresh `resolve` (prediction P1).
///
/// ```
/// use vibe_settings::loader::LayeredRaw;
/// use vibe_settings::resolver::{resolve, Origin};
/// use vibe_settings::schema::{KeyMeta, KeyType, Schema, Scope};
///
/// let mut schema = Schema::new();
/// schema.register(
///     KeyMeta::new("tree.palette", KeyType::String, Scope::User, "palette")?
///           .with_default(toml::Value::String("def".into())),
/// )?;
/// // L2 overrides the built-in default; CLI overrides L2.
/// let l2: toml::Table = toml::from_str(r#"tree.palette = "rosé-pine""#)?;
/// let cli: toml::Table = toml::from_str(r#"tree.palette = "solarized""#)?;
/// let raw = LayeredRaw { l1: toml::Table::new(), l2, l3: toml::Table::new() };
///
/// let rp = resolve(raw, &schema, cli, toml::Table::new());
/// assert_eq!(
///     rp.get("tree.palette").and_then(|v| v.as_str()),
///     Some("solarized"),
/// );
/// assert_eq!(rp.origin("tree.palette"), Some(Origin::Cli));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#resolved-prefs")]
pub fn resolve(
    raw: LayeredRaw,
    schema: &Schema,
    cli: toml::Table,
    env: toml::Table,
) -> ResolvedPrefs {
    let defaults = defaults_from_schema(schema);
    let mut merged = toml::Table::new();
    let mut provenance: BTreeMap<String, Origin> = BTreeMap::new();
    let mut diagnostics: Vec<String> = Vec::new();

    // §2 #precedence-law — lowest to highest. Each call deep-merges one layer
    // over `merged`, recording the winner on every leaf it touches.
    for (over, origin) in [
        (&defaults, Origin::Default),
        (&raw.l1, Origin::L1),
        (&raw.l2, Origin::L2),
        (&raw.l3, Origin::L3),
        (&cli, Origin::Cli),
        (&env, Origin::Env),
    ] {
        merge_layer(
            &mut merged,
            &mut provenance,
            over,
            origin,
            schema,
            "",
            &mut diagnostics,
        );
    }

    ResolvedPrefs {
        merged,
        provenance,
        layers: raw,
        defaults,
        cli,
        env,
        diagnostics,
    }
}

// ── read-path helpers (private; pure, no env, no panic) ─────────────────────

/// Materialise the built-in defaults layer from the schema's `KeyMeta.default`
/// (PROP-040 §2 `#precedence-law` — defaults are the lowest layer). Keys with
/// no default contribute nothing.
fn defaults_from_schema(schema: &Schema) -> toml::Table {
    let mut out = toml::Table::new();
    for path in schema.keys() {
        // `schema.keys()` yields borrowed paths; `get` resolves each back to
        // its `KeyMeta`. Filter to keys that actually declare a default.
        if let Some(meta) = schema.get(path)
            && let Some(default) = &meta.default
        {
            set_dotted(&mut out, path, default.clone());
        }
    }
    out
}

/// Look up a dotted path in a TOML table, walking nested tables. Returns the
/// value at the path (leaf or intermediate table). `None` when any segment is
/// absent or an intermediate segment is not a table.
fn lookup_dotted<'a>(table: &'a toml::Table, path: &str) -> Option<&'a toml::Value> {
    if path.is_empty() {
        return None;
    }
    let mut segments = path.split('.');
    let last = segments.next_back()?;
    let mut current = table;
    for seg in segments {
        current = current.get(seg)?.as_table()?;
    }
    current.get(last)
}

/// Insert `value` at a dotted path in a TOML table, creating intermediate
/// tables as needed. Silently no-ops on a kind conflict at an intermediate
/// segment (defaults should not collide; if they do, the later one loses).
fn set_dotted(table: &mut toml::Table, path: &str, value: toml::Value) {
    if path.is_empty() {
        return;
    }
    let mut segments = path.split('.').peekable();
    let mut current = table;
    while let Some(seg) = segments.next() {
        if segments.peek().is_none() {
            // Leaf — a defaults entry is declared once per path; insert.
            current.insert(seg.to_owned(), value);
            return;
        }
        // Intermediate — descend, creating an empty table when missing.
        let entry = current
            .entry(seg.to_owned())
            .or_insert_with(|| toml::Value::Table(toml::Table::new()));
        match entry {
            toml::Value::Table(t) => current = t,
            // Prior default at this segment is a non-table; cannot descend.
            _ => return,
        }
    }
}

#[cfg(test)]
mod tests;
