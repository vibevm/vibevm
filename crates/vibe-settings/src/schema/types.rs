//! The typed enums + `KeyMeta` for one declared setting (PROP-040 §6
//! `#schema-fields`, §7 `#scope-meta`, §10 `#applies`, §4
//! `#merge-strategy-opt-in`).
//!
//! Carries everything the resolver, the validator, and a settings UI need to
//! know about a preference without reading a file: dotted path, type, built-in
//! default, human description (mandatory, non-empty), scope, `applies`, array
//! `merge` strategy, deprecation, and `restricted` flag.
//!
//! Spec: [PROP-040 §6, §7](../../../../../../spec/modules/vibe-settings/PROP-040-settings.md#schema).

use std::fmt;

use thiserror::Error;

use crate::loader::Layer;

// ── KeyType ─────────────────────────────────────────────────────────────────

/// The tag half of a typed setting — the closed set of value shapes a
/// preference may take (PROP-040 §6 `#schema-fields`). A new shape is a spec
/// change; the closed enum is the type-system enforcement of "every key is
/// declared".
///
/// ```
/// use vibe_settings::schema::KeyType;
///
/// assert_eq!(KeyType::Bool.to_string(), "bool");
/// assert_eq!(KeyType::Array.label(), "array");
/// ```
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#schema-fields")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyType {
    /// A boolean flag (`true` / `false`).
    Bool,
    /// A 64-bit signed integer.
    Int,
    /// A UTF-8 string.
    String,
    /// A closed enumeration. The allowed-values list is not carried here yet —
    /// REVIEW(phase 2.4): a future `enum_values: Option<Vec<String>>` field on
    /// [`KeyMeta`] narrows this once the TUI surfaces it (§6 `#schema-fields`
    /// names the `enum` shape but not its constraint vocabulary).
    Enum,
    /// An array of values; merge behaviour governed by [`MergeStrategy`].
    Array,
    /// A nested TOML table — its children are free-form sub-keys, so the
    /// validator does not flag unknown names *inside* a registered Table key.
    Table,
}

impl KeyType {
    /// The short wire/diagnostic tag, e.g. `"bool"` (PROP-040 §6 lists these
    /// exact spellings).
    pub const fn label(self) -> &'static str {
        match self {
            KeyType::Bool => "bool",
            KeyType::Int => "int",
            KeyType::String => "string",
            KeyType::Enum => "enum",
            KeyType::Array => "array",
            KeyType::Table => "table",
        }
    }
}

impl fmt::Display for KeyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

// ── Scope ───────────────────────────────────────────────────────────────────

/// Where a preference may be set and whether it roams (PROP-040 §7
/// `#scope-metadata`). Each variant maps to a fixed set of writable file layers
/// (§7 `#scope-matrix`); the resolver refuses a write to any other layer with a
/// typed error.
///
/// ```
/// use vibe_settings::loader::Layer;
/// use vibe_settings::schema::Scope;
///
/// // User roams across all three layers; Machine stays at L1 only.
/// assert_eq!(
///     Scope::User.writable_layers(),
///     &[Layer::L1, Layer::L2, Layer::L3],
/// );
/// assert_eq!(Scope::Machine.writable_layers(), &[Layer::L1]);
/// // Team-only is L2 only — a user may not override it in L3.
/// assert_eq!(Scope::TeamOnly.writable_layers(), &[Layer::L2]);
/// assert_eq!(Scope::Project.to_string(), "project");
/// ```
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#scope-meta")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scope {
    /// Settable in L1 (and overridden by L2/L3). Roams (future cloud, §14).
    User,
    /// Machine-specific (paths, OS); settable in L1 only, **does not roam**
    /// (the VSCode `machine`/`machine-overridable` answer to §4.1.7).
    Machine,
    /// Project-specific; settable in L2/L3, not L1.
    Project,
    /// A team preference a user may not override in L3 — L2 only (e.g. a
    /// project's canonical palette).
    TeamOnly,
}

