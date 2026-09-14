//! The style linter — `vibe doc check --style`
//! (PROP-057 `##STYLE-LINT`, STYLE.md §11).
//!
//! Documentation written by a language model fails a smart reader in two
//! ways, and the linter is the mechanical half of the answer to both.
//!
//! The first way is tics — «delve», «robust», «it is important to note».
//! They are caught by a list, and the list is data in the documentation
//! package, per language ([`banned`]).
//!
//! The second way is misplaced density: a page written as if the reader
//! had already read every specification it cites. That one has no word
//! list, and it is what [`terms`] and the length limits of [`rules`] are
//! for — a term used before anybody introduced it, three terms in one
//! sentence, a forty-word instruction. The law calls the dense places
//! CONTAINERS and the narrative places CORRIDORS, and it puts the errors
//! in the procedures and the warnings in the corridors
//! (`##STYLE-CONTAINERS-AND-CORRIDORS`).
//!
//! ## What the linter will not do
//!
//! It reports; it never edits. A page is prose, prose is written by a
//! person in the central session, and a linter that could rewrite a
//! sentence would be a linter authors write FOR instead of writing well
//! (`##STYLE-LINT`: a false positive is fixed in the rule, with a BACKLOG
//! entry, never worked around in the text).
//!
//! It also gates by PAGE and not by finding. `--min` is the share of
//! pages that must carry no error — a hundred per cent is the standing
//! bar, and a lower one is for the intermediate runs of a campaign that
//! is still writing the pages, exactly as `--coverage` uses it.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#STYLE-LINT");

pub mod banned;
pub mod glossary;
pub mod inline;
pub mod prose;
pub mod report;
pub mod rules;
pub mod sentence;
pub mod terms;

use std::path::Path;

use vibe_specdoc::doc::{Block, BlockNode, Section, SpecDoc};

use crate::error::Result;
use crate::numbering::{BlockPath, Numbering, number_blocks};
use crate::pages::{self, Page};
use crate::style::report::{Finding, PageScore, Rule, Severity};

pub use report::Report;

/// The bar a finished manual answers to: every page clean.
pub const FULL_STYLE: u8 = 100;

/// Lint a documentation package's prose.
///
/// The language is the package's own (`[i18n] canonical`), and it
/// chooses the banned list: a package that ships no list for the language
/// it is written in is refused by name rather than passed on an empty
/// list.
pub fn check(package_dir: &Path, min_percent: u8) -> Result<Report> {
    let set = pages::read_package(package_dir)?;
    let lang = crate::manifest::language(package_dir)?;
    let list = banned::read(package_dir, &lang)?;
    let terms = glossary::terms(&set);

    let mut findings: Vec<Finding> = Vec::new();
    let mut scores: Vec<PageScore> = Vec::new();
    for page in &set.pages {
        let (found, readability) = page_findings(page, &list.entries, &terms, &lang);
        let of = |s: Severity| found.iter().filter(|f| f.severity == s).count();
        scores.push(PageScore {
            page: page.rel.clone(),
            errors: of(Severity::Error),
            warnings: of(Severity::Warning),
            readability,
        });
        findings.extend(found);
    }
    Ok(Report {
        lang,
        findings,
        pages: scores,
        unreadable: set.unreadable.iter().map(|u| u.rel.clone()).collect(),
        min_percent,
    })
}

/// Everything one page is told about, in document order, and how hard it
/// reads. One walk: the prose is lifted off the page once and every rule
/// reads the same nodes.
fn page_findings(
    page: &Page,
    list: &[banned::Entry],
    terms: &[glossary::Term],
    lang: &str,
) -> (Vec<Finding>, f64) {
    let nodes = prose::nodes(&page.doc);
    let mut out: Vec<Finding> = Vec::new();
    for node in &nodes {
        out.extend(rules::banned_words(&page.rel, node, list));
        out.extend(rules::length(&page.rel, node));
        out.extend(rules::signs(&page.rel, node, lang));
    }
    out.extend(terms::check(&page.rel, &nodes, terms));
    out.extend(rules::headings(&page.rel, &page.doc));
    out.extend(prompts_without_asserts(&page.rel, &page.doc));
    let readability = sentence::readability(&prose::joined(&nodes));
    (out, readability)
}

/// A prompt with no assert, on a page that opens with one
/// (`##STYLE-PROMPT-FIRST`, and the rule A2.29 hands the linter).
///
/// A SCENARIO page is one whose prompt stands before the first section —
/// the shape `##STYLE-PAGE-SKELETON` prescribes for a task page, which
/// «starts with the prompt». On such a page every prompt is a task, and a
/// task is checked by its asserts, because a prompt cannot be checked by
/// its output the way a shell example can. An illustrative prompt further
/// down an explanation page says `assert="none"` and is left alone.
fn prompts_without_asserts(page: &str, doc: &SpecDoc) -> Vec<Finding> {
    let numbering = number_blocks(doc);
    let scenario = doc
        .preamble
        .iter()
        .any(|node| matches!(node.block, Block::Prompt { .. }));
    if !scenario {
        return Vec::new();
    }
    let mut out: Vec<Finding> = Vec::new();
    collect_prompts(&doc.preamble, &[], &numbering, page, &mut out);
    fn walk(
        sections: &[Section],
        path: &[u16],
        numbering: &Numbering,
        page: &str,
        out: &mut Vec<Finding>,
    ) {
        for (i, section) in sections.iter().enumerate() {
            let mut here = path.to_vec();
            here.push(i as u16);
            collect_prompts(&section.blocks, &here, numbering, page, out);
            walk(&section.sections, &here, numbering, page, out);
        }
    }
    walk(&doc.sections, &[], &numbering, page, &mut out);
    out
}

fn collect_prompts(
    blocks: &[BlockNode],
    path: &[u16],
    numbering: &Numbering,
    page: &str,
    out: &mut Vec<Finding>,
) {
    for (i, node) in blocks.iter().enumerate() {
        let Block::Prompt { id, asserts, .. } = &node.block else {
            continue;
        };
        if !asserts.is_empty() {
            continue;
        }
        out.push(Finding {
            page: page.to_owned(),
            block: numbering
                .label(&BlockPath::new(path.to_vec(), i as u16))
                .unwrap_or_else(|| "p--".to_owned()),
            node: "prompt",
            rule: Rule::PromptWithoutAssert,
            severity: Severity::Error,
            message: format!(
                "the prompt `{id}` carries no assert, and this page opens with a prompt — a \
                 prompt cannot be checked by its output the way a shell example can"
            ),
            text: String::new(),
        });
    }
}

#[cfg(test)]
mod tests;
