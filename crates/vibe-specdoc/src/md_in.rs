//! `from_markdown` — the Markdown frontend (PROP-045 S1).
//!
//! An ADAPTER, not a parser: `progress_core::parse::parse_document` does
//! all the lexing (blocks, facts, markers, fence bindings, the anchor
//! laws), and this module rebuilds the pivot's section/block structure
//! over its output. Every grammar rule it touches — the anchor token, the
//! table row splitter, the list marker, the task box, the blockquote
//! prefix, the fence run — is re-used through progress-core's re-exports,
//! so the two crates cannot drift into dialects.
//!
//! Structural decisions the rebuild makes (all recorded in the slice-1
//! report): lists are flat runs of items (the markup contract's own fact
//! model); a GFM task box stays at the head of its item's text; table
//! alignment and the delimiter row are MD spelling; quote depth collapses
//! one level; comment-only blocks, thematic breaks and YAML frontmatter
//! are layout and do not enter the IR; a `<status>…</status>` fragment
//! wrapper keeps its literal spelling inside the unit text while its
//! payload also becomes the unit's status.

use crate::doc::{Block, Fact, Section, SpecDoc, StatusEl, Title, Unit};
use crate::{Error, Result};
use progress_core::doc::{BlockKind, FactKind, ParsedDoc};
use progress_core::model::{Granularity, Marker, MarkerForm};
use progress_core::parse::{
    blockquote_prefix_len, closes_fence, fence_run, is_delimiter_row, list_marker_len, row_cells,
    take_fact_id, task_box_len,
};

/// Parse one Markdown document into the pivot IR.
///
/// Fails loudly when the source markup itself has errors (progress-core's
/// issues, verbatim): the pivot refuses to represent a document whose
/// spelling no consumer can trust.
pub fn from_markdown(text: &str) -> Result<SpecDoc> {
    let parsed = progress_core::parse::parse_document("<specdoc>", text);
    if parsed.error_count() > 0 {
        let errs: Vec<String> = parsed
            .issues
            .iter()
            .filter(|i| i.severity == progress_core::doc::Severity::Error)
            .map(|i| format!("line {}: {}", i.line, i.message))
            .collect();
        return Err(Error::at(
            0,
            format!(
                "the source markup has {} error(s): {}",
                errs.len(),
                errs.join("; ")
            ),
        ));
    }
    Adapter::new(text, &parsed).run()
}

struct Adapter<'a> {
    lines: Vec<&'a str>,
    doc: &'a ParsedDoc,
}

impl<'a> Adapter<'a> {
    fn new(text: &'a str, doc: &'a ParsedDoc) -> Self {
        Adapter {
            lines: text.lines().collect(),
            doc,
        }
    }

    fn run(&self) -> Result<SpecDoc> {
        let mut out = SpecDoc::default();
        // (heading level, section) in document order; nested at the end.
        let mut flat: Vec<(usize, Section)> = Vec::new();
        let mut unit_idx = 0usize;
        let mut first_heading = true;
        let mut open: Option<usize> = None; // index into `flat`

        for b in &self.doc.blocks {
            match b.kind {
                // Layout, not prose (the markup contract's own exemption):
                // comments, thematic breaks, YAML frontmatter.
                BlockKind::Comment => continue,
                BlockKind::Heading => {
                    let u = &self.doc.units[unit_idx];
                    unit_idx += 1;
                    if first_heading && u.level == 1 {
                        out.title = Some(Title {
                            text: u.heading.clone(),
                            id: u.anchor.clone(),
                        });
                        open = None;
                    } else {
                        flat.push((
                            u.level,
                            Section {
                                id: u.anchor.clone(),
                                title: u.heading.clone(),
                                status: None,
                                blocks: Vec::new(),
                                sections: Vec::new(),
                            },
                        ));
                        open = Some(flat.len() - 1);
                    }
                    first_heading = false;
                }
                BlockKind::MarkerOnly => {
                    let Some(m) = self.doc.markers.iter().find(|m| m.line == b.line_start) else {
                        continue;
                    };
                    match m.granularity {
                        Granularity::Document => out.status = Some(StatusEl::from(m)),
                        Granularity::Section => {
                            if let Some(i) = open {
                                flat[i].1.status = Some(StatusEl::from(m));
                            }
                        }
                        _ => {}
                    }
                }
                BlockKind::Code => {
                    let fence = self.fence_block(b)?;
                    match open {
                        Some(i) => flat[i].1.blocks.push(fence),
                        None => out.preamble.push(fence),
                    }
                }
                BlockKind::Text => {
                    for blk in self.text_blocks(b)? {
                        match open {
                            Some(i) => flat[i].1.blocks.push(blk),
                            None => out.preamble.push(blk),
                        }
                    }
                }
            }
        }
        out.sections = nest(flat);
        Ok(out)
    }

