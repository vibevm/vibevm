//! The coverage gate — is everything the specifications promised to tell
//! actually told (PROP-057 `##OBS-COVERAGE-GATE`, PROP-047
//! `##DOC-COVERAGE-RATCHET`)?
//!
//! A spec fact marked `actionstage="doc"` **with an audience** is an
//! obligation: the corpus has said, in its own markup, that this rule
//! must eventually be told to those people. The gate asks one question of
//! each obligation — does a page written for that same audience cite it —
//! and answers per audience, because «told» is a relation between a rule
//! and a reader, never a property of the rule alone.
//!
//! ## Why this is a gate and not a table of contents
//!
//! `vibe progress report --view doc --audience …` LISTS the obligations;
//! it does not close them (PROP-043 `##AUDIENCE-DOC-USE`). A navigation
//! built from that list would answer «what did somebody remember to
//! write», which is the question a manual's own table of contents already
//! answers badly. The gate answers «is everything told», and the only way
//! to move it is to write the page — `##OBS-COVERAGE-GATE` says the site's
//! navigation comes from the page manifest and never from this report.
//!
//! ## Where the two halves come from
//!
//! The obligations come from the CALLER, as parsed documents. The corpus
//! is defined by the project's `facts.toml` — its include globs, its
//! exclusions, its vocabulary dispatch — and that definition has one home
//! in this tree, the grounding cell every `vibe facts` verb enters
//! through. Re-deriving it here would be a second opinion about which
//! files the project observes, and the day the two disagreed the gate
//! would be measuring a corpus nobody else can see.
//!
//! The citations come from the package's own pages, read directly
//! ([`crate::citations::edges`]) rather than from a committed map. A gate
//! that reads a generated artefact is green whenever the artefact is
//! stale or empty; this one cannot be green by emptiness, because an
//! unread page is a reported page and an uncited obligation has nowhere
//! to hide.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#OBS-COVERAGE-GATE");

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use progress_core::doc::ParsedDoc;
use progress_core::model::{Audience, Stage};

use crate::citations::{self, SpecSources};
use crate::error::Result;
use crate::manifest::page;
use crate::pages;

/// The threshold a run demands unless the caller lowers it: everything
/// promised is told. `--min` exists for the intermediate runs of a
/// campaign that is still writing the pages, never as the standing bar
/// (`##DOC-COVERAGE-RATCHET` — a ratchet only ever tightens).
pub const FULL_COVERAGE: u8 = 100;

/// One promise the specifications made, and to whom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Obligation {
    /// The observed file that states it, relative to the corpus root and
    /// forward-slashed — the spelling `progress-core` addresses it by.
    pub path: String,
    /// 1-based line of the fact.
    pub line: usize,
    /// The fact's own anchor.
    pub anchor: String,
    /// `<path>#<anchor>` — the address the obligation is listed under.
    pub address: String,
    /// Every audience the marker names. An obligation with no audience is
    /// not an obligation: `##OBS-COVERAGE-GATE` binds the promise to the
    /// people it was made to, and the vocabulary's silent default (`dev`)
    /// is the absence of a promise, not a promise to developers.
    pub audiences: Vec<Audience>,
}

/// Why one (obligation, audience) pair is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapReason {
    /// No page of this documentation cites the address at all.
    Uncited,
    /// Pages cite it, but none of them speaks to this audience. The rule
    /// is explained somewhere and not to the people it was promised to,
    /// which is a different repair: mark the page, or write another.
    WrongAudience,
}

impl GapReason {
    pub fn as_str(self) -> &'static str {
        match self {
            GapReason::Uncited => "no page cites it",
            GapReason::WrongAudience => "cited, but by no page for this audience",
        }
    }
}

/// One open promise: an obligation and the audience it is still owed to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gap {
    pub address: String,
    pub path: String,
    pub line: usize,
    pub audience: Audience,
    pub reason: GapReason,
    /// The pages that DO cite it, when some do. Empty for
    /// [`GapReason::Uncited`]; naming them is what turns
    /// «cited by nobody for `user`» into a one-line repair.
    pub cited_by: Vec<String>,
}

/// One `--coverage` run.
#[derive(Debug, Clone)]
pub struct Report {
    /// Every obligation the corpus states, in corpus order.
    pub obligations: Vec<Obligation>,
    /// Per audience: how many pairs were owed and how many are met.
    pub per_audience: Vec<AudienceTally>,
    /// The open pairs, in corpus order then vocabulary order.
    pub gaps: Vec<Gap>,
    /// Pages the pivot refused: a page that does not parse cites nothing,
    /// so it is named rather than counted as covering nothing.
    pub unreadable: Vec<String>,
    /// The percentage this run demanded.
    pub min_percent: u8,
}

/// One audience's row of the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudienceTally {
    pub audience: Audience,
    /// Obligations owed to this audience.
    pub owed: usize,
    /// How many of them a page for this audience cites.
    pub covered: usize,
}

