//! The definitions a page carries for the glossary terms it links
//! (PROP-057 `##READER-GLOSSARY-CARD`, `##PIPE-SHELL-PARSES-NOTHING`).
//!
//! A reader who meets a term in the middle of a page leaves the page to read
//! one sentence in the glossary and loses their place; the official manual
//! links its glossary 190 times. So the definition travels WITH the page, as
//! hidden HTML the pipeline resolved at build time.
//!
//! ## Why the definitions are in the island and not fetched
//!
//! The shell parses nothing (`##PIPE-SHELL-PARSES-NOTHING`), the local
//! reader works offline, and every hover would otherwise be a request. A
//! JSON wire for the definitions was weighed and refused for the same
//! reason: it would be a new format to carry what is already finished HTML
//! (`##GLOSSARY-CARD-DECISION`).
//!
//! ## Only the terms THIS page names
//!
//! The island carries the entries the page links and no others, in the order
//! it first links them. A page that carries the whole glossary would carry
//! the glossary on every page, and a card cannot show what a link does not
//! point at.
//!
//! The links are read with the pipeline's own inline grammar and its own
//! address algebra — [`crate::html::inline::hrefs`] and
//! [`crate::html::links::target_document`], the same pair the forward-link
//! measurement uses ([`crate::chapters`]). A pattern match over the text
//! would count a link inside a code span that no reader can click, and then
//! a page would carry a definition nothing on it can open.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#READER-GLOSSARY-CARD");

use vibe_specdoc::doc::SpecDoc;

use crate::content::Content;
use crate::glossary::{Entry, Glossary};
use crate::manifest::page::{block_units, walk};
use crate::pages::document_of;

use super::Attrs;
use super::emit::{close, line, open};
use super::inline::{hrefs, render_linked};
use super::links::{Links, target_document};

/// What an entry's hidden definition is addressed by: the prefix, and the
/// entry's own id after it. A link's `aria-describedby` names exactly this,
/// which is why the two are spelled in one place.
pub const GLOSS_DEF_ID: &str = "gloss-";

/// Emit the page's hidden definitions, or nothing at all.
///
/// Nothing is the common case and every one of its reasons is ordinary: the
/// documentation declared no glossary, this IS the glossary page, or the
/// page links no term. A page that links none carries no block rather than
/// an empty one, because an empty `aside` is a thing a stylesheet and a
/// reader both have to know to ignore.
pub(super) fn defs(
    out: &mut String,
    depth: usize,
    doc: &SpecDoc,
    page: Option<&str>,
    content: &Content,
) {
    let Some(glossary) = content.glossary.as_ref() else {
        return;
    };
    let Some(page) = page.filter(|page| document_of(page) != glossary.document) else {
        return;
    };
    let entries = referenced(doc, page, glossary);
    if entries.is_empty() {
        return;
    }
    // The definition's own links are resolved as everything else in this
    // island is: against the page being rendered. It gets no glossary lens
    // — a definition that opened a card inside a card would show a reader
    // the same paragraph twice, and the entry beside it is already here.
    let links = Links::page(&content.base);
    open(
        out,
        depth,
        "aside",
        &[
            ("class", "gloss-defs".to_owned()),
            ("hidden", String::new()),
            ("data-gloss-defs", String::new()),
        ],
    );
    for entry in entries {
        let attrs: Attrs = vec![
            ("class", "gloss-def".to_owned()),
            ("id", format!("{GLOSS_DEF_ID}{}", entry.id)),
            ("data-gloss", entry.id.clone()),
        ];
        open(out, depth + 1, "div", &attrs);
        line(
            out,
            depth + 2,
            "p",
            &[("class", "gloss-def__term".to_owned())],
            &render_linked(&entry.term, &links),
        );
        line(
            out,
            depth + 2,
            "p",
            &[("class", "gloss-def__text".to_owned())],
            &render_linked(&entry.definition, &links),
        );
        close(out, depth + 1, "div");
    }
    close(out, depth, "aside");
}

/// The entries this page links, each once, in the order it first links
/// them.
///
/// Reading order rather than the glossary's own: the block is a companion to
/// the page's prose, and a diff of two islands should show the term that was
/// added where it was added.
fn referenced<'a>(doc: &SpecDoc, page: &str, glossary: &'a Glossary) -> Vec<&'a Entry> {
    let mut out: Vec<&Entry> = Vec::new();
    walk(doc, &mut |block| {
        for unit in block_units(block) {
            for href in hrefs(&unit.text) {
                let Some(entry) = entry_at(&href, page, glossary) else {
                    continue;
                };
                if !out.iter().any(|seen| seen.id == entry.id) {
                    out.push(entry);
                }
            }
        }
    });
    out
}

/// The glossary entry one link target names, when it names one.
fn entry_at<'a>(href: &str, page: &str, glossary: &'a Glossary) -> Option<&'a Entry> {
    let anchor = href.rsplit_once('#')?.1;
    if target_document(page, href)? != glossary.document {
        return None;
    }
    glossary.entry(anchor)
}

#[cfg(test)]
mod tests;
