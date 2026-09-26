//! The glossary a documentation declares — the entity, its entries, and
//! what a defect of it is (PROP-057 `##GLOSSARY-DECLARED`,
//! `##GLOSSARY-CHECKED`, `##GLOSSARY-TRANSLATION`).
//!
//! ## Why this is an entity and not a path
//!
//! It used to be a constant in the style linter: `GLOSSARY_PAGE =
//! "glossary/index.xml"`. That worked for exactly one documentation — the
//! one whose author had read the constant. Nothing in a manifest said which
//! page defined the terms, so every other tool that wanted them would have
//! had to guess the same string, and any other manual with a glossary
//! somewhere else had none as far as the machine was concerned.
//!
//! So a documentation DECLARES its glossary, in one line of its manifest,
//! and everything done with terms is done with the declared one: the style
//! checks of `##STYLE-LINT`, and the reader's cards
//! (`##READER-GLOSSARY-CARD`). A package that declares none has no
//! glossary, and the path `glossary/index.xml` means nothing by itself
//! (`##GLOSSARY-CARD-DECISION`).
//!
//! ## What an entry is
//!
//! Each TOP-LEVEL section of the glossary page is one entry: the section's
//! id is the anchor a link names, its `title` is the term, and its FIRST
//! paragraph is the definition. More paragraphs and cited rules may follow
//! and are not the definition — a card shows one paragraph, and an entry
//! that needed two of them to say what a word means is an entry that has
//! not been written yet.
//!
//! The definition travels as the page wrote it: inline Markdown, resolved
//! by whoever renders it. This module reads structure and never renders —
//! the island's `html::inline` decides what a code span and a link are,
//! once, and a second reading here would disagree with the glossary page's
//! own rendering of the same characters.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#GLOSSARY-DECLARED");

use std::path::Path;

use vibe_specdoc::doc::{Block, SpecDoc};

use crate::error::{DocError, Result};
use crate::pages::{PageSet, document_of};

/// The manifest key a documentation declares its glossary under.
pub const GLOSSARY: &str = "glossary";

/// One entry of a glossary: a term, its anchor and the one paragraph that
/// defines it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The anchor a link must name — the section's id, `lock-file`.
    pub id: String,
    /// The term as the glossary titles it, in this edition's language.
    pub term: String,
    /// The first paragraph of the entry, as the page wrote it: inline
    /// Markdown, never rendered here.
    pub definition: String,
}

/// The glossary of one documentation package, read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Glossary {
    /// The declared document path — `glossary/index`, without the
    /// extension, exactly as the manifest spells it and as a link target
    /// resolves to.
    pub document: String,
    /// The entries in the order the page states them.
    pub entries: Vec<Entry>,
}

impl Glossary {
    /// The entry an anchor names, when the glossary has one.
    pub fn entry(&self, id: &str) -> Option<&Entry> {
        self.entries.iter().find(|entry| entry.id == id)
    }
}

/// One way a declared glossary is not usable (`##GLOSSARY-CHECKED`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Defect {
    /// The manifest names a page the package does not carry.
    MissingPage { document: String },
    /// The glossary page would not parse, so its entries cannot be read.
    UnreadablePage { document: String },
    /// A top-level section of the glossary page carries no `title`, so the
    /// entry names no term.
    EntryWithoutTerm { document: String, id: String },
    /// An entry does not open with a paragraph, so it states no
    /// definition.
    EntryWithoutDefinition { document: String, term: String },
}

impl Defect {
    /// The line a report prints, in the words of what is wrong and what to
    /// do about it.
    pub fn render(&self) -> String {
        match self {
            Defect::MissingPage { document } => format!(
                "[glossary].page names `{document}` and this package carries no page there — \
                 a glossary nobody can open defines nothing, and every link to a term in it \
                 is a dead link"
            ),
            Defect::UnreadablePage { document } => format!(
                "the glossary page `{document}` does not parse, so its terms cannot be read; \
                 fix the page and the glossary comes back with it"
            ),
            Defect::EntryWithoutTerm { document, id } => format!(
                "the glossary entry `{document}#{id}` carries no `title` — the title IS the \
                 term, and an entry without one defines a word nobody can name"
            ),
            Defect::EntryWithoutDefinition { document, term } => format!(
                "the glossary entry for `{term}` on `{document}` does not open with a \
                 paragraph — the first paragraph is the definition, and it is what a reader \
                 is shown in place"
            ),
        }
    }
}

