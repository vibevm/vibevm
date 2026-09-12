//! The three term rules: the first paragraph, the first use, and how many
//! terms one sentence may carry
//! (PROP-057 `##STYLE-CONTAINERS-AND-CORRIDORS`, `##STYLE-PAGE-SKELETON`).
//!
//! These are the density checks, and they are the reason the style law
//! exists: a page packed with the specification's own vocabulary reads
//! fine to whoever wrote the specification and to nobody else.
//!
//! ## What counts as introducing a term
//!
//! The law names two ways — a link to the glossary entry, or a gloss in
//! plain words in the same sentence — and STYLE.md §8 adds a third, the
//! italics a term wears at the moment it is defined. The gloss is the
//! only one a machine has to judge, and it is judged narrowly: the term
//! is followed, in the same sentence, by a comma, a colon, a dash or an
//! opening parenthesis, and then by enough words to be an explanation.
//! A looser test — «is there a comma anywhere in the sentence» — reports
//! a page as clean whenever it happens to use a comma, which is a check
//! that says yes for the wrong reason.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#STYLE-CONTAINERS-AND-CORRIDORS");

use std::collections::BTreeSet;

use crate::style::glossary::{GLOSSARY_PAGE, Term};
use crate::style::prose::{Kind, Node};
use crate::style::report::{Finding, Rule, Severity, quote};
use crate::style::sentence;

/// How many glossary terms one narrative sentence may carry (STYLE.md §2).
pub const TERMS_PER_SENTENCE: usize = 2;

/// How many words must follow the gloss mark before it counts as an
/// explanation rather than a comma.
const GLOSS_WORDS: usize = 4;

/// The marks a gloss opens with.
const GLOSS_MARKS: &[char] = &[',', ':', '—', '(', '–'];

/// One occurrence of one term.
struct Seen<'a> {
    term: &'a Term,
    range: (usize, usize),
}

/// Judge the term rules over one page's prose.
///
/// `nodes` is the page in document order; `terms` is the glossary,
/// longest first.
pub fn check(page: &str, nodes: &[Node], terms: &[Term]) -> Vec<Finding> {
    if page == GLOSSARY_PAGE || terms.is_empty() {
        // The glossary is where every term is defined. Judging its own
        // entries against «introduce before use» would ask each entry to
        // link to itself.
        return Vec::new();
    }
    let mut out: Vec<Finding> = Vec::new();
    let mut introduced: BTreeSet<&str> = BTreeSet::new();
    for node in nodes {
        if !judged(node) {
            continue;
        }
        let found = occurrences(&node.inline.text, terms);
        out.extend(density(page, node, &found));
        for seen in &found {
            if !introduced.insert(seen.term.anchor.as_str()) {
                continue;
            }
            if node.intro {
                out.push(first_paragraph_finding(page, node, seen));
                continue;
            }
            if introduces(node, seen) {
                continue;
            }
            out.push(first_use_finding(page, node, seen));
        }
    }
    out
}

/// Which prose a term rule judges: the corridors, and not the glossary's
/// own entries, the tables, the titles or the user's voice in a prompt.
fn judged(node: &Node) -> bool {
    node.kind.is_corridor() && node.kind != Kind::Caption
}

/// Every term occurrence in one stretch of prose, longest match first and
/// no overlaps, in order of appearance.
fn occurrences<'a>(text: &str, terms: &'a [Term]) -> Vec<Seen<'a>> {
    let mut taken: Vec<(usize, usize)> = Vec::new();
    let mut out: Vec<Seen<'a>> = Vec::new();
    for term in terms {
        for range in term.find(text) {
            if taken.iter().any(|(s, e)| range.0 < *e && *s < range.1) {
                continue;
            }
            taken.push(range);
            out.push(Seen { term, range });
        }
    }
    out.sort_by_key(|s| s.range.0);
    out
}

