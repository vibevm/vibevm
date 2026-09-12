//! A README, as a page (PROP-057 `##LEVEL-ZERO`).
//!
//! The documentation dialect is a closed subset of Markdown
//! (PROP-045 `##XML-DIALECT-IS-THE-MD-SUBSET`), and a README is
//! Markdown — so this is a translation between two forms of one
//! language, not a parser for a foreign one. Three block shapes carry
//! nearly every README ever written: a heading opens a section, a fenced
//! block is a fence, and everything else is a paragraph, which the
//! dialect already renders with its inline Markdown intact.
//!
//! ## What it deliberately does not understand
//!
//! Lists and tables arrive as paragraphs, with their own `-` and `|`
//! still in the text. That is a limit and it is stated rather than
//! hidden: turning it into a full Markdown parser would put a second
//! reader of Markdown in this repository, and the pivot is the first.
//! A package that wants its list rendered as a list writes a
//! documentation package, which is what level 1 is for.
//!
//! Headings are flattened to one level of section. Nesting them by
//! depth would invent a structure a README is not required to keep —
//! plenty of them jump from `#` to `###` — and a section that closed at
//! the wrong place would move text under the wrong heading, which is
//! worse than a flat page.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#LEVEL-ZERO");

/// One block of a README.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// A heading — it opens a section, at whatever depth it was written.
    Heading { text: String },
    /// A paragraph of prose, its inline Markdown untouched.
    Paragraph { text: String },
    /// A fenced block, with the language it declared.
    Fence { lang: Option<String>, body: String },
}

/// Read a README into the blocks a page is built from.
///
/// ```
/// use vibe_doc::site::level0::readme::{blocks, Block};
///
/// let read = blocks("# Title\n\ntext\n\n```sh\nrun\n```\n");
/// assert_eq!(read.len(), 3);
/// assert!(matches!(&read[0], Block::Heading { text } if text == "Title"));
/// assert!(matches!(&read[2], Block::Fence { lang, .. } if lang.as_deref() == Some("sh")));
/// ```
pub fn blocks(text: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let mut paragraph: Vec<String> = Vec::new();
    let mut fence: Option<(Option<String>, Vec<String>)> = None;

    for line in text.lines() {
        if let Some((lang, body)) = fence.as_mut() {
            if is_fence(line) {
                out.push(Block::Fence {
                    lang: lang.clone(),
                    body: body.join("\n"),
                });
                fence = None;
            } else {
                body.push(line.to_string());
            }
            continue;
        }
        if is_fence(line) {
            flush(&mut paragraph, &mut out);
            let lang = line.trim().trim_start_matches('`').trim().to_string();
            fence = Some((if lang.is_empty() { None } else { Some(lang) }, Vec::new()));
            continue;
        }
        if let Some(heading) = heading(line) {
            flush(&mut paragraph, &mut out);
            out.push(Block::Heading { text: heading });
            continue;
        }
        if line.trim().is_empty() {
            flush(&mut paragraph, &mut out);
            continue;
        }
        paragraph.push(line.trim_end().to_string());
    }

    // A fence nobody closed is still text somebody wrote, and dropping it
    // would lose the end of the file over a missing three characters.
    if let Some((lang, body)) = fence {
        out.push(Block::Fence {
            lang,
            body: body.join("\n"),
        });
    }
    flush(&mut paragraph, &mut out);
    out
}

/// Is this line a fence delimiter? Three backticks or more, and nothing
/// but the language after them.
fn is_fence(line: &str) -> bool {
    line.trim_start().starts_with("```")
}

/// The text of an ATX heading, when the line is one.
fn heading(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let hashes = trimmed.chars().take_while(|c| *c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = trimmed[hashes..].trim();
    // `#hashtag` is not a heading — a heading's hashes are followed by a
    // space, which is what tells the two apart in Markdown as well.
    if !trimmed[hashes..].starts_with(char::is_whitespace) || rest.is_empty() {
        return None;
    }
    Some(rest.trim_end_matches('#').trim().to_string())
}

fn flush(paragraph: &mut Vec<String>, out: &mut Vec<Block>) {
    if paragraph.is_empty() {
        return;
    }
    out.push(Block::Paragraph {
        text: paragraph.join(" "),
    });
    paragraph.clear();
}

#[cfg(test)]
mod tests;
