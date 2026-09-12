//! `to_markdown` — the Markdown backend (PROP-045 ##TARGET-MD).
//!
//! Sections emit as ATX headings at their nesting depth with `{#id}`
//! anchors; facts as `@fact:<ID> … @status:<stage>/<state>` units (the
//! typed spelling `@fact/code:<ID>` when the next block is the bound
//! fence); fences, tables, quotes and lists in their MD forms. The output
//! re-parses through `from_markdown` into the SAME IR — that round-trip
//! law is the corpus test; byte-identity with the original source is NOT
//! claimed (that is the measured degradation, e.g. tilde fences re-spell
//! as backticks, `N)` ordinals as `N.`).
//!
//! The fence emitter is run-aware: a fence whose text contains a
//! three-backtick line is re-emitted with a longer opening run — the same
//! run-matching law progress-core's scanner reads.
//!
//! For the DOCUMENTATION genre the projection is ONE WAY by law, not by
//! defect (##DOC-VOCAB-MD-ONE-WAY): `example` becomes adjacent fences,
//! `rule` a quoted address, `derived` a marked fence, `note` a labelled
//! quote, `figure` an image with its caption, `prompt` a `prompt` fence
//! with its needs, outcome and asserts, and a slot's `when` a sub-heading
//! naming the condition. Re-parsing that projection yields a DIFFERENT IR
//! — fences, quotes and paragraphs, never the genre's blocks — and the
//! expectation is pinned by a test rather than discovered. The projection
//! exists because the host scanners read XML only through it
//! (##PROJECTION-READ), so documentation stays observed; authoring is XML
//! only.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-045#materialisation");

use crate::doc::{Block, BlockNode, Cond, Fact, Section, SpecDoc, StatusEl, Unit};

/// Emit a document as house-style Markdown.
pub fn to_markdown(doc: &SpecDoc) -> String {
    let mut out = String::new();
    if let Some(t) = &doc.title {
        out.push_str("# ");
        out.push_str(&t.text);
        if let Some(id) = &t.id {
            out.push_str(&format!(" {{#{id}}}"));
        }
        out.push_str("\n\n");
    }
    if let Some(s) = &doc.status {
        out.push_str(&status_element_md(s));
        out.push_str("\n\n");
    }
    blocks_md(&doc.preamble, 1, &mut out);
    for s in &doc.sections {
        section_md(s, 2, &mut out);
    }
    out
}

fn section_md(s: &Section, level: usize, out: &mut String) {
    for _ in 0..level {
        out.push('#');
    }
    out.push(' ');
    out.push_str(&s.title);
    // A guarded section's own heading is the sub-heading that names the
    // condition (##ROW-DOCVOCAB-WHEN): the platform rides in the heading
    // text, where a reader and a scanner both see it.
    if let Some(c) = &s.when {
        out.push_str(&format!(" ({c})"));
    }
    if let Some(id) = &s.id {
        out.push_str(&format!(" {{#{id}}}"));
    }
    out.push_str("\n\n");
    if let Some(st) = &s.status {
        out.push_str(&status_element_md(st));
        out.push_str("\n\n");
    }
    blocks_md(&s.blocks, level, out);
    for sub in &s.sections {
        section_md(sub, level + 1, out);
    }
}

/// Blocks separated by one blank line, the section's body closed by one
/// blank line. `level` is the enclosing heading level, which a guarded
/// slot needs for its sub-heading.
fn blocks_md(blocks: &[BlockNode], level: usize, out: &mut String) {
    for (i, node) in blocks.iter().enumerate() {
        if i > 0 {
            out.push_str("\n\n");
        }
        // A typed fact spells `@fact/code:` — the binding lives on the
        // NEXT block, so look ahead before emitting this one.
        let typed_id = match blocks.get(i + 1).map(|n| &n.block) {
            Some(Block::Fence { fact: Some(id), .. }) => Some(id.as_str()),
            _ => None,
        };
        if let Some(c) = &node.when {
            out.push_str(&when_heading_md(c, level));
        }
        block_md(&node.block, typed_id, out);
    }
    if !blocks.is_empty() {
        out.push_str("\n\n");
    }
}

/// A guarded slot's sub-heading, one level under the enclosing section and
/// never past H6 (##ROW-DOCVOCAB-WHEN: «a sub-heading named after the
/// platform»).
fn when_heading_md(c: &Cond, level: usize) -> String {
    let hashes = "#".repeat((level + 1).min(6));
    format!("{hashes} {c}\n\n")
}

