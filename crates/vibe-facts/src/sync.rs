//! Asymmetric host-spec synchronization: the spec marker is authoritative.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-046#laws");

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use progress_core::doc::Severity;
use progress_core::model::Granularity;
use progress_core::parse::parse_document;
use vibe_core::manifest::{
    Manifest, SelectedSource, SourceDescriptor, SourceInterpretation, SourceSelector,
    select_source_rows,
};
use vibe_safefs::{Project, StableFileSnapshot};
use vibe_specdoc::Vocabulary;

use crate::{FactOrigin, FactStatus, Registry, RegistryError};

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
    let manifest_path = project_root.join(Manifest::FILENAME);
    let absolute = if project_root.is_absolute() {
        project_root.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|source| RegistryError::Io {
                path: project_root.to_path_buf(),
                source,
            })?
            .join(project_root)
    };
    let pinned = Project::open(&absolute).map_err(|error| {
        declared_refusal(&manifest_path, format!("cannot pin host root: {error}"))
    })?;
    let manifest_snapshot = pinned
        .read_file_snapshot_bounded(Manifest::FILENAME, HOST_MANIFEST_CAP)
        .map_err(|error| {
            declared_refusal(
                &manifest_path,
                format!("cannot capture host manifest: {error}"),
            )
        })?
        .ok_or_else(|| declared_refusal(&manifest_path, "host manifest is unavailable"))?;
    let manifest_text = std::str::from_utf8(&manifest_snapshot.bytes).map_err(|error| {
        RegistryError::InvalidManifest {
            path: manifest_path.clone(),
            reason: format!("manifest is not UTF-8: {error}"),
        }
    })?;
    let manifest =
        Manifest::parse_str(manifest_text).map_err(|error| RegistryError::InvalidManifest {
            path: manifest_path.clone(),
            reason: error.to_string(),
        })?;
    if let Some(project) = manifest.project.as_ref() {
        if !project.sources.is_empty() {
            let group = project
                .group
                .as_ref()
                .ok_or_else(|| RegistryError::InvalidManifest {
                    path: manifest_path.clone(),
                    reason: "declared host sources need a self-coordinate group".into(),
                })?;
            vibe_core::PackageName::parse(&project.name).map_err(|error| {
                RegistryError::InvalidManifest {
                    path: manifest_path.clone(),
                    reason: format!("declared host source name: {error}"),
                }
            })?;
            let coordinate = format!("{group}/{}", project.name);
            return load_declared_snapshot(
                &pinned,
                &manifest_snapshot,
                &manifest_path,
                needed,
                &coordinate,
                &project.sources,
            );
        }
    }
    let node = manifest
        .consumer_node()
        .ok_or_else(|| RegistryError::InvalidManifest {
            path: manifest_path.clone(),
            reason: "the root manifest carries neither `[project]` nor `[package]`".into(),
        })?;
    let group = node.group.ok_or_else(|| RegistryError::InvalidManifest {
        path: manifest_path.clone(),
        reason: "the root declares no `group` — spec:// addressing needs `<group>/<name>`".into(),
    })?;
    let package = format!("{group}/{}", node.name);
    let spec_root = project_root.join(vibe_core::layout::current_specs_root());
    let mut files = Vec::new();
    let mut current = project_root.to_path_buf();
    for component in vibe_core::layout::current_specs_root().components() {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(declared_refusal(
                    &current,
                    "host spec root contains a symlink",
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(source) => {
                return Err(RegistryError::Io {
                    path: current,
                    source,
                });
            }
        }
    }
    collect_markdown(&spec_root, project_root, needed, &mut files)?;
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
        add_spec_facts(&package, relative, &relative_text, &text, &mut snapshot)?;
    }
    recheck_manifest(&pinned, &manifest_path, &manifest_snapshot)?;
    Ok(snapshot)
}

const HOST_MANIFEST_CAP: usize = 1024 * 1024;
const HOST_SOURCE_CAP: usize = 16 * 1024 * 1024;
const HOST_TOTAL_CAP: u64 = 64 * 1024 * 1024;
const HOST_CITED_CAP: usize = 512;

fn declared_refusal(path: &Path, reason: impl Into<String>) -> RegistryError {
    RegistryError::SpecParse {
        path: path.to_path_buf(),
        line: 0,
        message: reason.into(),
    }
}

