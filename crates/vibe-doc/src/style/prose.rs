//! Which text on a page is prose, and which block it belongs to
//! (PROP-057 `##STYLE-CONTAINERS-AND-CORRIDORS`).
//!
//! The style law divides a page in two. **Containers** — `rule` blocks,
//! tables, `derived` blocks, fences, examples — are where density is
//! welcome; the exact value, the field name and the edge condition live
//! there and are supposed to be dense. **Corridors** — the narrative
//! paragraphs, the steps of a procedure, the call-outs — are where
//! density is forbidden, and they are what the length, term and sign
//! rules judge.
//!
//! One container is scanned anyway: a table CELL. A tic is a tic wherever
//! it is written, so the banned words reach the cells; the length and
//! term rules do not, because a cell that names three terms in four words
//! is a reference table doing its job.
//!
//! Every node carries the block number the reader sees in the margin, so
//! a finding names `p12` and a person opens `p12`
//! (`##READER-NUMBERED-BLOCKS`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#STYLE-CONTAINERS-AND-CORRIDORS");

use vibe_specdoc::doc::{Block, BlockNode, NoteKind, Section, SpecDoc};

use crate::numbering::{BlockPath, Numbering, number_blocks};
use crate::style::inline::{self, Inline};

/// What a piece of prose IS, which is what decides the rules it answers
/// to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The page's H1.
    Title,
    /// A narrative paragraph — a corridor.
    Paragraph,
    /// One item of a list: a step of a procedure or an alternative.
    Step,
    /// A `note` call-out, with its own kind.
    Note(NoteKind),
    /// A blockquote.
    Quote,
    /// One cell of a table — a container, scanned for tics alone.
    Cell,
    /// A `prompt` body, in the user's voice.
    Prompt,
    /// A prompt's `needs`.
    Needs,
    /// A prompt's `outcome`.
    Outcome,
    /// A figure's caption.
    Caption,
}

impl Kind {
    /// The word a finding prints.
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Title => "title",
            Kind::Paragraph => "p",
            Kind::Step => "step",
            Kind::Note(NoteKind::Warning) => "warning",
            Kind::Note(_) => "note",
            Kind::Quote => "quote",
            Kind::Cell => "cell",
            Kind::Prompt => "prompt",
            Kind::Needs => "needs",
            Kind::Outcome => "outcome",
            Kind::Caption => "caption",
        }
    }

    /// A corridor is where density is forbidden — the only text the
    /// length, term and sign rules judge.
    pub fn is_corridor(self) -> bool {
        matches!(
            self,
            Kind::Paragraph | Kind::Step | Kind::Note(_) | Kind::Quote | Kind::Caption
        )
    }

    /// The prompt and its two companions: the user's own voice, which the
    /// sentence-length rules deliberately leave alone (X-028).
    pub fn is_prompt(self) -> bool {
        matches!(self, Kind::Prompt | Kind::Needs | Kind::Outcome)
    }
}

/// One piece of prose on a page.
#[derive(Debug, Clone)]
pub struct Node {
    pub kind: Kind,
    /// The block number the margin shows (`p07`), or the word that names
    /// a node with no number of its own.
    pub block: String,
    /// The title of the section this node sits in; empty in the preamble.
    pub section: String,
    /// The unit's inline content, read once.
    pub inline: Inline,
    /// `true` when the owning section is a procedure — a section whose
    /// steps are numbered, where STE's tighter limit applies.
    pub procedural: bool,
    /// `true` for the page's first paragraph, the one that carries no
    /// glossary term at all and stands alone in `llms.txt`.
    pub intro: bool,
    /// `true` when a `rule` block stands immediately after this one.
    ///
    /// The one place a deferral phrase is not a deferral: «the rule, in
    /// the specification's own words:» followed by the quoted fact is a
    /// citation ADDING the exact wording, which is what STYLE.md §2 asks
    /// for. The same words with nothing behind them send the reader away.
    pub cites_next: bool,
}

/// Everything a page says, in document order, with its block numbers.
pub fn nodes(doc: &SpecDoc) -> Vec<Node> {
    let numbering = number_blocks(doc);
    let mut out: Vec<Node> = Vec::new();
    if let Some(title) = &doc.title {
        out.push(Node {
            kind: Kind::Title,
            block: "title".to_owned(),
            section: String::new(),
            inline: inline::read(&title.text),
            procedural: false,
            intro: false,
            cites_next: false,
        });
    }
    let preamble_procedural = is_procedure(&doc.preamble);
    blocks(
        &doc.preamble,
        &[],
        "",
        preamble_procedural,
        &numbering,
        &mut out,
    );
    for (i, section) in doc.sections.iter().enumerate() {
        walk(section, &[i as u16], &numbering, &mut out);
    }
    mark_intro(&mut out);
    out
}

