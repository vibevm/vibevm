//! `VersionEntry` — the canonical per-version index record, and the
//! re-export seam that makes the index's types the wire's types.
//! Schema: `schemas/index/e1/entry.jtd.json`, described for a reader by
//! PROP-005 §2.6. Every line of `primary.jsonl` is
//! one of these; every element of a `by-name/<name>.json` candidate's
//! `versions[]` is one of these; every `POST /v1/packages` body is one
//! of these.
//!
//! The definitions live in `vibe_wire::generated` (JTD is the source
//! of truth, PROP-000 §16); the behaviour that used to sit beside the
//! hand-written shapes (`SCHEMA_VERSION`, `minimal`, `sort_key`, the
//! `is_empty`/`Default` family, `new`/`finalise`) moved with the
//! orphan rule into `vibe_wire::behaviour`. The per-version
//! projections are split by concern as before — dependency relations
//! in `relations`, content and delivery in `content`, the aggregate
//! records in `aggregate` — and all are re-exported here, so every
//! `crate::types::*` path is unchanged.
//!
//! Reader tolerance (PROP-044 §4.4) is the generated form's own law
//! now: unknown fields are read and ignored, never rejected. The
//! catalog is a projection of the journal — what is read is never
//! written back — so there is nothing left to lose.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#entry");

mod aggregate;
mod content;
mod relations;

pub use aggregate::{NameEntry, PackageEntry, Tombstone};
pub use content::{
    BootSnippetEntry, DeliveryMode, FeaturesEntry, I18nEntry, SubskillEntry, WorkspaceOriginEntry,
};
pub use relations::{
    CompatibilityEntry, ConflictsEntry, ObsoletesEntry, ProvidesEntry, RequiresAnyEntry,
    RequiresEntry,
};
pub use vibe_wire::generated::shared::VersionEntry;

#[cfg(test)]
mod tests;
