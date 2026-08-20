//! Wire-types for the index — the `Serialize`/`Deserialize` shapes
//! that travel to disk (`primary.jsonl`, `by-name/<name>.json`,
//! `repomd.json`) or out of the HTTP API.
//!
//! The shapes are the generated wire types of `vibe-wire`, re-exported
//! here so every `vibe_index::types::*` path keeps its meaning while
//! the definition lives once, beside the schemas it is generated from
//! (PROP-000 §16). PROP-005 §3.2's standalone-duplicate trade-off is
//! retired: the wire edge is runtime now, because these types ARE the
//! wire's types. Still hand-written and staying that way: `repomd`
//! (its `size` is `u64` riding the wire as a canonical decimal string —
//! the B-091 fork, ruled 2026-08-20, `formats/breaks/003.md` — and its
//! file-entry union is tagged on this side's own law).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

pub mod entry;
pub mod kinds;
pub mod repomd;
pub(crate) mod wire_decimal;

pub use entry::{
    BootSnippetEntry, CompatibilityEntry, ConflictsEntry, DeliveryMode, FeaturesEntry, I18nEntry,
    NameEntry, ObsoletesEntry, PackageEntry, ProvidesEntry, RequiresAnyEntry, RequiresEntry,
    SubskillEntry, Tombstone, VersionEntry, WorkspaceOriginEntry,
};
pub use kinds::{NamingConvention, PackageKind};
pub use repomd::{Repomd, RepomdFileEntry};
pub use vibe_wire::generated::index::e1::by_purl::BindingSite;

/// Re-export of the reverse-FQDN [`Group`](vibe_core::Group) qualifier
/// (PROP-008 §2.1) — part of every index entry's identity, surfaced here
/// so consumers of `vibe_index::types` need not also depend on `vibe-core`.
pub use vibe_core::Group;
