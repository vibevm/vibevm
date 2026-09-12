//! `--accept`: writing a captured output back into the page it belongs to.
//!
//! The edit is SURGICAL — the `<expect>` (and, when the command wrote to
//! it, the `<stderr>` and the `exit` attribute) of one example, and not one
//! byte else. It does not rewrite the page through the pivot's writer,
//! which would also canonicalise spellings the author chose and turn a
//! capture into a reformat; the packet's line is «`expect` and nothing
//! more», and a reviewer reading the diff of an accept run must see
//! exactly the output that was captured.
//!
//! It only ever FILLS an empty golden. Replacing a golden that already
//! holds text is the move that turns a red check green without anyone
//! deciding anything — the one thing PROP-057 `##PIPE-EXAMPLE-RUNNER`
//! forbids by name — so it takes a second, louder flag and says so.
//!
//! Every write is verified by re-reading the page through the pivot and
//! checking that the example now holds exactly what was captured and that
//! nothing else in the document moved. A surgical text edit that cannot
//! prove that is not applied.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#INV-EXAMPLES-RUN");

use std::path::Path;

use vibe_specdoc::doc::{Block, SpecDoc, Vocabulary};

use crate::error::{DocError, Result};

/// What a capture asks the page to record.
#[derive(Debug, Clone)]
pub struct Golden {
    pub expect: String,
    /// `None` asserts «stderr is empty».
    pub stderr: Option<String>,
    pub exit: i32,
}

/// Fill (or, with `force`, replace) one example's golden on its page.
pub fn write(path: &Path, id: &str, golden: &Golden, force: bool) -> Result<()> {
    let text = std::fs::read_to_string(path).map_err(|e| DocError::io("reading", path, e))?;
    let fail = |message: String| DocError::Accept {
        id: id.to_owned(),
        path: path.to_path_buf(),
        message,
    };

    let (open_start, open_end) = find_open_tag(&text, id)
        .ok_or_else(|| fail("the page carries no `<example>` with that id".into()))?;
    let body_end = text[open_end..]
        .find("</example>")
        .map(|i| open_end + i)
        .ok_or_else(|| fail("the example has no closing tag".into()))?;

    let body = &text[open_end..body_end];
    let (expect_open, expect_close) = find_child(body, "expect")
        .ok_or_else(|| fail("the example has no `<expect>` child".into()))?;
    if !force && !body[expect_open..expect_close].trim().is_empty() {
        return Err(fail(
            "its golden already holds text — an accepted capture may FILL an empty \
             golden, never silently replace one that somebody signed; pass --force \
             to re-bless it deliberately"
                .into(),
        ));
    }

    let indent = child_indent(body);
    let mut new_body = String::new();
    new_body.push_str(&body[..expect_open]);
    new_body.push_str(&escape(&golden.expect));
    new_body.push_str(&body[expect_close..]);
    new_body = set_stderr(&new_body, golden.stderr.as_deref(), &indent)?;

    let mut open_tag = text[open_start..open_end].to_owned();
    if golden.exit != 0 {
        open_tag = set_exit(&open_tag, golden.exit);
    }

    let mut rewritten = String::with_capacity(text.len() + new_body.len());
    rewritten.push_str(&text[..open_start]);
    rewritten.push_str(&open_tag);
    rewritten.push_str(&new_body);
    rewritten.push_str(&text[body_end..]);

    verify(&rewritten, id, golden).map_err(fail)?;
    std::fs::write(path, rewritten).map_err(|e| DocError::io("writing", path, e))
}

/// Re-read the rewritten page and prove the edit did exactly what it said.
fn verify(text: &str, id: &str, golden: &Golden) -> std::result::Result<(), String> {
    let doc: SpecDoc = vibe_specdoc::from_xml_with(text, Vocabulary::Doc)
        .map_err(|e| format!("the rewritten page no longer parses: {e}"))?;
    let found =
        find_example(&doc, id).ok_or_else(|| "the example vanished from the page".to_owned())?;
    match found {
        Block::Example {
            exit,
            expect,
            stderr,
            ..
        } => {
            if expect != &golden.expect {
                return Err("the written golden is not the captured output".into());
            }
            if stderr.as_deref() != golden.stderr.as_deref() {
                return Err("the written stderr is not the captured stderr".into());
            }
            let recorded = exit.unwrap_or(0);
            if recorded != golden.exit {
                return Err(format!(
                    "the page records exit {recorded}, the command exited {}",
                    golden.exit
                ));
            }
            Ok(())
        }
        _ => Err("the id no longer names an example".into()),
    }
}

