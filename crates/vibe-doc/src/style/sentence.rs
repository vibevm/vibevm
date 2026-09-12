//! Sentences and words, counted the way the length rules mean them
//! (STYLE.md §5, X-028).
//!
//! Two decisions carry the whole module.
//!
//! **A step number is not a full stop.** The corpus writes a procedure as
//! `1. Run it. 2. Check it.` inside one paragraph, and a splitter that
//! broke on every `.` would report a four-word sentence where a reader
//! sees a step. So a `.` right after a one or two digit number at the
//! head of a segment opens a step; it does not close a sentence.
//!
//! **A code span is one word.** `vibe install org.vibevm.world/wal` is a
//! command, not eight words of English, and counting its parts would push
//! every sentence that shows a command over the limit for a reason no
//! author could act on. The mask [`crate::style::inline::CODE_MASK`] is
//! one character and counts as one word.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#STYLE-LINT");

/// The abbreviations whose full stop is inside a word. Short and closed:
/// a longer list would start swallowing real sentence ends.
const ABBREVIATIONS: &[&str] = &["e.g", "i.e", "etc", "vs", "cf", "Mr", "Ms", "Dr", "No"];

/// One sentence: its text and where it sits in the unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sentence {
    /// Byte range inside the text it was split from.
    pub range: (usize, usize),
    pub text: String,
}

impl Sentence {
    /// How many words it holds.
    pub fn words(&self) -> usize {
        count_words(&self.text)
    }
}

/// Split a unit's prose into sentences.
///
/// ```
/// use vibe_doc::style::sentence::split;
///
/// let s = split("Run it. Then check the result.");
/// assert_eq!(s.len(), 2);
/// assert_eq!(s[1].text, "Then check the result.");
///
/// // A numbered procedure inside one paragraph stays one sentence per step.
/// let steps = split("1. Run it. 2. Check it.");
/// assert_eq!(steps.len(), 2);
/// ```
pub fn split(text: &str) -> Vec<Sentence> {
    let bytes = text.as_bytes();
    let mut out: Vec<Sentence> = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        if c != b'.' && c != b'!' && c != b'?' {
            i += 1;
            continue;
        }
        let mut end = i + 1;
        while end < bytes.len() && matches!(bytes[end], b'.' | b'!' | b'?') {
            end += 1;
        }
        let terminal = end >= bytes.len() || bytes[end].is_ascii_whitespace();
        if !terminal || opens_a_step(text, start, i) || is_abbreviation(text, i) {
            i = end;
            continue;
        }
        push(text, start, end, &mut out);
        start = end;
        i = end;
    }
    push(text, start, text.len(), &mut out);
    out
}

fn push(text: &str, start: usize, end: usize, out: &mut Vec<Sentence>) {
    let slice = &text[start..end];
    if slice.trim().is_empty() {
        return;
    }
    let lead = slice.len() - slice.trim_start().len();
    out.push(Sentence {
        range: (start + lead, end),
        text: slice.trim().to_owned(),
    });
}

/// Whether the full stop at `at` closes a step marker — a one or two
/// digit number standing at the head of the current segment.
fn opens_a_step(text: &str, start: usize, at: usize) -> bool {
    let head = text[start..at].trim_start();
    !head.is_empty() && head.len() <= 2 && head.chars().all(|c| c.is_ascii_digit())
}

/// Whether the full stop at `at` sits inside one of the closed list of
/// abbreviations.
fn is_abbreviation(text: &str, at: usize) -> bool {
    let head = &text[..at];
    ABBREVIATIONS.iter().any(|a| {
        head.ends_with(a)
            && head[..head.len() - a.len()]
                .chars()
                .next_back()
                .is_none_or(|c| !c.is_alphanumeric())
    })
}

/// Count the words of a stretch of prose.
///
/// ```
/// assert_eq!(vibe_doc::style::sentence::count_words("Run the lock file check."), 5);
/// ```
pub fn count_words(text: &str) -> usize {
    text.split_whitespace()
        .filter(|w| w.chars().any(|c| !is_punctuation(c)))
        .count()
}

fn is_punctuation(c: char) -> bool {
    !c.is_alphanumeric() && c != crate::style::inline::CODE_MASK
}

/// The automated readability index of a stretch of prose — the score the
/// check REPORTS and never gates on (STYLE.md §11).
///
/// ARI rather than Flesch: Flesch counts syllables, and a syllable
/// counter is a per-language dictionary this project would then have to
/// keep for every adaptation. ARI counts characters, words and sentences,
/// which every language this manual is written in has.
///
/// The result is a US school grade: 8 is a broadsheet, 12 is a serious
/// magazine, 16 is an academic paper.
///
/// ```
/// let score = vibe_doc::style::sentence::readability("Run it. Check it. Ship it.");
/// assert!(score < 6.0, "three three-word sentences read easily: {score}");
/// ```
pub fn readability(text: &str) -> f64 {
    let sentences = split(text).len().max(1) as f64;
    let words = count_words(text).max(1) as f64;
    let letters = text.chars().filter(|c| c.is_alphanumeric()).count() as f64;
    4.71 * (letters / words) + 0.5 * (words / sentences) - 21.43
}

#[cfg(test)]
mod tests;
