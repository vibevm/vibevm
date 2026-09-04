//! Domain model shared by planning and typed rewrite adapters.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-A");

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::contract::{Contract, ContractAction, Owner};

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

impl ScrapePlan {
    /// Convert the validated domain plan to the generated epoch-1 wire type.
    /// JSON callers must serialize this value, never the internal model.
    pub fn to_wire(&self) -> Result<vibe_wire::generated::scrape::e1::plan::Plan, ScrapeError> {
        use vibe_wire::generated::scrape::e1::plan as w;
        let items = self
            .items
            .iter()
            .map(wire_item)
            .collect::<Result<Vec<_>, ScrapeError>>()?;
        let rewrites = self
            .rewrites
            .iter()
            .map(|prepared| wire_rewrite(prepared, &self.contract_value))
            .collect::<Result<Vec<_>, _>>()?;
        let assertions = self
            .contract_value
            .assertions
            .iter()
            .map(wire_assertion)
            .collect();
        let healthchecks = crate::health::to_wire_checks(&self.prepared_health)
            .map_err(|error| ScrapeError::blocked(error.to_string()))?;
        let relocations = self
            .relocations
            .iter()
            .map(|row| {
                let (sha256, bytes, mode) = relocation_evidence(row)?;
                Ok(w::Relocation {
                    id: row.id.clone(),
                    from: row.from.clone(),
                    to: row.to.clone(),
                    bytes: bytes.to_string(),
                    sha256,
                    mode,
                })
            })
            .collect::<Result<Vec<_>, ScrapeError>>()?;
        let native_lock_changes = self
            .native_lock_changes
            .iter()
            .map(|change| {
                let manager = match change.manager.as_str() {
                    "cargo" => w::LockManager::Cargo,
                    "npm" => w::LockManager::Npm,
                    "pnpm" => w::LockManager::Pnpm,
                    "yarn" => w::LockManager::Yarn,
                    "go" => w::LockManager::Go,
                    other => {
                        return Err(ScrapeError::blocked(format!(
                            "unknown prepared native lock manager `{other}`"
                        )));
                    }
                };
                Ok(w::NativeLockChange {
                    after_graph: change.after_graph.clone(),
                    after_sha256: change.after_sha256.clone(),
                    authorizing_rewrite_id: change.authorizing_rewrite_id.clone(),
                    before_graph: change.before_graph.clone(),
                    before_sha256: change.before_sha256.clone(),
                    manager,
                    path: change.path.clone(),
                    removed: change.removed.clone(),
                })
            })
            .collect::<Result<Vec<_>, ScrapeError>>()?;
        Ok(w::Plan {
            assertions,
            blockers: self
                .blockers
                .iter()
                .map(|blocker| w::Blocker {
                    code: blocker.code.clone(),
                    message: blocker.message.clone(),
                    path: blocker.path.clone(),
                })
                .collect(),
            command: w::Command::Scrape,
            contract: w::ContractIdentity {
                action: match self.contract_action {
                    ContractAction::DeleteLast => w::ContractAction::DeleteLast,
                    ContractAction::Preserve => w::ContractAction::Preserve,
                },
                contained: self.contract_contained,
                display_path: self.contract_display_path.clone(),
                sha256: self.contract_sha256.clone(),
            },
            contract_boundary: match &self.contract_boundary {
                ContractBoundary::DeleteLast {
                    path,
                    empty_ancestors,
                } => w::ContractBoundary::DeleteLast(Box::new(w::ContractBoundaryDeleteLast {
                    path: path.clone(),
                    empty_ancestors: empty_ancestors.clone(),
                })),
                ContractBoundary::Preserve => {
                    w::ContractBoundary::Preserve(Box::new(w::ContractBoundaryPreserve {}))
                }
            },
            health_baseline: crate::health::wire_baseline(self.prepared_health.baseline),
            health_limits: crate::health::wire_limits(&self.prepared_health)
                .map_err(|error| ScrapeError::blocked(error.to_string()))?,
            health_plan_id: self.prepared_health.plan_id.clone(),
            healthchecks,
            items,
            mode: if self.mode == "export" {
                w::Mode::Export
            } else {
                w::Mode::InPlace
            },
            native_lock_changes,
            plan_id: self.plan_id.clone(),
            project: w::ProjectIdentity {
                display_root: self.project_display_root.clone(),
                tree_digest: self.tree_digest.clone(),
            },
            relocations,
            rewrites,
            schema: 1,
            summary: w::Summary {
                keep: u32_count(self.summary.keep)?,
                rewrite: u32_count(self.summary.rewrite)?,
                relocate: u32_count(self.summary.relocate)?,
                delete_unmodified: u32_count(self.summary.delete_unmodified)?,
                delete_modified: u32_count(self.summary.delete_modified)?,
                delete_unknown: u32_count(self.summary.delete_unknown)?,
                delete_last: u32_count(self.summary.delete_last)?,
            },
        })
    }
}

