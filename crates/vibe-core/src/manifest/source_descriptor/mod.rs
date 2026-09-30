//! Authored source interpretation, selected before any parser sees bytes.
//!
//! This is a pure manifest declaration and selector. It neither reads source
//! content nor grants trust to an authored statement.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-DESCRIPTOR-GRAMMAR");

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{PackageMeta, declarant_path};
use crate::layout;

/// The exact file or directory default named by one authored row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceSelector {
    File(String),
    Directory(String),
}

impl SourceSelector {
    pub fn path(&self) -> &str {
        match self {
            Self::File(path) | Self::Directory(path) => path,
        }
    }
}

/// The semantic reader selected by the author, never inferred from a suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceInterpretation {
    VibeSpec,
    VibeDoc,
    Commonmark,
    XmlData,
    Opaque,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceRole {
    Spec,
    Documentation,
    Resource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceLoading {
    Scan,
    Explicit,
}

/// Validated authored interpretation. The host supplies separate provenance
/// for the selected declaration and witnessed source bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "SourceDescriptorWire", into = "SourceDescriptorWire")]
pub struct SourceDescriptor {
    selector: SourceSelector,
    media: String,
    interpretation: SourceInterpretation,
    role: SourceRole,
    loading: SourceLoading,
}

impl SourceDescriptor {
    pub fn selector(&self) -> &SourceSelector {
        &self.selector
    }

    pub fn media(&self) -> &str {
        &self.media
    }

    pub fn interpretation(&self) -> SourceInterpretation {
        self.interpretation
    }

    pub fn role(&self) -> SourceRole {
        self.role
    }

    pub fn loading(&self) -> SourceLoading {
        self.loading
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceDescriptorWire {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    directory: Option<String>,
    media: String,
    interpretation: SourceInterpretation,
    role: SourceRole,
    loading: SourceLoading,
}

impl TryFrom<SourceDescriptorWire> for SourceDescriptor {
    type Error = String;

    fn try_from(value: SourceDescriptorWire) -> Result<Self, Self::Error> {
        let selector = match (value.file, value.directory) {
            (Some(path), None) => SourceSelector::File(path),
            (None, Some(path)) => SourceSelector::Directory(path),
            _ => return Err("source row requires exactly one `file` or `directory`".into()),
        };
        valid_source_path(selector.path())?;
        if !valid_media(&value.media) {
            return Err("source `media` must be a normalized media type".into());
        }
        let valid_pair = match (value.interpretation, value.role) {
            (SourceInterpretation::VibeSpec, SourceRole::Spec) => {
                matches!(value.media.as_str(), "text/markdown" | "application/xml")
            }
            (SourceInterpretation::VibeDoc, SourceRole::Documentation) => {
                value.media == "application/xml"
            }
            (
                SourceInterpretation::Commonmark,
                SourceRole::Documentation | SourceRole::Resource,
            ) => value.media == "text/markdown",
            (SourceInterpretation::XmlData, SourceRole::Resource) => {
                matches!(value.media.as_str(), "application/xml" | "text/xml")
                    || value.media.ends_with("+xml")
            }
            (SourceInterpretation::Opaque, SourceRole::Resource | SourceRole::Documentation) => {
                true
            }
            _ => false,
        };
        if !valid_pair {
            return Err("source media, interpretation and role are incompatible".into());
        }
        Ok(Self {
            selector,
            media: value.media,
            interpretation: value.interpretation,
            role: value.role,
            loading: value.loading,
        })
    }
}

impl From<SourceDescriptor> for SourceDescriptorWire {
    fn from(value: SourceDescriptor) -> Self {
        let (file, directory) = match value.selector {
            SourceSelector::File(path) => (Some(path), None),
            SourceSelector::Directory(path) => (None, Some(path)),
        };
        Self {
            file,
            directory,
            media: value.media,
            interpretation: value.interpretation,
            role: value.role,
            loading: value.loading,
        }
    }
}

fn valid_media(media: &str) -> bool {
    let Some((kind, subtype)) = media.split_once('/') else {
        return false;
    };
    !kind.is_empty()
        && !subtype.is_empty()
        && !subtype.contains('/')
        && [kind, subtype].iter().all(|part| {
            part.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'.' | b'-' | b'+' | b'_')
            })
        })
}

fn generated_output(path: &str) -> bool {
    [
        layout::current_boot_index(),
        layout::current_boot_static_md(),
        layout::current_boot_static_xml(),
    ]
    .iter()
    .any(|generated| path.eq_ignore_ascii_case(&generated.to_string_lossy()))
}

fn valid_source_path(path: &str) -> Result<(), String> {
    declarant_path(Path::new(path))
        .map_err(|fault| format!("source path `{path}`: {}", fault.reason()))?;
    if generated_output(path) {
        return Err(format!(
            "source path `{path}` names a generated boot output"
        ));
    }
    Ok(())
}

/// Pure selection result. `NeedsInput` names the exact logical resource and
/// the conflicting declaration rows in their authored order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectedSource<'a> {
    Unclassified {
        identity: String,
    },
    Descriptor {
        identity: String,
        declaration_index: usize,
        descriptor: &'a SourceDescriptor,
    },
    NeedsInput {
        identity: String,
        declaration_indices: Vec<usize>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceSelectionError {
    InvalidPath(String),
}

impl PackageMeta {
    /// Resolve one normalized package-relative file. No suffix or content is
    /// consulted; package format and boot linkage remain independent.
    pub fn select_source(&self, file: &str) -> Result<SelectedSource<'_>, SourceSelectionError> {
        valid_source_path(file).map_err(SourceSelectionError::InvalidPath)?;
        let identity = format!("{}/{}:{file}", self.group, self.name);
        let mut best: Option<(u8, usize)> = None;
        let mut winners = Vec::new();
        for (index, descriptor) in self.sources.iter().enumerate() {
            let specificity = match descriptor.selector() {
                SourceSelector::File(path) if path == file => Some((1, path.split('/').count())),
                SourceSelector::Directory(path)
                    if file
                        .strip_prefix(path)
                        .is_some_and(|suffix| suffix.starts_with('/')) =>
                {
                    Some((0, path.split('/').count()))
                }
                _ => None,
            };
            if let Some(specificity) = specificity {
                if best.is_none_or(|previous| specificity > previous) {
                    best = Some(specificity);
                    winners.clear();
                }
                if best == Some(specificity) {
                    winners.push(index);
                }
            }
        }
        let Some(&first) = winners.first() else {
            return Ok(SelectedSource::Unclassified { identity });
        };
        if winners
            .iter()
            .any(|index| self.sources[*index] != self.sources[first])
        {
            return Ok(SelectedSource::NeedsInput {
                identity,
                declaration_indices: winners,
            });
        }
        Ok(SelectedSource::Descriptor {
            identity,
            declaration_index: first,
            descriptor: &self.sources[first],
        })
    }
}

#[cfg(test)]
mod tests;
