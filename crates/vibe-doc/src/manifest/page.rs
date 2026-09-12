//! One page's row in the manifest — what the navigation, `llms.txt` and
//! the reader's meta block need about a page without opening it
//! (PROP-057 `##SEO-MANIFEST-AND-RESOLVER`, `##READER-META-AND-PRINT`).
//!
//! Every value here is TAKEN from the page, never composed for it. The
//! title is the H1, the summary is the first paragraph, the audiences are
//! the `audience` attributes of its `<status>` markers, the anchors are
//! the ids that are already written down. A pipeline that wrote its own
//! summary would put a second description of the page in front of readers
//! — one nobody proofreads and no style check governs.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#READER-META-AND-PRINT");

use vibe_specdoc::doc::{Block, BlockNode, Section, SpecDoc, StatusEl, Unit};
use vibe_wire::generated::doc_manifest::{Audience, DocPage, PageGenre};

use super::{AUDIENCES, Reviews};
use crate::pages::Page;

/// Words a minute for the source language. The rate the reader's meta
/// block counts with (the vision's D-22 item 9): English reads faster
/// than the languages this manual is adapted into, and one number for
/// everything would overstate every adaptation.
pub const WORDS_PER_MINUTE_EN: u32 = 250;

/// Words a minute for every other language, Russian included — the rate
/// PROP-057 `##READER-META-AND-PRINT` names for an adaptation.
pub const WORDS_PER_MINUTE_OTHER: u32 = 200;

/// The language tag that reads at [`WORDS_PER_MINUTE_EN`].
const FAST_LANGUAGE: &str = "en";

/// Build one page's row.
pub fn row(page: &Page, lang: &str, reviews: &Reviews) -> DocPage {
    DocPage {
        path: page.rel.clone(),
        title: title_of(&page.doc),
        genre: genre_of(&page.doc),
        audiences: audiences_of(&page.doc),
        anchors: anchors_of(&page.doc),
        summary: summary_of(&page.doc),
        reading_time_min: reading_time_min(&page.doc, lang),
        reviewed_at: reviews.read_aloud(&page.rel),
    }
}

/// The page's H1.
///
/// A page with no H1 yields an empty title rather than a refusal: the
/// pivot admits a document without one (a pure-preamble file is legal),
/// and complaining about it is the style linter's work, not a
/// projection's. An empty string is visible in the navigation, which is
/// exactly the right amount of loud.
fn title_of(doc: &SpecDoc) -> String {
    doc.title
        .as_ref()
        .map(|t| t.text.clone())
        .unwrap_or_default()
}

/// Which of the two page skeletons the page follows.
///
/// The test is mechanical and it is the style law's own: a task page
/// opens with the request a person hands to an agent, so it carries a
/// `prompt` block; a concept page explains one thing and carries none
/// (`AUTHORING.md` §7, PROP-057 `##MANDATE-PROMPT-FIRST`). Nothing is
/// declared on the page, because a declaration and the blocks could
/// disagree, and then the manual would be two things at once.
fn genre_of(doc: &SpecDoc) -> PageGenre {
    let mut genre = PageGenre::Concept;
    walk(doc, &mut |block| {
        if matches!(block, Block::Prompt { .. }) {
            genre = PageGenre::Task;
        }
    });
    genre
}

/// Every audience the page's `<status>` markers name, in the
/// vocabulary's own order (`user`, `author`, `dev`, `agent` — the ladder
/// PROP-043 declares them on), without repetition.
fn audiences_of(doc: &SpecDoc) -> Vec<Audience> {
    let mut found: Vec<Audience> = Vec::new();
    let mut take = |status: Option<&StatusEl>| {
        for a in status.iter().flat_map(|s| s.audience.iter()) {
            let mapped = map_audience(*a);
            if !found.contains(&mapped) {
                found.push(mapped);
            }
        }
    };
    // `Audience` is a generated wire type and carries no `Copy`, so the
    // vocabulary order is walked by reference and cloned once at the end.
    take(doc.status.as_ref());
    for unit in units(doc) {
        take(unit.fact.as_ref().and_then(|f| f.status.as_ref()));
    }
    for section in sections(doc) {
        take(section.status.as_ref());
    }
    AUDIENCES
        .iter()
        .filter(|a| found.contains(a))
        .cloned()
        .collect()
}

/// The progress vocabulary's audience as the wire spells it. An
/// exhaustive match on purpose: the vocabulary is closed and amended only
/// by PROP-043, so a new value must stop the compiler here rather than
/// fall into a silent default.
fn map_audience(a: progress_core::model::Audience) -> Audience {
    match a {
        progress_core::model::Audience::User => Audience::User,
        progress_core::model::Audience::Author => Audience::Author,
        progress_core::model::Audience::Dev => Audience::Dev,
        progress_core::model::Audience::Agent => Audience::Agent,
    }
}

/// Every named anchor on the page, in document order: the title's id,
/// then, section by section, the section's id and the ids of the facts
/// its units carry.
///
/// Public because "the anchors of a page" must mean ONE thing in this
/// crate: the manifest row and the translation mirror check both ask the
/// question, and two walks would answer it differently the first time a
/// block grew a new place to carry a fact.
///
/// These are the addresses a citation may point at, and they are
/// immutable by law (`##INV-ANCHORS-IMMUTABLE`). The positional `pNN`
/// numbers are deliberately absent: they live by the current text, and a
/// manifest listing them would be a promise the next edit breaks.
pub fn anchors_of(doc: &SpecDoc) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Some(title) = &doc.title
        && let Some(id) = &title.id
    {
        out.push(id.clone());
    }
    fact_ids(&doc.preamble, &mut out);
    for section in &doc.sections {
        section_anchors(section, &mut out);
    }
    out
}

