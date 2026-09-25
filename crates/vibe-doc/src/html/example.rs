//! An example's frame in the island — the one a page AUTHORS and the one
//! a translation BORROWS (PROP-057 `##LOC-EXAMPLE-REF`).
//!
//! The two live together because they must come out identical. A command
//! has no translation, so an adaptation writes `<example ref="…"/>` and
//! the pipeline fills it with the source page's own fences; if the frames
//! were written twice, the day one of them learned a new field would be
//! the day an English page and its Russian mirror showed different
//! output. So one function emits the body and the two entry points differ
//! only in the attributes that say WHERE the body came from.
//!
//! A reference this build could not fill is marked `data-unresolved` and
//! carries no fences at all. That is the honest shape: an empty command
//! block would read as a command that does nothing, and a renderer does
//! not abort over a text it could not fetch ([`crate::content`]).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#LOC-EXAMPLE-REF");

use crate::content::ExampleBody;

use super::Attrs;
use super::emit::{anchor_line, close, open, pre_code};

/// The example a page authors: its id, the fixture its command runs in,
/// and the body the page spelled out.
pub(super) fn authored(
    out: &mut String,
    depth: usize,
    id: &str,
    fixture: &str,
    body: &ExampleBody,
    mut attrs: Attrs,
    num: Option<u32>,
) {
    attrs.push(("class", "example".to_owned()));
    attrs.push(("data-example", id.to_owned()));
    attrs.push(("data-fixture", fixture.to_owned()));
    push_exit(&mut attrs, body.exit);
    frame(out, depth, &attrs, body, num);
}

/// The example a translation borrows, and the gap when this build did not
/// have the source page in hand.
///
/// A filled reference carries `data-example` as well, so a shell looking
/// for «the example called `version` on this page» finds it whether the
/// page authored it or borrowed it.
pub(super) fn borrowed(
    out: &mut String,
    depth: usize,
    id: &str,
    found: Option<&ExampleBody>,
    mut attrs: Attrs,
    num: Option<u32>,
) {
    attrs.push(("class", "example".to_owned()));
    let Some(body) = found else {
        attrs.push(("data-example-ref", id.to_owned()));
        attrs.push(("data-unresolved", "true".to_owned()));
        open(out, depth, "div", &attrs);
        anchor_line(out, depth + 1, num);
        close(out, depth, "div");
        return;
    };
    attrs.push(("data-example", id.to_owned()));
    attrs.push(("data-example-ref", id.to_owned()));
    push_exit(&mut attrs, body.exit);
    frame(out, depth, &attrs, body, num);
}

/// The expected exit code, when it is not the default.
///
/// A zero says nothing and stays off the block; a non-zero code is the
/// page promising that the command FAILS, which a reader needs to see
/// before typing it.
fn push_exit(attrs: &mut Attrs, exit: Option<i32>) {
    if exit.unwrap_or(0) != 0 {
        attrs.push(("data-exit", exit.unwrap_or(0).to_string()));
    }
}

/// An example's two (or three) verbatim blocks.
fn frame(out: &mut String, depth: usize, attrs: &Attrs, body: &ExampleBody, num: Option<u32>) {
    open(out, depth, "div", attrs);
    anchor_line(out, depth + 1, num);
    pre_code(
        out,
        depth + 1,
        &[("class", "example-run".to_owned())],
        Some(body.lang.as_deref().unwrap_or("sh")),
        &body.run,
        None,
    );
    pre_code(
        out,
        depth + 1,
        &[("class", "example-output".to_owned())],
        None,
        &body.expect,
        None,
    );
    if let Some(err) = &body.stderr {
        pre_code(
            out,
            depth + 1,
            &[("class", "example-stderr".to_owned())],
            None,
            err,
            None,
        );
    }
    close(out, depth, "div");
}
