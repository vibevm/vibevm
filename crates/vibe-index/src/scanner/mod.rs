//! Source-of-truth walkers behind the `PackageScanner` seam: a cell
//! per source kind decides how the org's package repos come to exist
//! locally (`from-clones` takes a directory verbatim; `from-github`
//! clones the org via the REST API first), and the shared walk in
//! [`org_walk`] folds every `(repo, v-tag)` into entries the reindex
//! command upserts into the in-memory [`Index`](crate::index::Index).
//! The GitVerse arm stays an error-returning stub in the reindex
//! command, not a cell — the upstream API exposes no org-scoped repo
//! enumeration yet.

specmark::scope!("spec://vibevm/modules/vibe-index/PROP-005#root");

use crate::error::Result;
use crate::index::checkpoint::Checkpoint;

pub mod from_clones;
pub mod from_github;
pub mod git_cli;
pub mod manifest;
pub mod org_walk;

pub use from_clones::FromClonesScanner;
pub use from_github::{FromGithubOptions, FromGithubScanner, clone_org, list_repos};
pub use org_walk::{FromClonesOptions, ScanReport, SkipNote, scan_org_dir};

/// The scan seam (PROP-005 §2.8): one cell per source kind, selected
/// at the reindex composition root. A scanner reaches its source and
/// folds every `(repo, v-tag)` into index entries; `prior` enables
/// the incremental filter — repos whose recorded snapshot still
/// matches are skipped, and the reindex driver carries their existing
/// entries forward unchanged.
///
/// ```
/// use vibe_index::scanner::{FromClonesOptions, FromClonesScanner, PackageScanner};
/// use vibe_index::types::NamingConvention;
///
/// let org = tempfile::tempdir().unwrap();
/// let scanner = FromClonesScanner {
///     org_dir: org.path().to_path_buf(),
/// };
/// let opts = FromClonesOptions {
///     registry: "vibespecs".into(),
///     registry_url: "https://github.com/vibespecs".into(),
///     naming: NamingConvention::Fqdn,
///     generator: "doctest".into(),
///     indexed_at: chrono::Utc::now(),
/// };
/// // An empty org dir scans to an empty report through the seam.
/// let report = scanner.scan(&opts, None).unwrap();
/// assert!(report.entries.is_empty());
/// ```
pub trait PackageScanner {
    fn scan(&self, walk: &FromClonesOptions, prior: Option<&Checkpoint>) -> Result<ScanReport>;
}