impl Scope {
    /// The short wire/diagnostic tag, matching PROP-040 §7's spellings.
    pub const fn label(self) -> &'static str {
        match self {
            Scope::User => "user",
            Scope::Machine => "machine",
            Scope::Project => "project",
            Scope::TeamOnly => "team-only",
        }
    }

    /// The file layers a key with this scope may be **written** to
    /// (PROP-040 §7 `#scope-matrix`). Returned as a static slice — the matrix
    /// is fixed in the spec and encoded here in one place; the resolver reads
    /// it to refuse writes to forbidden layers.
    #[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#scope-matrix")]
    pub const fn writable_layers(self) -> &'static [Layer] {
        match self {
            // User: global default at L1, project-shared at L2, per-project at L3.
            Scope::User => &[Layer::L1, Layer::L2, Layer::L3],
            // Machine: machine-specific, lives in the user-machine layer only.
            Scope::Machine => &[Layer::L1],
            // Project: not a global concern — L2 (shared) and L3 (personal).
            Scope::Project => &[Layer::L2, Layer::L3],
            // Team-only: L2-committed policy, user cannot override in L3.
            Scope::TeamOnly => &[Layer::L2],
        }
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

// ── Applies ─────────────────────────────────────────────────────────────────

/// When a change to this key takes effect (PROP-040 §10 `#applies`). A surface
/// shows the indicator so the user is never left guessing whether a tweak is
/// live, needs a reload, or needs a restart (the hot-reload-vs-restart pain,
/// §4.3.2). The default is [`Applies::Live`] — most preferences take effect
/// immediately.
///
/// ```
/// use vibe_settings::schema::Applies;
///
/// assert_eq!(Applies::default(), Applies::Live);
/// assert_eq!(Applies::Restart.label(), "restart");
/// ```
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#applies")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Applies {
    /// Takes effect immediately (default — most preferences).
    #[default]
    Live,
    /// Takes effect on the next surface reload.
    Reload,
    /// Takes effect only after a restart.
    Restart,
}

impl Applies {
    /// The short wire/diagnostic tag (PROP-040 §10 spellings).
    pub const fn label(self) -> &'static str {
        match self {
            Applies::Live => "live",
            Applies::Reload => "reload",
            Applies::Restart => "restart",
        }
    }
}

impl fmt::Display for Applies {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

// ── MergeStrategy ───────────────────────────────────────────────────────────

/// How an array value merges across layers (PROP-040 §4 `#merge-strategy-opt-in`).
/// Scalars always last-win and tables always deep-merge regardless of this
/// field; arrays default to [`MergeStrategy::Replace`] (the safe, predictable
/// VSCode semantics), with opt-in strategies declared per key so no array is
/// ever merged silently.
///
/// The algorithm itself lives in the resolver cell (phase 2.3); this enum is
/// the metadata the resolver reads.
///
/// ```
/// use vibe_settings::schema::MergeStrategy;
///
/// assert_eq!(MergeStrategy::default(), MergeStrategy::Replace);
/// assert_eq!(MergeStrategy::Append.label(), "append");
/// ```
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#merge-strategy-opt-in")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MergeStrategy {
    /// Higher layer's array fully replaces the lower (default; non-obvious
    /// VSCode semantics, made explicit per §4 `#merge-algorithm`).
    #[default]
    Replace,
    /// Append the higher layer's array to the lower's.
    Append,
    /// Prepend the higher layer's array to the lower's.
    Prepend,
    /// Merge element-by-element by a key field — REVIEW(phase 2.4): the
    /// identifying-field name is not yet carried; resolver (phase 2.3) will
    /// either add it or restrict `MergeByKey` to identity-by-index. Tracked as
    /// a deferred detail of §4 `#merge-strategy-opt-in`.
    MergeByKey,
}

impl MergeStrategy {
    /// The short wire/diagnostic tag (PROP-040 §4 spellings).
    pub const fn label(self) -> &'static str {
        match self {
            MergeStrategy::Replace => "replace",
            MergeStrategy::Append => "append",
            MergeStrategy::Prepend => "prepend",
            MergeStrategy::MergeByKey => "merge-by-key",
        }
    }
}

impl fmt::Display for MergeStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

// ── Deprecation ─────────────────────────────────────────────────────────────

/// `deprecated` metadata for a key — the migration target and a human-readable
/// message (PROP-040 §6 `#deprecation`). Boot emits a warning naming the
/// migration; `vibe prefs migrate` rewrites the file automatically.
///
/// ```
/// use vibe_settings::schema::Deprecation;
///
/// let d = Deprecation::with_replacement("use `tree.sort`", "node.sort");
/// assert_eq!(d.replaced_by.as_deref(), Some("node.sort"));
/// ```
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#deprecation")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deprecation {
    /// The dotted path that replaces this key, if any.
    pub replaced_by: Option<String>,
    /// The human-readable message naming the migration (non-empty).
    pub message: String,
}

