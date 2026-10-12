//! Host-side held capture preparation, separate from query construction.
use super::*;
use crate::source_query::{PreparedOwnerCapture, PreparedWorldWitness};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use vibe_core::manifest::{Manifest, SelectedSource, SourceInterpretation, select_source_rows};
use vibe_safefs::{Project, StableFileSnapshot};
use vibe_spec::{DescriptorOutputRow, ValidatedDescriptorMap};

const MANIFEST_CAP: usize = 1024 * 1024;
const FILE_CAP: usize = wire::MAX_CONTENT_BYTES as usize;
const TOTAL_CAP: usize = 256 * 1024 * 1024;
const FILE_COUNT: usize = 4096;
mod dependencies;
mod membership;

pub(super) fn prepare_host(root: &Path) -> Result<SourceQueryService, QueryError> {
    let selected = membership::selected(root)?;
    let effective = membership::effective_node(&selected)?;
    let dependencies =
        dependencies::prepare(&selected.workspace.root, &selected.node_root, &effective)?;
    let root = selected.node_root.clone();
    let project = Project::open(&root).map_err(|_| unavailable("held host root"))?;
    let manifest_snapshot = snapshot(&project, "vibe.toml", MANIFEST_CAP)?;
    let text = std::str::from_utf8(&manifest_snapshot.bytes)
        .map_err(|_| unavailable("host manifest UTF-8"))?;
    let manifest = Manifest::parse_str(text).map_err(|_| unavailable("host manifest"))?;
    let spec_relative = if root.join("vibevm/vibespecs").is_dir() {
        "vibevm/vibespecs"
    } else {
        "spec"
    };
    let files = selected_files(&root, spec_relative, &manifest)?;
    let mut outputs = BTreeMap::new();
    let mut rows = Vec::new();
    let mut media = BTreeMap::new();
    let mut remaining = TOTAL_CAP;
    for file in files {
        let observed = snapshot(&project, &file, FILE_CAP.min(remaining))?;
        remaining = remaining
            .checked_sub(observed.bytes.len())
            .ok_or_else(|| unavailable("capture byte bound"))?;
        let (interpretation, actual_media) = selection(&manifest, &file)?;
        let is_vibe = matches!(
            interpretation,
            Some(SourceInterpretation::VibeSpec | SourceInterpretation::VibeDoc)
        );
        if is_vibe {
            media.insert(
                file.clone(),
                actual_media.ok_or_else(|| unavailable("selected media"))?,
            );
        }
        rows.push(DescriptorOutputRow {
            output: file.clone(),
            source: file.clone(),
            interpretation,
            output_sha256: observed.sha256.clone(),
            document_address: is_vibe.then(|| vibe_spec::canonical_doc_path(&file)),
        });
        outputs.insert(file, observed);
    }
    if outputs.is_empty() {
        outputs.insert("vibe.toml".into(), manifest_snapshot.clone());
        rows.push(DescriptorOutputRow {
            output: "vibe.toml".into(),
            source: "vibe.toml".into(),
            interpretation: None,
            output_sha256: manifest_snapshot.sha256.clone(),
            document_address: None,
        });
    }
    let map = ValidatedDescriptorMap::from_witnessed_rows(
        root.clone(),
        root.join(spec_relative),
        "observed-self".into(),
        manifest_snapshot.sha256.clone(),
        rows,
    )
    .and_then(|map| map.with_materialized_media(media))
    .map_err(|_| unavailable("self descriptor catalog"))?;
    let mut captures = dependencies.captures;
    captures.push(PreparedOwnerCapture::self_node(
        root,
        manifest_snapshot,
        outputs,
        Some(map),
    ));
    let world = WitnessedSourceWorld::from_prepared_node(
        &selected.workspace,
        &selected.node_rel,
        &dependencies.epoch,
        &BTreeSet::new(),
        captures,
        PreparedWorldWitness::new(dependencies.lock, selected.snapshots, dependencies.records),
    )
    .map_err(world_error)?;
    SourceQueryService::from_world(Arc::new(world))
}

pub(super) fn snapshot(
    project: &Project,
    relative: &str,
    cap: usize,
) -> Result<StableFileSnapshot, QueryError> {
    if cap == 0 {
        return Err(unavailable("capture byte bound"));
    }
    project
        .read_file_snapshot_bounded(relative, cap)
        .map_err(|_| unavailable("held source observation"))?
        .ok_or_else(|| unavailable("missing selected capture"))
}

fn selection(
    manifest: &Manifest,
    file: &str,
) -> Result<(Option<SourceInterpretation>, Option<String>), QueryError> {
    let selected = if let Some(package) = &manifest.package {
        package.select_source(file)
    } else if let Some(project) = &manifest.project {
        select_source_rows(&project.name, &project.sources, file)
    } else {
        return Err(unavailable("node source authority"));
    }
    .map_err(|_| unavailable("authored source selection"))?;
    match selected {
        SelectedSource::Unclassified { .. } => Ok((None, None)),
        SelectedSource::Descriptor { descriptor, .. } => Ok((
            Some(descriptor.interpretation()),
            Some(descriptor.media().into()),
        )),
        SelectedSource::NeedsInput { .. } => Err(refusal(
            ErrorCode::AmbiguousDocument,
            "descriptor selection",
            "Authored source interpretation is ambiguous",
            vec![file.into()],
        )),
    }
}

/// Bounded inventory only inside the host's source root, never a slot scan.
fn selected_files(
    root: &Path,
    relative: &str,
    manifest: &Manifest,
) -> Result<Vec<String>, QueryError> {
    let mut pending = vec![(root.join(relative), 0usize)];
    let mut selected = Vec::new();
    let mut visited = 0usize;
    while let Some((directory, depth)) = pending.pop() {
        if depth > 64 {
            return Err(unavailable("source depth bound"));
        }
        let metadata = match fs::symlink_metadata(&directory) {
            Ok(value) => value,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return Err(unavailable("source directory")),
        };
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(unavailable("source directory ownership"));
        }
        for entry in fs::read_dir(directory).map_err(|_| unavailable("source inventory"))? {
            visited += 1;
            if visited > FILE_COUNT {
                return Err(unavailable("source inventory bound"));
            }
            let entry = entry.map_err(|_| unavailable("source inventory entry"))?;
            let kind = entry
                .file_type()
                .map_err(|_| unavailable("source entry kind"))?;
            if kind.is_symlink() {
                return Err(unavailable("source link ownership"));
            }
            if kind.is_dir() {
                pending.push((entry.path(), depth + 1));
                continue;
            }
            if !kind.is_file() {
                continue;
            }
            let relative = entry
                .path()
                .strip_prefix(root)
                .map_err(|_| unavailable("source root containment"))?
                .to_string_lossy()
                .replace('\\', "/");
            if selection(manifest, &relative)?.0.is_some() {
                selected.push(relative);
            }
        }
    }
    selected.sort();
    Ok(selected)
}
