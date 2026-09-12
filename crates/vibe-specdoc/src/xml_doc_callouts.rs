//! The two documentation blocks that are call-outs on the page:
//! `<note>`, whose body is one addressable unit, and `<figure>`, whose
//! caption is (PROP-045 §7).
//!
//! Both carry PROSE children: inline Markdown kept literally and trimmed,
//! because the whitespace around them is the markup's layout and not
//! content (##INLINE-STAYS-MARKDOWN). That is the property that puts them
//! in one cell.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-045#shape");

use super::xml_in::{Ev, Parser};
use super::xml_support::{only_attrs, only_attrs_slot};
use crate::doc::{Block, NoteKind, Unit};
use crate::{Error, Result};
use progress_core::model::nearest;

impl<'a> Parser<'a> {
    /// `<note kind="…">`: a call-out whose body is one unit, and so takes
    /// a fact anchor like any other unit (##ROW-DOCVOCAB-NOTE).
    pub(super) fn note(
        &mut self,
        attrs: &[(String, String)],
        at: (usize, usize),
        was_empty: bool,
    ) -> Result<Block> {
        only_attrs_slot(attrs, &["kind"], "note", at, self)?;
        let Some(kind_value) = self.attr(attrs, "kind") else {
            return Err(self.err(at, "a <note> needs a `kind` (note|tip|warning)".into()));
        };
        let Some(kind) = NoteKind::parse(&kind_value) else {
            let hint = nearest(&kind_value, NoteKind::ALL.iter().map(|k| k.as_str()));
            return Err(self.err(
                at,
                match hint {
                    Some(h) => format!("unknown note kind `{kind_value}` — did you mean `{h}`?"),
                    None => format!(
                        "unknown note kind `{kind_value}` — the vocabulary is note|tip|warning"
                    ),
                },
            ));
        };
        let body = self.unit("note", was_empty, false)?;
        Ok(Block::Note { kind, body })
    }

    /// `<figure src alt>` with a `<caption>` (##ROW-DOCVOCAB-FIGURE). The
    /// `alt` is required: an image nobody can read is not documentation.
    pub(super) fn figure(
        &mut self,
        attrs: &[(String, String)],
        at: (usize, usize),
        was_empty: bool,
    ) -> Result<Block> {
        only_attrs_slot(attrs, &["src", "alt"], "figure", at, self)?;
        let Some(src) = self.attr(attrs, "src") else {
            return Err(self.err(
                at,
                "a <figure> needs a `src` naming a file in the package tree".into(),
            ));
        };
        let Some(alt) = self.attr(attrs, "alt") else {
            return Err(self.err(
                at,
                "a <figure> needs an `alt` — the text a reader gets instead of the image".into(),
            ));
        };
        if src.trim().is_empty() || alt.trim().is_empty() {
            return Err(self.err(at, "a <figure>'s `src` and `alt` cannot be empty".into()));
        }
        if was_empty {
            return Err(self.err(at, "a <figure> needs a <caption>".into()));
        }
        let mut caption: Option<Unit> = None;
        loop {
            self.skip_ws_text()?;
            match self.evs.get(self.i) {
                None => {
                    return Err(Error::at(
                        0,
                        "unexpected end of input — <figure> never closed",
                    ));
                }
                Some(Ev::End(n)) if n == "figure" => {
                    self.i += 1;
                    break;
                }
                Some(_) => {}
            }
            let (name, child_attrs, child_at, child_empty) = self.take_start()?;
            if name != "caption" {
                return Err(self.err(
                    child_at,
                    format!(
                        "the dialect has no <{name}> element (inside <figure>) — only <caption>"
                    ),
                ));
            }
            if caption.is_some() {
                return Err(self.err(child_at, "a <figure> holds one <caption>".into()));
            }
            only_attrs(&child_attrs, &[], "caption", child_at, self)?;
            caption = Some(self.unit("caption", child_empty, false)?);
        }
        let Some(caption) = caption else {
            return Err(self.err(at, "a <figure> needs a <caption>".into()));
        };
        Ok(Block::Figure { src, alt, caption })
    }
}
