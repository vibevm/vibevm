//! Host-validated descriptor output map used only for declared Vibe documents.
//!
//! These values are not a filesystem grant. The workspace owner supplies rows
//! after checking the installed manifest, slot record and output bytes; this
//! module checks their logical shape and never infers interpretation from a
//! filename suffix.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use vibe_core::manifest::SourceInterpretation;

use super::{ResolveError, ResolvedSourceFile, lookup};

/// One witnessed materializer output and the original source that produced it.
/// `source_witness` is the exact *package* source hash, not an original-file
/// digest: converted source bytes are not retained in today's slot record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescriptorOutputRow {
    pub output: String,
    pub source: String,
    /// `None` is an unclassified descriptor-owned resource, still raw.
    pub interpretation: Option<SourceInterpretation>,
    pub output_sha256: String,
    pub document_address: Option<String>,
}

/// Logically checked map whose physical byte checks belong to the workspace
/// reader. `spec_root` is the exact root selected by the resolver, not a
/// directory rediscovered by this type.
#[derive(Debug, Clone)]
pub struct ValidatedDescriptorMap {
    slot_root: PathBuf,
    spec_root: PathBuf,
    source_witness: String,
    manifest_sha256: String,
    by_address: BTreeMap<String, DescriptorOutputRow>,
}

impl ValidatedDescriptorMap {
    /// Check every row and index only author-selected Vibe interpretations.
    /// The caller must have proved the manifest, source hash and output bytes.
    pub fn from_witnessed_rows(
        slot_root: PathBuf,
        spec_root: PathBuf,
        source_witness: String,
        manifest_sha256: String,
        rows: Vec<DescriptorOutputRow>,
    ) -> Result<Self, ResolveError> {
        if !slot_root.is_absolute()
            || !spec_root.is_absolute()
            || !spec_root.starts_with(&slot_root)
            || source_witness.is_empty()
            || !sha256(&manifest_sha256)
        {
            return Err(invalid(
                "root, package witness or manifest digest is invalid",
            ));
        }
        let mut outputs = BTreeSet::new();
        let mut folded_outputs = BTreeMap::new();
        let mut by_address = BTreeMap::new();
        for row in rows {
            if !relative(&row.output) || !relative(&row.source) || !sha256(&row.output_sha256) {
                return Err(invalid("output/source path or output digest is invalid"));
            }
            if !outputs.insert(row.output.clone()) {
                return Err(invalid("two source rows claim one output path"));
            }
            let folded = row.output.to_lowercase();
            if folded_outputs
                .insert(folded, row.output.clone())
                .is_some_and(|prior| prior != row.output)
            {
                return Err(invalid(
                    "two source rows claim portable-case output aliases",
                ));
            }
            let is_vibe = matches!(
                row.interpretation,
                Some(SourceInterpretation::VibeSpec | SourceInterpretation::VibeDoc)
            );
            match (is_vibe, row.document_address.as_deref()) {
                (false, None) => continue,
                (false, Some(_)) => {
                    return Err(invalid("raw resource claims a Vibe document address"));
                }
                (true, None) => {
                    return Err(invalid("selected Vibe source has no document address"));
                }
                (true, Some(address)) => {
                    if serialization(&row.output).is_none() {
                        return Err(invalid("selected Vibe output is not Markdown or XML"));
                    }
                    if address.is_empty()
                        || address != lookup::canonical_doc_path(&row.source)
                        || address != lookup::canonical_doc_path(&row.output)
                    {
                        return Err(invalid(
                            "Vibe document address differs from its source/output",
                        ));
                    }
                    if by_address.insert(address.to_owned(), row).is_some() {
                        return Err(invalid(
                            "two selected Vibe forms claim one document address",
                        ));
                    }
                }
            }
        }
        Ok(Self {
            slot_root,
            spec_root,
            source_witness,
            manifest_sha256,
            by_address,
        })
    }

    pub fn spec_root(&self) -> &Path {
        &self.spec_root
    }

    pub fn source_witness(&self) -> &str {
        &self.source_witness
    }

    pub fn manifest_sha256(&self) -> &str {
        &self.manifest_sha256
    }

    pub(super) fn resolve(&self, doc_path: &str) -> Option<ResolvedSourceFile> {
        let row = self.by_address.get(doc_path)?;
        let path = self.slot_root.join(&row.output);
        let physical_stem = Path::new(&row.output)
            .file_stem()
            .and_then(|part| part.to_str())?
            .to_owned();
        Some(ResolvedSourceFile {
            path,
            // `format` names the compiler frontend. Both admitted Vibe disk
            // serializations enter the built-in Markdown projection reader.
            format: "markdown".to_owned(),
            physical_stem,
        })
    }
}

/// A suffix chooses only the serialization of an already selected Vibe row.
/// Raw resources never call this function and never acquire Vibe semantics.
fn serialization(output: &str) -> Option<&'static str> {
    match Path::new(output).extension().and_then(|part| part.to_str()) {
        Some("md") => Some("markdown"),
        Some("xml") => Some("xml"),
        _ => None,
    }
}

fn invalid(reason: &str) -> ResolveError {
    ResolveError::DescriptorMapInvalid {
        reason: reason.to_owned(),
    }
}

fn sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn relative(value: &str) -> bool {
    if value.is_empty() || value.contains('\\') || value.contains('\0') || value.starts_with('/') {
        return false;
    }
    let bytes = value.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return false;
    }
    !value
        .split('/')
        .any(|part| part.is_empty() || matches!(part, "." | ".."))
        && Path::new(value)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}
