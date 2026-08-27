//! Deterministic transformed-slot materialisation (PROP-045).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-045#materialisation");

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vibe_core::layout;
use vibe_core::manifest::SpecFormat;
use vibe_core::{ContentHash, Group};
use vibe_facts::{FactStatus, PackageOverlay, Registry, overlay_file_hash};
use vibe_specdoc::doc::{Block, Section, SpecDoc, StatusEl, Unit};

use super::slot_diff::{
    MaterialiseReport, PreparedSlotFile, compute_prepared_payload_hash, reconcile_slot,
    sort_prepared_files,
};
use super::{
    CopyMode, SLOT_RECORD_FILENAME, SlotFile, SlotFileDisposition, SlotRecord,
    compute_recorded_payload_hash, io_err, read_slot_record, slot_abs_path,
};
use crate::{WorkspaceError, path_to_slash};

/// Legacy transformed-slot identity filename, retained read-only.
pub const DERIVED_MANIFEST_FILENAME: &str = ".vibe-derived.toml";
/// Current specdoc pivot recipe recorded in transformed-slot manifests.
pub const CONVERTER_RECIPE: &str = vibe_specdoc::CONVERTER_RECIPE;

const DERIVED_MANIFEST_SCHEMA: u32 = 1;
const LEGACY0_EXCLUDES: &[&str] = &[".git", ".vibe", "target", "node_modules", ".vibeignore"];

/// One source-to-output decision recorded by the derived materialiser.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivedFile {
    pub source: String,
    pub output: String,
    pub disposition: DerivedFileDisposition,
}

/// Whether a derived-slot file passed through the pivot or copied unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DerivedFileDisposition {
    Converted,
    Copied,
}

/// Identity and file-accounting record for one transformed dependency slot.
///
/// ```
/// use vibe_core::manifest::SpecFormat;
/// use vibe_workspace::vibedeps::{DerivedManifest, DERIVED_MANIFEST_FILENAME};
///
/// let wire = format!(
///     "schema = 1\nsource_hash = \"sha256:source\"\n\
///      output_format = \"xml\"\nconverter_recipe = \"specdoc/1\"\n\
///      derived_hash = \"sha256:derived\"\n"
/// );
/// let manifest: DerivedManifest = toml::from_str(&wire).unwrap();
/// assert_eq!(manifest.output_format, SpecFormat::Xml);
/// assert_eq!(DERIVED_MANIFEST_FILENAME, ".vibe-derived.toml");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DerivedManifest {
    pub schema: u32,
    pub source_hash: String,
    pub output_format: SpecFormat,
    pub converter_recipe: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlay_hash: Option<String>,
    pub derived_hash: String,
    #[serde(default, rename = "file")]
    pub files: Vec<DerivedFile>,
}

