//! One descriptor-selected, already captured file at the authored-facts seam.
//!
//! The incumbent observer keeps a separately qualified legacy suffix branch.
//! Its explicit-package branch and facts adoption share this bounded declared
//! reader; broader structure/export/boot consumers remain separate.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-OBSERVATION-LAW");

use vibe_core::PackageKind;
use vibe_core::manifest::{
    PackageMeta, SelectedSource, SourceDescriptor, SourceInterpretation, SourceSelectionError,
};
use vibe_specdoc::doc::Vocabulary;

use super::{SourceFileWitness, address_prefix};
use crate::AuthoredFact;

mod walk;
pub use walk::{
    CapturedDeclaredDocument, DeclaredTreeObservation, DeclaredWalkError, exact_declared_files,
    observe_declared_source,
};

/// Classification and declaration origin are distinct from byte witnesses.
/// The selected row is authored intent, not a verified trust assertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclaredSourceDisposition {
    Unclassified,
    NeedsInput {
        declaration_indices: Vec<usize>,
    },
    Selected {
        declaration_index: usize,
        descriptor: SourceDescriptor,
    },
}

/// Host-computed identity and byte witness beside the authored declaration.
/// `reader` names an executed reader, never a guessed parser capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredReadProvenance {
    pub identity: String,
    pub disposition: DeclaredSourceDisposition,
    pub witness: Option<SourceFileWitness>,
    pub reader: Option<&'static str>,
}

/// An optional ordinary view may fail without losing raw bytes or native facts.
/// ```
/// use vibe_facts::OrdinaryStructureError;
/// assert_eq!(OrdinaryStructureError::InvalidUtf8, OrdinaryStructureError::InvalidUtf8);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryStructureError {
    InvalidUtf8,
}

mod export;
pub use export::{DeclaredExportError, DeclaredExportResolution, resolve_declared_export};

/// One classified read. Raw/unsupported results retain the exact supplied
/// bytes; neither they nor ambiguous/unclassified reads manufacture facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclaredDocumentObservation<'a> {
    Unavailable {
        provenance: DeclaredReadProvenance,
    },
    Unclassified {
        provenance: DeclaredReadProvenance,
        bytes: &'a [u8],
    },
    NeedsInput {
        provenance: DeclaredReadProvenance,
        bytes: &'a [u8],
    },
    Raw {
        provenance: DeclaredReadProvenance,
        bytes: &'a [u8],
    },
    /// Structure is optional; both outcomes retain the same original bytes.
    OrdinaryMarkdown {
        provenance: DeclaredReadProvenance,
        bytes: &'a [u8],
        structure: Result<vibe_specdoc::commonmark::CommonMarkDoc, OrdinaryStructureError>,
    },
    Unsupported {
        provenance: DeclaredReadProvenance,
        bytes: &'a [u8],
        reason: &'static str,
    },
    Invalid {
        provenance: DeclaredReadProvenance,
        bytes: &'a [u8],
        reason: String,
    },
    Facts {
        provenance: DeclaredReadProvenance,
        facts: Vec<AuthoredFact>,
    },
}

/// Classify one normalized package-relative path and consume the exact bytes
/// supplied by its host. `None` is unavailable, never an empty source. Only a
/// selected native interpretation invokes the existing specdoc parser and
/// authored-fact traversal; XML and Markdown dispatch follow declared media.
pub fn observe_declared_document<'a>(
    package: &PackageMeta,
    relative_path: &str,
    bytes: Option<&'a [u8]>,
) -> Result<DeclaredDocumentObservation<'a>, SourceSelectionError> {
    let coordinate = format!("{}/{}", package.group, package.name);
    observe_source_document(
        &coordinate,
        package.sources.as_deref().unwrap_or(&[]),
        package.kind == PackageKind::Doc,
        relative_path,
        bytes,
    )
}