impl Deprecation {
    /// A deprecation with no replacement target (the key is retired without a
    /// successor).
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Deprecation {
            replaced_by: None,
            message: message.into(),
        }
    }

    /// A deprecation pointing at a replacement key (PROP-040 §6 `#deprecation`).
    #[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#deprecation")]
    #[must_use]
    pub fn with_replacement(message: impl Into<String>, replaced_by: impl Into<String>) -> Self {
        Deprecation {
            replaced_by: Some(replaced_by.into()),
            message: message.into(),
        }
    }
}

// ── KeyMeta ─────────────────────────────────────────────────────────────────

/// Metadata for one setting key (PROP-040 §6 `#schema-fields`). A `KeyMeta`
/// carries everything the resolver, the validator, and a settings UI need to
/// know about a preference without reading a file: its dotted path, type,
/// built-in default, human description (mandatory, non-empty), scope (§7),
/// `applies` (§10), array `merge` strategy (§4), deprecation, and `restricted`
/// flag (§11.3-equivalent for untrusted L2).
///
/// Build with [`KeyMeta::new`] (validates the non-empty description) and chain
/// the `with_*` setters for optional fields. Defaults: no value, `applies =
/// Live`, `merge = Replace`, not deprecated, not restricted.
///
/// ```
/// use vibe_settings::schema::{Applies, KeyMeta, KeyType, MergeStrategy, Scope};
///
/// let key = KeyMeta::new("tree.palette", KeyType::String, Scope::User, "the Vibe Tree palette")
///     .unwrap()
///     .with_default(toml::Value::String("rosé-pine".into()))
///     .with_applies(Applies::Reload)
///     .with_merge(MergeStrategy::Replace);
/// assert_eq!(key.path, "tree.palette");
/// assert_eq!(key.applies, Applies::Reload);
/// assert!(matches!(key.default, Some(toml::Value::String(_))));
/// ```
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#schema-fields")]
#[derive(Debug, Clone, PartialEq)]
pub struct KeyMeta {
    /// The dotted path of the key, e.g. `"tree.palette"` (PROP-040 §5
    /// `#unified-introspection`).
    pub path: String,
    /// The declared value shape.
    pub key_type: KeyType,
    /// The built-in default; the resolver's lowest layer (§2 `#precedence-law`).
    pub default: Option<toml::Value>,
    /// Mandatory, non-empty human-readable description — surfaces in the
    /// settings UI (PROP-041) and AIUI introspection (§5).
    pub description: String,
    /// Where this key may be set and whether it roams (§7 `#scope-metadata`).
    pub scope: Scope,
    /// When a change takes effect (§10 `#applies`).
    pub applies: Applies,
    /// Array merge strategy (§4 `#merge-strategy-opt-in`); ignored for non-array
    /// values.
    pub merge: MergeStrategy,
    /// `deprecated` metadata, if any (§6 `#deprecation`).
    pub deprecated: Option<Deprecation>,
    /// `restricted` — read from L2 only in a trusted project; falls back to
    /// L1/default in an untrusted clone (§11 `#restricted-l2`).
    pub restricted: bool,
}

impl KeyMeta {
    /// Construct a `KeyMeta` with the mandatory fields, validating that
    /// `description` and `path` are non-empty (PROP-040 §6 `#schema-fields`:
    /// description is *mandatory, non-empty*). Optional fields default to:
    /// `default = None`, `applies = Live`, `merge = Replace`, no deprecation,
    /// `restricted = false`. Chain the `with_*` methods to override.
    ///
    /// ```
    /// # use vibe_settings::schema::{KeyMeta, KeyType, Scope, SchemaError};
    /// let key = KeyMeta::new("tree.fold", KeyType::Bool, Scope::User, "fold subtrees by default")?;
    /// assert_eq!(key.applies.to_string(), "live");
    /// assert_eq!(key.merge.to_string(), "replace");
    /// assert!(!key.restricted);
    /// # Ok::<(), SchemaError>(())
    /// ```
    #[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#schema-fields")]
    pub fn new(
        path: impl Into<String>,
        key_type: KeyType,
        scope: Scope,
        description: impl Into<String>,
    ) -> Result<Self, SchemaError> {
        let path = path.into();
        let description = description.into();
        if path.trim().is_empty() {
            return Err(SchemaError::EmptyPath);
        }
        if description.trim().is_empty() {
            return Err(SchemaError::EmptyDescription { path });
        }
        Ok(KeyMeta {
            path,
            key_type,
            default: None,
            description,
            scope,
            applies: Applies::Live,
            merge: MergeStrategy::Replace,
            deprecated: None,
            restricted: false,
        })
    }

