//! `vibe doc check --chapters` — the declared learning path, measured
//! (PROP-057 `##NAV-CHAPTERS`, `##NAV-CHAPTERS-CHECKED`).
//!
//! A documentation package may declare a learning path: chapters in the
//! order a reader walks them, pages in the order a reader meets them.
//! The path is written for people and moves nothing else — `pages` in the
//! page manifest and the `llms` tiers keep the layer law
//! (`spec://org.vibevm.core/vibevm/common/PROP-048#THE-LAYER-LAW`), which
//! orders a corpus for an agent's prompt cache. Two orders for two
//! audiences is the decision, not a thing to resolve
//! (`##NAV-CHAPTERS-DECISION`).
//!
//! ## What this measures
//!
//! One number: how often the path sends a reader ahead of itself. Every
//! link from a page to a page the path reaches LATER is a place where the
//! text needs something it has not taught yet — the textbook question a
//! taxonomy never has to answer.
//!
//! ## Why it is a measurement and never a gate
//!
//! It does not change the exit code, and that is the norm's own ruling
//! (`##NAV-CHAPTERS-CHECKED`): an orientation page points ahead ON
//! PURPOSE. A manual opens by saying what it will cover, and every one of
//! those sentences is a forward link. A gate here would teach an author
//! that the way to a green run is to stop writing the page that tells a
//! reader where they are, so the machine counts and a person reads the
//! count.
//!
//! Two kinds of link are not forward links at all, and both fall out of
//! one rule rather than being special-cased. A link INTO an appendix
//! chapter is excepted, because looking a term up in the glossary or a
//! flag up in the reference tables is not being sent ahead of the lesson
//! — those chapters are declared `appendix = true` exactly so this can be
//! said once. And a fragment alone names a place on the page that says
//! it, which is no link to another page in any reading
//! ([`crate::html::links::target_document`]).
//!
//! ## What it refuses to guess
//!
//! A package that declared no path is not measured, and the report says
//! so instead of printing a zero: «no link points ahead» and «nobody
//! declared an order» print the same digit and mean opposite things. A
//! page the pivot could not read is named rather than counted clean — its
//! links are unknown, and an unknown is not an absence.
//!
//! ## The machine form is a registered format
//!
//! The document `--json` prints is a wire surface of this project's own,
//! so it is a JTD schema in `formats/REGISTRY.toml` (record
//! `doc-chapters`) with generated types, and never a
//! `#[derive(Serialize)]` on a struct written here: a handwritten writer
//! of our own format is the mechanism by which the other wire bans break
//! unnoticed (PROP-044 §2, law 5). The field prose therefore lives in
//! `schemas/doc_chapters.jtd.json` and reaches this crate as the
//! generated type's own documentation — one description, read by the
//! schema's reader and by this file's reader alike.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#NAV-CHAPTERS-CHECKED");

use std::path::Path;

use vibe_wire::generated::doc_chapters::{DocChapters, ForwardLink};
use vibe_wire::generated::doc_manifest::NavigationChapter;

use crate::error::Result;
use crate::html::links::target_document;
use crate::manifest;
use crate::pages::{self, Page, document_of};

/// The schema version a measurement taken today carries.
pub const SCHEMA_VERSION: u32 = 1;

/// The shape of a package that declared no path: nothing measured, and
/// `measured` saying which of the two zeros this is.
fn unmeasured() -> DocChapters {
    DocChapters {
        schema_version: SCHEMA_VERSION,
        chapters: 0,
        pages: 0,
        forward_links: Vec::new(),
        forward_link_count: 0,
        unreadable: Vec::new(),
        measured: false,
    }
}

/// The human form: the links that point ahead, then the numbers.
///
/// There is deliberately no `ok()` beside it. Every other report in this
/// crate has one because the surface above turns it into an exit code;
/// this one has no verdict to give, and a function named `ok` would
/// invite the gate the norm refuses.
pub fn render(report: &DocChapters) -> String {
    let mut out = String::new();
    if !report.measured {
        out.push_str(
            "chapters: this package declares no learning path — its contents is the pages in the \
             order the manifest gives them, and there is nothing to measure\n",
        );
        return out;
    }
    for link in &report.forward_links {
        out.push_str(&format!("  AHEAD {} -> {}\n", link.page, link.target));
    }
    for page in &report.unreadable {
        out.push_str(&format!("  unreadable {page}\n"));
    }
    out.push_str(&format!(
        "chapters: {} chapter(s), {} page(s) on the path, {} link(s) pointing ahead of the reader, \
         {} unreadable page(s) — a measurement and not a gate\n",
        report.chapters,
        report.pages,
        report.forward_link_count,
        report.unreadable.len()
    ));
    out
}