/// The page's first paragraph is the first narrative paragraph of the
/// preamble — the one `##STYLE-PAGE-SKELETON` says carries no glossary
/// term, and the one `llms.txt` prints as the page's line.
fn mark_intro(nodes: &mut [Node]) {
    if let Some(first) = nodes
        .iter_mut()
        .find(|n| n.kind == Kind::Paragraph && n.section.is_empty())
    {
        first.intro = true;
    }
}

fn walk(section: &Section, path: &[u16], numbering: &Numbering, out: &mut Vec<Node>) {
    let procedural = is_procedure(&section.blocks);
    blocks(
        &section.blocks,
        path,
        &section.title,
        procedural,
        numbering,
        out,
    );
    for (i, sub) in section.sections.iter().enumerate() {
        let mut child = path.to_vec();
        child.push(i as u16);
        walk(sub, &child, numbering, out);
    }
}

/// A section is a PROCEDURE when its own blocks are numbered steps: an
/// ordered list, or paragraphs that open `1.`, `2.`, `3.`.
///
/// Read from the shape rather than from the heading, because the heading
/// is free text: the corpus says «By hand», «Do it yourself» and «The
/// steps» for the same thing, and a rule keyed on one of those spellings
/// would stop working the day an author wrote another.
///
/// ```
/// use vibe_doc::style::prose::nodes;
///
/// let doc = vibe_specdoc::from_xml_with(
///     "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
///      <spec xmlns=\"https://vibevm.org/spec/1\">\n\
///        <title id=\"root\">T</title>\n\
///        <steps title=\"By hand\"><p>1. Run it.</p><p>2. Check it.</p></steps>\n\
///      </spec>\n",
///     vibe_specdoc::Vocabulary::Doc,
/// )
/// .unwrap();
///
/// assert!(nodes(&doc).iter().filter(|n| n.section == "By hand").all(|n| n.procedural));
/// ```
fn is_procedure(blocks: &[BlockNode]) -> bool {
    blocks.iter().any(|node| match &node.block {
        Block::List { ordered, .. } => *ordered,
        Block::Paragraph(unit) => opens_a_step(&unit.text),
        _ => false,
    })
}

/// `1. `, `2) ` — a step marker at the head of a paragraph.
pub fn opens_a_step(text: &str) -> bool {
    let trimmed = text.trim_start();
    let digits: String = trimmed.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() || digits.len() > 2 {
        return false;
    }
    let rest = &trimmed[digits.len()..];
    matches!(rest.as_bytes().first(), Some(&b'.') | Some(&b')'))
        && rest[1..].starts_with(char::is_whitespace)
}

/// A page's prose as one string, for the readability score.
pub fn joined(nodes: &[Node]) -> String {
    let mut out = String::new();
    for node in corridors(nodes) {
        out.push_str(&node.inline.text);
        if !out.ends_with(' ') {
            out.push(' ');
        }
    }
    out
}

fn blocks(
    list: &[BlockNode],
    path: &[u16],
    section: &str,
    procedural: bool,
    numbering: &Numbering,
    out: &mut Vec<Node>,
) {
    for (i, node) in list.iter().enumerate() {
        let label = numbering
            .label(&BlockPath::new(path.to_vec(), i as u16))
            .unwrap_or_else(|| "p--".to_owned());
        let cites_next = matches!(
            list.get(i + 1).map(|next| &next.block),
            Some(Block::Rule { .. })
        );
        let mut push = |kind: Kind, text: &str| {
            out.push(Node {
                kind,
                block: label.clone(),
                section: section.to_owned(),
                inline: inline::read(text),
                procedural,
                intro: false,
                cites_next,
            });
        };
        match &node.block {
            Block::Paragraph(unit) => push(Kind::Paragraph, &unit.text),
            Block::Quote(unit) => push(Kind::Quote, &unit.text),
            Block::List { items, .. } => {
                for item in items {
                    push(Kind::Step, &item.text);
                }
            }
            Block::Table { rows } => {
                for cell in rows.iter().flatten() {
                    push(Kind::Cell, &cell.text);
                }
            }
            Block::Note { kind, body } => push(Kind::Note(*kind), &body.text),
            Block::Figure { caption, .. } => push(Kind::Caption, &caption.text),
            Block::Prompt {
                text,
                needs,
                outcome,
                ..
            } => {
                push(Kind::Prompt, text);
                if let Some(needs) = needs {
                    push(Kind::Needs, needs);
                }
                if let Some(outcome) = outcome {
                    push(Kind::Outcome, outcome);
                }
            }
            // Containers with no prose of their own: an example is a
            // command and its output, a rule is a quoted fact fetched at
            // build time, a fence is verbatim, a `derived` block has no
            // text until the generator runs.
            Block::Example { .. }
            | Block::ExampleRef { .. }
            | Block::Rule { .. }
            | Block::Derived { .. }
            | Block::Fence { .. } => {}
        }
    }
}

/// Every unit of a page that a length or term rule may judge — the same
/// walk, filtered to the corridors.
pub fn corridors(nodes: &[Node]) -> impl Iterator<Item = &Node> {
    nodes.iter().filter(|n| n.kind.is_corridor())
}

#[cfg(test)]
mod tests;