impl AudienceTally {
    /// Whole percent, rounded down; an audience nothing is owed to is
    /// complete by definition rather than divided by zero.
    pub fn percent(&self) -> u8 {
        percent(self.covered, self.owed)
    }
}

impl Report {
    /// Pairs owed across every audience — the denominator of the gate.
    pub fn owed(&self) -> usize {
        self.per_audience.iter().map(|a| a.owed).sum()
    }

    /// Pairs met.
    pub fn covered(&self) -> usize {
        self.per_audience.iter().map(|a| a.covered).sum()
    }

    /// The run's percentage, whole and rounded down. A tree that owes
    /// nothing is complete: there is no promise left untold.
    pub fn percent(&self) -> u8 {
        percent(self.covered(), self.owed())
    }

    /// Green when the coverage reaches the threshold and every page was
    /// readable. An unreadable page is never a pass: its citations are
    /// unknown, and unknown is not covered.
    pub fn ok(&self) -> bool {
        self.unreadable.is_empty() && self.percent() >= self.min_percent
    }

    /// The human form: the open pairs first, then a row per audience,
    /// then the one line a gate prints.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for gap in &self.gaps {
            out.push_str(&format!(
                "  UNCOVERED {} — {}\n    {}:{} — {}\n",
                gap.audience.as_str(),
                gap.address,
                gap.path,
                gap.line,
                gap.reason.as_str()
            ));
            if !gap.cited_by.is_empty() {
                out.push_str(&format!("    cited by: {}\n", gap.cited_by.join(", ")));
            }
        }
        for page in &self.unreadable {
            out.push_str(&format!("  unreadable {page}\n"));
        }
        // Every audience of the vocabulary gets a row, including the ones
        // owed nothing: «author 0/0» is the answer to «did anybody promise
        // authors anything», and a missing row would read as a zero
        // somebody forgot to print.
        for tally in &self.per_audience {
            out.push_str(&format!(
                "  {:<7} {}/{} ({}%)\n",
                tally.audience.as_str(),
                tally.covered,
                tally.owed,
                tally.percent()
            ));
        }
        out.push_str(&format!(
            "coverage: {} of {} obligation(s) told, {}% of {} audience pair(s) \
             (threshold {}%), {} unreadable page(s)\n",
            self.obligations.len() - uncovered_obligations(self),
            self.obligations.len(),
            self.percent(),
            self.owed(),
            self.min_percent,
            self.unreadable.len()
        ));
        out
    }
}

/// How many obligations are open for at least one of their audiences.
fn uncovered_obligations(report: &Report) -> usize {
    let mut seen: Vec<&str> = Vec::new();
    for gap in &report.gaps {
        if !seen.contains(&gap.address.as_str()) {
            seen.push(&gap.address);
        }
    }
    seen.len()
}

/// Whole percent, rounded down, with «nothing owed» reading as complete.
fn percent(covered: usize, owed: usize) -> u8 {
    if owed == 0 {
        return 100;
    }
    ((covered * 100) / owed) as u8
}

/// The obligations a parsed corpus states.
///
/// A fact becomes one when its whole-unit marker carries
/// `actionstage="doc"` AND names at least one audience. Both halves are
/// the norm's: the stage says the promise is a documentation promise, the
/// audience says who it was made to, and a gate over promises made to
/// nobody in particular would measure the corpus's default rather than
/// anyone's decision.
///
/// ```
/// use progress_core::parse::parse_document;
///
/// let text = concat!(
///     "## Rules {#r}\n\n",
///     "@fact:CARD-FIELDS A doc package declares a title. ",
///     "<status stage=\"spec\" state=\"done\" action=\"continue\" ",
///     "actionstage=\"doc\" audience=\"author\"/>\n",
/// );
/// let doc = parse_document("vibevm/vibespecs/common/PROP-057.md", text);
/// let owed = vibe_doc::coverage::obligations([&doc]);
/// assert_eq!(owed.len(), 1);
/// assert_eq!(owed[0].anchor, "CARD-FIELDS");
/// assert_eq!(owed[0].audiences, vec![progress_core::model::Audience::Author]);
/// ```
pub fn obligations<'a>(docs: impl IntoIterator<Item = &'a ParsedDoc>) -> Vec<Obligation> {
    let mut out = Vec::new();
    for doc in docs {
        for fact in doc.blocks.iter().flat_map(|block| &block.facts) {
            let Some(anchor) = fact.id.clone() else {
                continue;
            };
            let Some(marker) = fact
                .marker_index
                .and_then(|index| doc.markers.get(index))
                .filter(|m| m.actionstage == Some(Stage::Doc))
                .filter(|m| !m.audience.is_empty())
            else {
                continue;
            };
            // The vocabulary's own order, deduplicated: a marker may name
            // one audience twice, and a report that printed it twice would
            // owe the same promise twice.
            let audiences: Vec<Audience> = Audience::ALL
                .into_iter()
                .filter(|a| marker.audience.contains(a))
                .collect();
            out.push(Obligation {
                address: format!("{}#{anchor}", doc.path),
                path: doc.path.clone(),
                line: fact.line,
                anchor,
                audiences,
            });
        }
    }
    out
}

