//! `vibe doc check --translations` — an adaptation against the source it
//! mirrors (PROP-057 `##LOC-MIRROR`, `##LOC-EXAMPLE-REF`,
//! `##LOC-NO-REVISION`, campaign rule R-18).
//!
//! A translation is a separate package that mirrors its source file for
//! file: the same page addresses, the same anchors, the same fact
//! identifiers. That is not tidiness. It is what makes the language
//! selector able to move a reader to the same place in another language
//! with the fragment intact (`##READER-LANGUAGE-SWITCH-KEEPS-PLACE`), and
//! what lets a `spec://…#ANCHOR` citation mean one thing in every
//! language a manual is published in.
//!
//! ## What this check asks, and what it refuses to ask
//!
//! It asks about STRUCTURE: do the two packages carry the same pages, do
//! the pages carry the same anchors, does every example a translation
//! borrows exist on the source page it borrows from, and did the
//! translation author an example of its own — which is forbidden, because
//! command output is checked once, on the source, and a second copy is a
//! second thing to go wrong (`##LOC-EXAMPLE-REF`).
//!
//! It does NOT ask whether the adaptation still says what the source
//! says. There is no revision, no content hash and no «behind by» here,
//! and there never will be: such a comparison needs a history the product
//! does not keep by design (`##OBS-VERSION-CONTRACT`). Whether the prose
//! drifted is a human's question at a full reconciliation, and the answer
//! the machine keeps is the date somebody last read the page aloud
//! (`reviews.toml`, [`crate::manifest::reviews`]).
//!
//! ## The translation is what gets fixed
//!
//! Every problem below is reported as a defect of the TRANSLATION, never
//! of the source. A mirror is defined against the thing it mirrors, and a
//! check that let the argument run the other way would turn every
//! adaptation into a veto on its source.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#LOC-MIRROR");

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use vibe_specdoc::doc::{Block, SpecDoc};

use crate::citations::sources::{Source, SpecSources};
use crate::error::{DocError, Result};
use crate::manifest;
use crate::pages::{self, Page, PageSet};

/// The manifest key a translation declares its source under.
pub const TRANSLATES: &str = "translates";

/// One way a translation fails to mirror its source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// The source has this page and the translation does not.
    MissingPage { page: String },
    /// The translation has a page the source does not.
    ExtraPage { page: String },
    /// An anchor of the source page is absent from the translation.
    AnchorMissing { page: String, anchor: String },
    /// The translation page carries an anchor the source does not.
    AnchorExtra { page: String, anchor: String },
    /// The translation authored an example of its own.
    OwnExample { page: String, id: String },
    /// The translation borrows an example the source page does not have.
    UnknownExampleRef { page: String, id: String },
}

impl Problem {
    /// The page the problem is on.
    pub fn page(&self) -> &str {
        match self {
            Problem::MissingPage { page }
            | Problem::ExtraPage { page }
            | Problem::AnchorMissing { page, .. }
            | Problem::AnchorExtra { page, .. }
            | Problem::OwnExample { page, .. }
            | Problem::UnknownExampleRef { page, .. } => page,
        }
    }

    /// The line a report prints, in the words of what went wrong.
    pub fn render(&self) -> String {
        match self {
            Problem::MissingPage { page } => format!(
                "  MISSING PAGE {page}\n    the source has this page and the translation \
                 does not; a mirror is file for file"
            ),
            Problem::ExtraPage { page } => format!(
                "  EXTRA PAGE {page}\n    the translation has a page the source does not; \
                 a page that exists in one language only belongs in the source first"
            ),
            Problem::AnchorMissing { page, anchor } => format!(
                "  ANCHOR MISSING {page}#{anchor}\n    the source page carries this anchor \
                 and the translation does not; a citation must resolve in every language"
            ),
            Problem::AnchorExtra { page, anchor } => format!(
                "  ANCHOR ADDED {page}#{anchor}\n    the translation carries an anchor the \
                 source does not; adding one is forbidden, not merely unmirrored"
            ),
            Problem::OwnExample { page, id } => format!(
                "  OWN EXAMPLE {page}#{id}\n    a translation must not author examples — \
                 use `<example ref=\"{id}\"/>` so the output is checked once, on the source"
            ),
            Problem::UnknownExampleRef { page, id } => format!(
                "  UNKNOWN EXAMPLE {page}#{id}\n    the translation borrows an example the \
                 source page does not carry"
            ),
        }
    }
}

/// One `--translations` run.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// The coordinate this package adapts, or `None` when it adapts
    /// nothing and there is nothing to mirror.
    pub adapts: Option<String>,
    /// Which source held the adapted documentation.
    pub source: Option<Source>,
    /// How many pages the translation carries.
    pub pages: usize,
    /// Everything that does not mirror. Empty is the only green state.
    pub problems: Vec<Problem>,
    /// Pages of either package the pivot refused; an unreadable page
    /// hides its structure, so it is reported rather than counted clean.
    pub unreadable: Vec<String>,
}

impl Report {
    /// Green when the mirror holds and every page was readable.
    pub fn ok(&self) -> bool {
        self.problems.is_empty() && self.unreadable.is_empty()
    }