fn wire_rewrite(
    prepared: &PreparedRewrite,
    contract: &Contract,
) -> Result<vibe_wire::generated::scrape::e1::plan::Rewrite, ScrapeError> {
    use crate::contract::RewriteRule as R;
    use vibe_wire::generated::scrape::e1::plan as w;
    let rule = contract
        .rewrite
        .iter()
        .find(|rule| rule.id() == prepared.id)
        .ok_or_else(|| {
            ScrapeError::rewrite(format!(
                "prepared rewrite `{}` has no contract row",
                prepared.id
            ))
        })?;
    let spans = || {
        prepared
            .spans
            .iter()
            .map(|span| {
                Ok(w::Span {
                    start: u32::try_from(span.start)
                        .map_err(|_| ScrapeError::rewrite("rewrite span exceeds u32"))?,
                    end: u32::try_from(span.end)
                        .map_err(|_| ScrapeError::rewrite("rewrite span exceeds u32"))?,
                    node: span.node.clone(),
                })
            })
            .collect::<Result<Vec<_>, ScrapeError>>()
    };
    macro_rules! common {
        () => {
            (
                prepared.adapter_epoch,
                prepared.after_bytes.len().to_string(),
                prepared.after_sha256.clone(),
                prepared.before_sha256.clone(),
                prepared.id.clone(),
                prepared.path.clone(),
                prepared.reason.clone(),
                spans()?,
            )
        };
    }
    Ok(match rule {
        R::ManagedBlockRemoveV1 {
            marker, matches, ..
        } => {
            let (adapter_epoch, after_bytes, after_sha256, before_sha256, id, path, reason, spans) =
                common!();
            w::Rewrite::ManagedBlockRemoveV1(Box::new(w::RewriteManagedBlockRemoveV1 {
                adapter_epoch,
                after_bytes,
                after_sha256,
                before_sha256,
                id,
                marker: marker.clone(),
                matches: per_file_matches(*matches),
                path,
                reason,
                spans,
            }))
        }
        R::RustSpecmarkStripV1 { forms, matches, .. } => {
            let (adapter_epoch, after_bytes, after_sha256, before_sha256, id, path, reason, spans) =
                common!();
            w::Rewrite::RustSpecmarkStripV1(Box::new(w::RewriteRustSpecmarkStripV1 {
                adapter_epoch,
                after_bytes,
                after_sha256,
                before_sha256,
                forms: forms.iter().copied().map(wire_rust_form).collect(),
                id,
                matches: set_matches(*matches),
                path,
                reason,
                spans,
            }))
        }
        R::CargoPackageRemoveV1 {
            package,
            aliases,
            matches,
            ..
        } => {
            let (adapter_epoch, after_bytes, after_sha256, before_sha256, id, path, reason, spans) =
                common!();
            w::Rewrite::CargoPackageRemoveV1(Box::new(w::RewriteCargoPackageRemoveV1 {
                adapter_epoch,
                after_bytes,
                after_sha256,
                aliases: aliases.clone(),
                before_sha256,
                id,
                matches: set_matches(*matches),
                package: package.clone(),
                path,
                reason,
                spans,
            }))
        }
        R::NodePackageRemoveV1 {
            manager,
            packages,
            matches,
            ..
        } => {
            let (adapter_epoch, after_bytes, after_sha256, before_sha256, id, path, reason, spans) =
                common!();
            w::Rewrite::NodePackageRemoveV1(Box::new(w::RewriteNodePackageRemoveV1 {
                adapter_epoch,
                after_bytes,
                after_sha256,
                before_sha256,
                id,
                manager: match manager {
                    crate::contract::NodeManager::Npm => w::RewriteNodePackageRemoveV1Manager::Npm,
                    crate::contract::NodeManager::Pnpm => {
                        w::RewriteNodePackageRemoveV1Manager::Pnpm
                    }
                    crate::contract::NodeManager::Yarn => {
                        w::RewriteNodePackageRemoveV1Manager::Yarn
                    }
                },
                matches: set_matches(*matches),
                packages: packages.clone(),
                path,
                reason,
                spans,
            }))
        }
        R::GoModuleRemoveV1 {
            modules, matches, ..
        } => {
            let (adapter_epoch, after_bytes, after_sha256, before_sha256, id, path, reason, spans) =
                common!();
            w::Rewrite::GoModuleRemoveV1(Box::new(w::RewriteGoModuleRemoveV1 {
                adapter_epoch,
                after_bytes,
                after_sha256,
                before_sha256,
                id,
                matches: set_matches(*matches),
                modules: modules.clone(),
                path,
                reason,
                spans,
            }))
        }
        R::TomlArrayValuesRemoveV1 {
            table,
            key,
            values,
            matches,
            ..
        } => {
            let (adapter_epoch, after_bytes, after_sha256, before_sha256, id, path, reason, spans) =
                common!();
            w::Rewrite::TomlArrayValuesRemoveV1(Box::new(w::RewriteTomlArrayValuesRemoveV1 {
                adapter_epoch,
                after_bytes,
                after_sha256,
                before_sha256,
                id,
                key: key.clone(),
                matches: set_matches(*matches),
                path,
                reason,
                spans,
                table: table.clone(),
                values: values.clone(),
            }))
        }
        R::TypeScriptSpecCommentsStripV1 { matches, .. } => {
            let (adapter_epoch, after_bytes, after_sha256, before_sha256, id, path, reason, spans) =
                common!();
            w::Rewrite::TypescriptSpecCommentsStripV1(Box::new(
                w::RewriteTypescriptSpecCommentsStripV1 {
                    adapter_epoch,
                    after_bytes,
                    after_sha256,
                    before_sha256,
                    id,
                    matches: set_matches(*matches),
                    path,
                    reason,
                    spans,
                },
            ))
        }
        R::GoSpecDirectivesStripV1 { matches, .. } => {
            let (adapter_epoch, after_bytes, after_sha256, before_sha256, id, path, reason, spans) =
                common!();
            w::Rewrite::GoSpecDirectivesStripV1(Box::new(w::RewriteGoSpecDirectivesStripV1 {
                adapter_epoch,
                after_bytes,
                after_sha256,
                before_sha256,
                id,
                matches: set_matches(*matches),
                path,
                reason,
                spans,
            }))
        }
        R::JsonMemberRemoveV1 {
            object,
            members,
            matches,
            ..
        } => {
            let (adapter_epoch, after_bytes, after_sha256, before_sha256, id, path, reason, spans) =
                common!();
            w::Rewrite::JsonMemberRemoveV1(Box::new(w::RewriteJsonMemberRemoveV1 {
                adapter_epoch,
                after_bytes,
                after_sha256,
                before_sha256,
                id,
                matches: set_matches(*matches),
                members: members.clone(),
                object: object.clone(),
                path,
                reason,
                spans,
            }))
        }
        R::TextExactReplaceV1 { occurrences, .. } => {
            let (adapter_epoch, after_bytes, after_sha256, before_sha256, id, path, reason, spans) =
                common!();
            w::Rewrite::TextExactReplaceV1(Box::new(w::RewriteTextExactReplaceV1 {
                adapter_epoch,
                after_bytes,
                after_sha256,
                before_sha256,
                id,
                matches: u32_count(prepared.matches)?,
                occurrences: u32_count(*occurrences)?,
                path,
                reason,
                spans,
            }))
        }
    })
}