fn block_md(b: &Block, typed_id: Option<&str>, out: &mut String) {
    match b {
        Block::Paragraph(u) => unit_md(u, typed_id, out),
        Block::Quote(u) => {
            // `> ` before every line; a bare `>` for empty lines.
            let mut inner = String::new();
            unit_md(u, typed_id, &mut inner);
            for (i, line) in inner.lines().enumerate() {
                if i > 0 {
                    out.push('\n');
                }
                if line.is_empty() {
                    out.push('>');
                } else {
                    out.push_str("> ");
                    out.push_str(line);
                }
            }
        }
        Block::Fence { lang, text, .. } => fenced_md(lang.as_deref(), text, out),
        Block::List { ordered, items } => {
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push('\n');
                }
                let marker = if *ordered {
                    format!("{}. ", i + 1)
                } else {
                    "- ".to_string()
                };
                out.push_str(&marker);
                // A GFM task box rides before the fact anchor.
                let (box_prefix, rest) = split_task_box(&item.text);
                out.push_str(box_prefix);
                let mut inner = String::new();
                emit_unit_body(item, typed_id, rest, &mut inner);
                out.push_str(&inner);
            }
        }
        Block::Table { rows } => {
            for (ri, row) in rows.iter().enumerate() {
                if ri > 0 {
                    out.push('\n');
                }
                if ri == 1 {
                    // The delimiter row after the header — plain `---`
                    // cells (alignment is not modelled).
                    let width = rows.first().map(|r| r.len()).unwrap_or(1);
                    out.push_str("| ");
                    for _ in 0..width.saturating_sub(1) {
                        out.push_str("--- | ");
                    }
                    out.push_str("--- |\n");
                }
                out.push('|');
                for cell in row {
                    out.push(' ');
                    let mut inner = String::new();
                    unit_md(cell, None, &mut inner);
                    out.push_str(&inner);
                    out.push_str(" |");
                }
            }
        }
        // --- the documentation genre (PROP-045 §7) ----------------------
        Block::Example {
            lang,
            run,
            expect,
            stderr,
            ..
        } => {
            // Two adjacent fences, `sh` and `output`, and a third for
            // stderr when the page spells one (##ROW-DOCVOCAB-EXAMPLE-MD).
            fenced_md(Some(lang.as_deref().unwrap_or("sh")), run, out);
            out.push_str("\n\n");
            fenced_md(Some("output"), expect, out);
            if let Some(err) = stderr {
                out.push_str("\n\n");
                fenced_md(Some("stderr"), err, out);
            }
        }
        Block::ExampleRef { id } => {
            // The pivot holds no source page, so the projection names the
            // example it defers to; the pipeline copies the source's own
            // fences when it has the source in hand
            // (##ROW-DOCVOCAB-EXAMPLE-REF-MD).
            out.push_str(&format!(
                "Example `{id}` is copied from the source page at projection time."
            ));
        }
        Block::Rule { uri, .. } => {
            // A quote carrying the address as a link, and the address is
            // the unpinned one — a citation is live
            // (##ROW-DOCVOCAB-RULE-MD, ##DOC-VOCAB-RULE-ADDRESS).
            out.push_str(&format!("> <{uri}>"));
        }
        Block::Derived { kind, reference } => {
            // The text is not stored, so what projects is the provenance
            // line, marked «generated from …» (##ROW-DOCVOCAB-DERIVED-MD).
            fenced_md(
                Some("text"),
                &format!("generated from {kind}: {reference}"),
                out,
            );
        }
        Block::Note { kind, body } => {
            // A quote with the kind label on its first line
            // (##ROW-DOCVOCAB-NOTE-MD).
            let mut inner = String::new();
            unit_md(body, typed_id, &mut inner);
            out.push_str(&format!("> **{}**", kind.label()));
            for line in inner.lines() {
                out.push('\n');
                if line.is_empty() {
                    out.push('>');
                } else {
                    out.push_str("> ");
                    out.push_str(line);
                }
            }
        }
        Block::Figure { src, alt, caption } => {
            // An inline image plus a caption paragraph — two Markdown
            // blocks out of one IR block (##ROW-DOCVOCAB-FIGURE-MD).
            out.push_str(&format!("![{alt}]({src})\n\n"));
            unit_md(caption, typed_id, out);
        }
        Block::Prompt {
            text,
            needs,
            outcome,
            asserts,
            ..
        } => {
            // A `prompt` fence, a «needs» list, an «outcome» paragraph and
            // an assert list (##ROW-DOCVOCAB-PROMPT-MD). The fence is the
            // same one `llms*.txt` carries, which is why the body projects
            // verbatim.
            fenced_md(Some("prompt"), text, out);
            if let Some(n) = needs {
                out.push_str(&format!("\n\n- needs: {n}"));
            }
            if let Some(o) = outcome {
                out.push_str(&format!("\n\noutcome: {o}"));
            }
            if asserts.is_empty() {
                out.push_str("\n\n- assert: none");
            } else {
                out.push('\n');
                for a in asserts {
                    out.push_str(&format!("\n- assert: `{a}`"));
                }
            }
        }
    }
}

