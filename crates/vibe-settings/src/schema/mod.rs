//! The schema-first preference surface — `KeyMeta`, `Scope`, the enumerable
//! registry, and validation (PROP-040 §6 `#schema`, §7 `#scope-meta`,
//! §10 `#applies`, §4 `#merge-strategy-opt-in`).
//!
//! Every preference key is **declared** up front with type, default, scope, and
//! metadata (§6 `#schema-first`); unknown keys (typos, retired names) surface
//! as non-fatal warnings, never a silent ignore. The schema is a single typed,
//! tagged, enumerable tree (§5 `#unified-introspection`): [`Schema::keys`]
//! reaches every setting, [`Schema::get`] reads one, [`Schema::paths_in`]
//! fetches a namespace — so an agent (or test, or CLI) can walk the surface
//! without parsing files.
//!
//! This cell is **metadata only** — the merge algorithm, layer resolution, and
//! `set()` scope-refusal live in the resolver cell (phase 2.3). Here we carry
//! the typed shape of each key + a non-fatal validator; the resolver consumes
//! it.
//!
//! The cell splits along responsibility seams to honour the ≤600-line AI-Native
//! file budget:
//!   - [`types`] — the typed enums (`KeyType`, `Scope`, `Applies`,
//!     `MergeStrategy`), `Deprecation`, `KeyMeta`, and `SchemaError`;
//!   - [`registry`] — the enumerable [`Schema`] registry;
//!   - [`validate`] — `unknown_keys` / `validate` / `Diagnostic`.
//!
//! Frontend-agnostic (PROP-040 §1 `#frontend-agnostic`): `std`, `toml::Value`
//! for defaults, `thiserror` for typed errors — zero rendering deps.
//!
//! Spec: [PROP-040 §6, §7](../../../../../../spec/modules/vibe-settings/PROP-040-settings.md#schema).

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-040#schema");

mod registry;
mod types;
mod validate;

pub use registry::Schema;
pub use types::{Applies, Deprecation, KeyMeta, KeyType, MergeStrategy, SchemaError, Scope};
pub use validate::{Diagnostic, DiagnosticKind, unknown_keys, validate};
