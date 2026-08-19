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
    /// A bare name closed — the third and last of the withdrawal
    /// operations, beside [`Event::Yanked`] (one version) and
    /// [`Event::Removed`] (deletion). It is the ONE retirement fact:
    /// a rename is a retirement that names its successor (PROP-005
    /// §2.11), so the `renamed` arm this replaces has no heir of its
    /// own and none is coming.
    ///
    /// `name` is bare, not a `(Group, String)` pair as every other
    /// identity-bearing arm here carries — and the asymmetry is the
    /// file layout's rather than this type's: a tombstone rides on
    /// `by-name/<name>.json`, the candidate-set file that spans every
    /// group at once (PROP-005 §2.4), so burying a name closes it for
    /// all of them.
    ///
    /// `reason` is required, which is the difference that decided the
    /// collapse: a tombstone cannot exist without one, and `renamed`
    /// carried none — keeping both would have meant either
    /// synthesising prose into a required field or adding a reason to
    /// `renamed`, after which the two differed only in whether the
    /// successor was optional. That is one thing spelled twice.
    Buried {
        name: String,
        reason: String,
        /// Absent from the wire when `None` — the field is
        /// `optionalProperties` in the schema, matching
        /// `tombstone.superseded_by` byte for byte, because the
        /// projection copies it across verbatim.
        #[serde(skip_serializing_if = "Option::is_none")]
        superseded_by: Option<String>,
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