    /// Set the built-in default (chains).
    #[must_use]
    pub fn with_default(mut self, value: toml::Value) -> Self {
        self.default = Some(value);
        self
    }

    /// Override `applies` (chains).
    #[must_use]
    pub fn with_applies(mut self, applies: Applies) -> Self {
        self.applies = applies;
        self
    }

    /// Override the array `merge` strategy (chains).
    #[must_use]
    pub fn with_merge(mut self, merge: MergeStrategy) -> Self {
        self.merge = merge;
        self
    }

    /// Mark this key as deprecated (chains).
    #[must_use]
    pub fn with_deprecation(mut self, deprecation: Deprecation) -> Self {
        self.deprecated = Some(deprecation);
        self
    }

    /// Mark this key as `restricted` — L2-readable only in a trusted project
    /// (§11 `#restricted-l2`). Chains.
    #[must_use]
    pub fn restricted(mut self) -> Self {
        self.restricted = true;
        self
    }
}

// ── SchemaError ─────────────────────────────────────────────────────────────

/// Why a schema operation failed — registering a key, or constructing a
/// [`KeyMeta`] with a bad shape. Each variant cites the violated REQ anchor so
/// a diagnostic can point the reader at the contract clause (PROP-040 §6
/// `#schema-first`, §6 `#schema-fields`).
///
/// ```
/// use vibe_settings::schema::{KeyMeta, KeyType, SchemaError, Scope};
///
/// // An empty description violates §6 #schema-fields.
/// let err = KeyMeta::new("tree.x", KeyType::Bool, Scope::User, "   ").unwrap_err();
/// assert!(matches!(err, SchemaError::EmptyDescription { .. }));
/// assert!(err.to_string().contains("schema-fields"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#schema-first")]
pub enum SchemaError {
    /// The `path` was empty — a key must have a non-empty dotted path.
    #[error(
        "a setting key's path is required and must be non-empty \
         (violates spec://vibevm/modules/vibe-settings/PROP-040#schema-fields; \
          fix: pass a dotted path such as `tree.palette`)"
    )]
    EmptyPath,

    /// The `description` was empty — PROP-040 §6 `#schema-fields` mandates a
    /// non-empty description for every key.
    #[error(
        "setting `{path}` is missing a description — PROP-040 §6 mandates a non-empty description \
         (violates spec://vibevm/modules/vibe-settings/PROP-040#schema-fields; \
          fix: document the key's effect for the settings UI and AIUI)"
    )]
    EmptyDescription {
        /// The key whose description was empty.
        path: String,
    },

    /// The `path` is already registered — a *collision*, never a silent
    /// override (mirrors `vibe-actions`'s registry discipline; §6
    /// `#schema-first`).
    #[error(
        "setting `{path}` is already registered — duplicate registration \
         (violates spec://vibevm/modules/vibe-settings/PROP-040#schema-first; \
          fix: pick a distinct path, or drop the duplicate declaration)"
    )]
    DuplicateKey {
        /// The colliding path.
        path: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(path: &str, ty: KeyType, scope: Scope) -> KeyMeta {
        KeyMeta::new(path, ty, scope, "a test setting").unwrap()
    }

    // ── KeyType / Scope / Applies / MergeStrategy metadata ───────────────────

    #[test]
    fn key_type_labels_match_spec_spellings() {
        assert_eq!(KeyType::Bool.label(), "bool");
        assert_eq!(KeyType::Int.label(), "int");
        assert_eq!(KeyType::String.label(), "string");
        assert_eq!(KeyType::Enum.label(), "enum");
        assert_eq!(KeyType::Array.label(), "array");
        assert_eq!(KeyType::Table.label(), "table");
        assert_eq!(KeyType::Table.to_string(), "table");
    }

    #[test]
    fn scope_writable_layers_encode_the_matrix() {
        // PROP-040 §7 #scope-matrix — encoded once, right here.
        assert_eq!(
            Scope::User.writable_layers(),
            &[Layer::L1, Layer::L2, Layer::L3]
        );
        assert_eq!(Scope::Machine.writable_layers(), &[Layer::L1]);
        assert_eq!(Scope::Project.writable_layers(), &[Layer::L2, Layer::L3]);
        assert_eq!(Scope::TeamOnly.writable_layers(), &[Layer::L2]);
    }

    #[test]
    fn scope_labels_and_display() {
        assert_eq!(Scope::User.label(), "user");
        assert_eq!(Scope::Machine.label(), "machine");
        assert_eq!(Scope::Project.label(), "project");
        assert_eq!(Scope::TeamOnly.label(), "team-only");
        assert_eq!(Scope::TeamOnly.to_string(), "team-only");
    }

    #[test]
    fn applies_default_is_live() {
        assert_eq!(Applies::default(), Applies::Live);
        assert_eq!(Applies::Live.label(), "live");
        assert_eq!(Applies::Reload.label(), "reload");
        assert_eq!(Applies::Restart.label(), "restart");
    }

    #[test]
    fn merge_strategy_default_is_replace() {
        assert_eq!(MergeStrategy::default(), MergeStrategy::Replace);
        assert_eq!(MergeStrategy::Replace.label(), "replace");
        assert_eq!(MergeStrategy::Append.label(), "append");
        assert_eq!(MergeStrategy::Prepend.label(), "prepend");
        assert_eq!(MergeStrategy::MergeByKey.label(), "merge-by-key");
    }

    // ── Deprecation ────────────────────────────────────────────────────────

    #[test]
    fn deprecation_constructors() {
        let d = Deprecation::new("retired");
        assert_eq!(d.replaced_by, None);
        assert_eq!(d.message, "retired");

        let d = Deprecation::with_replacement("use b", "tree.b");
        assert_eq!(d.replaced_by.as_deref(), Some("tree.b"));
    }

    // ── KeyMeta construction ───────────────────────────────────────────────

    #[test]
    fn key_meta_new_sets_defaults_for_optional_fields() {
        let k = key("tree.palette", KeyType::String, Scope::User);
        assert_eq!(k.path, "tree.palette");
        assert_eq!(k.key_type, KeyType::String);
        assert_eq!(k.scope, Scope::User);
        assert!(k.default.is_none());
        assert_eq!(k.applies, Applies::Live);
        assert_eq!(k.merge, MergeStrategy::Replace);
        assert!(k.deprecated.is_none());
        assert!(!k.restricted);
    }

    #[test]
    fn key_meta_builder_chains() {
        let k = KeyMeta::new("tree.fold", KeyType::Bool, Scope::User, "fold")
            .unwrap()
            .with_default(toml::Value::Boolean(true))
            .with_applies(Applies::Restart)
            .with_merge(MergeStrategy::Append)
            .with_deprecation(Deprecation::new("old"))
            .restricted();
        assert!(matches!(k.default, Some(toml::Value::Boolean(true))));
        assert_eq!(k.applies, Applies::Restart);
        assert_eq!(k.merge, MergeStrategy::Append);
        assert!(k.deprecated.is_some());
        assert!(k.restricted);
    }

    #[test]
    fn key_meta_rejects_empty_description_citing_schema_fields() {
        let err = KeyMeta::new("tree.x", KeyType::Bool, Scope::User, "   ").unwrap_err();
        // Capture the Display string before the match moves `err`'s `path`.
        let msg = err.to_string();
        match err {
            SchemaError::EmptyDescription { path } => {
                assert_eq!(path, "tree.x");
            }
            other => panic!("expected EmptyDescription, got {other:?}"),
        }
        // Diagnostic points at the REQ.
        assert!(msg.contains("schema-fields"));
    }

    #[test]
    fn key_meta_rejects_empty_path() {
        let err = KeyMeta::new("   ", KeyType::Bool, Scope::User, "doc").unwrap_err();
        assert!(matches!(err, SchemaError::EmptyPath));
    }
}
