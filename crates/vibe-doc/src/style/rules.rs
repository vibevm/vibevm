//! The mechanical rules: tics, length, forbidden headings and signs
//! (STYLE.md §11, PROP-057 `##STYLE-LINT`; the limits are X-028's
//! calibration).
//!
//! The length limits are per BLOCK KIND, and the split is the law's:
//! errors in the procedures and the warnings, warnings in the corridors.
//! A step of a procedure is an instruction a person follows with their
//! hands busy, so twenty words is a hard limit; a corridor is an essay
//! paragraph, so thirty-five words is a note to the author and not a
//! stopped build. The prompt is the user's own voice and carries no limit
//! at all.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#STYLE-LINT");

use vibe_specdoc::doc::{NoteKind, Section, SpecDoc};

use crate::style::banned::{self, Entry};
use crate::style::prose::{Kind, Node};
use crate::style::report::{Finding, Rule, Severity, quote};
use crate::style::sentence;

/// The longest a sentence may be, by the kind of block it sits in, and
/// how loud going over is.
///
/// ```
/// use vibe_doc::style::prose::Kind;
/// use vibe_doc::style::report::Severity;
/// use vibe_doc::style::rules::sentence_limit;
///
/// assert_eq!(sentence_limit(Kind::Step, false), Some((20, Severity::Error)));
/// assert_eq!(sentence_limit(Kind::Paragraph, true), Some((25, Severity::Error)));
/// assert_eq!(sentence_limit(Kind::Paragraph, false), Some((35, Severity::Warning)));
/// assert_eq!(sentence_limit(Kind::Prompt, false), None);
/// ```
pub fn sentence_limit(kind: Kind, procedural: bool) -> Option<(usize, Severity)> {
    match kind {
        Kind::Step | Kind::Note(NoteKind::Warning) => Some((STEP_LIMIT, Severity::Error)),
        Kind::Paragraph if procedural => Some((PROCEDURE_LIMIT, Severity::Error)),
        Kind::Paragraph | Kind::Quote | Kind::Note(_) | Kind::Caption => {
            Some((CORRIDOR_LIMIT, Severity::Warning))
        }
        Kind::Title | Kind::Cell | Kind::Prompt | Kind::Needs | Kind::Outcome => None,
    }
}

/// A step of a list and the body of a warning: twenty words (STYLE.md §5).
pub const STEP_LIMIT: usize = 20;
/// A paragraph inside a procedure: twenty-five words (STYLE.md §5).
pub const PROCEDURE_LIMIT: usize = 25;
/// A corridor paragraph: thirty-five words, and a warning (X-028).
pub const CORRIDOR_LIMIT: usize = 35;
/// A paragraph of more than six sentences is a warning (STYLE.md §5).
pub const PARAGRAPH_SENTENCES: usize = 6;

/// The headings the law forbids by name (STYLE.md §3, §11).
pub const FORBIDDEN_HEADINGS: &[&str] = &[
    "overview",
    "summary",
    "conclusion",
    "next steps",
    "key takeaways",
];

/// The non-ASCII marks the manual's typography uses. Everything else
/// outside the letters of the page's own alphabet is an emoji or a sign
/// that does not belong in prose.
const TYPOGRAPHY: &[char] = &[
    '«', '»', '—', '–', '…', '‘', '’', '“', '”', '·', '°', '×', '→', '←', '≥', '≤', '±', '§', '¶',
    '′', '″', '½', '¼', '¾', '©', '®', '™', '\u{FFFC}',
];

/// The em dash a paragraph may carry once (STYLE.md §8).
const EM_DASH: char = '—';

/// Every tic and deferral in one node's prose.
///
/// A deferral phrase is forgiven in exactly one place: when the block is
/// followed by the `rule` that carries the quoted fact. «The rule, in the
/// specification's own words:» and then the words is a citation ADDING
/// the exact wording, which is what STYLE.md §2 asks of a page; the same
/// sentence with nothing behind it sends the reader to a specification
/// instead of explaining.
pub fn banned_words(page: &str, node: &Node, list: &[Entry]) -> Vec<Finding> {
    banned::scan(&node.inline.text, list)
        .into_iter()
        .filter(|hit| !(hit.deferral && node.cites_next))
        .map(|hit| {
            let rule = if hit.deferral {
                Rule::Deferral
            } else {
                Rule::Banned
            };
            let message = if hit.deferral {
                format!(
                    "`{}` sends the reader away instead of explaining — a citation is an \
                     addition, never a deferral",
                    hit.phrase
                )
            } else {
                format!("`{}` is on this language's list", hit.phrase)
            };
            Finding {
                page: page.to_owned(),
                block: node.block.clone(),
                node: node.kind.as_str(),
                rule,
                severity: Severity::Error,
                message,
                text: quote(around(&node.inline.text, hit.range)),
            }
        })
        .collect()
}

