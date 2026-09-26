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
//! the pages carry the same anchors, do they carry the same blocks in the
//! same order, does every example a translation borrows exist on the
//! source page it borrows from, and did the translation author an example
//! of its own — which is forbidden, because command output is checked
//! once, on the source, and a second copy is a second thing to go wrong
//! (`##LOC-EXAMPLE-REF`).
//!
//! The block comparison is the half anchors cannot cover (the campaign's
//! F-44): two pages can agree on every anchor and still part on a
//! paragraph the adaptation merged into its neighbour, and from there
//! `#p12` means one thing in English and another in Russian. Merging or
//! splitting paragraphs in a translation is not a style choice — it is
//! the one edit that breaks the promise the language selector makes.
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
use vibe_wire::generated::doc_manifest::NavigationChapter;

use crate::citations::sources::{Instance, Source, SpecSources};
use crate::error::{DocError, Result};
use crate::manifest;
use crate::numbering::{Numbering, block_at, number_blocks};
use crate::pages::{self, Page, PageSet, document_of};

mod borrowed;

pub use borrowed::{borrowed, borrowed_by};

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
    /// The translation names a chapter of the learning path the source
    /// does not declare.
    UnknownChapter { id: String },
    /// The two packages do not declare the same glossary: the source
    /// declares one and the translation does not, or the translation names
    /// another page.
    Glossary {
        source: Option<String>,
        translation: Option<String>,
    },
    /// The two pages part at a block: a different kind at the same
    /// number, or one page running out of blocks before the other.
    Block {
        page: String,
        /// The first block number where they part, as the reader sees it
        /// in every projection — `p07`.
        at: String,
        /// The kind the source carries there, or `nothing` past its end.
        source: String,
        /// The kind the translation carries there.
        translation: String,
    },
}

impl Problem {
    /// The page the problem is on, or the manifest when the problem is
    /// the manifest's: a chapter the source does not declare is a defect
    /// of the `[navigation]` table, and naming a page for it would send
    /// the reader of a report to the wrong file.
    pub fn page(&self) -> &str {
        match self {
            Problem::MissingPage { page }
            | Problem::ExtraPage { page }
            | Problem::AnchorMissing { page, .. }
            | Problem::AnchorExtra { page, .. }
            | Problem::OwnExample { page, .. }
            | Problem::UnknownExampleRef { page, .. }
            | Problem::Block { page, .. } => page,
            Problem::UnknownChapter { .. } | Problem::Glossary { .. } => {
                crate::derived::manifest::MANIFEST
            }
        }
    }