/// The measurement as a machine reads it.
pub fn to_json(report: &DocChapters) -> String {
    // Generated from the schema, so it holds only JSON scalars and
    // sequences and cannot fail to serialise; a fallible signature would
    // push an impossible arm onto every caller.
    let mut text = serde_json::to_string_pretty(report).unwrap_or_default();
    text.push('\n');
    text
}

/// Measure the learning path the package at `package_dir` declares.
///
/// A package that declares none is measured as nothing, which is not the
/// same as measured as zero: the report says which of the two it is, and
/// the caller prints it either way.
pub fn check(package_dir: &Path) -> Result<DocChapters> {
    let chapters = manifest::chapters(package_dir)?;
    if chapters.is_empty() {
        // Nothing is read: a package that declared no path is not asked
        // about its links, and reading every page to say so would be a
        // walk for an answer already given.
        return Ok(unmeasured());
    }
    let set = pages::read_package(package_dir)?;
    let mut report = measure(&chapters, &set.pages);
    report.unreadable = set.unreadable.iter().map(|u| u.rel.clone()).collect();
    Ok(report)
}

/// The measurement over an already-read package. Split from [`check`] so
/// it is testable without a tree, the way the mirror check's comparison
/// is.
pub fn measure(chapters: &[NavigationChapter], pages: &[Page]) -> DocChapters {
    if chapters.is_empty() {
        return unmeasured();
    }
    // Where the path takes a reader, and which pages it takes them to for
    // reference rather than for reading. Both are read off the
    // declaration in its written order: the rows ARE the path.
    let mut position: Vec<&str> = Vec::new();
    let mut appendix: Vec<&str> = Vec::new();
    for chapter in chapters {
        for page in &chapter.pages {
            if chapter.appendix {
                appendix.push(page);
            }
            if !position.contains(&page.as_str()) {
                position.push(page);
            }
        }
    }
    let place = |document: &str| position.iter().position(|p| *p == document);

    let mut forward_links: Vec<ForwardLink> = Vec::new();
    // The path's own order, not the manifest's: a report about a reading
    // order is read in that order.
    for document in &position {
        let Some(page) = pages.iter().find(|p| document_of(&p.rel) == *document) else {
            // A chapter naming a page the package does not carry is
            // `vibe check`'s finding, and repeating it as a measurement
            // would report one defect as two.
            continue;
        };
        let Some(from) = place(document) else {
            continue;
        };
        for target in links_of(page) {
            let Some(to) = place(&target) else {
                continue;
            };
            if to <= from || appendix.contains(&target.as_str()) {
                continue;
            }
            forward_links.push(ForwardLink {
                page: (*document).to_owned(),
                target,
            });
        }
    }

    DocChapters {
        schema_version: SCHEMA_VERSION,
        chapters: chapters.len() as u32,
        pages: position.len() as u32,
        forward_link_count: forward_links.len() as u32,
        forward_links,
        unreadable: Vec::new(),
        measured: true,
    }
}

/// Every page of this package one page's prose points at, in the order
/// the prose writes them.
///
/// The links are read with the pipeline's own inline grammar and its own
/// address algebra — the same scanner the island renders with
/// ([`crate::html::inline::hrefs`]) and the same walk a citation is
/// resolved by ([`target_document`]). A pattern match over the text would
/// count a link inside a code span that no reader can click, and then the
/// measurement would be of something the manual does not do.
fn links_of(page: &Page) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    manifest::page::walk(&page.doc, &mut |block| {
        for unit in manifest::page::block_units(block) {
            for href in crate::html::inline::hrefs(&unit.text) {
                if let Some(target) = target_document(&page.rel, &href) {
                    out.push(target);
                }
            }
        }
    });
    out
}

#[cfg(test)]
mod tests;
