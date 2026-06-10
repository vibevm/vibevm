//! Source-of-truth walkers — `from_clones` walks a local org-dir of
//! package-repo clones; `from_github` and `from_gitverse` (slice 8)
//! walk host APIs. Each scanner produces an [`Iterator<Item =
//! ScanResult<VersionEntry>>`] that the reindex command folds into
//! the in-memory [`Index`](crate::index::Index).

specmark::scope!("spec://vibevm/modules/vibe-index/PROP-005#root");

pub mod from_clones;
pub mod from_github;
pub mod git_cli;
pub mod manifest;

pub use from_clones::{FromClonesOptions, scan_org_dir};
pub use from_github::{FromGithubOptions, clone_org, list_repos};
