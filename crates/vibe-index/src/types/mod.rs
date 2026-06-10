//! Wire-types for the index — all `Serialize`/`Deserialize` shapes
//! that travel to disk (`primary.jsonl`, `by-name/<name>.json`,
//! `repomd.json`) or out of the HTTP API.
//!
//! Types here mirror the `vibe.toml` schema in `vibe-core`
//! deliberately rather than re-using `vibe-core::manifest`. PROP-005
//! §3.2 explained the trade-off: standalone redistribution beats
//! workspace re-use, so we duplicate the relevant subset and let a
//! parity test (slice 3) catch divergence at CI time.

specmark::scope!("spec://vibevm/modules/vibe-index/PROP-005#root");

pub mod entry;
pub mod kinds;
pub mod repomd;

pub use entry::{
    BootSnippetEntry, CompatibilityEntry, ConflictsEntry, DeliveryMode, FeaturesEntry, I18nEntry,
    NameEntry, ObsoletesEntry, PackageEntry, ProvidesEntry, RequiresAnyEntry, RequiresEntry,
    SubskillEntry, VersionEntry, WorkspaceOriginEntry,
};
pub use kinds::{NamingConvention, PackageKind};
pub use repomd::{Repomd, RepomdFileEntry};

/// Re-export of the reverse-FQDN [`Group`](vibe_core::Group) qualifier
/// (PROP-008 §2.1) — part of every index entry's identity, surfaced here
/// so consumers of `vibe_index::types` need not also depend on `vibe-core`.
pub use vibe_core::Group;
