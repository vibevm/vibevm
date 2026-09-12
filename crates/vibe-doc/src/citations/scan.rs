//! Finding the addresses a page writes, and the line each was written on.
//!
//! Two halves that look alike and are not. The `rule` addresses come from
//! the IR, where the pivot has already dropped any `~rN` pin, so they are
//! exactly what a `documents` edge should carry. The prose addresses come
//! from the page's raw bytes, because prose is one text string in the IR
//! and an address inside it is not a construct the pivot models.
//!
//! The LINE comes from the raw bytes either way: the IR is a document
//! model, not a parse tree, and it carries no line numbers. An edge needs
//! one — a red gate that cannot say WHERE is a gate somebody has to grep
//! for — so the raw text is walked in step with the IR's own order.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#OBS-RULE-EDGE-UNPINNED");

use vibe_specdoc::doc::{Block, BlockNode, Section, SpecDoc};

/// Every `rule` address on a page, in document order.
pub fn rule_uris(doc: &SpecDoc) -> Vec<String> {
    fn from_blocks(blocks: &[BlockNode], out: &mut Vec<String>) {
        for node in blocks {
            if let Block::Rule { uri, .. } = &node.block {
                out.push(uri.clone());
            }
        }
    }
    fn from_section(section: &Section, out: &mut Vec<String>) {
        from_blocks(&section.blocks, out);
        for sub in &section.sections {
            from_section(sub, out);
        }
    }
    let mut out = Vec::new();
    from_blocks(&doc.preamble, &mut out);
    for section in &doc.sections {
        from_section(section, &mut out);
    }
    out
}

/// Locate each `rule` in the page's own bytes, in the same order the IR
/// holds them.
///
/// Each `<rule` opening is matched against the address the IR expects
/// next, rather than counted positionally: a `<rule` written inside a
/// fence — as the authoring page does when it shows the element to a
/// reader — carries a different address and is stepped over.
pub fn rule_lines(raw: &str, uris: &[String]) -> Vec<u32> {
    let starts = line_starts(raw);
    let mut out = Vec::with_capacity(uris.len());
    let mut cursor = 0usize;
    for uri in uris {
        match find_rule(raw, cursor, uri) {
            Some(at) => {
                out.push(line_of(&starts, at));
                cursor = at + 1;
            }
            // A line the walk cannot find is reported as «unknown», never
            // guessed: a wrong `file:line` sends a reader to the wrong
            // place with full confidence.
            None => out.push(0),
        }
    }
    out
}

/// The next `<rule` at or after `from` whose element carries `uri` —
/// bare, or with the `~rN` pin the pivot drops.
fn find_rule(raw: &str, from: usize, uri: &str) -> Option<usize> {
    let mut at = from;
    while let Some(hit) = raw[at..].find("<rule") {
        let start = at + hit;
        let end = raw[start..]
            .find('>')
            .map(|e| start + e)
            .unwrap_or(raw.len());
        let element = &raw[start..end];
        if element.contains(&format!("ref=\"{uri}\""))
            || element.contains(&format!("ref=\"{uri}~r"))
        {
            return Some(start);
        }
        at = start + "<rule".len();
    }
    None
}

/// Every `spec://` address written inside a page's prose, with its line.
///
/// Deliberately over-inclusive and then classified: the point is to catch
/// a dead address wherever an author wrote one, and the three prose forms
/// (`Classification`) are what keeps an illustration out of the gate.
/// `rule` elements are excluded here — they come from the IR.
pub fn prose_uris(raw: &str) -> Vec<(String, u32)> {
    let starts = line_starts(raw);
    let mut out = Vec::new();
    let mut at = 0usize;
    while let Some(hit) = raw[at..].find("spec://") {
        let start = at + hit;
        at = start + "spec://".len();
        if in_rule_element(raw, start) {
            continue;
        }
        let uri = take_address(&raw[start..]);
        if uri.len() <= "spec://".len() {
            continue;
        }
        out.push((uri, line_of(&starts, start)));
    }
    out
}

/// Is this occurrence inside a `<rule …>` element's own attributes?
fn in_rule_element(raw: &str, at: usize) -> bool {
    let window = raw[..at].rfind('<').unwrap_or(0);
    raw[window..at].starts_with("<rule") && !raw[window..at].contains('>')
}

/// The address token at the head of `rest`: everything up to the first
/// character an address cannot carry. Sentence punctuation at the end is
/// trimmed — an address at the end of a sentence is followed by a full
/// stop, and the full stop is the sentence's, not the address's.
fn take_address(rest: &str) -> String {
    let end = rest
        .find(|c: char| {
            c.is_whitespace() || matches!(c, '"' | '\'' | '`' | '<' | '>' | ')' | ']' | '|')
        })
        .unwrap_or(rest.len());
    rest[..end]
        .trim_end_matches(['.', ',', ';', ':', '!', '?'])
        .to_owned()
}

/// The byte offset each line begins at, so an offset becomes a line
/// number by one binary search instead of one scan per address.
fn line_starts(raw: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (i, b) in raw.bytes().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

/// The 1-based line an offset falls on.
fn line_of(starts: &[usize], at: usize) -> u32 {
    match starts.binary_search(&at) {
        Ok(i) => (i + 1) as u32,
        Err(i) => i as u32,
    }
}