/// The opted-in host route reads only exact declared files cited by present
/// registry entries. A directory default has no proved census at this seam.
fn load_declared_snapshot(
    pinned: &Project,
    manifest: &StableFileSnapshot,
    manifest_path: &Path,
    needed: &BTreeSet<String>,
    coordinate: &str,
    rows: &[SourceDescriptor],
) -> Result<BTreeMap<String, SpecFact>, RegistryError> {
    if needed.len() > HOST_CITED_CAP {
        return Err(declared_refusal(
            &manifest_path,
            "too many cited host source documents",
        ));
    }
    if rows
        .iter()
        .any(|row| matches!(row.selector(), SourceSelector::Directory(_)))
    {
        return Err(declared_refusal(
            &manifest_path,
            "declared host directory sources need a complete captured census",
        ));
    }
    let mut snapshot = BTreeMap::new();
    let mut total = 0_u64;
    for doc_path in needed {
        let mut chosen: Option<(&str, &SourceDescriptor)> = None;
        let mut seen = BTreeSet::new();
        for row in rows {
            let SourceSelector::File(path) = row.selector() else {
                unreachable!()
            };
            if vibe_spec::canonical_doc_path(path) != *doc_path || !seen.insert(path.as_str()) {
                continue;
            }
            let selection = select_source_rows(coordinate, rows, path).map_err(|error| {
                declared_refusal(
                    Path::new(path),
                    format!("invalid source selector: {error:?}"),
                )
            })?;
            let descriptor = match selection {
                SelectedSource::Descriptor { descriptor, .. } => descriptor,
                SelectedSource::NeedsInput {
                    declaration_indices,
                    ..
                } => {
                    return Err(declared_refusal(
                        Path::new(path),
                        format!(
                            "cited source has ambiguous declaration rows {declaration_indices:?}"
                        ),
                    ));
                }
                SelectedSource::Unclassified { .. } => {
                    return Err(declared_refusal(
                        Path::new(path),
                        "cited source is unclassified",
                    ));
                }
            };
            match descriptor.interpretation() {
                SourceInterpretation::VibeSpec => {
                    if chosen.replace((path, descriptor)).is_some() {
                        return Err(declared_refusal(
                            Path::new(path),
                            format!("two selected Vibe sources claim cited document `{doc_path}`"),
                        ));
                    }
                }
                SourceInterpretation::Commonmark
                | SourceInterpretation::XmlData
                | SourceInterpretation::Opaque => {}
                SourceInterpretation::VibeDoc => {
                    return Err(declared_refusal(
                        Path::new(path),
                        "plain project host sync does not admit vibe-doc vocabulary",
                    ));
                }
            }
        }
        let (path, descriptor) = chosen.ok_or_else(|| {
            declared_refusal(
                &manifest_path,
                format!("cited document `{doc_path}` has no selected Vibe source"),
            )
        })?;
        let remaining = HOST_TOTAL_CAP.saturating_sub(total);
        let cap = HOST_SOURCE_CAP.min(usize::try_from(remaining).unwrap_or(usize::MAX));
        let captured = pinned
            .read_file_snapshot_bounded(path, cap)
            .map_err(|error| {
                declared_refusal(
                    Path::new(path),
                    format!("cited source cannot be captured safely: {error}"),
                )
            })?
            .ok_or_else(|| declared_refusal(Path::new(path), "cited source is unavailable"))?;
        total = total.checked_add(captured.size).ok_or_else(|| {
            declared_refusal(
                Path::new(path),
                "cited source aggregate byte limit exceeded",
            )
        })?;
        if total > HOST_TOTAL_CAP {
            return Err(declared_refusal(
                Path::new(path),
                "cited source aggregate byte limit exceeded",
            ));
        }
        let text = std::str::from_utf8(&captured.bytes).map_err(|error| {
            declared_refusal(
                Path::new(path),
                format!("cited source is not UTF-8: {error}"),
            )
        })?;
        let projected = match descriptor.media() {
            "text/markdown" => text.to_owned(),
            "application/xml" => {
                let doc = vibe_specdoc::from_xml_with(text, Vocabulary::Spec)
                    .map_err(|error| declared_refusal(Path::new(path), error.to_string()))?;
                vibe_specdoc::to_markdown(&doc)
            }
            _ => {
                return Err(declared_refusal(
                    Path::new(path),
                    "selected Vibe media has no host sync reader",
                ));
            }
        };
        add_spec_facts(coordinate, Path::new(path), path, &projected, &mut snapshot)?;
    }
    recheck_manifest(pinned, manifest_path, manifest)?;
    Ok(snapshot)
}

fn recheck_manifest(
    pinned: &Project,
    manifest_path: &Path,
    before: &StableFileSnapshot,
) -> Result<(), RegistryError> {
    let after = pinned
        .read_file_snapshot_bounded(Manifest::FILENAME, HOST_MANIFEST_CAP)
        .map_err(|error| declared_refusal(&manifest_path, format!("manifest recheck: {error}")))?
        .ok_or_else(|| declared_refusal(&manifest_path, "manifest disappeared during sync read"))?;
    if before != &after {
        return Err(declared_refusal(
            &manifest_path,
            "manifest changed during host sync read",
        ));
    }
    Ok(())
}

fn add_spec_facts(
    package: &str,
    relative: &Path,
    relative_text: &str,
    text: &str,
    snapshot: &mut BTreeMap<String, SpecFact>,
) -> Result<(), RegistryError> {
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
    Ok(())
}

fn collect_markdown(
    root: &Path,
    project_root: &Path,
    needed: &BTreeSet<String>,
    files: &mut Vec<PathBuf>,
) -> Result<(), RegistryError> {
    let metadata = match fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(source) => {
            return Err(RegistryError::Io {
                path: root.to_path_buf(),
                source,
            });
        }
    };
    if metadata.file_type().is_symlink() {
        return Err(declared_refusal(root, "host spec root contains a symlink"));
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
        let metadata = fs::symlink_metadata(&path).map_err(|source| RegistryError::Io {
            path: path.clone(),
            source,
        })?;
        if metadata.file_type().is_symlink() {
            let relative = path.strip_prefix(project_root).map_err(|_| {
                RegistryError::Invariant("host spec entry escaped project root".into())
            })?;
            let relative = relative.to_string_lossy().replace('\\', "/");
            let canonical = vibe_spec::canonical_doc_path(&relative);
            let prefix = format!("{canonical}/");
            if needed.contains(&canonical)
                || needed.iter().any(|document| document.starts_with(&prefix))
            {
                return Err(declared_refusal(&path, "cited host spec path is a symlink"));
            }
            continue;
        }
        if metadata.is_dir() {
            collect_markdown(&path, project_root, needed, files)?;
        } else if metadata.is_file() && vibe_specdoc::is_spec_source(&path) {
            files.push(path);
        }
    }
    Ok(())
}