fn wire_assertion(
    value: &crate::contract::Assertion,
) -> vibe_wire::generated::scrape::e1::plan::Assertion {
    use crate::contract::Assertion as A;
    use vibe_wire::generated::scrape::e1::plan as w;
    match value {
        A::PathsAbsentV1 { id, patterns } => {
            w::Assertion::PathsAbsentV1(Box::new(w::AssertionPathsAbsentV1 {
                id: id.clone(),
                patterns: patterns.clone(),
            }))
        }
        A::TextLiteralAbsentV1 {
            id,
            patterns,
            needles,
        } => w::Assertion::TextLiteralAbsentV1(Box::new(w::AssertionTextLiteralAbsentV1 {
            id: id.clone(),
            patterns: patterns.clone(),
            needles: needles.clone(),
        })),
        A::CargoPathPrefixAbsentV1 {
            id,
            manifests,
            prefixes,
        } => w::Assertion::CargoPathPrefixAbsentV1(Box::new(w::AssertionCargoPathPrefixAbsentV1 {
            id: id.clone(),
            manifests: manifests.clone(),
            prefixes: prefixes.clone(),
        })),
        A::LanguageMetadataAbsentV1 {
            id,
            language,
            patterns,
        } => {
            w::Assertion::LanguageMetadataAbsentV1(Box::new(w::AssertionLanguageMetadataAbsentV1 {
                id: id.clone(),
                language: match language {
                    crate::contract::Language::Rust => {
                        w::AssertionLanguageMetadataAbsentV1Language::Rust
                    }
                    crate::contract::Language::TypeScript => {
                        w::AssertionLanguageMetadataAbsentV1Language::Typescript
                    }
                    crate::contract::Language::Go => {
                        w::AssertionLanguageMetadataAbsentV1Language::Go
                    }
                },
                patterns: patterns.clone(),
            }))
        }
        A::DependencyIdentitiesAbsentV1 {
            id,
            manager,
            manifests,
            identities,
        } => w::Assertion::DependencyIdentitiesAbsentV1(Box::new(
            w::AssertionDependencyIdentitiesAbsentV1 {
                id: id.clone(),
                manager: wire_lock_manager(*manager),
                manifests: manifests.clone(),
                identities: identities.clone(),
            },
        )),
    }
}

