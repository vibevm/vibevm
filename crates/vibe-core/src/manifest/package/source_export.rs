//! Authored names for ordinary CommonMark sections.
//!
//! A row selects a logical source and a heading path. It proves neither that
//! the source exists nor that its bytes have been captured or interpreted.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-EXPORT-SELECTOR");

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::manifest::declarant_path;

/// One validated `[[package.source_exports]]` declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "SourceExportWire", into = "SourceExportWire")]
pub struct SourceExport {
    name: String,
    file: String,
    headings: Vec<String>,
}

impl SourceExport {
    pub fn new(name: String, file: String, headings: Vec<String>) -> Result<Self, String> {
        validate_name(&name)?;
        if declarant_path(Path::new(&file)).ok() != Some(file.as_str()) {
            return Err(
                "source export `file` must be one normalized exact package-relative file path"
                    .into(),
            );
        }
        if headings.is_empty() || headings.iter().any(|title| !valid_title(title)) {
            return Err("source export `headings` must be a nonempty root-to-section array of normalized titles".into());
        }
        Ok(Self {
            name,
            file,
            headings,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn file(&self) -> &str {
        &self.file
    }
    pub fn headings(&self) -> &[String] {
        &self.headings
    }
}

fn validate_name(name: &str) -> Result<(), String> {
    let bytes = name.as_bytes();
    if bytes.first().is_none_or(|b| !b.is_ascii_alphabetic())
        || bytes[1..]
            .iter()
            .any(|b| !b.is_ascii_alphanumeric() && !matches!(*b, b'.' | b'_' | b'-'))
    {
        return Err("source export `name` must match [A-Za-z][A-Za-z0-9._-]*".into());
    }
    Ok(())
}

fn valid_title(title: &str) -> bool {
    !title.is_empty()
        && !title.chars().any(char::is_control)
        && title.split_whitespace().collect::<Vec<_>>().join(" ") == title
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceExportWire {
    name: String,
    file: String,
    headings: Vec<String>,
}

impl TryFrom<SourceExportWire> for SourceExport {
    type Error = String;

    fn try_from(value: SourceExportWire) -> Result<Self, Self::Error> {
        Self::new(value.name, value.file, value.headings)
    }
}

impl From<SourceExport> for SourceExportWire {
    fn from(value: SourceExport) -> Self {
        Self {
            name: value.name,
            file: value.file,
            headings: value.headings,
        }
    }
}