/// The sentences and paragraphs that run long.
pub fn length(page: &str, node: &Node) -> Vec<Finding> {
    let mut out: Vec<Finding> = Vec::new();
    let sentences = sentence::split(&node.inline.text);
    if let Some((limit, severity)) = sentence_limit(node.kind, node.procedural) {
        for (index, s) in sentences.iter().enumerate() {
            let words = s.words();
            if words <= limit {
                continue;
            }
            out.push(Finding {
                page: page.to_owned(),
                block: node.block.clone(),
                node: node.kind.as_str(),
                rule: Rule::SentenceLength,
                severity,
                message: format!(
                    "sentence {} is {words} words, and a {} allows {limit}",
                    index + 1,
                    node.kind.as_str()
                ),
                text: quote(&s.text),
            });
        }
    }
    let counted = matches!(node.kind, Kind::Paragraph | Kind::Quote | Kind::Note(_));
    if counted && sentences.len() > PARAGRAPH_SENTENCES {
        out.push(Finding {
            page: page.to_owned(),
            block: node.block.clone(),
            node: node.kind.as_str(),
            rule: Rule::ParagraphLength,
            severity: Severity::Warning,
            message: format!(
                "{} sentences in one paragraph, and the limit is {PARAGRAPH_SENTENCES}",
                sentences.len()
            ),
            text: quote(&node.inline.text),
        });
    }
    out
}

/// Exclamation marks, emoji, bold in prose and a second em dash.
///
/// A table cell is left alone: bold marks a header, an arrow belongs in a
/// column of states, and a cell is a container rather than prose.
pub fn signs(page: &str, node: &Node) -> Vec<Finding> {
    if node.kind == Kind::Cell {
        return Vec::new();
    }
    let mut out: Vec<Finding> = Vec::new();
    let mut sign = |rule: Rule, message: String, text: String| {
        out.push(Finding {
            page: page.to_owned(),
            block: node.block.clone(),
            node: node.kind.as_str(),
            rule,
            severity: Severity::Error,
            message,
            text,
        });
    };
    let text = &node.inline.text;
    if text.contains('!') {
        sign(Rule::Sign, "an exclamation mark".to_owned(), quote(text));
    }
    let strange: Vec<char> = text
        .chars()
        .filter(|c| !c.is_ascii() && !c.is_alphabetic() && !TYPOGRAPHY.contains(c))
        .collect();
    if !strange.is_empty() {
        sign(
            Rule::Sign,
            format!(
                "a sign that is not the manual's typography: {}",
                strange.iter().collect::<String>()
            ),
            quote(text),
        );
    }
    if node.inline.bold > 0 {
        sign(
            Rule::Sign,
            format!(
                "{} bold span(s) — bold never appears in prose; an identifier goes in code font",
                node.inline.bold
            ),
            quote(text),
        );
    }
    let dashes = text.chars().filter(|c| *c == EM_DASH).count();
    if dashes > 1 {
        sign(
            Rule::Sign,
            format!("{dashes} em dashes in one paragraph, and the limit is one"),
            quote(text),
        );
    }
    out
}

/// The headings a page must not carry.
pub fn headings(page: &str, doc: &SpecDoc) -> Vec<Finding> {
    let mut out: Vec<Finding> = Vec::new();
    if let Some(title) = &doc.title {
        judge_heading(page, "title", &title.text, &mut out);
    }
    fn walk(page: &str, sections: &[Section], out: &mut Vec<Finding>) {
        for section in sections {
            let block = section.id.clone().unwrap_or_else(|| "section".to_owned());
            judge_heading(page, &block, &section.title, out);
            walk(page, &section.sections, out);
        }
    }
    walk(page, &doc.sections, &mut out);
    out
}

fn judge_heading(page: &str, block: &str, title: &str, out: &mut Vec<Finding>) {
    let flat = title.trim().to_lowercase();
    if !FORBIDDEN_HEADINGS.contains(&flat.as_str()) {
        return;
    }
    out.push(Finding {
        page: page.to_owned(),
        block: block.to_owned(),
        node: "heading",
        rule: Rule::Heading,
        severity: Severity::Error,
        message: format!(
            "a section called `{}` — the navigation does that, and a page that needs one \
             has not decided what it is about",
            title.trim()
        ),
        text: String::new(),
    });
}

/// The words around a hit, so the finding quotes a sentence and not a
/// word.
fn around(text: &str, range: (usize, usize)) -> &str {
    let start = text[..range.0].rfind(['.', '\n']).map_or(0, |at| at + 1);
    let end = text[range.1..]
        .find(['.', '\n'])
        .map_or(text.len(), |at| range.1 + at + 1);
    text[start..end].trim()
}

#[cfg(test)]
mod tests;
