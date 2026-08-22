//! XML side of the scanner: anchored spec units out of dialect-XML sources
//! (PROP-045 §2 — the same unit grammar `mdspec` reads out of Markdown).
//!
//! A unit is the span from an anchored generic `<section id=…>` or named
//! `<anchor title=…>` section (or `<title id=…>`) through everything it
//! governs — nesting IS the heading hierarchy, so a section's span holds its
//! own blocks and every nested section, exactly as a Markdown span runs to
//! the next same-or-higher heading. Fact units mint from generic `<fact id=…>`
//! or named `<ID fact="true">` elements inside `<p>` and `<item>` — the
//! first-token grain the Markdown scanner reads; a
//! `<td>` or `<quote>` fact sits below that grain and mints nothing here,
//! mirroring `mdspec` over the projection. Section ids and fact ids share ONE
//! duplicate namespace per document.
//!
//! Positions are NATIVE: the 1-based line of the element in the XML source
//! (PROP-045 §4 — the engine side lives without the projection caveat the
//! host readers carry). A unit's content hash is measured over the canonical
//! Markdown projection of its span (`mdout` states the verdict).
//!
//! The dialect is closed (a foreign element, attribute, DTD, PI or entity
//! is a loud `xml-dialect` warning and the document is dropped, never a
//! silent skip) — the engine idiom for a hard authoring error: the scanner
//! degrades, the warning names the construct and its native line.

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#spec-units");

use crate::config::SectionGrain;
use crate::generated::specmap::{SpecUnit, Warning};
use specmark_grammar::{is_valid_anchor, is_valid_fact_id};

mod blocks;
mod doc;
mod mdout;
mod reader;

use crate::mdspec::{KindLine, canonical_doc_path, duplicate_anchor_warning, parse_kind_line};
use mdout::{document_lines, item_line, para_line, section_lines, typed_id_after, unit_body};

/// The parsed document the reader produces (structure only — minting and
/// the duplicate namespace live in this module).
#[derive(Default)]
pub(super) struct XDoc {
    title: Option<XTitle>,
    status: Option<XStatus>,
    preamble: Vec<XBlock>,
    sections: Vec<XSection>,
    /// Native line of `</spec>` (the whole document's span end).
    end_line: u32,
}

pub(super) struct XTitle {
    text: String,
    id: Option<String>,
    line: u32,
}

pub(super) struct XSection {
    id: Option<String>,
    title: String,
    /// Native line of the opening tag.
    line: u32,
    /// Native line of the closing tag (inclusive span end).
    end_line: u32,
    status: Option<XStatus>,
    blocks: Vec<XBlock>,
    sections: Vec<XSection>,
}

pub(super) enum XBlock {
    Para {
        unit: XUnit,
        line: u32,
    },
    Quote(XUnit),
    List {
        ordered: bool,
        items: Vec<XUnit>,
    },
    Table {
        rows: Vec<Vec<XUnit>>,
    },
    Fence {
        lang: Option<String>,
        fact: Option<String>,
        text: String,
        line: u32,
    },
}

pub(super) struct XUnit {
    fact: Option<XFact>,
    text: String,
}

pub(super) struct XFact {
    id: Option<String>,
    status: Option<XStatus>,
    /// Native line of the `<fact>` element (the anchor's own line).
    line: u32,
}

impl XFact {
    /// A fact worth serialising: an id, a status, or both.
    fn is_meaningful(&self) -> bool {
        self.id.is_some() || self.status.is_some()
    }
}

/// The `<status>` attribute payload — values pass through verbatim (the
/// vocabulary check is the authoring frontend's); the emitter renders them
/// in the canonical fixed order.
pub(super) struct XStatus {
    stage: String,
    state: String,
    action: Option<String>,
    actionstage: Option<String>,
    audience: Vec<String>,
    comment: Option<String>,
    r#ref: Option<String>,
}

/// Parse one dialect-XML document into units + warnings — the same seam as
/// [`crate::mdspec::parse_units`]. The `long-section` quality check is
/// disabled here (threshold `0`), matching that test seam.
pub fn parse_units(file: &str, xml: &str, namespace: &str) -> (Vec<SpecUnit>, Vec<Warning>) {
    parse_units_with(file, xml, namespace, 0, SectionGrain::Leaf)
}

