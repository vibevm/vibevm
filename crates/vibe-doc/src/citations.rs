//! `rule` citations — resolving `spec://` addresses against the specs that
//! exist right now (PROP-057 `##OBS-RULE-EDGE-UNPINNED`, PROP-045
//! `##DOC-VOCAB-RULE-ADDRESS`).
//!
//! A `<rule ref="spec://…#ANCHOR"/>` is an insertion point, never a copy.
//! The page names an address; the pipeline fetches the fact's CURRENT text
//! at every render and puts it on the page. That is the whole mechanism,
//! and it is what makes a quoted rule unable to go stale: there is no
//! stored copy to go stale.
//!
//! ## What this module will not do
//!
//! It carries no revision, no content hash and no notion of «the spec
//! moved ahead». Those checks need a history the project does not keep by
//! design (`##OBS-VERSION-CONTRACT`), and an author who sees «this quote
//! is from r3, the rule is at r5» learns nothing they can act on. So
//! [`check`] asks exactly one question — **does the anchor exist** — and a
//! vanished anchor fails the build. Whether the PROSE around a citation
//! went stale is a human's question at a full reconciliation, not a
//! machine's (`##OBS-PROSE-STALENESS-IS-HUMAN`).
//!
//! The same asymmetry governs the edge the scanner mints for each `rule`:
//! it carries the address and no pin, so it can never go «suspect». A pin
//! that strays into the attribute is recorded by the pivot so the author's
//! bytes survive a round trip, and is never honoured here.
//!
//! ## Not every `spec://` in a manual is a citation
//!
//! A manual TALKS about addresses as well as using them, and the
//! difference is not cosmetic: an address in an illustration must not be
//! resolved, and an address in a `rule` must. Three forms are prose, not
//! citations ([`Classification`]): an address with an ellipsis or an
//! `<…>` placeholder in it, an address in the reserved teaching group
//! `org.acme`, and the manual's own pages, which resolve inside the
//! documentation package rather than against a specification.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#OBS-RULE-EDGE-UNPINNED");

pub mod anchor;
pub mod scan;
pub mod sources;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::error::{DocError, Result};
use crate::pages::{self, Page, PageSet};

pub use anchor::anchor_text;
pub use scan::rule_uris;
pub use sources::{Source, SpecSources};

use anchor::read_spec;
use scan::{prose_uris, rule_lines};

/// The reserved teaching group. An address under it is an illustration in
/// somebody's manual, never a package anyone published
/// (PROP-029 uses `com.example.shop` for the same purpose; the manual's
/// own pages settled on `org.acme`).
pub const TEACHING_GROUP: &str = "org.acme";

/// The projects the official manual's tutorials create. A project that
/// adopts a traceability map mints addresses under its own name,
/// `spec://hello-vibevm/modules/calculator/PROP-001#…`, and those exist
/// only in the reader's copy of the tutorial's project, never in a
/// package anyone could resolve. A page that shows the reader the exact
/// address to type is illustrating, not citing, exactly as an address
/// under [`TEACHING_GROUP`] is.
pub const TEACHING_PROJECTS: &[&str] = &["hello-vibe", "hello-vibevm"];

/// What an address found in a documentation package IS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classification {
    /// A real citation: it must resolve, and a `documents` edge is minted
    /// for it.
    Citation,
    /// An address written with an ellipsis or an `<…>` placeholder — the
    /// manual showing the SHAPE of an address, not using one.
    Placeholder,
    /// An address in the reserved teaching group [`TEACHING_GROUP`], or
    /// under the name of a tutorial's project [`TEACHING_PROJECTS`].
    Teaching,
    /// The manual addressing its own pages. It resolves inside the
    /// documentation package's tree, not against a specification.
    SelfAddress,
}

impl Classification {
    /// The word a report prints.
    pub fn as_str(self) -> &'static str {
        match self {
            Classification::Citation => "citation",
            Classification::Placeholder => "placeholder",
            Classification::Teaching => "teaching",
            Classification::SelfAddress => "self-address",
        }
    }
}

/// Where a citation was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// A `<rule ref="…"/>` block — the citation mechanism proper.
    Rule,
    /// An address inside the prose of a unit. It is checked, because a
    /// dead address is dead wherever it is written, but it mints no edge:
    /// the `documents` relation is what a `rule` declares.
    Prose,
}

/// One address found on one page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Citation {
    /// The page's address inside the package (`model/boot-lane.xml`).
    pub page: String,
    /// The 1-based line the address was written on, or `0` when the line
    /// could not be located in the page's own bytes.
    pub line: u32,
    /// The address, revision pin already dropped by the pivot.
    pub uri: String,
    pub origin: Origin,
    pub class: Classification,
}