fn section_anchors(section: &Section, out: &mut Vec<String>) {
    if let Some(id) = &section.id {
        out.push(id.clone());
    }
    fact_ids(&section.blocks, out);
    for sub in &section.sections {
        section_anchors(sub, out);
    }
}

fn fact_ids(blocks: &[BlockNode], out: &mut Vec<String>) {
    for node in blocks {
        for unit in block_units(&node.block) {
            if let Some(id) = unit.fact.as_ref().and_then(|f| f.id.as_ref()) {
                out.push(id.clone());
            }
        }
    }
}

/// The page's leading fact — the text of its first paragraph.
///
/// The style law requires that paragraph to stand alone and names it as
/// the page's line in `llms.txt` (`AUTHORING.md` §7), so the manifest
/// takes it rather than inventing a second one. A page with no paragraph
/// at all has no summary, and an empty string says so honestly.
fn summary_of(doc: &SpecDoc) -> String {
    let mut summary: Option<String> = None;
    walk(doc, &mut |block| {
        if summary.is_none()
            && let Block::Paragraph(unit) = block
        {
            summary = Some(unit.text.clone());
        }
    });
    summary.unwrap_or_default()
}

/// Minutes to read the page, rounded up, never below one.
///
/// The count is every word the page CARRIES — its prose, its tables, its
/// fences, the commands and golden output of its examples, the text of
/// its prompts. A `derived` block contributes nothing: it holds an
/// address, and its text does not exist until the product is run, which a
/// manifest build deliberately does not do. One rate over all of it: a
/// second rate for code would be a law nobody wrote down, and the reader
/// of a reference page does spend time on the table.
pub fn reading_time_min(doc: &SpecDoc, lang: &str) -> u32 {
    let rate = if lang.eq_ignore_ascii_case(FAST_LANGUAGE) {
        WORDS_PER_MINUTE_EN
    } else {
        WORDS_PER_MINUTE_OTHER
    };
    let words = word_count(doc);
    words.div_ceil(rate as usize).max(1) as u32
}

fn word_count(doc: &SpecDoc) -> usize {
    let mut words = 0usize;
    let mut count = |text: &str| words += text.split_whitespace().count();
    if let Some(title) = &doc.title {
        count(&title.text);
    }
    for section in sections(doc) {
        count(&section.title);
    }
    walk(doc, &mut |block| match block {
        Block::Fence { text, .. } => words += text.split_whitespace().count(),
        Block::Example {
            run,
            expect,
            stderr,
            ..
        } => {
            words += run.split_whitespace().count();
            words += expect.split_whitespace().count();
            words += stderr
                .iter()
                .map(|s| s.split_whitespace().count())
                .sum::<usize>();
        }
        Block::Prompt {
            text,
            needs,
            outcome,
            asserts,
            ..
        } => {
            words += text.split_whitespace().count();
            words += needs
                .iter()
                .map(|s| s.split_whitespace().count())
                .sum::<usize>();
            words += outcome
                .iter()
                .map(|s| s.split_whitespace().count())
                .sum::<usize>();
            words += asserts
                .iter()
                .map(|s| s.split_whitespace().count())
                .sum::<usize>();
        }
        other => {
            for unit in block_units(other) {
                words += unit.text.split_whitespace().count();
            }
        }
    });
    words
}

/// Every block of the document in reading order, preamble first.
///
/// The visitor borrows for the DOCUMENT's lifetime, not the call's, so a
/// caller may keep what it is handed — the translation check collects a
/// page's blocks to compare them, and a walk that forbade that would have
/// to be written a second time.
pub(crate) fn walk<'a>(doc: &'a SpecDoc, visit: &mut impl FnMut(&'a Block)) {
    for node in &doc.preamble {
        visit(&node.block);
    }
    for section in &doc.sections {
        walk_section(section, visit);
    }
}

fn walk_section<'a>(section: &'a Section, visit: &mut impl FnMut(&'a Block)) {
    for node in &section.blocks {
        visit(&node.block);
    }
    for sub in &section.sections {
        walk_section(sub, visit);
    }
}

/// Every section of the document, nested ones included.
fn sections(doc: &SpecDoc) -> Vec<&Section> {
    let mut out: Vec<&Section> = Vec::new();
    fn push<'a>(section: &'a Section, out: &mut Vec<&'a Section>) {
        out.push(section);
        for sub in &section.sections {
            push(sub, out);
        }
    }
    for section in &doc.sections {
        push(section, &mut out);
    }
    out
}

/// Every unit of the document — the carriers of facts.
fn units(doc: &SpecDoc) -> Vec<&Unit> {
    let mut out: Vec<&Unit> = Vec::new();
    for node in &doc.preamble {
        out.extend(block_units(&node.block));
    }
    for section in sections(doc) {
        for node in &section.blocks {
            out.extend(block_units(&node.block));
        }
    }
    out
}

/// The units one block carries. A block whose content is not a unit —
/// a fence, an example, a generated reference, a prompt — carries none,
/// and a fact cannot be anchored inside it.
pub(crate) fn block_units(block: &Block) -> Vec<&Unit> {
    match block {
        Block::Paragraph(unit) | Block::Quote(unit) => vec![unit],
        Block::List { items, .. } => items.iter().collect(),
        Block::Table { rows } => rows.iter().flatten().collect(),
        Block::Note { body, .. } => vec![body],
        Block::Figure { caption, .. } => vec![caption],
        Block::Fence { .. }
        | Block::Example { .. }
        | Block::ExampleRef { .. }
        | Block::Rule { .. }
        | Block::Derived { .. }
        | Block::Prompt { .. } => Vec::new(),
    }
}

#[cfg(test)]
mod tests;
