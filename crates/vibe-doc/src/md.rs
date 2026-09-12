//! The `.md` projection — the page as a file beside it
//! (PROP-057 `##SITE-TRAILING-SLASH`, `##READER-NUMBERED-BLOCKS`).
//!
//! Every page is published three ways: the island a browser renders, this
//! Markdown, and the XML beside them. All three carry the SAME block
//! numbers, because that is what lets a human quoting `p12` from the web
//! and an agent quoting `p12` from a text file be quoting one place.
//! Here the number opens the block as `[p12]`.
//!
//! ## Why this is not the pivot's `to_markdown`
//!
//! The pivot's Markdown backend is a projection of the SOURCE: it prints
//! a `rule` as its address, a `derived` as a provenance line, and an
//! `example ref` as a note saying the pipeline will copy the source's
//! fences. That is correct for what it is — the form the host's scanners
//! read a page through — and wrong for a reader, who wants the rule's
//! text, the generated output and the borrowed example.
//!
//! And the numbering cannot live there at all: `Numbering` never enters
//! the pivot (`##PIPE-NUMBERING`), because a number stored beside the IR
//! would enter the equality every law of the pivot is stated over.
//!
//! So this is a second backend over the same IR, and the spellings it
//! shares with the pivot — `@fact:<ID>`, `@status:<stage>/<state>`, the
//! fence forms — are deliberately the same: one project, one Markdown
//! dialect. What differs is the block number and the resolved content.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#READER-NUMBERED-BLOCKS");

use vibe_specdoc::doc::{Block, BlockNode, Section, SpecDoc, StatusEl, Unit};

use crate::content::{Content, ExampleBody};
use crate::numbering::{BlockPath, Numbering};

/// Render a page as Markdown, without block numbers.
pub fn to_markdown(doc: &SpecDoc, content: &Content) -> String {
    to_markdown_numbered(doc, content, &Numbering::none())
}

/// Render a page as Markdown carrying its block numbers.
///
/// ```
/// use vibe_doc::{content::Content, md, numbering::number_blocks};
///
/// let doc = vibe_specdoc::from_xml_with(
///     "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
///      <spec xmlns=\"https://vibevm.org/spec/1\">\n\
///        <title id=\"root\">Hello</title>\n<p>one</p>\n\
///      </spec>\n",
///     vibe_specdoc::Vocabulary::Doc,
/// )
/// .unwrap();
///
/// let text = md::to_markdown_numbered(&doc, &Content::new(), &number_blocks(&doc));
/// assert!(text.starts_with("# Hello {#root}\n"));
/// assert!(text.contains("[p01] one"));
/// ```
pub fn to_markdown_numbered(doc: &SpecDoc, content: &Content, numbering: &Numbering) -> String {
    let mut out = String::new();
    if let Some(title) = &doc.title {
        out.push_str("# ");
        out.push_str(&title.text);
        if let Some(id) = &title.id {
            out.push_str(&format!(" {{#{id}}}"));
        }
        out.push_str("\n\n");
    }
    if let Some(status) = &doc.status {
        out.push_str(&status_md(status));
        out.push_str("\n\n");
    }
    blocks_md(&mut out, &doc.preamble, &[], 1, content, numbering);
    for (i, section) in doc.sections.iter().enumerate() {
        section_md(&mut out, section, &[i as u16], 2, content, numbering);
    }
    out
}

fn section_md(
    out: &mut String,
    s: &Section,
    path: &[u16],
    level: usize,
    content: &Content,
    numbering: &Numbering,
) {
    out.push_str(&"#".repeat(level.min(6)));
    out.push(' ');
    out.push_str(&s.title);
    // A guarded section names its condition in its own heading, where a
    // reader and a scanner both see it (PROP-045 `##ROW-DOCVOCAB-WHEN`).
    if let Some(cond) = &s.when {
        out.push_str(&format!(" ({cond})"));
    }
    if let Some(id) = &s.id {
        out.push_str(&format!(" {{#{id}}}"));
    }
    out.push_str("\n\n");
    if let Some(status) = &s.status {
        out.push_str(&status_md(status));
        out.push_str("\n\n");
    }
    blocks_md(out, &s.blocks, path, level, content, numbering);
    for (i, sub) in s.sections.iter().enumerate() {
        let mut child = path.to_vec();
        child.push(i as u16);
        section_md(out, sub, &child, level + 1, content, numbering);
    }
}

