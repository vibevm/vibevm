//! The two documentation blocks that point OUTSIDE the document:
//! `<rule>`, the insertion point of a rule whose text is substituted at
//! build, and `<derived>`, a reference generated at build (PROP-045 §7).
//!
//! Neither stores what it points at — the generated text is not read,
//! because the generator is the record and the output is not. That is
//! what puts them in one cell, and it is also why the revision pin
//! belongs here: a pin is part of an address, and addresses are all this
//! cell holds.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-045#shape");

use super::xml_in::Parser;
use super::xml_support::only_attrs_slot;
use crate::Result;
use crate::doc::{Block, DerivedKind};
use progress_core::model::nearest;

impl<'a> Parser<'a> {
    /// `<rule ref="spec://…#ANCHOR"/>`: the insertion point of a rule
    /// whose text is substituted at build (##ROW-DOCVOCAB-RULE). The
    /// address's FORM is checked here; resolving the anchor is the
    /// pipeline's job, and pinning it is nobody's
    /// (##DOC-VOCAB-RULE-ADDRESS).
    pub(super) fn rule(
        &mut self,
        attrs: &[(String, String)],
        at: (usize, usize),
        was_empty: bool,
    ) -> Result<Block> {
        only_attrs_slot(attrs, &["ref"], "rule", at, self)?;
        let Some(address) = self.attr(attrs, "ref") else {
            return Err(self.err(
                at,
                "a <rule> needs a `ref` naming the rule it cites (spec://…#ANCHOR)".into(),
            ));
        };
        if !was_empty {
            self.expect_end("rule", at)?;
        }
        if !address.starts_with("spec://") {
            return Err(self.err(
                at,
                format!(
                    "a <rule> cites a spec address, found `{address}` — the form is \
                     spec://<group>/<name>/<path>#<ANCHOR>"
                ),
            ));
        }
        let Some((path, fragment)) = address.split_once('#') else {
            return Err(self.err(
                at,
                format!("the <rule> address `{address}` names no anchor — add `#<ANCHOR>`"),
            ));
        };
        let (anchor, rev) = split_revision(fragment);
        let rev = match rev {
            None => None,
            Some(digits) => match digits.parse::<u32>() {
                Ok(n) if n >= 1 => Some(n),
                _ => {
                    return Err(self.err(
                        at,
                        format!(
                            "invalid revision pin `~r{digits}` in `{address}` (expected N ≥ 1)"
                        ),
                    ));
                }
            },
        };
        if anchor.is_empty() {
            return Err(self.err(
                at,
                format!("the <rule> address `{address}` names no anchor — add `#<ANCHOR>`"),
            ));
        }
        Ok(Block::Rule {
            uri: format!("{path}#{anchor}"),
            rev,
        })
    }

    /// `<derived kind="…" ref="…"/>`: a reference generated at build
    /// (##ROW-DOCVOCAB-DERIVED). The generated text is not read, because
    /// it is not stored.
    pub(super) fn derived(
        &mut self,
        attrs: &[(String, String)],
        at: (usize, usize),
        was_empty: bool,
    ) -> Result<Block> {
        only_attrs_slot(attrs, &["kind", "ref"], "derived", at, self)?;
        let Some(kind_value) = self.attr(attrs, "kind") else {
            return Err(self.err(
                at,
                "a <derived> needs a `kind` (cli-help|jtd-schema|manifest-field)".into(),
            ));
        };
        let Some(kind) = DerivedKind::parse(&kind_value) else {
            let hint = nearest(&kind_value, DerivedKind::ALL.iter().map(|k| k.as_str()));
            return Err(self.err(
                at,
                match hint {
                    Some(h) => format!("unknown derived kind `{kind_value}` — did you mean `{h}`?"),
                    None => format!(
                        "unknown derived kind `{kind_value}` — the vocabulary is \
                         cli-help|jtd-schema|manifest-field"
                    ),
                },
            ));
        };
        let Some(reference) = self.attr(attrs, "ref") else {
            return Err(self.err(
                at,
                format!("a <derived kind=\"{kind}\"> needs a `ref` naming what to generate"),
            ));
        };
        if !was_empty {
            self.expect_end("derived", at)?;
        }
        if reference.trim().is_empty() {
            return Err(self.err(at, "a <derived> `ref` is empty".into()));
        }
        Ok(Block::Derived { kind, reference })
    }
}

/// Split a `~rN` revision pin off an address fragment
/// (##DOC-VOCAB-RULE-ADDRESS): the pin is recorded so the author's bytes
/// survive, and never becomes a pin on the citation.
fn split_revision(fragment: &str) -> (&str, Option<&str>) {
    match fragment.rsplit_once("~r") {
        Some((anchor, digits))
            if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) =>
        {
            (anchor, Some(digits))
        }
        _ => (fragment, None),
    }
}
