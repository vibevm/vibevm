//! Reversible emission markers (PROP-035 §11).
//!
//! When the compiler places a block into `STATIC.md`, it wraps it in an
//! open **and** close comment carrying the block's full `spec://` key — which
//! already encodes package (`group/name`), document (`doc-path`), and section
//! (`anchor`). The paired markers make the compiled document **reversible**:
//! [`decompile`] splits it straight back into its blocks, the same
//! bidirectional traceability specmap gives code. `#embed` splices carry their
//! own nested `<!-- embed: … -->` markers (see [`crate::expand_embeds`]); those
//! stay inside a block's body here.

use crate::doctree::FenceTracker;

/// The open marker for a compiled block keyed by its `spec://` address.
pub fn open(key: &str) -> String {
    format!("<!-- vibe:begin {key} -->")
}

/// The close marker matching [`open`].
pub fn close(key: &str) -> String {
    format!("<!-- vibe:end {key} -->")
}

/// One top-level block recovered from a compiled document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub key: String,
    pub body: String,
}

/// One recognised control line of the reversible emission grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ControlLine {
    Open(String),
    Close(String),
}

const OPEN_PREFIX: &str = "<!-- vibe:begin ";
const CLOSE_PREFIX: &str = "<!-- vibe:end ";
const SUFFIX: &str = " -->";

/// The exact control-line grammar, in one place: a whole line that, trimmed, is
/// `<!-- vibe:begin KEY -->` or `<!-- vibe:end KEY -->` with a non-blank `KEY`
/// that carries no comment terminator of its own.
///
/// The bare prefix is deliberately **not** enough. `[decompile]` splits on this
/// exact shape, so anything looser would let the inter-pass verifier refuse
/// bodies the reverse trip never mistakes for a marker — prose quoting a
/// truncated `<!-- vibe:begin` is content, not a counterfeit.
pub(crate) fn control_line(line: &str) -> Option<ControlLine> {
    let trimmed = line.trim();
    let (rest, wrap): (&str, fn(String) -> ControlLine) =
        if let Some(rest) = trimmed.strip_prefix(OPEN_PREFIX) {
            (rest, ControlLine::Open)
        } else {
            (trimmed.strip_prefix(CLOSE_PREFIX)?, ControlLine::Close)
        };
    let key = rest.strip_suffix(SUFFIX)?;
    if key.trim().is_empty() || key.contains("-->") {
        return None;
    }
    Some(wrap(key.to_string()))
}

/// The Markdown fence machine as a reversible reader must run it: the state
/// itself, plus whether the open fence was opened *inside* a block body.
///
/// An occurrence may legally leave a fence open (`LinkFenceSnapshot::Open`) for
/// the next occurrence to close — the linked lane carries exactly that state per
/// occurrence. The compiler's own `vibe:begin`/`vibe:end` lines are written at
/// occurrence boundaries, outside any body, so they must still be read through
/// such a **carried** fence; a fence a reader met with no block open belongs to
/// ordinary prose, and a marker quoted inside it is a sample.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ControlScanner {
    fence: FenceTracker,
    carried: bool,
}

impl ControlScanner {
    /// Resume at an occurrence boundary. An inherited open fence is carried by
    /// definition: the previous occurrence's body opened it inside a block.
    pub(crate) fn resume(snapshot: crate::doctree::FenceSnapshot, inside_block: bool) -> Self {
        Self {
            fence: FenceTracker::from_snapshot(snapshot),
            carried: inside_block && snapshot != crate::doctree::FenceSnapshot::Closed,
        }
    }

    /// Whether a control line at this position would be read structurally
    /// rather than as body text, and advance the machine over `line`.
    ///
    /// `inside_block` is the reader's state *before* this line, so a fence this
    /// line opens is carried exactly when a block is open around it.
    pub(crate) fn step(&mut self, line: &str, inside_block: bool) -> ControlPosition {
        let open_before = self.fence.snapshot() != crate::doctree::FenceSnapshot::Closed;
        self.fence.classify(line);
        let open_after = self.fence.snapshot() != crate::doctree::FenceSnapshot::Closed;
        if !open_before && open_after {
            self.carried = inside_block;
        } else if open_before && !open_after {
            self.carried = false;
        }
        ControlPosition {
            control: control_line(line),
            fenced: open_before,
            carried: open_before && self.carried,
        }
    }
}

/// One classified line: its control grammar, and whether a control there is
/// readable as structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ControlPosition {
    pub(crate) control: Option<ControlLine>,
    fenced: bool,
    carried: bool,
}

impl ControlPosition {
    /// A control here is structure when no fence hides it, or when the fence
    /// hiding it was carried across an occurrence boundary.
    pub(crate) fn readable(&self) -> bool {
        !self.fenced || self.carried
    }

    /// Whether a Markdown code fence was open across this line.
    pub(crate) fn fenced(&self) -> bool {
        self.fenced
    }
}