    /// One Code block → [`Block::Fence`]: the info string, the verbatim
    /// content between the fence lines, and the `@fact/code:<ID>` binding
    /// (`Fact::covers` points at exactly this block's line range).
    fn fence_block(&self, b: &progress_core::doc::Block) -> Result<Block> {
        let lines: Vec<&str> = b.source_text.lines().collect();
        let first = lines.first().copied().unwrap_or("");
        let trimmed = first.trim_start();
        let (ch, open_len) = fence_run(trimmed).expect("a Code block opens with a fence run");
        let lang = trimmed[open_len..].trim();
        let text = if lines.len() >= 2
            && closes_fence(lines[lines.len() - 1].trim_start(), ch, open_len)
        {
            lines[1..lines.len() - 1].join("\n")
        } else {
            // An unclosed fence runs to end of input: everything after the
            // opener is content.
            lines[1..].join("\n")
        };
        let fact = self
            .doc
            .blocks
            .iter()
            .flat_map(|bb| &bb.facts)
            .find(|f| f.covers == Some((b.line_start, b.line_end)))
            .and_then(|f| f.id.clone());
        Ok(Block::Fence {
            lang: (!lang.is_empty()).then(|| lang.to_string()),
            fact,
            text,
        })
    }

    /// One Text block → the pivot's paragraph/list/table/quote blocks.
    ///
    /// The walk is driven by the parse's own fact segmentation (the spans
    /// are exact); table LINE runs are reconstructed beside it so header
    /// rows and empty cells — which carry no facts — survive too.
    fn text_blocks(&self, b: &progress_core::doc::Block) -> Result<Vec<Block>> {
        let mut blocks: Vec<Block> = Vec::new();
        let text = b.source_text.as_str();
        let n_lines = text.lines().count();
        let line_span = |li: usize| -> (usize, usize) {
            let start = text.lines().take(li).map(|l| l.len() + 1).sum::<usize>();
            let l = text.lines().nth(li).unwrap_or("");
            (start, start + l.len())
        };
        let is_table = |li: usize| -> bool {
            text.lines()
                .nth(li)
                .is_some_and(|l| l.trim_start().starts_with('|'))
        };

        // Maximal runs of table lines, as (first_li, last_li) inclusive.
        let mut table_runs: Vec<(usize, usize)> = Vec::new();
        {
            let mut li = 0usize;
            while li < n_lines {
                if is_table(li) {
                    let start = li;
                    while li + 1 < n_lines && is_table(li + 1) {
                        li += 1;
                    }
                    table_runs.push((start, li));
                }
                li += 1;
            }
        }

        let mut facts: Vec<usize> = (0..b.facts.len()).collect(); // indices of unplaced facts

        let mut pending: usize = 0; // index into table_runs, in order
        while !facts.is_empty() || pending < table_runs.len() {
            let next_fact_li = facts.first().map(|&i| b.facts[i].line - b.line_start);
            let next_run_li = table_runs.get(pending).map(|&(s, _)| s);
            let table_first = match (next_fact_li, next_run_li) {
                (None, Some(_)) => true,          // only runs remain
                (Some(_), None) => false,         // only facts remain
                (Some(fl), Some(rl)) => rl <= fl, // the run starts at or before the fact
                (None, None) => break,            // unreachable via the while condition
            };
            if table_first {
                let (rs, re) = table_runs[pending];
                pending += 1;
                let (table, placed) = self.table_block(b, text, &line_span, rs, re, &facts);
                blocks.push(table);
                facts.retain(|i| !placed.contains(i));
                continue;
            }
            let &fi = facts.first().expect("a fact is pending");
            let f = &b.facts[fi];
            match f.kind {
                FactKind::Item => {
                    // One run of consecutive Item facts = one list.
                    let mut run = Vec::new();
                    let mut idx = 0usize;
                    while idx < facts.len() && b.facts[facts[idx]].kind == FactKind::Item {
                        run.push(facts[idx]);
                        idx += 1;
                    }
                    let ordered = run.first().map(|&i| self.item_is_ordered(b.facts[i].line));
                    for &i in &run {
                        facts.retain(|x| *x != i);
                    }
                    let items = run
                        .iter()
                        .map(|&i| self.item_unit(b, &b.facts[i]))
                        .collect::<Vec<_>>();
                    blocks.push(Block::List {
                        ordered: ordered.unwrap_or(false),
                        items,
                    });
                }
                FactKind::Cell => {
                    // Reached only when a Cell fact sits outside every
                    // recorded table run (impossible in practice — the run
                    // walk above fires first because a run starts at or
                    // before its first cell's line). Place it as a bare
                    // paragraph so nothing is silently dropped.
                    blocks.push(Block::Paragraph(self.unit_from_fact(b, f)));
                    facts.retain(|x| *x != fi);
                }
                FactKind::Para | FactKind::Lead => {
                    let unit = self.unit_from_fact(b, f);
                    let block = if self.is_quote_unit(b, f) {
                        Block::Quote(unit)
                    } else {
                        Block::Paragraph(unit)
                    };
                    blocks.push(block);
                    facts.retain(|x| *x != fi);
                }
            }
        }
        Ok(blocks)
    }

