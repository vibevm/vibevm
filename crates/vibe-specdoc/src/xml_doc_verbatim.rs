//! The two documentation blocks whose bodies are VERBATIM: `<example>`
//! and `<prompt>` (PROP-045 §7).
//!
//! Their children — `run`, `expect`, `stderr`, `assert`, and a `prompt`
//! body — behave exactly like `Block::Fence`'s text: multi-line, no
//! inline grammar, CDATA admitted (##DOC-VOCAB-VERBATIM-TEXTS) and NOT
//! trimmed, so a golden output that opens with a blank line keeps it.
//! That is the property that puts these two in one cell: everything else
//! about them differs, and this does not.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-045#shape");

use super::xml_in::{Ev, Parser};
use super::xml_support::{only_attrs, only_attrs_slot};
use crate::doc::Block;
use crate::{Error, Result};

impl<'a> Parser<'a> {
    /// `<example>`: the command and its golden output, or — in a
    /// translation — a reference to the source's example
    /// (##ROW-DOCVOCAB-EXAMPLE, ##ROW-DOCVOCAB-EXAMPLE-REF).
    pub(super) fn example(
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

    /// `<prompt id>`: the agent's task, what it needs, what the person
    /// sees, and the asserts (##ROW-DOCVOCAB-PROMPT).
    pub(super) fn prompt(
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
}