fn blocks_md(
    out: &mut String,
    nodes: &[BlockNode],
    path: &[u16],
    level: usize,
    content: &Content,
    numbering: &Numbering,
) {
    for (i, node) in nodes.iter().enumerate() {
        if let Some(cond) = &node.when {
            // A guarded slot takes a sub-heading naming the platform, one
            // level under its section and never past H6.
            out.push_str(&format!("{} {cond}\n\n", "#".repeat((level + 1).min(6))));
        }
        let label = numbering
            .get(&BlockPath::new(path.to_vec(), i as u16))
            .map(|n| format!("[{}] ", Numbering::spell(n)))
            .unwrap_or_default();
        block_md(out, &node.block, &label, content);
        out.push_str("\n\n");
    }
}

fn block_md(out: &mut String, b: &Block, label: &str, content: &Content) {
    match b {
        Block::Paragraph(u) => {
            out.push_str(label);
            out.push_str(&unit_md(u));
        }
        Block::Quote(u) => {
            let body = format!("{label}{}", unit_md(u));
            out.push_str(&quoted(&body));
        }
        Block::List { ordered, items } => {
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push('\n');
                }
                let marker = if *ordered {
                    format!("{}. ", i + 1)
                } else {
                    "- ".to_owned()
                };
                out.push_str(&marker);
                // The block's label opens the block, and a list's first
                // item is where the block begins.
                if i == 0 {
                    out.push_str(label);
                }
                out.push_str(&unit_md(item));
            }
        }
        Block::Table { rows } => {
            if !label.is_empty() {
                // A pipe table has no cell before its header, so the
                // label stands on the line above it — still the first
                // thing the block prints.
                out.push_str(label.trim_end());
                out.push('\n');
            }
            table_md(out, rows);
        }
        Block::Fence { lang, text, .. } => {
            if !label.is_empty() {
                out.push_str(label.trim_end());
                out.push('\n');
            }
            fence_md(out, lang.as_deref(), text);
        }
        Block::Rule { uri, .. } => {
            // The citation's own text, quoted, with the address under it
            // so a reader of the text file can follow it. No revision:
            // a citation is live (`##OBS-RULE-EDGE-UNPINNED`).
            let body = match content.rules.get(uri) {
                Some(found) => format!("{label}{}\n\n<{uri}>", found.text),
                None => format!("{label}<{uri}>"),
            };
            out.push_str(&quoted(&body));
        }
        Block::Example {
            lang,
            run,
            expect,
            stderr,
            ..
        } => example_md(out, label, lang.as_deref(), run, expect, stderr.as_deref()),
        Block::ExampleRef { id } => match content.examples.get(id) {
            Some(ExampleBody {
                lang,
                run,
                expect,
                stderr,
                ..
            }) => example_md(out, label, lang.as_deref(), run, expect, stderr.as_deref()),
            None => {
                out.push_str(label);
                out.push_str(&format!(
                    "Example `{id}` is copied from the source page at projection time."
                ));
            }
        },
        Block::Derived { kind, reference } => {
            if !label.is_empty() {
                out.push_str(label.trim_end());
                out.push('\n');
            }
            match content.derived_text(*kind, reference) {
                Some(text) => fence_md(out, Some("text"), text),
                None => fence_md(
                    out,
                    Some("text"),
                    &format!("generated from {kind}: {reference}"),
                ),
            }
        }
        Block::Note { kind, body } => {
            let text = format!("**{}**\n{label}{}", kind.label(), unit_md(body));
            out.push_str(&quoted(&text));
        }
        Block::Figure { src, alt, caption } => {
            out.push_str(&format!("{label}![{alt}]({src})\n\n{}", unit_md(caption)));
        }
        Block::Prompt {
            text,
            needs,
            outcome,
            asserts,
            ..
        } => {
            if !label.is_empty() {
                out.push_str(label.trim_end());
                out.push('\n');
            }
            fence_md(out, Some("prompt"), text);
            if let Some(needs) = needs {
                out.push_str(&format!("\n\n- needs: {needs}"));
            }
            if let Some(outcome) = outcome {
                out.push_str(&format!("\n\noutcome: {outcome}"));
            }
            if asserts.is_empty() {
                out.push_str("\n\n- assert: none");
            } else {
                out.push('\n');
                for a in asserts {
                    out.push_str(&format!("\n- assert: `{a}`"));
                }
            }
        }
    }
}