    /// Build one [`Block::Table`] from a run of `|`-lines: every row except
    /// the delimiter rows; each cell a unit, empty cells included; Cell
    /// facts attached to the non-empty cells of their row in order.
    #[allow(clippy::too_many_arguments)]
    fn table_block(
        &self,
        b: &progress_core::doc::Block,
        text: &str,
        line_span: &dyn Fn(usize) -> (usize, usize),
        rs: usize,
        re: usize,
        facts: &[usize],
    ) -> (Block, Vec<usize>) {
        let mut rows: Vec<Vec<Unit>> = Vec::new();
        let mut placed: Vec<usize> = Vec::new();
        for li in rs..=re {
            let (ls, le) = line_span(li);
            let cells = row_cells(text, ls, le);
            if cells.is_empty() || is_delimiter_row(&cells, text) {
                continue;
            }
            // The Cell facts of this row, in order.
            let row_line = b.line_start + li;
            let mut row_facts: Vec<usize> = facts
                .iter()
                .copied()
                .filter(|i| b.facts[*i].kind == FactKind::Cell && b.facts[*i].line == row_line)
                .collect();
            let mut row: Vec<Unit> = Vec::new();
            for (cs, ce) in cells {
                if text[cs..ce].trim().is_empty() {
                    row.push(Unit {
                        fact: None,
                        text: String::new(),
                    });
                } else {
                    match row_facts.first() {
                        Some(&i) => {
                            row.push(self.unit_from_fact(b, &b.facts[i]));
                            placed.push(i);
                            row_facts.remove(0);
                        }
                        None => row.push(Unit {
                            fact: None,
                            text: text[cs..ce].trim().to_string(),
                        }),
                    }
                }
            }
            rows.push(row);
        }
        (Block::Table { rows }, placed)
    }

    /// The unit of one fact: the verbatim span with the anchor prefix and
    /// the status spelling stripped (they live in [`Fact`]), trimmed of the
    /// whitespace that surrounded the markup.
    fn unit_from_fact(&self, b: &progress_core::doc::Block, f: &progress_core::doc::Fact) -> Unit {
        let body = &b.source_text[f.span.0..f.span.1];
        let effective = if f.kind == FactKind::Para || f.kind == FactKind::Lead {
            if body.lines().all(|l| l.trim_start().starts_with('>')) {
                self.strip_quote(body)
            } else {
                body.to_string()
            }
        } else {
            body.to_string()
        };
        let marker = self.marker_for(b, f);
        let (_, after_anchor) = take_fact_id(&effective, 0, effective.len());
        let bare = strip_marker(&effective[after_anchor..], marker);
        let fact = Fact {
            id: f.id.clone(),
            status: marker.map(StatusEl::from),
        };
        Unit {
            fact: fact.is_meaningful().then_some(fact),
            text: bare.trim().to_string(),
        }
    }

    /// A list item's unit: the fact's unit with its GFM task box (if any)
    /// restored at the head of the text — the box is the item's own
    /// content in the dialect, and progress-core consumes it as structure.
    fn item_unit(&self, b: &progress_core::doc::Block, f: &progress_core::doc::Fact) -> Unit {
        let mut unit = self.unit_from_fact(b, f);
        if let Some(line) = self.lines.get(f.line - 1) {
            let indent = line.len() - line.trim_start().len();
            if let Some(mlen) = list_marker_len(line) {
                let after = &line[indent + mlen..];
                let blen = task_box_len(after);
                if blen > 0 && !unit.text.is_empty() {
                    unit.text = format!("{}{}", &after[..blen], unit.text);
                }
            }
        }
        unit
    }