    /// The human form.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for problem in &self.problems {
            out.push_str(&problem.render());
            out.push('\n');
        }
        for page in &self.unreadable {
            out.push_str(&format!("  unreadable {page}\n"));
        }
        let Some(adapts) = &self.adapts else {
            out.push_str(
                "translations: this package declares no `[translates]` — it is a source \
                 documentation, and there is nothing to mirror\n",
            );
            return out;
        };
        let source = self
            .source
            .map(|s| format!(" found in {}", where_from(s)))
            .unwrap_or_default();
        out.push_str(&format!(
            "translations: adapting {adapts}{source}, {} page(s), {} problem(s), \
             {} unreadable page(s)\n",
            self.pages,
            self.problems.len(),
            self.unreadable.len()
        ));
        out
    }
}

/// Which of the four sources answered, in words a reader can act on.
/// «the in-tree» is the name of a mechanism; «the project's own in-tree
/// registry» is a place somebody can go and look.
fn where_from(source: Source) -> &'static str {
    match source {
        Source::Checkout => "the checkout",
        Source::InTree => "the project's own in-tree registry",
        Source::Lock => "the slot the lock file selected",
        Source::Store => "the machine store",
    }
}

/// Check the translation at `package_dir` against the source it declares.
///
/// A package that declares no `[translates]` is a source documentation:
/// the run is green and says so, because «run the check on everything»
/// must not be a way to fail for the packages the check does not apply
/// to.
///
/// A source the four sources cannot reach IS an error. A mirror check
/// with nothing to mirror against has no verdict to give, and reporting
/// green would be the worst of the three possible answers.
pub fn check(package_dir: &Path, sources: &SpecSources) -> Result<Report> {
    let Some((coordinate, _constraint)) = manifest::relations(package_dir, TRANSLATES)?
        .into_iter()
        .next()
    else {
        return Ok(Report::default());
    };
    let (group, name) = coordinate
        .split_once('/')
        .ok_or_else(|| DocError::Translation {
            adapts: coordinate.clone(),
            message: "`[translates].package` is not a `<group>/<name>` coordinate".to_owned(),
        })?;
    let instance = sources
        .instance_of(group, name)
        .ok_or_else(|| DocError::Translation {
            adapts: coordinate.clone(),
            message: format!(
                "no source holds it, so there is nothing to mirror against; warm it with \
                 `vibe cache add {coordinate}`, or point the check at a tree that carries it"
            ),
        })?;

    let translation = pages::read_package(package_dir)?;
    let source = pages::read_package(&instance.root)?;
    let mut report = compare(&source, &translation);
    report.adapts = Some(coordinate);
    report.source = Some(instance.source);
    Ok(report)
}

/// Compare two read packages. Split from [`check`] so the comparison is
/// testable without a registry, a store or a checkout anywhere near it.
pub fn compare(source: &PageSet, translation: &PageSet) -> Report {
    let mut report = Report {
        pages: translation.pages.len(),
        ..Report::default()
    };
    for page in source
        .unreadable
        .iter()
        .chain(translation.unreadable.iter())
    {
        report.unreadable.push(page.rel.clone());
    }

    let by_address: BTreeMap<&str, &Page> =
        source.pages.iter().map(|p| (p.rel.as_str(), p)).collect();
    let translated: BTreeSet<&str> = translation.pages.iter().map(|p| p.rel.as_str()).collect();

    for page in source.pages.iter() {
        if !translated.contains(page.rel.as_str()) {
            report.problems.push(Problem::MissingPage {
                page: page.rel.clone(),
            });
        }
    }
    for page in &translation.pages {
        let Some(original) = by_address.get(page.rel.as_str()) else {
            report.problems.push(Problem::ExtraPage {
                page: page.rel.clone(),
            });
            continue;
        };
        compare_page(original, page, &mut report);
    }
    report
}

/// One page against its original.
fn compare_page(source: &Page, translation: &Page, report: &mut Report) {
    let page = translation.rel.clone();
    let theirs: BTreeSet<String> = anchors(&source.doc).into_iter().collect();
    let ours: BTreeSet<String> = anchors(&translation.doc).into_iter().collect();
    for anchor in theirs.difference(&ours) {
        report.problems.push(Problem::AnchorMissing {
            page: page.clone(),
            anchor: anchor.clone(),
        });
    }
    for anchor in ours.difference(&theirs) {
        report.problems.push(Problem::AnchorExtra {
            page: page.clone(),
            anchor: anchor.clone(),
        });
    }

    let available: BTreeSet<String> = example_ids(&source.doc).into_iter().collect();
    for block in blocks(&translation.doc) {
        match block {
            Block::Example { id, .. } => report.problems.push(Problem::OwnExample {
                page: page.clone(),
                id: id.clone(),
            }),
            Block::ExampleRef { id } if !available.contains(id) => {
                report.problems.push(Problem::UnknownExampleRef {
                    page: page.clone(),
                    id: id.clone(),
                });
            }
            _ => {}
        }
    }
}

/// Every named anchor a page carries — the same walk the manifest's page
/// row takes, because «the anchors of a page» must mean one thing in this
/// crate.
fn anchors(doc: &SpecDoc) -> Vec<String> {
    manifest::page::anchors_of(doc)
}

/// The ids of the examples a page authors — what a translation may borrow
/// from it.
fn example_ids(doc: &SpecDoc) -> Vec<String> {
    blocks(doc)
        .into_iter()
        .filter_map(|b| match b {
            Block::Example { id, .. } => Some(id.clone()),
            _ => None,
        })
        .collect()
}

/// Every block of a page in document order.
fn blocks(doc: &SpecDoc) -> Vec<&Block> {
    let mut out: Vec<&Block> = Vec::new();
    manifest::page::walk(doc, &mut |block| out.push(block));
    out
}

#[cfg(test)]
mod tests;
