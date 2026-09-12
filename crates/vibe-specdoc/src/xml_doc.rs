//! The documentation genre's seven block readers (PROP-045 §7).
//!
//! Each element here is one the spec dialect deliberately cannot express —
//! a verifiable example, a live rule citation, a generated reference, a
//! call-out, a figure, an agent prompt — and each is read only when the
//! caller holds [`Vocabulary::Doc`]; the gate lives at the dispatch sites
//! and in `block`, so nothing in this module has to re-decide it.
//!
//! Two shapes recur. VERBATIM children (`run`, `expect`, `stderr`,
//! `assert`, and a `prompt` body) behave exactly like `Block::Fence`'s
//! text: multi-line, no inline grammar, CDATA admitted
//! (##DOC-VOCAB-VERBATIM-TEXTS) and NOT trimmed, so a golden output that
//! opens with a blank line keeps it. PROSE children (`needs`, `outcome`,
//! and the units of `note` and `figure`) keep inline Markdown literally
//! and are trimmed, because their surrounding whitespace is the markup's
//! layout and not content (##INLINE-STAYS-MARKDOWN).
//!
//! Child ORDER is fixed, and a wrong order is a loud error rather than a
//! silent reordering: the writer emits one order, so accepting another
//! would break the byte-idempotence law the moment such a file existed.
//!
//! This cell holds the dispatch and the three child helpers every reader
//! shares. The readers themselves are grouped by the shape of what they
//! read, one cell each: [`crate::xml_doc_verbatim`] for `example` and
//! `prompt`, whose bodies are verbatim; [`crate::xml_doc_citations`] for
//! `rule` and `derived`, which point outside the document;
//! [`crate::xml_doc_callouts`] for `note` and `figure`, whose children
//! are prose.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-045#shape");

use super::xml_in::{Ev, Parser};
use super::xml_support::kind;
use crate::doc::Block;
use crate::{Error, Result};

impl<'a> Parser<'a> {
    /// One documentation block, dispatched by element name. The caller has
    /// already established that the genre is open and that this opening
    /// carries no `title=` (##DOC-VOCAB-DISCRIMINATOR).
    pub(super) fn doc_block(
        &mut self,
        name: &str,
        attrs: &[(String, String)],
        at: (usize, usize),
        was_empty: bool,
    ) -> Result<Block> {
        match name {
            "example" => self.example(attrs, at, was_empty),
            "rule" => self.rule(attrs, at, was_empty),
            "derived" => self.derived(attrs, at, was_empty),
            "note" => self.note(attrs, at, was_empty),
            "figure" => self.figure(attrs, at, was_empty),
            "prompt" => self.prompt(attrs, at, was_empty),
            other => Err(self.err(
                at,
                format!(
                    "the documentation vocabulary has no <{other}> element — the vocabulary is closed"
                ),
            )),
        }
    }

    /// One attribute's value, cloned.
    pub(super) fn attr(&self, attrs: &[(String, String)], key: &str) -> Option<String> {
        attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    }

    /// A verbatim child's content: text and CDATA, exactly as written
    /// (##DOC-VOCAB-VERBATIM-TEXTS). Nothing is trimmed — a golden output
    /// is bytes, and its leading blank line is part of it.
    pub(super) fn verbatim_text(&mut self, tag: &str, was_empty: bool) -> Result<String> {
        let mut text = String::new();
        if was_empty {
            return Ok(text);
        }
        loop {
            match self.evs.get(self.i) {
                None => {
                    return Err(Error::at(
                        0,
                        format!("unexpected end of input — <{tag}> never closed"),
                    ));
                }
                Some(Ev::End(n)) if n == tag => {
                    self.i += 1;
                    break;
                }
                Some(Ev::Text(t)) | Some(Ev::CData(t)) => {
                    text.push_str(t);
                    self.i += 1;
                }
                Some(other) => {
                    let at = self.poss[self.i];
                    return Err(self.err(
                        at,
                        format!("a <{tag}> holds only verbatim text — found {}", kind(other)),
                    ));
                }
            }
        }
        Ok(text)
    }

    /// Consume the matching end tag of an element the dialect spells empty
    /// (`<rule …/>`); a body there is a loud error, not ignored content.
    pub(super) fn expect_end(&mut self, tag: &str, at: (usize, usize)) -> Result<()> {
        // Whitespace between the two tags is the markup's own layout; the
        // refusal below is for real content, and says what it found.
        while let Some(Ev::Text(t)) = self.evs.get(self.i) {
            if !t.trim().is_empty() {
                break;
            }
            self.i += 1;
        }
        match self.evs.get(self.i) {
            Some(Ev::End(n)) if n == tag => {
                self.i += 1;
                Ok(())
            }
            Some(other) => {
                let child_at = self.poss[self.i];
                Err(self.err(
                    child_at,
                    format!("a <{tag}> element is empty — found {}", kind(other)),
                ))
            }
            None => Err(Error::at(
                at.0,
                format!("unexpected end of input — <{tag}> never closed"),
            )),
        }
    }
}