fn wire_item(item: &PlanItem) -> Result<vibe_wire::generated::scrape::e1::plan::Item, ScrapeError> {
    use vibe_wire::generated::scrape::e1::plan as w;
    let bytes = item.bytes.map(|value| value.to_string());
    let class = wire_class(item.class);
    let entry_kind = match item.entry_kind {
        EntryKind::File => w::EntryKind::File,
        EntryKind::Directory => w::EntryKind::Directory,
    };
    let modification = wire_modification(item.modification);
    let owner = match item.owner {
        Owner::Project => w::Owner::Project,
        Owner::Vibe => w::Owner::Vibe,
    };
    let path = item.path.clone();
    let proof = item.proof.as_deref().map(wire_proof).transpose()?;
    let rule_ids = item.rule_ids.clone();
    let sha256 = item.sha256.clone();
    macro_rules! payload {
        ($type:ident) => {
            Box::new(w::$type {
                bytes,
                class,
                entry_kind,
                modification,
                owner,
                path,
                proof,
                rule_ids,
                sha256,
            })
        };
    }
    Ok(match item.disposition {
        Disposition::Keep => w::Item::Keep(Box::new(w::ItemKeep {
            bytes,
            class,
            entry_kind,
            modification,
            owner,
            path,
            rule_ids,
            sha256,
        })),
        Disposition::Rewrite => w::Item::Rewrite(payload!(ItemRewrite)),
        Disposition::Relocate => w::Item::Relocate(payload!(ItemRelocate)),
        Disposition::DeleteLast => w::Item::DeleteLast(payload!(ItemDeleteLast)),
        Disposition::Delete => match item.modification {
            ModificationState::Modified => w::Item::DeleteModified(payload!(ItemDeleteModified)),
            ModificationState::Unknown => w::Item::DeleteUnknown(payload!(ItemDeleteUnknown)),
            _ => w::Item::DeleteUnmodified(payload!(ItemDeleteUnmodified)),
        },
    })
}