/// [`parse_units`] carrying the `long-section` quality policy, measured over
/// the section's NATIVE XML lines (`</section>` line minus `<section>` line,
/// inclusive) — the walk entry point routes here with the live config.
pub(crate) fn parse_units_with(
    file: &str,
    xml: &str,
    namespace: &str,
    max_section_lines: usize,
    grain: SectionGrain,
) -> (Vec<SpecUnit>, Vec<Warning>) {
    let mut m = Minter {
        file,
        namespace,
        doc_path: canonical_doc_path(file),
        seen_anchors: Vec::new(),
        units: Vec::new(),
        warnings: Vec::new(),
    };
    match reader::read_document(xml) {
        Ok(doc) => m.walk(&doc, max_section_lines, grain),
        Err(v) => m.warn("xml-dialect", v.message, v.line),
    }
    (m.units, m.warnings)
}

/// The minting pass: walks the parsed tree in document order, keeps the
/// one-per-document anchor namespace, and emits units + warnings.
struct Minter<'a> {
    file: &'a str,
    namespace: &'a str,
    doc_path: String,
    seen_anchors: Vec<String>,
    units: Vec<SpecUnit>,
    warnings: Vec<Warning>,
}

impl Minter<'_> {
    fn walk(&mut self, doc: &XDoc, max_section_lines: usize, grain: SectionGrain) {
        if let Some(title) = &doc.title
            && let Some(id) = &title.id
        {
            if !is_valid_anchor(id) {
                self.warn(
                    "invalid-anchor",
                    format!(
                        "anchor `{{#{id}}}` is not an id `[A-Za-z][A-Za-z0-9_-]*`; unit skipped"
                    ),
                    title.line,
                );
            } else {
                self.register(id, title.line);
                let span = document_lines(doc).join("\n");
                let kind = self.kind_of(doc.status.as_ref(), &doc.preamble);
                // The whole document is the title unit's span; its leaf test
                // is "no section in the body", as with an H1.
                self.long_section(
                    title.line,
                    doc.end_line,
                    &title.text,
                    max_section_lines,
                    grain,
                    doc.sections.is_empty(),
                );
                self.units
                    .push(self.unit(id.clone(), &title.text, &span, title.line, kind));
            }
        }
        self.block_facts(&doc.preamble);
        for s in &doc.sections {
            self.section(s, 2, max_section_lines, grain);
        }
    }

    fn section(
        &mut self,
        s: &XSection,
        level: usize,
        max_section_lines: usize,
        grain: SectionGrain,
    ) {
        let kind = self.kind_of(s.status.as_ref(), &s.blocks);
        if let Some(id) = &s.id {
            if !is_valid_anchor(id) {
                self.warn(
                    "invalid-anchor",
                    format!(
                        "anchor `{{#{id}}}` is not an id `[A-Za-z][A-Za-z0-9_-]*`; unit skipped"
                    ),
                    s.line,
                );
            } else {
                self.register(id, s.line);
                let span = section_lines(s, level).join("\n");
                self.long_section(
                    s.line,
                    s.end_line,
                    &s.title,
                    max_section_lines,
                    grain,
                    s.sections.is_empty(),
                );
                self.units
                    .push(self.unit(id.clone(), &s.title, &span, s.line, kind));
            }
        }
        self.block_facts(&s.blocks);
        for sub in &s.sections {
            self.section(sub, level + 1, max_section_lines, grain);
        }
    }

    /// Fact units out of a block list: from `<p>` and `<item>` carriers (the
    /// Markdown unit grain). Three carriers are BELOW that grain and mint
    /// nothing — each mirroring `mdspec` over the projection: a `<td>`/`
    /// `<quote>` fact (the projection's line opens with `|` / `> `, not the
    /// anchor), and a fact the NEXT block's fence is bound to (the
    /// projection spells it `@fact/code:`, which the markdown scanner does
    /// not read as an anchor — the typed binding is progress grain, not
    /// specmap grain).
    fn block_facts(&mut self, blocks: &[XBlock]) {
        for (i, b) in blocks.iter().enumerate() {
            let typed = typed_id_after(blocks, i);
            match b {
                XBlock::Para { unit, .. } => {
                    self.para_fact(unit, typed);
                }
                XBlock::List { items, ordered } => {
                    for (j, item) in items.iter().enumerate() {
                        self.item_fact(item, j, *ordered, typed);
                    }
                }
                _ => {}
            }
        }
    }

    /// A paragraph fact's span is the paragraph's own canonical line(s).
    fn para_fact(&mut self, u: &XUnit, typed: Option<&str>) {
        let Some(f) = u.fact.as_ref().filter(|f| f.is_meaningful()) else {
            return;
        };
        let Some(id) = &f.id else { return };
        if f.id.as_deref() == typed {
            // The next block's fence binds this fact — the projection
            // spells the anchor `@fact/code:`, below the unit grain.
            return;
        }
        if !is_valid_fact_id(id) {
            // An explicitly authored id the shared grammar refuses — louder
            // than the Markdown side's silent prose (there `##9bad` may be
            // prose; here `id="9bad"` is unambiguous intent).
            self.warn(
                "invalid-anchor",
                format!("fact id `{id}` is not a valid id `[A-Za-z][A-Za-z0-9_-]*`; unit skipped"),
                f.line,
            );
            return;
        }
        self.register(id, f.line);
        let heading = unit_body(u);
        let span = para_line(u, typed);
        self.units
            .push(self.unit(id.clone(), &heading, &span, f.line, None));
    }

    /// A list-item fact's span is the item's own canonical line(s), marker
    /// included — the item plus its continuation lines, as in Markdown.
    fn item_fact(&mut self, item: &XUnit, j: usize, ordered: bool, typed: Option<&str>) {
        let Some(f) = item.fact.as_ref().filter(|f| f.is_meaningful()) else {
            return;
        };
        let Some(id) = &f.id else { return };
        if f.id.as_deref() == typed {
            // The `@fact/code:` spelling — below the unit grain (see
            // [`Minter::block_facts`]).
            return;
        }
        if !is_valid_fact_id(id) {
            self.warn(
                "invalid-anchor",
                format!("fact id `{id}` is not a valid id `[A-Za-z][A-Za-z0-9_-]*`; unit skipped"),
                f.line,
            );
            return;
        }
        self.register(id, f.line);
        let heading = unit_body(item);
        let span = item_line(item, j, ordered, typed);
        self.units
            .push(self.unit(id.clone(), &heading, &span, f.line, None));
    }

    /// The kind line: the section's (or document's) first body element — a
    /// leading `<p>` whose text opens with the backticked declaration. A
    /// `<status>` before it, or any other first block, is what the Markdown
    /// projection would put on the first line instead, so no kind — the
    /// parity is with what `mdspec` reads back, not a new grammar.
    fn kind_of(&mut self, status: Option<&XStatus>, blocks: &[XBlock]) -> Option<KindLine> {
        if status.is_some() {
            return None;
        }
        let XBlock::Para { unit, line } = blocks.first()? else {
            return None;
        };
        let first = unit.text.lines().next()?;
        match parse_kind_line(first) {
            Ok(Some(kl)) => Some(kl),
            Ok(None) => None,
            Err(msg) => {
                self.warn("malformed-kind-line", msg, *line);
                None
            }
        }
    }

    /// The `long-section` quality warning over NATIVE lines (inclusive
    /// `<section>`…`</section>`); the same inclusive threshold and leaf-grain
    /// law as the Markdown side. `is_leaf` — the section governs no nested
    /// section.
    fn long_section(
        &mut self,
        line: u32,
        end_line: u32,
        heading: &str,
        threshold: usize,
        grain: SectionGrain,
        is_leaf: bool,
    ) {
        if threshold == 0 || (grain != SectionGrain::All && !is_leaf) {
            return;
        }
        let spanned = (end_line.saturating_sub(line) + 1) as usize;
        if spanned >= threshold {
            self.warn(
                "long-section",
                format!(
                    "section `{heading}` spans {spanned} lines (threshold {threshold}) — \
                     long sections read poorly and churn often; split into smaller leaves"
                ),
                line,
            );
        }
    }

    fn register(&mut self, id: &str, line: u32) {
        if self.seen_anchors.iter().any(|a| a == id) {
            self.warnings
                .push(duplicate_anchor_warning(id, self.file, line));
        } else {
            self.seen_anchors.push(id.to_string());
        }
    }

    fn unit(
        &self,
        anchor: String,
        heading: &str,
        span: &str,
        line: u32,
        kind: Option<KindLine>,
    ) -> SpecUnit {
        let (k, r, st, d) = match kind {
            Some(k) => (
                Some(Box::new(k.kind)),
                Some(Box::new(k.revision)),
                k.status.map(Box::new),
                k.disputes.map(Box::new),
            ),
            None => (None, None, None, None),
        };
        SpecUnit {
            uri: format!("spec://{}/{}#{anchor}", self.namespace, self.doc_path),
            docPath: self.doc_path.clone(),
            file: self.file.to_string(),
            anchor,
            heading: heading.to_string(),
            contentHash: crate::content_hash(span),
            line,
            kind: k,
            revision: r,
            status: st,
            disputes: d,
        }
    }

    fn warn(&mut self, code: &str, message: String, line: u32) {
        self.warnings.push(Warning {
            code: code.to_string(),
            message,
            file: self.file.to_string(),
            line,
        });
    }
}

#[cfg(test)]
mod tests;
