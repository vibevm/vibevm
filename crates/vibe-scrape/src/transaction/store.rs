//! Capability-rooted durable storage for scrape transactions.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-E");

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest as _, Sha256};
use vibe_safefs::{
    CleanupIntent, CleanupPreparation, DirectoryDurability, EntryIdentity, EntryState,
    EntryStateKind, ExternalDirectory, ExternalProjectLock, ExternalStore, OwnedDirectory,
    OwnedDirectoryCreateError, OwnedDirectoryIdentity, OwnedTreeCleanupError,
    OwnedTreeCleanupProgress, Project, TreeEntry as SafefsTreeEntry,
    TreeManifest as SafefsTreeManifest,
};
use vibe_wire::generated::scrape::e1::{plan::Plan as ScrapePlanWire, report as report_wire};

use super::model::*;
use super::report::report_to_wire_plan;
use super::sha256::project_key as derive_project_key;
use super::traits::{ProjectLock, TransactionStore};
use super::validate;

const JOURNAL_FILE: &str = "journal.json";
const OWNER_FILE: &str = "owner.json";
const TRANSACTIONS_DIRECTORY: &str = "t";
const REPORTS_DIRECTORY: &str = "reports";
const SNAPSHOTS_DIRECTORY: &str = "snapshots";
const VERIFICATION_DIRECTORY: &str = "v";
// The embedded canonical plan is itself bounded to 16 MiB. JSON string
// escaping plus the executable recovery projection require a separately
// bounded envelope rather than silently making the plan's legal maximum
// unpersistable.
const MAX_OWNER_BYTES: usize = 4096;
const MAX_RETIREMENT_BYTES: usize = 16 * 1024 * 1024;
const MAX_SNAPSHOT_BYTES: usize = 64 * 1024 * 1024;
const MAX_SNAPSHOT_TOTAL_BYTES: u64 = 256 * 1024 * 1024;
const MAX_DIRECTORY_CHILDREN: usize = 16_384;
const MAX_TREE_ENTRIES: usize = 65_536;

include!("store/trait_methods.rs");
include!("store/inherent_methods.rs");

static TRANSACTION_NONCE: AtomicU64 = AtomicU64::new(0);

/// A transaction store rooted at an explicit absolute, caller-injected state
/// directory. The directory is not opened until a project has been pinned, so
/// the first creation can be ordered after the disjointness proof.
#[derive(Debug)]
pub struct SystemTransactionStore {
    state_root: PathBuf,
    external: Option<ExternalStore>,
    proven_project: Option<ProjectKey>,
    proven_display_root: Option<String>,
    external_lock: Option<ExternalProjectLock>,
    locked_project: Option<ProjectKey>,
    live_verification_workspace: Option<OwnedDirectory>,
}

impl SystemTransactionStore {
    pub fn new(state_root: impl Into<PathBuf>) -> Result<Self, TransactionError> {
        let state_root = state_root.into();
        if !state_root.is_absolute() {
            return Err(TransactionError::Store(
                "scrape transaction state root must be absolute".to_owned(),
            ));
        }
        Ok(Self {
            state_root,
            external: None,
            proven_project: None,
            proven_display_root: None,
            external_lock: None,
            locked_project: None,
            live_verification_workspace: None,
        })
    }

    #[must_use]
    pub fn state_root(&self) -> &Path {
        &self.state_root
    }

    /// Read one exact journaled snapshot for a recovery adapter. The name must
    /// occur in the journal's durable prefix; source contract paths are never
    /// consulted.
    pub fn read_snapshot(
        &mut self,
        journal: &Journal,
        name: &str,
    ) -> Result<Vec<u8>, TransactionError> {
        self.require_locked(&journal.project_key)?;
        let index = journal
            .snapshots
            .iter()
            .position(|record| record.name == name)
            .ok_or_else(|| {
                TransactionError::Store(format!("snapshot `{name}` is not journaled"))
            })?;
        if index >= journal.snapshots_persisted {
            return Err(TransactionError::Store(format!(
                "snapshot `{name}` is outside the durable prefix"
            )));
        }
        self.read_snapshot_record(journal, &journal.snapshots[index])?
            .ok_or_else(|| TransactionError::Store(format!("snapshot `{name}` is absent")))
    }

    fn canonical_report_bytes(
        &mut self,
        journal: &Journal,
        report: &TransactionReport,
    ) -> Result<Vec<u8>, TransactionError> {
        let plan: ScrapePlanWire =
            strict_json_parse(&journal.canonical_plan, "embedded canonical scrape plan")?;
        let wire = report_to_wire_plan(report, &plan)?;
        validate_canonical_report_identity(journal, report, &wire)?;
        strict_json_bytes(
            &wire,
            MAX_CANONICAL_REPORT_BYTES,
            "canonical transaction report",
        )
    }

    fn require_stable_complete_report(
        &mut self,
        journal: &Journal,
    ) -> Result<Vec<u8>, TransactionError> {
        let report = journal
            .report
            .as_ref()
            .ok_or_else(|| store_error("complete journal has no embedded report"))?;
        if report.cleanup != Cleanup::Complete {
            return Err(store_error(
                "transaction retirement requires cleanup-complete report evidence",
            ));
        }
        let expected = self.canonical_report_bytes(journal, report)?;
        let observed = self
            .external()?
            .read_stable_bounded(
                &report_relative(&journal.transaction_id)?,
                MAX_CANONICAL_REPORT_BYTES,
            )
            .map_err(|error| store_error(format!("reading stable report: {error:#}")))?
            .ok_or_else(|| store_error("stable complete report is absent"))?;
        if observed.bytes != expected {
            return Err(store_error(
                "stable report differs from the canonical complete journal report",
            ));
        }
        Ok(expected)
    }

    system_transaction_store_inherent_methods!();
}

include!("store/domain_defs.rs");
include!("store/safefs_wire.rs");

impl TransactionStore for SystemTransactionStore {
    system_transaction_store_trait_methods!();
}

include!("store/codec_identity.rs");
include!("store/updates.rs");
include!("store/snapshot_manifest.rs");

#[cfg(test)]
mod tests {
    include!("store/tests_core.rs");
    include!("store/tests_updates.rs");
}