/// More than two terms in one narrative sentence.
fn density(page: &str, node: &Node, found: &[Seen<'_>]) -> Vec<Finding> {
    if node.kind != Kind::Paragraph && node.kind != Kind::Step {
        return Vec::new();
    }
    let mut out: Vec<Finding> = Vec::new();
    for s in sentence::split(&node.inline.text) {
        let inside: Vec<&str> = found
            .iter()
            .filter(|seen| seen.range.0 >= s.range.0 && seen.range.1 <= s.range.1)
            .map(|seen| seen.term.phrase.as_str())
            .collect();
        if inside.len() <= TERMS_PER_SENTENCE {
            continue;
        }
        out.push(Finding {
            page: page.to_owned(),
            block: node.block.clone(),
            node: node.kind.as_str(),
            rule: Rule::TermsPerSentence,
            severity: Severity::Warning,
            message: format!(
                "{} glossary terms in one sentence ({}) — three means the sentence is a \
                 container in disguise",
                inside.len(),
                inside.join(", ")
            ),
            text: quote(&s.text),
        });
    }
    out
}

fn first_paragraph_finding(page: &str, node: &Node, seen: &Seen<'_>) -> Finding {
    Finding {
        page: page.to_owned(),
        block: node.block.clone(),
        node: node.kind.as_str(),
        rule: Rule::TermInFirstParagraph,
        severity: Severity::Error,
        message: format!(
            "`{}` in the first paragraph — it says what the page is for to a stranger, and \
             it is the page's line in `llms.txt`",
            seen.term.phrase
        ),
        text: quote(&node.inline.text),
    }
}

fn first_use_finding(page: &str, node: &Node, seen: &Seen<'_>) -> Finding {
    Finding {
        page: page.to_owned(),
        block: node.block.clone(),
        node: node.kind.as_str(),
        rule: Rule::TermBeforeIntroduction,
        severity: Severity::Error,
        message: format!(
            "`{}` is used before it is introduced — link it to `{}#{}` or gloss it in the \
             same sentence",
            seen.term.phrase, GLOSSARY_PAGE, seen.term.anchor
        ),
        text: quote(sentence_of(&node.inline.text, seen.range)),
    }
}

/// Whether this occurrence introduces the term: a link to its glossary
/// entry, the italics of a definition, or a gloss in the same sentence.
fn introduces(node: &Node, seen: &Seen<'_>) -> bool {
    if let Some(link) = node.inline.link_at(seen.range)
        && link.target.contains(&seen.term.anchor)
    {
        return true;
    }
    if node.inline.is_italic(seen.range) {
        return true;
    }
    glossed(&node.inline.text, seen.range)
}

/// A gloss follows the term in the same sentence.
fn glossed(text: &str, range: (usize, usize)) -> bool {
    let Some(s) = sentence::split(text)
        .into_iter()
        .find(|s| s.range.0 <= range.0 && range.1 <= s.range.1)
    else {
        return false;
    };
    // What may stand between the term and its gloss: the marks that
    // close a quoted or parenthesised term, and whitespace. A WORD may
    // not — «the lock file is written» glosses nothing, however many
    // commas the rest of the sentence holds.
    let tail = &text[range.1..s.range.1];
    let after = tail.trim_start_matches([')', '`', '»', '”', '\'', '"', ' ', '\n', '\t']);
    let Some(mark) = after.chars().next() else {
        return false;
    };
    if !GLOSS_MARKS.contains(&mark) {
        return false;
    }
    sentence::count_words(&after[mark.len_utf8()..]) >= GLOSS_WORDS
}

/// The sentence one occurrence sits in, for the finding's quote.
fn sentence_of(text: &str, range: (usize, usize)) -> &str {
    let found = sentence::split(text)
        .into_iter()
        .find(|s| s.range.0 <= range.0 && range.1 <= s.range.1)
        .map(|s| s.range);
    match found {
        Some((start, end)) => text[start..end].trim(),
        None => text,
    }
}

#[cfg(test)]
mod tests;
