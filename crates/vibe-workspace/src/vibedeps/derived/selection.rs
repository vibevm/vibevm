//! Descriptor selection on original package paths, before any conversion.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-REPRESENTATION-COLLISION");

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use vibe_core::manifest::{
    Manifest, PackageMeta, SelectedSource, SourceInterpretation, SourceSelector,
};
use vibe_core::{ContentHash, Group, PackageKind};
use vibe_safefs::Project;

use super::super::slot_diff::{
    MaterialiseReport, PreparedSlotFile, reconcile_slot_descriptor, sort_prepared_files,
};
use super::super::{CopyMode, SLOT_RECORD_SCHEMA, SlotFile, SlotRecord, slot_abs_path};
use super::{collect_files, transform::is_spec_document};
use crate::{WorkspaceError, path_to_slash};

const MAX_MANIFEST_BYTES: usize = 1024 * 1024;

pub(super) struct DescriptorPackage {
    pub(super) meta: PackageMeta,
    /// The same bytes must be materialized into the slot's vibe.toml.
    pub(super) manifest_bytes: Vec<u8>,
    pub(super) project: Project,
}

pub(super) enum SelectedMaterialization<'a> {
    Raw,
    Vibe {
        interpretation: SourceInterpretation,
        media: &'a str,
    },
}

/// The presence of authored source rows opts into descriptor authority. A
/// legacy manifest without those keys keeps the frozen suffix route.
pub(super) fn read_descriptor_package(
    root: &Path,
    group: &Group,
    name: &str,
    version: &semver::Version,
) -> Result<Option<DescriptorPackage>, WorkspaceError> {
    // This first read is a compatibility probe only. It cannot grant source
    // semantics: legacy no-descriptor package trees keep their old read path,
    // including hardlinked manifests. A descriptor candidate is re-opened
    // through a no-follow capability and must be byte-identical below.
    let probe_path = root.join(Manifest::FILENAME);
    let mut probe = Vec::new();
    let file = match fs::File::open(&probe_path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(source_error(
                &probe_path,
                format!("cannot probe source manifest: {error}"),
            ));
        }
    };
    file.take((MAX_MANIFEST_BYTES + 1) as u64)
        .read_to_end(&mut probe)
        .map_err(|error| {
            source_error(
                &probe_path,
                format!("cannot read bounded source manifest probe: {error}"),
            )
        })?;
    if probe.len() > MAX_MANIFEST_BYTES {
        return Err(source_error(
            &probe_path,
            "source manifest exceeds the 1 MiB selection bound",
        ));
    }
    let probe_text = std::str::from_utf8(&probe)
        .map_err(|_| source_error(&probe_path, "source manifest is not UTF-8"))?;
    let value: toml::Value = toml::from_str(probe_text).map_err(|error| {
        source_error(
            &probe_path,
            format!("source manifest TOML is invalid: {error}"),
        )
    })?;
    let authored = value
        .get("package")
        .and_then(toml::Value::as_table)
        .is_some_and(|table| table.contains_key("sources") || table.contains_key("source_exports"));
    if !authored {
        return Ok(None);
    }

    let absolute = if root.is_absolute() {
        root.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| source_error(root, format!("cannot resolve source root: {error}")))?
            .join(root)
    };
    let project = Project::open(&absolute)
        .map_err(|error| source_error(&absolute, format!("cannot pin source root: {error}")))?;
    let path = absolute.join(Manifest::FILENAME);
    let Some(snapshot) = project
        .read_file_snapshot_bounded(Manifest::FILENAME, MAX_MANIFEST_BYTES)
        .map_err(|error| {
            source_error(&path, format!("cannot capture bounded manifest: {error}"))
        })?
    else {
        return Ok(None);
    };
    let bytes = snapshot.bytes;
    if bytes != probe {
        return Err(source_error(
            &path,
            "descriptor manifest changed between compatibility probe and pinned capture",
        ));
    }
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| source_error(&path, "source manifest is not UTF-8"))?;
    let manifest = Manifest::parse_package_source(text)
        .map_err(|error| source_error(&path, format!("descriptor manifest is invalid: {error}")))?;
    let meta = manifest
        .package
        .ok_or_else(|| source_error(&path, "descriptor rows require [package]"))?;
    if &meta.group != group || meta.name != name || &meta.version != version {
        return Err(source_error(
            &path,
            "descriptor manifest identity differs from selected package",
        ));
    }
    Ok(Some(DescriptorPackage {
        meta,
        manifest_bytes: bytes,
        project,
    }))
}

pub(super) fn classify<'a>(
    package: &'a PackageMeta,
    relative: &Path,
) -> Result<SelectedMaterialization<'a>, WorkspaceError> {
    let wire = path_to_slash(relative);
    let selection = package.select_source(&wire).map_err(|error| {
        source_error(
            relative,
            format!("source path cannot be selected: {error:?}"),
        )
    })?;
    let descriptor = match selection {
        SelectedSource::Unclassified { .. } => return Ok(SelectedMaterialization::Raw),
        SelectedSource::NeedsInput {
            declaration_indices,
            ..
        } => {
            return Err(source_error(
                relative,
                format!(
                    "source interpretation is ambiguous at declaration rows {declaration_indices:?}"
                ),
            ));
        }
        SelectedSource::Descriptor { descriptor, .. } => descriptor,
    };
    let selected = match descriptor.interpretation() {
        SourceInterpretation::VibeSpec
            if matches!(descriptor.media(), "text/markdown" | "application/xml") =>
        {
            Ok(SelectedMaterialization::Vibe {
                interpretation: SourceInterpretation::VibeSpec,
                media: descriptor.media(),
            })
        }
        SourceInterpretation::VibeDoc
            if descriptor.media() == "application/xml" && package.kind == PackageKind::Doc =>
        {
            Ok(SelectedMaterialization::Vibe {
                interpretation: SourceInterpretation::VibeDoc,
                media: descriptor.media(),
            })
        }
        SourceInterpretation::Commonmark
        | SourceInterpretation::XmlData
        | SourceInterpretation::Opaque => Ok(SelectedMaterialization::Raw),
        _ => Err(source_error(
            relative,
            "selected Vibe source has incompatible media or package kind",
        )),
    }?;
    if matches!(&selected, SelectedMaterialization::Vibe { .. }) && !is_spec_document(relative) {
        return Err(source_error(
            relative,
            "selected Vibe source is outside the supported specs-root/README layout",
        ));
    }
    Ok(selected)
}