/// A resolved citation: the fact's text as it stands right now, and where
/// it was read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleText {
    /// The address that was resolved.
    pub uri: String,
    /// The anchor inside the document (`PIPE-LIBRARY`, or the dotted tree
    /// path spelled with its `.` separators).
    pub anchor: String,
    /// The fact's own text, verbatim from the pivot — inline Markdown
    /// conventions ride inside it exactly as the spec's author wrote them.
    pub text: String,
    /// The language the cited SPECIFICATION is written in — never the
    /// language of the page quoting it. A translated page quotes an
    /// English rule in English, and says so.
    pub lang: String,
    /// Which of the four sources answered.
    pub source: Source,
    /// The file the text came from.
    pub path: PathBuf,
}

/// Why one address did not resolve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unresolved {
    pub page: String,
    pub line: u32,
    pub uri: String,
    pub origin: Origin,
    /// The refusal, in the words of whichever layer refused.
    pub reason: String,
}

/// One `--citations` run.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// Every address the walk found, in page then document order.
    pub citations: Vec<Citation>,
    /// The ones that did not resolve. Empty is the only green state.
    pub unresolved: Vec<Unresolved>,
    /// Pages the pivot refused: an unreadable page hides its citations,
    /// so it is reported rather than counted as clean.
    pub unreadable: Vec<String>,
}

impl Report {
    /// Green when every real citation resolved and every page was
    /// readable.
    pub fn ok(&self) -> bool {
        self.unresolved.is_empty() && self.unreadable.is_empty()
    }

    /// How many of each class the walk found.
    pub fn counts(&self) -> Counts {
        let mut c = Counts::default();
        for citation in &self.citations {
            match citation.class {
                Classification::Citation => c.citations += 1,
                Classification::Placeholder => c.placeholders += 1,
                Classification::Teaching => c.teaching += 1,
                Classification::SelfAddress => c.self_addresses += 1,
            }
            if citation.origin == Origin::Rule {
                c.rules += 1;
            }
        }
        c.unresolved = self.unresolved.len();
        c
    }

    /// The human form.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for bad in &self.unresolved {
            out.push_str(&format!(
                "  UNRESOLVED {}:{} — `{}`\n    {}\n",
                bad.page, bad.line, bad.uri, bad.reason
            ));
        }
        for page in &self.unreadable {
            out.push_str(&format!("  unreadable {page}\n"));
        }
        let c = self.counts();
        // Each class is printed as its own number and none is subtracted
        // from another: a citation and a self-address are resolved by
        // different machinery, and one arithmetic line mixing them would
        // be wrong in exactly the case somebody reads it for.
        out.push_str(&format!(
            "citations: {} rule(s), {} address(es) to resolve, {} self-address(es), \
             {} unresolved, {} placeholder(s), {} teaching, {} unreadable page(s)\n",
            c.rules,
            c.citations,
            c.self_addresses,
            c.unresolved,
            c.placeholders,
            c.teaching,
            self.unreadable.len()
        ));
        out
    }
}

/// The counted shape of a `--citations` run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    /// Addresses that must resolve against a specification.
    pub citations: usize,
    /// Addresses the manual writes to its own pages; they resolve inside
    /// the documentation package.
    pub self_addresses: usize,
    /// Everything above that did NOT resolve, of either kind.
    pub unresolved: usize,
    pub placeholders: usize,
    pub teaching: usize,
    /// How many of all classes came from a `<rule>` block.
    pub rules: usize,
}

