//! In-memory index + on-disk persistence.
//!
//! The [`Index`] struct holds the canonical RAM copy. Persistence
//! reads and writes the on-disk files described in PROP-005 §2.4
//! atomically (tmp + rename + fsync). Slice 2 ships the read/write
//! pipeline for `repomd.json`, `primary.jsonl`, and
//! `by-name/<kind>/<name>.json`. `by-cap/` and `by-purl/` join in
//! later slices.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#layout");

pub mod by_name;
pub mod checkpoint;
pub mod inverted;
pub mod memory;
pub mod persistence;
pub mod primary;
pub mod quarantine;
pub mod repomd;
pub mod search;

pub use memory::Index;
pub use persistence::{atomic_write, compute_sha256_hex, sha256_of_bytes};
pub use quarantine::Quarantined;
pub use search::{SearchHit, lookup_capability, lookup_purl, search, tokenise};

/// The single files [`Index::write_to`] owns at the data directory's
/// root — the fixed part of the catalog's surface (PROP-005 §2.4).
///
/// **Stated once, here.** Two consumers ask the same question of a
/// catalog — "is this still the projection of its journal?" —
/// `cargo xtask rebuild --check` and the golden-corpus test, and both
/// need this list to know what the comparison owns. It used to be
/// written out twice, identically, in those two places; a second copy
/// of a normative value diverges exactly where neither reader survives
/// the divergence, because the one holding the shorter list goes GREEN
/// over a real drift instead of naming it.
///
/// **A whitelist, and deliberately not a directory walk.** The data
/// directory also carries `README.md` and `.gitignore` (written once by
/// `init`) and the whole of `state/` — the journal, the server lock, the
/// scanner checkpoint. None of them is produced by the writer, so none
/// of them belongs to the comparison, and the rule that says so holds
/// tomorrow too: the catalog IS what the writer writes. A blacklist
/// would have to enumerate the world and would rot the day the
/// directory grows a file nobody listed.
pub const WRITER_FILES: [&str; 3] = ["repomd.json", "primary.jsonl", "primary.jsonl.gz"];

/// The directory trees [`Index::write_to`] owns — the recursive part of
/// its surface. Same single-home reason as [`WRITER_FILES`].
pub const WRITER_DIRS: [&str; 3] = ["by-name", "by-cap", "by-purl"];
