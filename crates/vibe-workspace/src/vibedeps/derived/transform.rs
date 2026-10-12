//! The frozen suffix converter and the descriptor-selected counterpart.

use std::fs;
use std::path::{Path, PathBuf};

use vibe_core::layout;
use vibe_core::manifest::{SourceInterpretation, SpecFormat};
use vibe_facts::PackageOverlay;
use vibe_safefs::Project;
use vibe_specdoc::doc::Vocabulary;

use super::apply_overlay;
use crate::vibedeps::{
    ConversionFailure, ConversionFailureClass, ConversionIssue, SlotFileDisposition,
};
use crate::{WorkspaceError, path_to_slash};

const MAX_DECLARED_VIBE_SOURCE_BYTES: usize = 16 * 1024 * 1024;

pub(super) enum DeclaredTransform {
    Unchanged,
    Converted(PathBuf, Vec<u8>),
    Failed {
        source_bytes: Vec<u8>,
        reason: ConversionFailure,
        legacy_error: vibe_specdoc::Error,
    },
}

pub(super) fn recorded_transform(
    source: &Path,
    outcome: DeclaredTransform,
) -> Option<(PathBuf, Vec<u8>, SlotFileDisposition)> {
    match outcome {
        DeclaredTransform::Unchanged => None,
        DeclaredTransform::Converted(path, bytes) => {
            Some((path, bytes, SlotFileDisposition::Converted))
        }
        DeclaredTransform::Failed {
            source_bytes,
            reason,
            ..
        } => Some((
            source.to_path_buf(),
            source_bytes,
            SlotFileDisposition::CopiedUnconverted(reason),
        )),
    }
}

