//! The `from-clones` scanner cell — the org directory already exists
//! locally (a vendor mirror, a cron's clone tree), so reaching the
//! source is a no-op and scanning IS the walk. PROP-005 §2.8's
//! default source for operators who maintain their own clone tree.

specmark::scope!("spec://vibevm/modules/vibe-index/PROP-005#reindex");

use std::path::PathBuf;

use specmark::{cell, spec};

use crate::error::Result;
use crate::index::checkpoint::Checkpoint;
use crate::scanner::PackageScanner;
use crate::scanner::org_walk::{FromClonesOptions, ScanReport, scan_org_dir_with_filter};

/// Walks a local directory of org clones, one subdirectory per
/// package repo. The directory is taken verbatim — the operator (or
/// the composition root, for `--from-clones`) guarantees it exists.
#[cell(seam = "PackageScanner", variant = "from-clones")]
#[spec(implements = "spec://vibevm/modules/vibe-index/PROP-005#reindex")]
pub struct FromClonesScanner {
    /// The org directory — one subdirectory per package repo clone.
    pub org_dir: PathBuf,
}

impl PackageScanner for FromClonesScanner {
    fn scan(&self, walk: &FromClonesOptions, prior: Option<&Checkpoint>) -> Result<ScanReport> {
        scan_org_dir_with_filter(&self.org_dir, walk, prior)
    }
}