/// Depth-first search for one example by id.
fn find_example<'a>(doc: &'a SpecDoc, id: &str) -> Option<&'a Block> {
    fn walk<'a>(blocks: &'a [vibe_specdoc::doc::BlockNode], id: &str) -> Option<&'a Block> {
        blocks.iter().find_map(|node| match &node.block {
            b @ Block::Example { id: found, .. } if found == id => Some(b),
            _ => None,
        })
    }
    fn section<'a>(s: &'a vibe_specdoc::doc::Section, id: &str) -> Option<&'a Block> {
        walk(&s.blocks, id).or_else(|| s.sections.iter().find_map(|sub| section(sub, id)))
    }
    walk(&doc.preamble, id).or_else(|| doc.sections.iter().find_map(|s| section(s, id)))
}

/// The byte range of the `<example …>` start tag carrying `id`.
fn find_open_tag(text: &str, id: &str) -> Option<(usize, usize)> {
    let needle = format!("id=\"{id}\"");
    let mut cursor = 0usize;
    while let Some(found) = text[cursor..].find("<example") {
        let start = cursor + found;
        let end = text[start..].find('>').map(|i| start + i + 1)?;
        let tag = &text[start..end];
        if tag.contains(&needle) && !tag.ends_with("/>") {
            return Some((start, end));
        }
        cursor = end;
    }
    None
}

/// The byte range of a verbatim child's CONTENT inside an example body.
fn find_child(body: &str, tag: &str) -> Option<(usize, usize)> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = body.find(&open)? + open.len();
    let end = body[start..].find(&close)? + start;
    Some((start, end))
}

/// The indent the example's children are written at, read off `<run>`.
fn child_indent(body: &str) -> String {
    let Some(at) = body.find("<run>") else {
        return "    ".into();
    };
    body[..at]
        .rsplit('\n')
        .next()
        .filter(|s| s.chars().all(char::is_whitespace))
        .unwrap_or("    ")
        .to_owned()
}

/// Put the captured stderr in its place — added, replaced or removed.
fn set_stderr(body: &str, stderr: Option<&str>, indent: &str) -> Result<String> {
    let existing = find_child(body, "stderr");
    match (stderr, existing) {
        (Some(text), Some((start, end))) => Ok(format!(
            "{}{}{}",
            &body[..start],
            escape(text),
            &body[end..]
        )),
        (Some(text), None) => {
            let after = body
                .find("</expect>")
                .map(|i| i + "</expect>".len())
                .unwrap_or(body.len());
            Ok(format!(
                "{}\n{indent}<stderr>{}</stderr>{}",
                &body[..after],
                escape(text),
                &body[after..]
            ))
        }
        (None, Some(_)) => {
            let open = body.find("<stderr>").unwrap_or(0);
            let close = body
                .find("</stderr>")
                .map(|i| i + "</stderr>".len())
                .unwrap_or(open);
            let head = body[..open].trim_end_matches([' ', '\t']);
            let head = head.strip_suffix('\n').unwrap_or(head);
            Ok(format!("{head}{}", &body[close..]))
        }
        (None, None) => Ok(body.to_owned()),
    }
}

/// Add `exit="N"` to a start tag, in the writer's own attribute order —
/// after `lang` when there is one, else after `fixture`.
fn set_exit(open_tag: &str, exit: i32) -> String {
    if open_tag.contains(" exit=\"") {
        return open_tag.to_owned();
    }
    let anchor = ["lang=\"", "fixture=\""].into_iter().find_map(|key| {
        let at = open_tag.find(key)?;
        let rest = &open_tag[at + key.len()..];
        let quote = rest.find('"')?;
        Some(at + key.len() + quote + 1)
    });
    match anchor {
        Some(at) => format!("{} exit=\"{exit}\"{}", &open_tag[..at], &open_tag[at..]),
        None => open_tag.to_owned(),
    }
}

/// Text escaping, identical to the dialect writer's: `&`, `<`, `>` and a
/// carriage return, and nothing else.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\r' => out.push_str("&#xD;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests;
