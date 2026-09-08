//! Read-only preparation of the schema-1 scrape rewrite algebra.
//!
//! This module never writes the inspected project.  Each adapter computes exact
//! after-bytes and the caller later applies them under the transaction's
//! before-digest precondition.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-B");

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};
use vibe_safefs::Project;

use crate::contract::{
    Assertion, Contract, DependencyManager, Language, NodeManager, PerFileMatches, RewriteRule,
    RustForm, SetMatches,
};
use crate::glob::Glob;
use crate::model::{
    Blocker, ByteSpan, EntryKind, Inventory, InventoryEntry, NativeLockChange, PreparedRewrite,
    ScrapeError,
};

#[path = "rewrite/cargo/edit.rs"]
mod cargo_edit;
#[path = "rewrite/cargo/lock.rs"]
mod cargo_lock;
#[path = "rewrite/cargo/topology.rs"]
mod cargo_topology;
#[path = "rewrite/go_node.rs"]
mod go_node;
#[path = "rewrite/javascript.rs"]
mod javascript;
#[path = "rewrite/projected.rs"]
mod projected;
#[path = "rewrite/rust/ast.rs"]
mod rust_ast;
#[path = "rewrite/rust/imports.rs"]
mod rust_imports;
#[path = "rewrite/rust/lexer.rs"]
mod rust_lexer;
#[path = "rewrite/text.rs"]
mod text;

#[cfg(test)]
use cargo_edit::prepare_cargo;
use cargo_edit::prepare_cargo_resolved;
use cargo_lock::prepare_cargo_lock;
use cargo_topology::{cargo_topology_from_current, observed_specmark_aliases_for_source};
use go_node::{
    candidate, cargo_lock_candidate, prepare_go_directives, prepare_go_mod, prepare_go_sum,
    prepare_node_lock, prepare_node_manifest, prepare_record, validate_relocations, virtual_bytes,
};
#[cfg(test)]
use go_node::{cargo_path_prefix_present, go_module_on_line};
use javascript::{prepare_json_members, prepare_typescript};
use rust_ast::prepare_rust;
use text::{
    check_per_file_cardinality, check_set_cardinality, digest, fail, inventory_files,
    prepare_exact_text, prepare_managed, prepare_toml_array, read_candidate, selected_paths,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewritePreparation {
    pub rewrites: Vec<PreparedRewrite>,
    pub blockers: Vec<Blocker>,
}

#[derive(Debug, Clone)]
struct Edit {
    start: usize,
    end: usize,
    replacement: Vec<u8>,
}

#[derive(Debug)]
struct Candidate {
    path: String,
    before: Vec<u8>,
    after: Vec<u8>,
    matches: usize,
    spans: Vec<ByteSpan>,
    native_lock_evidence: Option<NativeLockEvidence>,
}

#[derive(Debug)]
struct NativeLockEvidence {
    manager: &'static str,
    before_graph: Vec<String>,
    after_graph: Vec<String>,
    removed: Vec<String>,
}

type RewriteOutput = (Vec<u8>, usize, Vec<String>, Vec<ByteSpan>);
type CargoOutput = (Vec<u8>, usize, Vec<String>, BTreeSet<String>, Vec<ByteSpan>);
type RustImportOutput = (Vec<Edit>, BTreeSet<String>, BTreeSet<String>);
type JsonEditOutput = (Vec<Edit>, usize, Vec<String>, Vec<ByteSpan>);

/// One exact entry in the actual projected final tree. Core constructs this
/// only after dispositions, rewrites and relocation destinations are resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedEntry {
    pub path: String,
    pub kind: EntryKind,
    pub bytes: Option<Vec<u8>>,
    pub unix_mode: Option<u32>,
}

/// Validate the actual projected final inventory. No classify rule is used as
/// a proxy for a disposition: callers pass the real kept/rewritten/relocated
/// paths, including directories and mapped relocation destinations.
pub fn validate_projected_final(
    contract: &Contract,
    projected_entries: &[ProjectedEntry],
) -> Result<(), ScrapeError> {
    projected::validate_projected_final(contract, projected_entries)
}

/// Prepare every schema-1 rewrite without mutating `root`.
///
/// Rules are evaluated and returned in fixed adapter-kind, rule-id, byte-sorted
/// path order. A later rule that targets the same file sees the already prepared
/// bytes and therefore has an exact transactional preimage.
pub trait InventoryView {
    fn entries(&self) -> &[InventoryEntry];
}

impl InventoryView for Inventory {
    fn entries(&self) -> &[InventoryEntry] {
        &self.entries
    }
}

impl InventoryView for [InventoryEntry] {
    fn entries(&self) -> &[InventoryEntry] {
        self
    }
}