/// An example: the command, then the output it must produce, then its
/// stderr when the page spells one.
fn example_md(
    out: &mut String,
    label: &str,
    lang: Option<&str>,
    run: &str,
    expect: &str,
    stderr: Option<&str>,
) {
    if !label.is_empty() {
        out.push_str(label.trim_end());
        out.push('\n');
    }
    fence_md(out, Some(lang.unwrap_or("sh")), run);
    out.push_str("\n\n");
    fence_md(out, Some("output"), expect);
    if let Some(err) = stderr {
        out.push_str("\n\n");
        fence_md(out, Some("stderr"), err);
    }
}

fn table_md(out: &mut String, rows: &[Vec<Unit>]) {
    for (i, row) in rows.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        if i == 1 {
            let width = rows.first().map(Vec::len).unwrap_or(1);
            out.push_str("| ");
            for _ in 0..width.saturating_sub(1) {
                out.push_str("--- | ");
            }
            out.push_str("--- |\n");
        }
        out.push('|');
        for cell in row {
            out.push(' ');
            out.push_str(&unit_md(cell));
            out.push_str(" |");
        }
    }
}

/// A fence whose run is long enough to quote its own content — the same
/// run-matching law the Markdown scanner reads.
fn fence_md(out: &mut String, lang: Option<&str>, text: &str) {
    let run = "`".repeat(
        text.lines()
            .filter(|l| l.trim_start().starts_with("```"))
            .map(|l| l.trim_start().chars().take_while(|&c| c == '`').count())
            .max()
            .unwrap_or(2)
            .max(2)
            + 1,
    );
    out.push_str(&run);
    if let Some(lang) = lang {
        out.push_str(lang);
    }
    out.push('\n');
    if !text.is_empty() {
        out.push_str(text);
        out.push('\n');
    }
    out.push_str(&run);
}

/// `> ` before every line; a bare `>` for an empty one.
fn quoted(body: &str) -> String {
    let mut out = String::new();
    for (i, l) in body.lines().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        if l.is_empty() {
            out.push('>');
        } else {
            out.push_str("> ");
            out.push_str(l);
        }
    }
    out
}

/// One unit: its fact anchor, its text, its status — the spelling the
/// project's Markdown already uses everywhere else.
fn unit_md(u: &Unit) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(fact) = u.fact.as_ref().filter(|f| f.is_meaningful())
        && let Some(id) = &fact.id
    {
        parts.push(format!("@fact:{id}"));
    }
    if !u.text.is_empty() {
        parts.push(u.text.clone());
    }
    if let Some(status) = u.fact.as_ref().and_then(|f| f.status.as_ref()) {
        parts.push(format!("@status:{}/{}", status.stage, status.state));
    }
    parts.join(" ")
}

fn status_md(status: &StatusEl) -> String {
    let mut out = format!("@status:{}/{}", status.stage, status.state);
    if !status.audience.is_empty() {
        out.push_str(&format!(
            " @audience:{}",
            status
                .audience
                .iter()
                .map(|a| a.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ));
    }
    out
}

#[cfg(test)]
mod tests;
