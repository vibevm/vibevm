//! Reading a specification and finding the node an anchor names.
//!
//! The specification is read through the PIVOT — the same reader its
//! author writes against — so a citation quotes what the document says,
//! not what a second parser of ours believes it says. That matters here
//! more than anywhere: the text this module returns is printed on a page
//! as the rule itself.
//!
//! What an anchor resolves to depends on what it names. A FACT resolves
//! to its own body, the sentence that states the rule, which is exactly
//! what a page wants to quote. A SECTION resolves to its heading: a
//! section is a container, and inlining a container's whole subtree
//! would quote a chapter where the author asked for a rule.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-045#DOC-VOCAB-RULE-ADDRESS");

use std::path::Path;

use vibe_specdoc::doc::{Block, BlockNode, Section, SpecDoc, Unit};

/// Read a specification through the pivot, by the serialisation its
/// extension declares.
pub fn read_spec(path: &Path) -> Result<SpecDoc, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let parsed = match path.extension().and_then(|e| e.to_str()) {
        Some("xml") => vibe_specdoc::from_xml(&raw),
        _ => vibe_specdoc::from_markdown(&raw),
    };
    parsed.map_err(|e| format!("{}: {e}", path.display()))
}

/// The text of the node an anchor path names (`["PIPE-LIBRARY"]`, or a
/// dotted tree path `["chapter", "RULE"]`).
pub fn anchor_text(doc: &SpecDoc, path: &[String]) -> Option<String> {
    let (first, rest) = path.split_first()?;
    if rest.is_empty() {
        if let Some(text) = fact_text_in(&doc.preamble, first) {
            return Some(text);
        }
        for section in &doc.sections {
            if let Some(text) = fact_text_in_section(section, first) {
                return Some(text);
            }
        }
        if let Some(title) = &doc.title
            && title.id.as_deref() == Some(first.as_str())
        {
            return Some(title.text.clone());
        }
    }
    let section = doc
        .sections
        .iter()
        .find_map(|s| find_section(s, first.as_str()))?;
    descend(section, rest)
}

/// Follow the remaining path segments inside a section.
fn descend(section: &Section, rest: &[String]) -> Option<String> {
    let Some((next, tail)) = rest.split_first() else {
        return Some(section.title.clone());
    };
    if tail.is_empty()
        && let Some(text) = fact_text_in(&section.blocks, next)
    {
        return Some(text);
    }
    let child = section
        .sections
        .iter()
        .find_map(|s| find_section(s, next.as_str()))?;
    descend(child, tail)
}

fn find_section<'a>(section: &'a Section, id: &str) -> Option<&'a Section> {
    if section.id.as_deref() == Some(id) {
        return Some(section);
    }
    section.sections.iter().find_map(|s| find_section(s, id))
}

fn fact_text_in_section(section: &Section, id: &str) -> Option<String> {
    if let Some(text) = fact_text_in(&section.blocks, id) {
        return Some(text);
    }
    section
        .sections
        .iter()
        .find_map(|s| fact_text_in_section(s, id))
}

/// The body of the anchored unit inside these blocks, whichever block
/// kind carries it — a paragraph, a list item, a table cell, a quote, a
/// call-out body, a figure's caption. A fence bound to the fact
/// (`@fact/code:<ID>`) is part of the fact's body too and is appended, so
/// a typed fact quotes as the author wrote it.
fn fact_text_in(blocks: &[BlockNode], id: &str) -> Option<String> {
    for (i, node) in blocks.iter().enumerate() {
        let unit = match &node.block {
            Block::Paragraph(u) | Block::Quote(u) => unit_with_id(u, id),
            Block::Note { body, .. } => unit_with_id(body, id),
            Block::Figure { caption, .. } => unit_with_id(caption, id),
            Block::List { items, .. } => items.iter().find_map(|u| unit_with_id(u, id)),
            Block::Table { rows } => rows.iter().flatten().find_map(|u| unit_with_id(u, id)),
            Block::Fence { fact, text, .. } if fact.as_deref() == Some(id) => {
                return Some(text.clone());
            }
            _ => None,
        };
        if let Some(text) = unit {
            if let Some(BlockNode {
                block:
                    Block::Fence {
                        fact: Some(f),
                        text: code,
                        ..
                    },
                ..
            }) = blocks.get(i + 1)
                && f == id
            {
                return Some(format!("{text}\n\n{code}"));
            }
            return Some(text);
        }
    }
    None
}

fn unit_with_id(unit: &Unit, id: &str) -> Option<String> {
    let fact = unit.fact.as_ref()?;
    (fact.id.as_deref() == Some(id)).then(|| unit.text.clone())
}
