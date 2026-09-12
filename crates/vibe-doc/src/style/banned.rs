//! The banned words and phrases, per language (STYLE.md §3 and §10,
//! PROP-057 `##STYLE-LINT`).
//!
//! The lists are DATA in the documentation package —
//! `style/banned.en.txt`, `style/banned.ru.txt` — not a constant in this
//! crate. A tic is a property of a language and of a house style, and an
//! adaptation into a language nobody here writes must be able to bring
//! its own list without a release of vibe.
//!
//! Matching is what the law says it is: case-insensitive, on whole words,
//! and a phrase as a sequence of words. «just» does not match «justify»,
//! and «note that» matches across any run of whitespace. Nothing matches
//! inside a code span, because the mask has already removed it: a manual
//! that documents the field `[requires] capabilities` writes the word
//! `capabilities` in code font, and a linter that flagged it would be
//! telling the author to rename the product.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#STYLE-LINT");

use std::path::{Path, PathBuf};

use crate::error::{DocError, Result};

/// Where a documentation package keeps its style data.
pub const STYLE_DIR: &str = "style";

/// The three phrases the law names as DEFERRALS rather than as tics: they
/// send the reader to the specification instead of explaining, and
/// STYLE.md §11 gives them a rule and a message of their own.
///
/// They stand in the banned list as well, because the list is what an
/// author reads; the linter reports each occurrence ONCE, under the
/// deferral rule, so a page is never told the same thing twice.
pub const DEFERRALS: &[&str] = &[
    "see the specification",
    "refer to the spec",
    "as described in",
];

/// One entry of a list, ready to match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The entry as the file spells it.
    pub phrase: String,
    /// Its words, lower-cased.
    words: Vec<String>,
}

impl Entry {
    /// Whether this entry is one of the three deferrals.
    pub fn is_deferral(&self) -> bool {
        DEFERRALS
            .iter()
            .any(|d| d.eq_ignore_ascii_case(&self.phrase))
    }
}

/// A language's whole list.
#[derive(Debug, Clone, Default)]
pub struct List {
    pub lang: String,
    pub entries: Vec<Entry>,
}

/// The file a language's list lives in.
pub fn list_path(package_dir: &Path, lang: &str) -> PathBuf {
    package_dir
        .join(STYLE_DIR)
        .join(format!("banned.{lang}.txt"))
}

/// Read the list for a language.
///
/// A package written in a language it ships no list for is a package the
/// check cannot run on, and it says so by name rather than passing an
/// empty list: «no banned word found» and «no list to find one in» print
/// the same zero.
pub fn read(package_dir: &Path, lang: &str) -> Result<List> {
    let path = list_path(package_dir, lang);
    let text = std::fs::read_to_string(&path).map_err(|e| DocError::Style {
        message: format!(
            "the package is written in `{lang}` and carries no `{}`: {e}",
            path.display()
        ),
    })?;
    Ok(List {
        lang: lang.to_owned(),
        entries: parse(&text),
    })
}

/// Read a list from its text.
///
/// ```
/// let list = vibe_doc::style::banned::parse("# a comment\ndelve\nnote that\n\n");
/// assert_eq!(list.len(), 2);
/// assert_eq!(list[1].phrase, "note that");
/// ```
pub fn parse(text: &str) -> Vec<Entry> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| Entry {
            phrase: line.to_owned(),
            words: line.split_whitespace().map(lower).collect(),
        })
        .filter(|e| !e.words.is_empty())
        .collect()
}

/// One occurrence of a banned entry in a stretch of prose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    /// Byte range in the text that was scanned.
    pub range: (usize, usize),
    /// The entry that matched.
    pub phrase: String,
    /// Whether the entry is a deferral rather than a tic.
    pub deferral: bool,
}

/// Every banned entry in one stretch of prose, in order of appearance.
///
/// ```
/// use vibe_doc::style::banned::{parse, scan};
///
/// let list = parse("just\nnote that");
/// let hits = scan("Note  that it is just a copy, and justify nothing.", &list);
/// assert_eq!(hits.len(), 2);
/// assert_eq!(hits[0].phrase, "note that");
/// assert_eq!(hits[1].phrase, "just");
/// ```
pub fn scan(text: &str, list: &[Entry]) -> Vec<Hit> {
    let words = words_of(text);
    let mut hits: Vec<Hit> = Vec::new();
    for entry in list {
        let n = entry.words.len();
        if n > words.len() {
            continue;
        }
        for start in 0..=words.len() - n {
            if (0..n).all(|k| words[start + k].0 == entry.words[k]) {
                hits.push(Hit {
                    range: (words[start].1, words[start + n - 1].2),
                    phrase: entry.phrase.clone(),
                    deferral: entry.is_deferral(),
                });
            }
        }
    }
    // Longest first at any one position, so a page that writes «a number
    // of» is told about the phrase and not, a second time, about «a».
    hits.sort_by(|a, b| a.range.0.cmp(&b.range.0).then(b.range.1.cmp(&a.range.1)));
    hits.dedup_by(|later, first| later.range.0 == first.range.0);
    hits
}

/// The words of a stretch of prose, lower-cased, with their byte ranges.
/// A word is a run of alphanumerics and the marks that live inside a word
/// (`-`, `'`), so `cutting-edge` is one word and `state-of-the-art` is
/// one word.
fn words_of(text: &str) -> Vec<(String, usize, usize)> {
    let mut out: Vec<(String, usize, usize)> = Vec::new();
    let mut start: Option<usize> = None;
    for (at, c) in text.char_indices() {
        if c.is_alphanumeric() || c == '-' || c == '\'' || c == '’' {
            start.get_or_insert(at);
            continue;
        }
        if let Some(from) = start.take() {
            out.push((lower(&text[from..at]), from, at));
        }
    }
    if let Some(from) = start {
        out.push((lower(&text[from..]), from, text.len()));
    }
    out
}

fn lower(word: &str) -> String {
    word.to_lowercase()
}

#[cfg(test)]
mod tests;
