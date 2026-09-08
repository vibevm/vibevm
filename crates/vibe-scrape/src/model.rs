//! Domain model shared by planning and typed rewrite adapters.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-A");

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::contract::{Contract, ContractAction, Owner};

mod wire;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrapeRequest {
    pub root: PathBuf,
    pub contract: Option<PathBuf>,
    pub mode: ScrapeMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ScrapeMode {
    InPlace,
    Export { output: PathBuf },
}

#[derive(Debug, Clone)]
pub struct ContractSnapshot {
    pub source_path: PathBuf,
    pub display_path: String,
    pub contained: bool,
    pub bytes: Vec<u8>,
    pub sha256: String,
    pub identity: vibe_safefs::FileIdentity,
    pub value: Contract,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Inventory {
    pub entries: Vec<InventoryEntry>,
    pub tree_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InventoryEntry {
    pub path: String,
    pub kind: EntryKind,
    pub sha256: Option<String>,
    pub bytes: Option<u64>,
    pub unix_mode: Option<u32>,
    #[serde(skip)]
    pub identity: Option<vibe_safefs::FileIdentity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EntryKind {
    File,
    Directory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PreparedRewrite {
    pub id: String,
    pub kind: String,
    pub path: String,
    pub adapter_epoch: u32,
    pub spans: Vec<ByteSpan>,
    pub before_sha256: String,
    pub before_bytes: u64,
    #[serde(skip)]
    pub after_bytes: Vec<u8>,
    pub after_sha256: String,
    pub matches: u64,
    pub reason: String,
    /// Manager-native dependency-graph proof carried by a lockfile rewrite.
    /// It is projected separately from the rewrite tagged union on the wire.
    #[serde(skip)]
    pub native_lock_change: Option<NativeLockChange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NativeLockChange {
    pub manager: String,
    pub path: String,
    pub before_sha256: String,
    pub after_sha256: String,
    pub before_graph: Vec<String>,
    pub after_graph: Vec<String>,
    pub removed: Vec<String>,
    pub authorizing_rewrite_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ByteSpan {
    pub start: u64,
    pub end: u64,
    pub node: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Blocker {
    pub code: String,
    pub path: Option<String>,
    pub rule_id: Option<String>,
    pub message: String,
}

impl Blocker {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            path: None,
            rule_id: None,
            message: message.into(),
        }
    }
    pub fn at(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }
    pub fn rule(mut self, id: impl Into<String>) -> Self {
        self.rule_id = Some(id.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScrapePlan {
    pub schema: u32,
    pub command: String,
    pub mode: String,
    pub plan_id: String,
    pub tree_digest: String,
    pub contract_sha256: String,
    pub items: Vec<PlanItem>,
    pub rewrites: Vec<PreparedRewrite>,
    pub relocations: Vec<PlannedRelocation>,
    pub native_lock_changes: Vec<NativeLockChange>,
    pub assertions: Vec<String>,
    pub healthchecks: Vec<String>,
    pub contract_boundary: ContractBoundary,
    pub blockers: Vec<Blocker>,
    pub summary: PlanSummary,
    #[serde(skip)]
    pub prepared_health: crate::health::PreparedHealth,
    #[serde(skip)]
    pub project_display_root: String,
    #[serde(skip)]
    pub contract_display_path: String,
    #[serde(skip)]
    pub contract_contained: bool,
    #[serde(skip)]
    pub contract_action: ContractAction,
    #[serde(skip)]
    pub contract_value: Contract,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanItem {
    pub path: String,
    pub entry_kind: EntryKind,
    pub disposition: Disposition,
    pub class: FileClass,
    pub proof: Option<String>,
    pub modification: ModificationState,
    pub owner: Owner,
    pub sha256: Option<String>,
    pub bytes: Option<u64>,
    pub unix_mode: Option<u32>,
    pub rule_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Disposition {
    Keep,
    Rewrite,
    Relocate,
    Delete,
    DeleteLast,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FileClass {
    GeneratedOwned,
    ManagedRegion,
    AuthoredMetadata,
    AuthoredProduct,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModificationState {
    Unmodified,
    Modified,
    Unknown,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlannedRelocation {
    pub id: String,
    pub from: String,
    pub to: String,
    pub required: bool,
    /// Exact source-to-destination projection, including every descendant.
    /// Later projected-final validation consumes this rather than re-expanding
    /// the contract row against a changed tree.
    pub mapped_descendants: Vec<MappedRelocation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MappedRelocation {
    pub from: String,
    pub to: String,
    pub entry_kind: EntryKind,
    pub sha256: Option<String>,
    pub bytes: Option<u64>,
    pub unix_mode: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ContractBoundary {
    DeleteLast {
        path: String,
        empty_ancestors: Vec<String>,
    },
    Preserve,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct PlanSummary {
    pub keep: u64,
    pub rewrite: u64,
    pub relocate: u64,
    pub delete_unmodified: u64,
    pub delete_modified: u64,
    pub delete_unknown: u64,
    pub delete_last: u64,
}

#[derive(Debug, Clone)]
pub struct PreparedScrape {
    pub contract: ContractSnapshot,
    pub inventory: Inventory,
    pub rewrites: Vec<PreparedRewrite>,
    pub health: crate::health::PreparedHealth,
    pub plan: ScrapePlan,
    pub mode: ScrapeMode,
}

#[derive(Debug, thiserror::Error)]
pub enum ScrapeError {
    #[error("{0}")]
    Request(String),
    #[error("{0}")]
    Contract(String),
    #[error("{0}")]
    Inventory(String),
    #[error("{0}")]
    Rewrite(String),
    #[error("{0}")]
    Blocked(String),
    #[error("{0}")]
    Io(String),
}

impl ScrapeError {
    pub fn request(message: impl Into<String>) -> Self {
        Self::Request(message.into())
    }
    pub fn contract(message: impl Into<String>) -> Self {
        Self::Contract(message.into())
    }
    pub fn inventory(message: impl Into<String>) -> Self {
        Self::Inventory(message.into())
    }
    pub fn rewrite(message: impl Into<String>) -> Self {
        Self::Rewrite(message.into())
    }
    pub fn blocked(message: impl Into<String>) -> Self {
        Self::Blocked(message.into())
    }
    pub fn io(message: impl Into<String>) -> Self {
        Self::Io(message.into())
    }
}

fn u32_count(value: u64) -> Result<u32, ScrapeError> {
    u32::try_from(value).map_err(|_| ScrapeError::contract("plan count exceeds wire u32"))
}

fn relocation_evidence(row: &PlannedRelocation) -> Result<(String, u64, u32), ScrapeError> {
    if row.mapped_descendants.len() == 1 {
        let entry = &row.mapped_descendants[0];
        if entry.entry_kind == EntryKind::File {
            let sha256 = entry.sha256.clone().ok_or_else(|| {
                ScrapeError::inventory(format!(
                    "relocation `{}` file `{}` has no inventoried digest",
                    row.id, entry.from
                ))
            })?;
            let bytes = entry.bytes.ok_or_else(|| {
                ScrapeError::inventory(format!(
                    "relocation `{}` file `{}` has no inventoried size",
                    row.id, entry.from
                ))
            })?;
            return Ok((sha256, bytes, entry.unix_mode.unwrap_or(0)));
        }
    }

    let mut hash = Sha256::new();
    hash.update(b"vibe-scrape-relocation-tree-e1\0");
    let mut total = 0_u64;
    let mut root_mode = 0_u32;
    for entry in &row.mapped_descendants {
        let suffix = entry.from.strip_prefix(&row.from).ok_or_else(|| {
            ScrapeError::inventory(format!(
                "relocation `{}` mapped member `{}` is outside source `{}`",
                row.id, entry.from, row.from
            ))
        })?;
        hash.update(match entry.entry_kind {
            EntryKind::File => b"f\0".as_slice(),
            EntryKind::Directory => b"d\0".as_slice(),
        });
        hash.update(suffix.as_bytes());
        hash.update(b"\0");
        if let Some(digest) = &entry.sha256 {
            hash.update(digest.as_bytes());
        }
        hash.update(b"\0");
        if let Some(bytes) = entry.bytes {
            total = total.checked_add(bytes).ok_or_else(|| {
                ScrapeError::inventory(format!(
                    "relocation `{}` total byte count overflows u64",
                    row.id
                ))
            })?;
            hash.update(bytes.to_be_bytes());
        }
        hash.update(b"\0");
        if let Some(mode) = entry.unix_mode {
            hash.update(mode.to_be_bytes());
            if entry.from == row.from {
                root_mode = mode;
            }
        }
        hash.update(b"\n");
    }
    Ok((format!("sha256:{:x}", hash.finalize()), total, root_mode))
}