/// One fenced block with a run long enough to quote its own content.
fn fenced_md(lang: Option<&str>, text: &str, out: &mut String) {
    let run = fence_run_for(text);
    out.push_str(&run);
    if let Some(l) = lang {
        out.push_str(l);
    }
    out.push('\n');
    if !text.is_empty() {
        out.push_str(text);
        out.push('\n');
    }
    out.push_str(&run);
}

/// One unit as MD: `[@fact[/code]:ID ]text[ status]` joined with single
/// spaces, so an empty body still reads `@fact:X @status:s/s`.
fn unit_md(u: &Unit, typed_id: Option<&str>, out: &mut String) {
    emit_unit_body(u, typed_id, &u.text, out);
}

fn emit_unit_body(u: &Unit, typed_id: Option<&str>, text: &str, out: &mut String) {
    let mut parts: Vec<String> = Vec::new();
    if let Some(f) = u.fact.as_ref().filter(|f| f.is_meaningful()) {
        parts.push(fact_prefix_md(f, typed_id));
    }
    if !text.is_empty() {
        parts.push(text.to_string());
    }
    if let Some(requirements) = u.fact.as_ref().and_then(|f| f.requirements.as_ref()) {
        parts.push(format!("@requires:{}", requirements.to_csv()));
    }
    if let Some(st) = u.fact.as_ref().and_then(|f| f.status.as_ref()) {
        parts.push(status_suffix_md(st));
    }
    out.push_str(&parts.join(" "));
}

fn fact_prefix_md(f: &Fact, typed_id: Option<&str>) -> String {
    let typed = f.id.as_deref().is_some_and(|id| Some(id) == typed_id);
    match &f.id {
        Some(id) if typed => format!("@fact/code:{id}"),
        Some(id) => format!("@fact:{id}"),
        None => String::new(),
    }
}

/// The status suffix: the qualified shorthand when the payload is just
/// stage/state, the point element when it carries more.
fn status_suffix_md(st: &StatusEl) -> String {
    if st.action.is_none()
        && st.actionstage.is_none()
        && st.audience.is_empty()
        && st.comment.is_none()
        && st.r#ref.is_none()
    {
        format!("@status:{}/{}", st.stage, st.state)
    } else {
        status_element_md(st)
    }
}

/// The `<status …/>` point element, MD spelling (the attribute order
/// matches the XML backend's canonical order).
fn status_element_md(s: &StatusEl) -> String {
    let mut out = format!("<status stage=\"{}\" state=\"{}\"", s.stage, s.state);
    if let Some(a) = s.action {
        out.push_str(&format!(" action=\"{a}\""));
    }
    if let Some(a) = s.actionstage {
        out.push_str(&format!(" actionstage=\"{a}\""));
    }
    if !s.audience.is_empty() {
        let joined = s
            .audience
            .iter()
            .map(|a| a.to_string())
            .collect::<Vec<_>>()
            .join(",");
        out.push_str(&format!(" audience=\"{joined}\""));
    }
    if let Some(c) = &s.comment {
        out.push_str(&format!(" comment=\"{c}\""));
    }
    if let Some(r) = &s.r#ref {
        out.push_str(&format!(" ref=\"{r}\""));
    }
    out.push_str("/>");
    out
}

/// The fence run that safely wraps `text`: at least three backticks, one
/// more than the longest run inside it (run-matching, the scanner's own
/// law — a quoted fence stays quoted).
fn fence_run_for(text: &str) -> String {
    let longest = text
        .lines()
        .map(|l| {
            let t = l.trim_start();
            let run = t.chars().take_while(|&c| c == '`').count();
            let mut best = run;
            // A backtick run anywhere in the line could open a closer if
            // it starts a line after emission; measure all runs.
            let mut i = 0usize;
            let b = t.as_bytes();
            while i < b.len() {
                if b[i] == b'`' {
                    let start = i;
                    while i < b.len() && b[i] == b'`' {
                        i += 1;
                    }
                    best = best.max(i - start);
                } else {
                    i += 1;
                }
            }
            best
        })
        .max()
        .unwrap_or(0);
    let n = 3.max(longest + 1);
    "`".repeat(n)
}

