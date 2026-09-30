//! Pure resolution of one authored name against one captured Markdown snapshot.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-EXPORT-SELECTOR");

use std::ops::Range;

use sha2::{Digest, Sha256};

use super::{CommonMarkDoc, NodeId, NodeKind, normalized};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedExport {
    /// Package-unique authored name, not a Vibe fact identifier.
    pub name: String,
    /// Arena-local identity valid only for this parsed snapshot.
    pub node: NodeId,
    /// `sha256:<hex>` of exactly the UTF-8 bytes this snapshot parsed.
    pub source_sha256: String,
    /// Exact section byte span within those bytes.
    pub span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportResolutionError {
    MalformedName,
    MalformedHeadings,
    Missing { index: usize, title: String },
    Ambiguous { index: usize, title: String },
}

impl CommonMarkDoc {
    /// Resolve a validated manifest row without reading a file or importing
    /// manifest types. The host still must select the CommonMark descriptor,
    /// capture bytes and bind the returned witness to its resource identity.
    pub fn resolve_export(
        &self,
        name: &str,
        headings: &[String],
    ) -> Result<ResolvedExport, ExportResolutionError> {
        if !valid_name(name) {
            return Err(ExportResolutionError::MalformedName);
        }
        if headings.is_empty() || headings.iter().any(|title| !valid_title(title)) {
            return Err(ExportResolutionError::MalformedHeadings);
        }
        let mut parent = self.root();
        for (index, title) in headings.iter().enumerate() {
            let mut found = None;
            for &child in self.children(parent) {
                if matches!(&self.node(child).kind, NodeKind::Section { title: found_title, .. } if found_title == title)
                {
                    if found.replace(child).is_some() {
                        return Err(ExportResolutionError::Ambiguous {
                            index,
                            title: title.clone(),
                        });
                    }
                }
            }
            parent = found.ok_or_else(|| ExportResolutionError::Missing {
                index,
                title: title.clone(),
            })?;
        }
        Ok(ResolvedExport {
            name: name.to_owned(),
            node: parent,
            source_sha256: format!("sha256:{:x}", Sha256::digest(self.source().as_bytes())),
            span: self.node(parent).span.clone(),
        })
    }
}

fn valid_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.first().is_some_and(u8::is_ascii_alphabetic)
        && bytes[1..]
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(*b, b'.' | b'_' | b'-'))
}

fn valid_title(title: &str) -> bool {
    !title.is_empty() && !title.chars().any(char::is_control) && normalized(title) == title
}