/// Exact authored rows must be present in the captured inventory; otherwise
/// a missing file or skipped symlink could silently disappear.
pub(super) fn require_exact_sources(
    package: &PackageMeta,
    sources: &[(PathBuf, PathBuf)],
    root: &Path,
) -> Result<(), WorkspaceError> {
    let found = sources
        .iter()
        .map(|(rel, _)| path_to_slash(rel))
        .collect::<std::collections::BTreeSet<_>>();
    for descriptor in &package.sources {
        if let SourceSelector::File(path) = descriptor.selector() {
            if !found.contains(path) {
                return Err(source_error(
                    &root.join(path),
                    "declared exact source is absent or not a regular file",
                ));
            }
        }
    }
    Ok(())
}

/// Output-path identity is separate from source identity. Exact and available
/// lowercase aliases are checked here; target-specific normalization aliases
/// are caught later by strict no-replace publication and need recovery.
pub(super) fn reserve_output(
    slot: &Path,
    source: &Path,
    output: &Path,
    exact: &mut BTreeMap<String, String>,
    folded: &mut BTreeMap<String, String>,
) -> Result<(), WorkspaceError> {
    let source = path_to_slash(source);
    let output = path_to_slash(output);
    if let Some(first) = exact.insert(output.clone(), source.clone()) {
        return Err(source_error(
            &slot.join(&output),
            format!("distinct sources `{first}` and `{source}` require the same output path"),
        ));
    }
    let key = output.to_lowercase();
    if let Some(first) = folded.insert(key, output.clone())
        && first != output
    {
        return Err(source_error(
            &slot.join(&output),
            format!("portable-case output collision between `{first}` and `{output}`"),
        ));
    }
    Ok(())
}

pub(super) fn source_error(path: &Path, reason: impl Into<String>) -> WorkspaceError {
    WorkspaceError::SpecMaterialization {
        path: PathBuf::from(path),
        reason: reason.into(),
    }
}

/// Mixed retains every byte and source path, but descriptor-owned packages
/// still require interpretation and output identity preflight plus strict
/// publication. Legacy Mixed continues through its original materializer.
pub(super) fn materialise_mixed_descriptor(
    workspace_root: &Path,
    content_src: &Path,
    group: &Group,
    name: &str,
    version: &semver::Version,
    mode: CopyMode,
    source_hash: &ContentHash,
    declared: &DescriptorPackage,
) -> Result<MaterialiseReport, WorkspaceError> {
    let slot = slot_abs_path(workspace_root, group, name, version);
    let mut sources = Vec::new();
    collect_files(content_src, content_src, &mut sources)?;
    sources.sort_by(|left, right| left.0.cmp(&right.0));
    require_exact_sources(&declared.meta, &sources, content_src)?;
    let mut exact = BTreeMap::new();
    let mut folded = BTreeMap::new();
    let mut addresses = BTreeMap::<String, String>::new();
    let mut prepared = Vec::new();
    let mut files = Vec::new();
    for (rel, source) in sources {
        if matches!(
            classify(&declared.meta, &rel)?,
            SelectedMaterialization::Vibe { .. }
        ) && is_spec_document(&rel)
        {
            let address = vibe_spec::canonical_doc_path(&path_to_slash(&rel));
            let wire = path_to_slash(&rel);
            if let Some(first) = addresses.insert(address.clone(), wire.clone()) {
                return Err(source_error(
                    &source,
                    format!(
                        "selected Vibe sources `{first}` and `{wire}` claim one document address `{address}`"
                    ),
                ));
            }
        }
        reserve_output(&slot, &rel, &rel, &mut exact, &mut folded)?;
        let file = if rel == Path::new(Manifest::FILENAME) {
            PreparedSlotFile::from_bytes(rel.clone(), declared.manifest_bytes.clone())
        } else {
            PreparedSlotFile::from_source(rel.clone(), source, mode)?
        };
        files.push(SlotFile {
            path: file.path_wire(),
            sha256: file.sha256().to_owned(),
            source: None,
            disposition: None,
        });
        prepared.push(file);
    }
    sort_prepared_files(&mut prepared);
    files.sort_by(|left, right| left.path.cmp(&right.path));
    let record = SlotRecord {
        schema: SLOT_RECORD_SCHEMA,
        source_hash: source_hash.clone(),
        spec_format: vibe_core::manifest::SpecFormat::Mixed,
        converter_recipe: None,
        overlay_hash: None,
        derived_hash: None,
        files,
    };
    reconcile_slot_descriptor(&slot, &prepared, &record).map_err(|error| source_error(&slot,
        format!("descriptor Mixed slot publication refused; state may be partial and requires inspection before retry: {error}")))
}