/// Split a compiled document back into its top-level blocks by their markers,
/// proving the emission is reversible (§11). Nested markers (an `#embed`'s) stay
/// in the enclosing block's body, and so does a fenced code sample that merely
/// spells a marker out.
///
/// Only two shapes are structure: an `open` while no block is open, and the
/// `close` of the open block's *own* key. Every other control line — a nested
/// `open`, a `close` naming somebody else — is body text, so a quoted sample
/// cannot truncate the block that contains it. A fence carried across an
/// occurrence boundary does not hide the compiler's own framing.
pub fn decompile(text: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut current: Option<(String, Vec<&str>)> = None;
    let mut scanner = ControlScanner::default();

    for line in text.lines() {
        let position = scanner.step(line, current.is_some());
        let structural = position.readable()
            && match (&current, &position.control) {
                (None, Some(ControlLine::Open(_))) => true,
                (Some((key, _)), Some(ControlLine::Close(closed))) => closed == key,
                _ => false,
            };
        if structural {
            match position.control {
                Some(ControlLine::Open(key)) => current = Some((key, Vec::new())),
                _ => {
                    if let Some((key, body)) = current.take() {
                        blocks.push(Block {
                            key,
                            body: body.join("\n"),
                        });
                    }
                }
            }
            continue;
        }
        if let Some((_, body)) = current.as_mut() {
            body.push(line);
        }
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_and_close_are_paired() {
        assert_eq!(
            open("spec://org.vibevm.core/vibevm/a#r"),
            "<!-- vibe:begin spec://org.vibevm.core/vibevm/a#r -->"
        );
        assert_eq!(
            close("spec://org.vibevm.core/vibevm/a#r"),
            "<!-- vibe:end spec://org.vibevm.core/vibevm/a#r -->"
        );
    }

    #[test]
    fn decompile_recovers_blocks() {
        let doc = format!(
            "{}\nbody line one\nbody line two\n{}\n",
            open("spec://org.vibevm.core/vibevm/a#r"),
            close("spec://org.vibevm.core/vibevm/a#r"),
        );
        let blocks = decompile(&doc);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].key, "spec://org.vibevm.core/vibevm/a#r");
        assert_eq!(blocks[0].body, "body line one\nbody line two");
    }

    #[test]
    fn two_blocks_recovered_in_order() {
        let doc = format!(
            "{}\nfirst\n{}\n{}\nsecond\n{}\n",
            open("spec://org.vibevm.core/vibevm/a#r"),
            close("spec://org.vibevm.core/vibevm/a#r"),
            open("spec://org.vibevm.core/vibevm/b#r"),
            close("spec://org.vibevm.core/vibevm/b#r"),
        );
        let keys: Vec<String> = decompile(&doc).into_iter().map(|b| b.key).collect();
        assert_eq!(
            keys,
            [
                "spec://org.vibevm.core/vibevm/a#r",
                "spec://org.vibevm.core/vibevm/b#r"
            ]
        );
    }

    #[test]
    fn the_grammar_needs_the_whole_comment_not_just_the_prefix() {
        assert_eq!(
            control_line("<!-- vibe:begin spec://org.a/b/c#r -->"),
            Some(ControlLine::Open("spec://org.a/b/c#r".to_string()))
        );
        assert_eq!(
            control_line("   <!-- vibe:end spec://org.a/b/c#r -->  "),
            Some(ControlLine::Close("spec://org.a/b/c#r".to_string()))
        );
        // Prose that only starts like a marker is content, not a control line.
        assert_eq!(control_line("<!-- vibe:begin spec://org.a/b/c#r"), None);
        assert_eq!(control_line("<!-- vibe:begin  -->"), None);
        assert_eq!(control_line("<!-- vibe:begin x --> and more"), None);
        assert_eq!(control_line("<!-- vibe:begin a --> b -->"), None);
        assert_eq!(control_line("<!-- vibe:beginner x -->"), None);
    }

    #[test]
    fn a_fenced_marker_sample_stays_inside_its_block() {
        let doc = format!(
            "{}\nbefore\n```markdown\n{}\n{}\n```\nafter\n{}\n",
            open("spec://org.vibevm.core/vibevm/a#r"),
            open("spec://org.vibevm.core/vibevm/quoted#r"),
            close("spec://org.vibevm.core/vibevm/quoted#r"),
            close("spec://org.vibevm.core/vibevm/a#r"),
        );
        let blocks = decompile(&doc);
        assert_eq!(blocks.len(), 1, "the fenced sample is content: {blocks:?}");
        assert_eq!(blocks[0].key, "spec://org.vibevm.core/vibevm/a#r");
        assert!(
            blocks[0]
                .body
                .contains("vibe:begin spec://org.vibevm.core/vibevm/quoted#r")
        );
        assert!(blocks[0].body.contains("after"));
    }

    #[test]
    fn nested_embed_markers_stay_in_the_body() {
        let doc = format!(
            "{}\n<!-- embed: spec://org.vibevm.core/vibevm/x#r -->\nembedded text\n<!-- /embed: spec://org.vibevm.core/vibevm/x#r -->\n{}\n",
            open("spec://org.vibevm.core/vibevm/a#r"),
            close("spec://org.vibevm.core/vibevm/a#r"),
        );
        let blocks = decompile(&doc);
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].body.contains("embedded text"));
        assert!(blocks[0].body.contains("<!-- embed:"));
    }
}
