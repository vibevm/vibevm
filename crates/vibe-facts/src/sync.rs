//! Asymmetric host-spec synchronization: the spec marker is authoritative.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-046#laws");

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use progress_core::doc::Severity;
use progress_core::model::Granularity;
use progress_core::parse::parse_document;

use crate::{FactOrigin, FactStatus, Registry, RegistryError, host_package};

#[derive(Debug, Clone, PartialEq, Eq)]
struct SpecFact {
    status: FactStatus,
    path: PathBuf,
    line: usize,
}

/// One host fact whose registry overlay differs from the authoritative spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncMismatch {
    pub address: String,
    pub spec_status: Option<FactStatus>,
    pub registry_status: Option<FactStatus>,
    pub path: Option<PathBuf>,
    pub line: Option<usize>,
}

impl SyncMismatch {
    pub fn spec_status_text(&self) -> String {
        self.spec_status
            .map(|status| status.to_string())
            .unwrap_or_else(|| "missing".to_string())
    }

    pub fn registry_status_text(&self) -> String {
        self.registry_status
            .map(|status| status.to_string())
            .unwrap_or_else(|| "indeterminate".to_string())
    }
}

/// Compare every present `origin = "spec"` record with its host marker.
/// Completeness is deliberately not checked: absent registry records are valid.
pub fn check(project_root: &Path, registry: &Registry) -> Result<Vec<SyncMismatch>, RegistryError> {
    let needed = needed_doc_paths(registry);
    if needed.is_empty() {
        return Ok(Vec::new());
    }
    let snapshot = load_spec_snapshot(project_root, &needed)?;
    Ok(compare(registry, &snapshot))
}

/// The canonical doc-paths the registry's host records actually cite —
/// the sync reads ONLY these files, so an unrelated spec file's marker
/// defect can never fail the L2 gate.
fn needed_doc_paths(registry: &Registry) -> BTreeSet<String> {
    registry
        .entries()
        .filter(|entry| entry.origin == FactOrigin::Spec)
        .filter_map(|entry| doc_path_of_address(&entry.address))
        .collect()
}

/// The `<doc>` segment of a full `spec://<group>/<name>/<doc>#<anchor>`
/// address. Malformed addresses yield `None` (store validation already
/// rejects them; the sync never widens its file set on garbage).
fn doc_path_of_address(address: &str) -> Option<String> {
    let rest = address.strip_prefix("spec://")?;
    let (path, _anchor) = rest.split_once('#')?;
    let mut parts = path.splitn(3, '/');
    let _group = parts.next()?;
    let _name = parts.next()?;
    parts.next().map(str::to_string)
}

/// Apply the L2 tie-break. Found host facts copy spec status into the
/// registry; registry records whose host anchor no longer exists are removed.
pub fn reconcile(
    project_root: &Path,
    registry: &mut Registry,
) -> Result<Vec<SyncMismatch>, RegistryError> {
    let needed = needed_doc_paths(registry);
    if needed.is_empty() {
        return Ok(Vec::new());
    }
    let snapshot = load_spec_snapshot(project_root, &needed)?;
    let mismatches = compare(registry, &snapshot);
    for mismatch in &mismatches {
        match mismatch.spec_status {
            Some(status) => {
                let mut entry = registry.get(&mismatch.address).cloned().ok_or_else(|| {
                    RegistryError::Invariant(format!(
                        "sync mismatch `{}` has no registry entry",
                        mismatch.address
                    ))
                })?;
                entry.status = Some(status);
                registry.upsert(project_root, entry)?;
            }
            None => {
                registry.remove(project_root, &mismatch.address)?;
            }
        }
    }
    Ok(mismatches)
}

