//! What a `--style` run says (PROP-057 `##STYLE-LINT`).
//!
//! A finding has to be actionable from the line alone: the page, the
//! block the margin numbers, the rule that fired and the words that
//! fired it. An author fixes prose by reading the sentence, so the
//! sentence is in the report and the report is the queue.
//!
//! Warnings never gate. The law puts the errors in the procedures and the
//! warnings in the corridors, and a corridor sentence of thirty-six words
//! is a sentence somebody should look at, not a build somebody should
//! stop.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#STYLE-LINT");

use std::fmt;

/// How loud a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Reported, never gated.
    Warning,
    /// Counted against the page: a page with one is not clean.
    Error,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Which rule fired. Named, because the queue is sorted by it and because
/// a false positive is fixed in the rule — never worked around in the
/// text (`##STYLE-LINT`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rule {
    /// A word or phrase from the package's list for this language.
    Banned,
    /// «see the specification» and its two companions: a citation is an
    /// addition, never a deferral.
    Deferral,
    /// A sentence over the limit its block kind carries.
    SentenceLength,
    /// A paragraph over six sentences.
    ParagraphLength,
    /// A glossary term in the page's first paragraph.
    TermInFirstParagraph,
    /// A term whose first use on the page introduces nothing.
    TermBeforeIntroduction,
    /// More than two glossary terms in one narrative sentence.
    TermsPerSentence,
    /// A heading the law forbids by name.
    Heading,
    /// An exclamation mark, an emoji, bold in prose, or a second em dash
    /// in one paragraph.
    Sign,
    /// A prompt with no assert on a page that opens with one.
    PromptWithoutAssert,
}

impl Rule {
    /// The name a report and a queue entry print.
    pub fn as_str(self) -> &'static str {
        match self {
            Rule::Banned => "banned-word",
            Rule::Deferral => "deferral",
            Rule::SentenceLength => "sentence-length",
            Rule::ParagraphLength => "paragraph-length",
            Rule::TermInFirstParagraph => "term-in-first-paragraph",
            Rule::TermBeforeIntroduction => "term-before-introduction",
            Rule::TermsPerSentence => "terms-per-sentence",
            Rule::Heading => "heading",
            Rule::Sign => "sign",
            Rule::PromptWithoutAssert => "prompt-without-assert",
        }
    }

    /// Every rule, in report order.
    pub const ALL: &'static [Rule] = &[
        Rule::Banned,
        Rule::Deferral,
        Rule::SentenceLength,
        Rule::ParagraphLength,
        Rule::TermInFirstParagraph,
        Rule::TermBeforeIntroduction,
        Rule::TermsPerSentence,
        Rule::Heading,
        Rule::Sign,
        Rule::PromptWithoutAssert,
    ];
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One thing the linter found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The page's address inside the package (`model/boot-lane.xml`).
    pub page: String,
    /// The block number the margin shows (`p07`), or the word that names
    /// a node with no number of its own (`title`).
    pub block: String,
    /// The kind of prose it sits in (`p`, `step`, `warning`, `cell`).
    pub node: &'static str,
    pub rule: Rule,
    pub severity: Severity,
    /// What is wrong, in one line.
    pub message: String,
    /// The words that fired the rule, so the author does not have to
    /// open the page to know which sentence is meant.
    pub text: String,
}

/// How long a quoted stretch of prose may be in a finding. Long enough to
/// recognise the sentence, short enough that a hundred findings stay a
/// page.
pub const QUOTE_LIMIT: usize = 90;

/// Trim a stretch of prose to [`QUOTE_LIMIT`] characters for a finding.
///
/// The code mask goes back to something a person recognises: a finding is
/// read by an author holding the page, and an object-replacement
/// character in the middle of a sentence tells them nothing.
///
/// ```
/// let masked = vibe_doc::style::inline::read("Run `vibe install` now.");
/// assert_eq!(vibe_doc::style::report::quote(&masked.text), "Run `…` now.");
/// ```
pub fn quote(text: &str) -> String {
    let text = text.replace(crate::style::inline::CODE_MASK, "`…`");
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= QUOTE_LIMIT {
        return flat;
    }
    let head: String = flat.chars().take(QUOTE_LIMIT).collect();
    format!("{head}…")
}

