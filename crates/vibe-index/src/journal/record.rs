//! The journal's record vocabulary — one registry fact per NDJSON line.
//!
//! The journal is the TRUTH layer of PROP-044 §3: the catalog is a
//! projection that can be torn down and rebuilt from these records,
//! and a writer never accepts as input what it itself published. This
//! module defines only the shapes; reading and writing them lives in
//! [`super::store`].

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use chrono::{DateTime, Utc};
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::types::{Group, NamingConvention, VersionEntry};

/// One journal record — NDJSON, one per line.
///
/// `at` is the moment of the FACT, supplied by the caller: the clock
/// enters at the edge and never inside this module (PROP-044 §4.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalRecord {
    pub at: DateTime<Utc>,
    pub actor: String,
    pub event: Event,
}

/// One registry fact. Not to be confused with the namesake
/// `progress_core::journal::Event` — the CAMPAIGN journal of PROP-043,
/// a different crate keeping a different law; this one is the package
/// registry's journal of catalog facts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Event {
    /// The registry's identity, written by `init` as the journal's
    /// first record. It lives HERE and not in the catalog because a
    /// projection rebuilds the catalog from scratch: an identity
    /// readable only from `repomd.json` would be an authoritative fact
    /// living in a derived artifact (PROP-044 `##FORBID-SECRET-TRUTH`),
    /// and `rebuild --check` — tear down, reproject, compare — could
    /// never pass.
    Initialised {
        registry: String,
        registry_url: String,
        naming: NamingConvention,
    },
    /// The entry is BOXED, and the box is invisible on the wire: serde
    /// renders `Box<T>` exactly as `T`, so the NDJSON shape is unchanged.
    /// It is here because a catalog entry is ~900 bytes against a handful
    /// for every other variant, and `replay` hands back a
    /// `Vec<JournalRecord>` — unboxed, every record in that vector would
    /// reserve the largest variant's width no matter which fact it holds.
    Published { entry: Box<VersionEntry> },
    /// A freeze OBSERVED by this registry, not authored by it: the
    /// authority is the flag inside the package's hashed content
    /// (ruling D14); the event records when this registry saw the
    /// freeze and which hash it was tied to.
    Frozen {
        group: Group,
        name: String,
        version: Version,
        content_hash: String,
    },
    Yanked {
        group: Group,
        name: String,
        version: Version,
        reason: String,
    },
    Removed {
        group: Group,
        name: String,
        version: Option<Version>,
    },
    Renamed {
        from: (Group, String),
        to: (Group, String),
    },
    Notice {
        group: Group,
        name: String,
        text: String,
    },
    ChannelSet {
        group: Group,
        name: String,
        channel: String,
        version: Version,
    },
    ChannelUnset {
        group: Group,
        name: String,
        channel: String,
    },
    ForceReplaced {
        group: Group,
        name: String,
        version: Version,
        old_hash: String,
        new_hash: String,
        reason: String,
    },
    /// A full scan asserts its result is the whole published-entry set
    /// as of this record. The projector clears the ENTRY set and keeps
    /// every registry fact — tombstones, yanks, freezes and notices
    /// are not scan products and a scan cannot re-derive them, so a
    /// watershed that dropped them would make state unrecoverable
    /// (PROP-044 law 2).
    EntrySetReplaced { source: String },
}