/// The document path a package declares as its glossary, or `None` when it
/// declares none.
///
/// The narrow door beside [`crate::manifest::relations`] and
/// [`crate::manifest::chapters`], and for the same reason: a caller that
/// wants one line of a manifest should not have to build a card first, or a
/// missing `title` in the source would look like a glossary defect.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// std::fs::write(
///     dir.path().join("vibe.toml"),
///     "[glossary]\npage = \"glossary/index\"\n",
/// )
/// .unwrap();
///
/// assert_eq!(
///     vibe_doc::glossary::declared(dir.path()).unwrap().as_deref(),
///     Some("glossary/index")
/// );
/// ```
pub fn declared(package_dir: &Path) -> Result<Option<String>> {
    let path = package_dir.join(crate::derived::manifest::MANIFEST);
    let text = std::fs::read_to_string(&path).map_err(|e| DocError::io("reading", &path, e))?;
    let parsed: toml::Value = toml::from_str(&text)
        .map_err(|e| DocError::manifest(&path, format!("does not parse: {e}")))?;
    Ok(parsed
        .get(GLOSSARY)
        .and_then(|table| table.get("page"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned))
}

/// The declared glossary of a package whose pages are already read, or
/// `None` when the package declares none or the page is not there.
///
/// A missing page is `None` and not an error, for the reason a renderer
/// resolves nothing it cannot fetch: a build renders every page it has and
/// the CHECK is where a declaration pointing at nothing is a failure
/// ([`check`], `##GLOSSARY-CHECKED`).
pub fn read(package_dir: &Path, set: &PageSet) -> Result<Option<Glossary>> {
    let Some(document) = declared(package_dir)? else {
        return Ok(None);
    };
    let Some(page) = set.pages.iter().find(|p| document_of(&p.rel) == document) else {
        return Ok(None);
    };
    Ok(Some(Glossary {
        document,
        entries: entries(&page.doc),
    }))
}

/// The entries one glossary page states, in its own order.
///
/// A section with no id is skipped: the anchor is what a link names, and an
/// entry nothing can address is an entry nothing can show. A section with
/// no first paragraph yields an entry with an empty definition rather than
/// none, so a reader still meets the term and the check still has something
/// to name.
pub fn entries(doc: &SpecDoc) -> Vec<Entry> {
    doc.sections
        .iter()
        .filter_map(|section| {
            Some(Entry {
                id: section.id.clone()?,
                term: section.title.trim().to_owned(),
                definition: definition(section).unwrap_or_default(),
            })
        })
        .collect()
}

/// The first paragraph of an entry — its definition — when it opens with
/// one.
fn definition(section: &vibe_specdoc::doc::Section) -> Option<String> {
    match &section.blocks.first()?.block {
        Block::Paragraph(unit) => Some(unit.text.trim().to_owned()),
        _ => None,
    }
}

/// A declared glossary against the package that declared it
/// (`##GLOSSARY-CHECKED`).
///
/// Three findings and one idea between them: a declaration a reader can
/// follow. The page has to be there, every entry has to name its term, and
/// every entry has to state a definition — because the definition is what
/// the reader is shown in place, and an entry that opens with a table shows
/// nothing.
///
/// A package that declares no glossary yields nothing: a documentation
/// without one is not a documentation with a broken one, and that is every
/// manual written before the table existed.
pub fn check(package_dir: &Path, set: &PageSet) -> Result<Vec<Defect>> {
    let Some(document) = declared(package_dir)? else {
        return Ok(Vec::new());
    };
    if let Some(bad) = set
        .unreadable
        .iter()
        .find(|p| document_of(&p.rel) == document)
    {
        return Ok(vec![Defect::UnreadablePage {
            document: document_of(&bad.rel).to_owned(),
        }]);
    }
    let Some(page) = set.pages.iter().find(|p| document_of(&p.rel) == document) else {
        return Ok(vec![Defect::MissingPage { document }]);
    };
    let mut out: Vec<Defect> = Vec::new();
    for (at, section) in page.doc.sections.iter().enumerate() {
        if section.title.trim().is_empty() {
            // Named by its anchor when it has one, and by its place on the
            // page when it has not — an entry with neither is still an
            // entry somebody has to find.
            out.push(Defect::EntryWithoutTerm {
                document: document.clone(),
                id: section
                    .id
                    .clone()
                    .unwrap_or_else(|| format!("entry {}", at + 1)),
            });
            continue;
        }
        if definition(section).is_none() {
            out.push(Defect::EntryWithoutDefinition {
                document: document.clone(),
                term: section.title.trim().to_owned(),
            });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