    /// The line a report prints, in the words of what went wrong.
    ///
    /// `adapts` is the coordinate of the documentation being mirrored, so
    /// a block divergence can name BOTH pages — the one to read and the
    /// one to fix — instead of an address that could be either.
    pub fn render(&self, adapts: &str) -> String {
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
            Problem::UnknownChapter { id } => format!(
                "  UNKNOWN CHAPTER {id}\n    the translation renames a chapter of the learning \
                 path that `{adapts}` does not declare; a translation names the source's \
                 chapters and adds none of its own\n    \
                 (spec://org.vibevm.core/vibevm/common/PROP-057#NAV-CHAPTERS-TRANSLATION)"
            ),
            Problem::Glossary {
                source,
                translation,
            } => match (source, translation) {
                (Some(theirs), None) => format!(
                    "  GLOSSARY MISSING {theirs}\n    `{adapts}` declares its glossary and this \
                     translation declares none; a translation's glossary is its own mirrored \
                     page at the same path, with the terms in its language\n    \
                     (spec://org.vibevm.core/vibevm/common/PROP-057#GLOSSARY-TRANSLATION)"
                ),
                (theirs, ours) => format!(
                    "  GLOSSARY {} {}\n    a translation declares the SAME glossary as the \
                     documentation it adapts, and two pages are two vocabularies for one \
                     manual\n    \
                     (spec://org.vibevm.core/vibevm/common/PROP-057#GLOSSARY-TRANSLATION)",
                    ours.as_deref().unwrap_or("none"),
                    match theirs {
                        Some(theirs) => format!("(`{adapts}` declares `{theirs}`)"),
                        None => format!("(`{adapts}` declares none)"),
                    }
                ),
            },
            Problem::Block {
                page,
                at,
                source,
                translation,
            } => format!(
                "  BLOCK {at} {page}\n    the source carries `{source}` there and the \
                 translation carries `{translation}`\n    \
                 source:      spec://{adapts}/{}#{at}\n    \
                 translation: {page}#{at}",
                document_of(page)
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
        let adapts = self.adapts.as_deref().unwrap_or("<unknown>");
        for problem in &self.problems {
            out.push_str(&problem.render(adapts));
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

/// What a package adapts, and which instance of it answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adapted {
    /// The `<group>/<name>` coordinate of the documentation being
    /// mirrored, exactly as `[translates].package` spells it.
    pub coordinate: String,
    /// The instance the four sources offered for it, nearest first.
    pub instance: Instance,
}

/// The source documentation `package_dir` adapts, located.
///
/// `Ok(None)` means the package declares no `[translates]` — it is a
/// source documentation, and there is nothing to look for.
///
/// One lookup, two callers: the mirror check ([`check`]) reads the source
/// to compare structure, and [`borrowed`] reads it for the example bodies
/// a translation's pages borrow. A second spelling of «which source does
/// this package adapt» would be the day one of them looked in a different
/// tree than the other reported on.
///
/// Both refusals are errors rather than a silent `None`: a coordinate that
/// is not `<group>/<name>` is a manifest defect, and a source no source
/// holds leaves a mirror check with no verdict to give — green would be
/// the worst of the three possible answers there.
///
/// ```
/// use vibe_doc::citations::SpecSources;
/// use vibe_doc::translations::adapted;
///
/// let dir = tempfile::tempdir().unwrap();
/// std::fs::write(
///     dir.path().join("vibe.toml"),
///     "[package]\nname = \"pair\"\ngroup = \"com.example.docs\"\n",
/// )
/// .unwrap();
///
/// // A source documentation adapts nothing, so there is nothing to find
/// // — and that is not a failure.
/// assert_eq!(adapted(dir.path(), &SpecSources::new())?, None);
/// # Ok::<(), vibe_doc::DocError>(())
/// ```
pub fn adapted(package_dir: &Path, sources: &SpecSources) -> Result<Option<Adapted>> {
    let Some((coordinate, _constraint)) = manifest::relations(package_dir, TRANSLATES)?
        .into_iter()
        .next()
    else {
        return Ok(None);
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
    Ok(Some(Adapted {
        coordinate,
        instance,
    }))
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
    let Some(Adapted {
        coordinate,
        instance,
    }) = adapted(package_dir, sources)?
    else {
        return Ok(Report::default());
    };

    let translation = pages::read_package(package_dir)?;
    let source = pages::read_package(&instance.root)?;
    let mut report = compare(&source, &translation);
    // The chapters are the manifests' business rather than the pages',
    // so they are compared from the two manifests and folded in here.
    report.problems.extend(unknown_chapters(
        &manifest::chapters(&instance.root)?,
        &manifest::chapters(package_dir)?,
    ));
    // The glossary is a manifest's statement too, and the same shape of
    // rule: one fact, declared on both sides, and a divergence that would
    // give the two languages two different vocabularies
    // (`##GLOSSARY-TRANSLATION`).
    report.problems.extend(unmirrored_glossary(
        crate::glossary::declared(&instance.root)?,
        crate::glossary::declared(package_dir)?,
    ));
    report.adapts = Some(coordinate);
    report.source = Some(instance.source);
    Ok(report)
}

/// The chapters a translation names that its source does not declare
/// (`##NAV-CHAPTERS-TRANSLATION`).
///
/// A translation renames the source's chapters and invents none: the
/// path is one fact, declared once, in the documentation being adapted.
/// A row under an id the source never used renames nothing — it is a
/// title nobody will ever show, which is how a chapter id renamed on one
/// side and not the other looks.
///
/// A source that declares no path at all makes every row of the
/// translation unknown, and that is the correct reading rather than a
/// special case: naming chapters of a path that does not exist is the
/// same defect written larger.
fn unknown_chapters(
    source: &[NavigationChapter],
    translation: &[NavigationChapter],
) -> Vec<Problem> {
    translation
        .iter()
        .filter(|row| !source.iter().any(|theirs| theirs.id == row.id))
        .map(|row| Problem::UnknownChapter { id: row.id.clone() })
        .collect()
}

/// The glossary a translation declares against its source's
/// (`##GLOSSARY-TRANSLATION`).
///
/// A translation declares the SAME `[glossary]` as the documentation it
/// adapts: the page addresses mirror file for file (`##LOC-MIRROR`), so the
/// adaptation's glossary is its own copy of that page with the terms in its
/// language. Declaring another page would give one manual two vocabularies
/// in two languages, and declaring none would leave the adaptation's
/// readers without the cards and its author without the term checks.
///
/// A source that declares none and a translation that declares one is
/// reported for the same reason read the other way round: the path is the
/// source's to decide, and an adaptation inventing one is an adaptation
/// deciding something about the documentation it adapts.
fn unmirrored_glossary(source: Option<String>, translation: Option<String>) -> Vec<Problem> {
    if source == translation {
        return Vec::new();
    }
    vec![Problem::Glossary {
        source,
        translation,
    }]
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

    compare_blocks(source, translation, report);
}

/// The count and the kinds of the blocks, number by number.
///
/// Matching anchors are not enough (the campaign's F-44): two pages can
/// agree on every anchor and still part on a paragraph the adaptation
/// merged into its neighbour, and then `#p12` means one thing in English
/// and another in Russian — which is exactly what the language selector
/// promises it does not (`##READER-LANGUAGE-SWITCH-KEEPS-PLACE`).
///
/// Only the FIRST divergence on a page is reported. After a block is
/// added or dropped every number below it differs, and a check that
/// listed all of them would bury the one line that says where to look.
fn compare_blocks(source: &Page, translation: &Page, report: &mut Report) {
    let theirs = block_kinds(&source.doc);
    let ours = block_kinds(&translation.doc);
    for i in 0..theirs.len().max(ours.len()) {
        let (a, b) = (theirs.get(i), ours.get(i));
        if a == b {
            continue;
        }
        report.problems.push(Problem::Block {
            page: translation.rel.clone(),
            at: Numbering::spell(i as u32 + 1),
            source: a.map(|k| (*k).to_owned()).unwrap_or_else(nothing),
            translation: b.map(|k| (*k).to_owned()).unwrap_or_else(nothing),
        });
        return;
    }
}

/// What a page carries at each of its block numbers, in the order
/// [`number_blocks`] assigns them — so position `i` here IS `p(i+1)`
/// there, and the `footnotes` section is outside both.
fn block_kinds(doc: &SpecDoc) -> Vec<&'static str> {
    let numbering = number_blocks(doc);
    (1..=numbering.len() as u32)
        .filter_map(|n| numbering.path_of(n))
        .filter_map(|path| block_at(doc, path))
        .map(|node| kind_of(&node.block))
        .collect()
}

/// The word for a block's kind, as a divergence names it.
///
/// `example` and `example ref` are ONE kind here, and that is the whole
/// point: a translation replaces an authored example with a reference by
/// law (`##LOC-EXAMPLE-REF`), so telling the two apart would paint every
/// correct adaptation red. The real violation — a translation that
/// authors its own — is caught by its own rule, where it can be explained
/// in words the author can act on.
fn kind_of(block: &Block) -> &'static str {
    match block {
        Block::Paragraph(_) => "p",
        Block::List { .. } => "list",
        Block::Table { .. } => "table",
        Block::Fence { .. } => "fence",
        Block::Quote(_) => "quote",
        Block::Example { .. } | Block::ExampleRef { .. } => "example",
        Block::Rule { .. } => "rule",
        Block::Derived { .. } => "derived",
        Block::Note { .. } => "note",
        Block::Figure { .. } => "figure",
        Block::Prompt { .. } => "prompt",
    }
}

/// What a page carries past its last block.
fn nothing() -> String {
    "nothing".to_owned()
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