fn compare(registry: &Registry, snapshot: &BTreeMap<String, SpecFact>) -> Vec<SyncMismatch> {
    registry
        .entries()
        .filter(|entry| entry.origin == FactOrigin::Spec)
        .filter_map(|entry| match snapshot.get(&entry.address) {
            Some(spec) if entry.status == Some(spec.status) => None,
            Some(spec) => Some(SyncMismatch {
                address: entry.address.clone(),
                spec_status: Some(spec.status),
                registry_status: entry.status,
                path: Some(spec.path.clone()),
                line: Some(spec.line),
            }),
            None => Some(SyncMismatch {
                address: entry.address.clone(),
                spec_status: None,
                registry_status: entry.status,
                path: None,
                line: None,
            }),
        })
        .collect()
}

fn load_spec_snapshot(
    project_root: &Path,
    needed: &BTreeSet<String>,
) -> Result<BTreeMap<String, SpecFact>, RegistryError> {
    let package = host_package(project_root)?;
    let spec_root = project_root.join("spec");
    let mut files = Vec::new();
    collect_markdown(&spec_root, &mut files)?;
    files.sort();

    let mut snapshot = BTreeMap::new();
    for file in files {
        let relative = file.strip_prefix(project_root).map_err(|_| {
            RegistryError::Invariant(format!(
                "spec file `{}` is outside project root `{}`",
                file.display(),
                project_root.display()
            ))
        })?;
        let relative_text = relative.to_string_lossy().replace('\\', "/");
        if !needed.contains(&vibe_spec::canonical_doc_path(&relative_text)) {
            continue;
        }
        // A spec source in either PROP-045 serialisation: `.md` verbatim,
        // `.xml` as its canonical Markdown projection — the scan below is
        // form-blind and addresses stay extensionless.
        let (text, _kind) =
            vibe_specdoc::load_spec_text(&file).map_err(|error| RegistryError::SpecParse {
                path: relative.to_path_buf(),
                line: 0,
                message: error.message,
            })?;
        let doc = parse_document(&relative_text, &text);
        if let Some(issue) = doc
            .issues
            .iter()
            .find(|issue| issue.severity == Severity::Error)
        {
            return Err(RegistryError::SpecParse {
                path: relative.to_path_buf(),
                line: issue.line,
                message: issue.message.clone(),
            });
        }

        // The router's forward law: extension stripped, PROP/FEAT slug
        // truncated — the address form every citation in the repo uses.
        let document = vibe_spec::canonical_doc_path(&relative_text);
        for fact in doc.blocks.iter().flat_map(|block| &block.facts) {
            let Some(anchor) = fact.id.as_deref() else {
                continue;
            };
            if !fact.marked {
                continue;
            }
            let isolated = parse_document(&relative_text, &fact.body);
            let marker = isolated.markers.iter().find(|marker| {
                !matches!(
                    marker.granularity,
                    Granularity::Document | Granularity::Section
                )
            });
            let Some(marker) = marker else {
                return Err(RegistryError::SpecParse {
                    path: relative.to_path_buf(),
                    line: fact.line,
                    message: format!(
                        "fact `{anchor}` is marked but progress-core returned no unit marker"
                    ),
                });
            };
            let address = format!("spec://{package}/{document}#{anchor}");
            let spec_fact = SpecFact {
                status: FactStatus::new(marker.stage, marker.state),
                path: relative.to_path_buf(),
                line: fact.line + marker.line.saturating_sub(1),
            };
            if snapshot.insert(address.clone(), spec_fact).is_some() {
                return Err(RegistryError::SpecParse {
                    path: relative.to_path_buf(),
                    line: fact.line,
                    message: format!("duplicate full fact address `{address}`"),
                });
            }
        }
    }
    Ok(snapshot)
}

fn collect_markdown(root: &Path, files: &mut Vec<PathBuf>) -> Result<(), RegistryError> {
    if !root.exists() {
        return Ok(());
    }
    let entries = fs::read_dir(root).map_err(|source| RegistryError::Io {
        path: root.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| RegistryError::Io {
            path: root.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_markdown(&path, files)?;
        } else if vibe_specdoc::is_spec_source(&path) {
            files.push(path);
        }
    }
    Ok(())
}