/// Classify one address (see the module's «not every `spec://`» note).
///
/// `self_coordinate` is the documenting package's own `<group>/<name>`;
/// an address under it is the manual pointing at itself.
///
/// ```
/// use vibe_doc::citations::{classify, Classification};
///
/// assert_eq!(
///     classify("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-LIBRARY", "org.vibevm.core/vibevm-docs"),
///     Classification::Citation
/// );
/// assert_eq!(classify("spec://…#ANCHOR", "x/y"), Classification::Placeholder);
/// assert_eq!(
///     classify("spec://org.acme/notes/flows/notes/PROTOCOL#RETRY", "x/y"),
///     Classification::Teaching
/// );
/// assert_eq!(
///     classify("spec://org.vibevm.core/vibevm-docs/model/boot-lane#p7", "org.vibevm.core/vibevm-docs"),
///     Classification::SelfAddress
/// );
/// ```
pub fn classify(uri: &str, self_coordinate: &str) -> Classification {
    // The ellipsis and the angle-bracket placeholder are the two forms
    // the manual uses to show an address without writing one. Both are
    // checked FIRST: `spec://org.acme/…/X` is a placeholder about a
    // teaching address, and the placeholder half is what makes it
    // unresolvable.
    //
    // The angle brackets are matched in BOTH spellings, because a page is
    // XML: an author who writes `<page>` in prose has to escape it, and
    // the raw bytes this walk reads carry `&lt;page&gt;`. Reading only the
    // literal form would leave every escaped placeholder looking like a
    // citation to a package called `&lt;group&gt;`.
    const PLACEHOLDER_MARKS: &[&str] = &["…", "...", "<", ">", "&lt;", "&gt;"];
    if PLACEHOLDER_MARKS.iter().any(|mark| uri.contains(mark)) {
        return Classification::Placeholder;
    }
    let Some(body) = uri.strip_prefix("spec://") else {
        return Classification::Placeholder;
    };
    if body.starts_with(&format!("{TEACHING_GROUP}/")) {
        return Classification::Teaching;
    }
    if TEACHING_PROJECTS
        .iter()
        .any(|project| body.starts_with(&format!("{project}/")))
    {
        return Classification::Teaching;
    }
    if body.starts_with(&format!("{self_coordinate}/")) {
        return Classification::SelfAddress;
    }
    Classification::Citation
}

/// Resolve one citation to the fact's current text.
pub fn resolve(uri: &str, sources: &SpecSources) -> std::result::Result<RuleText, String> {
    let address = vibe_spec::SpecAddress::parse(uri).map_err(|e| e.to_string())?;
    if address.anchor.is_empty() {
        return Err(format!(
            "`{uri}` names a document but no anchor — a `rule` cites one fact, \
             not a whole specification"
        ));
    }
    let found = sources.locate(&address)?;
    let doc = read_spec(&found.path)?;
    let anchor = address.anchor.join(".");
    let text = anchor_text(&doc, &address.anchor).ok_or_else(|| {
        format!(
            "`{}` carries no anchor `{anchor}` — the rule was renamed or removed \
             (a rename leaves a tombstone; see \
             spec://org.vibevm.core/vibevm/common/PROP-057#INV-ANCHORS-IMMUTABLE)",
            found.display_path()
        )
    })?;
    Ok(RuleText {
        uri: uri.to_owned(),
        anchor,
        text,
        lang: found.lang,
        source: found.source,
        path: found.path,
    })
}

/// Resolve every `rule` citation of a package, by address.
///
/// The map is what a backend renders from: a `rule` block holds an
/// address, and the text beside it is the fact as it stands at this
/// build. An address that does not resolve is simply absent from the map
/// — [`check`] is where that is a failure; a renderer's job is to say so
/// on the page, not to abort the render.
pub fn resolve_rules(set: &PageSet, sources: &SpecSources) -> BTreeMap<String, RuleText> {
    let mut out = BTreeMap::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for page in &set.pages {
        for uri in rule_uris(&page.doc) {
            if !seen.insert(uri.clone()) {
                continue;
            }
            if let Ok(text) = resolve(&uri, sources) {
                out.insert(uri, text);
            }
        }
    }
    out
}

/// One `documents` edge a page mints: the tail is the page, the head is
/// the address, and there is NO pin — an unpinned edge cannot go suspect,
/// which is exactly the property a live citation needs
/// (`##OBS-RULE-EDGE-UNPINNED`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocEdge {
    /// The page's path relative to the PACKAGE root, forward-slashed —
    /// the form a map's `file` field carries.
    pub file: String,
    /// The page's address inside the spec root (`model/boot-lane.xml`).
    pub page: String,
    /// The 1-based line of the `<rule>` element, or `0` when it could not
    /// be located.
    pub line: u32,
    /// The cited address, unpinned.
    pub uri: String,
}

/// Every `documents` edge a documentation package's pages declare, in page
/// then document order.
///
/// Only `rule` blocks mint edges. An address in prose is checked, because
/// a dead address is dead wherever it is written, but the `documents`
/// relation is a declaration and `rule` is how a page declares it.
pub fn edges(package_dir: &Path, self_coordinate: &str) -> Result<Vec<DocEdge>> {
    let set = pages::read_package(package_dir)?;
    let mut out = Vec::new();
    for page in &set.pages {
        let raw = std::fs::read_to_string(&page.path)
            .map_err(|e| DocError::io("reading", &page.path, e))?;
        let uris = rule_uris(&page.doc);
        let lines = rule_lines(&raw, &uris);
        for (uri, line) in uris.into_iter().zip(lines) {
            if classify(&uri, self_coordinate) != Classification::Citation {
                continue;
            }
            out.push(DocEdge {
                file: format!("{}/{}", pages::SPEC_ROOT, page.rel),
                page: page.rel.clone(),
                line,
                uri,
            });
        }
    }
    Ok(out)
}

