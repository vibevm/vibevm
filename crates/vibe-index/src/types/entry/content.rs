//! Content and delivery projections carried by a
//! [`VersionEntry`](super::VersionEntry) — re-exports of the generated
//! wire types (shared vocabulary; JTD is the source of truth). Each
//! mirrors a `vibe.toml` table (PROP-005 §2.6).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#entry");

pub use vibe_wire::generated::shared::{
    BootSnippetEntry, DeliveryMode, FeaturesEntry, I18nEntry, SubskillEntry, WorkspaceOriginEntry,
};