/// Run the coverage gate for the documentation package at `package_dir`.
///
/// `corpus_root` is the tree the obligations were parsed from — their
/// `path` is relative to it, and an edge resolves into it or into some
/// other package entirely. `min_percent` is the bar; [`FULL_COVERAGE`] is
/// the standing one.
pub fn check(
    package_dir: &Path,
    coordinate: &str,
    sources: &SpecSources,
    corpus_root: &Path,
    obligations: Vec<Obligation>,
    min_percent: u8,
) -> Result<Report> {
    let set = pages::read_package(package_dir)?;
    // Which audiences each page speaks to, by the page's own markup —
    // the same reading the manifest does, so navigation and gate cannot
    // disagree about who a page is for (`##CARD-NO-AUDIENCE-DECLARATION`).
    let page_audiences: BTreeMap<String, Vec<Audience>> = set
        .pages
        .iter()
        .map(|p| (p.rel.clone(), page::audiences(&p.doc)))
        .collect();

    // `<canonical file>#<anchor>` → the pages that cite it. The file is
    // canonicalised on BOTH sides because an address resolves to an
    // absolute path and an obligation carries a repo-relative one; on
    // Windows the two spellings of one file differ in more than the
    // separator.
    let mut cited: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut located: BTreeMap<String, Option<String>> = BTreeMap::new();
    for edge in citations::edges(package_dir, coordinate)? {
        let Some(key) = edge_key(&edge.uri, sources, &mut located) else {
            continue;
        };
        let pages = cited.entry(key).or_default();
        if !pages.contains(&edge.page) {
            pages.push(edge.page.clone());
        }
    }

    let mut tallies: Vec<AudienceTally> = Audience::ALL
        .into_iter()
        .map(|audience| AudienceTally {
            audience,
            owed: 0,
            covered: 0,
        })
        .collect();
    let mut gaps = Vec::new();
    for obligation in &obligations {
        let key = canonical_key(&corpus_root.join(&obligation.path), &obligation.anchor);
        let citing = key.and_then(|k| cited.get(&k)).cloned().unwrap_or_default();
        for audience in &obligation.audiences {
            // The rows were built from `Audience::ALL` a few lines up,
            // so every value of the closed vocabulary has one and this
            // arm cannot be taken. `continue` is its total spelling:
            // there is nothing to count against a row that is not there,
            // and a panic here would be a refusal to report the other
            // ninety obligations over an impossibility.
            let Some(tally) = tallies.iter_mut().find(|t| t.audience == *audience) else {
                continue;
            };
            tally.owed += 1;
            let covered = citing.iter().any(|page| {
                page_audiences
                    .get(page)
                    .is_some_and(|speaks| speaks.contains(audience))
            });
            if covered {
                tally.covered += 1;
                continue;
            }
            gaps.push(Gap {
                address: obligation.address.clone(),
                path: obligation.path.clone(),
                line: obligation.line,
                audience: *audience,
                reason: if citing.is_empty() {
                    GapReason::Uncited
                } else {
                    GapReason::WrongAudience
                },
                cited_by: citing.clone(),
            });
        }
    }

    Ok(Report {
        obligations,
        per_audience: tallies,
        gaps,
        unreadable: set.unreadable.iter().map(|u| u.rel.clone()).collect(),
        min_percent,
    })
}

/// The `<canonical file>#<anchor>` key one cited address resolves to, or
/// `None` when no source holds it. The per-address answer is memoised:
/// a manual cites one specification from many pages, and canonicalising
/// a path is a filesystem call.
fn edge_key(
    uri: &str,
    sources: &SpecSources,
    located: &mut BTreeMap<String, Option<String>>,
) -> Option<String> {
    if let Some(known) = located.get(uri) {
        return known.clone();
    }
    let key = vibe_spec::SpecAddress::parse(uri).ok().and_then(|address| {
        let found = sources.locate(&address).ok()?;
        canonical_key(&found.path, &address.anchor.join("."))
    });
    located.insert(uri.to_owned(), key.clone());
    key
}

/// `<canonicalised path>#<anchor>`, or `None` when the path does not
/// exist. A path that cannot be canonicalised names no file, and two
/// names that do not both resolve are not the same file.
fn canonical_key(path: &Path, anchor: &str) -> Option<String> {
    let real: PathBuf = std::fs::canonicalize(path).ok()?;
    Some(format!("{}#{anchor}", real.to_string_lossy()))
}

#[cfg(test)]
mod tests;