/// Role-neutral pure adapter; actual caller metadata decides Doc eligibility.
pub(super) fn observe_source_document<'a>(
    coordinate: &str,
    rows: &[SourceDescriptor],
    doc_allowed: bool,
    relative_path: &str,
    bytes: Option<&'a [u8]>,
) -> Result<DeclaredDocumentObservation<'a>, SourceSelectionError> {
    let selected = vibe_core::manifest::select_source_rows(coordinate, rows, relative_path)?;
    let (identity, disposition) = match selected {
        SelectedSource::Unclassified { identity } => {
            (identity, DeclaredSourceDisposition::Unclassified)
        }
        SelectedSource::NeedsInput {
            identity,
            declaration_indices,
        } => (
            identity,
            DeclaredSourceDisposition::NeedsInput {
                declaration_indices,
            },
        ),
        SelectedSource::Descriptor {
            identity,
            declaration_index,
            descriptor,
        } => (
            identity,
            DeclaredSourceDisposition::Selected {
                declaration_index,
                descriptor: descriptor.clone(),
            },
        ),
    };
    let mut provenance = DeclaredReadProvenance {
        identity,
        disposition,
        witness: bytes.map(|raw| SourceFileWitness::of(relative_path, raw)),
        reader: None,
    };
    let Some(raw) = bytes else {
        return Ok(DeclaredDocumentObservation::Unavailable { provenance });
    };
    let DeclaredSourceDisposition::Selected { descriptor, .. } = &provenance.disposition else {
        return Ok(match &provenance.disposition {
            DeclaredSourceDisposition::Unclassified => DeclaredDocumentObservation::Unclassified {
                provenance,
                bytes: raw,
            },
            DeclaredSourceDisposition::NeedsInput { .. } => {
                DeclaredDocumentObservation::NeedsInput {
                    provenance,
                    bytes: raw,
                }
            }
            DeclaredSourceDisposition::Selected { .. } => unreachable!(),
        });
    };
    let interpretation = descriptor.interpretation();
    let media = descriptor.media();
    if interpretation == SourceInterpretation::VibeDoc && !doc_allowed {
        return Ok(DeclaredDocumentObservation::Unsupported {
            provenance,
            bytes: raw,
            reason: "documentation vocabulary requires a doc package",
        });
    }
    match interpretation {
        SourceInterpretation::Commonmark => {
            let structure = std::str::from_utf8(raw)
                .map(vibe_specdoc::commonmark::CommonMarkDoc::parse)
                .map_err(|_| OrdinaryStructureError::InvalidUtf8);
            if structure.is_ok() {
                provenance.reader = Some("vibe-specdoc:commonmark");
            }
            return Ok(DeclaredDocumentObservation::OrdinaryMarkdown {
                provenance,
                bytes: raw,
                structure,
            });
        }
        SourceInterpretation::Opaque | SourceInterpretation::XmlData => {
            return Ok(DeclaredDocumentObservation::Raw {
                provenance,
                bytes: raw,
            });
        }
        SourceInterpretation::VibeSpec | SourceInterpretation::VibeDoc => {}
    }
    let (reader, vocabulary) = match (interpretation, media) {
        (SourceInterpretation::VibeSpec, "application/xml") => {
            ("vibe-specdoc:xml/spec", Vocabulary::Spec)
        }
        (SourceInterpretation::VibeSpec, "text/markdown") => {
            ("vibe-specdoc:markdown/spec", Vocabulary::Spec)
        }
        (SourceInterpretation::VibeDoc, "application/xml") => {
            ("vibe-specdoc:xml/doc", Vocabulary::Doc)
        }
        _ => {
            return Ok(DeclaredDocumentObservation::Unsupported {
                provenance,
                bytes: raw,
                reason: "declared media has no native reader",
            });
        }
    };
    let text = match std::str::from_utf8(raw) {
        Ok(text) => text,
        Err(error) => {
            return Ok(DeclaredDocumentObservation::Invalid {
                provenance,
                bytes: raw,
                reason: format!("invalid UTF-8: {error}"),
            });
        }
    };
    provenance.reader = Some(reader);
    let document = parse_native_document(text, media, vocabulary);
    let document = match document {
        Ok(document) => document,
        Err(error) => {
            return Ok(DeclaredDocumentObservation::Invalid {
                provenance,
                bytes: raw,
                reason: error.to_string(),
            });
        }
    };
    if relative_path.contains('#') {
        return Ok(DeclaredDocumentObservation::Unsupported {
            provenance,
            bytes: raw,
            reason: "native document address cannot represent a literal fragment separator",
        });
    }
    let prefix = match address_prefix(coordinate, relative_path) {
        Ok(prefix) => prefix,
        Err(error) => {
            return Ok(DeclaredDocumentObservation::Invalid {
                provenance,
                bytes: raw,
                reason: error.to_string(),
            });
        }
    };
    Ok(DeclaredDocumentObservation::Facts {
        provenance,
        facts: crate::authored_facts(&document, &prefix),
    })
}

#[cfg(test)]
#[path = "declared/data_tests.rs"]
mod data_tests;

/// The native facts pivot dispatch, shared by original and installed readers.
pub(super) fn parse_native_document(
    text: &str,
    media: &str,
    vocabulary: Vocabulary,
) -> Result<vibe_specdoc::doc::SpecDoc, vibe_specdoc::Error> {
    if media == "application/xml" {
        vibe_specdoc::from_xml_with(text, vocabulary)
    } else {
        vibe_specdoc::from_markdown(text)
    }
}