/// Materialise a package in the requested representation.
///
/// `Mixed` delegates to the byte-for-byte materialiser. `Markdown` and `Xml`
/// transform only spec-genre documents: any
/// `.md`/`.xml` below the live specs root, plus the package-root README pair.
/// Files already in the target format and every non-spec file copy verbatim.
/// An opposite-format candidate that the pivot rejects also copies verbatim
/// and is recorded as `copied`, preserving install availability honestly.
/// Existing recorded slots are reconciled by output path and hash, so renamed
/// outputs are removed while unrecorded build artifacts remain untouched.
#[allow(clippy::too_many_arguments)]
pub fn materialise_with_spec_format(
    workspace_root: &Path,
    group: &Group,
    name: &str,
    version: &semver::Version,
    content_src: &Path,
    mode: CopyMode,
    spec_format: SpecFormat,
    source_hash: &ContentHash,
) -> Result<Vec<PathBuf>, WorkspaceError> {
    materialise_with_spec_format_report(
        workspace_root,
        group,
        name,
        version,
        content_src,
        mode,
        spec_format,
        source_hash,
    )
    .map(MaterialiseReport::into_footprint)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn materialise_with_spec_format_report(
    workspace_root: &Path,
    group: &Group,
    name: &str,
    version: &semver::Version,
    content_src: &Path,
    mode: CopyMode,
    spec_format: SpecFormat,
    source_hash: &ContentHash,
) -> Result<MaterialiseReport, WorkspaceError> {
    if spec_format == SpecFormat::Mixed {
        return super::materialise_with_report(
            workspace_root,
            group,
            name,
            version,
            content_src,
            mode,
            source_hash,
        );
    }
    let package = format!("{group}/{name}");
    let registry =
        Registry::load(workspace_root).map_err(|error| WorkspaceError::SpecMaterialization {
            path: workspace_root.join(layout::current_vibefacts_root()),
            reason: format!("package adoption registry cannot be loaded: {error}"),
        })?;
    let overlay = registry.package_overlay(&package);
    let overlay_hash = overlay_file_hash(workspace_root, &package)
        .map(|hash| {
            ContentHash::parse(&hash).map_err(|error| WorkspaceError::SpecMaterialization {
                path: workspace_root.join(layout::current_vibefacts_root()),
                reason: format!("package overlay hash is invalid: {error}"),
            })
        })
        .transpose()?;
    let slot = slot_abs_path(workspace_root, group, name, version);
    if !content_src.is_dir() {
        return Err(WorkspaceError::SpecMaterialization {
            path: content_src.to_path_buf(),
            reason: "source content tree does not exist or is not a directory".to_string(),
        });
    }
    super::refuse_reserved_source_record(content_src)?;
    let mut sources = Vec::new();
    collect_files(content_src, content_src, &mut sources)?;
    sources.sort_by(|a, b| a.0.cmp(&b.0));

    let mut outputs = BTreeSet::new();
    let mut incoming = Vec::new();
    let mut files = Vec::new();
    for (rel, source) in sources {
        let transformed = transform_file(&source, &rel, spec_format, &package, &overlay)?;
        let output_rel = transformed
            .as_ref()
            .map_or_else(|| rel.clone(), |(path, _)| path.clone());
        let output_wire = path_to_slash(&output_rel);
        if !outputs.insert(output_wire.clone()) {
            return Err(WorkspaceError::SpecMaterialization {
                path: slot.join(&output_rel),
                reason: format!(
                    "more than one source maps to `{output_wire}` under {} output",
                    spec_format.as_str()
                ),
            });
        }
        let (prepared, disposition) = if let Some((_, bytes)) = transformed {
            (
                PreparedSlotFile::from_bytes(output_rel.clone(), bytes),
                SlotFileDisposition::Converted,
            )
        } else {
            (
                PreparedSlotFile::from_source(output_rel.clone(), source, mode)?,
                SlotFileDisposition::Copied,
            )
        };
        files.push(SlotFile {
            path: output_wire,
            sha256: prepared.sha256().to_string(),
            source: Some(path_to_slash(&rel)),
            disposition: Some(disposition),
        });
        incoming.push(prepared);
    }

    // One canonical order for the payload vector and the persisted rows:
    // the shared flattened forward-slash sorter — exactly the order
    // `validate_file_rows` enforces and `compute_recorded_payload_hash`
    // consumes. Host `Path` order compares component-wise and diverges from
    // it (e.g. `a/x` sorts before `a.md` component-wise but after it
    // flattened), which would desync `derived_hash` from verification.
    sort_prepared_files(&mut incoming);
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let derived_hash = compute_prepared_payload_hash(&incoming)?;
    let record = SlotRecord {
        schema: super::SLOT_RECORD_SCHEMA,
        source_hash: source_hash.clone(),
        spec_format,
        converter_recipe: Some(CONVERTER_RECIPE.to_string()),
        overlay_hash,
        derived_hash: Some(derived_hash),
        files,
    };
    reconcile_slot(&slot, &incoming, &record)
}

/// Read and validate the typed derived manifest from `slot`.
pub fn read_derived_manifest(slot: &Path) -> Result<DerivedManifest, String> {
    let record_path = slot.join(SLOT_RECORD_FILENAME);
    match fs::symlink_metadata(&record_path) {
        Ok(_) => {
            let record = read_slot_record(slot)?;
            if record.spec_format == SpecFormat::Mixed {
                return Err("slot record describes a mixed slot, not a derived slot".to_string());
            }
            let converter_recipe = record
                .converter_recipe
                .ok_or_else(|| "transformed slot record has no converter_recipe".to_string())?;
            let derived_hash = record
                .derived_hash
                .ok_or_else(|| "transformed slot record has no derived_hash".to_string())?;
            let files = record
                .files
                .into_iter()
                .map(|file| {
                    let source = file.source.ok_or_else(|| {
                        format!("transformed slot row `{}` has no source", file.path)
                    })?;
                    let disposition = match file.disposition.ok_or_else(|| {
                        format!("transformed slot row `{}` has no disposition", file.path)
                    })? {
                        SlotFileDisposition::Converted => DerivedFileDisposition::Converted,
                        SlotFileDisposition::Copied => DerivedFileDisposition::Copied,
                    };
                    Ok(DerivedFile {
                        source,
                        output: file.path,
                        disposition,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            return Ok(DerivedManifest {
                schema: DERIVED_MANIFEST_SCHEMA,
                source_hash: record.source_hash.as_str().to_string(),
                output_format: record.spec_format,
                converter_recipe,
                overlay_hash: record.overlay_hash.map(|hash| hash.as_str().to_string()),
                derived_hash: derived_hash.as_str().to_string(),
                files,
            });
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "cannot inspect slot record `{}`: {error}",
                record_path.display()
            ));
        }
    }
    let path = slot.join(DERIVED_MANIFEST_FILENAME);
    let text = fs::read_to_string(&path).map_err(|e| format!("cannot read manifest: {e}"))?;
    let manifest: DerivedManifest =
        toml::from_str(&text).map_err(|e| format!("cannot parse manifest: {e}"))?;
    if manifest.schema != DERIVED_MANIFEST_SCHEMA {
        return Err(format!(
            "schema is {}, current schema is {DERIVED_MANIFEST_SCHEMA}",
            manifest.schema
        ));
    }
    Ok(manifest)
}

/// Cheap freshness gate consulted before presence trust. New slot records are
/// preferred; legacy derived manifests remain readable when no record exists.
pub fn format_is_current(slot: &Path, expected: SpecFormat) -> bool {
    let record_path = slot.join(SLOT_RECORD_FILENAME);
    match fs::symlink_metadata(&record_path) {
        Ok(_) => {
            return read_slot_record(slot).is_ok_and(|record| {
                record.spec_format == expected
                    && match expected {
                        SpecFormat::Mixed => true,
                        SpecFormat::Markdown | SpecFormat::Xml => {
                            record.converter_recipe.as_deref() == Some(CONVERTER_RECIPE)
                                && record.overlay_hash.as_ref().map(ContentHash::as_str)
                                    == current_overlay_hash_for_slot(slot).as_deref()
                        }
                    }
            });
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return false,
    }
    let manifest_path = slot.join(DERIVED_MANIFEST_FILENAME);
    match expected {
        SpecFormat::Mixed => !manifest_path.exists(),
        SpecFormat::Markdown | SpecFormat::Xml => {
            read_derived_manifest(slot).is_ok_and(|manifest| {
                manifest.output_format == expected
                    && manifest.overlay_hash == current_overlay_hash_for_slot(slot)
            })
        }
    }
}

/// Derive the package overlay's current byte identity from a canonical slot
/// path without widening the public freshness API.
pub fn current_overlay_hash_for_slot(slot: &Path) -> Option<String> {
    let package_dir = slot.parent()?;
    let vibedeps_dir = package_dir.parent()?;
    if vibedeps_dir.file_name()?.to_str()? != super::VIBEDEPS_DIR {
        return None;
    }
    let workspace_root = vibedeps_dir
        .ancestors()
        .nth(layout::current_vibedeps_root().components().count())?;
    let package_key = package_dir.file_name()?.to_str()?;
    overlay_file_hash(workspace_root, package_key)
}

/// Recipe-0 content hash over a transformed slot. New records hash exactly
/// their payload rows; legacy slots retain the frozen walk/exclusion recipe.
pub fn compute_derived_hash(slot: &Path) -> Result<String, String> {
    let record_path = slot.join(SLOT_RECORD_FILENAME);
    match fs::symlink_metadata(&record_path) {
        Ok(_) => {
            let record = read_slot_record(slot)?;
            if record.spec_format == SpecFormat::Mixed {
                return Err("cannot compute a derived hash for a mixed slot record".to_string());
            }
            return compute_recorded_payload_hash(slot, &record.files)
                .map(|hash| hash.as_str().to_string());
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "cannot inspect slot record `{}`: {error}",
                record_path.display()
            ));
        }
    }
    let mut entries = Vec::new();
    collect_hash_files(slot, slot, &mut entries)?;
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hasher = Sha256::new();
    for (rel, path) in entries {
        hasher.update(path_to_slash(&rel).as_bytes());
        hasher.update([0]);
        let bytes =
            fs::read(&path).map_err(|e| format!("cannot read `{}`: {e}", path.display()))?;
        hasher.update(bytes);
        hasher.update([0]);
    }
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write;
        let _ = write!(&mut hex, "{byte:02x}");
    }
    Ok(format!("sha256:{hex}"))
}

fn collect_files(
    dir: &Path,
    root: &Path,
    out: &mut Vec<(PathBuf, PathBuf)>,
) -> Result<(), WorkspaceError> {
    for entry in fs::read_dir(dir).map_err(|e| io_err(dir, e))? {
        let entry = entry.map_err(|e| io_err(dir, e))?;
        if entry.file_name() == ".git" {
            continue;
        }
        let path = entry.path();
        let kind = entry.file_type().map_err(|e| io_err(&path, e))?;
        if kind.is_dir() {
            collect_files(&path, root, out)?;
        } else if kind.is_file() {
            let rel = path
                .strip_prefix(root)
                .map_err(|_| WorkspaceError::SpecMaterialization {
                    path: path.clone(),
                    reason: "walked path escaped the source root".to_string(),
                })?
                .to_path_buf();
            out.push((rel, path));
        }
    }
    Ok(())
}

fn transform_file(
    source: &Path,
    rel: &Path,
    format: SpecFormat,
    package: &str,
    overlay: &PackageOverlay,
) -> Result<Option<(PathBuf, Vec<u8>)>, WorkspaceError> {
    if !is_spec_document(rel) {
        return Ok(None);
    }
    let extension = rel.extension().and_then(|s| s.to_str());
    let opposite = matches!(
        (format, extension),
        (SpecFormat::Xml, Some("md")) | (SpecFormat::Markdown, Some("xml"))
    );
    let address_prefix = format!(
        "spec://{package}/{}#",
        vibe_spec::canonical_doc_path(&path_to_slash(rel))
    );
    let overlay_applies = overlay.contains_document(&address_prefix);
    if !opposite && !overlay_applies {
        return Ok(None);
    }
    let text = match fs::read_to_string(source) {
        Ok(text) => text,
        Err(error) if overlay_applies => {
            return Err(WorkspaceError::SpecMaterialization {
                path: source.to_path_buf(),
                reason: format!("overlay-targeted spec cannot be read as text: {error}"),
            });
        }
        Err(_) => return Ok(None),
    };
    let parsed = match extension {
        Some("md") => vibe_specdoc::from_markdown(&text),
        Some("xml") => vibe_specdoc::from_xml(&text),
        _ => return Ok(None),
    };
    let mut doc = match parsed {
        Ok(doc) => doc,
        Err(error) if overlay_applies => {
            return Err(WorkspaceError::SpecMaterialization {
                path: source.to_path_buf(),
                reason: format!("overlay-targeted spec cannot enter the pivot: {error}"),
            });
        }
        Err(_) => return Ok(None),
    };
    apply_overlay(&mut doc, &address_prefix, overlay);
    let (converted, output) = match format {
        SpecFormat::Xml => (vibe_specdoc::to_xml(&doc), rel.with_extension("xml")),
        SpecFormat::Markdown => (vibe_specdoc::to_markdown(&doc), rel.with_extension("md")),
        SpecFormat::Mixed => return Ok(None),
    };
    Ok(Some((output, converted.into_bytes())))
}

fn apply_overlay(doc: &mut SpecDoc, address_prefix: &str, overlay: &PackageOverlay) {
    apply_blocks(&mut doc.preamble, address_prefix, overlay);
    for section in &mut doc.sections {
        apply_section(section, address_prefix, overlay);
    }
}

fn apply_section(section: &mut Section, address_prefix: &str, overlay: &PackageOverlay) {
    apply_blocks(&mut section.blocks, address_prefix, overlay);
    for child in &mut section.sections {
        apply_section(child, address_prefix, overlay);
    }
}

fn apply_blocks(blocks: &mut [Block], address_prefix: &str, overlay: &PackageOverlay) {
    for block in blocks {
        match block {
            Block::Paragraph(unit) | Block::Quote(unit) => {
                apply_unit(unit, address_prefix, overlay);
            }
            Block::List { items, .. } => {
                for unit in items {
                    apply_unit(unit, address_prefix, overlay);
                }
            }
            Block::Table { rows } => {
                for unit in rows.iter_mut().flatten() {
                    apply_unit(unit, address_prefix, overlay);
                }
            }
            Block::Fence { .. } => {}
        }
    }
}

fn apply_unit(unit: &mut Unit, address_prefix: &str, overlay: &PackageOverlay) {
    let Some(fact) = unit.fact.as_mut() else {
        return;
    };
    let Some(id) = fact.id.as_deref() else {
        return;
    };
    let Some(status) = overlay.status_for(&format!("{address_prefix}{id}")) else {
        return;
    };
    match fact.status.as_mut() {
        Some(authored) => {
            authored.stage = status.stage();
            authored.state = status.state();
        }
        None => fact.status = Some(status_element(status)),
    }
}

fn status_element(status: FactStatus) -> StatusEl {
    StatusEl {
        stage: status.stage(),
        state: status.state(),
        action: None,
        actionstage: None,
        audience: Vec::new(),
        comment: None,
        r#ref: None,
    }
}

fn is_spec_document(rel: &Path) -> bool {
    let extension = rel.extension().and_then(|s| s.to_str());
    if !matches!(extension, Some("md" | "xml")) {
        return false;
    }
    let under_spec = rel.starts_with(layout::current_specs_root());
    let root_readme = rel
        .parent()
        .is_some_and(|parent| parent.as_os_str().is_empty())
        && matches!(rel.file_stem().and_then(|s| s.to_str()), Some("README"));
    under_spec || root_readme
}

/// True for the slot's generated boot artifacts (either `STATIC` spelling and
/// `INDEX.md`) — projections bootgen regenerates after
/// materialisation, excluded from the derived hash and the format-purity
/// claim exactly like the derived manifest itself.
pub(crate) fn is_generated_boot_artifact(root: &Path, path: &Path) -> bool {
    let Ok(rel) = path.strip_prefix(root) else {
        return false;
    };
    rel == layout::current_boot_static_md()
        || rel == layout::current_boot_static_xml()
        || rel == layout::current_boot_index()
}

fn collect_hash_files(
    dir: &Path,
    root: &Path,
    out: &mut Vec<(PathBuf, PathBuf)>,
) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| format!("cannot walk `{}`: {e}", dir.display()))? {
        let entry = entry.map_err(|e| format!("cannot walk `{}`: {e}", dir.display()))?;
        let name = entry.file_name();
        let name_text = name.to_string_lossy();
        if LEGACY0_EXCLUDES.contains(&name_text.as_ref())
            || name_text == DERIVED_MANIFEST_FILENAME
            || name_text == SLOT_RECORD_FILENAME
        {
            continue;
        }
        // Slot-internal GENERATED boot artifacts are outside the derived
        // identity: bootgen writes a child generated STATIC / INDEX lane
        // into a dependency slot AFTER materialisation, and by
        // the boot-lane law those artifacts are Markdown regardless of
        // spec_format — hashing them would stale every transformed slot
        // the moment its boot regenerates, and counting them would fake a
        // purity violation. Same exclusion genre as the manifest itself.
        if is_generated_boot_artifact(root, &entry.path()) {
            continue;
        }
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|e| format!("cannot inspect `{}`: {e}", path.display()))?;
        if kind.is_dir() {
            collect_hash_files(&path, root, out)?;
        } else if kind.is_file() {
            let rel = path
                .strip_prefix(root)
                .map_err(|_| format!("walked path `{}` escaped hash root", path.display()))?
                .to_path_buf();
            out.push((rel, path));
        }
    }
    Ok(())
}
