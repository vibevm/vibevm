//! The check cells — one module per [`CheckId`](crate::CheckId).
//!
//! Each submodule holds exactly one unit struct implementing the
//! [`Check`](crate::Check) seam, carrying a `#[cell]` manifest, and
//! registered once in [`crate::all_checks`]. A cell imports the seam
//! and shared core (this module + the crate root) only — never a
//! sibling cell.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#linter");

pub mod activation_conflict;
pub mod boot_directory;
pub mod features_graph;
pub mod i18n_coverage;
pub mod lockfile_files;
pub mod manifest_validity;
pub mod redirect_block;
pub mod review_aging;
pub mod subskill_structure;
pub mod wal_freshness;
pub mod wal_wellformed;

pub use activation_conflict::ActivationConflictCheck;
pub use boot_directory::BootDirectoryCheck;
pub use features_graph::FeaturesGraphCheck;
pub use i18n_coverage::I18nCoverageCheck;
pub use lockfile_files::LockfileFilesCheck;
pub use manifest_validity::ManifestValidityCheck;
pub use redirect_block::RedirectBlockCheck;
pub use review_aging::ReviewAgingCheck;
pub use subskill_structure::SubskillStructureCheck;
pub use wal_freshness::WalFreshnessCheck;
pub use wal_wellformed::WalWellformedCheck;

use std::path::{Path, PathBuf};

use vibe_core::manifest::Manifest;

/// Find every locally-discoverable manifest. Today: scans
/// `packages/` (vibevm's own dogfooding tree) at depth 3. Also includes
/// the project root itself, which always carries a `vibe.toml`. Returns
/// `(manifest_root, label)` pairs. Consumers ([`features_graph`],
/// [`i18n_coverage`], [`subskill_structure`]) tolerate a
/// non-package `vibe.toml`: they short-circuit on an empty
/// `[features]` / `[i18n]` table and a missing `subskills/` tree.
pub(crate) fn scan_local_packages(project_root: &Path) -> Vec<(PathBuf, String)> {
    let mut out: Vec<(PathBuf, String)> = Vec::new();
    if project_root.join(Manifest::FILENAME).is_file() {
        out.push((project_root.to_path_buf(), "project root".to_string()));
    }
    let packages_dir = project_root.join("packages");
    if packages_dir.is_dir() {
        for entry in walkdir::WalkDir::new(&packages_dir)
            .max_depth(4)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_name() == Manifest::FILENAME
                && let Some(parent) = entry.path().parent()
            {
                let rel = parent
                    .strip_prefix(project_root)
                    .map(|p| p.display().to_string().replace('\\', "/"))
                    .unwrap_or_else(|_| parent.display().to_string());
                out.push((parent.to_path_buf(), rel));
            }
        }
    }
    out
}
