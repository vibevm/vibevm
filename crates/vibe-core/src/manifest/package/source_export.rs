//! Named ordinary sections; authored declarations are not byte observations.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-EXPORT-SELECTOR");
use serde::{Deserialize, Serialize};

/// One strict authored export, resolved only against selected captured CommonMark.
/// ```
/// use vibe_core::manifest::SourceExport;
/// let row = SourceExport::new("intro.v1".into(), "docs/guide.md".into(), vec!["Guide".into()])?;
/// assert_eq!(row.name(), "intro.v1");
/// # Ok::<(), String>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "SourceExportWire", into = "SourceExportWire")]
pub struct SourceExport {
    name: String,
    file: String,
    headings: Vec<String>,
}
impl SourceExport {
    pub fn new(name: String, file: String, headings: Vec<String>) -> Result<Self, String> {
        let bytes = name.as_bytes();
        if !bytes.first().is_some_and(u8::is_ascii_alphabetic)
            || !bytes
                .iter()
                .skip(1)
                .all(|b| b.is_ascii_alphanumeric() || matches!(*b, b'.' | b'_' | b'-'))
        {
            return Err("source export name must match [A-Za-z][A-Za-z0-9._-]*".into());
        }
        crate::manifest::source_descriptor::valid_source_path(&file)?;
        if headings.is_empty()
            || headings.iter().any(|title| {
                title.is_empty()
                    || title.chars().any(char::is_control)
                    || title.split_whitespace().collect::<Vec<_>>().join(" ") != *title
            })
        {
            return Err("source export headings must be a nonempty root-to-section array of normalized titles".into());
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceExportWire {
    name: String,
    file: String,
    headings: Vec<String>,
}
impl TryFrom<SourceExportWire> for SourceExport {
    type Error = String;
    fn try_from(row: SourceExportWire) -> Result<Self, String> {
        Self::new(row.name, row.file, row.headings)
    }
}
impl From<SourceExport> for SourceExportWire {
    fn from(row: SourceExport) -> Self {
        Self {
            name: row.name,
            file: row.file,
            headings: row.headings,
        }
    }
}

pub(super) fn validate_unique(rows: &[SourceExport]) -> Result<(), String> {
    let mut names = std::collections::BTreeSet::new();
    for row in rows {
        if !names.insert(row.name()) {
            return Err("duplicate source export name".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
