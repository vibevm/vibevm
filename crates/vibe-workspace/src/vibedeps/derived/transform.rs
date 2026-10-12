//! The frozen suffix converter and the descriptor-selected counterpart.

use std::fs;
use std::path::{Path, PathBuf};

use vibe_core::layout;
use vibe_core::manifest::{SourceInterpretation, SpecFormat};
use vibe_facts::PackageOverlay;
use vibe_safefs::Project;
use vibe_specdoc::doc::Vocabulary;

use super::apply_overlay;
use crate::{WorkspaceError, path_to_slash};

const MAX_DECLARED_VIBE_SOURCE_BYTES: usize = 16 * 1024 * 1024;

pub(super) fn transform_file(
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

pub(super) fn transform_declared_file(
    project: &Project,
    source: &Path,
    rel: &Path,
    format: SpecFormat,
    package: &str,
    overlay: &PackageOverlay,
    interpretation: SourceInterpretation,
    media: &str,
) -> Result<Option<(PathBuf, Vec<u8>)>, WorkspaceError> {
    if !is_spec_document(rel) {
        return Err(source_error(
            source,
            "selected Vibe source is outside the supported specs-root/README layout",
        ));
    }
    if !needs_declared_conversion(rel, format, package, overlay, media) {
        return Ok(None);
    }
    let relative = path_to_slash(rel);
    let snapshot = project
        .read_file_snapshot_bounded(&relative, MAX_DECLARED_VIBE_SOURCE_BYTES)
        .map_err(|error| {
            source_error(
                source,
                format!("declared Vibe source cannot be captured within 16 MiB: {error}"),
            )
        })?
        .ok_or_else(|| source_error(source, "declared Vibe source disappeared before capture"))?;
    transform_declared_bytes(
        rel,
        format,
        package,
        overlay,
        interpretation,
        media,
        &snapshot.bytes,
    )
}

/// The materializer's pure builtin kernel, also used for replay over an
/// already-verified original capture. It never opens a source or output path.
pub(super) fn transform_declared_bytes(
    rel: &Path,
    format: SpecFormat,
    package: &str,
    overlay: &PackageOverlay,
    interpretation: SourceInterpretation,
    media: &str,
    bytes: &[u8],
) -> Result<Option<(PathBuf, Vec<u8>)>, WorkspaceError> {
    if !is_spec_document(rel) {
        return Err(source_error(
            rel,
            "selected Vibe source is outside the supported specs-root/README layout",
        ));
    }
    let target_extension = match format {
        SpecFormat::Xml => "xml",
        SpecFormat::Markdown => "md",
        SpecFormat::Mixed => return Ok(None),
    };
    if !needs_declared_conversion(rel, format, package, overlay, media) {
        return Ok(None);
    }
    let address_prefix = format!(
        "spec://{package}/{}#",
        vibe_spec::canonical_doc_path(&path_to_slash(rel))
    );

    let text = std::str::from_utf8(bytes)
        .map_err(|_| source_error(rel, "declared Vibe source is not UTF-8"))?;
    let mut doc = match (interpretation, media) {
        (SourceInterpretation::VibeSpec, "application/xml") => {
            vibe_specdoc::from_xml_with(text, Vocabulary::Spec)
        }
        (SourceInterpretation::VibeDoc, "application/xml") => {
            vibe_specdoc::from_xml_with(text, Vocabulary::Doc)
        }
        (SourceInterpretation::VibeSpec, "text/markdown") => vibe_specdoc::from_markdown(text),
        _ => {
            return Err(source_error(
                rel,
                "declared Vibe media has no compatible parser",
            ));
        }
    }
    .map_err(|error| {
        source_error(
            rel,
            format!("declared Vibe source cannot enter the pivot: {error}"),
        )
    })?;
    apply_overlay(&mut doc, &address_prefix, overlay);
    let converted = if format == SpecFormat::Xml {
        vibe_specdoc::to_xml(&doc)
    } else {
        vibe_specdoc::to_markdown(&doc)
    };
    Ok(Some((
        rel.with_extension(target_extension),
        converted.into_bytes(),
    )))
}

fn needs_declared_conversion(
    rel: &Path,
    format: SpecFormat,
    package: &str,
    overlay: &PackageOverlay,
    media: &str,
) -> bool {
    let (target_media, target_extension) = match format {
        SpecFormat::Xml => ("application/xml", "xml"),
        SpecFormat::Markdown => ("text/markdown", "md"),
        SpecFormat::Mixed => return false,
    };
    let address_prefix = format!(
        "spec://{package}/{}#",
        vibe_spec::canonical_doc_path(&path_to_slash(rel))
    );
    media != target_media
        || rel.extension().and_then(|e| e.to_str()) != Some(target_extension)
        || overlay.contains_document(&address_prefix)
}

pub(super) fn is_spec_document(rel: &Path) -> bool {
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

fn source_error(path: &Path, reason: impl Into<String>) -> WorkspaceError {
    WorkspaceError::SpecMaterialization {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}
