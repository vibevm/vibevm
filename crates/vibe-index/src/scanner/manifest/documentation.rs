//! The documentation and localisation surface a manifest declares,
//! projected onto the wire (PROP-057 `##REL-INDEX-FIELDS`): the available
//! languages, the documents a package ships, what it documents, what it
//! translates, and its media.
//!
//! Every builder here normalises emptiness to absence, exactly as the
//! relation builders beside it do: the writer never emits a
//! present-but-empty section, so a reader never has to tell `{}` from a
//! missing key.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use std::path::Path;

use vibe_core::manifest::i18n::I18nDecl;
use vibe_core::manifest::{DocumentationDecl, DocumentsDecl, MediaDecl, TranslatesDecl};

use crate::types::{DocumentationEntry, DocumentsEntry, I18nEntry, MediaEntry, TranslatesEntry};

pub fn i18n_from(i: &I18nDecl) -> Option<I18nEntry> {
    let entry = I18nEntry {
        available: i.available.clone(),
        default: Some(i.canonical.clone()),
    };
    (!entry.is_empty()).then_some(entry)
}

/// Project `[[documents]]` — the subjects a `doc` package documents
/// (PROP-057 `##REL-INDEX-FIELDS`). The list rides into the index so
/// that «who documents X» is a fold over `primary.jsonl` and never a
/// download (`##REL-REVERSE-QUERIES-SITE-SIDE`); an empty list is
/// absence, like every sibling here.
pub fn documents_from(list: &[DocumentsDecl]) -> Vec<DocumentsEntry> {
    list.iter()
        .map(|d| DocumentsEntry {
            package: d.package.clone(),
            version: d.version.clone(),
        })
        .collect()
}

/// Project `[documentation]` — the other end of the same edge, written
/// by the SUBJECT and legal in a package of any kind. Officiality is the
/// convergence of the two ends, computed at render time, so nothing here
/// is a stored flag (PROP-057 `##REL-NO-OFFICIAL-FLAG`).
pub fn documentation_from(d: &Option<DocumentationDecl>) -> Option<DocumentationEntry> {
    let entry = DocumentationEntry {
        primary: d.as_ref().and_then(|d| d.primary.clone()),
        official: d.as_ref().map(|d| d.official.clone()).unwrap_or_default(),
    };
    (!entry.is_empty()).then_some(entry)
}

/// Project `[translates]` — the source this adaptation follows. The
/// source stores no list of its translations, so this edge is the only
/// one there is and the language selector is built by folding it
/// (PROP-057 `##LOC-NO-TRANSLATIONS-TABLE`).
pub fn translates_from(t: &Option<TranslatesDecl>) -> Option<TranslatesEntry> {
    t.as_ref().map(|t| TranslatesEntry {
        package: t.package.clone(),
        version: t.version.clone(),
    })
}

/// Project `[media]` — the card's images as package-relative paths.
/// Separators are normalised to `/` for the same reason `boot_snippet`
/// normalises its own: the wire is one path grammar, not the scanning
/// host's.
pub fn media_from(m: &Option<MediaDecl>) -> Option<MediaEntry> {
    let entry = MediaEntry {
        icon: m.as_ref().and_then(|m| m.icon.as_deref()).map(wire_path),
        banner: m.as_ref().and_then(|m| m.banner.as_deref()).map(wire_path),
        preview: m.as_ref().and_then(|m| m.preview.as_deref()).map(wire_path),
    };
    (!entry.is_empty()).then_some(entry)
}

/// A package-relative path as the wire spells it: `/` separators
/// whatever the scanning host uses.
fn wire_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