pub(super) fn transform_file(
    source: &Path,
    rel: &Path,
    format: SpecFormat,
    package: &str,
    overlay: &PackageOverlay,
) -> Result<DeclaredTransform, WorkspaceError> {
    if !is_spec_document(rel) {
        return Ok(DeclaredTransform::Unchanged);
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
        return Ok(DeclaredTransform::Unchanged);
    }
    let text = match fs::read_to_string(source) {
        Ok(text) => text,
        Err(error) if overlay_applies => {
            return Err(WorkspaceError::SpecMaterialization {
                path: source.to_path_buf(),
                reason: format!("overlay-targeted spec cannot be read as text: {error}"),
            });
        }
        Err(_) => return Ok(DeclaredTransform::Unchanged),
    };
    if format == SpecFormat::Xml && extension == Some("md") {
        let outcome = transform_declared_bytes_outcome(
            rel,
            format,
            package,
            overlay,
            SourceInterpretation::VibeSpec,
            "text/markdown",
            text.as_bytes(),
        )?;
        if overlay_applies {
            if let DeclaredTransform::Failed { legacy_error, .. } = &outcome {
                return Err(source_error(
                    source,
                    format!("overlay-targeted spec cannot enter the pivot: {legacy_error}"),
                ));
            }
        }
        return Ok(outcome);
    }
    let parsed = match extension {
        Some("md") => vibe_specdoc::from_markdown(&text),
        Some("xml") => vibe_specdoc::from_xml(&text),
        _ => return Ok(DeclaredTransform::Unchanged),
    };
    let mut doc = match parsed {
        Ok(doc) => doc,
        Err(error) if overlay_applies => {
            return Err(WorkspaceError::SpecMaterialization {
                path: source.to_path_buf(),
                reason: format!("overlay-targeted spec cannot enter the pivot: {error}"),
            });
        }
        Err(_) => return Ok(DeclaredTransform::Unchanged),
    };
    apply_overlay(&mut doc, &address_prefix, overlay);
    let (converted, output) = match format {
        SpecFormat::Xml => (vibe_specdoc::to_xml(&doc), rel.with_extension("xml")),
        SpecFormat::Markdown => (vibe_specdoc::to_markdown(&doc), rel.with_extension("md")),
        SpecFormat::Mixed => return Ok(DeclaredTransform::Unchanged),
    };
    Ok(DeclaredTransform::Converted(output, converted.into_bytes()))
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
) -> Result<DeclaredTransform, WorkspaceError> {
    if !is_spec_document(rel) {
        return Err(source_error(
            source,
            "selected Vibe source is outside the supported specs-root/README layout",
        ));
    }
    if !needs_declared_conversion(rel, format, package, overlay, media) {
        return Ok(DeclaredTransform::Unchanged);
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
    transform_declared_bytes_outcome(
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
    match transform_declared_bytes_outcome(
        rel,
        format,
        package,
        overlay,
        interpretation,
        media,
        bytes,
    )? {
        DeclaredTransform::Unchanged => Ok(None),
        DeclaredTransform::Converted(path, bytes) => Ok(Some((path, bytes))),
        DeclaredTransform::Failed { legacy_error, .. } => Err(source_error(
            rel,
            format!("declared Vibe source cannot enter the pivot: {legacy_error}"),
        )),
    }
}

/// One pure parser/converter for both materialization and C3 verified replay.
/// Only a selected Markdown source requested as XML may return `Failed`;
/// every other unsupported input stays a loud error.
fn transform_declared_bytes_outcome(
    rel: &Path,
    format: SpecFormat,
    package: &str,
    overlay: &PackageOverlay,
    interpretation: SourceInterpretation,
    media: &str,
    bytes: &[u8],
) -> Result<DeclaredTransform, WorkspaceError> {
    if !is_spec_document(rel) {
        return Err(source_error(
            rel,
            "selected Vibe source is outside the supported specs-root/README layout",
        ));
    }
    let target_extension = match format {
        SpecFormat::Xml => "xml",
        SpecFormat::Markdown => "md",
        SpecFormat::Mixed => return Ok(DeclaredTransform::Unchanged),
    };
    if !needs_declared_conversion(rel, format, package, overlay, media) {
        return Ok(DeclaredTransform::Unchanged);
    }
    let address_prefix = format!(
        "spec://{package}/{}#",
        vibe_spec::canonical_doc_path(&path_to_slash(rel))
    );

    let text = std::str::from_utf8(bytes)
        .map_err(|_| source_error(rel, "declared Vibe source is not UTF-8"))?;
    let mut doc = match (interpretation, media) {
        (SourceInterpretation::VibeSpec, "application/xml") => {
            vibe_specdoc::from_xml_with(text, Vocabulary::Spec).map_err(|error| {
                source_error(
                    rel,
                    format!("declared Vibe source cannot enter the pivot: {error}"),
                )
            })?
        }
        (SourceInterpretation::VibeDoc, "application/xml") => {
            vibe_specdoc::from_xml_with(text, Vocabulary::Doc).map_err(|error| {
                source_error(
                    rel,
                    format!("declared Vibe source cannot enter the pivot: {error}"),
                )
            })?
        }
        (SourceInterpretation::VibeSpec, "text/markdown") => {
            match vibe_specdoc::from_markdown_detailed(text) {
                Ok(doc) => doc,
                Err(failure) if format == SpecFormat::Xml => {
                    let legacy_error = failure.clone().legacy_error();
                    let issues = match failure {
                        vibe_specdoc::MarkdownConversionFailure::Issues(issues) => issues
                            .into_iter()
                            .map(|issue| {
                                Ok(ConversionIssue {
                                    class: conversion_issue_class(issue.code),
                                    line: u32::try_from(issue.line).map_err(|_| {
                                        source_error(rel, "parser issue line exceeds u32")
                                    })?,
                                    message: issue.message,
                                })
                            })
                            .collect::<Result<Vec<_>, WorkspaceError>>()?,
                        vibe_specdoc::MarkdownConversionFailure::PivotAdapter(error) => {
                            vec![ConversionIssue {
                                class: ConversionFailureClass::PivotAdapter,
                                line: u32::try_from(error.line).map_err(|_| {
                                    source_error(rel, "pivot issue line exceeds u32")
                                })?,
                                message: error.message,
                            }]
                        }
                    };
                    return Ok(DeclaredTransform::Failed {
                        source_bytes: bytes.to_vec(),
                        reason: ConversionFailure { issues },
                        legacy_error,
                    });
                }
                Err(failure) => {
                    return Err(source_error(
                        rel,
                        format!(
                            "declared Vibe source cannot enter the pivot: {}",
                            failure.legacy_error()
                        ),
                    ));
                }
            }
        }
        _ => {
            return Err(source_error(
                rel,
                "declared Vibe media has no compatible parser",
            ));
        }
    };
    apply_overlay(&mut doc, &address_prefix, overlay);
    let converted = if format == SpecFormat::Xml {
        vibe_specdoc::to_xml(&doc)
    } else {
        vibe_specdoc::to_markdown(&doc)
    };
    Ok(DeclaredTransform::Converted(
        rel.with_extension(target_extension),
        converted.into_bytes(),
    ))
}

fn conversion_issue_class(code: vibe_specdoc::MarkdownIssueCode) -> ConversionFailureClass {
    use vibe_specdoc::MarkdownIssueCode as Code;
    match code {
        Code::Vocabulary => ConversionFailureClass::Vocabulary,
        Code::Malformed => ConversionFailureClass::Malformed,
        Code::MissingAttr => ConversionFailureClass::MissingAttr,
        Code::Stranded => ConversionFailureClass::Stranded,
        Code::MidParagraph => ConversionFailureClass::MidParagraph,
        Code::DuplicateStatus => ConversionFailureClass::DuplicateStatus,
        Code::WrapperMismatch => ConversionFailureClass::WrapperMismatch,
        Code::Unmarked => ConversionFailureClass::Unmarked,
        Code::MissingAnchor => ConversionFailureClass::MissingAnchor,
        Code::DuplicateId => ConversionFailureClass::DuplicateId,
        Code::FenceBinding => ConversionFailureClass::FenceBinding,
        Code::TerminalRequirements => ConversionFailureClass::TerminalRequirements,
        Code::SwallowedAnchor => ConversionFailureClass::SwallowedAnchor,
    }
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