/// Split a leading GFM task box (`[ ] `, `[x] `, `[X] `) off an item's
/// text; the box re-attaches before the fact anchor on emission.
fn split_task_box(text: &str) -> (&str, &str) {
    let b = text.as_bytes();
    if b.len() >= 3 && b[0] == b'[' && b[2] == b']' && matches!(b[1], b' ' | b'x' | b'X') {
        let mut i = 3;
        while i < b.len() && (b[i] == b' ' || b[i] == b'\t') {
            i += 1;
        }
        if i > 3 || b.len() == 3 {
            // `[ ]glued` is prose, not a box — require the spacing or the
            // end of the text, exactly like the scanner's box grammar.
            if i > 3 || b.len() == 3 {
                return (&text[..i], &text[i..]);
            }
        }
    }
    ("", text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{from_markdown, from_xml, to_xml};

    /// The pivot law, stated as a property over a shape-heavy sample:
    /// whatever came from MD re-parses to the same IR after the MD → XML →
    /// MD walk.
    #[test]
    fn emitted_markdown_re_parses_to_the_same_ir() {
        let md = "# T {#t}\n\n<status stage=\"spec\" state=\"done\" audience=\"user\"/>\n\n\
                  @fact:A Lead claim. @status:impl/done\n\n\
                  - item one\n- @fact:B item two @spec/work\n\n\
                  | H1 | H2 |\n|---|---|\n| a |  |\n\n\
                  > quoted words\n\n\
                  ```rust\nlet x = 1;\n```\n";
        let ir1 = from_markdown(md).expect("parses");
        let out1 = to_markdown(&ir1);
        let ir2 = from_markdown(&out1).expect("re-parses");
        assert_eq!(ir1, ir2, "md out:\n{out1}");
        // and through XML the same law holds
        let xml = to_xml(&ir1);
        let ir3 = from_xml(&xml).unwrap();
        assert_eq!(ir1, ir3);
        assert_eq!(to_xml(&ir2), xml);
    }

    #[test]
    fn typed_fact_emits_the_code_spelling_and_the_fence() {
        let ir = from_markdown(
            "# T {#t}\n\n@fact/code:RUN run this @impl/done\n\n```bash\ncargo test\n```\n",
        )
        .unwrap();
        let md = to_markdown(&ir);
        assert!(
            md.contains("@fact/code:RUN run this @status:impl/done"),
            "{md}"
        );
        assert!(md.contains("```bash\ncargo test\n```"), "{md}");
        assert_eq!(from_markdown(&md).unwrap(), ir, "{md}");
    }

    /// The corner the packet names: a fence whose text contains a
    /// three-backtick fence line — the emitter must widen its own run.
    #[test]
    fn a_fence_quoting_a_fence_widens_its_run() {
        let ir = from_markdown("# T {#t}\n\n````\nouter\n```\n````\n").unwrap();
        match &ir.preamble[0].block {
            Block::Fence { text, .. } => assert_eq!(text, "outer\n```"),
            other => panic!("{other:?}"),
        }
        let md = to_markdown(&ir);
        assert!(md.contains("````\nouter\n```\n````"), "{md}");
        assert_eq!(from_markdown(&md).unwrap(), ir, "{md}");
    }

    #[test]
    fn ordered_list_numbers_from_one() {
        let ir = from_markdown("# T {#t}\n\n1) one\n2) two\n").unwrap();
        let md = to_markdown(&ir);
        assert!(md.contains("1. one\n2. two"), "{md}");
        assert_eq!(from_markdown(&md).unwrap(), ir);
    }

    #[test]
    fn empty_section_emits_heading_only() {
        let ir = from_markdown("# T {#t}\n\n## Empty {#e}\n\n## After {#a}\n\ntext\n").unwrap();
        let md = to_markdown(&ir);
        assert!(md.contains("## Empty {#e}\n\n## After {#a}"), "{md}");
        assert_eq!(from_markdown(&md).unwrap(), ir);
    }

    #[test]
    fn tilde_fence_re_spells_as_backticks() {
        let ir = from_markdown("# T {#t}\n\n~~~\ncode with ``` inside\n~~~\n").unwrap();
        let md = to_markdown(&ir);
        assert!(md.contains("```\ncode with ``` inside\n```"), "{md}");
        assert_eq!(from_markdown(&md).unwrap(), ir, "{md}");
    }
}