fn wire_lock_manager(
    value: crate::contract::DependencyManager,
) -> vibe_wire::generated::scrape::e1::plan::LockManager {
    use crate::contract::DependencyManager as D;
    use vibe_wire::generated::scrape::e1::plan::LockManager as W;
    match value {
        D::Cargo => W::Cargo,
        D::Npm => W::Npm,
        D::Pnpm => W::Pnpm,
        D::Yarn => W::Yarn,
        D::Go => W::Go,
    }
}
fn wire_class(value: FileClass) -> vibe_wire::generated::scrape::e1::plan::FileClass {
    use vibe_wire::generated::scrape::e1::plan::FileClass as W;
    match value {
        FileClass::GeneratedOwned => W::GeneratedOwned,
        FileClass::ManagedRegion => W::ManagedRegion,
        FileClass::AuthoredMetadata => W::AuthoredMetadata,
        FileClass::AuthoredProduct => W::AuthoredProduct,
        FileClass::Unknown => W::Unknown,
    }
}
fn wire_modification(
    value: ModificationState,
) -> vibe_wire::generated::scrape::e1::plan::Modification {
    use vibe_wire::generated::scrape::e1::plan::Modification as W;
    match value {
        ModificationState::Unmodified => W::Unmodified,
        ModificationState::Modified => W::Modified,
        ModificationState::Unknown => W::Unknown,
        ModificationState::NotApplicable => W::NotApplicable,
    }
}
fn wire_proof(value: &str) -> Result<vibe_wire::generated::scrape::e1::plan::Proof, ScrapeError> {
    use vibe_wire::generated::scrape::e1::plan::Proof as W;
    match value {
        "contract-assertion-v1" => Ok(W::ContractAssertionV1),
        "sha256-v1" => Ok(W::Sha256V1),
        "vibe-generated-v1" => Ok(W::VibeGeneratedV1),
        _ => Err(ScrapeError::contract(format!(
            "unknown proof `{value}` in prepared plan"
        ))),
    }
}
fn set_matches(
    value: crate::contract::SetMatches,
) -> vibe_wire::generated::scrape::e1::plan::SetMatches {
    use vibe_wire::generated::scrape::e1::plan::SetMatches as W;
    match value {
        crate::contract::SetMatches::ZeroOrMore => W::ZeroOrMore,
        crate::contract::SetMatches::OneOrMore => W::OneOrMore,
        crate::contract::SetMatches::ExactlyOne => W::ExactlyOne,
    }
}
fn per_file_matches(
    value: crate::contract::PerFileMatches,
) -> vibe_wire::generated::scrape::e1::plan::PerFileMatches {
    use vibe_wire::generated::scrape::e1::plan::PerFileMatches as W;
    match value {
        crate::contract::PerFileMatches::ZeroOrOnePerFile => W::ZeroOrOnePerFile,
        crate::contract::PerFileMatches::ExactlyOnePerFile => W::ExactlyOnePerFile,
    }
}
fn wire_rust_form(
    value: crate::contract::RustForm,
) -> vibe_wire::generated::scrape::e1::plan::RustForm {
    use vibe_wire::generated::scrape::e1::plan::RustForm as W;
    match value {
        crate::contract::RustForm::Scope => W::Scope,
        crate::contract::RustForm::Spec => W::Spec,
        crate::contract::RustForm::Verifies => W::Verifies,
        crate::contract::RustForm::Cell => W::Cell,
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