impl InventoryView for Vec<InventoryEntry> {
    fn entries(&self) -> &[InventoryEntry] {
        self
    }
}

pub fn prepare_rewrites<I: InventoryView + ?Sized>(
    project: &Project,
    contract: &Contract,
    inventory: &I,
) -> Result<RewritePreparation, ScrapeError> {
    contract.validate()?;
    let inventory = inventory.entries();
    let inventory_by_path = inventory
        .iter()
        .map(|entry| (entry.path.clone(), entry))
        .collect::<BTreeMap<_, _>>();
    validate_relocations(contract, inventory)?;
    let files = inventory_files(inventory);
    let mut current = BTreeMap::<String, Vec<u8>>::new();
    let mut records = Vec::new();
    let mut blockers: Vec<Blocker> = Vec::new();
    for entry in inventory
        .iter()
        .filter(|entry| entry.kind == EntryKind::File)
    {
        let bytes = read_candidate(project, entry)?;
        let observed_digest = digest(&bytes);
        if entry.sha256.as_deref() != Some(observed_digest.as_str()) {
            return Err(fail(format!(
                "rewrite preparation observed concurrent change at `{}`",
                entry.path
            )));
        }
        current.insert(entry.path.clone(), bytes);
    }
    let mut rules = contract.rewrite.iter().collect::<Vec<_>>();
    rules.sort_by_key(|rule| {
        let priority = match rule {
            RewriteRule::ManagedBlockRemoveV1 { .. } => 0,
            RewriteRule::RustSpecmarkStripV1 { .. }
            | RewriteRule::TypeScriptSpecCommentsStripV1 { .. }
            | RewriteRule::GoSpecDirectivesStripV1 { .. } => 1,
            RewriteRule::CargoPackageRemoveV1 { .. }
            | RewriteRule::TomlArrayValuesRemoveV1 { .. }
            | RewriteRule::JsonMemberRemoveV1 { .. } => 2,
            RewriteRule::NodePackageRemoveV1 { .. } | RewriteRule::GoModuleRemoveV1 { .. } => 3,
            RewriteRule::TextExactReplaceV1 { .. } => 4,
        };
        (priority, rule.id())
    });

    for rule in rules {
        let mut candidates = Vec::new();
        match rule {
            RewriteRule::ManagedBlockRemoveV1 {
                id,
                paths,
                marker,
                matches,
            } => {
                for path in paths {
                    if !files.contains(path) {
                        if *matches == PerFileMatches::ExactlyOnePerFile {
                            return Err(fail(format!("rewrite `{id}` target `{path}` is absent")));
                        }
                        continue;
                    }
                    let before = virtual_bytes(project, path, &inventory_by_path, &current)?;
                    let expected = contract
                        .baseline
                        .iter()
                        .find(|baseline| baseline.path == *path)
                        .ok_or_else(|| {
                            ScrapeError::blocked(format!(
                                "managed block `{id}` at `{path}` requires an exact whole-target baseline digest proving provider ownership"
                            ))
                        })?;
                    if expected.sha256 != digest(&before) {
                        return Err(ScrapeError::blocked(format!(
                            "managed block `{id}` at `{path}` differs from its ownership baseline"
                        )));
                    }
                    let (after, count, nodes, spans) = prepare_managed(&before, marker)?;
                    check_per_file_cardinality(id, *matches, path, count)?;
                    candidates.push(candidate(
                        path.clone(),
                        before.clone(),
                        after,
                        count,
                        nodes,
                        spans,
                    ));
                }
            }
            RewriteRule::RustSpecmarkStripV1 {
                id,
                patterns,
                exclude,
                forms,
                matches,
            } => {
                let topology = cargo_topology_from_current(&current, &files)?;
                let forms = forms
                    .iter()
                    .map(|form| {
                        match form {
                            RustForm::Scope => "scope",
                            RustForm::Spec => "spec",
                            RustForm::Verifies => "verifies",
                            RustForm::Cell => "cell",
                        }
                        .to_owned()
                    })
                    .collect::<BTreeSet<_>>();
                let paths = selected_paths(&files, patterns, exclude)?;
                let mut total = 0;
                let mut authority_blocked = false;
                for path in paths {
                    let specmark_aliases =
                        match observed_specmark_aliases_for_source(contract, &topology, &path) {
                            Ok(aliases) => aliases,
                            Err(ScrapeError::Blocked(message)) => {
                                blockers.push(
                                    Blocker::new("rust-cargo-ownership-unresolved", message)
                                        .at(&path)
                                        .rule(id),
                                );
                                authority_blocked = true;
                                continue;
                            }
                            Err(error) => return Err(error),
                        };
                    let before = virtual_bytes(project, &path, &inventory_by_path, &current)?;
                    let (after, count, nodes, spans) =
                        prepare_rust(&before, &specmark_aliases, &forms)?;
                    total += count;
                    candidates.push(candidate(path, before, after, count, nodes, spans));
                }
                if !authority_blocked {
                    check_set_cardinality(id, *matches, total)?;
                }
            }
            RewriteRule::CargoPackageRemoveV1 {
                id,
                manifests,
                package,
                aliases,
                matches,
            } => {
                let topology = cargo_topology_from_current(&current, &files)?;
                let paths = selected_paths(&files, manifests, &[])?;
                let mut owned_locks = BTreeSet::new();
                let mut total = 0;
                let mut authority_blocked = false;
                for path in &paths {
                    let before = virtual_bytes(project, path, &inventory_by_path, &current)?;
                    let workspace_aliases = match topology.workspace_aliases_for(path) {
                        Ok(aliases) => aliases,
                        Err(ScrapeError::Blocked(message)) => {
                            blockers.push(
                                Blocker::new("cargo-ownership-ambiguous", message)
                                    .at(path)
                                    .rule(id),
                            );
                            authority_blocked = true;
                            continue;
                        }
                        Err(error) => return Err(error),
                    };
                    if let Some(lock) = topology.owned_lock_for(path)? {
                        owned_locks.insert(lock);
                    }
                    let (after, count, nodes, _, spans) =
                        prepare_cargo_resolved(&before, package, aliases, &workspace_aliases)?;
                    total += count;
                    candidates.push(candidate(
                        path.clone(),
                        before.clone(),
                        after,
                        count,
                        nodes,
                        spans,
                    ));
                }
                if !authority_blocked {
                    check_set_cardinality(id, *matches, total)?;
                }
                for lockfile in owned_locks {
                    let before = virtual_bytes(project, &lockfile, &inventory_by_path, &current)?;
                    match prepare_cargo_lock(&before, package) {
                        Ok(((after, lock_count, _nodes, spans), Some(evidence))) => {
                            candidates.push(cargo_lock_candidate(
                                lockfile.clone(),
                                before,
                                after,
                                lock_count,
                                spans,
                                evidence,
                            ));
                        }
                        Ok(((after, lock_count, nodes, spans), None)) => candidates.push(
                            candidate(lockfile.clone(), before, after, lock_count, nodes, spans),
                        ),
                        Err(ScrapeError::Blocked(message)) => blockers.push(
                            Blocker::new("native-lock-reconciliation-required", message)
                                .at(&lockfile)
                                .rule(id),
                        ),
                        Err(error) => return Err(error),
                    }
                }
            }
            RewriteRule::TomlArrayValuesRemoveV1 {
                id,
                path,
                table,
                key,
                values,
                matches,
            } => {
                if !files.contains(path) {
                    return Err(fail(format!("rewrite `{id}` target `{path}` is absent")));
                }
                let before = virtual_bytes(project, path, &inventory_by_path, &current)?;
                let (after, count, nodes, spans) = prepare_toml_array(&before, table, key, values)?;
                check_set_cardinality(id, *matches, count)?;
                candidates.push(candidate(
                    path.clone(),
                    before.clone(),
                    after,
                    count,
                    nodes,
                    spans,
                ));
            }
            RewriteRule::TypeScriptSpecCommentsStripV1 {
                id,
                patterns,
                exclude,
                matches,
            } => {
                let paths = selected_paths(&files, patterns, exclude)?;
                let mut total = 0;
                for path in paths {
                    let before = virtual_bytes(project, &path, &inventory_by_path, &current)?;
                    let (after, count, nodes, spans) =
                        prepare_typescript(&before, path.ends_with(".tsx"))?;
                    total += count;
                    candidates.push(candidate(path, before, after, count, nodes, spans));
                }
                check_set_cardinality(id, *matches, total)?;
            }
            RewriteRule::GoSpecDirectivesStripV1 {
                id,
                patterns,
                exclude,
                matches,
            } => {
                let paths = selected_paths(&files, patterns, exclude)?;
                let mut total = 0;
                for path in paths {
                    let before = virtual_bytes(project, &path, &inventory_by_path, &current)?;
                    let (after, count, nodes, spans) = prepare_go_directives(&before)?;
                    total += count;
                    candidates.push(candidate(path, before, after, count, nodes, spans));
                }
                check_set_cardinality(id, *matches, total)?;
            }
            RewriteRule::JsonMemberRemoveV1 {
                id,
                path,
                object,
                members,
                matches,
            } => {
                if !files.contains(path) {
                    return Err(fail(format!("rewrite `{id}` target `{path}` is absent")));
                }
                let before = virtual_bytes(project, path, &inventory_by_path, &current)?;
                let (after, count, nodes, spans) = prepare_json_members(&before, object, members)?;
                check_set_cardinality(id, *matches, count)?;
                candidates.push(candidate(path.clone(), before, after, count, nodes, spans));
            }
            RewriteRule::NodePackageRemoveV1 {
                id,
                package_json,
                lockfile,
                manager,
                packages,
                script_paths,
                config_paths,
                matches,
            } => {
                if !files.contains(package_json) || !files.contains(lockfile) {
                    return Err(fail(format!(
                        "rewrite `{id}` requires both package manifest and selected lockfile"
                    )));
                }
                let package_before =
                    virtual_bytes(project, package_json, &inventory_by_path, &current)?;
                let (package_after, package_count, package_nodes, package_spans) =
                    prepare_node_manifest(&package_before, packages, script_paths, config_paths)?;
                let lock_before = virtual_bytes(project, lockfile, &inventory_by_path, &current)?;
                let (lock_after, lock_count, lock_nodes, lock_spans) =
                    match prepare_node_lock(&lock_before, *manager, packages) {
                        Ok(output) => output,
                        Err(ScrapeError::Blocked(message)) => {
                            blockers.push(
                                Blocker::new("native-lock-reconciliation-required", message)
                                    .at(lockfile)
                                    .rule(id),
                            );
                            (lock_before.clone(), 0, Vec::new(), Vec::new())
                        }
                        Err(error) => return Err(error),
                    };
                let total = package_count + lock_count;
                check_set_cardinality(id, *matches, total)?;
                candidates.push(candidate(
                    package_json.clone(),
                    package_before,
                    package_after,
                    package_count,
                    package_nodes,
                    package_spans,
                ));
                candidates.push(candidate(
                    lockfile.clone(),
                    lock_before,
                    lock_after,
                    lock_count,
                    lock_nodes,
                    lock_spans,
                ));
            }
            RewriteRule::GoModuleRemoveV1 {
                id,
                go_mod,
                go_sum,
                modules,
                matches,
            } => {
                if !files.contains(go_mod) {
                    return Err(fail(format!("rewrite `{id}` target `{go_mod}` is absent")));
                }
                let before = virtual_bytes(project, go_mod, &inventory_by_path, &current)?;
                let (after, count, nodes, spans) = prepare_go_mod(&before, modules)?;
                let mut total = count;
                candidates.push(candidate(
                    go_mod.clone(),
                    before,
                    after,
                    count,
                    nodes,
                    spans,
                ));
                if let Some(path) = go_sum
                    && files.contains(path)
                {
                    let before = virtual_bytes(project, path, &inventory_by_path, &current)?;
                    match prepare_go_sum(&before, modules) {
                        Ok((after, count, nodes, spans)) => {
                            total += count;
                            candidates.push(candidate(
                                path.clone(),
                                before,
                                after,
                                count,
                                nodes,
                                spans,
                            ));
                        }
                        Err(ScrapeError::Blocked(message)) => blockers.push(
                            Blocker::new("native-lock-reconciliation-required", message)
                                .at(path)
                                .rule(id),
                        ),
                        Err(error) => return Err(error),
                    }
                }
                check_set_cardinality(id, *matches, total)?;
            }
            RewriteRule::TextExactReplaceV1 {
                id,
                path,
                sha256,
                before: needle,
                after: replacement,
                occurrences,
            } => {
                if !files.contains(path) {
                    return Err(fail(format!(
                        "exact-text rewrite target `{path}` is absent"
                    )));
                }
                let before = virtual_bytes(project, path, &inventory_by_path, &current)?;
                let occurrences = usize::try_from(*occurrences).map_err(|_| {
                    fail(format!(
                        "exact-text rewrite `{id}` occurrence count exceeds this platform's address space"
                    ))
                })?;
                let (after, count, nodes, spans) =
                    prepare_exact_text(&before, sha256, needle, replacement, occurrences)?;
                candidates.push(candidate(
                    path.clone(),
                    before.clone(),
                    after,
                    count,
                    nodes,
                    spans,
                ));
            }
        }
        for candidate in candidates {
            if candidate.before == candidate.after {
                continue;
            }
            current.insert(candidate.path.clone(), candidate.after.clone());
            records.push(prepare_record(rule.id(), rule.kind_name(), candidate));
        }
    }

    blockers.sort_by(|left, right| {
        (&left.code, &left.path, &left.rule_id, &left.message).cmp(&(
            &right.code,
            &right.path,
            &right.rule_id,
            &right.message,
        ))
    });
    blockers.dedup();
    Ok(RewritePreparation {
        rewrites: records,
        blockers,
    })
}

#[cfg(test)]
mod tests;
