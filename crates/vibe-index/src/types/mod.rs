//! Wire-types for the index — all `Serialize`/`Deserialize` shapes
//! that travel to disk (`primary.jsonl`, `by-name/<kind>/<name>.json`,
//! `repomd.json`) or out of the HTTP API.
//!
//! Types here mirror the `vibe.toml` schema in `vibe-core`
//! deliberately rather than re-using `vibe-core::manifest`. PROP-005
//! §3.2 explained the trade-off: standalone redistribution beats
//! workspace re-use, so we duplicate the relevant subset and let a
//! parity test (slice 3) catch divergence at CI time.

pub mod entry;
pub mod kinds;
pub mod repomd;

pub use entry::{
    BootSnippetEntry, CompatibilityEntry, ConflictsEntry, DeliveryMode, FeaturesEntry, I18nEntry,
    ObsoletesEntry, PackageEntry, ProvidesEntry, RequiresAnyEntry, RequiresEntry, SubskillEntry,
    VersionEntry,
};
pub use kinds::{NamingConvention, PackageKind};
pub use repomd::{Repomd, RepomdFileEntry};