    /// `true` when the opener line of the item at 1-based `line` spells an
    /// ordered marker (`N.` / `N)`).
    fn item_is_ordered(&self, line: usize) -> bool {
        self.lines
            .get(line - 1)
            .map(|l| {
                l.trim_start()
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_digit())
            })
            .unwrap_or(false)
    }

    /// The first marker progress-core attached to this fact: its
    /// granularity matches the fact kind (or is a fragment wrapper inside
    /// the unit) and its line falls inside the fact's own line range.
    fn marker_for(
        &self,
        b: &progress_core::doc::Block,
        f: &progress_core::doc::Fact,
    ) -> Option<&'a Marker> {
        let end_line = b.line_start + b.source_text[..f.span.1].matches('\n').count();
        let gran_ok = |g: Granularity| match f.kind {
            FactKind::Para | FactKind::Lead => {
                g == Granularity::Paragraph || g == Granularity::Fragment
            }
            FactKind::Item => g == Granularity::Item || g == Granularity::Fragment,
            FactKind::Cell => g == Granularity::Cell || g == Granularity::Fragment,
        };
        self.doc
            .markers
            .iter()
            .filter(|m| m.line >= f.line && m.line <= end_line && gran_ok(m.granularity))
            .min_by_key(|m| m.line)
    }

    fn is_quote_unit(&self, b: &progress_core::doc::Block, f: &progress_core::doc::Fact) -> bool {
        b.source_text[f.span.0..f.span.1]
            .lines()
            .all(|l| l.trim_start().starts_with('>'))
    }

    /// Strip the blockquote prefix of every line (`>` runs with spacing,
    /// the same reader the anchor grammar uses).
    fn strip_quote(&self, s: &str) -> String {
        s.lines()
            .map(|l| {
                let indent = l.len() - l.trim_start().len();
                let rest = &l[indent..];
                &rest[blockquote_prefix_len(rest)..]
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Nest the flat heading list by level: a heading closes every open
/// section of the same or higher level; level skips (an H3 under an H1)
/// nest as deep as the open structure allows.
fn nest(flat: Vec<(usize, Section)>) -> Vec<Section> {
    fn attach(stack: &mut [(usize, Section)], roots: &mut Vec<Section>, s: Section) {
        match stack.last_mut() {
            Some((_, parent)) => parent.sections.push(s),
            None => roots.push(s),
        }
    }
    let mut roots = Vec::new();
    let mut stack: Vec<(usize, Section)> = Vec::new();
    for (level, section) in flat {
        while stack.last().is_some_and(|(l, _)| *l >= level) {
            let (_, done) = stack.pop().expect("checked by the while condition");
            attach(&mut stack, &mut roots, done);
        }
        stack.push((level, section));
    }
    while let Some((_, done)) = stack.pop() {
        attach(&mut stack, &mut roots, done);
    }
    roots
}

/// Remove the marker's SPELLING from a unit's text — at the end first
/// (the common trailing status), then at the start. A wrapper-form
/// (`<status …>…</status>`) is never stripped: the wrapped fragment is
/// part of the unit's text and stays verbatim.
fn strip_marker<'s>(s: &'s str, m: Option<&Marker>) -> &'s str {
    let Some(m) = m else {
        return s;
    };
    if m.form == MarkerForm::Wrapper {
        return s;
    }
    // End: a self-closing `<status …/>` as the last token…
    if m.form == MarkerForm::Point {
        let t = s.trim_end();
        if let Some(at) = t.rfind("<status")
            && let Some(el) = progress_core::element::lex_element(t, at)
            && el.self_closing
            && t[at + el.tag_len..].trim().is_empty()
        {
            return &t[..at];
        }
    }
    // …or one of the shorthand spellings as the last token, longest first.
    let sh = shorthand_spellings(m);
    for cand in &sh {
        let t = s.trim_end();
        if t.ends_with(cand.as_str()) {
            return &t[..t.len() - cand.len()];
        }
    }
    // Start (post-anchor): the same candidates as the first token.
    if m.form == MarkerForm::Point {
        let t = s.trim_start();
        if t.starts_with("<status")
            && let Some(el) = progress_core::element::lex_element(t, 0)
            && el.self_closing
        {
            return &t[el.tag_len..];
        }
    }
    for cand in &sh {
        let t = s.trim_start();
        if let Some(rest) = t.strip_prefix(cand.as_str())
            && rest.chars().next().is_none_or(|c| c.is_whitespace())
        {
            return rest;
        }
    }
    s
}

/// The possible shorthand spellings of a marker, longest first: the
/// qualified form, the bare stage/state pair, the bare stage.
fn shorthand_spellings(m: &Marker) -> Vec<String> {
    vec![
        format!("@status:{}/{}", m.stage, m.state),
        format!("@{}/{}", m.stage, m.state),
        format!("@{}", m.stage),
    ]
}
