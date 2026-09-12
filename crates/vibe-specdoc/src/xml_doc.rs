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

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-045#shape");

use super::xml_in::{Ev, Parser};
use super::xml_support::{kind, only_attrs, only_attrs_slot};
use crate::doc::{Block, DerivedKind, NoteKind, Unit};
use crate::{Error, Result};
use progress_core::model::nearest;

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

    /// `<example>`: the command and its golden output, or — in a
    /// translation — a reference to the source's example
    /// (##ROW-DOCVOCAB-EXAMPLE, ##ROW-DOCVOCAB-EXAMPLE-REF).
    fn example(
        &mut self,
        attrs: &[(String, String)],
        at: (usize, usize),
        was_empty: bool,
    ) -> Result<Block> {
        only_attrs_slot(
            attrs,
            &["id", "fixture", "lang", "exit", "ref"],
            "example",
            at,
            self,
        )?;
        if let Some(id) = self.attr(attrs, "ref") {
            // The reference form owns the element: it has no example of
            // its own, so it can carry neither the attributes of one nor a
            // body.
            for other in ["id", "fixture", "lang", "exit"] {
                if self.attr(attrs, other).is_some() {
                    return Err(self.err(
                        at,
                        format!(
                            "<example ref=\"…\"> is a reference to the source's example and \
                             carries no `{other}` of its own"
                        ),
                    ));
                }
            }
            if !was_empty {
                self.expect_end("example", at)?;
            }
            if id.trim().is_empty() {
                return Err(self.err(at, "an <example ref=\"…\"> needs a non-empty id".into()));
            }
            return Ok(Block::ExampleRef { id });
        }
        let Some(id) = self.attr(attrs, "id") else {
            return Err(self.err(
                at,
                "an <example> needs an `id` — a translation cites it, and the runner reports by it"
                    .into(),
            ));
        };
        let Some(fixture) = self.attr(attrs, "fixture") else {
            return Err(self.err(
                at,
                "an <example> needs a `fixture` — the command runs in a hermetic fixture \
                 project, never in the source tree"
                    .into(),
            ));
        };
        self.mint_fact(id.clone(), at)?;
        let lang = self.attr(attrs, "lang");
        let exit = match self.attr(attrs, "exit") {
            None => None,
            Some(v) => Some(v.parse::<i32>().map_err(|_| {
                self.err(
                    at,
                    format!("the `exit` attribute is a process exit code, found `{v}`"),
                )
            })?),
        };
        if was_empty {
            return Err(self.err(
                at,
                "an <example> needs a <run> and an <expect> — an example without its expected \
                 output cannot be checked"
                    .into(),
            ));
        }
        let mut run: Option<String> = None;
        let mut expect: Option<String> = None;
        let mut stderr: Option<String> = None;
        loop {
            // The layout whitespace BETWEEN the children is the writer's,
            // not content; the content of each child is verbatim and is
            // read by `verbatim_text` once the child is open.
            self.skip_ws_text()?;
            match self.evs.get(self.i) {
                None => {
                    return Err(Error::at(
                        0,
                        "unexpected end of input — <example> never closed",
                    ));
                }
                Some(Ev::End(n)) if n == "example" => {
                    self.i += 1;
                    break;
                }
                Some(_) => {}
            }
            let (name, child_attrs, child_at, child_empty) = self.take_start()?;
            if !matches!(name.as_str(), "run" | "expect" | "stderr") {
                return Err(self.err(
                    child_at,
                    format!(
                        "the dialect has no <{name}> element (inside <example>) — only <run>, \
                         <expect> and <stderr>"
                    ),
                ));
            }
            only_attrs(&child_attrs, &[], &name, child_at, self)?;
            let text = self.verbatim_text(&name, child_empty)?;
            match name.as_str() {
                "run" if run.is_none() => run = Some(text),
                "expect" if run.is_some() && expect.is_none() => expect = Some(text),
                "stderr" if expect.is_some() && stderr.is_none() => stderr = Some(text),
                _ => {
                    return Err(self.err(
                        child_at,
                        format!(
                            "<{name}> is out of order or repeated — an <example> holds one <run>, \
                             then one <expect>, then at most one <stderr>"
                        ),
                    ));
                }
            }
        }
        let Some(run) = run else {
            return Err(self.err(at, "an <example> needs a <run> command".into()));
        };
        if run.trim().is_empty() {
            return Err(self.err(at, "an <example>'s <run> command is empty".into()));
        }
        let Some(expect) = expect else {
            return Err(self.err(
                at,
                "an <example> needs an <expect> — write <expect></expect> to assert that the \
                 command prints nothing"
                    .into(),
            ));
        };
        Ok(Block::Example {
            id,
            fixture,
            lang,
            exit,
            run,
            expect,
            stderr,
        })
    }

    /// `<rule ref="spec://…#ANCHOR"/>`: the insertion point of a rule
    /// whose text is substituted at build (##ROW-DOCVOCAB-RULE). The
    /// address's FORM is checked here; resolving the anchor is the
    /// pipeline's job, and pinning it is nobody's
    /// (##DOC-VOCAB-RULE-ADDRESS).
    fn rule(
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
    fn derived(
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

    /// `<note kind="…">`: a call-out whose body is one unit, and so takes
    /// a fact anchor like any other unit (##ROW-DOCVOCAB-NOTE).
    fn note(
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
    fn figure(
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

    /// `<prompt id>`: the agent's task, what it needs, what the person
    /// sees, and the asserts (##ROW-DOCVOCAB-PROMPT).
    fn prompt(
        &mut self,
        attrs: &[(String, String)],
        at: (usize, usize),
        was_empty: bool,
    ) -> Result<Block> {
        only_attrs_slot(attrs, &["id", "assert"], "prompt", at, self)?;
        let Some(id) = self.attr(attrs, "id") else {
            return Err(self.err(
                at,
                "a <prompt> needs an `id` — the runner and the skill address it by that".into(),
            ));
        };
        // `assert="none"` is the ONE spelling of a deliberately
        // unassertable prompt (PROP-057 ##STYLE-PROMPT-FIRST); it is not a
        // way to write asserts.
        let declared_none = match self.attr(attrs, "assert").as_deref() {
            None => false,
            Some("none") => true,
            Some(other) => {
                return Err(self.err(
                    at,
                    format!(
                        "the `assert` attribute of a <prompt> is only ever \"none\", for an \
                         illustrative prompt, found `{other}` — a real check is an <assert> child"
                    ),
                ));
            }
        };
        self.mint_fact(id.clone(), at)?;
        if was_empty {
            return Err(self.err(at, "an empty <prompt> asks the agent nothing".into()));
        }
        let mut text = String::new();
        let mut needs: Option<String> = None;
        let mut outcome: Option<String> = None;
        let mut asserts: Vec<String> = Vec::new();
        let mut body_closed = false;
        loop {
            match self.evs.get(self.i) {
                None => {
                    return Err(Error::at(
                        0,
                        "unexpected end of input — <prompt> never closed",
                    ));
                }
                Some(Ev::End(n)) if n == "prompt" => {
                    self.i += 1;
                    break;
                }
                Some(Ev::Text(t)) | Some(Ev::CData(t)) => {
                    if body_closed {
                        if !t.trim().is_empty() {
                            let child_at = self.poss[self.i];
                            return Err(self.err(
                                child_at,
                                "the body of a <prompt> comes before its <needs>, <outcome> and \
                                 <assert> children"
                                    .into(),
                            ));
                        }
                        self.i += 1;
                        continue;
                    }
                    text.push_str(t);
                    self.i += 1;
                }
                Some(_) => {
                    body_closed = true;
                    let (name, child_attrs, child_at, child_empty) = self.take_start()?;
                    only_attrs(&child_attrs, &[], &name, child_at, self)?;
                    match name.as_str() {
                        "needs" if needs.is_none() && outcome.is_none() && asserts.is_empty() => {
                            needs = Some(self.leaf_text("needs", child_empty)?);
                        }
                        "outcome" if outcome.is_none() && asserts.is_empty() => {
                            outcome = Some(self.leaf_text("outcome", child_empty)?);
                        }
                        "assert" => {
                            if declared_none {
                                return Err(self.err(
                                    child_at,
                                    "a <prompt assert=\"none\"> declares that it has no check — \
                                     drop the attribute or drop the <assert>"
                                        .into(),
                                ));
                            }
                            asserts.push(self.verbatim_text("assert", child_empty)?);
                        }
                        "needs" | "outcome" => {
                            return Err(self.err(
                                child_at,
                                format!(
                                    "<{name}> is out of order or repeated — a <prompt> holds its \
                                     body, then one <needs>, then one <outcome>, then its \
                                     <assert> commands"
                                ),
                            ));
                        }
                        other => {
                            return Err(self.err(
                                child_at,
                                format!(
                                    "the dialect has no <{other}> element (inside <prompt>) — \
                                     only <needs>, <outcome> and <assert>"
                                ),
                            ));
                        }
                    }
                }
            }
        }
        let text = text.trim().to_string();
        if text.is_empty() {
            return Err(self.err(at, "an empty <prompt> asks the agent nothing".into()));
        }
        if asserts.is_empty() && !declared_none {
            return Err(self.err(
                at,
                "a <prompt> needs at least one <assert> — a prompt cannot be checked by its \
                 output the way an example can; an illustrative prompt says so with \
                 assert=\"none\""
                    .into(),
            ));
        }
        if asserts.iter().any(|a| a.trim().is_empty()) {
            return Err(self.err(at, "an <assert> command is empty".into()));
        }
        Ok(Block::Prompt {
            id,
            text,
            needs,
            outcome,
            asserts,
        })
    }

    /// One attribute's value, cloned.
    fn attr(&self, attrs: &[(String, String)], key: &str) -> Option<String> {
        attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    }

    /// A verbatim child's content: text and CDATA, exactly as written
    /// (##DOC-VOCAB-VERBATIM-TEXTS). Nothing is trimmed — a golden output
    /// is bytes, and its leading blank line is part of it.
    fn verbatim_text(&mut self, tag: &str, was_empty: bool) -> Result<String> {
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
    fn expect_end(&mut self, tag: &str, at: (usize, usize)) -> Result<()> {
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
