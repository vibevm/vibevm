//! The glossary's terms, and what counts as introducing one
//! (PROP-057 `##STYLE-CONTAINERS-AND-CORRIDORS`, `##STYLE-PAGE-SKELETON`).
//!
//! The terms are not a list this crate keeps: they are the sections of
//! the package's own glossary page, read at every run. A manual that adds
//! a term adds it in one place, and the linter knows it the same day.
//!
//! ## Ordinary words do not count
//!
//! `##STYLE-PAGE-SKELETON` was clarified at the first corpus check
//! (2026-09-12): a glossary word used in its ordinary English sense —
//! package, project, kind, feature, workspace, translation — is not a
//! term the reader has to be introduced to, while a word that means
//! something only in vibe — lock file, manifest, registry, store, index,
//! anchor, skill, contribution, fingerprint, receipt — is. Without that
//! rule the first paragraph of a page about packages could not say the
//! word «package», which is the opposite of what the law is for.
//!
//! The six ordinary words are spelled here rather than in package data
//! because the norm spells them: they are the clarification's own list,
//! and an author who wants a seventh is asking for a change to the rule,
//! not to a file.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#STYLE-CONTAINERS-AND-CORRIDORS");

use regex::Regex;

use crate::pages::PageSet;

/// The page a documentation package keeps its glossary on.
pub const GLOSSARY_PAGE: &str = "glossary/index.xml";

/// The glossary words the norm exempts: used in their ordinary English
/// sense, they are not terms (`##STYLE-PAGE-SKELETON`, 2026-09-12).
pub const ORDINARY: &[&str] = &[
    "package",
    "project",
    "kind",
    "feature",
    "workspace",
    "translation",
];

/// One term of the glossary.
#[derive(Debug, Clone)]
pub struct Term {
    /// The anchor a link must name: `lock-file`.
    pub anchor: String,
    /// The term as the glossary writes it: `lock file`.
    pub phrase: String,
    /// How many words it is, so the longest match wins.
    pub words: usize,
    pattern: Regex,
}

impl Term {
    /// Every occurrence of this term in a stretch of prose, as byte
    /// ranges.
    pub fn find(&self, text: &str) -> Vec<(usize, usize)> {
        self.pattern
            .find_iter(text)
            .map(|m| (m.start(), m.end()))
            .collect()
    }
}

/// Read the terms of a package's glossary, longest first.
///
/// A package with no glossary page yields no terms, and the term rules
/// then find nothing. That is the honest answer: the rules ask «was this
/// word introduced», and a manual with no glossary has introduced
/// nothing to compare against.
pub fn terms(set: &PageSet) -> Vec<Term> {
    let Some(page) = set.pages.iter().find(|p| p.rel == GLOSSARY_PAGE) else {
        return Vec::new();
    };
    let mut out: Vec<Term> = Vec::new();
    for section in &page.doc.sections {
        let Some(anchor) = &section.id else {
            continue;
        };
        let phrase = base_form(&section.title);
        if phrase.is_empty() || ORDINARY.iter().any(|o| o.eq_ignore_ascii_case(&phrase)) {
            continue;
        }
        let Some(pattern) = compile(&phrase) else {
            continue;
        };
        out.push(Term {
            anchor: anchor.clone(),
            words: phrase.split_whitespace().count(),
            phrase,
            pattern,
        });
    }
    // Longest first: «freshness fingerprint» must win over
    // «fingerprint», or the page is told to introduce the wrong word.
    out.sort_by(|a, b| b.words.cmp(&a.words).then(a.phrase.cmp(&b.phrase)));
    out
}

/// The term without its disambiguating parenthesis: the glossary heads an
/// entry `index (of a registry)` because two things are called an index,
/// and the words on the page are still «index».
fn base_form(title: &str) -> String {
    match title.split_once(" (") {
        Some((head, _)) => head.trim().to_owned(),
        None => title.trim().to_owned(),
    }
}

/// The pattern one term matches: whole words, any run of whitespace
/// between them, case-insensitive, and the last word may be a plural.
///
/// ```
/// use vibe_doc::style::glossary::pattern_for;
///
/// let re = pattern_for("lock file").unwrap();
/// assert!(re.is_match("the lock  file is written"));
/// assert!(re.is_match("two lock files"));
/// assert!(!re.is_match("deadlock filed"));
/// ```
pub fn pattern_for(phrase: &str) -> Option<Regex> {
    compile(phrase)
}

fn compile(phrase: &str) -> Option<Regex> {
    let words: Vec<&str> = phrase.split_whitespace().collect();
    let (last, head) = words.split_last()?;
    let mut source = String::from(r"(?i)\b");
    for word in head {
        source.push_str(&regex::escape(word));
        source.push_str(r"\s+");
    }
    source.push_str(&inflect(last));
    source.push_str(r"\b");
    Regex::new(&source).ok()
}

/// The last word of a term, with the plural the English of this manual
/// actually writes. A term is a noun phrase, so only the head noun
/// inflects, and only in the three regular ways.
fn inflect(word: &str) -> String {
    let escaped = regex::escape(word);
    if let Some(stem) = word.strip_suffix('y')
        && !word.ends_with("ay")
        && !word.ends_with("ey")
        && !word.ends_with("oy")
    {
        return format!("(?:{}|{}ies)", escaped, regex::escape(stem));
    }
    if word.ends_with('s') || word.ends_with("ch") || word.ends_with("sh") || word.ends_with('x') {
        return format!("(?:{escaped})(?:es)?");
    }
    format!("(?:{escaped})(?:s)?")
}

#[cfg(test)]
mod tests;