/// One page's score.
#[derive(Debug, Clone, PartialEq)]
pub struct PageScore {
    pub page: String,
    pub errors: usize,
    pub warnings: usize,
    /// The automated readability index of the page's corridors —
    /// reported, never gated.
    pub readability: f64,
}

impl PageScore {
    /// A page with no error is clean; warnings do not stop a build.
    pub fn clean(&self) -> bool {
        self.errors == 0
    }
}

/// One `--style` run.
#[derive(Debug, Clone)]
pub struct Report {
    /// The language the page prose was judged in.
    pub lang: String,
    pub findings: Vec<Finding>,
    pub pages: Vec<PageScore>,
    /// Pages the pivot refused. Their prose is unknown, and unknown is
    /// not clean.
    pub unreadable: Vec<String>,
    /// The share of clean pages this run demanded.
    pub min_percent: u8,
}

impl Report {
    /// How many pages carry no error.
    pub fn clean_pages(&self) -> usize {
        self.pages.iter().filter(|p| p.clean()).count()
    }

    /// The share of pages that carry no error, rounded DOWN: a run that
    /// is one page short of the bar must not print the bar.
    pub fn percent(&self) -> u8 {
        let total = self.pages.len() + self.unreadable.len();
        if total == 0 {
            return 100;
        }
        ((self.clean_pages() * 100) / total) as u8
    }

    pub fn errors(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .count()
    }

    pub fn warnings(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Warning)
            .count()
    }

    /// Green when the share of clean pages meets the bar and every page
    /// was readable.
    pub fn ok(&self) -> bool {
        self.unreadable.is_empty() && self.percent() >= self.min_percent
    }

    /// How many findings each rule raised, in report order.
    pub fn by_rule(&self) -> Vec<(Rule, usize, usize)> {
        Rule::ALL
            .iter()
            .map(|rule| {
                let of = |s: Severity| {
                    self.findings
                        .iter()
                        .filter(|f| f.rule == *rule && f.severity == s)
                        .count()
                };
                (*rule, of(Severity::Error), of(Severity::Warning))
            })
            .filter(|(_, e, w)| *e + *w > 0)
            .collect()
    }

    /// The median readability of the pages, and the page that reads
    /// hardest.
    pub fn readability(&self) -> Option<(f64, &PageScore)> {
        if self.pages.is_empty() {
            return None;
        }
        let mut scores: Vec<f64> = self.pages.iter().map(|p| p.readability).collect();
        scores.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = scores[scores.len() / 2];
        let worst = self
            .pages
            .iter()
            .max_by(|a, b| {
                a.readability
                    .partial_cmp(&b.readability)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(&self.pages[0]);
        Some((median, worst))
    }

    /// The human form: every finding, then the tally.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for finding in &self.findings {
            out.push_str(&format!(
                "  {} {} {} [{}] {} — {}\n",
                finding.severity.as_str().to_uppercase(),
                finding.page,
                finding.block,
                finding.node,
                finding.rule,
                finding.message
            ));
            if !finding.text.is_empty() {
                out.push_str(&format!("       {}\n", finding.text));
            }
        }
        for page in &self.unreadable {
            out.push_str(&format!("  ERROR {page} does not parse\n"));
        }
        for (rule, errors, warnings) in self.by_rule() {
            out.push_str(&format!(
                "  {rule:<24} {errors} error(s), {warnings} warning(s)\n"
            ));
        }
        if let Some((median, worst)) = self.readability() {
            out.push_str(&format!(
                "  readability (ARI)        median {median:.1}, hardest {} at {:.1}\n",
                worst.page, worst.readability
            ));
        }
        out.push_str(&format!(
            "style: {} of {} page(s) clean, {}% (threshold {}%), {} error(s), {} warning(s), \
             {} unreadable page(s) [{}]\n",
            self.clean_pages(),
            self.pages.len() + self.unreadable.len(),
            self.percent(),
            self.min_percent,
            self.errors(),
            self.warnings(),
            self.unreadable.len(),
            self.lang
        ));
        out
    }
}

#[cfg(test)]
mod tests;
