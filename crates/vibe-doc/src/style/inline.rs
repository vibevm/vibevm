//! Inline Markdown, read once so every style rule reads the same text
//! (PROP-057 `##STYLE-LINT`).
//!
//! The pivot keeps inline content as one literal string per unit
//! (PROP-045 `##INLINE-STAYS-MARKDOWN`), so a rule that scanned the raw
//! text would be scanning three different languages at once: prose, code
//! spans and link targets. It would then flag `[requires] capabilities` —
//! a field name in code font — as the marketing tic `capabilities`, and
//! count the characters of `../glossary/index.xml#lock-file` as words of
//! a sentence.
//!
//! So the text is read ONCE into [`Inline`]: code spans collapse to a
//! single placeholder (one token, no letters), a link keeps its text and
//! surrenders its target, and the emphasis markers are removed with their
//! ranges recorded. Every rule then reads `Inline::text` and means the
//! same thing by «the words on the page».

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#STYLE-LINT");

/// What a code span becomes: U+FFFC OBJECT REPLACEMENT CHARACTER. One
/// character, so a span counts as one word; not a letter, so it changes
/// no readability score and matches no term.
pub const CODE_MASK: char = '\u{FFFC}';

/// One Markdown link, by its place in the masked text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// Byte range of the link's TEXT inside [`Inline::text`].
    pub range: (usize, usize),
    /// The target, verbatim (`../glossary/index.xml#lock-file`).
    pub target: String,
}

/// One unit's inline content, read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Inline {
    /// The prose: code spans masked, link targets gone, emphasis markers
    /// removed.
    pub text: String,
    /// Byte ranges in [`Inline::text`] that were written in italics — the
    /// mark a term carries at the moment it is defined (STYLE.md §8).
    pub italic: Vec<(usize, usize)>,
    /// How many bold spans the unit carried. Bold never appears in prose,
    /// so the count is the finding.
    pub bold: usize,
    /// Every link, in order.
    pub links: Vec<Link>,
}

impl Inline {
    /// Whether `at` falls inside an italic span.
    pub fn is_italic(&self, at: (usize, usize)) -> bool {
        self.italic.iter().any(|(s, e)| *s <= at.0 && at.1 <= *e)
    }

    /// The link whose TEXT is exactly the range `at`, when there is one.
    pub fn link_at(&self, at: (usize, usize)) -> Option<&Link> {
        self.links
            .iter()
            .find(|l| l.range.0 <= at.0 && at.1 <= l.range.1)
    }
}

/// Read one unit's text.
///
/// ```
/// use vibe_doc::style::inline::{read, CODE_MASK};
///
/// let inline = read("Run `vibe install`, then open the [lock file](../glossary/index.xml#lock-file).");
/// assert!(inline.text.contains(CODE_MASK));
/// assert!(inline.text.contains("lock file"));
/// assert!(!inline.text.contains("glossary"));
/// assert_eq!(inline.links[0].target, "../glossary/index.xml#lock-file");
/// assert_eq!(inline.bold, 0);
/// ```
pub fn read(raw: &str) -> Inline {
    let mut out = Inline::default();
    let chars: Vec<char> = raw.chars().collect();
    let mut i = 0usize;
    let mut italic_open: Option<usize> = None;
    while i < chars.len() {
        match chars[i] {
            '`' => {
                i = code_span(&chars, i, &mut out.text);
            }
            '[' => match link(&chars, i) {
                Some((text, target, next)) => {
                    let start = out.text.len();
                    // The link's own text is inline content too: a link
                    // may hold a code span, and the mask has to reach it.
                    let inner = read(&text);
                    out.text.push_str(&inner.text);
                    let end = out.text.len();
                    for (s, e) in inner.italic {
                        out.italic.push((start + s, start + e));
                    }
                    out.bold += inner.bold;
                    out.links.push(Link {
                        range: (start, end),
                        target,
                    });
                    i = next;
                }
                None => {
                    out.text.push('[');
                    i += 1;
                }
            },
            '*' if i + 1 < chars.len() && chars[i + 1] == '*' => {
                // Bold is counted, not located: «bold never appears in
                // prose» is a rule about the unit, not about a word.
                out.bold += 1;
                i += 2;
            }
            '*' => {
                match italic_open.take() {
                    Some(start) => out.italic.push((start, out.text.len())),
                    None => italic_open = Some(out.text.len()),
                }
                i += 1;
            }
            c => {
                out.text.push(c);
                i += 1;
            }
        }
    }
    // An unpaired `*` opens nothing: `italic_open` goes out of scope
    // here unrecorded, because an italic that never closes is a defect of
    // the unit, and treating it as running to the end of the paragraph
    // would mark every term after it as «introduced».
    out
}

/// Consume a code span opened at `at`, appending one mask character.
///
/// An unterminated backtick is a literal backtick: a paragraph that opens
/// a span and never closes it is a defect of the page, and swallowing the
/// rest of it would hide every other finding in the same paragraph.
fn code_span(chars: &[char], at: usize, text: &mut String) -> usize {
    let mut ticks = 0usize;
    while at + ticks < chars.len() && chars[at + ticks] == '`' {
        ticks += 1;
    }
    let fence: Vec<char> = vec!['`'; ticks];
    let mut i = at + ticks;
    while i < chars.len() {
        if chars[i..].starts_with(&fence[..]) {
            text.push(CODE_MASK);
            return i + ticks;
        }
        i += 1;
    }
    for _ in 0..ticks {
        text.push('`');
    }
    at + ticks
}

/// Read `[text](target)` at `at`, or `None` when the brackets are prose.
fn link(chars: &[char], at: usize) -> Option<(String, String, usize)> {
    let mut depth = 0usize;
    let mut close = None;
    for (offset, c) in chars.iter().enumerate().skip(at) {
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(offset);
                    break;
                }
            }
            _ => {}
        }
    }
    let close = close?;
    if chars.get(close + 1) != Some(&'(') {
        return None;
    }
    let mut end = None;
    for (offset, c) in chars.iter().enumerate().skip(close + 2) {
        if *c == ')' {
            end = Some(offset);
            break;
        }
        // A newline inside a target means the `(` opened a parenthetical,
        // not a link.
        if *c == '\n' {
            return None;
        }
    }
    let end = end?;
    let text: String = chars[at + 1..close].iter().collect();
    let target: String = chars[close + 2..end].iter().collect();
    Some((text, target, end + 1))
}

#[cfg(test)]
mod tests;
