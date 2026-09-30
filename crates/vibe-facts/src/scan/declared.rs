//! One descriptor-selected, already captured file at the authored-facts seam.
//!
//! The incumbent tree walker is still a legacy suffix consumer. This bounded
//! read path makes its authority explicit before a later routing change.

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
    CapturedDeclaredDocument, DeclaredTreeObservation, DeclaredWalkError, observe_declared_source,
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
    let selected = package.select_source(relative_path)?;
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
    if interpretation == SourceInterpretation::VibeDoc && package.kind != PackageKind::Doc {
        return Ok(DeclaredDocumentObservation::Unsupported {
            provenance,
            bytes: raw,
            reason: "documentation vocabulary requires a doc package",
        });
    }
    match interpretation {
        SourceInterpretation::Opaque | SourceInterpretation::XmlData => {
            return Ok(DeclaredDocumentObservation::Raw {
                provenance,
                bytes: raw,
            });
        }
        SourceInterpretation::Commonmark => {
            return Ok(DeclaredDocumentObservation::Unsupported {
                provenance,
                bytes: raw,
                reason: "ordinary CommonMark structural reader is not wired at this seam",
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
    let document = if media == "application/xml" {
        vibe_specdoc::from_xml_with(text, vocabulary)
    } else {
        vibe_specdoc::from_markdown(text)
    };
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
    let coordinate = format!("{}/{}", package.group, package.name);
    let prefix = match address_prefix(&coordinate, relative_path) {
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
mod tests;