/// Check every address a documentation package writes.
///
/// One question per address, and no other: does the anchor exist. A
/// placeholder, a teaching address and a self-address are counted and
/// passed over — resolving them is what would be wrong.
pub fn check(package_dir: &Path, self_coordinate: &str, sources: &SpecSources) -> Result<Report> {
    let set = pages::read_package(package_dir)?;
    let mut report = Report::default();
    for page in &set.unreadable {
        report.unreadable.push(page.rel.clone());
    }
    for page in &set.pages {
        let raw = std::fs::read_to_string(&page.path)
            .map_err(|e| DocError::io("reading", &page.path, e))?;
        for found in page_citations(page, &raw, self_coordinate) {
            if found.class == Classification::Citation
                && let Err(reason) = resolve(&found.uri, sources)
            {
                report.unresolved.push(Unresolved {
                    page: found.page.clone(),
                    line: found.line,
                    uri: found.uri.clone(),
                    origin: found.origin,
                    reason,
                });
            }
            if found.class == Classification::SelfAddress
                && let Err(reason) = resolve_self(package_dir, &found.uri, self_coordinate)
            {
                report.unresolved.push(Unresolved {
                    page: found.page.clone(),
                    line: found.line,
                    uri: found.uri.clone(),
                    origin: found.origin,
                    reason,
                });
            }
            report.citations.push(found);
        }
    }
    Ok(report)
}

/// A self-address resolves against the documentation package's own tree:
/// the page must exist. Its FRAGMENT is deliberately not checked when it
/// is a positional `pNN` — those live by the current text and mean
/// nothing outside a build (PROP-057 `##READER-NUMBERED-BLOCKS`).
fn resolve_self(
    package_dir: &Path,
    uri: &str,
    self_coordinate: &str,
) -> std::result::Result<(), String> {
    let rest = uri
        .strip_prefix("spec://")
        .and_then(|b| b.strip_prefix(self_coordinate))
        .and_then(|b| b.strip_prefix('/'))
        .ok_or_else(|| format!("`{uri}` is not an address under `{self_coordinate}`"))?;
    let (doc_path, anchor) = match rest.split_once('#') {
        Some((p, a)) => (p, Some(a)),
        None => (rest, None),
    };
    if doc_path.is_empty() {
        // `spec://<self>/` — the package itself, not a page. The manual
        // writes this when it names its own coordinate in prose.
        return Ok(());
    }
    let path = package_dir
        .join(pages::SPEC_ROOT)
        .join(format!("{doc_path}.xml"));
    if !path.is_file() {
        return Err(format!(
            "`{uri}` names no page of this package — `{}/{doc_path}.xml` does not exist",
            pages::SPEC_ROOT
        ));
    }
    let Some(anchor) = anchor else {
        return Ok(());
    };
    if is_positional(anchor) {
        return Ok(());
    }
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let doc = vibe_specdoc::from_xml_with(&text, vibe_specdoc::Vocabulary::Doc)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let path_segments: Vec<String> = anchor.split('.').map(str::to_owned).collect();
    anchor_text(&doc, &path_segments)
        .map(|_| ())
        .ok_or_else(|| format!("`{uri}` names no anchor `{anchor}` on that page"))
}

/// A positional block number (`p07`) — an address into the CURRENT text,
/// assigned at build and never an anchor of the page's own.
fn is_positional(anchor: &str) -> bool {
    anchor
        .strip_prefix('p')
        .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()))
}

/// Every address one page writes — its `rule` blocks first, then the
/// addresses that ride inside its prose.
fn page_citations(page: &Page, raw: &str, self_coordinate: &str) -> Vec<Citation> {
    let mut out = Vec::new();
    let uris = rule_uris(&page.doc);
    let lines = rule_lines(raw, &uris);
    for (uri, line) in uris.iter().zip(&lines) {
        out.push(Citation {
            page: page.rel.clone(),
            line: *line,
            class: classify(uri, self_coordinate),
            uri: uri.clone(),
            origin: Origin::Rule,
        });
    }
    for (uri, line) in prose_uris(raw) {
        out.push(Citation {
            page: page.rel.clone(),
            line,
            class: classify(&uri, self_coordinate),
            uri,
            origin: Origin::Prose,
        });
    }
    out
}

#[cfg(test)]
mod tests;
